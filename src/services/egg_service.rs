use crate::error::AppError;
use crate::models::EggRecord;
use rusqlite::{params, Connection};
use uuid::Uuid;

/// Creates a new egg record
pub async fn add_egg_record(conn: &Connection, record: &EggRecord) -> Result<Uuid, AppError> {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();

    conn.execute(
        "INSERT INTO egg_records (uuid, record_date, total_eggs, notes) VALUES (?1, ?2, ?3, ?4)",
        params![
            record.uuid.to_string(),
            &date_str,
            record.total_eggs,
            &record.notes
        ],
    )?;

    // Capture CRDT operation
    crate::services::operation_capture::capture_egg_create(
        conn,
        &record.uuid.to_string(),
        &date_str,
        record.total_eggs,
    )
    .await?;

    Ok(record.uuid)
}

/// Loads an egg record for a specific date
pub fn get_egg_record(conn: &Connection, date: &str) -> Result<EggRecord, AppError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, record_date, total_eggs, notes 
         FROM egg_records 
         WHERE record_date = ?1 AND deleted = 0",
    )?;

    let record = stmt.query_row(params![date], |row| EggRecord::try_from(row))?;

    Ok(record)
}

/// Updates an existing egg record
pub async fn update_egg_record(conn: &Connection, record: &EggRecord) -> Result<(), AppError> {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();

    let rows_affected = conn.execute(
        "UPDATE egg_records 
         SET total_eggs = ?1, notes = ?2, updated_at = CURRENT_TIMESTAMP 
         WHERE record_date = ?3",
        params![record.total_eggs, &record.notes, &date_str],
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Record for {} not found",
            date_str
        )));
    }

    // Capture CRDT operation
    crate::services::operation_capture::capture_egg_update(
        conn,
        &record.uuid.to_string(),
        record.total_eggs,
    )
    .await?;

    Ok(())
}

/// Deletes an egg record
pub async fn delete_egg_record(conn: &Connection, date: &str) -> Result<(), AppError> {
    // Get UUID before deletion
    let uuid: String = conn.query_row(
        "SELECT uuid FROM egg_records WHERE record_date = ?1",
        params![date],
        |row| row.get(0),
    )?;

    let rows_affected = conn.execute(
        "DELETE FROM egg_records WHERE record_date = ?1",
        params![date],
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Record for {} not found", date)));
    }

    // Capture CRDT deletion
    crate::services::operation_capture::capture_egg_delete(conn, &uuid).await?;

    Ok(())
}

/// Loads all egg records for a time period (sorted by date descending)
pub fn list_egg_records(
    conn: &Connection,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<EggRecord>, AppError> {
    let query = match (start_date, end_date) {
        (Some(_), Some(_)) => {
            "SELECT uuid, record_date, total_eggs, notes 
             FROM egg_records 
             WHERE record_date BETWEEN ?1 AND ?2 AND deleted = 0
             ORDER BY record_date DESC"
        }
        (Some(_), None) => {
            "SELECT uuid, record_date, total_eggs, notes 
             FROM egg_records 
             WHERE record_date >= ?1 AND deleted = 0
             ORDER BY record_date DESC"
        }
        (None, Some(_)) => {
            "SELECT uuid, record_date, total_eggs, notes 
             FROM egg_records 
             WHERE record_date <= ?1 AND deleted = 0
             ORDER BY record_date DESC"
        }
        (None, None) => {
            "SELECT uuid, record_date, total_eggs, notes 
             FROM egg_records 
             WHERE deleted = 0
             ORDER BY record_date DESC"
        }
    };

    let mut stmt = conn.prepare(query)?;

    let mut out = Vec::new();
    match (start_date, end_date) {
        (Some(start), Some(end)) => {
            let rows = stmt.query_map(params![start, end], |row| EggRecord::try_from(row))?;
            for r in rows {
                out.push(r?);
            }
        }
        (Some(start), None) | (None, Some(start)) => {
            let rows = stmt.query_map(params![start], |row| EggRecord::try_from(row))?;
            for r in rows {
                out.push(r?);
            }
        }
        (None, None) => {
            let rows = stmt.query_map([], |row| EggRecord::try_from(row))?;
            for r in rows {
                out.push(r?);
            }
        }
    }

    Ok(out)
}

// mapping helper removed; use EggRecord::try_from

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    #[tokio::test]
    async fn test_add_and_get_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        let date = chrono::Local::now().date_naive();
        let record = EggRecord::new(date, 12);
        let id = add_egg_record(&conn, &record).await.unwrap();
        assert!(!id.is_nil());

        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        let loaded = get_egg_record(&conn, &date_str).unwrap();
        assert_eq!(loaded.total_eggs, 12);
    }

    #[tokio::test]
    async fn test_update_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        let date = chrono::Local::now().date_naive();
        let mut record = EggRecord::new(date, 10);
        add_egg_record(&conn, &record).await.unwrap();

        record.total_eggs = 15;
        record.notes = Some("Aktualisiert".to_string());
        update_egg_record(&conn, &record).await.unwrap();

        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        let loaded = get_egg_record(&conn, &date_str).unwrap();
        assert_eq!(loaded.total_eggs, 15);
        assert_eq!(loaded.notes, Some("Aktualisiert".to_string()));
    }

    #[tokio::test]
    async fn test_delete_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        let date = chrono::Local::now().date_naive();
        let record = EggRecord::new(date, 8);
        add_egg_record(&conn, &record).await.unwrap();

        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        delete_egg_record(&conn, &date_str).await.unwrap();

        let result = get_egg_record(&conn, &date_str);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_egg_records() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        // Mehrere Einträge hinzufügen mit unterschiedlichen Daten
        for i in 0..5 {
            let date = chrono::NaiveDate::from_ymd_opt(2025, 11, 5 + i).unwrap();
            let record = EggRecord::new(date, (i + 1) as i32 * 2);
            add_egg_record(&conn, &record).await.unwrap();
        }

        let records = list_egg_records(&conn, None, None).unwrap();
        assert_eq!(records.len(), 5);
    }
}
