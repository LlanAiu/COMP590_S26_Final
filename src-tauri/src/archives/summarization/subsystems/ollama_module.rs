// builtin
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

// external
use crossbeam_channel::{bounded, select, Receiver, RecvTimeoutError, Sender};
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

// internal
use crate::archives::summarization::constants::{OLLAMA_MODEL, SHUTDOWN_DRAIN_TIMEOUT_MS};
use crate::{
    archives::summarization::summary::Summary, error::SummarizationError, globals::Transcript,
};

const TEMP_CATEGORIES: &[&str] = &["Action Items", "Decisions", "Questions", "Highlights"];

pub struct OllamaModule {
    ollama: Arc<Ollama>,
    model: String,
    handle: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

impl OllamaModule {
    pub fn new() -> OllamaModule {
        let ollama: Ollama = Ollama::default();

        OllamaModule {
            ollama: Arc::new(ollama),
            model: OLLAMA_MODEL.into(),
            handle: None,
            stop_sender: None,
        }
    }

    pub fn setup_stream(
        &mut self,
        consolidated_receiver: Receiver<Transcript>,
        summary_sender: Sender<Summary>,
    ) {
        let (stop_tx, stop_rx) = bounded::<()>(1);
        let model = self.model.clone();
        let ollama_ref = Arc::clone(&self.ollama);

        let handle: JoinHandle<()> = thread::spawn(move || loop {
            select! {
                recv(stop_rx) -> _ => {
                    loop {
                        match consolidated_receiver.recv_timeout(Duration::from_millis(SHUTDOWN_DRAIN_TIMEOUT_MS)) {
                            Ok(sentences) => {
                                let prompt = build_prompt(&sentences);

                                let tx = summary_sender.clone();
                                let prompt_clone = prompt.clone();
                                let model_clone = model.clone();
                                let ollama_clone = Arc::clone(&ollama_ref);
                                tauri::async_runtime::spawn(async move {
                                    let res = send_message_ollama(ollama_clone, prompt_clone, model_clone).await;
                                    match res {
                                        Ok(response) => {
                                            let _ = tx.try_send(Summary::from_raw(response));
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
                            let prompt = build_prompt(&sentences);

                            let tx = summary_sender.clone();
                            let prompt_clone = prompt.clone();
                            let model_clone = model.clone();
                            let ollama_clone = Arc::clone(&ollama_ref);
                            tauri::async_runtime::spawn(async move {
                                let res = send_message_ollama(ollama_clone, prompt_clone, model_clone).await;
                                match res {
                                    Ok(response) => {
                                        let _ = tx.try_send(Summary::from_raw(response));
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

fn build_prompt(transcript: &Transcript) -> String {
    let joined = transcript.join("\n");

    let mut prompt = String::new();
    prompt.push_str("You are an assistant that reads an audio transcript and returns concise notes and assigns each note a category from the provided list.\n\n");
    prompt.push_str("Categories:\n");
    for category in TEMP_CATEGORIES.iter() {
        prompt.push_str(&format!("- {}\n", category));
    }
    prompt.push_str("\nTranscript:\n");
    prompt.push_str(&joined);
    prompt.push_str("\n\nOutput format: JSON object with keys \"notes\" (array of short notes) and \"categories\" (array of category names corresponding to each note).\n");

    prompt
}

async fn send_message_ollama(
    ollama: Arc<Ollama>,
    message: String,
    model: String,
) -> Result<String, SummarizationError> {
    let res = ollama
        .generate(GenerationRequest::new(model, message))
        .await
        .map_err(|err| SummarizationError::InternalError(err.to_string()))?;

    Ok(res.response)
}
