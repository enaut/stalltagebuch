use crate::error::AppError;
use crate::models::Photo;
use rusqlite::{params, Connection};

pub fn add_wachtel_photo(
    conn: &Connection,
    wachtel_id: i64,
    path: String,
    thumbnail_path: Option<String>,
    is_profile: bool,
) -> Result<i64, AppError> {
    conn.execute(
        "INSERT INTO photos (wachtel_id, path, thumbnail_path, is_profile) VALUES (?1, ?2, ?3, ?4)",
        params![
            wachtel_id,
            path,
            thumbnail_path,
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
    conn.execute(
        "INSERT INTO photos (event_id, path, thumbnail_path, is_profile) VALUES (?1, ?2, ?3, 0)",
        params![event_id, path, thumbnail_path],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_wachtel_photos(conn: &Connection, wachtel_id: i64) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE wachtel_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![wachtel_id], |row| {
        Ok(Photo {
            id: row.get(0)?,
            wachtel_id: row.get(1)?,
            event_id: row.get(2)?,
            path: row.get(3)?,
            thumbnail_path: row.get(4)?,
            is_profile: matches!(row.get::<_, i64>(5)?, 1),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_event_photos(conn: &Connection, event_id: i64) -> Result<Vec<Photo>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE event_id = ?1 ORDER BY id"
    )?;
    let rows = stmt.query_map(params![event_id], |row| {
        Ok(Photo {
            id: row.get(0)?,
            wachtel_id: row.get(1)?,
            event_id: row.get(2)?,
            path: row.get(3)?,
            thumbnail_path: row.get(4)?,
            is_profile: matches!(row.get::<_, i64>(5)?, 1),
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get_profile_photo(conn: &Connection, wachtel_id: i64) -> Result<Option<Photo>, AppError> {
    use rusqlite::OptionalExtension;
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_id, path, thumbnail_path, is_profile FROM photos WHERE wachtel_id = ?1 AND is_profile = 1 ORDER BY id LIMIT 1"
    )?;
    let res = stmt
        .query_row(params![wachtel_id], |row| {
            Ok(Photo {
                id: row.get(0)?,
                wachtel_id: row.get(1)?,
                event_id: row.get(2)?,
                path: row.get(3)?,
                thumbnail_path: row.get(4)?,
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
