// builtin

// external
use async_trait::async_trait;

// internal
use crate::error::VolumeError;
use types::{CreateVolumeRequest, UpdateVolumeRequest, Volume, VolumeIndexEntry};

// modules
pub mod constants;
pub mod implementations;
pub mod types;

#[async_trait]
pub trait VolumeDatabase: Send + Sync {
    async fn create_volume(&self, req: CreateVolumeRequest) -> Result<Volume, VolumeError>;

    async fn read_volume(&self, id: &str) -> Result<Volume, VolumeError>;

    async fn edit_volume(&self, id: &str, req: UpdateVolumeRequest) -> Result<Volume, VolumeError>;

    async fn delete_volume(&self, id: &str) -> Result<(), VolumeError>;

    async fn list_index(&self) -> Result<Vec<VolumeIndexEntry>, VolumeError>;

    // Make `child_id` a sub-volume of `parent_id`. Returns the updated child volume.
    async fn nest_volume(&self, parent_id: &str, child_id: &str) -> Result<Volume, VolumeError>;

    // Remove a volume from its parent (if any), moving it to the top-level. Returns the updated volume.
    async fn flatten_volume(&self, id: &str) -> Result<Volume, VolumeError>;
}
