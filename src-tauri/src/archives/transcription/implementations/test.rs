// builtin

// external

// internal
use crate::archives::transcription::AudioTranscriber;

pub struct TestTranscriber;

impl TestTranscriber {
    pub fn new() -> TestTranscriber {
        TestTranscriber
    }
}

impl AudioTranscriber for TestTranscriber {
    fn start_record_audio(&mut self) {}

    fn stop_record_audio(&mut self) {}

    fn get_transcript(&self) -> Vec<String> {
        vec![
            "What is this?".to_string(),
            "A completely useless system out here".to_string(),
        ]
    }
}
