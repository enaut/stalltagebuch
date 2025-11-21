use crate::error::AppError;
use crate::models::photo::{PhotoResult, PhotoSize};
use crate::models::Photo;
use image::{imageops::FilterType, ImageFormat};
use rusqlite::{params, Connection, OptionalExtension};
use std::io::Cursor;
use uuid::Uuid;

/// Returns the absolute path to a photo (for UI display)
pub fn get_absolute_photo_path(relative_path: &str) -> String {
    #[cfg(target_os = "android")]
    {
        // Photos are in /storage/emulated/0/Android/data/PACKAGE/files/photos/
        format!(
            "/storage/emulated/0/Android/data/de.teilgedanken.stalltagebuch/files/photos/{}",
            relative_path
        )
    }

    #[cfg(not(target_os = "android"))]
    {
        relative_path.to_string()
    }
}

/// Creates multi-size WebP thumbnails from a JPEG image
/// Returns (small_filename, medium_filename) or error
fn create_thumbnails(original_path: &str, uuid: &str) -> Result<(String, String), AppError> {
    log::debug!("Creating thumbnails for UUID: {}", uuid);

    // Load original image
    let img = image::open(original_path)
        .map_err(|e| AppError::Other(format!("Fehler beim Laden des Bildes: {}", e)))?;

    let parent_dir = std::path::Path::new(original_path)
        .parent()
        .ok_or_else(|| AppError::Other("Kein Elternverzeichnis gefunden".to_string()))?;

    // Create small thumbnail (128px, 70% quality)
    let small_filename = format!("{}_small.webp", uuid);
    let small_path = parent_dir.join(&small_filename);
    let small_img = img.resize(128, 128, FilterType::Lanczos3);

    let mut small_buffer = Cursor::new(Vec::new());
    small_img
        .write_to(&mut small_buffer, ImageFormat::WebP)
        .map_err(|e| {
            AppError::Other(format!(
                "Fehler beim Schreiben des kleinen Thumbnails: {}",
                e
            ))
        })?;

    std::fs::write(&small_path, small_buffer.into_inner()).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Speichern des kleinen Thumbnails: {}",
            e
        ))
    })?;

    log::debug!("Small thumbnail created: {:?}", small_path);

    // Create medium thumbnail (512px, 75% quality)
    let medium_filename = format!("{}_medium.webp", uuid);
    let medium_path = parent_dir.join(&medium_filename);
    let medium_img = img.resize(512, 512, FilterType::Lanczos3);

    let mut medium_buffer = Cursor::new(Vec::new());
    medium_img
        .write_to(&mut medium_buffer, ImageFormat::WebP)
        .map_err(|e| {
            AppError::Other(format!(
                "Fehler beim Schreiben des mittleren Thumbnails: {}",
                e
            ))
        })?;

    std::fs::write(&medium_path, medium_buffer.into_inner()).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Speichern des mittleren Thumbnails: {}",
            e
        ))
    })?;

    log::debug!("Medium thumbnail created: {:?}", medium_path);

    Ok((small_filename, medium_filename))
}

/// Renames a photo file with UUID and returns the new path + thumbnail names
/// Uses spawn_blocking to avoid blocking the async runtime
async fn rename_photo_with_uuid(original_path: &str) -> Result<(String, String, String), AppError> {
    let original_path = original_path.to_string();

    tokio::task::spawn_blocking(move || {
        log::debug!("=== rename_photo_with_uuid called ===");
        log::debug!("Original path: {}", original_path);

        let uuid = uuid::Uuid::new_v4().to_string();
        let new_filename = format!("{}.jpg", uuid);

        log::debug!("Generated UUID: {}", uuid);
        log::debug!("New filename: {}", new_filename);

        // File is already in the correct directory, simply rename it there
        let old_path = std::path::Path::new(&original_path);

        if let Some(parent_dir) = old_path.parent() {
            let new_path = parent_dir.join(&new_filename);

            log::debug!("Old path: {:?}", old_path);
            log::debug!("New path: {:?}", new_path);
            log::debug!("Checking if old_path exists: {}", old_path.exists());

            if old_path.exists() {
                log::debug!("Path exists, copying...");
                match std::fs::copy(old_path, &new_path) {
                    Ok(_) => {
                        log::debug!("Copy successful, removing original...");
                        if let Err(e) = std::fs::remove_file(old_path) {
                            log::warn!("Could not remove original: {}", e);
                        } else {
                            log::debug!("Original removed");
                        }

                        // Create multi-size WebP thumbnails
                        log::debug!("Creating thumbnails...");
                        let (small_thumb, medium_thumb) =
                            create_thumbnails(new_path.to_str().unwrap(), &uuid)?;

                        log::debug!("=== rename_photo_with_uuid completed ===");
                        return Ok((new_filename, small_thumb, medium_thumb));
                    }
                    Err(e) => {
                        log::error!("ERROR during copy: {}", e);
                        return Err(AppError::Other(format!("Fehler beim Kopieren: {}", e)));
                    }
                }
            } else {
                log::error!("ERROR: Original path doesn't exist!");
                return Err(AppError::Other(format!(
                    "Originaldatei nicht gefunden: {}",
                    original_path
                )));
            }
        }

        Err(AppError::Other(
            "Kein Elternverzeichnis gefunden".to_string(),
        ))
    })
    .await
    .map_err(|e| AppError::Other(format!("Task join error: {}", e)))?
}

