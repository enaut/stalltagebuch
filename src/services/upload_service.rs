use crate::error::AppError;
use crate::models::{Photo, PhotoMetadata, WachtelMetadata, EventMetadata};
use crate::models::{Wachtel, WachtelEvent};
use crate::services::sync_service;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Eindeutige Geräte-ID (sollte beim ersten Start generiert und gespeichert werden)
fn get_device_id() -> String {
    // TODO: Sollte einmalig generiert und in den Einstellungen gespeichert werden
    uuid::Uuid::new_v4().to_string()
}

/// Berechnet SHA256 Hash einer Datei
fn calculate_checksum(file_path: &Path) -> Result<String, AppError> {
    let data = fs::read(file_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("sha256:{:x}", result))
}

/// Berechnet relativen Pfad für ein Photo basierend auf Zuordnung
fn get_relative_photo_path(
    conn: &Connection,
    photo: &Photo,
) -> Result<String, AppError> {
    let filename = Path::new(&photo.path)
        .file_name()
        .ok_or_else(|| AppError::Other("Ungültiger Foto-Pfad".to_string()))?
        .to_string_lossy()
        .to_string();
    
    // Event-Foto?
    if let Some(event_id) = photo.event_id {
        let (event_uuid, wachtel_uuid): (String, String) = conn.query_row(
            "SELECT e.uuid, w.uuid FROM wachtel_events e 
             JOIN wachtels w ON e.wachtel_id = w.id 
             WHERE e.id = ?1",
            [event_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        return Ok(format!("wachtels/{}/events/{}/photos/{}", 
            wachtel_uuid, event_uuid, filename));
    }
    
    // Wachtel-Foto (inkl. Profilfoto - alle im photos/ Ordner)
    if let Some(wachtel_id) = photo.wachtel_id {
        let wachtel_uuid: String = conn.query_row(
            "SELECT uuid FROM wachtels WHERE id = ?1",
            [wachtel_id],
            |row| row.get(0),
        )?;
        
        return Ok(format!("wachtels/{}/photos/{}", wachtel_uuid, filename));
    }
    
    // Verwaistes Foto
    Ok(format!("orphaned_photos/{}", filename))
}

/// Erstellt TOML Metadata für ein Photo
pub fn create_photo_metadata(
    conn: &Connection,
    photo: &Photo,
    file_path: &Path,
) -> Result<PhotoMetadata, AppError> {
    let checksum = calculate_checksum(file_path)?;
    let device_id = get_device_id();
    let relative_path = get_relative_photo_path(conn, photo)?;
    
    // Hole Wachtel-UUID und Event-UUID falls vorhanden
    let (wachtel_uuid, event_uuid, notes) = if let Some(event_id) = photo.event_id {
        let result: Result<(String, String, Option<String>), _> = conn.query_row(
            "SELECT w.uuid, e.uuid, e.notes FROM wachtel_events e 
             JOIN wachtels w ON e.wachtel_id = w.id 
             WHERE e.id = ?1",
            [event_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        result.map(|(w, e, n)| (Some(w), Some(e), n)).unwrap_or((None, None, None))
    } else if let Some(wachtel_id) = photo.wachtel_id {
        let wachtel_uuid: Option<String> = conn.query_row(
            "SELECT uuid FROM wachtels WHERE id = ?1",
            [wachtel_id],
            |row| row.get(0),
        ).ok();
        (wachtel_uuid, None, None)
    } else {
        (None, None, None)
    };

    Ok(PhotoMetadata {
        photo_id: photo.uuid.clone(),
        wachtel_id: photo.wachtel_id,
        wachtel_uuid,
        event_id: photo.event_id,
        event_uuid,
        notes,
        device_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
        checksum,
        is_profile: photo.is_profile,
        relative_path,
    })
}

/// Erstellt TOML Metadata für eine Wachtel
pub fn create_wachtel_metadata(
    conn: &Connection,
    wachtel: &Wachtel,
) -> Result<WachtelMetadata, AppError> {
    let device_id = get_device_id();
    
    // Prüfe ob Profilfoto vorhanden
    let has_profile_photo: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM photos WHERE wachtel_id = ?1 AND is_profile = 1",
        [wachtel.id],
        |row| row.get(0),
    ).unwrap_or(false);
    
    Ok(WachtelMetadata {
        uuid: wachtel.uuid.clone(),
        name: wachtel.name.clone(),
        gender: wachtel.gender.as_str().to_string(),
        ring_color: wachtel.ring_color.as_ref().map(|r| r.as_str().to_string()),
        device_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        has_profile_photo,
    })
}

/// Erstellt TOML Metadata für ein Event
pub fn create_event_metadata(
    conn: &Connection,
    event: &WachtelEvent,
) -> Result<EventMetadata, AppError> {
    let device_id = get_device_id();
    
    // Hole Wachtel-UUID
    let wachtel_uuid: String = conn.query_row(
        "SELECT uuid FROM wachtels WHERE id = ?1",
        [event.wachtel_id],
        |row| row.get(0),
    )?;
    
    Ok(EventMetadata {
        uuid: event.uuid.clone(),
        wachtel_uuid,
        event_type: event.event_type.as_str().to_string(),
        event_date: event.event_date.format("%Y-%m-%d").to_string(),
        notes: event.notes.clone(),
        device_id,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Listet alle Fotos auf, die noch nicht synchronisiert wurden
pub fn list_pending_photos(conn: &Connection) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.uuid, p.wachtel_id, p.event_id, p.path, p.thumbnail_path, p.is_profile
         FROM photos p
         LEFT JOIN sync_queue sq ON p.uuid = sq.photo_uuid
         WHERE sq.photo_uuid IS NULL OR sq.status IN ('pending', 'failed')
         ORDER BY p.id"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let relative_path: String = row.get(4)?;
        let relative_thumb: Option<String> = row.get(5)?;
        
        Ok(Photo {
            id: row.get(0)?,
            uuid: row.get(1)?,
            wachtel_id: row.get(2)?,
            event_id: row.get(3)?,
            path: crate::services::photo_service::get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| crate::services::photo_service::get_absolute_photo_path(&t)),
            is_profile: matches!(row.get::<_, i64>(6)?, 1),
        })
    })?;
    
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Fügt ein Photo zur Sync-Queue hinzu
pub fn add_to_sync_queue(conn: &Connection, photo_uuid: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO sync_queue (photo_uuid, status) VALUES (?1, 'pending')",
        [photo_uuid],
    )?;
    Ok(())
}

/// Aktualisiert den Status eines Photos in der Sync-Queue
pub fn update_sync_status(
    conn: &Connection,
    photo_uuid: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE sync_queue 
         SET status = ?1, 
             last_attempt = CURRENT_TIMESTAMP,
             retry_count = retry_count + 1,
             error_message = ?2
         WHERE photo_uuid = ?3",
        rusqlite::params![status, error_message, photo_uuid],
    )?;
    Ok(())
}

/// Lädt ein Photo und seine Metadata auf den Server hoch
pub async fn upload_photo(
    conn: &Connection,
    photo: &Photo,
    server_url: &str,
    username: &str,
    app_password: &str,
    remote_path: &str,
) -> Result<(), AppError> {
    // Baue WebDAV Client
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

    // Hole App-Directory
    #[cfg(target_os = "android")]
    let app_dir = crate::database::get_app_directory()
        .ok_or_else(|| AppError::NotFound("App-Verzeichnis nicht gefunden".to_string()))?;
    
    #[cfg(not(target_os = "android"))]
    let app_dir = std::env::current_dir()?;

    let photo_file = app_dir.join(&photo.path);
    
    if !photo_file.exists() {
        return Err(AppError::NotFound(format!(
            "Foto nicht gefunden: {}",
            photo.path
        )));
    }

    // Erstelle Metadata
    let metadata = create_photo_metadata(conn, photo, &photo_file)?;
    let metadata_toml = metadata.to_toml()
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

/// Prüft welche Fotos bereits auf dem Server sind
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
        .map_err(|e| AppError::Other(format!("WebDAV Client Fehler: {:?}", e)))?;

    // Liste Dateien im Remote-Verzeichnis
    let list = client
        .list(remote_path, reqwest_dav::Depth::Number(1))
        .await
        .map_err(|e| AppError::Other(format!("WebDAV List fehlgeschlagen: {:?}", e)))?;

    // Filtere nur .jpg Dateien und extrahiere UUIDs
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
pub async fn sync_wachtel(
    conn: &Connection,
    wachtel: &Wachtel,
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
    let metadata = create_wachtel_metadata(conn, wachtel)?;
    let metadata_toml = metadata.to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;
    
    // Erstelle Ordnerstruktur
    let wachtel_dir = format!("{}/wachtels/{}", remote_path.trim_end_matches('/'), wachtel.uuid);
    let _ = client.mkcol(&format!("{}/wachtels", remote_path.trim_end_matches('/'))).await;
    let _ = client.mkcol(&wachtel_dir).await;
    let _ = client.mkcol(&format!("{}/photos", wachtel_dir)).await;
    let _ = client.mkcol(&format!("{}/events", wachtel_dir)).await;
    
    // Upload profile.toml
    let metadata_path = format!("{}/profile.toml", wachtel_dir);
    client
        .put(&metadata_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Wachtel-Metadata Upload fehlgeschlagen: {:?}", e)))?;
    
    Ok(())
}

/// Synchronisiert ein Event
pub async fn sync_event(
    conn: &Connection,
    event: &WachtelEvent,
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
    let metadata_toml = metadata.to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;
    
    // Hole Wachtel-UUID
    let wachtel_uuid: String = conn.query_row(
        "SELECT uuid FROM wachtels WHERE id = ?1",
        [event.wachtel_id],
        |row| row.get(0),
    )?;
    
    // Erstelle Ordnerstruktur
    let event_dir = format!(
        "{}/wachtels/{}/events/{}",
        remote_path.trim_end_matches('/'),
        wachtel_uuid,
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
pub async fn sync_all_wachtels(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_wachtels started ===");
    
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;
    
    if !settings.enabled {
        return Err(AppError::Validation("Synchronisierung ist deaktiviert".to_string()));
    }
    
    // Hole alle Wachtels
    let mut stmt = conn.prepare("SELECT id, uuid, name, gender, ring_color FROM wachtels")?;
    let wachtels: Vec<Wachtel> = stmt.query_map([], |row| {
        Ok(Wachtel {
            id: row.get(0)?,
            uuid: row.get(1)?,
            name: row.get(2)?,
            gender: crate::models::Gender::from_str(&row.get::<_, String>(3)?),
            ring_color: row.get::<_, Option<String>>(4)?
                .map(|s| crate::models::Ringfarbe::from_str(&s)),
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    
    eprintln!("Found {} wachtels to sync", wachtels.len());
    let mut synced_count = 0;
    
    for wachtel in wachtels {
        eprintln!("Syncing wachtel: uuid={}, name={}", wachtel.uuid, wachtel.name);
        match sync_wachtel(
            conn,
            &wachtel,
            &settings.server_url,
            &settings.username,
            &settings.app_password,
            &settings.remote_path,
        ).await {
            Ok(_) => {
                eprintln!("Wachtel sync successful!");
                synced_count += 1;
            }
            Err(e) => {
                eprintln!("Wachtel sync failed: {}", e);
            }
        }
    }
    
    eprintln!("=== sync_all_wachtels completed: {} synced ===", synced_count);
    Ok(synced_count)
}

/// Synchronisiert alle Events
pub async fn sync_all_events(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_events started ===");
    
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;
    
    if !settings.enabled {
        return Err(AppError::Validation("Synchronisierung ist deaktiviert".to_string()));
    }
    
    // Hole alle Events
    let mut stmt = conn.prepare(
        "SELECT id, uuid, wachtel_id, event_type, event_date, notes FROM wachtel_events"
    )?;
    let events: Vec<WachtelEvent> = stmt.query_map([], |row| {
        let event_date_str: String = row.get(4)?;
        let event_date = chrono::NaiveDate::parse_from_str(&event_date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(e)
            ))?;
        
        Ok(WachtelEvent {
            id: row.get(0)?,
            uuid: row.get(1)?,
            wachtel_id: row.get(2)?,
            event_type: crate::models::EventType::from_str(&row.get::<_, String>(3)?),
            event_date,
            notes: row.get(5)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    
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
        ).await {
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
    eprintln!("Settings loaded: server={}, user={}, remote_path={}", 
        settings.server_url, settings.username, settings.remote_path);

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
            eprintln!("Processing photo: uuid={}, path={}", photo.uuid, photo.path);
        
        // Überspringe wenn bereits remote vorhanden
        if remote_uuids.contains(&photo.uuid) {
                        eprintln!("Photo already exists remotely, marking as uploaded");
            // Markiere als uploaded
            update_sync_status(conn, &photo.uuid, "uploaded", None)?;
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
                update_sync_status(conn, &photo.uuid, "uploaded", None)?;
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

    eprintln!("=== sync_all_photos completed: {} uploaded ===", uploaded_count);
    Ok(uploaded_count)
}

/// Erstellt TOML Metadata für einen Eier-Eintrag
pub fn create_egg_record_metadata(
    egg_record: &crate::models::EggRecord,
    created_at: String,
    updated_at: String,
) -> Result<crate::models::EggRecordMetadata, AppError> {
    let device_id = get_device_id();
    
    Ok(crate::models::EggRecordMetadata {
        uuid: egg_record.uuid.clone(),
        record_date: egg_record.record_date.format("%Y-%m-%d").to_string(),
        total_eggs: egg_record.total_eggs,
        notes: egg_record.notes.clone(),
        device_id,
        created_at,
        updated_at,
    })
}

/// Synchronisiert einen Eier-Eintrag
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
    let metadata = create_egg_record_metadata(egg_record, created_at, updated_at)?;
    let metadata_toml = metadata.to_toml()
        .map_err(|e| AppError::Other(format!("TOML Serialisierung fehlgeschlagen: {}", e)))?;
    
    // Erstelle Ordnerstruktur
    let egg_records_dir = format!("{}/egg_records", remote_path.trim_end_matches('/'));
    let _ = client.mkcol(&egg_records_dir).await;
    
    // Upload egg_record.toml
    let metadata_path = format!("{}/{}.toml", egg_records_dir, metadata.uuid);
    client
        .put(&metadata_path, metadata_toml.into_bytes())
        .await
        .map_err(|e| AppError::Other(format!("Egg-Record-Metadata Upload fehlgeschlagen: {:?}", e)))?;
    
    Ok(())
}

/// Synchronisiert alle Eier-Einträge
pub async fn sync_all_egg_records(conn: &Connection) -> Result<usize, AppError> {
    eprintln!("=== sync_all_egg_records started ===");
    
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync-Einstellungen nicht konfiguriert".to_string()))?;
    
    if !settings.enabled {
        return Err(AppError::Validation("Synchronisierung ist deaktiviert".to_string()));
    }
    
    // Hole alle Eier-Einträge
    let mut stmt = conn.prepare(
        "SELECT id, uuid, record_date, total_eggs, notes, created_at, updated_at FROM egg_records"
    )?;
    let egg_records: Vec<(crate::models::EggRecord, String, String)> = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let uuid: String = row.get(1)?;
        let date_str: String = row.get(2)?;
        let total_eggs: i32 = row.get(3)?;
        let notes: Option<String> = row.get(4)?;
        let created_at: String = row.get(5)?;
        let updated_at: String = row.get(6)?;
        
        let record_date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(e)
            ))?;
        
        Ok((
            crate::models::EggRecord {
                id: Some(id),
                uuid,
                record_date,
                total_eggs,
                notes,
            },
            created_at,
            updated_at,
        ))
    })?.collect::<Result<Vec<_>, _>>()?;
    
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
        ).await {
            Ok(_) => {
                eprintln!("Egg record sync successful: {}", egg_record.uuid);
                synced_count += 1;
            }
            Err(e) => {
                eprintln!("Egg record sync failed: {}", e);
            }
        }
    }
    
    eprintln!("=== sync_all_egg_records completed: {} synced ===", synced_count);
    Ok(synced_count)
}

/// Synchronisiert ALLES: Wachtels, Events, Fotos und Eier-Einträge
pub async fn sync_all(conn: &Connection) -> Result<(usize, usize, usize, usize), AppError> {
    eprintln!("=== FULL SYNC STARTED ===");
    
    // 1. Synchronisiere Wachtels (Stammdaten)
    let wachtels_synced = sync_all_wachtels(conn).await?;
    
    // 2. Synchronisiere Events
    let events_synced = sync_all_events(conn).await?;
    
    // 3. Synchronisiere Eier-Einträge
    let egg_records_synced = sync_all_egg_records(conn).await?;
    
    // 4. Synchronisiere Fotos
    let photos_synced = sync_all_photos(conn).await?;
    
    // Aktualisiere last_sync
    sync_service::update_last_sync(conn)?;
    
    eprintln!(
        "=== FULL SYNC COMPLETED: {} wachtels, {} events, {} egg records, {} photos ===",
        wachtels_synced, events_synced, egg_records_synced, photos_synced
    );
    
    Ok((wachtels_synced, events_synced, egg_records_synced, photos_synced))
}
