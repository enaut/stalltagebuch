use rusqlite::{Connection, params};
use crate::error::AppError;
use crate::models::EggRecord;
use chrono::NaiveDate;

/// Erstellt einen neuen Eier-Eintrag
pub fn add_egg_record(conn: &Connection, record: &EggRecord) -> Result<i64, AppError> {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();
    
    conn.execute(
        "INSERT INTO egg_records (record_date, total_eggs, notes) VALUES (?1, ?2, ?3)",
        params![date_str, record.total_eggs, record.notes],
    )?;
    
    Ok(conn.last_insert_rowid())
}

/// Lädt einen Eier-Eintrag für ein bestimmtes Datum
pub fn get_egg_record(conn: &Connection, date: &str) -> Result<EggRecord, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, record_date, total_eggs, notes, created_at, updated_at 
         FROM egg_records 
         WHERE record_date = ?1"
    )?;
    
    let record = stmt.query_row(params![date], |row| {
        let date_str: String = row.get(1)?;
        let record_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(e)
            ))?;
        
        Ok(EggRecord {
            id: Some(row.get(0)?),
            record_date,
            total_eggs: row.get(2)?,
            notes: row.get(3)?,
        })
    })?;
    
    Ok(record)
}

/// Aktualisiert einen existierenden Eier-Eintrag
pub fn update_egg_record(conn: &Connection, record: &EggRecord) -> Result<(), AppError> {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();
    
    let rows_affected = conn.execute(
        "UPDATE egg_records 
         SET total_eggs = ?1, notes = ?2, updated_at = CURRENT_TIMESTAMP 
         WHERE record_date = ?3",
        params![record.total_eggs, record.notes, date_str],
    )?;
    
    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Eintrag für {} nicht gefunden", date_str)));
    }
    
    Ok(())
}

/// Löscht einen Eier-Eintrag
pub fn delete_egg_record(conn: &Connection, date: &str) -> Result<(), AppError> {
    let rows_affected = conn.execute(
        "DELETE FROM egg_records WHERE record_date = ?1",
        params![date],
    )?;
    
    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Eintrag für {} nicht gefunden", date)));
    }
    
    Ok(())
}

/// Lädt alle Eier-Einträge für einen Zeitraum (sortiert nach Datum absteigend)
pub fn list_egg_records(
    conn: &Connection,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<Vec<EggRecord>, AppError> {
    let query = match (start_date, end_date) {
        (Some(_), Some(_)) => {
            "SELECT id, record_date, total_eggs, notes, created_at, updated_at 
             FROM egg_records 
             WHERE record_date BETWEEN ?1 AND ?2 
             ORDER BY record_date DESC"
        }
        (Some(_), None) => {
            "SELECT id, record_date, total_eggs, notes, created_at, updated_at 
             FROM egg_records 
             WHERE record_date >= ?1 
             ORDER BY record_date DESC"
        }
        (None, Some(_)) => {
            "SELECT id, record_date, total_eggs, notes, created_at, updated_at 
             FROM egg_records 
             WHERE record_date <= ?1 
             ORDER BY record_date DESC"
        }
        (None, None) => {
            "SELECT id, record_date, total_eggs, notes, created_at, updated_at 
             FROM egg_records 
             ORDER BY record_date DESC"
        }
    };
    
    let mut stmt = conn.prepare(query)?;
    
    let records = match (start_date, end_date) {
        (Some(start), Some(end)) => {
            stmt.query_map(params![start, end], map_egg_record)?
        }
        (Some(start), None) | (None, Some(start)) => {
            stmt.query_map(params![start], map_egg_record)?
        }
        (None, None) => {
            stmt.query_map([], map_egg_record)?
        }
    };
    
    let mut result = Vec::new();
    for record in records {
        result.push(record?);
    }
    
    Ok(result)
}

/// Zählt die Anzahl der Eier-Einträge
#[allow(dead_code)]
pub fn count_egg_records(conn: &Connection) -> Result<i32, AppError> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM egg_records",
        [],
        |row| row.get(0),
    )?;
    
    Ok(count)
}

/// Helper-Funktion zum Mappen eines Row zu EggRecord
fn map_egg_record(row: &rusqlite::Row) -> rusqlite::Result<EggRecord> {
    let date_str: String = row.get(1)?;
    let record_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
            1,
            rusqlite::types::Type::Text,
            Box::new(e)
        ))?;
    
    Ok(EggRecord {
        id: Some(row.get(0)?),
        record_date,
        total_eggs: row.get(2)?,
        notes: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use chrono::Local;
    
    #[test]
    fn test_add_and_get_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();
        
        let record = EggRecord::today(12);
        let id = add_egg_record(&conn, &record).unwrap();
        assert!(id > 0);
        
        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        let loaded = get_egg_record(&conn, &date_str).unwrap();
        assert_eq!(loaded.total_eggs, 12);
    }
    
    #[test]
    fn test_update_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();
        
        let mut record = EggRecord::today(10);
        add_egg_record(&conn, &record).unwrap();
        
        record.total_eggs = 15;
        record.notes = Some("Aktualisiert".to_string());
        update_egg_record(&conn, &record).unwrap();
        
        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        let loaded = get_egg_record(&conn, &date_str).unwrap();
        assert_eq!(loaded.total_eggs, 15);
        assert_eq!(loaded.notes, Some("Aktualisiert".to_string()));
    }
    
    #[test]
    fn test_delete_egg_record() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();
        
        let record = EggRecord::today(8);
        add_egg_record(&conn, &record).unwrap();
        
        let date_str = record.record_date.format("%Y-%m-%d").to_string();
        delete_egg_record(&conn, &date_str).unwrap();
        
        let result = get_egg_record(&conn, &date_str);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_list_egg_records() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();
        
        // Mehrere Einträge hinzufügen
        for i in 1..=5 {
            let record = EggRecord::new(Local::now().naive_local().date(), i * 2);
            add_egg_record(&conn, &record).unwrap();
        }
        
        let records = list_egg_records(&conn, None, None).unwrap();
        assert_eq!(records.len(), 5);
    }
}
