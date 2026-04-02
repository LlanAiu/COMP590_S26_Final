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
    #[error("No input queue set for submodule `{0}`")]
    NoQueueSet(&'static str),
    #[error("Error occured during shutdown: `{0}`")]
    ShutdownError(String),
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}

#[derive(Error, Debug)]
pub enum SummarizationError {
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}

#[derive(Error, Debug)]
pub enum GenerationError {
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}
