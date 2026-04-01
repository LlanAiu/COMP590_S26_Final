// builtin

// external
use crossbeam_channel::bounded;

// internal
use crate::archives::summarization::constants::{CHUNK_SENTENCE_LENGTH, CONSOLIDATED_CHANNEL_SIZE};
use crate::archives::summarization::subsystems::{
    half_stream::HalfStream, ollama_module::OllamaModule,
};
use crate::archives::summarization::Summarizer;
use crate::error::SummarizationError;
use crate::globals::Transcript;

pub struct OllamaSummarizer {
    half_stream: HalfStream,
    ollama: OllamaModule,
}

impl OllamaSummarizer {
    pub fn new() -> OllamaSummarizer {
        OllamaSummarizer {
            half_stream: HalfStream::new(CHUNK_SENTENCE_LENGTH),
            ollama: OllamaModule::new(),
        }
    }
}

impl Summarizer for OllamaSummarizer {
    fn setup_summarization(&mut self) -> Result<(), SummarizationError> {
        let (transcript_tx, transcript_rx) = bounded::<Transcript>(8);
        let (consolidated_tx, consolidated_rx) = bounded::<Transcript>(CONSOLIDATED_CHANNEL_SIZE);
        let (summary_tx, summary_rx) = bounded::<String>(1);

        self.half_stream
            .setup_stream(transcript_rx, consolidated_tx);

        self.ollama.setup_stream(consolidated_rx, summary_tx);

        Ok(())
    }

    fn close_summarization(&mut self) -> Result<(), SummarizationError> {
        self.half_stream.close_stream()?;
        self.ollama.close_stream()?;
        Ok(())
    }
}
