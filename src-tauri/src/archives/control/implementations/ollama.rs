use std::sync::Arc;

use chrono::Utc;

use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;

use crate::archives::control::constants::OLLAMA_MODEL;
use crate::archives::control::types::{ControlAction, ControlError};
use crate::archives::summarization::summary::Summary;
use crate::archives::volumes::implementations::file_database::FileDatabase;
use crate::archives::volumes::VolumeDatabase;

/// Ollama-backed controller. It sends a prompt describing the `Summary` and
/// available volumes, and expects a JSON array of `ControlAction` objects.
pub struct OllamaController {
    ollama: Arc<Ollama>,
    model: String,
}

impl OllamaController {
    pub fn new(model: Option<String>) -> OllamaController {
        let ollama = Ollama::default();
        OllamaController {
            ollama: Arc::new(ollama),
            model: model.unwrap_or_else(|| OLLAMA_MODEL.to_string()),
        }
    }

    pub async fn interpret(
        &self,
        summary: &Summary,
        volumes: &[crate::archives::volumes::types::VolumeIndexEntry],
    ) -> Result<Vec<ControlAction>, ControlError> {
        let notes_json = serde_json::to_string(&summary.notes)
            .map_err(|e| ControlError::ParseError(e.to_string()))?;

        // build a compact list of existing volumes (id + title) to help the model
        let mut vols = String::new();
        for v in volumes.iter() {
            vols.push_str(&format!("- id: {} title: {}\n", v.id, v.title));
        }

        let prompt = format!(
            r#"You are a control agent that receives a list of notes extracted from an audio transcript and a list of existing volumes (id + title). Return a JSON array of actions to perform on the volumes database. Allowed action objects (each object must include a `type` field):

- Create: {{"type":"create","req":{{"title":"...","content":"...","description":"...","tags":[...]}}}}
- Nest:   {{"type":"nest","parent_id":"<existing id>","child_id":"<existing id>"}}
- Flatten:{{"type":"flatten","id":"<existing id>"}}
- Merge:  {{"type":"merge","a_id":"<existing id>","b_id":"<existing id>","req":{{...}}}}
- Split:  {{"type":"split","id":"<existing id>","first":{{...}},"second":{{...}}}}

Only reference existing volumes by id. When creating or merging/splitting, the `req` objects fully determine the new volume metadata and content. The model should return valid JSON only (no surrounding markdown fences). Use the following inputs:

notes: {notes}
existing_volumes:
{vols}

Return a JSON array of action objects."#,
            notes = notes_json,
            vols = vols
        );

        let gen_req = GenerationRequest::new(self.model.clone(), prompt);
        let res = self
            .ollama
            .generate(gen_req)
            .await
            .map_err(|e| ControlError::OllamaError(e.to_string()))?;
        let response = res.response;

        // try to parse the response as JSON array of ControlAction
        let actions: Vec<ControlAction> =
            serde_json::from_str(&response).map_err(|e| ControlError::ParseError(e.to_string()))?;
        Ok(actions)
    }

    /// Apply actions against the provided FileDatabase. Returns a vector of
    /// human-readable results per action (created volume ids or updated ids).
    pub fn apply_actions(
        &self,
        db: Arc<FileDatabase>,
        actions: Vec<ControlAction>,
    ) -> Result<Vec<String>, ControlError> {
        let mut results: Vec<String> = vec![];
        let mut log_entries: Vec<crate::archives::control::types::ControlLogEntry> = vec![];
        for action in actions.into_iter() {
            match action {
                ControlAction::Create { req } => {
                    let created = tauri::async_runtime::block_on(db.create_volume(req))
                        .map_err(|e| ControlError::ActionError(e.to_string()))?;
                    let desc = format!("created:{}", created.meta.id);
                    results.push(desc.clone());
                    log_entries.push(crate::archives::control::types::ControlLogEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        description: desc,
                    });
                }
                ControlAction::Nest {
                    parent_id,
                    child_id,
                } => {
                    let updated =
                        tauri::async_runtime::block_on(db.nest_volume(&parent_id, &child_id))
                            .map_err(|e| ControlError::ActionError(e.to_string()))?;
                    let desc = format!("nested:{}->{}", parent_id, updated.meta.id);
                    results.push(desc.clone());
                    log_entries.push(crate::archives::control::types::ControlLogEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        description: desc,
                    });
                }
                ControlAction::Flatten { id } => {
                    let updated = tauri::async_runtime::block_on(db.flatten_volume(&id))
                        .map_err(|e| ControlError::ActionError(e.to_string()))?;
                    let desc = format!("flattened:{}", updated.meta.id);
                    results.push(desc.clone());
                    log_entries.push(crate::archives::control::types::ControlLogEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        description: desc,
                    });
                }
                ControlAction::Merge { a_id, b_id, req } => {
                    let created =
                        tauri::async_runtime::block_on(db.merge_volumes(&a_id, &b_id, req))
                            .map_err(|e| ControlError::ActionError(e.to_string()))?;
                    let desc = format!("merged:{}+{}->{}", a_id, b_id, created.meta.id);
                    results.push(desc.clone());
                    log_entries.push(crate::archives::control::types::ControlLogEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        description: desc,
                    });
                }
                ControlAction::Split { id, first, second } => {
                    let created =
                        tauri::async_runtime::block_on(db.split_volume(&id, first, second))
                            .map_err(|e| ControlError::ActionError(e.to_string()))?;
                    let ids = created
                        .into_iter()
                        .map(|v| v.meta.id)
                        .collect::<Vec<_>>()
                        .join(",");
                    let desc = format!("split:{}->{}", id, ids);
                    results.push(desc.clone());
                    log_entries.push(crate::archives::control::types::ControlLogEntry {
                        timestamp: Utc::now().to_rfc3339(),
                        description: desc,
                    });
                }
            }
        }

        // persist log entries next to the volumes base (auto-archives/control_log.json)
        if !log_entries.is_empty() {
            if let Some(base) = db.base.parent() {
                let root = base.to_path_buf();
                let log_path = root.join("control_log.json");
                // read existing
                let mut existing: Vec<crate::archives::control::types::ControlLogEntry> =
                    if log_path.exists() {
                        match std::fs::read_to_string(&log_path) {
                            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| vec![]),
                            Err(_) => vec![],
                        }
                    } else {
                        vec![]
                    };
                existing.extend(log_entries.into_iter());
                if let Ok(serialized) = serde_json::to_vec_pretty(&existing) {
                    let _ = FileDatabase::atomic_write(&log_path, &serialized);
                }
            }
        }

        Ok(results)
    }
}
