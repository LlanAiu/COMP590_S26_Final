// builtin

// external

// internal
use crate::{
    archives::transcription::{implementations::parakeet::ParakeetTranscriber, AudioTranscriber},
    error::{ApplicationError, TranscriptionError},
    globals::Transcript,
};

// modules
pub mod transcription;

pub struct Archives {
    transcriber: ParakeetTranscriber,
}

impl Archives {
    pub fn new() -> Result<Archives, ApplicationError> {
        let transcriber: ParakeetTranscriber = ParakeetTranscriber::new()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;
        return Ok(Archives { transcriber });
    }

    pub fn start_audio_recording(&mut self) -> Result<(), TranscriptionError> {
        self.transcriber.start_record_audio()
    }

    pub fn stop_audio_recording(&mut self) -> Result<Transcript, TranscriptionError> {
        self.transcriber.stop_record_audio()
    }
}
