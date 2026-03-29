// builtin

// external
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};

use crate::error::GenerationError;

// internal

pub async fn send_message_ollama(message: String) -> Result<String, GenerationError> {
    let ollama: Ollama = Ollama::default();

    let model: String = "gemma3:1b".into();

    let res = ollama
        .generate(GenerationRequest::new(model, message))
        .await
        .map_err(|err| GenerationError::InternalError(err.to_string()))?;

    Ok(res.response)
}
