// builtin
use std::sync::{Arc, Mutex};

// external
use audioadapter_buffers::owned::InterleavedOwned;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

// internal
use crate::archives::transcription::constants::WAV_BUFFER_SIZE;
use crate::archives::utils::chunk_buffer::ChunkBuffer;
use crate::archives::utils::chunk_queue::ChunkQueue;
use crate::error::TranscriptionError;

// TODO: pass this data from the recorder
const FROM_HZ: usize = 22_500;

pub struct Downsampler {
    buffer: Arc<Mutex<ChunkBuffer<f32>>>,
    in_queue_ref: Option<Arc<Mutex<ChunkQueue<f32>>>>,
    target_hz: usize,
}

impl Downsampler {
    pub fn new(target_hz: usize) -> Downsampler {
        let buffer = ChunkBuffer::new(WAV_BUFFER_SIZE);

        let buffer_ref = Arc::new(Mutex::new(buffer));

        Downsampler {
            buffer: buffer_ref,
            in_queue_ref: None,
            target_hz,
        }
    }

    pub fn set_output_queue(
        &self,
        queue_ref: &Arc<Mutex<ChunkQueue<f32>>>,
    ) -> Result<(), TranscriptionError> {
        let mut guard = self
            .buffer
            .lock()
            .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

        guard.set_out_queue(queue_ref);

        Ok(())
    }

    pub fn set_input_queue(&mut self, queue_ref: &Arc<Mutex<ChunkQueue<f32>>>) {
        self.in_queue_ref = Some(Arc::clone(queue_ref));
    }

    pub fn process_next(&self) -> Result<(), TranscriptionError> {
        if let Some(queue) = &self.in_queue_ref {
            let mut chunk_data: Option<Vec<f32>> = None;
            {
                let mut queue_guard = queue
                    .lock()
                    .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

                if !queue_guard.is_empty() {
                    chunk_data = queue_guard.next_chunk();
                }
            }
            if let Some(chunk) = chunk_data {
                let processed = self.downsample_to(chunk, FROM_HZ)?;
                let mut buffer_guard = self
                    .buffer
                    .lock()
                    .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;

                buffer_guard.add_slice(&processed)?;
            }
            return Ok(());
        }
        Err(TranscriptionError::NoQueueSet("[DOWNSAMPLER]"))
    }

    fn downsample_to(
        &self,
        chunk: Vec<f32>,
        from_hz: usize,
    ) -> Result<Vec<f32>, TranscriptionError> {
        if from_hz == self.target_hz {
            return Ok(chunk.clone());
        }

        let ratio = self.target_hz as f64 / from_hz as f64;

        let frames = chunk.len();

        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            oversampling_factor: 128,
            interpolation: SincInterpolationType::Cubic,
            window: WindowFunction::BlackmanHarris2,
        };

        let mut resampler =
            Async::<f32>::new_sinc(ratio, 1.1, &params, frames.max(1), 1, FixedAsync::Input)
                .map_err(|e| {
                    TranscriptionError::InternalError(format!(
                        "[RESAMPLE] rubato init error: {:?}",
                        e
                    ))
                })?;

        let in_buf = InterleavedOwned::new_from(chunk.clone(), 1, frames).map_err(|e| {
            TranscriptionError::InternalError(format!("[RESAMPLE] input buffer error: {:?}", e))
        })?;

        let out = resampler.process(&in_buf, 0, None).map_err(|e| {
            TranscriptionError::InternalError(format!("[RESAMPLE] rubato process error: {:?}", e))
        })?;

        Ok(out.take_data())
    }
}
