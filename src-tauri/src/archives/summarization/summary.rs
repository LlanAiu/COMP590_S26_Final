// builtin

// external

// internal

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub content: String,
    pub category: String,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub notes: Vec<Note>,
}

impl Summary {
    pub fn from_json(input: &str) -> Result<Summary, serde_json::Error> {
        let mut processed = input.trim();

        if processed.starts_with("```") {
            if let Some(pos) = processed.find('\n') {
                processed = &processed[pos + 1..];
            } else {
                processed = "";
            }
        }

        if processed.ends_with("```") {
            if let Some(pos) = processed.rfind("```") {
                processed = &processed[..pos];
            }
        }

        let processed = processed.trim();

        println!("Attempting to convert to JSON: {}", processed);

        let notes: Vec<Note> = serde_json::from_str(processed)?;
        Ok(Summary { notes })
    }
}
