// builtin

// external

// internal

use crate::{
    archives::transcription::{implementations::test::TestTranscriber, AudioTranscriber},
    globals::Mode,
};

// modules
pub mod transcription;

pub struct Archives {
    transcriber: Box<dyn AudioTranscriber>,
}

impl Archives {
    pub fn new(mode: Mode) -> Archives {
        match mode {
            Mode::TEST => todo!(),
            Mode::NORMAL => Archives {
                transcriber: Box::new(TestTranscriber::new()),
            },
        }
    }
}
