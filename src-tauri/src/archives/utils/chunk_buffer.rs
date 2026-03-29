// builtin

// external

// internal

use std::{
    mem::take,
    sync::{Arc, Mutex},
};

use crate::{archives::utils::chunk_queue::ChunkQueue, error::TranscriptionError};

pub struct ChunkBuffer<T>
where
    T: Clone + Send + Sync + 'static,
{
    data: Vec<T>,
    chunk_size: usize,
    queue_ref: Option<Arc<Mutex<ChunkQueue<T>>>>,
}

impl<T> ChunkBuffer<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(chunk_size: usize) -> ChunkBuffer<T> {
        ChunkBuffer {
            data: Vec::with_capacity(chunk_size),
            chunk_size: chunk_size,
            queue_ref: None,
        }
    }

    pub fn set_out_queue(&mut self, queue_ref: &Arc<Mutex<ChunkQueue<T>>>) {
        self.queue_ref = Some(Arc::clone(queue_ref));
    }

    pub fn add_data(&mut self, data: T) -> Result<(), TranscriptionError> {
        self.data.push(data);
        if self.data.len() >= self.chunk_size {
            let chunk = take(&mut self.data);
            match &self.queue_ref {
                Some(queue) => {
                    let mut guard = queue
                        .lock()
                        .map_err(|err| TranscriptionError::InternalError(err.to_string()))?;
                    guard.add_chunk(chunk);
                }
                None => {
                    return Err(TranscriptionError::InternalError(
                        "Chunked buffer filled prior to on_full handling being set".into(),
                    ))
                }
            };
        }
        Ok(())
    }

    pub fn add_slice(&mut self, slice: &[T]) -> Result<(), TranscriptionError> {
        for item in slice {
            self.add_data(item.clone())?;
        }
        Ok(())
    }
}
