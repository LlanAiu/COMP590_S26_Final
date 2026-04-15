// Types for the control module: actions and errors

use serde::{Deserialize, Serialize};

use crate::archives::volumes::types::CreateVolumeRequest;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ControlAction {
    Create {
        req: CreateVolumeRequest,
    },
    Nest {
        parent_id: String,
        child_id: String,
    },
    Flatten {
        id: String,
    },
    Merge {
        a_id: String,
        b_id: String,
        req: CreateVolumeRequest,
    },
    Split {
        id: String,
        first: CreateVolumeRequest,
        second: CreateVolumeRequest,
    },
    // reserved for future actions like 'extract_keypoints'
}

#[derive(Debug)]
pub enum ControlError {
    ParseError(String),
    OllamaError(String),
    ActionError(String),
}

impl From<serde_json::Error> for ControlError {
    fn from(e: serde_json::Error) -> Self {
        ControlError::ParseError(e.to_string())
    }
}

impl From<String> for ControlError {
    fn from(s: String) -> Self {
        ControlError::ActionError(s)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ControlLogEntry {
    pub timestamp: String,
    pub description: String,
}
