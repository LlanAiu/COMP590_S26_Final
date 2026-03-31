// builtin

// external

// internal
use crate::{
    archives::transcription::AudioTranscriber, error::TranscriptionError, globals::Transcript,
};

pub struct TestTranscriber;

impl TestTranscriber {
    pub fn new() -> TestTranscriber {
        TestTranscriber
    }
}

impl AudioTranscriber for TestTranscriber {
    fn start_record_audio(&mut self) -> Result<(), TranscriptionError> {
        Ok(())
    }

    fn stop_record_audio(&mut self) -> Result<Transcript, TranscriptionError> {
        Ok(vec!["Testing...".into(), "One, two, three...".into()])
    }
}
