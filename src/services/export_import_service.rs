// Export/Import service for full local backup

use crate::error::AppError;
use crate::services::photo_service::get_absolute_photo_path;
use base64::Engine as _;
use chrono::Utc;
use rusqlite::{Connection, Row};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportMode {
    MergePreferImport,
}

#[derive(Serialize, Deserialize)]
struct ExportMetadata {
    format_version: u32,
    exported_at: String,
    app_version: String,
}

#[derive(Serialize, Deserialize)]
struct ExportQuails {
    quails: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ExportEvents {
    events: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ExportEggRecords {
    egg_records: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ExportPhotos {
    photos: Vec<serde_json::Value>,
}

fn query_table(conn: &Connection, sql: &str) -> Result<Vec<serde_json::Value>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], row_to_json)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

fn row_to_json(row: &Row<'_>) -> rusqlite::Result<serde_json::Value> {
    let mut obj = serde_json::Map::new();
    for (idx, col) in row.as_ref().column_names().iter().enumerate() {
        let col_name = col.to_string();
        let value: rusqlite::types::Value = row.get(idx)?;
        let json_value = match value {
            rusqlite::types::Value::Null => serde_json::Value::Null,
            rusqlite::types::Value::Integer(v) => serde_json::Value::from(v),
            rusqlite::types::Value::Real(v) => serde_json::Value::from(v),
            rusqlite::types::Value::Text(v) => serde_json::Value::from(v),
            rusqlite::types::Value::Blob(v) => {
                serde_json::Value::from(base64::engine::general_purpose::STANDARD.encode(v))
            }
        };
        obj.insert(col_name, json_value);
    }
    Ok(serde_json::Value::Object(obj))
}

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::Other(format!("Fehler beim Erstellen des Verzeichnisses: {}", e))
        })?;
    }
    Ok(())
}

fn get_export_base_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        PathBuf::from(
            "/storage/emulated/0/Android/data/de.teilgedanken.stalltagebuch/files/exports",
        )
    }

    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from("./exports")
    }
}

pub async fn export_to_zip(conn: &Connection) -> Result<PathBuf, AppError> {
    let base_dir = get_export_base_dir();
    fs::create_dir_all(&base_dir).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Erstellen des Export-Verzeichnisses: {}",
            e
        ))
    })?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let file_name = format!("stalltagebuch-export-{}.zip", timestamp);
    let export_path = base_dir.join(file_name);

    ensure_parent_dir(&export_path)?;

    let file = fs::File::create(&export_path)
        .map_err(|e| AppError::Other(format!("Fehler beim Erstellen der Exportdatei: {}", e)))?;

    let mut zip = zip::ZipWriter::new(file);
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // metadata.json
    let metadata = ExportMetadata {
        format_version: 1,
        exported_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    let metadata_json = serde_json::to_vec_pretty(&metadata).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Serialisieren von metadata.json: {}",
            e
        ))
    })?;
    zip.start_file("metadata.json", options).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Hinzufügen von metadata.json zum ZIP: {}",
            e
        ))
    })?;
    zip.write_all(&metadata_json).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Schreiben von metadata.json ins ZIP: {}",
            e
        ))
    })?;

    // Tabellen exportieren
    let quails = query_table(conn, "SELECT * FROM quails")?;
    let events = query_table(conn, "SELECT * FROM quail_events")?;
    let egg_records = query_table(conn, "SELECT * FROM egg_records")?;
    let photos = query_table(conn, "SELECT * FROM photos")?;

    let quails_json = serde_json::to_vec_pretty(&ExportQuails { quails }).map_err(|e| {
        AppError::Other(format!("Fehler beim Serialisieren von quails.json: {}", e))
    })?;
    zip.start_file("data/quails.json", options).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Hinzufügen von data/quails.json: {}",
            e
        ))
    })?;
    zip.write_all(&quails_json).map_err(|e| {
        AppError::Other(format!("Fehler beim Schreiben von data/quails.json: {}", e))
    })?;

    let events_json = serde_json::to_vec_pretty(&ExportEvents { events }).map_err(|e| {
        AppError::Other(format!("Fehler beim Serialisieren von events.json: {}", e))
    })?;
    zip.start_file("data/events.json", options).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Hinzufügen von data/events.json: {}",
            e
        ))
    })?;
    zip.write_all(&events_json).map_err(|e| {
        AppError::Other(format!("Fehler beim Schreiben von data/events.json: {}", e))
    })?;

    let egg_records_json =
        serde_json::to_vec_pretty(&ExportEggRecords { egg_records }).map_err(|e| {
            AppError::Other(format!(
                "Fehler beim Serialisieren von egg_records.json: {}",
                e
            ))
        })?;
    zip.start_file("data/egg_records.json", options)
        .map_err(|e| {
            AppError::Other(format!(
                "Fehler beim Hinzufügen von data/egg_records.json: {}",
                e
            ))
        })?;
    zip.write_all(&egg_records_json).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Schreiben von data/egg_records.json: {}",
            e
        ))
    })?;

    let photos_json = serde_json::to_vec_pretty(&ExportPhotos { photos }).map_err(|e| {
        AppError::Other(format!("Fehler beim Serialisieren von photos.json: {}", e))
    })?;
    zip.start_file("data/photos.json", options).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Hinzufügen von data/photos.json: {}",
            e
        ))
    })?;
    zip.write_all(&photos_json).map_err(|e| {
        AppError::Other(format!("Fehler beim Schreiben von data/photos.json: {}", e))
    })?;

    // Fotos exportieren (nur Originale anhand von relative_path/path)
    let mut stmt = conn.prepare(
        "SELECT COALESCE(relative_path, path) as rel_path FROM photos WHERE deleted = 0",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let rel_path: String = row.get(0)?;
        let abs_path = get_absolute_photo_path(&rel_path);
        let abs = PathBuf::from(&abs_path);
        if abs.exists() {
            let mut file_data = Vec::new();
            let mut f = fs::File::open(&abs)
                .map_err(|e| AppError::Other(format!("Fehler beim Öffnen eines Fotos: {}", e)))?;
            f.read_to_end(&mut file_data)
                .map_err(|e| AppError::Other(format!("Fehler beim Lesen eines Fotos: {}", e)))?;

            let zip_path = format!("photos/{}", rel_path);
            zip.start_file(zip_path, options).map_err(|e| {
                AppError::Other(format!("Fehler beim Hinzufügen eines Fotos zum ZIP: {}", e))
            })?;
            zip.write_all(&file_data).map_err(|e| {
                AppError::Other(format!("Fehler beim Schreiben eines Fotos ins ZIP: {}", e))
            })?;
        }
    }

    zip.finish()
        .map_err(|e| AppError::Other(format!("Fehler beim finalisieren der ZIP-Datei: {}", e)))?;

    Ok(export_path)
}

