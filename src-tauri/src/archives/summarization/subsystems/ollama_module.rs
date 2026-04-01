// builtin
use std::thread::{self, JoinHandle};

// external
use crossbeam_channel::{bounded, select, Receiver, Sender};

// internal
use crate::{error::SummarizationError, globals::Transcript, ollama::send_message_ollama};

// TODO: Feed the categories in, eventually, maybe provide an Arc to the vec or something?
const TEMP_CATEGORIES: &[&str] = &["Action Items", "Decisions", "Questions", "Highlights"];

pub struct OllamaModule {
    handle: Option<JoinHandle<()>>,
    stop_sender: Option<Sender<()>>,
}

impl OllamaModule {
    pub fn new() -> OllamaModule {
        OllamaModule {
            handle: None,
            stop_sender: None,
        }
    }

    pub fn setup_stream(
        &mut self,
        consolidated_receiver: Receiver<Transcript>,
        summary_sender: Sender<String>,
    ) {
        let (stop_tx, stop_rx) = bounded::<()>(1);

        let handle: JoinHandle<()> = thread::spawn(move || loop {
            select! {
                recv(stop_rx) -> _ => {
                    break;
                }
                recv(consolidated_receiver) -> msg => {
                    match msg {
                        Ok(sentences) => {
                            let joined = sentences.join("\n");

                            let mut prompt = String::new();
                            prompt.push_str("You are an assistant that reads an audtio transcript and returns concise notes and assigns each note a category from the provided list.\n\n");
                            prompt.push_str("Categories:\n");
                            for category in TEMP_CATEGORIES.iter() {
                                prompt.push_str(&format!("- {}\n", category));
                            }
                            prompt.push_str("\nTranscript:\n");
                            prompt.push_str(&joined);
                            prompt.push_str("\n\nOutput format: JSON object with keys \"notes\" (array of short notes) and \"categories\" (array of category names corresponding to each note).\n");

                            let tx = summary_sender.clone();
                            let prompt_clone = prompt.clone();
                            tauri::async_runtime::spawn(async move {
                                let res = send_message_ollama(prompt_clone).await;
                                match res {
                                    Ok(response) => {
                                        let _ = tx.try_send(response);
                                    }
                                    Err(err) => {
                                        eprintln!("[OLLAMA_MODULE] Ollama generation failed: {:?}", err);
                                    }
                                }
                            });
                        }
                        Err(_) => {
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
            stop.send(())
                .map_err(|err| SummarizationError::InternalError(err.to_string()))?;
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
