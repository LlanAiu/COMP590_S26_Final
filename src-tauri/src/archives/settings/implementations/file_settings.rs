use std::path::PathBuf;

use serde_json;

use crate::archives::settings::constants::SETTINGS_FILENAME;
use crate::archives::settings::types::Settings;

pub struct FileSettings {
    pub base: PathBuf,
    path: PathBuf,
}

impl FileSettings {
    pub fn new(base: PathBuf) -> FileSettings {
        let path = base.join(SETTINGS_FILENAME);
        FileSettings { base, path }
    }

    pub fn load(&self) -> Result<Settings, String> {
        if !self.path.exists() {
            return Ok(Settings::default());
        }

        match std::fs::read_to_string(&self.path) {
            Ok(s) => match serde_json::from_str::<Settings>(&s) {
                Ok(cfg) => Ok(cfg),
                Err(e) => Err(format!("failed to parse settings.json: {}", e)),
            },
            Err(e) => Err(format!("failed to read settings.json: {}", e)),
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<(), String> {
        match serde_json::to_vec_pretty(settings) {
            Ok(bytes) => std::fs::write(&self.path, bytes).map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        }
    }
}
