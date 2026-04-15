// builtin
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// external
use crossbeam_channel::{bounded, select, Receiver, RecvTimeoutError, Sender};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

// internal
use crate::archives::summarization::constants::{OLLAMA_MODEL, SHUTDOWN_DRAIN_TIMEOUT_MS};
use crate::archives::volumes::implementations::file_database::FileDatabase;
use crate::archives::volumes::VolumeDatabase;
use crate::{
    archives::summarization::summary::Summary, error::SummarizationError, globals::Transcript,
};

const TEMP_CATEGORIES: &[&str] = &["To-Dos's", "Build Updates", "Things to Research"];

pub struct OllamaModule {
    ollama: Arc<Ollama>,
    model: Arc<RwLock<String>>,
    handle: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
    categories_db: Option<Arc<FileDatabase>>,
}

impl OllamaModule {
    pub fn new() -> OllamaModule {
        let ollama: Ollama = Ollama::default();

        OllamaModule {
            ollama: Arc::new(ollama),
            model: Arc::new(RwLock::new(OLLAMA_MODEL.into())),
            handle: None,
            stop_sender: None,
            categories_db: None,
        }
    }

    pub fn new_with_db(db: Option<Arc<FileDatabase>>) -> OllamaModule {
        let mut m = OllamaModule::new();
        m.categories_db = db;
        m
    }

    pub fn new_with_db_and_model(
        db: Option<Arc<FileDatabase>>,
        model: Option<String>,
    ) -> OllamaModule {
        let mut m = OllamaModule::new();
        m.categories_db = db;
        if let Some(mdl) = model {
            if let Ok(mut w) = m.model.write() {
                *w = mdl;
            }
        }
        m
    }

    pub fn set_model(&self, model: String) {
        if let Ok(mut w) = self.model.write() {
            *w = model;
        }
    }

