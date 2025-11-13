use crate::error::AppError;
use crate::models::Photo;
use rusqlite::{params, Connection, OptionalExtension};
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

/// Renames a photo file with UUID and returns the new path
fn rename_photo_with_uuid(original_path: &str) -> Result<(String, String), AppError> {
    eprintln!("=== rename_photo_with_uuid called ===");
    eprintln!("Original path: {}", original_path);

    let uuid = uuid::Uuid::new_v4().to_string();
    let new_filename = format!("{}.jpg", uuid);
    let thumb_filename = format!("{}_thumb.jpg", uuid);

    eprintln!("Generated UUID: {}", uuid);
    eprintln!("New filename: {}", new_filename);

    // File is already in the correct directory, simply rename it there
    let old_path = std::path::Path::new(original_path);

    if let Some(parent_dir) = old_path.parent() {
        let new_path = parent_dir.join(&new_filename);

        eprintln!("Old path: {:?}", old_path);
        eprintln!("New path: {:?}", new_path);
        eprintln!("Checking if old_path exists: {}", old_path.exists());

        if old_path.exists() {
            eprintln!("Path exists, copying...");
            match std::fs::copy(old_path, &new_path) {
                Ok(_) => {
                    eprintln!("Copy successful, removing original...");
                    if let Err(e) = std::fs::remove_file(old_path) {
                        eprintln!("Warning: Could not remove original: {}", e);
                    } else {
                        eprintln!("Original removed");
                    }
                }
                Err(e) => {
                    eprintln!("ERROR during copy: {}", e);
                    return Err(AppError::Other(format!("Fehler beim Kopieren: {}", e)));
                }
            }

            eprintln!("=== rename_photo_with_uuid completed ===");
            return Ok((new_filename, thumb_filename));
        } else {
            eprintln!("ERROR: Original path doesn't exist!");
            return Err(AppError::Other(format!(
                "Originaldatei nicht gefunden: {}",
                original_path
            )));
        }
    }

    Ok((new_filename, thumb_filename))
}

