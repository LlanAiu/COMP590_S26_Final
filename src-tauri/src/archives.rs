// builtin
use crate::archives::{
    volumes::{types::UpdateVolumeRequest, VolumeDatabase},
    writer::ollama::OllamaWriter,
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

// external

// internal
use crate::{
    archives::{
        control::subsystems::OllamaController,
        summarization::{implementations::ollama::OllamaSummarizer, summary::Summary, Summarizer},
        transcription::{implementations::parakeet::ParakeetTranscriber, AudioTranscriber},
        volumes::implementations::file_database::FileDatabase,
    },
    error::ApplicationError,
};
use chrono::Utc;

// modules
pub mod control;
pub mod summarization;
pub mod transcription;
pub mod volumes;
pub mod writer;

pub struct Archives {
    transcriber: ParakeetTranscriber,
    summarizer: OllamaSummarizer,
    summaries: Arc<Mutex<Vec<Summary>>>,
    summary_thread: Option<JoinHandle<()>>,
    volume_database: Arc<FileDatabase>,
    control: Arc<OllamaController>,
    writer: Arc<OllamaWriter>,
}

impl Archives {
    pub fn new() -> Result<Archives, ApplicationError> {
        let transcriber: ParakeetTranscriber = ParakeetTranscriber::new()
            .map_err(|err| ApplicationError::InternalError(err.to_string()))?;

        let base_data_dir: PathBuf = dirs_next::data_dir().unwrap_or_else(|| PathBuf::from("."));
        let volumes_dir: PathBuf = base_data_dir.join("auto-archives").join("volumes");
        let file_db: Arc<FileDatabase> = Arc::new(FileDatabase::new(volumes_dir));

        let db_ref: Arc<FileDatabase> = Arc::clone(&file_db);
        let summarizer: OllamaSummarizer = OllamaSummarizer::new(Some(db_ref.clone()));
        let controller = Arc::new(OllamaController::new(None));

        let writer = Arc::new(OllamaWriter::new(None));

        return Ok(Archives {
            transcriber,
            summarizer,
            summaries: Arc::new(Mutex::new(Vec::new())),
            summary_thread: None,
            volume_database: file_db,
            control: controller,
            writer,
        });
    }

    pub fn run_control_on_summary(
        &self,
        summary: Summary,
    ) -> Result<Vec<String>, ApplicationError> {
        // gather index snapshot
        let db_handle = Arc::clone(&self.volume_database);
        let index_res = tauri::async_runtime::block_on(db_handle.list_index());
        let index = match index_res {
            Ok(i) => i,
            Err(e) => {
                return Err(ApplicationError::InternalError(format!(
                    "failed to list volumes: {}",
                    e.to_string()
                )))
            }
        };

        // interpret via controller
        // collect any existing AI summaries for volumes to provide more context
        let mut volumes_ai: Vec<(String, String)> = Vec::new();
        for entry in index.iter() {
            if let Ok(v) = tauri::async_runtime::block_on(db_handle.read_volume(&entry.id)) {
                if let Some(s) = v.meta.ai_summary.clone() {
                    volumes_ai.push((entry.id.clone(), s));
                }
            }
        }

        let actions_res =
            tauri::async_runtime::block_on(self.control.interpret(&summary, &index, &volumes_ai));
        let actions = match actions_res {
            Ok(a) => a,
            Err(e) => {
                return Err(ApplicationError::InternalError(format!(
                    "control interpret failed: {:?}",
                    e
                )))
            }
        };

        // apply
        let apply_res = self
            .control
            .apply_actions(Arc::clone(&self.volume_database), actions);
        match apply_res {
            Ok(r) => Ok(r),
            Err(e) => Err(ApplicationError::InternalError(format!(
                "control apply failed: {:?}",
                e
            ))),
        }
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
        let controller_handle = Arc::clone(&self.control);

        // helper to append a control log entry next to the volumes base
        fn append_control_log(db: &std::sync::Arc<FileDatabase>, desc: String) {
            if let Some(base) = db.base.parent() {
                let root = base.to_path_buf();
                let log_path = root.join("control_log.json");
                let mut existing: Vec<crate::archives::control::types::ControlLogEntry> =
                    if log_path.exists() {
                        match std::fs::read_to_string(&log_path) {
                            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| vec![]),
                            Err(_) => vec![],
                        }
                    } else {
                        vec![]
                    };
                existing.push(crate::archives::control::types::ControlLogEntry {
                    timestamp: Utc::now().to_rfc3339(),
                    description: desc,
                });
                if let Ok(serialized) = serde_json::to_vec_pretty(&existing) {
                    let _ = FileDatabase::atomic_write(&log_path, &serialized);
                }
            }
        }

        println!("GOT summaries: {:?}", summaries_snapshot);

        thread::spawn(move || {
            let mut updated_ids: Vec<String> = Vec::new();
            for summary in summaries_snapshot.into_iter() {
                for note in summary.notes.clone().into_iter() {
                    let category = note.category.trim().to_string();
                    if category.is_empty() {
                        println!("Ignoring note with empty category: {}", note.content);
                        continue;
                    }

                    let index_res = tauri::async_runtime::block_on(db_handle.list_index());
                    let index = match index_res {
                        Ok(i) => i,
                        Err(e) => {
                            eprintln!("Failed to list volumes for category matching: {}", e);
                            continue;
                        }
                    };

                    let matched = index.into_iter().find(|entry| entry.title == category);
                    if let Some(entry) = matched {
                        match tauri::async_runtime::block_on(db_handle.read_volume(&entry.id)) {
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

                                match tauri::async_runtime::block_on(
                                    db_handle.edit_volume(&entry.id, update),
                                ) {
                                    Ok(updated) => {
                                        println!(
                                            "Appended note to volume '{}'(id={})",
                                            updated.meta.title, updated.meta.id
                                        );
                                        // log the write in the control log so the frontend can show it
                                        append_control_log(
                                            &db_handle,
                                            format!(
                                                "Appended note to volume '{}'(id={})",
                                                updated.meta.title, updated.meta.id
                                            ),
                                        );
                                        updated_ids.push(updated.meta.id.clone());
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

                // After notes for this summary are appended, run the control agent
                // collect index and any stored AI summaries to provide richer context
                let index_list = match tauri::async_runtime::block_on(db_handle.list_index()) {
                    Ok(i) => i,
                    Err(_) => vec![],
                };
                let mut volumes_ai: Vec<(String, String)> = Vec::new();
                for entry in index_list.iter() {
                    if let Ok(v) = tauri::async_runtime::block_on(db_handle.read_volume(&entry.id))
                    {
                        if let Some(s) = v.meta.ai_summary.clone() {
                            volumes_ai.push((entry.id.clone(), s));
                        }
                    }
                }
                match tauri::async_runtime::block_on(controller_handle.interpret(
                    &summary,
                    &index_list,
                    &volumes_ai,
                )) {
                    Ok(actions) => {
                        match controller_handle.apply_actions(Arc::clone(&db_handle), actions) {
                            Ok(results) => println!("[CONTROL] applied actions: {:?}", results),
                            Err(e) => eprintln!("[CONTROL] apply failed: {:?}", e),
                        }
                    }
                    Err(e) => eprintln!("[CONTROL] interpret failed: {:?}", e),
                }
            }

            // After processing summaries and running control actions, extract keypoints
            // for any volumes we modified, and persist them in the volume meta.
            // Deduplicate ids
            updated_ids.sort();
            updated_ids.dedup();
            for id in updated_ids.into_iter() {
                match tauri::async_runtime::block_on(db_handle.read_volume(&id)) {
                    Ok(vol) => {
                        // extract keypoints and persist
                        match tauri::async_runtime::block_on(
                            controller_handle.extract_keypoints(&vol.content),
                        ) {
                            Ok(points) => {
                                if let Err(e) = tauri::async_runtime::block_on(
                                    db_handle.set_keypoints(&id, points),
                                ) {
                                    eprintln!("Failed to persist keypoints for {}: {:?}", id, e);
                                } else {
                                    println!("Persisted keypoints for volume {}", id);
                                    append_control_log(
                                        &db_handle,
                                        format!("Persisted keypoints for volume {}", id),
                                    );
                                }
                            }
                            Err(e) => eprintln!("Keypoint extraction failed for {}: {:?}", id, e),
                        }

                        // generate an AI-only summary and persist in volume metadata
                        match tauri::async_runtime::block_on(
                            controller_handle.generate_ai_summary(&vol.content),
                        ) {
                            Ok(ai_sum) => {
                                if let Err(e) = tauri::async_runtime::block_on(
                                    db_handle.set_ai_summary(&id, ai_sum.clone()),
                                ) {
                                    eprintln!("Failed to persist AI summary for {}: {:?}", id, e);
                                } else {
                                    println!("Persisted AI summary for volume {}", id);
                                    append_control_log(
                                        &db_handle,
                                        format!("Persisted AI summary for volume {}", id),
                                    );
                                }
                            }
                            Err(e) => eprintln!("AI summary generation failed for {}: {:?}", id, e),
                        }
                    }
                    Err(e) => eprintln!("Failed to read volume {} for keypoints: {}", id, e),
                }
            }
        });

        Ok(())
    }

    pub fn get_volume_database(&self) -> Arc<FileDatabase> {
        Arc::clone(&self.volume_database)
    }
}
