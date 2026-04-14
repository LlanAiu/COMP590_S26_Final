pub const SETTINGS_FILENAME: &str = "settings.json";

// allowed/hardcoded Ollama model options
pub const OLLAMA_MODELS: &[&str] = &["gemma3:1b", "gemma3:4b", "gemma4:e2b"];

// defaults
pub const DEFAULT_SUMMARIZATION_MODEL: &str = "gemma3:4b";
pub const DEFAULT_WRITER_MODEL: &str = "gemma3:1b";
pub const DEFAULT_CONTROL_MODEL: &str = "gemma4:e2b";
