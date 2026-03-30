// builtin

// external

// internal
use crate::{
    archives::transcription::{
        implementations::{parakeet::ParakeetTranscriber, test::TestTranscriber},
        AudioTranscriber,
    },
    globals::Mode,
};

// modules
pub mod transcription;
pub mod utils;

pub struct Archives {
    transcriber: Box<dyn AudioTranscriber>,
}

impl Archives {
    pub fn new(mode: Mode) -> Archives {
        match mode {
            Mode::TEST => Archives {
                transcriber: Box::new(TestTranscriber::new()),
            },
            Mode::NORMAL => Archives {
                transcriber: Box::new(ParakeetTranscriber::new()),
            },
        }
    }

    pub fn start_audio_recording(&mut self) {
        self.transcriber.start_record_audio();
    }

    pub fn stop_audio_recording(&mut self) {
        self.transcriber.stop_record_audio();
    }

    pub fn get_transcript(&self) -> Vec<String> {
        self.transcriber.get_transcript()
    }
}
