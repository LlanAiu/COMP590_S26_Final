// builtin

// external
use crossbeam_channel::{bounded, Receiver, Sender};

// internal

pub struct ChunkChannel<T> {
    sender: Sender<T>,
    receiver: Receiver<T>,
}

impl<T> ChunkChannel<T> {
    pub fn new(channel_size: usize) -> ChunkChannel<T> {
        let (tx, rx): (Sender<T>, Receiver<T>) = bounded(channel_size);

        ChunkChannel {
            sender: tx,
            receiver: rx,
        }
    }

    pub fn get_sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    pub fn get_receiver(&self) -> Receiver<T> {
        self.receiver.clone()
    }
}
