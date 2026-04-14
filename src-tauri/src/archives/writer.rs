// writer trait + implementations module
pub mod implementations;

use crate::archives::summarization::summary::Note;

#[derive(Debug)]
pub enum WriterError {
    OllamaError(String),
    Other(String),
}

pub trait Writer: Send + Sync {
    fn compose<'a>(&'a self, current: &'a str, notes: Vec<Note>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, WriterError>> + Send + 'a>>;
}

pub use implementations::*;
