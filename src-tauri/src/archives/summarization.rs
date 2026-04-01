// builtin

// external

// internal
use crate::error::SummarizationError;

// modules
pub mod constants;
pub mod implementations;
pub mod subsystems;
pub mod summary;

pub trait Summarizer {
    fn setup_summarization(&mut self) -> Result<(), SummarizationError>;

    fn close_summarization(&mut self) -> Result<(), SummarizationError>;
}
