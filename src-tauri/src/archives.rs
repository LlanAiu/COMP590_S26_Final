// builtin

// external

// internal
use crate::{
    archives::transcription::{
        implementations::{parakeet::ParakeetTranscriber, test::TestTranscriber},
        AudioTranscriber,
    },
    error::{ApplicationError, TranscriptionError},
    globals::{Mode, Transcript},
};

// modules
pub mod transcription;

pub struct Archives {
    transcriber: Box<dyn AudioTranscriber>,
}

impl Archives {
    pub fn new(mode: Mode) -> Result<Archives, ApplicationError> {
        match mode {
            Mode::TEST => Ok(Archives {
                transcriber: Box::new(TestTranscriber::new()),
            }),
            Mode::NORMAL => {
                let transcriber: ParakeetTranscriber = ParakeetTranscriber::new()
                    .map_err(|err| ApplicationError::InternalError(err.to_string()))?;
                return Ok(Archives {
                    transcriber: Box::new(transcriber),
                });
            }
        }
    }

    pub fn start_audio_recording(&mut self) -> Result<(), TranscriptionError> {
        self.transcriber.start_record_audio()
    }

    pub fn stop_audio_recording(&mut self) -> Result<Transcript, TranscriptionError> {
        self.transcriber.stop_record_audio()
    }
}
