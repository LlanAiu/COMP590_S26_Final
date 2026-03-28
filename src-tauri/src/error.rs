// builtin
use thiserror::Error;

// external

// internal

#[derive(Error, Debug)]
pub enum TranscriptionError {
    #[error("Something went wrong: `{0}`")]
    InternalError(String),
}
