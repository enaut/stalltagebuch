use crate::error::AppError;
use crate::models::Quail;
use rusqlite::Connection;
use uuid::Uuid;

/// Creates a new quail profile in the database
pub async fn create_profile(conn: &Connection, quail: &Quail) -> Result<Uuid, AppError> {
    // Validation
    quail.validate()?;

    conn.execute(
        "INSERT INTO quails (uuid, name, gender, ring_color, profile_photo)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            quail.uuid.to_string(),
            &quail.name,
            quail.gender.as_str(),
            &quail.ring_color.as_ref().map(|c| c.as_str().to_string()),
            quail.profile_photo.as_ref().map(|u| u.to_string()),
        ),
    )?;

    // Capture CRDT operation
    crate::services::operation_capture::capture_quail_create(
        conn,
        &quail.uuid.to_string(),
        &quail.name,
        quail.gender.as_str(),
        quail.ring_color.as_ref().map(|c| c.as_str()),
        quail
            .profile_photo
            .as_ref()
            .map(|u| u.to_string())
            .as_deref(),
    )
    .await?;

    Ok(quail.uuid)
}

/// Loads a quail profile by UUID
pub fn get_profile(conn: &Connection, uuid: &Uuid) -> Result<Quail, AppError> {
    let mut stmt = conn.prepare(
        "SELECT uuid, name, gender, ring_color, profile_photo
         FROM quails WHERE uuid = ?1",
    )?;

    let quail = stmt
        .query_row([uuid.to_string()], |row| Quail::try_from(row))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("Quail profile".to_string()),
            _ => AppError::Database(e),
        })?;

    Ok(quail)
}

/// Updates an existing quail profile
pub async fn update_profile(conn: &Connection, quail: &Quail) -> Result<(), AppError> {
    // Validation
    quail.validate()?;

    let rows_affected = conn.execute(
        "UPDATE quails 
         SET name = ?1, gender = ?2, ring_color = ?3, profile_photo = ?4
         WHERE uuid = ?5",
        (
            &quail.name,
            quail.gender.as_str(),
            &quail.ring_color.as_ref().map(|c| c.as_str().to_string()),
            quail.profile_photo.as_ref().map(|u| u.to_string()),
            quail.uuid.to_string(),
        ),
    )?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Quail profile".to_string()));
    }

    // Capture CRDT operations for each field
    let quail_id = quail.uuid.to_string();
    crate::services::operation_capture::capture_quail_update(
        conn,
        &quail_id,
        "name",
        serde_json::Value::String(quail.name.clone()),
    )
    .await?;
    crate::services::operation_capture::capture_quail_update(
        conn,
        &quail_id,
        "gender",
        serde_json::Value::String(quail.gender.as_str().to_string()),
    )
    .await?;
    if let Some(color) = &quail.ring_color {
        crate::services::operation_capture::capture_quail_update(
            conn,
            &quail_id,
            "ring_color",
            serde_json::Value::String(color.as_str().to_string()),
        )
        .await?;
    }
    if let Some(photo) = &quail.profile_photo {
        crate::services::operation_capture::capture_quail_update(
            conn,
            &quail_id,
            "profile_photo",
            serde_json::Value::String(photo.to_string()),
        )
        .await?;
    }

    Ok(())
}

/// Deletes a quail profile (CASCADE also deletes individual egg entries)
pub async fn delete_profile(conn: &Connection, uuid: &Uuid) -> Result<(), AppError> {
    let rows_affected = conn.execute("DELETE FROM quails WHERE uuid = ?1", [uuid.to_string()])?;

    if rows_affected == 0 {
        return Err(AppError::NotFound("Quail profile".to_string()));
    }

    // Capture CRDT deletion
    crate::services::operation_capture::capture_quail_delete(conn, &uuid.to_string()).await?;

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
            "SELECT uuid, name, gender, ring_color, profile_photo
             FROM quails 
             WHERE name LIKE '%' || ?1 || '%'
             ORDER BY name",
            vec![filter],
        ),
        _ => (
            "SELECT uuid, name, gender, ring_color, profile_photo
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
    quail_uuid: &Uuid,
) -> Result<Option<crate::models::EventType>, AppError> {
    use rusqlite::OptionalExtension;

    let event_type_str: Option<String> = conn
        .query_row(
            "SELECT event_type FROM quail_events 
             WHERE quail_id = ?1 
             ORDER BY event_date DESC 
             LIMIT 1",
            [quail_uuid.to_string()],
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

    #[tokio::test]
    async fn test_create_and_get_profile() {
        let conn = setup_test_db();
        let mut quail = Quail::new("Testwachtel".to_string());
        quail.gender = crate::models::Gender::Female;

        let uuid = create_profile(&conn, &quail).await.unwrap();
        assert_eq!(uuid, quail.uuid);

        let loaded = get_profile(&conn, &uuid).unwrap();
        assert_eq!(loaded.name, "Testwachtel");
        assert_eq!(loaded.gender, crate::models::Gender::Female);
    }

    #[tokio::test]
    async fn test_update_profile() {
        let conn = setup_test_db();
        let mut quail = Quail::new("Original".to_string());

        let id = create_profile(&conn, &quail).await.unwrap();
        quail.uuid = id;
        quail.name = "Updated".to_string();

        update_profile(&conn, &quail).await.unwrap();

        let loaded = get_profile(&conn, &id).unwrap();
        assert_eq!(loaded.name, "Updated");
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let conn = setup_test_db();
        let quail = Quail::new("ToDelete".to_string());

        let uuid = create_profile(&conn, &quail).await.unwrap();
        delete_profile(&conn, &uuid).await.unwrap();

        let result = get_profile(&conn, &uuid);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let conn = setup_test_db();

        create_profile(&conn, &Quail::new("Alice".to_string()))
            .await
            .unwrap();
        create_profile(&conn, &Quail::new("Bob".to_string()))
            .await
            .unwrap();
        create_profile(&conn, &Quail::new("Charlie".to_string()))
            .await
            .unwrap();

        let all = list_profiles(&conn, None).unwrap();
        assert_eq!(all.len(), 3);

        let filtered = list_profiles(&conn, Some("li")).unwrap();
        assert_eq!(filtered.len(), 2); // Alice, Charlie
    }
}