pub async fn import_from_zip(
    conn: &Connection,
    import_path: &Path,
    _mode: ImportMode,
) -> Result<(), AppError> {
    let file = fs::File::open(import_path)
        .map_err(|e| AppError::Other(format!("Fehler beim Öffnen der Importdatei: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Other(format!("Fehler beim Lesen der ZIP-Datei: {}", e)))?;

    // JSON-Dateien einlesen
    let mut read_json = |name: &str| -> Result<Option<serde_json::Value>, AppError> {
        match archive.by_name(name) {
            Ok(mut f) => {
                let mut buf = String::new();
                f.read_to_string(&mut buf).map_err(|e| {
                    AppError::Other(format!("Fehler beim Lesen von {}: {}", name, e))
                })?;
                let v: serde_json::Value = serde_json::from_str(&buf).map_err(|e| {
                    AppError::Other(format!("Fehler beim Parsen von {}: {}", name, e))
                })?;
                Ok(Some(v))
            }
            Err(_) => Ok(None),
        }
    };

    let quails_v = read_json("data/quails.json")?;
    let events_v = read_json("data/events.json")?;
    let egg_records_v = read_json("data/egg_records.json")?;
    let photos_v = read_json("data/photos.json")?;

    let tx = conn.unchecked_transaction()?;

    // merge_prefer_import: vorhandene Datensätze mit derselben UUID überschreiben

    if let Some(serde_json::Value::Object(obj)) = quails_v {
        if let Some(serde_json::Value::Array(quails)) = obj.get("quails") {
            for q in quails {
                if let Some(uuid) = q.get("uuid").and_then(|v| v.as_str()) {
                    tx.execute("DELETE FROM quails WHERE uuid = ?1", [uuid])?;
                }
                let json_str = serde_json::to_string(q).unwrap_or_default();
                tx.execute("INSERT INTO quails (uuid, name, gender, ring_color, profile_photo, created_at, updated_at, rev, logical_clock, deleted) VALUES (
                    json_extract(?1, '$.uuid'),
                    json_extract(?1, '$.name'),
                    json_extract(?1, '$.gender'),
                    json_extract(?1, '$.ring_color'),
                    json_extract(?1, '$.profile_photo'),
                    COALESCE(json_extract(?1, '$.created_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.updated_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.rev'), 0),
                    COALESCE(json_extract(?1, '$.logical_clock'), 0),
                    COALESCE(json_extract(?1, '$.deleted'), 0)
                )", [json_str])?;
            }
        }
    }

    if let Some(serde_json::Value::Object(obj)) = events_v {
        if let Some(serde_json::Value::Array(events)) = obj.get("events") {
            for e in events {
                if let Some(uuid) = e.get("uuid").and_then(|v| v.as_str()) {
                    tx.execute("DELETE FROM quail_events WHERE uuid = ?1", [uuid])?;
                }
                let json_str = serde_json::to_string(e).unwrap_or_default();
                tx.execute("INSERT INTO quail_events (uuid, quail_id, event_type, event_date, notes, created_at, updated_at, rev, logical_clock, deleted) VALUES (
                    json_extract(?1, '$.uuid'),
                    json_extract(?1, '$.quail_id'),
                    json_extract(?1, '$.event_type'),
                    json_extract(?1, '$.event_date'),
                    json_extract(?1, '$.notes'),
                    COALESCE(json_extract(?1, '$.created_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.updated_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.rev'), 0),
                    COALESCE(json_extract(?1, '$.logical_clock'), 0),
                    COALESCE(json_extract(?1, '$.deleted'), 0)
                )", [json_str])?;
            }
        }
    }

    if let Some(serde_json::Value::Object(obj)) = egg_records_v {
        if let Some(serde_json::Value::Array(records)) = obj.get("egg_records") {
            for r in records {
                if let Some(uuid) = r.get("uuid").and_then(|v| v.as_str()) {
                    tx.execute("DELETE FROM egg_records WHERE uuid = ?1", [uuid])?;
                }
                let json_str = serde_json::to_string(r).unwrap_or_default();
                tx.execute("INSERT INTO egg_records (uuid, record_date, total_eggs, notes, created_at, updated_at, rev, logical_clock, deleted) VALUES (
                    json_extract(?1, '$.uuid'),
                    json_extract(?1, '$.record_date'),
                    json_extract(?1, '$.total_eggs'),
                    json_extract(?1, '$.notes'),
                    COALESCE(json_extract(?1, '$.created_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.updated_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.rev'), 0),
                    COALESCE(json_extract(?1, '$.logical_clock'), 0),
                    COALESCE(json_extract(?1, '$.deleted'), 0)
                )", [json_str])?;
            }
        }
    }

    if let Some(serde_json::Value::Object(obj)) = photos_v {
        if let Some(serde_json::Value::Array(photos)) = obj.get("photos") {
            for p in photos {
                if let Some(uuid) = p.get("uuid").and_then(|v| v.as_str()) {
                    tx.execute("DELETE FROM photos WHERE uuid = ?1", [uuid])?;
                }
                let json_str = serde_json::to_string(p).unwrap_or_default();
                tx.execute("INSERT INTO photos (uuid, quail_id, event_id, path, relative_path, thumbnail_path, thumbnail_small_path, thumbnail_medium_path, sync_status, sync_error, last_sync_attempt, retry_count, created_at, updated_at, rev, logical_clock, deleted) VALUES (
                    json_extract(?1, '$.uuid'),
                    json_extract(?1, '$.quail_id'),
                    json_extract(?1, '$.event_id'),
                    json_extract(?1, '$.path'),
                    json_extract(?1, '$.relative_path'),
                    json_extract(?1, '$.thumbnail_path'),
                    json_extract(?1, '$.thumbnail_small_path'),
                    json_extract(?1, '$.thumbnail_medium_path'),
                    COALESCE(json_extract(?1, '$.sync_status'), 'local_only'),
                    json_extract(?1, '$.sync_error'),
                    json_extract(?1, '$.last_sync_attempt'),
                    COALESCE(json_extract(?1, '$.retry_count'), 0),
                    COALESCE(json_extract(?1, '$.created_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.updated_at'), CURRENT_TIMESTAMP),
                    COALESCE(json_extract(?1, '$.rev'), 0),
                    COALESCE(json_extract(?1, '$.logical_clock'), 0),
                    COALESCE(json_extract(?1, '$.deleted'), 0)
                )", [json_str])?;
            }
        }
    }

    tx.commit()?;

    // Fotos extrahieren
    let photos_base = if cfg!(target_os = "android") {
        PathBuf::from("/storage/emulated/0/Android/data/de.teilgedanken.stalltagebuch/files/photos")
    } else {
        PathBuf::from("./photos")
    };
    fs::create_dir_all(&photos_base).map_err(|e| {
        AppError::Other(format!(
            "Fehler beim Erstellen des Foto-Verzeichnisses: {}",
            e
        ))
    })?;

    let num_files = archive.len();
    for i in 0..num_files {
        let mut file = archive
            .by_index(i)
            .map_err(|e| AppError::Other(format!("Fehler beim Zugriff auf ZIP-Eintrag: {}", e)))?;
        let name = file.name().to_string();
        if name.starts_with("photos/") && !name.ends_with('/') {
            let rel = &name["photos/".len()..];
            let target_path = photos_base.join(rel);
            ensure_parent_dir(&target_path)?;
            let mut out_file = fs::File::create(&target_path).map_err(|e| {
                AppError::Other(format!("Fehler beim Erstellen einer Fotodatei: {}", e))
            })?;
            std::io::copy(&mut file, &mut out_file).map_err(|e| {
                AppError::Other(format!("Fehler beim Schreiben einer Fotodatei: {}", e))
            })?;
        }
    }

    Ok(())
}
