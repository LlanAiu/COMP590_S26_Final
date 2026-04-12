// builtin

// external
use serde::{Deserialize, Serialize};

// internal

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolumeMeta {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub parent: Option<String>,
    #[serde(default)]
    pub keypoints: Option<Vec<String>>,
    pub version: u64,
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Volume {
    pub meta: VolumeMeta,
    pub content: String,
    pub attachments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateVolumeRequest {
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateVolumeRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub version: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VolumeIndexEntry {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub snippet: Option<String>,
    pub description: Option<String>,
    pub parent: Option<String>,
}
