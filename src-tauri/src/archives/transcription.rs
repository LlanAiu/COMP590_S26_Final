// builtin

// external

use crossbeam_channel::Receiver;

// internal
use crate::{error::TranscriptionError, globals::Transcript};

// modules
pub mod constants;
pub mod implementations;
pub mod subsystems;

pub trait AudioTranscriber {
    fn start_record_audio(&mut self) -> Result<Receiver<Transcript>, TranscriptionError>;

    fn stop_record_audio(&mut self) -> Result<Transcript, TranscriptionError>;
}
