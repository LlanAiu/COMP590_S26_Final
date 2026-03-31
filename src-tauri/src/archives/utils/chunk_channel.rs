// builtin

// external

// internal

use std::mem::take;

use crossbeam_channel::{bounded, Receiver, Sender};

pub struct ChunkChannel<T> {
    sender: Option<Sender<T>>,
    receiver: Option<Receiver<T>>,
}

impl<T> ChunkChannel<T> {
    pub fn new(channel_size: usize) -> ChunkChannel<T> {
        let (tx, rx): (Sender<T>, Receiver<T>) = bounded(channel_size);

        ChunkChannel {
            sender: Some(tx),
            receiver: Some(rx),
        }
    }

    pub fn get_sender(&mut self) -> Option<Sender<T>> {
        take(&mut self.sender)
    }

    pub fn get_receiver(&mut self) -> Option<Receiver<T>> {
        take(&mut self.receiver)
    }
}