pub async fn add_quail_photo(
    conn: &Connection,
    quail_id: Uuid,
    path: String,
    thumbnail_path: Option<String>,
) -> Result<Uuid, AppError> {
    eprintln!("=== add_quail_photo called ===");
    eprintln!(
        "Quail ID: {}, Path: {}, Thumbnail: {:?}",
        quail_id, path, thumbnail_path
    );

    // Benenne Foto mit UUID um
    let (new_path, new_thumb_name) = rename_photo_with_uuid(&path)?;
    let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
        .map_err(|_| AppError::Other("Invalid UUID from filename".to_string()))?;
    eprintln!("UUID extracted: {}", uuid);

    // Benenne auch Thumbnail mit UUID um falls vorhanden
    let final_thumb = if let Some(thumb_path) = thumbnail_path {
        eprintln!("Renaming thumbnail: {}", thumb_path);
        let old_thumb = std::path::Path::new(&thumb_path);
        if let Some(parent_dir) = old_thumb.parent() {
            let new_thumb_path = parent_dir.join(&new_thumb_name);
            eprintln!("Thumbnail new path: {:?}", new_thumb_path);

            if old_thumb.exists() {
                match std::fs::copy(old_thumb, &new_thumb_path) {
                    Ok(_) => {
                        eprintln!("Thumbnail copied successfully");
                        let _ = std::fs::remove_file(old_thumb);
                        Some(new_thumb_name)
                    }
                    Err(e) => {
                        eprintln!("Warning: Could not copy thumbnail: {}", e);
                        None
                    }
                }
            } else {
                eprintln!("Warning: Thumbnail doesn't exist");
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    conn.execute(
        "INSERT INTO photos (uuid, quail_id, path, thumbnail_path) VALUES (?1, ?2, ?3, ?4)",
        params![
            uuid.to_string(),
            quail_id.to_string(),
            &new_path,
            &final_thumb,
        ],
    )?;

    // Capture CRDT operation
    crate::services::operation_capture::capture_photo_create(
        conn,
        &uuid.to_string(),
        Some(&quail_id.to_string()),
        None,
        &new_path,
        final_thumb.as_deref(),
    )
    .await?;

    Ok(uuid)
}

pub async fn add_event_photo(
    conn: &Connection,
    event_id: Uuid,
    path: String,
    thumbnail_path: Option<String>,
) -> Result<Uuid, AppError> {
    // Benenne Foto mit UUID um
    let (new_path, new_thumb_name) = rename_photo_with_uuid(&path)?;
    let uuid = Uuid::parse_str(new_path.trim_end_matches(".jpg"))
        .map_err(|_| AppError::Other("Invalid UUID from filename".to_string()))?;

    // Benenne auch Thumbnail mit UUID um falls vorhanden
    let final_thumb = if let Some(thumb_path) = thumbnail_path {
        let old_thumb = std::path::Path::new(&thumb_path);
        if let Some(parent_dir) = old_thumb.parent() {
            let new_thumb_path = parent_dir.join(&new_thumb_name);
            if old_thumb.exists() {
                if std::fs::copy(old_thumb, &new_thumb_path).is_ok() {
                    let _ = std::fs::remove_file(old_thumb);
                    Some(new_thumb_name)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    conn.execute(
        "INSERT INTO photos (uuid, event_id, path, thumbnail_path) VALUES (?1, ?2, ?3, ?4)",
        params![
            uuid.to_string(),
            event_id.to_string(),
            &new_path,
            &final_thumb
        ],
    )?;

    // Capture CRDT operation
    crate::services::operation_capture::capture_photo_create(
        conn,
        &uuid.to_string(),
        None,
        Some(&event_id.to_string()),
        &new_path,
        final_thumb.as_deref(),
    )
    .await?;

    Ok(uuid)
}

pub fn list_quail_photos(conn: &Connection, quail_uuid: &Uuid) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, quail_id, event_id, COALESCE(relative_path, path) as rel_path, thumbnail_path 
         FROM photos 
         WHERE quail_id = ?1 AND deleted = 0",
    )?;
    let rows = stmt.query_map(params![quail_uuid.to_string()], |row| {
        let uuid_str: String = row.get(0)?;
        let quail_id_str: Option<String> = row.get(1)?;
        let event_id_str: Option<String> = row.get(2)?;
        let relative_path: String = row.get(3)?;
        let relative_thumb: Option<String> = row.get(4)?;

        Ok(Photo {
            uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
            quail_id: quail_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
            event_id: event_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_event_photos(conn: &Connection, event_uuid: &Uuid) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, quail_id, event_id, path, thumbnail_path FROM photos WHERE event_id = ?1",
    )?;
    let rows = stmt.query_map(params![event_uuid.to_string()], |row| {
        let uuid_str: String = row.get(0)?;
        let quail_id_str: Option<String> = row.get(1)?;
        let event_id_str: Option<String> = row.get(2)?;
        let relative_path: String = row.get(3)?;
        let relative_thumb: Option<String> = row.get(4)?;

        Ok(Photo {
            uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
            quail_id: quail_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
            event_id: event_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_profile_photo(conn: &Connection, quail_uuid: &Uuid) -> Result<Option<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.uuid, p.quail_id, p.event_id, COALESCE(p.relative_path, p.path) as rel_path, p.thumbnail_path 
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

            Ok(Photo {
                uuid: Uuid::parse_str(&uuid_str).map_err(|_| rusqlite::Error::InvalidQuery)?,
                quail_id: quail_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
                event_id: event_id_str.map(|s| Uuid::parse_str(&s).ok()).flatten(),
                path: get_absolute_photo_path(&relative_path),
                thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
            })
        })
        .optional()?;
    Ok(res)
}

pub fn set_profile_photo(
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
