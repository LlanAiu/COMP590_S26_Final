// builtin
use std::{
    env,
    mem::take,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

// external
use crossbeam_channel::{bounded, select, Receiver, Sender};
use parakeet_rs::{ParakeetTDT, TimestampMode, Transcriber};

// internal
use crate::{
    archives::transcription::constants::{TRANSCRIPTION_CHANNELS, TRANSCRIPTION_DESIRED_HZ},
    error::TranscriptionError,
    globals::{Chunk, Transcript},
};

pub struct ParakeetModule {
    parakeet_path: PathBuf,
    transcript: Arc<Mutex<Transcript>>,
    parakeet_thread: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

impl ParakeetModule {
    pub fn new() -> Result<ParakeetModule, TranscriptionError> {
        let model_path: PathBuf = match env::var("TAURI_MODEL_DIR") {
            Ok(dir) => PathBuf::from(dir),
            Err(_) => {
                return Err(TranscriptionError::InternalError(
                    "TDT model path not found in environment!".to_string(),
                ))
            }
        };

        Ok(ParakeetModule {
            parakeet_path: model_path,
            transcript: Arc::new(Mutex::new(Vec::new())),
            parakeet_thread: None,
            stop_sender: None,
        })
    }

    pub fn setup_stream(&mut self, sampled_receiver: Receiver<Chunk>) {
        let model_path_str: PathBuf = self.parakeet_path.clone();
        let transcript_ref = Arc::clone(&self.transcript);
        let (stop_tx, stop_rx) = bounded::<()>(1);

        let handle = thread::spawn(move || {
            let mut parakeet = match ParakeetTDT::from_pretrained(&model_path_str, None) {
                Ok(model) => model,
                Err(err) => {
                    eprintln!("Failed to load parakeet model: {:?}", err);
                    return;
                }
            };

            loop {
                select! {
                    recv(stop_rx) -> _ => {
                        break;
                    }
                    recv(sampled_receiver) -> msg => {
                        match msg {
                            Ok(chunk) => {
                                let res = parakeet.transcribe_samples(
                                    chunk, TRANSCRIPTION_DESIRED_HZ, TRANSCRIPTION_CHANNELS, Some(TimestampMode::Sentences));

                                match res {
                                    Ok(transcript) => {
                                        let mut guard = transcript_ref.lock().unwrap();
                                        for token in transcript.tokens {
                                            guard.push(token.text);
                                        }
                                        drop(guard);
                                    }
                                    Err(err) => {
                                        eprintln!("[PARAKEET] Failed to transcribe audio: {:?}", err);
                                        continue;
                                    }
                                }
                            }
                            Err(_) => {
                                break;
                            }
                        }
                    }
                }
            }
        });

        self.parakeet_thread = Some(handle);
        self.stop_sender = Some(stop_tx);
    }

    pub fn close_stream(&mut self) -> Result<(), TranscriptionError> {
        if let Some(stop) = self.stop_sender.take() {
            stop.send(())
                .map_err(|err| TranscriptionError::ShutdownError(err.to_string()))?;
        }

        if let Some(handle) = self.parakeet_thread.take() {
            if let Err(_) = handle.join() {
                return Err(TranscriptionError::ShutdownError(
                    "[PARAKEET] Failed to close parakeet thread".into(),
                ));
            }
        }

        Ok(())
    }

    pub fn get_and_clear_transcript(&self) -> Result<Transcript, TranscriptionError> {
        let mut guard = self
            .transcript
            .lock()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        let transcript = take(&mut *guard);

        Ok(transcript)
    }
}
