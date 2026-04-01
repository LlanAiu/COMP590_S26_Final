// builtin

// external

// internal
use crate::{archives::summarization::Summarizer, error::SummarizationError};

pub struct TestSummarizer;

impl TestSummarizer {
    pub fn new() -> TestSummarizer {
        TestSummarizer
    }
}

impl Summarizer for TestSummarizer {
    fn setup_summarization(&mut self) -> Result<(), SummarizationError> {
        Ok(())
    }

    fn close_summarization(&mut self) -> Result<(), SummarizationError> {
        Ok(())
    }
}
