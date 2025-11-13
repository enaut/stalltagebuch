use crate::error::AppError;
use crate::models::{EventMetadata, Photo, PhotoMetadata, QuailMetadata};
use crate::models::{Quail, QuailEvent};
use crate::services::sync_service;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Unique device ID (loaded from sync_settings or generated once)
pub fn get_device_id(conn: &Connection) -> Result<String, AppError> {
    use crate::services::sync_service;
    
    let settings = sync_service::load_sync_settings(conn)?;
    
    if let Some(mut settings) = settings {
        if let Some(device_id) = settings.device_id {
            return Ok(device_id);
        }
        // Generate and store new device_id
        let new_id = uuid::Uuid::new_v4().to_string();
        settings.device_id = Some(new_id.clone());
        sync_service::save_sync_settings(conn, &settings)?;
        Ok(new_id)
    } else {
        // No settings yet, return temporary ID
        Ok(uuid::Uuid::new_v4().to_string())
    }
}

/// Calculates SHA256 hash of a file
fn calculate_checksum(file_path: &Path) -> Result<String, AppError> {
    let data = fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("sha256:{:x}", result))
}

/// Calculates relative path for a photo based on association
fn get_relative_photo_path(conn: &Connection, photo: &Photo) -> Result<String, AppError> {
    let filename = Path::new(&photo.path)
        .file_name()
        .ok_or_else(|| AppError::Other("Invalid photo path".to_string()))?
        .to_string_lossy()
        .to_string();

    // Event-Foto?
    if let Some(ref event_id) = photo.event_id {
        let (event_uuid, quail_uuid): (String, String) = conn.query_row(
            "SELECT e.uuid, w.uuid FROM quail_events e 
             JOIN quails w ON e.quail_id = w.uuid 
             WHERE e.uuid = ?1",
            [&event_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        return Ok(format!(
            "quails/{}/events/{}/photos/{}",
            quail_uuid, event_uuid, filename
        ));
    }

    // Quail photo (incl. profile photo - all in photos/ folder)
    if let Some(ref quail_id) = photo.quail_id {
        return Ok(format!("quails/{}/photos/{}", quail_id, filename));
    }

    // Orphaned photo
    Ok(format!("orphaned_photos/{}", filename))
}

/// Creates TOML metadata for a photo
pub fn create_photo_metadata(
    conn: &Connection,
    photo: &Photo,
    file_path: &Path,
) -> Result<PhotoMetadata, AppError> {
    let checksum = calculate_checksum(file_path)?;
    let device_id = get_device_id(conn)?;
    let relative_path = get_relative_photo_path(conn, photo)?;

    // Get quail UUID and event UUID if available
    let (quail_uuid, event_uuid, notes) = if let Some(ref event_id) = photo.event_id {
        let result: Result<(String, String, Option<String>), _> = conn.query_row(
            "SELECT w.uuid, e.uuid, e.notes FROM quail_events e 
             JOIN quails w ON e.quail_id = w.uuid 
             WHERE e.uuid = ?1",
            [&event_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        result
            .map(|(w, e, n)| (Some(w), Some(e), n))
            .unwrap_or((None, None, None))
    } else if let Some(ref quail_id) = photo.quail_id {
        (Some(quail_id.to_string()), None, None)
    } else {
        (None, None, None)
    };

    Ok(PhotoMetadata {
        photo_id: photo.uuid.to_string(),
        quail_uuid,
        event_uuid,
        notes,
        device_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        checksum,
        relative_path,
    })
}

/// Creates TOML metadata for a quail
pub fn create_quail_metadata(conn: &Connection, quail: &Quail) -> Result<QuailMetadata, AppError> {
    let device_id = get_device_id(conn)?;

    Ok(QuailMetadata {
        uuid: quail.uuid.to_string(),
        name: quail.name.clone(),
        gender: quail.gender.as_str().to_string(),
        ring_color: quail.ring_color.as_ref().map(|r| r.as_str().to_string()),
        device_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        profile_photo: quail.profile_photo.map(|uuid| uuid.to_string()),
    })
}

/// Creates TOML metadata for an event
pub fn create_event_metadata(
    conn: &Connection,
    event: &QuailEvent,
) -> Result<EventMetadata, AppError> {
    let device_id = get_device_id(conn)?;

    let quail_uuid = event.quail_id.to_string();

    Ok(EventMetadata {
        uuid: event.uuid.to_string(),
        quail_uuid,
        event_type: event.event_type.as_str().to_string(),
        event_date: event.event_date.format("%Y-%m-%d").to_string(),
        notes: event.notes.clone(),
        device_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Lists all photos that have not been synchronized yet
pub fn list_pending_photos(conn: &Connection) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.uuid, p.quail_id, p.event_id, COALESCE(p.relative_path, p.path) as rel_path, p.thumbnail_path
         FROM photos p
         LEFT JOIN sync_queue sq ON p.uuid = sq.photo_id
         WHERE sq.photo_id IS NULL OR sq.status IN ('pending', 'failed')",
    )?;

    let rows = stmt.query_map([], |row| {
        let uuid_str: String = row.get(0)?;
        let quail_id_str: Option<String> = row.get(1)?;
        let event_id_str: Option<String> = row.get(2)?;
        let relative_path: String = row.get(3)?;
        let relative_thumb: Option<String> = row.get(4)?;

        Ok(Photo {
            uuid: uuid::Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
            quail_id: quail_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
            event_id: event_id_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
            path: crate::services::photo_service::get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb
                .map(|t| crate::services::photo_service::get_absolute_photo_path(&t)),
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Adds a photo to the sync queue
pub fn add_to_sync_queue(conn: &Connection, photo_uuid: &uuid::Uuid) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO sync_queue (photo_id, status) VALUES (?1, 'pending')",
        [&photo_uuid.to_string()],
    )?;
    Ok(())
}

/// Updates the status of a photo in the sync queue
pub fn update_sync_status(
    conn: &Connection,
    photo_uuid: &uuid::Uuid,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE sync_queue 
         SET status = ?1, 
             retry_count = retry_count + 1,
             last_error = ?2
         WHERE photo_id = ?3",
        rusqlite::params![status, error_message, photo_uuid.to_string()],
    )?;
    Ok(())
}

/// Uploads a photo and its metadata to the server
pub async fn upload_photo(
    conn: &Connection,
    photo: &Photo,
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<(), AppError> {
    // Build WebDAV client
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        server_url.trim_end_matches('/'),
        username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            username.to_string(),
            app_password.to_string(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV client error: {:?}", e)))?;

    // Get app directory
    #[cfg(target_os = "android")]
    let app_dir = crate::database::get_app_directory()
        .ok_or_else(|| AppError::NotFound("App directory not found".to_string()))?;

    #[cfg(not(target_os = "android"))]
    let app_dir = std::env::current_dir()?;

    let photo_file = app_dir.join(&photo.path);

    if !photo_file.exists() {
        return Err(AppError::NotFound(format!(
            "Photo not found: {}",
            photo.path
        )));
    }

    // Create metadata
    let metadata = create_photo_metadata(conn, photo, &photo_file)?;
    let metadata_toml = metadata
        .to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;

    // Lade Foto-Daten
    let photo_data = fs::read(&photo_file)?;

    // Verwende den relativen Pfad aus den Metadaten
    let photo_remote_path = format!(
        "{}/{}",
        remote_path.trim_end_matches('/'),
        metadata.relative_path
    );

    // Erstelle alle notwendigen Ordner
    let path_parts: Vec<&str> = metadata.relative_path.split('/').collect();
    for i in 1..path_parts.len() {
        let dir_path = format!(
            "{}/{}",
            remote_path.trim_end_matches('/'),
            path_parts[..i].join("/")
        );
        // Versuche Ordner zu erstellen (ignoriere Fehler wenn bereits existiert)
        let _ = client.mkcol(&dir_path).await;
    }

    // Upload Foto
    client
        .put(&photo_remote_path, photo_data)
        .await
        .map_err(|e| AppError::Other(format!("Photo Upload fehlgeschlagen: {:?}", e)))?;

    // Upload Metadata (im selben Ordner wie das Foto)
    let metadata_remote_path = photo_remote_path.replace(".jpg", ".toml");
    client
        .put(&metadata_remote_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Metadata Upload fehlgeschlagen: {:?}", e)))?;

    Ok(())
}

/// Checks which photos are already on the server
pub async fn list_remote_photos(
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<Vec<String>, AppError> {
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        server_url.trim_end_matches('/'),
        username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            username.to_string(),
            app_password.to_string(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV client error: {:?}", e)))?;

    // List files in remote directory
    let list = client
        .list(remote_path, reqwest_dav::Depth::Number(1))
        .await
        .map_err(|e| AppError::Other(format!("WebDAV list failed: {:?}", e)))?;

    // Filter only .jpg files and extract UUIDs
    let mut uuids = Vec::new();
    for item in list {
        if let reqwest_dav::list_cmd::ListEntity::File(file) = item {
            if file.href.ends_with(".jpg") && !file.href.contains("thumb_") {
                // Extrahiere Dateiname
                if let Some(filename) = file.href.split('/').last() {
                    // UUID ist Dateiname ohne .jpg
                    if let Some(uuid) = filename.strip_suffix(".jpg") {
                        uuids.push(uuid.to_string());
                    }
                }
            }
        }
    }

    Ok(uuids)
}

/// Synchronisiert eine Wachtel (Stammdaten)
pub async fn sync_quail(
    conn: &Connection,
    quail: &Quail,
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<(), AppError> {
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        server_url.trim_end_matches('/'),
        username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            username.to_string(),
            app_password.to_string(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV Client Fehler: {:?}", e)))?;

    // Erstelle Metadata
    let metadata = create_quail_metadata(conn, quail)?;
    let metadata_toml = metadata
        .to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;

    // Erstelle Ordnerstruktur
    let quail_dir = format!(
        "{}/quails/{}",
        remote_path.trim_end_matches('/'),
        quail.uuid.to_string()
    );
    let _ = client
        .mkcol(&format!("{}/quails", remote_path.trim_end_matches('/')))
        .await;
    let _ = client.mkcol(&quail_dir).await;
    let _ = client.mkcol(&format!("{}/photos", quail_dir)).await;
    let _ = client.mkcol(&format!("{}/events", quail_dir)).await;

    // Upload profile.toml
    let metadata_path = format!("{}/profile.toml", quail_dir);
    client
        .put(&metadata_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Wachtel-Metadata Upload fehlgeschlagen: {:?}", e)))?;

    Ok(())
}

/// Synchronisiert ein Event
pub async fn sync_event(
    conn: &Connection,
    event: &QuailEvent,
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<(), AppError> {
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        server_url.trim_end_matches('/'),
        username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            username.to_string(),
            app_password.to_string(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV Client Fehler: {:?}", e)))?;

    // Erstelle Metadata
    let metadata = create_event_metadata(conn, event)?;
    let metadata_toml = metadata
        .to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;

    // Hole Wachtel-UUID
    let quail_uuid = event.quail_id.to_string();

    // Erstelle Ordnerstruktur
    let event_dir = format!(
        "{}/quails/{}/events/{}",
        remote_path.trim_end_matches('/'),
        quail_uuid,
        metadata.uuid
    );
    let _ = client.mkcol(&event_dir).await;
    let _ = client.mkcol(&format!("{}/photos", event_dir)).await;

    // Upload event.toml
    let metadata_path = format!("{}/event.toml", event_dir);
    client
        .put(&metadata_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Event-Metadata Upload fehlgeschlagen: {:?}", e)))?;

    Ok(())
}

/// Synchronisiert alle Wachtels
pub async fn sync_all_quails(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_quails started ===");

    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;

    if !settings.enabled {
        return Err(AppError::Validation(
            "Synchronisierung ist deaktiviert".to_string(),
        ));
    }

    // Hole alle Wachtels
    let mut stmt =
        conn.prepare("SELECT uuid, name, gender, ring_color, profile_photo FROM quails")?;
    let quails: Vec<Quail> = stmt
        .query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            let profile_photo_str: Option<String> = row.get(4)?;
            Ok(Quail {
                uuid: uuid::Uuid::parse_str(&uuid_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                name: row.get(1)?,
                gender: crate::models::Gender::from_str(&row.get::<_, String>(2)?),
                ring_color: row
                    .get::<_, Option<String>>(3)?
                    .map(|s| crate::models::RingColor::from_str(&s)),
                profile_photo: profile_photo_str.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    eprintln!("Found {} quails to sync", quails.len());
    let mut synced_count = 0;

    for quail in quails {
        eprintln!(
            "Syncing quail: uuid={}, name={}",
            quail.uuid.to_string(),
            quail.name
        );
        match sync_quail(
            conn,
            &quail,
            &settings.server_url,
            &settings.username,
            &settings.app_password,
            &settings.remote_path,
        )
        .await
        {
            Ok(_) => {
                eprintln!("Wachtel sync successful!");
                synced_count += 1;
            }
            Err(e) => {
                eprintln!("Wachtel sync failed: {}", e);
            }
        }
    }

    eprintln!("=== sync_all_quails completed: {} synced ===", synced_count);
    Ok(synced_count)
}

/// Synchronisiert alle Events
pub async fn sync_all_events(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_events started ===");

    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;

    if !settings.enabled {
        return Err(AppError::Validation(
            "Synchronisierung ist deaktiviert".to_string(),
        ));
    }

    // Hole alle Events
    let mut stmt =
        conn.prepare("SELECT uuid, quail_id, event_type, event_date, notes FROM quail_events")?;
    let events: Vec<QuailEvent> = stmt
        .query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            let quail_id_str: String = row.get(1)?;
            let event_date_str: String = row.get(3)?;
            let event_date = chrono::NaiveDate::parse_from_str(&event_date_str, "%Y-%m-%d")
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            Ok(QuailEvent {
                uuid: uuid::Uuid::parse_str(&uuid_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                quail_id: uuid::Uuid::parse_str(&quail_id_str)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                event_type: crate::models::EventType::from_str(&row.get::<_, String>(2)?),
                event_date,
                notes: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    eprintln!("Found {} events to sync", events.len());
    let mut synced_count = 0;

    for event in events {
        eprintln!("Syncing event: type={}", event.event_type.as_str());
        match sync_event(
            conn,
            &event,
            &settings.server_url,
            &settings.username,
            &settings.app_password,
            &settings.remote_path,
        )
        .await
        {
            Ok(_) => {
                eprintln!("Event sync successful!");
                synced_count += 1;
            }
            Err(e) => {
                eprintln!("Event sync failed: {}", e);
            }
        }
    }

    eprintln!("=== sync_all_events completed: {} synced ===", synced_count);
    Ok(synced_count)
}

/// Synchronisiert alle ausstehenden Fotos
pub async fn sync_all_photos(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_photos started ===");

    // Lade Sync-Einstellungen
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;
    eprintln!(
        "Settings loaded: server={}, user={}, remote_path={}",
        settings.server_url, settings.username, settings.remote_path
    );

    if !settings.enabled {
        eprintln!("Sync is disabled!");
        return Err(AppError::Validation(
            "Synchronisierung ist deaktiviert".to_string(),
        ));
    }

    // Liste remote Fotos
    eprintln!("Listing remote photos...");
    let remote_uuids = list_remote_photos(
        &settings.server_url,
        &settings.username,
        &settings.app_password,
        &settings.remote_path,
    )
    .await?;
    eprintln!("Found {} remote photos", remote_uuids.len());

    // Finde lokale Fotos die noch nicht hochgeladen wurden
    eprintln!("Finding pending photos...");
    let pending_photos = list_pending_photos(conn)?;
    eprintln!("Found {} pending photos", pending_photos.len());

    let mut uploaded_count = 0;

    for photo in pending_photos {
        eprintln!(
            "Processing photo: uuid={}, path={}",
            photo.uuid.to_string(),
            photo.path
        );

        // Überspringe wenn bereits remote vorhanden
        if remote_uuids.contains(&photo.uuid.to_string()) {
            eprintln!("Photo already exists remotely, marking as completed");
            // Markiere als completed
            update_sync_status(conn, &photo.uuid, "completed", None)?;
            continue;
        }

        // Füge zur Queue hinzu falls nicht vorhanden
        eprintln!("Adding to sync queue...");
        add_to_sync_queue(conn, &photo.uuid)?;

        // Setze Status auf uploading
        eprintln!("Setting status to uploading...");
        update_sync_status(conn, &photo.uuid, "uploading", None)?;

        // Upload
        eprintln!("Starting upload...");
        match upload_photo(
            conn,
            &photo,
            &settings.server_url,
            &settings.username,
            &settings.app_password,
            &settings.remote_path,
        )
        .await
        {
            Ok(_) => {
                eprintln!("Upload successful!");
                update_sync_status(conn, &photo.uuid, "completed", None)?;
                uploaded_count += 1;
            }
            Err(e) => {
                eprintln!("Upload failed: {}", e);
                let error_msg = format!("{}", e);
                update_sync_status(conn, &photo.uuid, "failed", Some(&error_msg))?;
            }
        }
    }

    // Aktualisiere last_sync
    sync_service::update_last_sync(conn)?;

    eprintln!(
        "=== sync_all_photos completed: {} uploaded ===",
        uploaded_count
    );
    Ok(uploaded_count)
}

/// Creates TOML metadata for an egg record
pub fn create_egg_record_metadata(
    conn: &Connection,
    egg_record: &crate::models::EggRecord,
    created_at: String,
    updated_at: String,
) -> Result<crate::models::EggRecordMetadata, AppError> {
    let device_id = get_device_id(conn)?;

    Ok(crate::models::EggRecordMetadata {
        uuid: egg_record.uuid.to_string(),
        record_date: egg_record.record_date.format("%Y-%m-%d").to_string(),
        total_eggs: egg_record.total_eggs,
        notes: egg_record.notes.clone(),
        device_id,
        created_at,
        updated_at,
    })
}

/// Synchronizes an egg record
pub async fn sync_egg_record(
    conn: &Connection,
    egg_record: &crate::models::EggRecord,
    created_at: String,
    updated_at: String,
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<(), AppError> {
    let webdav_url = format!(
        "{}/remote.php/dav/files/{}",
        server_url.trim_end_matches('/'),
        username
    );

    let client = reqwest_dav::ClientBuilder::new()
        .set_host(webdav_url)
        .set_auth(reqwest_dav::Auth::Basic(
            username.to_string(),
            app_password.to_string(),
        ))
        .build()
        .map_err(|e| AppError::Other(format!("WebDAV Client Fehler: {:?}", e)))?;

    // Erstelle Metadata
    let metadata = create_egg_record_metadata(conn, egg_record, created_at, updated_at)?;
    let metadata_toml = metadata
        .to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;

    // Erstelle Ordnerstruktur
    let egg_records_dir = format!("{}/egg_records", remote_path.trim_end_matches('/'));
    let _ = client.mkcol(&egg_records_dir).await;

    // Upload egg_record.toml
    let metadata_path = format!("{}/{}.toml", egg_records_dir, metadata.uuid);
    client
        .put(&metadata_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| {
            AppError::Other(format!(
                "Egg-Record-Metadata Upload fehlgeschlagen: {:?}",
                e
            ))
        })?;

    Ok(())
}

/// Synchronizes all egg records
pub async fn sync_all_egg_records(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_egg_records started ===");

    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync settings not configured".to_string()))?;

    if !settings.enabled {
        return Err(AppError::Validation(
            "Synchronization is disabled".to_string(),
        ));
    }

    // Get all egg records
    let mut stmt = conn.prepare(
        "SELECT uuid, record_date, total_eggs, notes, created_at, updated_at FROM egg_records",
    )?;
    let egg_records: Vec<(crate::models::EggRecord, String, String)> = stmt
        .query_map([], |row| {
            let uuid_str: String = row.get(0)?;
            let date_str: String = row.get(1)?;
            let total_eggs: i32 = row.get(2)?;
            let notes: Option<String> = row.get(3)?;
            let created_at: String = row.get(4)?;
            let updated_at: String = row.get(5)?;

            let record_date =
                chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            Ok((
                crate::models::EggRecord {
                    uuid: uuid::Uuid::parse_str(&uuid_str)
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    record_date,
                    total_eggs,
                    notes,
                },
                created_at,
                updated_at,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut synced_count = 0;

    for (egg_record, created_at, updated_at) in egg_records {
        match sync_egg_record(
            conn,
            &egg_record,
            created_at,
            updated_at,
            &settings.server_url,
            &settings.username,
            &settings.app_password,
            &settings.remote_path,
        )
        .await
        {
            Ok(_) => {
                eprintln!(
                    "Egg record sync successful: {}",
                    egg_record.uuid.to_string()
                );
                synced_count += 1;
            }
            Err(e) => {
                eprintln!("Egg record sync failed: {}", e);
            }
        }
    }

    eprintln!(
        "=== sync_all_egg_records completed: {} synced ===",
        synced_count
    );
    Ok(synced_count)
}

/// Synchronizes EVERYTHING: Quails, Events, Photos and Egg Records
pub async fn sync_all(conn: &Connection) -> Result<(usize, usize, usize, usize), AppError> {
    eprintln!("=== FULL SYNC STARTED ===");

    // 1. Synchronize quails (master data)
    let quails_synced = sync_all_quails(conn).await?;

    // 2. Synchronisiere Events
    let events_synced = sync_all_events(conn).await?;

    // 3. Synchronisiere Eier-Einträge
    let egg_records_synced = sync_all_egg_records(conn).await?;

    // 4. Synchronisiere Fotos
    let photos_synced = sync_all_photos(conn).await?;

    // Aktualisiere last_sync
    sync_service::update_last_sync(conn)?;

    eprintln!(
        "=== FULL SYNC COMPLETED: {} quails, {} events, {} egg records, {} photos ===",
        quails_synced, events_synced, egg_records_synced, photos_synced
    );

    Ok((
        quails_synced,
        events_synced,
        egg_records_synced,
        photos_synced,
    ))
}

/// Uploads a batch of operations to sync/ops/<device>/<YYYYMM>/<ULID>.ndjson
/// 
/// This is a minimal skeleton for the new multi-master sync.
pub async fn upload_ops_batch(
    conn: &Connection,
    ops: Vec<crate::services::crdt_service::Operation>,
) -> Result<(), AppError> {
    use crate::services::{sync_paths, sync_service};
    
    if ops.is_empty() {
        return Ok(());
    }
    
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync settings not configured".to_string()))?;
    
    if !settings.enabled {
        return Err(AppError::Validation("Sync disabled".to_string()));
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
    let full_path = format!("{}/{}/{}", settings.remote_path.trim_end_matches('/'), ops_dir, filename);
    
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
            eprintln!("MKCOL '{}' note: {:?}", path, e);
        }
    }
    
    // Upload (atomic create via If-None-Match not directly supported, use put)
    client
        .put(&full_path, ndjson_content.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Upload ops batch failed: {:?}", e)))?;
    
    eprintln!("Uploaded ops batch: {} operations to {}", ops.len(), full_path);
    
    Ok(())
}
