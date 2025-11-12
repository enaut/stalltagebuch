use serde::{Deserialize, Serialize};

/// Synchronization settings for Nextcloud/WebDAV
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncSettings {
    pub id: i64,
    pub server_url: String,
    pub username: String,
    pub app_password: String,
    pub remote_path: String,
    pub enabled: bool,
    pub last_sync: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SyncSettings {
    pub fn new(
        server_url: String,
        username: String,
        app_password: String,
        remote_path: String,
    ) -> Self {
        Self {
            id: 0,
            server_url,
            username,
            app_password,
            remote_path,
            enabled: true,
            last_sync: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
