// builtin

use std::sync::Arc;

// external
use crossbeam_channel::{bounded, Receiver};

// internal
use crate::archives::summarization::constants::{
    CHUNK_SENTENCE_LENGTH, CONSOLIDATED_CHANNEL_SIZE, SUMMARY_CHANNEL_SIZE,
};
use crate::archives::summarization::subsystems::{
    half_stream::HalfStream, ollama_module::OllamaModule,
};
use crate::archives::summarization::summary::Summary;
use crate::archives::summarization::Summarizer;
use crate::archives::volumes::implementations::file_database::FileDatabase;
use crate::error::SummarizationError;
use crate::globals::Transcript;

pub struct OllamaSummarizer {
    half_stream: HalfStream,
    ollama: OllamaModule,
}

impl OllamaSummarizer {
    pub fn new(db: Option<Arc<FileDatabase>>, model: Option<String>) -> OllamaSummarizer {
        OllamaSummarizer {
            half_stream: HalfStream::new(CHUNK_SENTENCE_LENGTH),
            ollama: OllamaModule::new_with_db_and_model(db, model),
        }
    }

    pub fn set_model(&self, model: String) {
        self.ollama.set_model(model);
    }
}

impl Summarizer for OllamaSummarizer {
    fn setup_summarization(
        &mut self,
        transcript_receiver: Receiver<Transcript>,
    ) -> Result<Receiver<Summary>, SummarizationError> {
        let (consolidated_tx, consolidated_rx) = bounded::<Transcript>(CONSOLIDATED_CHANNEL_SIZE);
        let (summary_tx, summary_rx) = bounded::<Summary>(SUMMARY_CHANNEL_SIZE);

        self.half_stream
            .setup_stream(transcript_receiver, consolidated_tx);

        self.ollama.setup_stream(consolidated_rx, summary_tx);

        Ok(summary_rx)
    }

    fn close_summarization(&mut self) -> Result<(), SummarizationError> {
        self.half_stream.close_stream()?;
        self.ollama.close_stream()?;
        Ok(())
    }
}
