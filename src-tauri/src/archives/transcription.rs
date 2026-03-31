// builtin

// external

// internal
use crate::{error::TranscriptionError, globals::Transcript};

// modules
pub mod constants;
pub mod downsampler;
pub mod implementations;
pub mod recorder;

pub trait AudioTranscriber {
    fn start_record_audio(&mut self) -> Result<(), TranscriptionError>;

    fn stop_record_audio(&mut self) -> Result<(), TranscriptionError>;

    fn get_transcript(&self) -> Result<Transcript, TranscriptionError>;
}
