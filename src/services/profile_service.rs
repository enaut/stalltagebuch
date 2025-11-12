use crate::error::AppError;
use crate::models::Wachtel;
use rusqlite::Connection;

/// Erstellt ein neues Wachtel-Profil in der Datenbank
pub fn create_profile(conn: &Connection, wachtel: &Wachtel) -> Result<i64, AppError> {
    // Validierung
    wachtel.validate()?;

    conn.execute(
        "INSERT INTO wachtels (uuid, name, gender, ring_color)
         VALUES (?1, ?2, ?3, ?4)",
        (
            &wachtel.uuid,
            &wachtel.name,
            wachtel.gender.as_str(),
            &wachtel.ring_color.as_ref().map(|c| c.as_str().to_string()),
        ),
    )?;

    Ok(conn.last_insert_rowid())
}

/// Lädt ein Wachtel-Profil anhand der ID
pub fn get_profile(conn: &Connection, id: i64) -> Result<Wachtel, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, uuid, name, gender, ring_color
         FROM wachtels WHERE id = ?1",
    )?;

    let wachtel = stmt
        .query_row([id], |row| Wachtel::try_from(row))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                AppError::NotFound("Wachtel-Profil".to_string())
            }
            _ => AppError::Database(e),
        })?;

    Ok(wachtel)
}

/// Aktualisiert ein bestehendes Wachtel-Profil
pub fn update_profile(conn: &Connection, wachtel: &Wachtel) -> Result<(), AppError> {
    // Validierung
    wachtel.validate()?;

    let id = wachtel
        .id
        .ok_or_else(|| AppError::Validation("Wachtel muss eine ID haben".to_string()))?;

    let rows_affected = conn.execute(
        "UPDATE wachtels 
         SET name = ?1, gender = ?2, ring_color = ?3
         WHERE id = ?4",
        (
            &wachtel.name,
            wachtel.gender.as_str(),
            &wachtel.ring_color.as_ref().map(|c| c.as_str().to_string()),
            id,
        ),
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Wachtel-Profil".to_string()));
    }

    Ok(())
}

/// Löscht ein Wachtel-Profil (CASCADE löscht auch individuelle Eier-Einträge)
pub fn delete_profile(conn: &Connection, id: i64) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM wachtels WHERE id = ?1", [id])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Wachtel-Profil".to_string()));
    }

    Ok(())
}

/// Listet alle Wachtel-Profile auf, optional gefiltert nach Name (Standard: nur lebende)
#[allow(dead_code)]
pub fn list_profiles(
    conn: &Connection,
    name_filter: Option<&str>,
) -> Result<Vec<Wachtel>, AppError> {
    list_profiles_with_status(conn, name_filter, true)
}

/// Listet Wachtel-Profile mit optionalem Status-Filter
/// Note: Status filtering is no longer supported since status is now managed via events
pub fn list_profiles_with_status(
    conn: &Connection,
    name_filter: Option<&str>,
    _only_alive: bool,
) -> Result<Vec<Wachtel>, AppError> {
    let (query, params): (&str, Vec<&str>) = match name_filter {
        Some(filter) if !filter.trim().is_empty() => (
            "SELECT id, uuid, name, gender, ring_color
             FROM wachtels 
             WHERE name LIKE '%' || ?1 || '%'
             ORDER BY name",
            vec![filter],
        ),
        _ => (
            "SELECT id, uuid, name, gender, ring_color
             FROM wachtels 
             ORDER BY name",
            vec![],
        ),
    };

    let mut stmt = conn.prepare(query)?;

    let wachtels = stmt
        .query_map(rusqlite::params_from_iter(params), |row| {
            Wachtel::try_from(row)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(wachtels)
}

/// Zählt die Gesamtanzahl der Profile
pub fn count_profiles(conn: &Connection) -> Result<i32, AppError> {
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM wachtels", [], |row| row.get(0))?;

    Ok(count)
}

/// Hilfsfunktion: Holt den aktuellen Status eines Profils basierend auf dem neuesten Event
pub fn get_profile_current_status(
    conn: &Connection,
    wachtel_id: i64,
) -> Result<Option<crate::models::EventType>, AppError> {
    use rusqlite::OptionalExtension;

    let event_type_str: Option<String> = conn
        .query_row(
            "SELECT event_type FROM wachtel_events 
             WHERE wachtel_id = ?1 
             ORDER BY event_date DESC, id DESC 
             LIMIT 1",
            [wachtel_id],
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
        let mut wachtel = Wachtel::new("Testwachtel".to_string());
        wachtel.gender = crate::models::Gender::Female;

        let id = create_profile(&conn, &wachtel).unwrap();
        assert!(id > 0);

        let loaded = get_profile(&conn, id).unwrap();
        assert_eq!(loaded.name, "Testwachtel");
        assert_eq!(loaded.gender, crate::models::Gender::Female);
    }

    #[test]
    fn test_update_profile() {
        let conn = setup_test_db();
        let mut wachtel = Wachtel::new("Original".to_string());

        let id = create_profile(&conn, &wachtel).unwrap();
        wachtel.id = Some(id);
        wachtel.name = "Updated".to_string();

        update_profile(&conn, &wachtel).unwrap();

        let loaded = get_profile(&conn, id).unwrap();
        assert_eq!(loaded.name, "Updated");
    }

    #[test]
    fn test_delete_profile() {
        let conn = setup_test_db();
        let wachtel = Wachtel::new("ToDelete".to_string());

        let id = create_profile(&conn, &wachtel).unwrap();
        delete_profile(&conn, id).unwrap();

        let result = get_profile(&conn, id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_profiles() {
        let conn = setup_test_db();

        create_profile(&conn, &Wachtel::new("Alice".to_string())).unwrap();
        create_profile(&conn, &Wachtel::new("Bob".to_string())).unwrap();
        create_profile(&conn, &Wachtel::new("Charlie".to_string())).unwrap();

        let all = list_profiles(&conn, None).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = list_profiles(&conn, Some("li")).unwrap();
        assert_eq!(filtered.len(), 2); // Alice, Charlie
    }
}
