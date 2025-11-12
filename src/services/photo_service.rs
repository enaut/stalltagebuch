use crate::error::AppError;
use crate::models::Photo;
use rusqlite::{params, Connection, OptionalExtension};

/// Gibt den absoluten Pfad zu einem Foto zurück (für UI-Anzeige)
pub fn get_absolute_photo_path(relative_path: &str) -> String {
    #[cfg(target_os = "android")]
    {
        // Fotos sind in /storage/emulated/0/Android/data/PACKAGE/files/photos/
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

/// Benennt eine Foto-Datei mit UUID um und gibt den neuen Pfad zurück
fn rename_photo_with_uuid(original_path: &str) -> Result<(String, String), AppError> {
    eprintln!("=== rename_photo_with_uuid called ===");
    eprintln!("Original path: {}", original_path);

    let uuid = uuid::Uuid::new_v4().to_string();
    let new_filename = format!("{}.jpg", uuid);
    let thumb_filename = format!("{}_thumb.jpg", uuid);

    eprintln!("Generated UUID: {}", uuid);
    eprintln!("New filename: {}", new_filename);

    // Datei ist bereits im richtigen Verzeichnis, einfach dort umbenennen
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

pub fn add_wachtel_photo(
    conn: &Connection,
    wachtel_id: i64,
    path: String,
    thumbnail_path: Option<String>,
    is_profile: bool,
) -> Result<i64, AppError> {
    eprintln!("=== add_wachtel_photo called ===");
    eprintln!(
        "Wachtel ID: {}, Path: {}, Thumbnail: {:?}, Is profile: {}",
        wachtel_id, path, thumbnail_path, is_profile
    );

    // Benenne Foto mit UUID um
    let (new_path, new_thumb_name) = rename_photo_with_uuid(&path)?;
    let uuid = new_path.trim_end_matches(".jpg").to_string();
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
        "INSERT INTO photos (uuid, wachtel_id, path, thumbnail_path, is_profile) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            uuid,
            wachtel_id,
            new_path,
            final_thumb,
            if is_profile { 1 } else { 0 }
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn add_event_photo(
    conn: &Connection,
    event_id: i64,
    path: String,
    thumbnail_path: Option<String>,
) -> Result<i64, AppError> {
    // Benenne Foto mit UUID um
    let (new_path, new_thumb_name) = rename_photo_with_uuid(&path)?;
    let uuid = new_path.trim_end_matches(".jpg").to_string();

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
        "INSERT INTO photos (uuid, event_id, path, thumbnail_path, is_profile) VALUES (?1, ?2, ?3, ?4, 0)",
        params![uuid, event_id, new_path, final_thumb],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_wachtel_photos(conn: &Connection, wachtel_id: i64) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE wachtel_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![wachtel_id], |row| {
        let relative_path: String = row.get(4)?;
        let relative_thumb: Option<String> = row.get(5)?;

        Ok(Photo {
            id: row.get(0)?,
            uuid: row.get(1)?,
            wachtel_id: row.get(2)?,
            event_id: row.get(3)?,
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
            is_profile: matches!(row.get::<_, i64>(6)?, 1),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_event_photos(conn: &Connection, event_id: i64) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE event_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![event_id], |row| {
        let relative_path: String = row.get(4)?;
        let relative_thumb: Option<String> = row.get(5)?;

        Ok(Photo {
            id: row.get(0)?,
            uuid: row.get(1)?,
            wachtel_id: row.get(2)?,
            event_id: row.get(3)?,
            path: get_absolute_photo_path(&relative_path),
            thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
            is_profile: matches!(row.get::<_, i64>(6)?, 1),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_profile_photo(conn: &Connection, wachtel_id: i64) -> Result<Option<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE wachtel_id = ?1 AND is_profile = 1 ORDER BY id LIMIT 1"
    )?;
    let res = stmt
        .query_row(params![wachtel_id], |row| {
            let relative_path: String = row.get(4)?;
            let relative_thumb: Option<String> = row.get(5)?;

            Ok(Photo {
                id: row.get(0)?,
                uuid: row.get(1)?,
                wachtel_id: row.get(2)?,
                event_id: row.get(3)?,
                path: get_absolute_photo_path(&relative_path),
                thumbnail_path: relative_thumb.map(|t| get_absolute_photo_path(&t)),
                is_profile: true,
            })
        })
        .optional()?;
    Ok(res)
}

pub fn set_profile_photo(
    conn: &Connection,
    wachtel_id: i64,
    photo_id: i64,
) -> Result<(), AppError> {
    // Setze alle auf 0
    conn.execute(
        "UPDATE photos SET is_profile = 0 WHERE wachtel_id = ?1",
        params![wachtel_id],
    )?;
    // Ziel auf 1
    let rows = conn.execute(
        "UPDATE photos SET is_profile = 1 WHERE id = ?1 AND wachtel_id = ?2",
        params![photo_id, wachtel_id],
    )?;
    if rows == 0 {
        return Err(AppError::NotFound(
            "Foto nicht gefunden oder gehört nicht zur Wachtel".into(),
        ));
    }
    Ok(())
}

pub fn delete_photo(conn: &Connection, photo_id: i64) -> Result<(), AppError> {
    let rows = conn.execute("DELETE FROM photos WHERE id = ?1", params![photo_id])?;
    if rows == 0 {
        return Err(AppError::NotFound("Foto nicht gefunden".into()));
    }
    Ok(())
}
