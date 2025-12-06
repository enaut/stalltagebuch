use crate::models::{Photo, PhotoGalleryConfig, PhotoResult, PhotoSize};
use crate::thumbnail::{rename_photo_with_uuid, ThumbnailError};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

/// Error type for photo gallery operations
#[derive(Debug)]
pub enum PhotoGalleryError {
    DatabaseError(rusqlite::Error),
    ThumbnailError(ThumbnailError),
    NotFound(String),
    IoError(std::io::Error),
    Other(String),
}

impl std::fmt::Display for PhotoGalleryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhotoGalleryError::DatabaseError(e) => write!(f, "Database error: {}", e),
            PhotoGalleryError::ThumbnailError(e) => write!(f, "Thumbnail error: {}", e),
            PhotoGalleryError::NotFound(msg) => write!(f, "Not found: {}", msg),
            PhotoGalleryError::IoError(e) => write!(f, "IO error: {}", e),
            PhotoGalleryError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for PhotoGalleryError {}

impl From<rusqlite::Error> for PhotoGalleryError {
    fn from(err: rusqlite::Error) -> Self {
        PhotoGalleryError::DatabaseError(err)
    }
}

impl From<ThumbnailError> for PhotoGalleryError {
    fn from(err: ThumbnailError) -> Self {
        PhotoGalleryError::ThumbnailError(err)
    }
}

impl From<std::io::Error> for PhotoGalleryError {
    fn from(err: std::io::Error) -> Self {
        PhotoGalleryError::IoError(err)
    }
}

/// Photo Gallery Service
pub struct PhotoGalleryService {
    config: PhotoGalleryConfig,
}

impl PhotoGalleryService {
    /// Initialize the photo gallery service with configuration
    pub fn new(config: PhotoGalleryConfig) -> Self {
        Self { config }
    }

    /// Returns the absolute path to a photo (for UI display)
    pub fn get_absolute_photo_path(&self, relative_path: &str) -> String {
        if self.config.storage_path.is_empty() {
            relative_path.to_string()
        } else {
            format!(
                "{}/{}",
                self.config.storage_path.trim_end_matches('/'),
                relative_path
            )
        }
    }

    /// Return configured thumbnail sizes (small, medium)
    pub fn thumbnail_sizes(&self) -> (u32, u32) {
        (
            self.config.thumbnail_small_size,
            self.config.thumbnail_medium_size,
        )
    }

    /// Add a photo for a quail
    ///
    /// Returns the UUID of the created photo. The caller is responsible for
    /// any additional operations like CRDT operation capture.
    pub async fn add_quail_photo(
        &self,
        conn: &Connection,
        quail_id: Uuid,
        path: String,
    ) -> Result<Uuid, PhotoGalleryError> {
        log::debug!("=== add_quail_photo called ===");
        log::debug!("Quail ID: {}, Path: {}", quail_id, path);

        // Rename photo and create multi-size thumbnails (in blocking thread)
        let (new_path, small_thumb, medium_thumb) = rename_photo_with_uuid(
            &path,
            self.config.thumbnail_small_size,
            self.config.thumbnail_medium_size,
        )
        .await?;

        let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
            .map_err(|_| PhotoGalleryError::Other("Invalid UUID from filename".to_string()))?;

        log::debug!("UUID extracted: {}", uuid);
        log::debug!("Thumbnails: small={}, medium={}", small_thumb, medium_thumb);

        conn.execute(
            "INSERT INTO photos (uuid, quail_id, path, relative_path, thumbnail_small_path, thumbnail_medium_path, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'local_only')",
            params![
                uuid.to_string(),
                quail_id.to_string(),
                "",  // path is now empty, relative_path holds the filename
                &new_path,
                &small_thumb,
                &medium_thumb,
            ],
        )?;

        Ok(uuid)
    }

