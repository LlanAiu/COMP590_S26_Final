// builtin

// external

// internal

pub struct ChunkQueue<T>
where
    T: Clone + Send + Sync + 'static,
{
    queue: Vec<T>,
    chunk_size: usize,
}

impl<T> ChunkQueue<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(chunk_size: usize) -> ChunkQueue<T> {
        ChunkQueue {
            queue: Vec::new(),
            chunk_size,
        }
    }

    pub fn add_chunk(&mut self, mut chunk: Vec<T>) {
        self.queue.append(&mut chunk);
    }
}
