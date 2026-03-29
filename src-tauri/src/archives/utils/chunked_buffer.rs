// builtin

// external

// internal

use std::mem::take;

use crate::error::TranscriptionError;

pub struct ChunkedBuffer<T> {
    data: Vec<T>,
    chunk_size: usize,
    on_full: Option<Box<dyn Fn(Vec<T>)>>,
}

impl<T> ChunkedBuffer<T> {
    pub fn new(chunk_size: usize) -> ChunkedBuffer<T> {
        ChunkedBuffer {
            data: Vec::with_capacity(chunk_size),
            chunk_size: chunk_size,
            on_full: None,
        }
    }

    pub fn set_on_full<F: Fn(Vec<T>) + 'static>(&mut self, on_full: F) {
        self.on_full = Some(Box::new(on_full))
    }

    pub fn add_data(&mut self, data: T) -> Result<(), TranscriptionError> {
        self.data.push(data);
        if self.data.len() >= self.chunk_size {
            let chunk = take(&mut self.data);
            match &self.on_full {
                Some(func) => func(chunk),
                None => {
                    return Err(TranscriptionError::InternalError(
                        "Chunked buffer filled prior to on_full handling being set".into(),
                    ))
                }
            };
        }
        Ok(())
    }
}