pub async fn add_quail_photo(
    conn: &Connection,
    quail_id: Uuid,
    path: String,
    _thumbnail_path: Option<String>,
) -> Result<Uuid, AppError> {
    log::debug!("=== add_quail_photo called ===");
    log::debug!("Quail ID: {}, Path: {}", quail_id, path);

    // Rename photo and create multi-size thumbnails (in blocking thread)
    let (new_path, small_thumb, medium_thumb) = rename_photo_with_uuid(&path).await?;
    let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
        .map_err(|_| AppError::Other("Invalid UUID from filename".to_string()))?;
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

    // Capture CRDT operation
    crate::services::operation_capture::capture_photo_create(
        conn,
        &uuid.to_string(),
        Some(&quail_id.to_string()),
        None,
        &new_path,
        Some(&small_thumb),
    )
    .await?;

    Ok(uuid)
}

pub async fn add_event_photo(
    conn: &Connection,
    event_id: Uuid,
    path: String,
    _thumbnail_path: Option<String>,
) -> Result<Uuid, AppError> {
    // Rename photo and create multi-size thumbnails (in blocking thread)
    let (new_path, small_thumb, medium_thumb) = rename_photo_with_uuid(&path).await?;
    let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
        .map_err(|_| AppError::Other("Invalid UUID from filename".to_string()))?;

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

    // Capture CRDT operation
    crate::services::operation_capture::capture_photo_create(
        conn,
        &uuid.to_string(),
        None,
        Some(&event_id.to_string()),
        &new_path,
        Some(&small_thumb),
    )
    .await?;

    Ok(uuid)
}

pub fn list_quail_photos(conn: &Connection, quail_uuid: &Uuid) -> Result<Vec<Photo>, AppError> {
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
            event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
            thumbnail_small_path: thumbnail_small.map(|t| get_absolute_photo_path(&t)),
            thumbnail_medium_path: thumbnail_medium.map(|t| get_absolute_photo_path(&t)),
            sync_status,
            sync_error,
            retry_count,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_event_photos(conn: &Connection, event_uuid: &Uuid) -> Result<Vec<Photo>, AppError> {
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
            event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
            thumbnail_small_path: thumbnail_small.map(|t| get_absolute_photo_path(&t)),
            thumbnail_medium_path: thumbnail_medium.map(|t| get_absolute_photo_path(&t)),
            sync_status,
            sync_error,
            retry_count,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_profile_photo(conn: &Connection, quail_uuid: &Uuid) -> Result<Option<Photo>, AppError> {
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
                event_id: event_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
                path: get_absolute_photo_path(&relative_path),
                thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
                thumbnail_small_path: thumbnail_small.map(|t| get_absolute_photo_path(&t)),
                thumbnail_medium_path: thumbnail_medium.map(|t| get_absolute_photo_path(&t)),
                sync_status,
                sync_error,
                retry_count,
            })
        })
        .optional()?;
    Ok(res)
}

