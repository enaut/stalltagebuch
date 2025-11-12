use crate::error::AppError;
use crate::models::{EventType, QuailEvent};
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

/// Creates a new event for a quail
pub fn create_event(
    conn: &Connection,
    quail_id: i64,
    event_type: EventType,
    event_date: NaiveDate,
    notes: Option<String>,
) -> Result<i64, AppError> {
    let mut event = QuailEvent::new(quail_id, event_type, event_date);
    event.notes = notes;

    event.validate()?;

    conn.execute(
        "INSERT INTO quail_events (uuid, quail_id, event_type, event_date, notes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            event.uuid,
            event.quail_id,
            event.event_type.as_str(),
            event.event_date.to_string(),
            event.notes,
        ],
    )?;

    let event_id = conn.last_insert_rowid();

    Ok(event_id)
}

/// Returns all events for a specific quail
pub fn get_events_for_wachtel(
    conn: &Connection,
    quail_id: i64,
) -> Result<Vec<QuailEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, quail_id, event_type, event_date, notes
         FROM quail_events
         WHERE quail_id = ?1
         ORDER BY event_date DESC, id DESC",
    )?;

    let events = stmt
        .query_map(params![quail_id], |row| QuailEvent::try_from(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(events)
}

/// Returns the latest event for a quail
#[allow(dead_code)]
pub fn get_latest_event(conn: &Connection, quail_id: i64) -> Result<Option<QuailEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, quail_id, event_type, event_date, notes
         FROM quail_events
         WHERE quail_id = ?1
         ORDER BY event_date DESC, id DESC
         LIMIT 1",
    )?;

    let event = stmt
        .query_row(params![quail_id], |row| QuailEvent::try_from(row))
        .optional()?;

    Ok(event)
}

/// Returns the birth date of a quail (from the "born" event)
#[allow(dead_code)]
pub fn get_birth_date(conn: &Connection, quail_id: i64) -> Result<Option<NaiveDate>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT event_date
         FROM quail_events
         WHERE quail_id = ?1 AND event_type = 'born'
         LIMIT 1",
    )?;

    let date_str: Option<String> = stmt
        .query_row(params![quail_id], |row| row.get(0))
        .optional()?;

    if let Some(date_str) = &date_str {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| AppError::Database(rusqlite::Error::InvalidQuery))?;
        Ok(Some(date))
    } else {
        Ok(None)
    }
}
/// Updates an existing event
#[allow(dead_code)]
pub fn update_event(
    conn: &Connection,
    event_id: i64,
    notes: Option<String>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE quail_events 
         SET notes = ?1
         WHERE id = ?2",
        params![notes, event_id],
    )?;

    Ok(())
}

/// Deletes an event
pub fn delete_event(conn: &Connection, event_id: i64) -> Result<(), AppError> {
    let mut stmt = conn.prepare("SELECT quail_id, event_type FROM quail_events WHERE id = ?1")?;
    let (_quail_id, _event_type_str): (i64, String) =
        stmt.query_row(params![event_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
    conn.execute("DELETE FROM quail_events WHERE id = ?1", params![event_id])?;
    Ok(())
}

/// Gets a single event by ID
pub fn get_event_by_id(conn: &Connection, event_id: i64) -> Result<Option<QuailEvent>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, quail_id, event_type, event_date, notes FROM quail_events WHERE id = ?1",
    )?;
    let evt = stmt
        .query_row(params![event_id], |row| QuailEvent::try_from(row))
        .optional()?;
    Ok(evt)
}

/// Full update of an event (type, date, notes)
pub fn update_event_full(
    conn: &Connection,
    event_id: i64,
    event_type: EventType,
    event_date: NaiveDate,
    notes: Option<String>,
) -> Result<(), AppError> {
    let existing = get_event_by_id(conn, event_id)?
        .ok_or_else(|| AppError::NotFound("Event not found".to_string()))?;
    let candidate = QuailEvent {
        id: Some(event_id),
        uuid: existing.uuid.clone(),
        quail_id: existing.quail_id,
        event_type: event_type.clone(),
        event_date,
        notes: notes.clone(),
    };
    candidate.validate()?;
    conn.execute(
        "UPDATE quail_events SET event_type = ?1, event_date = ?2, notes = ?3 WHERE id = ?4",
        params![event_type.as_str(), event_date.to_string(), notes, event_id],
    )?;
    Ok(())
}
