use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a photo with metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Photo {
    pub uuid: Uuid,
    pub quail_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub thumbnail_small_path: Option<String>,
    pub thumbnail_medium_path: Option<String>,
    pub sync_status: Option<String>,
    pub sync_error: Option<String>,
    pub retry_count: Option<i32>,
}

/// Size variants for photo retrieval
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    Small,    // 128px WebP for lists
    Medium,   // 512px WebP for detail views
    Original, // Full size JPG
}

/// Result of photo retrieval operation
#[derive(Debug, Clone, PartialEq)]
pub enum PhotoResult {
    Available(Vec<u8>),
    Downloading,
    Failed(String, i32), // (error message, retry_count)
}

/// Configuration for photo gallery initialization
#[derive(Debug, Clone)]
pub struct PhotoGalleryConfig {
    /// Base directory for photo storage
    pub storage_path: String,
    /// Database connection (will be passed as reference)
    pub enable_thumbnails: bool,
    /// Thumbnail sizes configuration
    pub thumbnail_small_size: u32,
    pub thumbnail_medium_size: u32,
}

impl Default for PhotoGalleryConfig {
    fn default() -> Self {
        Self {
            storage_path: String::new(),
            enable_thumbnails: true,
            thumbnail_small_size: 128,
            thumbnail_medium_size: 512,
        }
    }
}
