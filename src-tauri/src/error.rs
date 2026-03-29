// builtin
use thiserror::Error;

// external

// internal

#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("No audio input device found on device")]
    NoDevicesFound,
    #[error("No input sample range supports >= target sample rate of `{0}`")]
    UnsupportedSampleRange(u32),
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}

#[derive(Error, Debug)]
pub enum GenerationError {
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}
