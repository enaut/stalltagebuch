use crate::error::AppError;
use rusqlite::Connection;

/// Liefert stabile device_id (erzeugt & speichert falls fehlend)
pub fn get_device_id(conn: &Connection) -> Result<String, AppError> {
    use crate::services::sync_service;
    if let Some(mut settings) = sync_service::load_sync_settings(conn)? {
        if let Some(id) = &settings.device_id {
            return Ok(id.clone());
        }
        let new_id = uuid::Uuid::new_v4().to_string();
        settings.device_id = Some(new_id.clone());
        sync_service::save_sync_settings(conn, &settings)?;
        Ok(new_id)
    } else {
        // Fallback: ephemeral ID (Settings noch nicht konfiguriert)
        Ok(uuid::Uuid::new_v4().to_string())
    }
}

/// Uploads a batch of operations to sync/ops/<device>/<YYYYMM>/<ULID>.ndjson
///
/// This is a minimal skeleton for the new multi-master sync.
/// If sync is not configured or disabled, this function returns Ok() without error.
pub async fn upload_ops_batch(
    conn: &Connection,
    ops: Vec<crate::services::crdt_service::Operation>,
) -> Result<(), AppError> {
    use crate::services::{sync_paths, sync_service};

    if ops.is_empty() {
        return Ok(());
    }

    // If sync is not configured, just skip upload (app works locally)
    let settings = match sync_service::load_sync_settings(conn)? {
        Some(s) => s,
        None => {
            log::debug!("Sync not configured, skipping operation upload");
            return Ok(());
        }
    };

    // If sync is disabled, skip upload
    if !settings.enabled {
        log::debug!("Sync disabled, skipping operation upload");
        return Ok(());
    }

    let device_id = get_device_id(conn)?;
    let year_month = sync_paths::current_year_month();
    let ulid = ulid::Ulid::new().to_string();

    // Build NDJSON content
    let mut ndjson_lines = Vec::new();
    for op in &ops {
        let line = serde_json::to_string(op)
            .map_err(|e| AppError::Other(format!("JSON serialize failed: {}", e)))?;
        ndjson_lines.push(line);
    }
    let ndjson_content = ndjson_lines.join("\n") + "\n";

    // Build remote path
    let ops_dir = sync_paths::ops_path(&device_id, &year_month);
    let filename = format!("{}.ndjson", ulid);
    let full_path = format!(
        "{}/{}/{}",
        settings.remote_path.trim_end_matches('/'),
        ops_dir,
        filename
    );

    // Create WebDAV client
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

    // Create directories if needed (WebDAV cannot create nested collections in one call)
    let base = settings.remote_path.trim_end_matches('/');
    let sync_base = format!("{}/sync", base);
    let ops_base = format!("{}/ops", sync_base);
    let device_base = format!("{}/{}", ops_base, device_id);
    let month_base = format!("{}/{}", device_base, year_month);

    // Try to create each level; ignore errors like 405 Method Not Allowed or 409 Conflict
    for path in [&sync_base, &ops_base, &device_base, &month_base] {
        if let Err(e) = client.mkcol(path).await {
            // Best-effort: path may already exist; only log
            log::debug!("MKCOL '{}' note: {:?}", path, e);
        }
    }

    // Upload (atomic create via If-None-Match not directly supported, use put)
    client
        .put(&full_path, ndjson_content.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Upload ops batch failed: {:?}", e)))?;

    log::info!(
        "Uploaded ops batch: {} operations to {}",
        ops.len(),
        full_path
    );

    Ok(())
}

/// Counts how many photos are pending upload (sync_status='local_only')
pub fn count_pending_photos(conn: &Connection) -> Result<usize, AppError> {
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM photos 
         WHERE deleted = 0 AND (sync_status = 'local_only' OR sync_status IS NULL)",
        [],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Uploads binary photo files to sync/photos/ with the original image only
///
/// Only uploads photos with sync_status='local_only'. Thumbnails are no
/// longer uploaded — they are generated locally on the receiving side.
/// Uses JoinSet for parallel uploads (max 3 concurrent photos).
/// If sync is not configured or disabled, this function returns Ok(0) without error.
pub async fn upload_photos_batch(conn: &Connection) -> Result<usize, AppError> {
    use crate::services::sync_service;
    use tokio::task::JoinSet;

    // If sync is not configured, just skip upload (app works locally)
    let settings = match sync_service::load_sync_settings(conn)? {
        Some(s) => s,
        None => {
            log::debug!("Sync not configured, skipping photo upload");
            return Ok(0);
        }
    };

    // If sync is disabled, skip upload
    if !settings.enabled {
        log::debug!("Sync disabled, skipping photo upload");
        return Ok(0);
    }

    // Create WebDAV client
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        settings.server_url.trim_end_matches('/'),
        settings.username
    );

    let client = std::sync::Arc::new(
        reqwest_dav::ClientBuilder::new()
            .set_host(webdav_url)
            .set_auth(reqwest_dav::Auth::Basic(
                settings.username.clone(),
                settings.app_password.clone(),
            ))
            .build()
            .map_err(|e| AppError::Other(format!("WebDAV client error: {:?}", e)))?,
    );

    let base = settings.remote_path.trim_end_matches('/');
    let sync_base = format!("{}/sync", base);
    let photos_dir = format!("{}/photos", sync_base);

    // Create photos directory if needed
    if let Err(e) = client.mkcol(&sync_base).await {
        log::debug!("MKCOL sync note: {:?}", e);
    }
    if let Err(e) = client.mkcol(&photos_dir).await {
        log::debug!("MKCOL photos note: {:?}", e);
    }

    // List existing remote photos
    let remote_photos = list_remote_photos_simple(&client, &photos_dir).await?;

    // Get local photos that need upload (sync_status='local_only' or NULL)
    let mut stmt = conn.prepare(
        "SELECT uuid, COALESCE(relative_path, path) as rel_path, thumbnail_small_path, thumbnail_medium_path
         FROM photos 
         WHERE deleted = 0 AND (sync_status = 'local_only' OR sync_status IS NULL)",
    )?;

    let rows: Vec<(String, String, Option<String>, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let total_photos = rows.len();
    log::info!("Found {} photos to upload", total_photos);

    if total_photos == 0 {
        return Ok(0);
    }

    let mut join_set = JoinSet::new();
    let mut uploaded_count = 0;

    for (uuid, rel_path, _small_thumb, _medium_thumb) in rows {
        let client_clone = client.clone();
        let photos_dir_clone = photos_dir.clone();
        let remote_photos_clone = remote_photos.clone();

        // Limit concurrent uploads to 3
        while join_set.len() >= 3 {
            if let Some(result) = join_set.join_next().await {
                match result {
                    Ok(Ok((uuid_done, success))) => {
                        if success {
                            uploaded_count += 1;
                            // Update sync_status to 'synced'
                            let conn_update = crate::database::init_database()?;
                            let _ = conn_update.execute(
                                "UPDATE photos SET sync_status = 'synced', retry_count = 0, sync_error = NULL WHERE uuid = ?1",
                                rusqlite::params![uuid_done],
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        join_set.spawn(async move {
            upload_single_photo(
                uuid,
                rel_path,
                client_clone,
                photos_dir_clone,
                remote_photos_clone,
            )
            .await
        });
    }

    // Wait for remaining uploads
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok((uuid_done, success))) => {
                if success {
                    uploaded_count += 1;
                    // Update sync_status to 'synced'
                    let conn_update = crate::database::init_database()?;
                    let _ = conn_update.execute(
                        "UPDATE photos SET sync_status = 'synced', retry_count = 0, sync_error = NULL WHERE uuid = ?1",
                        rusqlite::params![uuid_done],
                    );
                }
            }
            _ => {}
        }
    }

    log::info!("Uploaded {} of {} photos", uploaded_count, total_photos);
    Ok(uploaded_count)
}

/// Uploads a single photo with all three versions (original + 2 thumbnails)
async fn upload_single_photo(
    uuid: String,
    rel_path: String,
    client: std::sync::Arc<reqwest_dav::Client>,
    photos_dir: String,
    remote_photos: Vec<String>,
) -> Result<(String, bool), AppError> {
    // Update status to 'uploading'
    let conn = crate::database::init_database()?;
    conn.execute(
        "UPDATE photos SET sync_status = 'uploading', last_sync_attempt = ?1 WHERE uuid = ?2",
        rusqlite::params![chrono::Utc::now().timestamp_millis(), &uuid],
    )?;

    let photo_name = format!("{}.jpg", uuid);

    // Skip if already uploaded
    if remote_photos.contains(&photo_name) {
        log::debug!("Photo {} already exists remotely", uuid);
        return Ok((uuid, true));
    }

    // Upload original
    let abs_path = crate::services::photo_service::get_absolute_photo_path(&rel_path);
    let file_path = std::path::Path::new(&abs_path);

    if !file_path.exists() {
        let error_msg = format!("Photo file not found locally: {}", abs_path);
        log::warn!("{}", error_msg);

        // Mark as failed
        conn.execute(
            "UPDATE photos SET sync_status = 'local_only', sync_error = ?1 WHERE uuid = ?2",
            rusqlite::params![error_msg, &uuid],
        )?;

        return Ok((uuid, false));
    }

    // Read and upload original
    match std::fs::read(file_path) {
        Ok(data) => {
            let remote_path = format!("{}/{}", photos_dir, photo_name);
            if let Err(e) = client.put(&remote_path, data).await {
                let error_msg = format!("Failed to upload original: {:?}", e);
                log::error!("Photo {}: {}", uuid, error_msg);

                conn.execute(
                    "UPDATE photos SET sync_status = 'local_only', sync_error = ?1 WHERE uuid = ?2",
                    rusqlite::params![error_msg, &uuid],
                )?;

                return Ok((uuid, false));
            }
            log::info!("Uploaded original photo: {}", photo_name);
        }
        Err(e) => {
            let error_msg = format!("Failed to read photo: {:?}", e);
            log::error!("{}: {}", abs_path, error_msg);

            conn.execute(
                "UPDATE photos SET sync_status = 'local_only', sync_error = ?1 WHERE uuid = ?2",
                rusqlite::params![error_msg, &uuid],
            )?;

            return Ok((uuid, false));
        }
    }

    // NOTE: Thumbnails are intentionally *not* uploaded anymore.
    // The receiving side should generate thumbnails locally from the
    // downloaded original image to avoid redundant uploads and save
    // remote storage / bandwidth.

    Ok((uuid, true))
}

/// Lists existing photo files in sync/photos/ directory
async fn list_remote_photos_simple(
    client: &reqwest_dav::Client,
    photos_dir: &str,
) -> Result<Vec<String>, AppError> {
    let list = match client.list(photos_dir, reqwest_dav::Depth::Number(1)).await {
        Ok(l) => l,
        Err(_) => return Ok(Vec::new()), // Directory doesn't exist yet
    };

    let mut names = Vec::new();
    for item in list {
        if let reqwest_dav::list_cmd::ListEntity::File(file) = item {
            if let Some(name) = file.href.split('/').last() {
                if name.ends_with(".jpg") || name.ends_with(".webp") {
                    names.push(name.to_string());
                }
            }
        }
    }

    Ok(names)
}
