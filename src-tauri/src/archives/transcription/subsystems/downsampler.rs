// builtin
use std::thread::{self, JoinHandle};

// external
use audioadapter_buffers::owned::InterleavedOwned;
use cpal::SupportedStreamConfig;
use crossbeam_channel::{bounded, select, Receiver, Sender};
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

// internal
use crate::{error::TranscriptionError, globals::Chunk};

pub struct Downsampler {
    target_hz: u32,
    handle: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

impl Downsampler {
    pub fn new(target_hz: u32) -> Downsampler {
        Downsampler {
            target_hz,
            handle: None,
            stop_sender: None,
        }
    }

    pub fn setup_stream(
        &mut self,
        config: SupportedStreamConfig,
        audio_receiver: Receiver<Chunk>,
        sampled_sender: Sender<Chunk>,
    ) {
        let from_hz: usize = config.sample_rate() as usize;
        let to_hz: usize = self.target_hz as usize;
        let (stop_tx, stop_rx) = bounded::<()>(1);

        let handle = thread::spawn(move || loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(audio_receiver) -> msg => {
                    match msg {
                        Ok(chunk) => {
                            let res: Result<Chunk, TranscriptionError> = downsample_chunk(chunk, from_hz, to_hz);
                            match res {
                                Ok(processed) => {
                                    if sampled_sender.send(processed).is_err() {
                                        break;
                                    }
                                }
                                Err(err) => {
                                    eprintln!("[DOWNSAMPLER] Failed to downsample chunk: {:?}", err);
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
        });

        self.stop_sender = Some(stop_tx);
        self.handle = Some(handle);
    }

    pub fn close_stream(&mut self) {
        if let Some(stop) = self.stop_sender.take() {
            let _ = stop.send(());
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn downsample_chunk(
    chunk: Vec<f32>,
    from_hz: usize,
    to_hz: usize,
) -> Result<Vec<f32>, TranscriptionError> {
    if from_hz == to_hz {
        return Ok(chunk.clone());
    }

    let ratio: f64 = to_hz as f64 / from_hz as f64;

    let frames: usize = chunk.len();

    let params: SincInterpolationParameters = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        oversampling_factor: 128,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler: Async<f32> =
        Async::<f32>::new_sinc(ratio, 1.1, &params, frames.max(1), 1, FixedAsync::Input).map_err(
            |e| TranscriptionError::InternalError(format!("[RESAMPLE] rubato init error: {:?}", e)),
        )?;

    let in_buf: InterleavedOwned<f32> = InterleavedOwned::new_from(chunk.clone(), 1, frames)
        .map_err(|e| {
            TranscriptionError::InternalError(format!("[RESAMPLE] input buffer error: {:?}", e))
        })?;

    let out: InterleavedOwned<f32> = resampler.process(&in_buf, 0, None).map_err(|e| {
        TranscriptionError::InternalError(format!("[RESAMPLE] rubato process error: {:?}", e))
    })?;

    Ok(out.take_data())
}
