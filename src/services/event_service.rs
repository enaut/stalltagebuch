use crate::error::AppError;
use crate::models::{EventType, WachtelEvent};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

/// Erstellt ein neues Ereignis für eine Wachtel
pub fn create_event(
    conn: &Connection,
    wachtel_id: i64,
    event_type: EventType,
    event_date: NaiveDate,
    notes: Option<String>,
) -> Result<i64, AppError> {
    let event = WachtelEvent {
        id: None,
        wachtel_id,
        event_type: event_type.clone(),
        event_date,
        notes,
    };

    event.validate()?;

    conn.execute(
        "INSERT INTO wachtel_events (wachtel_id, event_type, event_date, notes)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            event.wachtel_id,
            event.event_type.as_str(),
            event.event_date.to_string(),
            event.notes,
        ],
    )?;

    let event_id = conn.last_insert_rowid();

    Ok(event_id)
}

/// Gibt alle Ereignisse für eine bestimmte Wachtel zurück
pub fn get_events_for_wachtel(
    conn: &Connection,
    wachtel_id: i64,
) -> Result<Vec<WachtelEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_type, event_date, notes
         FROM wachtel_events
         WHERE wachtel_id = ?1
         ORDER BY event_date DESC, id DESC",
    )?;

    let events = stmt
        .query_map(params![wachtel_id], |row| {
            Ok(WachtelEvent {
                id: Some(row.get(0)?),
                wachtel_id: row.get(1)?,
                event_type: EventType::from_str(&row.get::<_, String>(2)?),
                event_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                notes: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(events)
}

/// Gibt das letzte Ereignis für eine Wachtel zurück
pub fn get_latest_event(
    conn: &Connection,
    wachtel_id: i64,
) -> Result<Option<WachtelEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_type, event_date, notes
         FROM wachtel_events
         WHERE wachtel_id = ?1
         ORDER BY event_date DESC, id DESC
         LIMIT 1",
    )?;

    let event = stmt
        .query_row(params![wachtel_id], |row| {
            Ok(WachtelEvent {
                id: Some(row.get(0)?),
                wachtel_id: row.get(1)?,
                event_type: EventType::from_str(&row.get::<_, String>(2)?),
                event_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                notes: row.get(4)?,
            })
        })
        .optional()?;

    Ok(event)
}

/// Gibt das Geburtsdatum einer Wachtel zurück (aus dem "geboren" Event)
pub fn get_birth_date(conn: &Connection, wachtel_id: i64) -> Result<Option<NaiveDate>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT event_date
         FROM wachtel_events
         WHERE wachtel_id = ?1 AND event_type = 'geboren'
         LIMIT 1",
    )?;

    let date_str: Option<String> = stmt
        .query_row(params![wachtel_id], |row| row.get(0))
        .optional()?;

    if let Some(date_str) = &date_str {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Database(rusqlite::Error::InvalidQuery))?;
        Ok(Some(date))
    } else {
        Ok(None)
    }
}

/// Aktualisiert ein bestehendes Ereignis
pub fn update_event(
    conn: &Connection,
    event_id: i64,
    notes: Option<String>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE wachtel_events 
         SET notes = ?1
         WHERE id = ?2",
        params![notes, event_id],
    )?;

    Ok(())
}

/// Löscht ein Ereignis
pub fn delete_event(conn: &Connection, event_id: i64) -> Result<(), AppError> {
    let mut stmt =
        conn.prepare("SELECT wachtel_id, event_type FROM wachtel_events WHERE id = ?1")?;
    let (_wachtel_id, _event_type_str): (i64, String) =
        stmt.query_row(params![event_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    conn.execute(
        "DELETE FROM wachtel_events WHERE id = ?1",
        params![event_id],
    )?;
    Ok(())
}

/// Hole ein einzelnes Ereignis per ID
pub fn get_event_by_id(conn: &Connection, event_id: i64) -> Result<Option<WachtelEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, wachtel_id, event_type, event_date, notes FROM wachtel_events WHERE id = ?1",
    )?;
    let evt = stmt
        .query_row(params![event_id], |row| {
            Ok(WachtelEvent {
                id: Some(row.get(0)?),
                wachtel_id: row.get(1)?,
                event_type: EventType::from_str(&row.get::<_, String>(2)?),
                event_date: NaiveDate::parse_from_str(&row.get::<_, String>(3)?, "%Y-%m-%d")
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?,
                notes: row.get(4)?,
            })
        })
        .optional()?;
    Ok(evt)
}

/// Vollständiges Update eines Ereignisses (Typ, Datum, Notizen)
pub fn update_event_full(
    conn: &Connection,
    event_id: i64,
    event_type: EventType,
    event_date: NaiveDate,
    notes: Option<String>,
) -> Result<(), AppError> {
    let existing = get_event_by_id(conn, event_id)?
        .ok_or_else(|| AppError::NotFound("Ereignis nicht gefunden".to_string()))?;
    let candidate = WachtelEvent {
        id: Some(event_id),
        wachtel_id: existing.wachtel_id,
        event_type: event_type.clone(),
        event_date,
        notes: notes.clone(),
    };
    candidate.validate()?;
    conn.execute(
        "UPDATE wachtel_events SET event_type = ?1, event_date = ?2, notes = ?3 WHERE id = ?4",
        params![event_type.as_str(), event_date.to_string(), notes, event_id],
    )?;
    Ok(())
}