pub async fn set_profile_photo(
    conn: &Connection,
    quail_uuid: &Uuid,
    photo_uuid: &Uuid,
) -> Result<(), AppError> {
    // Verify photo belongs to quail
    let photo_quail: Option<String> = conn
        .query_row(
            "SELECT quail_id FROM photos WHERE uuid = ?1",
            params![photo_uuid.to_string()],
            |row| row.get(0),
        )
        .optional()?;

    match photo_quail {
        Some(qid) if qid == quail_uuid.to_string() => {
            // Set profile_photo FK
            let rows = conn.execute(
                "UPDATE quails SET profile_photo = ?1 WHERE uuid = ?2",
                params![photo_uuid.to_string(), quail_uuid.to_string()],
            )?;
            if rows == 0 {
                return Err(AppError::NotFound("Wachtel nicht gefunden".into()));
            }

            // Capture CRDT operation for the update
            crate::services::operation_capture::capture_quail_update(
                conn,
                &quail_uuid.to_string(),
                "profile_photo",
                serde_json::Value::String(photo_uuid.to_string()),
            )
            .await?;

            Ok(())
        }
        Some(_) => Err(AppError::NotFound("Foto gehört nicht zur Wachtel".into())),
        None => Err(AppError::NotFound("Foto nicht gefunden".into())),
    }
}

pub async fn delete_photo(conn: &Connection, photo_uuid: &Uuid) -> Result<(), AppError> {
    let rows = conn.execute(
        "DELETE FROM photos WHERE uuid = ?1",
        params![photo_uuid.to_string()],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound("Foto nicht gefunden".into()));
    }

    // Capture CRDT deletion
    crate::services::operation_capture::capture_photo_delete(conn, &photo_uuid.to_string()).await?;

    Ok(())
}

/// Get photo with on-demand download capability
/// Returns Available(bytes), Downloading, or Failed(error, retry_count)
pub async fn get_photo_with_download(
    conn: &Connection,
    photo_uuid: &Uuid,
    size: PhotoSize,
) -> Result<PhotoResult, AppError> {
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

    let (relative_path, small_thumb, medium_thumb, sync_status, retry_count) = match photo_info {
        Some(info) => info,
        None => return Err(AppError::NotFound("Foto nicht gefunden".into())),
    };

    // Determine which file to load based on size
    let file_path = match size {
        PhotoSize::Small => small_thumb.as_ref().unwrap_or(&relative_path),
        PhotoSize::Medium => medium_thumb.as_ref().unwrap_or(&relative_path),
        PhotoSize::Original => &relative_path,
    };

    let absolute_path = get_absolute_photo_path(file_path);

    // Check if file exists locally
    if std::path::Path::new(&absolute_path).exists() {
        match std::fs::read(&absolute_path) {
            Ok(bytes) => return Ok(PhotoResult::Available(bytes)),
            Err(e) => {
                log::warn!("Fehler beim Lesen der Datei {}: {}", absolute_path, e);
            }
        }
    }

    // File doesn't exist locally - check sync status
    let status = sync_status.unwrap_or_else(|| "local_only".to_string());
    let retry_count = retry_count.unwrap_or(0);

    match status.as_str() {
        "downloading" => Ok(PhotoResult::Downloading),
        "download_failed" if retry_count >= 5 => Ok(PhotoResult::Failed(
            "Maximale Anzahl an Versuchen erreicht".to_string(),
            retry_count,
        )),
        "download_failed" | "download_pending" | "synced" => {
            // Attempt download
            spawn_photo_download(conn, photo_uuid, file_path, retry_count).await
        }
        _ => Ok(PhotoResult::Failed(
            "Foto nicht remote verfügbar".to_string(),
            retry_count,
        )),
    }
}