    /// Add a photo for an event
    ///
    /// Returns the UUID of the created photo. The caller is responsible for
    /// any additional operations like CRDT operation capture.
    pub async fn add_event_photo(
        &self,
        conn: &Connection,
        event_id: Uuid,
        path: String,
    ) -> Result<Uuid, PhotoGalleryError> {
        // Rename photo and create multi-size thumbnails (in blocking thread)
        let (new_path, small_thumb, medium_thumb) = rename_photo_with_uuid(
            &path,
            self.config.thumbnail_small_size,
            self.config.thumbnail_medium_size,
        )
        .await?;

        let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
            .map_err(|_| PhotoGalleryError::Other("Invalid UUID from filename".to_string()))?;

        conn.execute(
            "INSERT INTO photos (uuid, event_id, path, relative_path, thumbnail_small_path, thumbnail_medium_path, sync_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'local_only')",
            params![
                uuid.to_string(),
                event_id.to_string(),
                "",  // path is now empty, relative_path holds the filename
                &new_path,
                &small_thumb,
                &medium_thumb,
            ],
        )?;

        Ok(uuid)
    }

    /// List photos for a quail
    pub fn list_quail_photos(
        &self,
        conn: &Connection,
        quail_uuid: &Uuid,
    ) -> Result<Vec<Photo>, PhotoGalleryError> {
        let mut stmt = conn.prepare(
            "SELECT uuid, quail_id, event_id, COALESCE(relative_path, path) as rel_path, thumbnail_path,
                    thumbnail_small_path, thumbnail_medium_path, sync_status, sync_error, retry_count
             FROM photos 
             WHERE quail_id = ?1 AND deleted = 0",
        )?;

        let rows = stmt.query_map(params![quail_uuid.to_string()], |row| {
            let uuid_str: String = row.get(0)?;
            let quail_id_str: Option<String> = row.get(1)?;
            let event_id_str: Option<String> = row.get(2)?;
            let relative_path: String = row.get(3)?;
            let relative_thumb: Option<String> = row.get(4)?;
            let thumbnail_small: Option<String> = row.get(5)?;
            let thumbnail_medium: Option<String> = row.get(6)?;
            let sync_status: Option<String> = row.get(7)?;
            let sync_error: Option<String> = row.get(8)?;
            let retry_count: Option<i32> = row.get(9)?;

            Ok(Photo {
                uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                quail_id: quail_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                collection_id: None,
                event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                path: self.get_absolute_photo_path(&relative_path),
                thumbnail_path: relative_thumb.map(|t| self.get_absolute_photo_path(&t)),
                thumbnail_small_path: thumbnail_small.map(|t| self.get_absolute_photo_path(&t)),
                thumbnail_medium_path: thumbnail_medium.map(|t| self.get_absolute_photo_path(&t)),
                sync_status,
                sync_error,
                retry_count,
            })
        })?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// List photos for an event
    pub fn list_event_photos(
        &self,
        conn: &Connection,
        event_uuid: &Uuid,
    ) -> Result<Vec<Photo>, PhotoGalleryError> {
        let mut stmt = conn.prepare(
            "SELECT uuid, quail_id, event_id, COALESCE(relative_path, path) as rel_path, thumbnail_path,
                    thumbnail_small_path, thumbnail_medium_path, sync_status, sync_error, retry_count
             FROM photos 
             WHERE event_id = ?1 AND deleted = 0",
        )?;

        let rows = stmt.query_map(params![event_uuid.to_string()], |row| {
            let uuid_str: String = row.get(0)?;
            let quail_id_str: Option<String> = row.get(1)?;
            let event_id_str: Option<String> = row.get(2)?;
            let relative_path: String = row.get(3)?;
            let relative_thumb: Option<String> = row.get(4)?;
            let thumbnail_small: Option<String> = row.get(5)?;
            let thumbnail_medium: Option<String> = row.get(6)?;
            let sync_status: Option<String> = row.get(7)?;
            let sync_error: Option<String> = row.get(8)?;
            let retry_count: Option<i32> = row.get(9)?;

            Ok(Photo {
                uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                quail_id: quail_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                collection_id: None,
                event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                path: self.get_absolute_photo_path(&relative_path),
                thumbnail_path: relative_thumb.map(|t| self.get_absolute_photo_path(&t)),
                thumbnail_small_path: thumbnail_small.map(|t| self.get_absolute_photo_path(&t)),
                thumbnail_medium_path: thumbnail_medium.map(|t| self.get_absolute_photo_path(&t)),
                sync_status,
                sync_error,
                retry_count,
            })
        })?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Get profile photo for a quail
    pub fn get_profile_photo(
        &self,
        conn: &Connection,
        quail_uuid: &Uuid,
    ) -> Result<Option<Photo>, PhotoGalleryError> {
        let mut stmt = conn.prepare(
            "SELECT p.uuid, p.quail_id, p.event_id, COALESCE(p.relative_path, p.path) as rel_path, p.thumbnail_path,
                    p.thumbnail_small_path, p.thumbnail_medium_path, p.sync_status, p.sync_error, p.retry_count
             FROM photos p 
             JOIN quails q ON q.profile_photo = p.uuid 
             WHERE q.uuid = ?1",
        )?;

        let res = stmt
            .query_row(params![quail_uuid.to_string()], |row| {
                let uuid_str: String = row.get(0)?;
                let quail_id_str: Option<String> = row.get(1)?;
                let event_id_str: Option<String> = row.get(2)?;
                let relative_path: String = row.get(3)?;
                let relative_thumb: Option<String> = row.get(4)?;
                let thumbnail_small: Option<String> = row.get(5)?;
                let thumbnail_medium: Option<String> = row.get(6)?;
                let sync_status: Option<String> = row.get(7)?;
                let sync_error: Option<String> = row.get(8)?;
                let retry_count: Option<i32> = row.get(9)?;

                Ok(Photo {
                    uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    quail_id: quail_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                collection_id: None,
                    event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                    path: self.get_absolute_photo_path(&relative_path),
                    thumbnail_path: relative_thumb.map(|t| self.get_absolute_photo_path(&t)),
                    thumbnail_small_path: thumbnail_small.map(|t| self.get_absolute_photo_path(&t)),
                    thumbnail_medium_path: thumbnail_medium
                        .map(|t| self.get_absolute_photo_path(&t)),
                    sync_status,
                    sync_error,
                    retry_count,
                })
            })
            .optional()?;

        Ok(res)
    }

    /// Delete a photo
    ///
    /// The caller is responsible for any additional operations like CRDT operation capture.
    pub async fn delete_photo(
        &self,
        conn: &Connection,
        photo_uuid: &Uuid,
    ) -> Result<(), PhotoGalleryError> {
        let rows = conn.execute(
            "DELETE FROM photos WHERE uuid = ?1",
            params![photo_uuid.to_string()],
        )?;

        if rows == 0 {
            return Err(PhotoGalleryError::NotFound("Photo not found".into()));
        }

        Ok(())
    }

    /// Get photo with local file check
    pub fn get_photo(
        &self,
        conn: &Connection,
        photo_uuid: &Uuid,
        size: PhotoSize,
    ) -> Result<PhotoResult, PhotoGalleryError> {
        // Query photo info from database
        let photo_info: Option<(String, Option<String>, Option<String>, Option<String>, Option<i32>)> = conn
            .query_row(
                "SELECT COALESCE(relative_path, path), thumbnail_small_path, thumbnail_medium_path, sync_status, retry_count
                 FROM photos 
                 WHERE uuid = ?1 AND deleted = 0",
                params![photo_uuid.to_string()],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .optional()?;

        let (relative_path, small_thumb, medium_thumb, sync_status, retry_count) = match photo_info
        {
            Some(info) => info,
            None => return Err(PhotoGalleryError::NotFound("Photo not found".into())),
        };

        // Determine which file to load based on size
        let file_path = match size {
            PhotoSize::Small => small_thumb.as_ref().unwrap_or(&relative_path),
            PhotoSize::Medium => medium_thumb.as_ref().unwrap_or(&relative_path),
            PhotoSize::Original => &relative_path,
        };

        let absolute_path = self.get_absolute_photo_path(file_path);

        // Check if file exists locally
        if std::path::Path::new(&absolute_path).exists() {
            match std::fs::read(&absolute_path) {
                Ok(bytes) => return Ok(PhotoResult::Available(bytes)),
                Err(e) => {
                    log::warn!("Failed to read file {}: {}", absolute_path, e);
                }
            }
        }

        // File doesn't exist locally - check sync status
        let status = sync_status.unwrap_or_else(|| "local_only".to_string());
        let retry_count = retry_count.unwrap_or(0);

        match status.as_str() {
            "downloading" => Ok(PhotoResult::Downloading),
            "download_failed" if retry_count >= 5 => Ok(PhotoResult::Failed(
                "Max retries reached".to_string(),
                retry_count,
            )),
            _ => Ok(PhotoResult::Failed(
                "Photo not available locally".to_string(),
                retry_count,
            )),
        }
    }
}
