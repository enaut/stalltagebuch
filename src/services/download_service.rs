use crate::error::AppError;
use crate::services::{crdt_service, sync_paths, sync_service};
use rusqlite::Connection;
use std::collections::HashMap;

/// Downloads and merges operations from sync/ops/ directory
///
/// This is a minimal skeleton for the new multi-master sync downloader.
pub async fn download_and_merge_ops(conn: &Connection) -> Result<usize, AppError> {
    let settings = sync_service::load_sync_settings(conn)?
        .ok_or_else(|| AppError::NotFound("Sync settings not configured".to_string()))?;

    if !settings.enabled {
        return Err(AppError::Validation("Sync disabled".to_string()));
    }

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

    // Get local manifest from sync_checkpoint
    let mut manifest = load_manifest(conn)?;

    let ops_base_path = format!(
        "{}/{}",
        settings.remote_path.trim_end_matches('/'),
        sync_paths::OPS_DIR
    );

    // List all devices in ops/
    let device_dirs = list_directory(&client, &ops_base_path).await?;

    let mut all_ops = Vec::new();

    for device_dir in device_dirs {
        let device_path = format!("{}/{}", ops_base_path, device_dir);

        // List all year-month directories for this device
        let month_dirs = list_directory(&client, &device_path).await?;

        for month_dir in month_dirs {
            let month_path = format!("{}/{}", device_path, month_dir);

            // List all NDJSON files in this month
            let files = list_files_with_etags(&client, &month_path).await?;

            for (filename, etag) in files {
                let file_path = format!("{}/{}", month_path, filename);

                // Check if we already have this version
                if manifest.get(&file_path) == Some(&etag) {
                    continue; // Already downloaded
                }

                // Download and parse
                let response = client
                    .get(&file_path)
                    .await
                    .map_err(|e| AppError::Other(format!("Download failed: {:?}", e)))?;
                
                let content_bytes = response.bytes().await
                    .map_err(|e| AppError::Other(format!("Read response failed: {:?}", e)))?;

                let content_str = String::from_utf8(content_bytes.to_vec())
                    .map_err(|e| AppError::Other(format!("UTF-8 decode failed: {}", e)))?;

                // Parse NDJSON
                for line in content_str.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }

                    let op: crdt_service::Operation = serde_json::from_str(line)
                        .map_err(|e| AppError::Other(format!("JSON parse failed: {}", e)))?;

                    all_ops.push(op);
                }

                // Update manifest
                manifest.insert(file_path.clone(), etag);
            }
        }
    }

    // Sort operations by clock (deterministic total order)
    all_ops.sort_by(|a, b| a.clock.cmp(&b.clock));

    // Apply operations
    let ops_applied = apply_operations(conn, &all_ops)?;

    // Save updated manifest
    save_manifest(conn, &manifest)?;

    eprintln!(
        "Downloaded and merged {} operations from {} files",
        ops_applied,
        manifest.len()
    );

    Ok(ops_applied)
}

/// Lists directory contents (subdirectories or files)
async fn list_directory(
    client: &reqwest_dav::Client,
    path: &str,
) -> Result<Vec<String>, AppError> {
    let list_result = client
        .list(path, reqwest_dav::Depth::Number(1))
        .await
        .map_err(|e| AppError::Other(format!("PROPFIND failed: {:?}", e)))?;

    let mut names = Vec::new();

    for item in list_result {
        if let reqwest_dav::list_cmd::ListEntity::File(file) = item {
            let name = file.href
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() {
                names.push(name);
            }
        } else if let reqwest_dav::list_cmd::ListEntity::Folder(folder) = item {
            let name = folder.href
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or("")
                .to_string();
            if !name.is_empty() && name != path.split('/').last().unwrap_or("") {
                names.push(name);
            }
        }
    }

    Ok(names)
}

/// Lists files with their ETags
async fn list_files_with_etags(
    client: &reqwest_dav::Client,
    path: &str,
) -> Result<Vec<(String, String)>, AppError> {
    let list_result = client
        .list(path, reqwest_dav::Depth::Number(1))
        .await
        .map_err(|e| AppError::Other(format!("PROPFIND failed: {:?}", e)))?;

    let mut files = Vec::new();

    for item in list_result {
        if let reqwest_dav::list_cmd::ListEntity::File(file) = item {
            let filename = file.href
                .trim_end_matches('/')
                .split('/')
                .last()
                .unwrap_or("")
                .to_string();

            if filename.ends_with(".ndjson") {
                let etag = file.tag.unwrap_or_default();
                files.push((filename, etag));
            }
        }
    }

    Ok(files)
}

