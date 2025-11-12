use crate::error::AppError;
use crate::models::SyncSettings;
use rusqlite::{Connection, Result};

/// Lädt die Synchronisierungseinstellungen aus der Datenbank
pub fn load_sync_settings(conn: &Connection) -> Result<Option<SyncSettings>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, server_url, username, app_password, remote_path, enabled, last_sync, created_at, updated_at 
         FROM sync_settings 
         ORDER BY id DESC 
         LIMIT 1"
    )?;

    let result = stmt.query_row([], |row| {
        Ok(SyncSettings {
            id: row.get(0)?,
            server_url: row.get(1)?,
            username: row.get(2)?,
            app_password: row.get(3)?,
            remote_path: row.get(4)?,
            enabled: row.get(5)?,
            last_sync: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    });

    match result {
        Ok(settings) => Ok(Some(settings)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(AppError::Database(e)),
    }
}

/// Speichert oder aktualisiert die Synchronisierungseinstellungen
pub fn save_sync_settings(
    conn: &Connection,
    settings: &SyncSettings,
) -> Result<i64, AppError> {
    // Prüfe ob bereits Einstellungen existieren
    let existing = load_sync_settings(conn)?;

    if let Some(existing) = existing {
        // Update
        conn.execute(
            "UPDATE sync_settings 
             SET server_url = ?1, username = ?2, app_password = ?3, remote_path = ?4, enabled = ?5
             WHERE id = ?6",
            (
                &settings.server_url,
                &settings.username,
                &settings.app_password,
                &settings.remote_path,
                settings.enabled,
                existing.id,
            ),
        )?;
        Ok(existing.id)
    } else {
        // Insert
        conn.execute(
            "INSERT INTO sync_settings (server_url, username, app_password, remote_path, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                &settings.server_url,
                &settings.username,
                &settings.app_password,
                &settings.remote_path,
                settings.enabled,
            ),
        )?;
        Ok(conn.last_insert_rowid())
    }
}

/// Aktualisiert den Zeitstempel der letzten Synchronisierung
pub fn update_last_sync(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "UPDATE sync_settings SET last_sync = CURRENT_TIMESTAMP WHERE id = (SELECT MAX(id) FROM sync_settings)",
        [],
    )?;
    Ok(())
}

/// Aktiviert oder deaktiviert die Synchronisierung
pub fn set_sync_enabled(conn: &Connection, enabled: bool) -> Result<(), AppError> {
    conn.execute(
        "UPDATE sync_settings SET enabled = ?1 WHERE id = (SELECT MAX(id) FROM sync_settings)",
        [enabled],
    )?;
    Ok(())
}

/// Löscht alle Synchronisierungseinstellungen
pub fn delete_sync_settings(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM sync_settings", [])?;
    Ok(())
}