/// Spawns a background task to download a photo
async fn spawn_photo_download(
    conn: &Connection,
    photo_uuid: &Uuid,
    relative_path: &str,
    retry_count: i32,
) -> Result<PhotoResult, AppError> {
    // Update status to downloading
    conn.execute(
        "UPDATE photos SET sync_status = 'downloading', last_sync_attempt = ?1 WHERE uuid = ?2",
        params![
            chrono::Utc::now().timestamp_millis(),
            photo_uuid.to_string()
        ],
    )?;

    let photo_uuid_clone = *photo_uuid;
    let relative_path_clone = relative_path.to_string();
    let retry_count_clone = retry_count;

    // Spawn download task
    tokio::spawn(async move {
        // Calculate backoff with Full Jitter
        if retry_count_clone > 0 {
            let base_delay = 60 * (1 << (retry_count_clone - 1).min(4)); // 60s, 120s, 240s, 480s, 960s max
            let max_delay = base_delay.min(300); // Cap at 5 minutes
            let jitter = rand::random::<u64>() % (max_delay + 1);

            log::debug!(
                "Photo download retry {} for {}: waiting {}s",
                retry_count_clone,
                photo_uuid_clone,
                jitter
            );

            tokio::time::sleep(std::time::Duration::from_secs(jitter)).await;
        }

        // Perform actual download
        match download_photo_from_remote(&photo_uuid_clone, &relative_path_clone).await {
            Ok(()) => {
                log::info!("Successfully downloaded photo: {}", photo_uuid_clone);
                // Update status to synced
                if let Ok(conn) = crate::database::init_database() {
                    let _ = conn.execute(
                        "UPDATE photos SET sync_status = 'synced', retry_count = 0, sync_error = NULL WHERE uuid = ?1",
                        params![photo_uuid_clone.to_string()],
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to download photo {}: {}", photo_uuid_clone, e);
                // Update status to failed
                if let Ok(conn) = crate::database::init_database() {
                    let _ = conn.execute(
                        "UPDATE photos SET sync_status = 'download_failed', retry_count = ?1, sync_error = ?2 WHERE uuid = ?3",
                        params![retry_count_clone + 1, e.to_string(), photo_uuid_clone.to_string()],
                    );
                }
            }
        }
    });

    Ok(PhotoResult::Downloading)
}

/// Downloads a photo from remote storage
async fn download_photo_from_remote(
    photo_uuid: &Uuid,
    relative_path: &str,
) -> Result<(), AppError> {
    // Load sync settings
    let conn = crate::database::init_database()?;
    let settings = crate::services::sync_service::load_sync_settings(&conn)?
        .ok_or_else(|| AppError::Other("Sync nicht konfiguriert".to_string()))?;

    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        settings.server_url.trim_end_matches('/'),
        settings.username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            settings.username.clone(),
            settings.app_password.clone(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV client error: {:?}", e)))?;

    let remote_path = format!("{}/sync/photos/{}", settings.remote_path, relative_path);

    // Download file
    let response = client
        .get(&remote_path)
        .await
        .map_err(|e| AppError::Other(format!("Download failed: {:?}", e)))?;

    // Convert response to bytes
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::Other(format!("Failed to read response bytes: {}", e)))?;

    // Save to local storage
    let absolute_path = get_absolute_photo_path(relative_path);
    std::fs::write(&absolute_path, bytes)
        .map_err(|e| AppError::Other(format!("Failed to save file: {}", e)))?;

    log::info!("Downloaded photo {} to {}", photo_uuid, absolute_path);
    Ok(())
}

/// Retry all failed downloads that haven't exceeded max retries
pub async fn retry_failed_downloads(conn: &Connection) -> Result<usize, AppError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, COALESCE(relative_path, path), retry_count
         FROM photos 
         WHERE sync_status = 'download_failed' AND retry_count < 5 AND deleted = 0",
    )?;

    let photos: Vec<(Uuid, String, i32)> = stmt
        .query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            Ok((
                Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                row.get(1)?,
                row.get(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let count = photos.len();
    log::info!("Retrying {} failed photo downloads", count);

    for (uuid, relative_path, retry_count) in photos {
        let _ = spawn_photo_download(conn, &uuid, &relative_path, retry_count).await;
    }

    Ok(count)
}

/// Cleanup orphaned photos (photos without valid quail_id or event_id references)
pub async fn cleanup_orphaned_photos(conn: &Connection) -> Result<usize, AppError> {
    // Find orphaned photos
    let mut stmt = conn.prepare(
        "SELECT uuid, COALESCE(relative_path, path), thumbnail_small_path, thumbnail_medium_path
         FROM photos 
         WHERE deleted = 0 AND (
            (quail_id IS NOT NULL AND quail_id NOT IN (SELECT uuid FROM quails WHERE deleted = 0))
            OR
            (event_id IS NOT NULL AND event_id NOT IN (SELECT uuid FROM quail_events WHERE deleted = 0))
         )",
    )?;

    let orphaned: Vec<(Uuid, String, Option<String>, Option<String>)> = stmt
        .query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            Ok((
                Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let count = orphaned.len();
    log::info!("Found {} orphaned photos to clean up", count);

    for (uuid, relative_path, small_thumb, medium_thumb) in orphaned {
        // Delete physical files
        let _ = std::fs::remove_file(get_absolute_photo_path(&relative_path));
        if let Some(small) = small_thumb {
            let _ = std::fs::remove_file(get_absolute_photo_path(&small));
        }
        if let Some(medium) = medium_thumb {
            let _ = std::fs::remove_file(get_absolute_photo_path(&medium));
        }

        // Mark as deleted in database
        conn.execute(
            "UPDATE photos SET deleted = 1 WHERE uuid = ?1",
            params![uuid.to_string()],
        )?;
    }

    Ok(count)
}
