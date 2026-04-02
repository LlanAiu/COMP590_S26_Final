// builtin

// external

// internal
use crate::{
    archives::transcription::{implementations::parakeet::ParakeetTranscriber, AudioTranscriber},
    error::{ApplicationError, TranscriptionError},
};

// modules
pub mod summarization;
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
        let transcript_rx = self.transcriber.start_record_audio()?;

        Ok(())
    }

    pub fn stop_audio_recording(&mut self) -> Result<(), TranscriptionError> {
        let transcript = self.transcriber.stop_record_audio()?;

        println!("GOT TRANSCRIPT: {:?}", transcript);

        Ok(())
    }
}
