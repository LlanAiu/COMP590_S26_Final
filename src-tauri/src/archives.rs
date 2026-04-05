// builtin
use crate::archives::volumes::{types::UpdateVolumeRequest, VolumeDatabase};
use futures::executor::block_on;
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

        let base_data_dir: PathBuf = dirs_next::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let volumes_dir: PathBuf = base_data_dir.join("auto-archives").join("volumes");
        let file_db: Arc<FileDatabase> = Arc::new(FileDatabase::new(volumes_dir));

        let db_ref: Arc<FileDatabase> = Arc::clone(&file_db);
        let summarizer: OllamaSummarizer = OllamaSummarizer::new(Some(db_ref));

        return Ok(Archives {
            transcriber,
            summarizer,
            summaries: Arc::new(Mutex::new(Vec::new())),
            summary_thread: None,
            volume_database: file_db,
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

        let summaries_snapshot = guard.clone();
        let db_handle = Arc::clone(&self.volume_database);

        println!("GOT summaries: {:?}", summaries_snapshot);

        thread::spawn(move || {
            for summary in summaries_snapshot.into_iter() {
                for note in summary.notes.into_iter() {
                    let category = note.category.trim().to_string();
                    if category.is_empty() {
                        println!("Ignoring note with empty category: {}", note.content);
                        continue;
                    }

                    let index_res = block_on(db_handle.list_index());
                    let index = match index_res {
                        Ok(i) => i,
                        Err(e) => {
                            eprintln!("Failed to list volumes for category matching: {}", e);
                            continue;
                        }
                    };

                    let matched = index.into_iter().find(|entry| entry.title == category);
                    if let Some(entry) = matched {
                        match block_on(db_handle.read_volume(&entry.id)) {
                            Ok(vol) => {
                                let mut new_content = vol.content.clone();
                                if !new_content.ends_with('\n') {
                                    new_content.push('\n');
                                }
                                new_content.push_str("\n");
                                new_content.push_str(&note.content);

                                let update = UpdateVolumeRequest {
                                    title: None,
                                    content: Some(new_content),
                                    description: None,
                                    tags: None,
                                    version: Some(vol.meta.version),
                                };

                                match block_on(db_handle.edit_volume(&entry.id, update)) {
                                    Ok(updated) => {
                                        println!(
                                            "Appended note to volume '{}'(id={})",
                                            updated.meta.title, updated.meta.id
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to append note to volume {}: {}",
                                            entry.id, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to read matched volume {}: {}", entry.id, e);
                            }
                        }
                    } else {
                        println!(
                            "Ignored note — no matching volume for category '{}': {}",
                            category, note.content
                        );
                    }
                }
            }
        });

        Ok(())
    }

    pub fn get_volume_database(&self) -> Arc<FileDatabase> {
        Arc::clone(&self.volume_database)
    }
}
