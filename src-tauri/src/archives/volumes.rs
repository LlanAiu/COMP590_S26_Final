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

    // Merge two volumes into a new volume. The provided `req` fully determines the
    // metadata and content of the resulting merged volume. Returns the new volume.
    async fn merge_volumes(
        &self,
        a_id: &str,
        b_id: &str,
        req: CreateVolumeRequest,
    ) -> Result<Volume, VolumeError>;

    // Split a single volume into two new volumes. Each `CreateVolumeRequest` fully
    // determines the metadata and content of the resulting volumes. Returns the
    // two created volumes.
    async fn split_volume(
        &self,
        id: &str,
        first: CreateVolumeRequest,
        second: CreateVolumeRequest,
    ) -> Result<Vec<Volume>, VolumeError>;
}
