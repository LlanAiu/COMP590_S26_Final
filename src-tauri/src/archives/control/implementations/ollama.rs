use std::sync::Arc;

use chrono::Utc;

use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use serde_json::Value;

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
        volumes_ai: &[(String, String)],
    ) -> Result<Vec<ControlAction>, ControlError> {
        let notes_json = serde_json::to_string(&summary.notes)
            .map_err(|e| ControlError::ParseError(e.to_string()))?;

        // build a compact list of existing volumes (id + title + optional AI summary)
        let mut vols = String::new();
        for v in volumes.iter() {
            let mut line = format!("- id: {} title: {}", v.id, v.title);
            if let Some((_, ai)) = volumes_ai.iter().find(|(vid, _)| vid == &v.id) {
                line.push_str(&format!(" ai_summary: {}", ai));
            }
            line.push('\n');
            vols.push_str(&line);
        }

        let prompt = format!(
            r#"You are a control agent that receives a list of notes extracted from an audio transcript and a list of existing volumes (id + title). Return a JSON array of actions to perform on the volumes database. Allowed action objects (each object must include a `type` field):

- Create: {{"type":"create","req":{{"title":"...","content":"...","description":"...","tags":[...]}}}}
- Nest:   {{"type":"nest","parent_id":"<existing id>","child_id":"<existing id>"}}
- Flatten:{{"type":"flatten","id":"<existing id>"}}
- Merge:  {{"type":"merge","a_id":"<existing id>","b_id":"<existing id>","req":{{"title":"...","content":"...",...}}}}
- Split:  {{"type":"split","id":"<existing id>","first":{{"title":"...","content":"...",...}},"second":{{...}}}}

Important constraints:
- Only perform Create/Merge/Split actions when there is clear organizational agreement (for example, at least two notes share the same category/title or the summary explicitly instructs creating/merging). Do not create volumes for every single note.
- Every `req` object used for creating a new volume MUST include `title` and `content`. If you cannot produce a concise `title`, return a suggestion object (do not perform the create). The system may automatically derive a title from `content` only when the title is explicitly missing but content is present; prefer concise titles (<=8 words).
- Always reference existing volumes by exact `id` when using `nest`, `flatten`, `merge`, or `split`.
- Return strictly valid JSON (no markdown fences, no surrounding commentary). If you cannot determine safe actions, return an empty JSON array `[]`.

Use the following inputs:

notes: {notes}
existing_volumes:
{vols}

Return a JSON array of action objects. If no actions are appropriate, please return an empty array"#,
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

        // Log the raw response for debugging
        println!("[CONTROL][OLLAMA] raw response:\n{}", response);

        // Try to sanitize common assistant formatting (remove ``` fences or surrounding text)
        let mut processed = response.trim().to_string();
        if processed.starts_with("```") {
            if let Some(pos) = processed.find('\n') {
                processed = processed[pos + 1..].to_string();
            }
            if processed.ends_with("```") {
                if let Some(pos) = processed.rfind("```") {
                    processed = processed[..pos].to_string();
                }
            }
            processed = processed.trim().to_string();
        }

        // If response contains a JSON array somewhere, extract the first `[...]` span
        if let (Some(start), Some(end)) = (processed.find('['), processed.rfind(']')) {
            if start < end {
                let candidate = &processed[start..=end];
                println!("[CONTROL][OLLAMA] extracted JSON candidate:\n{}", candidate);
                match serde_json::from_str::<Vec<ControlAction>>(candidate) {
                    Ok(actions) => return Ok(actions),
                    Err(err) => {
                        eprintln!(
                            "[CONTROL][PARSER] failed to parse extracted candidate: {:?}",
                            err
                        );
                        // fallthrough to try parsing the original/sanitized string below
                    }
                }
            }
        }

        // try to parse the processed text (after fence trimming)
        match serde_json::from_str::<Vec<ControlAction>>(&processed) {
            Ok(actions) => Ok(actions),
            Err(err) => {
                eprintln!("[CONTROL][PARSER] parse error: {:?}", err);
                eprintln!("[CONTROL][PARSER] sanitized response was:\n{}", processed);
                // Fallback: attempt to parse as generic JSON and fix missing titles where safe.
                match serde_json::from_str::<Value>(&processed) {
                    Ok(Value::Array(mut arr)) => {
                        let mut modified = false;
                        for item in arr.iter_mut() {
                            if let Some(obj) = item.as_object_mut() {
                                if let Some(req) = obj.get_mut("req") {
                                    if let Some(req_obj) = req.as_object_mut() {
                                        let has_title = req_obj.get("title").is_some();
                                        let has_content = req_obj
                                            .get("content")
                                            .and_then(|v| v.as_str())
                                            .is_some();
                                        if !has_title && has_content {
                                            if let Some(content) =
                                                req_obj.get("content").and_then(|v| v.as_str())
                                            {
                                                // derive a short title (first up to 8 words)
                                                let title = content
                                                    .split_whitespace()
                                                    .take(8)
                                                    .collect::<Vec<_>>()
                                                    .join(" ");
                                                req_obj.insert(
                                                    "title".to_string(),
                                                    Value::String(title),
                                                );
                                                modified = true;
                                                println!("[CONTROL][PARSER] injected derived title for action: {}", req_obj.get("title").unwrap());
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if modified {
                            if let Ok(fixed_str) = serde_json::to_string(&arr) {
                                match serde_json::from_str::<Vec<ControlAction>>(&fixed_str) {
                                    Ok(actions) => return Ok(actions),
                                    Err(err2) => eprintln!(
                                        "[CONTROL][PARSER] still failed after fixes: {:?}",
                                        err2
                                    ),
                                }
                            }
                        }
                    }
                    _ => {}
                }

                // return parse error with original error message
                Err(ControlError::ParseError(err.to_string()))
            }
        }
    }

    /// Extract short keypoints from the provided text. Returns a vector of
    /// short strings (keypoints). Uses the same Ollama client and expects a
    /// JSON array of strings as the model output.
    pub async fn extract_keypoints(&self, text: &str) -> Result<Vec<String>, ControlError> {
        let prompt = format!(
            r#"Extract up to 8 concise, relevant keypoints from the following text. Return the result as a JSON array of short strings (no surrounding commentary). Each keypoint should be a brief actionable bullet starting with a verb (present tense), e.g. "Start", "Add", "Remove". Only include important, relevant facts — omit side-comments and filler. Example: ["Start X","Add Y",...]. Text:

{}"#,
            text = text
        );

        let gen_req = GenerationRequest::new(self.model.clone(), prompt);
        let res = self
            .ollama
            .generate(gen_req)
            .await
            .map_err(|e| ControlError::OllamaError(e.to_string()))?;
        let response = res.response;

        println!("[CONTROL][OLLAMA] raw keypoints response:\n{}", response);

        let mut processed = response.trim().to_string();
        if processed.starts_with("```") {
            if let Some(pos) = processed.find('\n') {
                processed = processed[pos + 1..].to_string();
            }
            if processed.ends_with("```") {
                if let Some(pos) = processed.rfind("```") {
                    processed = processed[..pos].to_string();
                }
            }
            processed = processed.trim().to_string();
        }

        if let (Some(start), Some(end)) = (processed.find('['), processed.rfind(']')) {
            if start < end {
                let candidate = &processed[start..=end];
                if let Ok(points) = serde_json::from_str::<Vec<String>>(candidate) {
                    return Ok(points);
                }
            }
        }

        match serde_json::from_str::<Vec<String>>(&processed) {
            Ok(points) => Ok(points),
            Err(err) => {
                eprintln!("[CONTROL][PARSER] keypoints parse error: {:?}", err);
                Err(ControlError::ParseError(err.to_string()))
            }
        }
    }

    /// Generate a short AI-only summary for a volume's content. Returns a short
    /// paragraph (single string). The model should return only the summary text
    /// with no surrounding fences or commentary.
    pub async fn generate_ai_summary(&self, text: &str) -> Result<String, ControlError> {
        let prompt = format!(
            r#"Produce a concise AI-only summary (1-3 sentences) that captures the essential context, purpose, and important details of the following text. Be focused and relevant; do not repeat small talk, filler, or off-topic commentary. Return only the summary text with no markdown or commentary. Text:

{}"#,
            text = text
        );

        let gen_req = GenerationRequest::new(self.model.clone(), prompt);
        let res = self
            .ollama
            .generate(gen_req)
            .await
            .map_err(|e| ControlError::OllamaError(e.to_string()))?;
        let mut response = res.response.trim().to_string();

        // strip fences if present
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
                    println!("[CONTROL][APPLY] creating volume: {}", req.title);
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
                    println!(
                        "[CONTROL][APPLY] nesting child {} into parent {}",
                        child_id, parent_id
                    );
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
                    println!("[CONTROL][APPLY] flattening {}", id);
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
                    println!(
                        "[CONTROL][APPLY] merging {} + {} -> new '{}'",
                        a_id, b_id, req.title
                    );
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
                    println!(
                        "[CONTROL][APPLY] splitting {} into '{}' and '{}'",
                        id, first.title, second.title
                    );
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
