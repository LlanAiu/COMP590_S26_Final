// builtin
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

// external

// internal
use crate::{
    archives::{
        summarization::{implementations::ollama::OllamaSummarizer, summary::Summary, Summarizer},
        transcription::{implementations::parakeet::ParakeetTranscriber, AudioTranscriber},
        volumes::implementations::file_database::FileDatabase,
    },
    error::ApplicationError,
};

// modules
pub mod summarization;
pub mod transcription;
pub mod volumes;

pub struct Archives {
    transcriber: ParakeetTranscriber,
    summarizer: OllamaSummarizer,
    summaries: Arc<Mutex<Vec<Summary>>>,
    summary_thread: Option<JoinHandle<()>>,
    volume_database: Arc<FileDatabase>,
}

impl Archives {
    pub fn new() -> Result<Archives, ApplicationError> {
        let transcriber: ParakeetTranscriber = ParakeetTranscriber::new()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        let summarizer: OllamaSummarizer = OllamaSummarizer::new();

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let volumes_dir = cwd.join("volumes");
        let file_db = FileDatabase::new(volumes_dir);

        return Ok(Archives {
            transcriber,
            summarizer,
            summaries: Arc::new(Mutex::new(Vec::new())),
            summary_thread: None,
            volume_database: Arc::new(file_db),
        });
    }

    pub fn start_audio_recording(&mut self) -> Result<(), ApplicationError> {
        let transcript_rx = self
            .transcriber
            .start_record_audio()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        let summary_rx = self
            .summarizer
            .setup_summarization(transcript_rx)
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        let summaries_ref = Arc::clone(&self.summaries);
        let handle = thread::spawn(move || {
            for data in summary_rx.iter() {
                let mut guard = summaries_ref.lock().unwrap();

                guard.push(data);

                drop(guard);
            }
        });

        self.summary_thread = Some(handle);

        Ok(())
    }

    pub fn stop_audio_recording(&mut self) -> Result<(), ApplicationError> {
        let transcript = self
            .transcriber
            .stop_record_audio()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        self.summarizer
            .close_summarization()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        println!("GOT TRANSCRIPT: {:?}", transcript);

        if let Some(handle) = self.summary_thread.take() {
            if let Err(_) = handle.join() {
                return Err(ApplicationError::InternalError(
                    "[ARCHIVES] Failed to join summary thread".into(),
                ));
            }
        }

        let guard = self
            .summaries
            .lock()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        println!("GOT summaries: {:?}", *guard);

        Ok(())
    }

    pub fn get_volume_database(&self) -> Arc<FileDatabase> {
        Arc::clone(&self.volume_database)
    }
}
