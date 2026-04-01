// builtin

// external

// internal
use crate::{archives::summarization::summary::Summary, globals::Transcript};

// modules
pub mod implementations;
pub mod subsystems;
pub mod summary;

pub trait Summarizer {
    fn summarize_transcript(&mut self, transcript: Transcript) -> Summary;
}