/// Loads manifest from sync_checkpoint table
fn load_manifest(conn: &Connection) -> Result<HashMap<String, String>, AppError> {
    let mut manifest = HashMap::new();

    let mut stmt = conn
        .prepare("SELECT path, etag FROM sync_manifest")
        .or_else(|_| {
            // Table doesn't exist yet, create it
            conn.execute(
                "CREATE TABLE IF NOT EXISTS sync_manifest (
                    path TEXT PRIMARY KEY,
                    etag TEXT NOT NULL
                )",
                [],
            )?;
            conn.prepare("SELECT path, etag FROM sync_manifest")
        })?;

    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;

    for row in rows {
        let (path, etag) = row?;
        manifest.insert(path, etag);
    }

    Ok(manifest)
}

/// Saves manifest to sync_manifest table
fn save_manifest(conn: &Connection, manifest: &HashMap<String, String>) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;

    for (path, etag) in manifest {
        tx.execute(
            "INSERT OR REPLACE INTO sync_manifest (path, etag) VALUES (?1, ?2)",
            rusqlite::params![path, etag],
        )?;
    }

    tx.commit()?;

    Ok(())
}

/// Applies operations to local database
fn apply_operations(
    conn: &Connection,
    ops: &[crdt_service::Operation],
) -> Result<usize, AppError> {
    let tx = conn.unchecked_transaction()?;
    let mut applied = 0;

    for op in ops {
        // Check if operation already applied (idempotency)
        let already_applied: bool = tx
            .query_row(
                "SELECT 1 FROM op_log WHERE op_id = ?1",
                rusqlite::params![&op.op_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if already_applied {
            continue;
        }

        // Apply based on entity type
        match op.entity_type.as_str() {
            "quail" => apply_quail_op(&tx, op)?,
            "event" => apply_event_op(&tx, op)?,
            "photo" => apply_photo_op(&tx, op)?,
            "egg" => apply_egg_op(&tx, op)?,
            _ => {
                eprintln!("Unknown entity type: {}", op.entity_type);
                continue;
            }
        }

        // Record in op_log
        let op_kind_str = serde_json::to_string(&op.op)
            .map_err(|e| AppError::Other(format!("Serialize op_kind failed: {}", e)))?;
        tx.execute(
            "INSERT INTO op_log (
                op_id, entity_type, entity_id, ts, logical_counter, device_id, op_kind, payload
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &op.op_id,
                &op.entity_type,
                &op.entity_id,
                op.clock.ts,
                op.clock.logical_counter,
                &op.clock.device_id,
                op_kind_str,
                "" // payload unused for now
            ],
        )?;

        applied += 1;
    }

    tx.commit()?;

    Ok(applied)
}

/// Applies a quail operation (LWW-Register merge)
fn apply_quail_op(
    tx: &rusqlite::Transaction,
    op: &crdt_service::Operation,
) -> Result<(), AppError> {
    use crate::services::crdt_service::CrdtOp;

    match &op.op {
        CrdtOp::LwwSet { field, value } => {
            // Check current logical_clock
            let current: Option<(i64, i32)> = tx
                .query_row(
                    "SELECT logical_clock, deleted FROM quails WHERE uuid = ?1",
                    rusqlite::params![&op.entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            // Only apply if this is newer
            if let Some((current_clock, deleted)) = current {
                if current_clock >= op.clock.ts || deleted == 1 {
                    return Ok(()); // Skip older operation or deleted entity
                }
            }

            // Apply field update
            match field.as_str() {
                "name" => {
                    let name = value.as_str().ok_or_else(|| AppError::Validation("Invalid name value".to_string()))?;
                    tx.execute(
                        "INSERT OR REPLACE INTO quails (uuid, name, gender, ring_color, profile_photo, rev, logical_clock, deleted)
                         SELECT ?1, ?2, COALESCE(gender, 'unknown'), ring_color, profile_photo, ?3, ?3, 0
                         FROM (SELECT NULL) LEFT JOIN quails ON uuid = ?1",
                        rusqlite::params![&op.entity_id, name, op.clock.ts],
                    )?;
                }
                "gender" => {
                    let gender = value.as_str().ok_or_else(|| AppError::Validation("Invalid gender value".to_string()))?;
                    tx.execute(
                        "UPDATE quails SET gender = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![gender, op.clock.ts, &op.entity_id],
                    )?;
                }
                "ring_color" => {
                    let color = value.as_str();
                    tx.execute(
                        "UPDATE quails SET ring_color = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![color, op.clock.ts, &op.entity_id],
                    )?;
                }
                "profile_photo" => {
                    let photo = value.as_str();
                    tx.execute(
                        "UPDATE quails SET profile_photo = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![photo, op.clock.ts, &op.entity_id],
                    )?;
                }
                _ => {
                    eprintln!("Unknown quail field: {}", field);
                }
            }
        }
        CrdtOp::Delete => {
            tx.execute(
                "UPDATE quails SET deleted = 1, logical_clock = ?1 WHERE uuid = ?2",
                rusqlite::params![op.clock.ts, &op.entity_id],
            )?;
        }
        _ => {} // Other ops not applicable to quails
    }

    Ok(())
}

/// Applies an event operation
fn apply_event_op(
    tx: &rusqlite::Transaction,
    op: &crdt_service::Operation,
) -> Result<(), AppError> {
    use crate::services::crdt_service::CrdtOp;

    match &op.op {
        CrdtOp::LwwSet { field, value } => {
            let current: Option<(i64, i32)> = tx
                .query_row(
                    "SELECT logical_clock, deleted FROM quail_events WHERE uuid = ?1",
                    rusqlite::params![&op.entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((current_clock, deleted)) = current {
                if current_clock >= op.clock.ts || deleted == 1 {
                    return Ok(());
                }
            }

            match field.as_str() {
                "quail_id" => {
                    let quail_id = value.as_str().ok_or_else(|| AppError::Validation("Invalid quail_id".to_string()))?;
                    tx.execute(
                        "INSERT OR REPLACE INTO quail_events (uuid, quail_id, event_type, event_date, notes, rev, logical_clock, deleted)
                         SELECT ?1, ?2, COALESCE(event_type, 'alive'), COALESCE(event_date, date('now')), notes, ?3, ?3, 0
                         FROM (SELECT NULL) LEFT JOIN quail_events ON uuid = ?1",
                        rusqlite::params![&op.entity_id, quail_id, op.clock.ts],
                    )?;
                }
                "event_type" => {
                    let event_type = value.as_str().ok_or_else(|| AppError::Validation("Invalid event_type".to_string()))?;
                    tx.execute(
                        "UPDATE quail_events SET event_type = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![event_type, op.clock.ts, &op.entity_id],
                    )?;
                }
                "event_date" => {
                    let event_date = value.as_str().ok_or_else(|| AppError::Validation("Invalid event_date".to_string()))?;
                    tx.execute(
                        "UPDATE quail_events SET event_date = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![event_date, op.clock.ts, &op.entity_id],
                    )?;
                }
                "notes" => {
                    let notes = value.as_str();
                    tx.execute(
                        "UPDATE quail_events SET notes = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![notes, op.clock.ts, &op.entity_id],
                    )?;
                }
                _ => {
                    eprintln!("Unknown event field: {}", field);
                }
            }
        }
        CrdtOp::Delete => {
            tx.execute(
                "UPDATE quail_events SET deleted = 1, logical_clock = ?1 WHERE uuid = ?2",
                rusqlite::params![op.clock.ts, &op.entity_id],
            )?;
        }
        _ => {}
    }

    Ok(())
}

/// Applies a photo operation
fn apply_photo_op(
    tx: &rusqlite::Transaction,
    op: &crdt_service::Operation,
) -> Result<(), AppError> {
    use crate::services::crdt_service::CrdtOp;

    match &op.op {
        CrdtOp::LwwSet { field, value } => {
            let current: Option<(i64, i32)> = tx
                .query_row(
                    "SELECT logical_clock, deleted FROM photos WHERE uuid = ?1",
                    rusqlite::params![&op.entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((current_clock, deleted)) = current {
                if current_clock >= op.clock.ts || deleted == 1 {
                    return Ok(());
                }
            }

            match field.as_str() {
                "quail_id" => {
                    let quail_id = value.as_str();
                    tx.execute(
                        "UPDATE photos SET quail_id = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![quail_id, op.clock.ts, &op.entity_id],
                    )?;
                }
                "event_id" => {
                    let event_id = value.as_str();
                    tx.execute(
                        "UPDATE photos SET event_id = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![event_id, op.clock.ts, &op.entity_id],
                    )?;
                }
                "relative_path" => {
                    let path = value.as_str().ok_or_else(|| AppError::Validation("Invalid path".to_string()))?;
                    tx.execute(
                        "INSERT OR REPLACE INTO photos (uuid, path, relative_path, quail_id, event_id, thumbnail_path, rev, logical_clock, deleted)
                         SELECT ?1, COALESCE(path, ''), ?2, COALESCE(quail_id, ''), COALESCE(event_id, ''), thumbnail_path, ?3, ?3, 0
                         FROM (SELECT NULL) LEFT JOIN photos ON uuid = ?1",
                        rusqlite::params![&op.entity_id, path, op.clock.ts],
                    )?;
                }
                "relative_thumb" => {
                    let thumb = value.as_str();
                    tx.execute(
                        "UPDATE photos SET thumbnail_path = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![thumb, op.clock.ts, &op.entity_id],
                    )?;
                }
                _ => {
                    eprintln!("Unknown photo field: {}", field);
                }
            }
        }
        CrdtOp::Delete => {
            tx.execute(
                "UPDATE photos SET deleted = 1, logical_clock = ?1 WHERE uuid = ?2",
                rusqlite::params![op.clock.ts, &op.entity_id],
            )?;
        }
        _ => {}
    }

    Ok(())
}

/// Applies an egg operation
fn apply_egg_op(
    tx: &rusqlite::Transaction,
    op: &crdt_service::Operation,
) -> Result<(), AppError> {
    use crate::services::crdt_service::CrdtOp;

    match &op.op {
        CrdtOp::LwwSet { field, value } => {
            let current: Option<(i64, i32)> = tx
                .query_row(
                    "SELECT logical_clock, deleted FROM egg_records WHERE uuid = ?1",
                    rusqlite::params![&op.entity_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .ok();

            if let Some((current_clock, deleted)) = current {
                if current_clock >= op.clock.ts || deleted == 1 {
                    return Ok(());
                }
            }

            match field.as_str() {
                "date" => {
                    let date = value.as_str().ok_or_else(|| AppError::Validation("Invalid date".to_string()))?;
                    tx.execute(
                        "INSERT OR REPLACE INTO egg_records (uuid, record_date, total_eggs, notes, rev, logical_clock, deleted)
                         SELECT ?1, ?2, COALESCE(total_eggs, 0), notes, ?3, ?3, 0
                         FROM (SELECT NULL) LEFT JOIN egg_records ON uuid = ?1",
                        rusqlite::params![&op.entity_id, date, op.clock.ts],
                    )?;
                }
                "count" => {
                    let count = value.as_i64().ok_or_else(|| AppError::Validation("Invalid count".to_string()))? as i32;
                    tx.execute(
                        "UPDATE egg_records SET total_eggs = ?1, logical_clock = ?2 WHERE uuid = ?3",
                        rusqlite::params![count, op.clock.ts, &op.entity_id],
                    )?;
                }
                _ => {
                    eprintln!("Unknown egg field: {}", field);
                }
            }
        }
        CrdtOp::PnIncrement { field, delta } => {
            if field == "count" {
                tx.execute(
                    "UPDATE egg_records SET total_eggs = total_eggs + ?1, logical_clock = ?2 WHERE uuid = ?3",
                    rusqlite::params![delta, op.clock.ts, &op.entity_id],
                )?;
            }
        }
        CrdtOp::Delete => {
            tx.execute(
                "UPDATE egg_records SET deleted = 1, logical_clock = ?1 WHERE uuid = ?2",
                rusqlite::params![op.clock.ts, &op.entity_id],
            )?;
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        let mut manifest = HashMap::new();
        manifest.insert("sync/ops/device1/202412/01JGTEST.ndjson".to_string(), "\"abc123\"".to_string());
        manifest.insert("sync/ops/device2/202412/01JGTEST2.ndjson".to_string(), "\"def456\"".to_string());

        save_manifest(&conn, &manifest).unwrap();
        let loaded = load_manifest(&conn).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.get("sync/ops/device1/202412/01JGTEST.ndjson"), Some(&"\"abc123\"".to_string()));
    }
}
