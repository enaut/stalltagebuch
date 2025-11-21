use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhotoSize {
    Small,   // 128px WebP for lists
    Medium,  // 512px WebP for detail views
    Original, // Full size JPG
}

#[derive(Debug, Clone, PartialEq)]
pub enum PhotoResult {
    Available(Vec<u8>),
    Downloading,
    Failed(String, i32), // (error message, retry_count)
}
