use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub summarization_model: String,
    pub writer_model: String,
    pub control_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            summarization_model: crate::archives::settings::constants::DEFAULT_SUMMARIZATION_MODEL.to_string(),
            writer_model: crate::archives::settings::constants::DEFAULT_WRITER_MODEL.to_string(),
            control_model: crate::archives::settings::constants::DEFAULT_CONTROL_MODEL.to_string(),
        }
    }
}