    pub fn setup_stream(
        &mut self,
        consolidated_receiver: Receiver<Transcript>,
        summary_sender: Sender<Summary>,
    ) {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let model = Arc::clone(&self.model);
        let ollama_ref = Arc::clone(&self.ollama);
        let categories_db_clone = self.categories_db.clone();

        let handle: JoinHandle<()> = thread::spawn(move || loop {
            select! {
                recv(stop_rx) -> _ => {
                    loop {
                        match consolidated_receiver.recv_timeout(Duration::from_millis(SHUTDOWN_DRAIN_TIMEOUT_MS)) {
                            Ok(sentences) => {
                                let mut categories: Vec<String> = TEMP_CATEGORIES.iter().map(|s| s.to_string()).collect();
                                if let Some(db) = categories_db_clone.as_ref() {
                                    match tauri::async_runtime::block_on(db.list_index()) {
                                        Ok(list) => {
                                            if !list.is_empty() {
                                                // Build category strings; append stored AI summary from volume metadata when available.
                                                for e in list.into_iter() {
                                                    let mut entry_str = match e.description {
                                                        Some(d) if !d.trim().is_empty() => format!("{} — {}", e.title.clone(), d.trim()),
                                                        _ => e.title.clone(),
                                                    };
                                                    if let Ok(vol) = tauri::async_runtime::block_on(db.read_volume(&e.id)) {
                                                        if let Some(ai) = vol.meta.ai_summary {
                                                            if !ai.trim().is_empty() {
                                                                entry_str.push_str(&format!(" — AI: {}", ai.trim()));
                                                            }
                                                        }
                                                    }
                                                    categories.push(entry_str);
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            eprintln!("[OLLAMA_MODULE] Failed to load categories from DB: {}", err.to_string());
                                        }
                                    }
                                }

                                let prompt = build_prompt(&sentences, &categories);

                                let tx = summary_sender.clone();
                                let prompt_clone = prompt.clone();
                                let model_clone = { let r = model.read().unwrap(); r.clone() };
                                let ollama_clone = Arc::clone(&ollama_ref);
                                tauri::async_runtime::spawn(async move {
                                    let res = send_message_ollama(ollama_clone, prompt_clone, model_clone).await;
                                    match res {
                                        Ok(response) => {
                                            match Summary::from_json(&response) {
                                                Ok(summary) => {
                                                    let _ = tx.try_send(summary);
                                                }
                                                Err(err) => {
                                                    eprintln!("[OLLAMA_MODULE] Couldn't convert response to summary: {:?}", err);
                                                }
                                            };
                                        }
                                        Err(err) => {
                                            eprintln!("[OLLAMA_MODULE] Ollama generation failed while draining: {:?}", err);
                                        }
                                    }
                                });
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                break;
                            }
                            Err(RecvTimeoutError::Disconnected) => {
                                break;
                            }
                        }
                    }

                    break;
                }
                recv(consolidated_receiver) -> msg => {
                    match msg {
                        Ok(sentences) => {
                            let mut categories: Vec<String> = Vec::new();
                            if let Some(db) = categories_db_clone.as_ref() {
                                match tauri::async_runtime::block_on(db.list_index()) {
                                    Ok(list) => {
                                        if !list.is_empty() {
                                            for e in list.into_iter() {
                                                let mut entry_str = match e.description {
                                                    Some(d) if !d.trim().is_empty() => format!("{} — {}", e.title.clone(), d.trim()),
                                                    _ => e.title.clone(),
                                                };
                                                if let Ok(vol) = tauri::async_runtime::block_on(db.read_volume(&e.id)) {
                                                    if let Some(ai) = vol.meta.ai_summary {
                                                        if !ai.trim().is_empty() {
                                                            entry_str.push_str(&format!(" — AI: {}", ai.trim()));
                                                        }
                                                    }
                                                }
                                                categories.push(entry_str);
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("[OLLAMA_MODULE] Failed to load categories from DB: {}", err.to_string());
                                    }
                                }
                            }

                            let prompt = build_prompt(&sentences, &categories);

                            let tx = summary_sender.clone();
                            let prompt_clone = prompt.clone();
                            let model_clone = { let r = model.read().unwrap(); r.clone() };
                            let ollama_clone = Arc::clone(&ollama_ref);
                            tauri::async_runtime::spawn(async move {
                                let res = send_message_ollama(ollama_clone, prompt_clone, model_clone).await;
                                match res {
                                    Ok(response) => {
                                        match Summary::from_json(&response) {
                                            Ok(summary) => {
                                                let _ = tx.try_send(summary);
                                            }
                                            Err(err) => {
                                                eprintln!("[OLLAMA_MODULE] Couldn't convert response to summary: {:?}", err);
                                            }
                                        };
                                    }
                                    Err(err) => {
                                        eprintln!("[OLLAMA_MODULE] Ollama generation failed: {:?}", err);
                                    }
                                }
                            });
                        }
                        Err(err) => {
                            eprintln!("[OLLAMA_MODULE] Processing channel disconnected: {:?}", err.to_string());
                            break;
                        }
                    }
                }
            }
        });

        self.stop_sender = Some(stop_tx);
        self.handle = Some(handle);
    }

    pub fn close_stream(&mut self) -> Result<(), SummarizationError> {
        if let Some(stop) = self.stop_sender.take() {
            if let Err(err) = stop.send(()) {
                eprintln!(
                    "[OLLAMA_MODULE] stop sender send failed (likely already disconnected): {}",
                    err.to_string()
                );
            }
        }

        if let Some(handle) = self.handle.take() {
            if let Err(_) = handle.join() {
                return Err(SummarizationError::InternalError(
                    "[OLLAMA_MODULE] Failed to close ollama thread".into(),
                ));
            }
        }

        Ok(())
    }
}

fn build_prompt(transcript: &Transcript, categories: &[String]) -> String {
    let joined = transcript.join("\n");

    let mut prompt = String::new();
    prompt.push_str("As a personal archivist, your job is to parse through the provided audio transcript of the user's monologue and return 0 - 2 concise, but complete notes. Then assign each note to the category that the note most closely aligns with from the provided list. Expect faulty punctuation and transcription typos that you'll need to infer the meaning through.\n\n");
    prompt.push_str("Categories (Name - Description) -- DO NOT EXTRACT NOTES FROM HERE:\n");
    for category in categories.iter() {
        prompt.push_str(&format!("- {}\n", category));
    }
    prompt.push_str("\n Transcript -- EXTRACT NOTES FROM HERE:\n");
    prompt.push_str(&joined);
    prompt.push_str("\n\nOutput format: A JSON array of note objects composed of \"content\" (the important idea) and \"category\" (which of the listed category names it fits under). Only extract the important information, ignoring filler words or irrelevant content. \n");

    prompt
}

async fn send_message_ollama(
    ollama: Arc<Ollama>,
    message: String,
    model: String,
) -> Result<String, SummarizationError> {
    let res = ollama
        .generate(GenerationRequest::new(model, message).think(false))
        .await
        .map_err(|err| SummarizationError::InternalError(err.to_string()))?;

    Ok(res.response)
}
