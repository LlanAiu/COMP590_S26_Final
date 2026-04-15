use std::sync::{Arc, RwLock};

use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

use crate::archives::control::constants::OLLAMA_MODEL;
use crate::archives::summarization::summary::Note;
use crate::archives::writer::{Writer, WriterError};

pub struct OllamaWriter {
    ollama: Arc<Ollama>,
    model: Arc<RwLock<String>>,
}

impl OllamaWriter {
    pub fn new(model: Option<String>) -> OllamaWriter {
        let ollama = Ollama::default();
        OllamaWriter {
            ollama: Arc::new(ollama),
            model: Arc::new(RwLock::new(
                model.unwrap_or_else(|| OLLAMA_MODEL.to_string()),
            )),
        }
    }

    pub fn set_model(&self, model: String) {
        if let Ok(mut w) = self.model.write() {
            *w = model;
        }
    }
}

impl Writer for OllamaWriter {
    fn compose<'a>(
        &'a self,
        current: &'a str,
        notes: Vec<Note>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, WriterError>> + Send + 'a>>
    {
        Box::pin(async move {
            // build a clear prompt to integrate notes into the existing document
            let mut prompt = String::new();
            prompt.push_str("Given an existing document and a set of new short notes, integrate the new notes into the existing document so the result reads naturally and preserves the original content and voice.\n\n");
            prompt.push_str("Requirements:\n");
            prompt.push_str("- Keep existing content; do not delete unless the note explicitly says to remove or replace.\n");
            prompt.push_str("- Integrate each note in a logical place (create headings or sections if needed).\n");
            prompt.push_str("- Avoid duplicating content; merge similar ideas.\n");
            prompt.push_str("- Preserve factual content verbatim when appropriate, but write clearly and concisely.\n");
            prompt.push_str("- Return only the full updated document text (no commentary, no JSON, no fences).\n\n");

            prompt.push_str("Existing document:\n");
            prompt.push_str(current);
            prompt.push_str("\n\nNew notes to integrate:\n");
            for (i, n) in notes.iter().enumerate() {
                prompt.push_str(&format!("- [{}] {}\n", i + 1, n.content.trim()));
            }

            let gen_req =
                GenerationRequest::new(self.model.read().unwrap().clone(), prompt).think(false);
            let res = self
                .ollama
                .generate(gen_req)
                .await
                .map_err(|e| WriterError::OllamaError(e.to_string()))?;

            let mut response = res.response.trim().to_string();
            if response.starts_with("```") {
                if let Some(pos) = response.find('\n') {
                    response = response[pos + 1..].to_string();
                }
                if response.ends_with("```") {
                    if let Some(pos) = response.rfind("```") {
                        response = response[..pos].to_string();
                    }
                }
                response = response.trim().to_string();
            }

            Ok(response)
        })
    }
}
