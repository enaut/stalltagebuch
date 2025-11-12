use crate::error::AppError;
use crate::models::Quail;
use rusqlite::Connection;

/// Creates a new quail profile in the database
pub fn create_profile(conn: &Connection, quail: &Quail) -> Result<i64, AppError> {
    // Validation
    quail.validate()?;

    conn.execute(
        "INSERT INTO quails (uuid, name, gender, ring_color)
         VALUES (?1, ?2, ?3, ?4)",
        (
            &quail.uuid,
            &quail.name,
            quail.gender.as_str(),
            &quail.ring_color.as_ref().map(|c| c.as_str().to_string()),
        ),
    )?;

    Ok(conn.last_insert_rowid())
}

/// Loads a quail profile by ID
pub fn get_profile(conn: &Connection, id: i64) -> Result<Quail, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, name, gender, ring_color
         FROM quails WHERE id = ?1",
    )?;

    let quail = stmt
        .query_row([id], |row| Quail::try_from(row))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Quail profile".to_string()),
            _ => AppError::Database(e),
        })?;

    Ok(quail)
}

/// Updates an existing quail profile
pub fn update_profile(conn: &Connection, quail: &Quail) -> Result<(), AppError> {
    // Validation
    quail.validate()?;

    let id = quail
        .id
        .ok_or_else(|| AppError::Validation("Quail must have an ID".to_string()))?;

    let rows_affected = conn.execute(
        "UPDATE quails 
         SET name = ?1, gender = ?2, ring_color = ?3
         WHERE id = ?4",
        (
            &quail.name,
            quail.gender.as_str(),
            &quail.ring_color.as_ref().map(|c| c.as_str().to_string()),
            id,
        ),
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Quail profile".to_string()));
    }

    Ok(())
}

/// Deletes a quail profile (CASCADE also deletes individual egg entries)
pub fn delete_profile(conn: &Connection, id: i64) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM quails WHERE id = ?1", [id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Quail profile".to_string()));
    }

    Ok(())
}

/// Lists all quail profiles, optionally filtered by name (default: only living ones)
#[allow(dead_code)]
pub fn list_profiles(conn: &Connection, name_filter: Option<&str>) -> Result<Vec<Quail>, AppError> {
    list_profiles_with_status(conn, name_filter, true)
}

/// Lists quail profiles with optional status filter
/// Note: Status filtering is no longer supported since status is now managed via events
pub fn list_profiles_with_status(
    conn: &Connection,
    name_filter: Option<&str>,
    _only_alive: bool,
) -> Result<Vec<Quail>, AppError> {
    let (query, params): (&str, Vec<&str>) = match name_filter {
        Some(filter) if !filter.trim().is_empty() => (
            "SELECT id, uuid, name, gender, ring_color
             FROM quails 
             WHERE name LIKE '%' || ?1 || '%'
             ORDER BY name",
            vec![filter],
        ),
        _ => (
            "SELECT id, uuid, name, gender, ring_color
             FROM quails 
             ORDER BY name",
            vec![],
        ),
    };

    let mut stmt = conn.prepare(query)?;

    let quails = stmt
        .query_map(rusqlite::params_from_iter(params), |row| {
            Quail::try_from(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(quails)
}

/// Counts the total number of profiles
pub fn count_profiles(conn: &Connection) -> Result<i32, AppError> {
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM quails", [], |row| row.get(0))?;

    Ok(count)
}

/// Helper function: Gets the current status of a profile based on the latest event
pub fn get_profile_current_status(
    conn: &Connection,
    quail_id: i64,
) -> Result<Option<crate::models::EventType>, AppError> {
    use rusqlite::OptionalExtension;

    let event_type_str: Option<String> = conn
        .query_row(
            "SELECT event_type FROM quail_events 
             WHERE quail_id = ?1 
             ORDER BY event_date DESC, id DESC 
             LIMIT 1",
            [quail_id],
            |row| row.get(0),
        )
        .optional()?;

    Ok(event_type_str.map(|s| crate::models::EventType::from_str(&s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::database::schema::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_create_and_get_profile() {
        let conn = setup_test_db();
        let mut quail = Quail::new("Testwachtel".to_string());
        quail.gender = crate::models::Gender::Female;

        let id = create_profile(&conn, &quail).unwrap();
        assert!(id > 0);

        let loaded = get_profile(&conn, id).unwrap();
        assert_eq!(loaded.name, "Testwachtel");
        assert_eq!(loaded.gender, crate::models::Gender::Female);
    }

    #[test]
    fn test_update_profile() {
        let conn = setup_test_db();
        let mut quail = Quail::new("Original".to_string());

        let id = create_profile(&conn, &quail).unwrap();
        quail.id = Some(id);
        quail.name = "Updated".to_string();

        update_profile(&conn, &quail).unwrap();

        let loaded = get_profile(&conn, id).unwrap();
        assert_eq!(loaded.name, "Updated");
    }

    #[test]
    fn test_delete_profile() {
        let conn = setup_test_db();
        let quail = Quail::new("ToDelete".to_string());

        let id = create_profile(&conn, &quail).unwrap();
        delete_profile(&conn, id).unwrap();

        let result = get_profile(&conn, id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_profiles() {
        let conn = setup_test_db();

        create_profile(&conn, &Quail::new("Alice".to_string())).unwrap();
        create_profile(&conn, &Quail::new("Bob".to_string())).unwrap();
        create_profile(&conn, &Quail::new("Charlie".to_string())).unwrap();

        let all = list_profiles(&conn, None).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = list_profiles(&conn, Some("li")).unwrap();
        assert_eq!(filtered.len(), 2); // Alice, Charlie
    }
}
