// builtin

// external

use crossbeam_channel::Receiver;

// internal
use crate::{
    archives::summarization::summary::Summary, error::SummarizationError, globals::Transcript,
};

// modules
pub mod constants;
pub mod implementations;
pub mod subsystems;
pub mod summary;

pub trait Summarizer {
    fn setup_summarization(
        &mut self,
        transcript_receiver: Receiver<Transcript>,
    ) -> Result<Receiver<Summary>, SummarizationError>;

    fn close_summarization(&mut self) -> Result<(), SummarizationError>;
}
