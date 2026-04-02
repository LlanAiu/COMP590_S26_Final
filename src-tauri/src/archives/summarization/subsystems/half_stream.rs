// builtin
use std::thread::{self, JoinHandle};
use std::time::Duration;

// external
use crossbeam_channel::{bounded, select, Receiver, RecvTimeoutError, Sender};

// internal
use crate::archives::summarization::constants::SHUTDOWN_DRAIN_TIMEOUT_MS;
use crate::{error::SummarizationError, globals::Transcript};

pub struct HalfStream {
    chunk_size: usize,
    handle: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

impl HalfStream {
    pub fn new(chunk_size: usize) -> HalfStream {
        HalfStream {
            chunk_size,
            handle: None,
            stop_sender: None,
        }
    }

    pub fn setup_stream(
        &mut self,
        transcript_receiver: Receiver<Transcript>,
        consolidated_sender: Sender<Transcript>,
    ) {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let chunk_size: usize = self.chunk_size;

        let handle: JoinHandle<()> = thread::spawn(move || {
            let mut buffer: Vec<String> = Vec::new();

            loop {
                select! {
                    recv(stop_rx) -> _ => {
                        loop {
                            match transcript_receiver.recv_timeout(Duration::from_millis(SHUTDOWN_DRAIN_TIMEOUT_MS)) {
                                Ok(sentences) => {
                                    for s in sentences.into_iter() {
                                        buffer.push(s);
                                    }

                                    while buffer.len() >= chunk_size {
                                        let out_chunk = buffer.drain(0..chunk_size).collect::<Vec<String>>();
                                        if consolidated_sender.send(out_chunk).is_err() {
                                            return;
                                        }
                                    }
                                }
                                Err(RecvTimeoutError::Timeout) => {
                                    if !buffer.is_empty() {
                                        let remaining = buffer.drain(..).collect::<Vec<String>>();
                                        let _ = consolidated_sender.send(remaining);
                                    }
                                    break;
                                }
                                Err(RecvTimeoutError::Disconnected) => {
                                    if !buffer.is_empty() {
                                        let remaining = buffer.drain(..).collect::<Vec<String>>();
                                        let _ = consolidated_sender.send(remaining);
                                    }
                                    break;
                                }
                            }
                        }

                        break;
                    }
                    recv(transcript_receiver) -> msg => {
                        match msg {
                            Ok(sentences) => {
                                for s in sentences.into_iter() {
                                    buffer.push(s);
                                }

                                while buffer.len() >= chunk_size {
                                    let out_chunk = buffer.drain(0..chunk_size).collect::<Vec<String>>();
                                    if consolidated_sender.send(out_chunk).is_err() {
                                        return;
                                    }
                                }
                            }
                            Err(_) => {
                                if !buffer.is_empty() {
                                    let remaining = buffer.drain(..).collect::<Vec<String>>();
                                    let _ = consolidated_sender.send(remaining);
                                }
                                break;
                            }
                        }
                    }
                }
            }
        });

        self.stop_sender = Some(stop_tx);
        self.handle = Some(handle);
    }

    pub fn close_stream(&mut self) -> Result<(), SummarizationError> {
        if let Some(stop) = self.stop_sender.take() {
            stop.send(())
                .map_err(|err| SummarizationError::InternalError(err.to_string()))?;
        }

        if let Some(handle) = self.handle.take() {
            if let Err(_) = handle.join() {
                return Err(SummarizationError::InternalError(
                    "[HALF_STREAM] Failed to close consolidator thread".into(),
                ));
            }
        }

        Ok(())
    }
}
