// builtin
use std::collections::VecDeque;

// external

// internal

pub struct ChunkQueue<T>
where
    T: Clone + Send + Sync + 'static,
{
    queue: VecDeque<Vec<T>>,
}

impl<T> ChunkQueue<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new() -> ChunkQueue<T> {
        ChunkQueue {
            queue: VecDeque::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: Vec<T>) {
        self.queue.push_back(chunk);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn next_chunk(&mut self) -> Option<Vec<T>> {
        self.queue.pop_front()
    }
}
