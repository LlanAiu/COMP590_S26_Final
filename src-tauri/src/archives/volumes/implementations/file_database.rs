// builtin
use std::fs;
use std::path::{Path, PathBuf};

// external
use chrono::Utc;
use tauri::async_runtime::spawn_blocking;
use uuid::Uuid;

// internal
use crate::archives::volumes::constants::{ATTACHMENTS_DIR, CONTENT_FILE, META_FILE, TRASH_DIR};
use crate::archives::volumes::types::*;
use crate::archives::volumes::VolumeDatabase;
use crate::error::VolumeError;

pub struct FileDatabase {
    pub base: PathBuf,
}

impl FileDatabase {
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    fn sanitize_slug(title: &str) -> String {
        let s = title.to_lowercase();
        s.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }

    fn find_directory_for_id(&self, id: &str) -> Option<PathBuf> {
        if !self.base.exists() {
            return None;
        }
        let rd = match fs::read_dir(&self.base) {
            Ok(r) => r,
            Err(_) => return None,
        };
        for e in rd.flatten() {
            if let Ok(ft) = e.file_type() {
                if ft.is_dir() {
                    if let Some(name) = e.file_name().to_str() {
                        if name.starts_with(id) {
                            return Some(e.path());
                        }
                    }
                }
            }
        }
        None
    }

    fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
        let tmp = path.with_extension("tmp");
        if let Some(parent) = tmp.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&tmp, data)?;
        fs::rename(&tmp, path)?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl VolumeDatabase for FileDatabase {
    async fn create_volume(&self, req: CreateVolumeRequest) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            fs::create_dir_all(&base)?;

            let id = Uuid::new_v4().to_string();
            let slug = FileDatabase::sanitize_slug(&req.title);
            let dir_name = if slug.is_empty() {
                id.clone()
            } else {
                format!("{}-{}", id, slug)
            };
            let temp_dir = base.join(format!("{}.tmp", &id));
            let final_dir = base.join(&dir_name);

            fs::create_dir_all(&temp_dir)?;
            fs::create_dir_all(temp_dir.join(ATTACHMENTS_DIR))?;

            let now = Utc::now().to_rfc3339();
            let meta = VolumeMeta {
                id: id.clone(),
                title: req.title.clone(),
                description: req.description.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
                tags: req.tags.clone(),
                version: 1,
                deleted: false,
            };

            let content_path = temp_dir.join(CONTENT_FILE);
            FileDatabase::atomic_write(&content_path, req.content.as_bytes())?;

            let meta_path = temp_dir.join(META_FILE);
            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

            fs::rename(&temp_dir, &final_dir)?;

            Ok(Volume {
                meta,
                content: req.content,
                attachments: vec![],
            })
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn read_volume(&self, id: &str) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = match db.find_directory_for_id(&id) {
                Some(d) => d,
                None => return Err(VolumeError::NotFound),
            };

            let meta_path = dir.join(META_FILE);
            let content_path = dir.join(CONTENT_FILE);

            let meta_str = fs::read_to_string(&meta_path)?;
            let meta: VolumeMeta = serde_json::from_str(&meta_str)?;
            let content = fs::read_to_string(&content_path)?;

            let mut attachments = vec![];
            let attach_dir = dir.join(ATTACHMENTS_DIR);
            if attach_dir.exists() {
                for entry in fs::read_dir(attach_dir).unwrap_or_else(|_| fs::read_dir(".").unwrap())
                {
                    if let Ok(e) = entry {
                        if let Some(name) = e.file_name().to_str() {
                            attachments.push(name.to_string());
                        }
                    }
                }
            }

            Ok(Volume {
                meta,
                content,
                attachments,
            })
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn edit_volume(&self, id: &str, req: UpdateVolumeRequest) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = db.find_directory_for_id(&id).ok_or(VolumeError::NotFound)?;

            let meta_path = dir.join(META_FILE);
            let content_path = dir.join(CONTENT_FILE);

            let meta_str = fs::read_to_string(&meta_path)?;
            let mut meta: VolumeMeta = serde_json::from_str(&meta_str)?;

            if let Some(v) = req.version {
                if v != meta.version {
                    return Err(VolumeError::Conflict("version mismatch".into()));
                }
            }

            if let Some(title) = req.title.clone() {
                meta.title = title;
            }
            if let Some(desc) = req.description.clone() {
                meta.description = Some(desc);
            }
            if let Some(tags) = req.tags.clone() {
                meta.tags = tags;
            }

            if let Some(content) = req.content.clone() {
                FileDatabase::atomic_write(&content_path, content.as_bytes())?;
            }

            meta.updated_at = Utc::now().to_rfc3339();
            meta.version = meta.version.saturating_add(1);

            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

            let content = fs::read_to_string(&content_path)?;

            Ok(Volume {
                meta,
                content,
                attachments: vec![],
            })
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn delete_volume(&self, id: &str) -> Result<(), VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<(), VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = db.find_directory_for_id(&id).ok_or(VolumeError::NotFound)?;

            let trash_dir = base.join(TRASH_DIR);
            fs::create_dir_all(&trash_dir)?;
            let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
            let name = dir.file_name().and_then(|s| s.to_str()).unwrap_or("volume");
            let dest = trash_dir.join(format!("{}-{}", name, ts));
            fs::rename(&dir, &dest)?;
            Ok(())
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn list_index(&self) -> Result<Vec<VolumeIndexEntry>, VolumeError> {
        let base = self.base.clone();
        spawn_blocking(move || -> Result<Vec<VolumeIndexEntry>, VolumeError> {
            let mut out = vec![];
            if !base.exists() {
                return Ok(out);
            }
            for entry in fs::read_dir(&base)? {
                let e = entry?;
                if !e.file_type()?.is_dir() {
                    continue;
                }
                let meta_path = e.path().join(META_FILE);
                if !meta_path.exists() {
                    continue;
                }
                let meta_str = fs::read_to_string(&meta_path)?;
                let meta: VolumeMeta = serde_json::from_str(&meta_str)?;
                if meta.deleted {
                    continue;
                }
                let snippet = fs::read_to_string(e.path().join(CONTENT_FILE))
                    .ok()
                    .and_then(|s| Some(s.lines().take(3).collect::<Vec<_>>().join(" ")));
                out.push(VolumeIndexEntry {
                    id: meta.id.clone(),
                    title: meta.title.clone(),
                    updated_at: meta.updated_at.clone(),
                    snippet,
                    description: meta.description.clone(),
                });
            }
            Ok(out)
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }
}
