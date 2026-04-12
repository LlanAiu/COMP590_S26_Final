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

    fn find_directory_for_id_recursive(&self, id: &str) -> Option<PathBuf> {
        if !self.base.exists() {
            return None;
        }
        let mut stack = vec![self.base.clone()];
        while let Some(dir) = stack.pop() {
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    if let Ok(ft) = e.file_type() {
                        if ft.is_dir() {
                            if let Some(name) = e.file_name().to_str() {
                                if name.starts_with(id) {
                                    return Some(e.path());
                                }
                            }
                            stack.push(e.path());
                        }
                    }
                }
            }
        }
        None
    }

    pub fn atomic_write(path: &Path, data: &[u8]) -> std::io::Result<()> {
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
                parent: None,
                keypoints: None,
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
            let dir = match db.find_directory_for_id_recursive(&id) {
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
            let dir = db
                .find_directory_for_id_recursive(&id)
                .ok_or(VolumeError::NotFound)?;

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
            let dir = db
                .find_directory_for_id_recursive(&id)
                .ok_or(VolumeError::NotFound)?;

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

    async fn nest_volume(&self, parent_id: &str, child_id: &str) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let parent_id = parent_id.to_string();
        let child_id = child_id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            if parent_id == child_id {
                return Err(VolumeError::Other(
                    "cannot nest a volume into itself".into(),
                ));
            }

            let parent_dir = db
                .find_directory_for_id_recursive(&parent_id)
                .ok_or(VolumeError::NotFound)?;
            let child_dir = db
                .find_directory_for_id_recursive(&child_id)
                .ok_or(VolumeError::NotFound)?;

            if parent_dir.starts_with(&child_dir) {
                return Err(VolumeError::Other("cannot nest into descendant".into()));
            }

            let sub_dir = parent_dir.join("subvolumes");
            fs::create_dir_all(&sub_dir)?;

            let name = child_dir
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or(VolumeError::Other("invalid child directory name".into()))?;
            let mut dest = sub_dir.join(name);
            if dest.exists() {
                let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
                dest = sub_dir.join(format!("{}-{}", name, ts));
            }

            fs::rename(&child_dir, &dest)?;

            let meta_path = dest.join(META_FILE);
            let meta_str = fs::read_to_string(&meta_path)?;
            let mut meta: VolumeMeta = serde_json::from_str(&meta_str)?;
            meta.parent = Some(parent_id.clone());
            meta.updated_at = Utc::now().to_rfc3339();
            meta.version = meta.version.saturating_add(1);

            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

            let content = fs::read_to_string(dest.join(CONTENT_FILE))?;

            let mut attachments = vec![];
            let attach_dir = dest.join(ATTACHMENTS_DIR);
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

    async fn merge_volumes(
        &self,
        a_id: &str,
        b_id: &str,
        req: CreateVolumeRequest,
    ) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let a_id = a_id.to_string();
        let b_id = b_id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };

            let a_dir = db
                .find_directory_for_id_recursive(&a_id)
                .ok_or(VolumeError::NotFound)?;
            let b_dir = db
                .find_directory_for_id_recursive(&b_id)
                .ok_or(VolumeError::NotFound)?;

            // create temp dir for the new merged volume
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

            // copy attachments from both sources into new attachments dir
            let mut attachments = vec![];
            let dst_attach = temp_dir.join(ATTACHMENTS_DIR);
            let mut copy_from = |src: &PathBuf, prefix: &str| {
                let attach_dir = src.join(ATTACHMENTS_DIR);
                if attach_dir.exists() {
                    if let Ok(rd) = fs::read_dir(&attach_dir) {
                        for e in rd.flatten() {
                            if let Some(name) = e.file_name().to_str() {
                                let dst_name = format!("{}-{}", prefix, name);
                                let dst_path = dst_attach.join(&dst_name);
                                let _ = fs::copy(e.path(), &dst_path)?;
                                attachments.push(dst_name);
                            }
                        }
                    }
                }
                Ok::<(), std::io::Error>(())
            };

            copy_from(&a_dir, &a_id)?;
            copy_from(&b_dir, &b_id)?;

            let now = Utc::now().to_rfc3339();
            let meta = VolumeMeta {
                id: id.clone(),
                title: req.title.clone(),
                description: req.description.clone(),
                created_at: now.clone(),
                updated_at: now.clone(),
                tags: req.tags.clone(),
                parent: None,
                keypoints: None,
                version: 1,
                deleted: false,
            };

            // write content and meta
            let content_path = temp_dir.join(CONTENT_FILE);
            FileDatabase::atomic_write(&content_path, req.content.as_bytes())?;

            let meta_path = temp_dir.join(META_FILE);
            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

            fs::rename(&temp_dir, &final_dir)?;

            // move originals to trash
            let trash_dir = base.join(TRASH_DIR);
            fs::create_dir_all(&trash_dir)?;
            let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
            if let Some(name) = a_dir.file_name().and_then(|s| s.to_str()) {
                let dest = trash_dir.join(format!("{}-{}", name, ts));
                let _ = fs::rename(&a_dir, &dest)?;
            }
            if let Some(name) = b_dir.file_name().and_then(|s| s.to_str()) {
                let dest = trash_dir.join(format!("{}-{}", name, ts));
                let _ = fs::rename(&b_dir, &dest)?;
            }

            Ok(Volume {
                meta,
                content: req.content,
                attachments,
            })
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn split_volume(
        &self,
        id: &str,
        first: CreateVolumeRequest,
        second: CreateVolumeRequest,
    ) -> Result<Vec<Volume>, VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<Vec<Volume>, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = db
                .find_directory_for_id_recursive(&id)
                .ok_or(VolumeError::NotFound)?;

            // read attachments from original
            let mut original_attachments = vec![];
            let attach_dir = dir.join(ATTACHMENTS_DIR);
            if attach_dir.exists() {
                if let Ok(rd) = fs::read_dir(&attach_dir) {
                    for e in rd.flatten() {
                        if let Some(name) = e.file_name().to_str() {
                            original_attachments.push((e.path(), name.to_string()));
                        }
                    }
                }
            }

            let create_new =
                |req: CreateVolumeRequest, prefix: &str| -> Result<Volume, VolumeError> {
                    fs::create_dir_all(&base)?;
                    let new_id = Uuid::new_v4().to_string();
                    let slug = FileDatabase::sanitize_slug(&req.title);
                    let dir_name = if slug.is_empty() {
                        new_id.clone()
                    } else {
                        format!("{}-{}", new_id, slug)
                    };
                    let temp_dir = base.join(format!("{}.tmp", &new_id));
                    let final_dir = base.join(&dir_name);

                    fs::create_dir_all(&temp_dir)?;
                    fs::create_dir_all(temp_dir.join(ATTACHMENTS_DIR))?;

                    // copy attachments into new attachments dir (prefix to avoid collisions)
                    let mut attachments = vec![];
                    for (src_path, name) in &original_attachments {
                        let dst_name = format!("{}-{}", prefix, name);
                        let dst_path = temp_dir.join(ATTACHMENTS_DIR).join(&dst_name);
                        let _ = fs::copy(src_path, &dst_path)?;
                        attachments.push(dst_name);
                    }

                    let now = Utc::now().to_rfc3339();
                    let meta = VolumeMeta {
                        id: new_id.clone(),
                        title: req.title.clone(),
                        description: req.description.clone(),
                        created_at: now.clone(),
                        updated_at: now.clone(),
                        tags: req.tags.clone(),
                        parent: None,
                        keypoints: None,
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
                        attachments,
                    })
                };

            let v1 = create_new(first, &format!("{}-a", id))?;
            let v2 = create_new(second, &format!("{}-b", id))?;

            // move original to trash
            let trash_dir = base.join(TRASH_DIR);
            fs::create_dir_all(&trash_dir)?;
            let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
            if let Some(name) = dir.file_name().and_then(|s| s.to_str()) {
                let dest = trash_dir.join(format!("{}-{}", name, ts));
                let _ = fs::rename(&dir, &dest)?;
            }

            Ok(vec![v1, v2])
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }

    async fn flatten_volume(&self, id: &str) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = db
                .find_directory_for_id_recursive(&id)
                .ok_or(VolumeError::NotFound)?;

            let meta_path = dir.join(META_FILE);
            let meta_str = fs::read_to_string(&meta_path)?;
            let mut meta: VolumeMeta = serde_json::from_str(&meta_str)?;

            if meta.parent.is_none() {
                // already top-level
                let content = fs::read_to_string(dir.join(CONTENT_FILE))?;
                let mut attachments = vec![];
                let attach_dir = dir.join(ATTACHMENTS_DIR);
                if attach_dir.exists() {
                    for entry in
                        fs::read_dir(attach_dir).unwrap_or_else(|_| fs::read_dir(".").unwrap())
                    {
                        if let Ok(e) = entry {
                            if let Some(name) = e.file_name().to_str() {
                                attachments.push(name.to_string());
                            }
                        }
                    }
                }
                return Ok(Volume {
                    meta,
                    content,
                    attachments,
                });
            }

            // move to base
            let name = dir
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or(VolumeError::Other("invalid directory name".into()))?;
            let mut dest = base.join(name);
            if dest.exists() {
                let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
                dest = base.join(format!("{}-{}", name, ts));
            }

            fs::rename(&dir, &dest)?;

            meta.parent = None;
            meta.updated_at = Utc::now().to_rfc3339();
            meta.version = meta.version.saturating_add(1);

            let meta_path = dest.join(META_FILE);
            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

            let content = fs::read_to_string(dest.join(CONTENT_FILE))?;

            let mut attachments = vec![];
            let attach_dir = dest.join(ATTACHMENTS_DIR);
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

    async fn list_index(&self) -> Result<Vec<VolumeIndexEntry>, VolumeError> {
        let base = self.base.clone();
        spawn_blocking(move || -> Result<Vec<VolumeIndexEntry>, VolumeError> {
            let mut out = vec![];
            if !base.exists() {
                return Ok(out);
            }

            let mut stack = vec![base.clone()];
            while let Some(dir) = stack.pop() {
                if let Ok(rd) = fs::read_dir(&dir) {
                    for e in rd.flatten() {
                        if let Ok(ft) = e.file_type() {
                            if ft.is_dir() {
                                // skip the trash directory entirely
                                if let Some(name) = e.file_name().to_str() {
                                    if name == TRASH_DIR {
                                        continue;
                                    }
                                }
                                // push for recursion
                                stack.push(e.path());
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
                                    .and_then(|s| {
                                        Some(s.lines().take(3).collect::<Vec<_>>().join(" "))
                                    });
                                out.push(VolumeIndexEntry {
                                    id: meta.id.clone(),
                                    title: meta.title.clone(),
                                    updated_at: meta.updated_at.clone(),
                                    snippet,
                                    description: meta.description.clone(),
                                    parent: meta.parent.clone(),
                                });
                            }
                        }
                    }
                }
            }
            Ok(out)
        })
        .await
        .map_err(|e| VolumeError::Other(format!("JoinError: {}", e)))?
    }
}

impl FileDatabase {
    pub async fn set_keypoints(
        &self,
        id: &str,
        keypoints: Vec<String>,
    ) -> Result<Volume, VolumeError> {
        let base = self.base.clone();
        let id = id.to_string();
        spawn_blocking(move || -> Result<Volume, VolumeError> {
            let db = FileDatabase { base: base.clone() };
            let dir = db.find_directory_for_id(&id).ok_or(VolumeError::NotFound)?;

            let meta_path = dir.join(META_FILE);
            let content_path = dir.join(CONTENT_FILE);

            let meta_str = fs::read_to_string(&meta_path)?;
            let mut meta: VolumeMeta = serde_json::from_str(&meta_str)?;

            meta.keypoints = Some(keypoints);
            meta.updated_at = Utc::now().to_rfc3339();
            meta.version = meta.version.saturating_add(1);

            let meta_json = serde_json::to_vec_pretty(&meta)?;
            FileDatabase::atomic_write(&meta_path, &meta_json)?;

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
}
