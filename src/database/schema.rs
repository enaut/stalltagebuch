use rusqlite::{Connection, Result};

/// Initialisiert das komplette Datenbankschema für die Stalltagebuch-App
pub fn init_schema(conn: &Connection) -> Result<()> {
    // Foreign Keys aktivieren
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Schema-Version für Migrations-Tracking
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Prüfen ob Schema bereits existiert
    let current_version: i32 = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        create_initial_schema(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    // Migration zu Version 2: ring_color Spalte hinzufügen
    if current_version < 2 {
        apply_migration_v2(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (2)", [])?;
    }

    // Migration zu Version 3: status Spalte hinzufügen
    if current_version < 3 {
        apply_migration_v3(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (3)", [])?;
    }

    // Migration zu Version 4: wachtel_events Tabelle hinzufügen
    if current_version < 4 {
        apply_migration_v4(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (4)", [])?;
    }

    // Migration zu Version 5: Entferne überflüssige Felder aus wachtels
    if current_version < 5 {
        apply_migration_v5(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (5)", [])?;
    }

    // Migration zu Version 6: Fotos in eigene Tabelle auslagern
    if current_version < 6 {
        apply_migration_v6(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (6)", [])?;
    }

    // Migration zu Version 7: sync_settings Tabelle hinzufügen
    if current_version < 7 {
        apply_migration_v7(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (7)", [])?;
    }

    // Migration zu Version 8: UUID Spalte zu photos hinzufügen
    if current_version < 8 {
        apply_migration_v8(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (8)", [])?;
    }

    // Migration zu Version 9: sync_queue Tabelle hinzufügen
    if current_version < 9 {
        apply_migration_v9(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (9)", [])?;
    }

    // Migration zu Version 10: UUID Spalten zu wachtel_events hinzufügen
    if current_version < 10 {
        apply_migration_v10(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (10)", [])?;
    }

    // Migration zu Version 11: UUID Spalten zu egg_records hinzufügen
    if current_version < 11 {
        apply_migration_v11(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (11)", [])?;
    }

    Ok(())
}

/// Erstellt das initiale Schema (Version 1)
fn create_initial_schema(conn: &Connection) -> Result<()> {
    // Tabelle: wachtels (Wachtel-Profile)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS wachtels (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            gender TEXT CHECK(gender IN ('male', 'female', 'unknown')) NOT NULL DEFAULT 'unknown',
            birth_date TEXT,
            age_months INTEGER,
            ring_color TEXT,
            photo_path TEXT,
            thumbnail_path TEXT,
            notes TEXT,
            status TEXT CHECK(status IN ('am_leben', 'krank', 'gestorben', 'geschlachtet', 'markiert_zum_schlachten')) NOT NULL DEFAULT 'am_leben',
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Indizes für wachtels
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_name ON wachtels(name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_uuid ON wachtels(uuid)",
        [],
    )?;

    // Tabelle: egg_records (Tägliche Gesamt-Eierzahl)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS egg_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            record_date TEXT NOT NULL UNIQUE,
            total_eggs INTEGER NOT NULL CHECK(total_eggs >= 0),
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Index für egg_records
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_egg_records_date ON egg_records(record_date DESC)",
        [],
    )?;

    // Tabelle: individual_egg_records (Pro-Wachtel Eierzahl)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS individual_egg_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            wachtel_id INTEGER NOT NULL,
            record_date TEXT NOT NULL,
            eggs_laid INTEGER NOT NULL CHECK(eggs_laid >= 0),
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (wachtel_id) REFERENCES wachtels(id) ON DELETE CASCADE,
            UNIQUE(wachtel_id, record_date)
        )",
        [],
    )?;

    // Indizes für individual_egg_records
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_individual_egg_records_wachtel 
         ON individual_egg_records(wachtel_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_individual_egg_records_date 
         ON individual_egg_records(record_date DESC)",
        [],
    )?;

    Ok(())
}

/// Migration zu Version 2: ring_color Spalte hinzufügen
fn apply_migration_v2(conn: &Connection) -> Result<()> {
    // Prüfen ob die Spalte bereits existiert
    let column_exists: Result<i32> = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('wachtels') WHERE name='ring_color'",
        [],
        |row| row.get(0),
    );

    if column_exists.unwrap_or(0) == 0 {
        conn.execute("ALTER TABLE wachtels ADD COLUMN ring_color TEXT", [])?;
    }

    Ok(())
}

/// Migration zu Version 3: status Spalte hinzufügen
fn apply_migration_v3(conn: &Connection) -> Result<()> {
    // Prüfen ob die Spalte bereits existiert
    let column_exists: Result<i32> = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('wachtels') WHERE name='status'",
        [],
        |row| row.get(0),
    );

    if column_exists.unwrap_or(0) == 0 {
        conn.execute(
            "ALTER TABLE wachtels ADD COLUMN status TEXT NOT NULL DEFAULT 'am_leben'",
            [],
        )?;
    }

    Ok(())
}

/// Migration zu Version 4: wachtel_events Tabelle erstellen und Daten migrieren
fn apply_migration_v4(conn: &Connection) -> Result<()> {
    // Tabelle: wachtel_events (Ereignisse im Leben einer Wachtel)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS wachtel_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            wachtel_id INTEGER NOT NULL,
            event_type TEXT CHECK(event_type IN ('geboren', 'am_leben', 'krank', 'gesund', 'markiert_zum_schlachten', 'geschlachtet', 'gestorben')) NOT NULL,
            event_date TEXT NOT NULL,
            notes TEXT,
            photo_path TEXT,
            thumbnail_path TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (wachtel_id) REFERENCES wachtels(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Indizes für wachtel_events
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_wachtel_id ON wachtel_events(wachtel_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_date ON wachtel_events(event_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_type ON wachtel_events(event_type)",
        [],
    )?;

    // Trigger für updated_at Timestamps
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtel_events_timestamp 
         AFTER UPDATE ON wachtel_events
         BEGIN
            UPDATE wachtel_events SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Migriere bestehende Daten:
    // 1. Für jede Wachtel mit birth_date: erstelle "geboren" Event
    conn.execute(
        "INSERT INTO wachtel_events (wachtel_id, event_type, event_date)
         SELECT id, 'geboren', birth_date
         FROM wachtels
         WHERE birth_date IS NOT NULL",
        [],
    )?;

    // 2. Für jede Wachtel mit notes: erstelle ein generisches Event mit der Notiz
    // (verknüpft mit dem aktuellen Status)
    conn.execute(
        "INSERT INTO wachtel_events (wachtel_id, event_type, event_date, notes)
         SELECT id, status, COALESCE(birth_date, created_at), notes
         FROM wachtels
         WHERE notes IS NOT NULL AND notes != ''",
        [],
    )?;

    Ok(())
}

/// Migration zu Version 5: Entferne überflüssige Felder (birth_date, age_months, status, notes)
/// Diese werden jetzt durch wachtel_events verwaltet
fn apply_migration_v5(conn: &Connection) -> Result<()> {
    // SQLite unterstützt kein ALTER TABLE DROP COLUMN direkt vor Version 3.35.0
    // Wir müssen die Tabelle neu erstellen

    // 1. Erstelle neue Tabelle ohne die überflüssigen Spalten
    conn.execute(
        "CREATE TABLE wachtels_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            gender TEXT CHECK(gender IN ('male', 'female', 'unknown')) NOT NULL DEFAULT 'unknown',
            ring_color TEXT,
            photo_path TEXT,
            thumbnail_path TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 2. Kopiere Daten aus der alten Tabelle (nur die Spalten die wir behalten)
    conn.execute(
        "INSERT INTO wachtels_new (id, uuid, name, gender, ring_color, photo_path, thumbnail_path, created_at, updated_at)
         SELECT id, uuid, name, gender, ring_color, photo_path, thumbnail_path, created_at, updated_at
         FROM wachtels",
        [],
    )?;

    // 3. Lösche alte Tabelle
    conn.execute("DROP TABLE wachtels", [])?;

    // 4. Benenne neue Tabelle um
    conn.execute("ALTER TABLE wachtels_new RENAME TO wachtels", [])?;

    // 5. Erstelle Indizes neu
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_name ON wachtels(name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_uuid ON wachtels(uuid)",
        [],
    )?;

    // 6. Erstelle Trigger neu
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtels_timestamp 
         AFTER UPDATE ON wachtels
         BEGIN
            UPDATE wachtels SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    Ok(())
}

/// Migration zu Version 6: Auslagerung von Fotos in eigene Tabelle `photos`
/// - Erstellt `photos` mit optionaler Verknüpfung zu `wachtels` ODER `wachtel_events`
/// - Migriert vorhandene `photo_path`/`thumbnail_path`
/// - Entfernt `photo_path`/`thumbnail_path` aus `wachtels` und `wachtel_events`
fn apply_migration_v6(conn: &Connection) -> Result<()> {
    // 1) Neue Tabelle `photos`
    conn.execute(
        "CREATE TABLE IF NOT EXISTS photos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            wachtel_id INTEGER,
            event_id INTEGER,
            path TEXT NOT NULL,
            thumbnail_path TEXT,
            is_profile INTEGER NOT NULL DEFAULT 0 CHECK(is_profile IN (0,1)),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (wachtel_id) REFERENCES wachtels(id) ON DELETE CASCADE,
            FOREIGN KEY (event_id) REFERENCES wachtel_events(id) ON DELETE CASCADE,
            CHECK( (wachtel_id IS NOT NULL AND event_id IS NULL) OR (wachtel_id IS NULL AND event_id IS NOT NULL) )
        )",
        [],
    )?;

    // Indizes für photos
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photos_wachtel ON photos(wachtel_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photos_event ON photos(event_id)",
        [],
    )?;

    // Trigger für updated_at in photos
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_photos_timestamp 
         AFTER UPDATE ON photos
         BEGIN
            UPDATE photos SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // 2) Migriere vorhandene Bildpfade aus `wachtels` als Profilbilder
    conn.execute(
        "INSERT INTO photos (wachtel_id, path, thumbnail_path, is_profile)
         SELECT id, photo_path, thumbnail_path, 1
         FROM wachtels
         WHERE photo_path IS NOT NULL",
        [],
    )?;

    // 3) Migriere vorhandene Bildpfade aus `wachtel_events` als Event-Bilder
    conn.execute(
        "INSERT INTO photos (event_id, path, thumbnail_path, is_profile)
         SELECT id, photo_path, thumbnail_path, 0
         FROM wachtel_events
         WHERE photo_path IS NOT NULL",
        [],
    )?;

    // 4) Entferne photo-Spalten aus `wachtels` via Tabellenerneuerung
    conn.execute(
        "CREATE TABLE wachtels_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            gender TEXT CHECK(gender IN ('male', 'female', 'unknown')) NOT NULL DEFAULT 'unknown',
            ring_color TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "INSERT INTO wachtels_new (id, uuid, name, gender, ring_color, created_at, updated_at)
         SELECT id, uuid, name, gender, ring_color, created_at, updated_at
         FROM wachtels",
        [],
    )?;

    conn.execute("DROP TABLE wachtels", [])?;
    conn.execute("ALTER TABLE wachtels_new RENAME TO wachtels", [])?;

    // Indizes neu
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_name ON wachtels(name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtels_uuid ON wachtels(uuid)",
        [],
    )?;

    // Trigger neu
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtels_timestamp 
         AFTER UPDATE ON wachtels
         BEGIN
            UPDATE wachtels SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // 5) Entferne photo-Spalten aus `wachtel_events` via Tabellenerneuerung
    conn.execute(
        "CREATE TABLE wachtel_events_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            wachtel_id INTEGER NOT NULL,
            event_type TEXT CHECK(event_type IN ('geboren', 'am_leben', 'krank', 'gesund', 'markiert_zum_schlachten', 'geschlachtet', 'gestorben')) NOT NULL,
            event_date TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (wachtel_id) REFERENCES wachtels(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "INSERT INTO wachtel_events_new (id, wachtel_id, event_type, event_date, notes, created_at, updated_at)
         SELECT id, wachtel_id, event_type, event_date, notes, created_at, updated_at
         FROM wachtel_events",
        [],
    )?;

    conn.execute("DROP TABLE wachtel_events", [])?;
    conn.execute(
        "ALTER TABLE wachtel_events_new RENAME TO wachtel_events",
        [],
    )?;

    // Indizes und Trigger für `wachtel_events` erneut anlegen
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_wachtel_id ON wachtel_events(wachtel_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_date ON wachtel_events(event_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_type ON wachtel_events(event_type)",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtel_events_timestamp 
         AFTER UPDATE ON wachtel_events
         BEGIN
            UPDATE wachtel_events SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    Ok(())
}

/// Migration zu Version 7: sync_settings Tabelle erstellen
fn apply_migration_v7(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_settings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            server_url TEXT NOT NULL,
            username TEXT NOT NULL,
            app_password TEXT NOT NULL,
            remote_path TEXT NOT NULL DEFAULT '/Stalltagebuch',
            enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0,1)),
            last_sync TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Trigger für updated_at in sync_settings
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_sync_settings_timestamp 
         AFTER UPDATE ON sync_settings
         BEGIN
            UPDATE sync_settings SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    Ok(())
}

/// Migration v8: UUID Spalte zu photos hinzufügen
/// - Fügt uuid TEXT UNIQUE Spalte hinzu
/// - Generiert UUIDs für alle existierenden Fotos
/// - Benennt Dateien um von `photo_{id}.jpg` zu `{uuid}.jpg`
fn apply_migration_v8(conn: &Connection) -> Result<()> {
    // UUID Spalte hinzufügen
    conn.execute("ALTER TABLE photos ADD COLUMN uuid TEXT", [])?;

    // UUIDs für existierende Fotos generieren
    let mut stmt = conn.prepare("SELECT id, path FROM photos")?;
    let photos: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>>>()?;

    for (id, old_path) in photos {
        let uuid = uuid::Uuid::new_v4().to_string();

        // Pfad aktualisieren: photo_X.jpg -> {uuid}.jpg
        let new_path = if old_path.starts_with("photo_") {
            format!("{}.jpg", uuid)
        } else {
            old_path.clone()
        };

        conn.execute(
            "UPDATE photos SET uuid = ?1, path = ?2 WHERE id = ?3",
            rusqlite::params![uuid, new_path, id],
        )?;

        // Datei umbenennen wenn sie existiert
        #[cfg(target_os = "android")]
        {
            use std::path::Path;
            if let Some(app_dir) = crate::database::get_app_directory() {
                let old_file = app_dir.join(&old_path);
                let new_file = app_dir.join(&new_path);
                if old_file.exists() {
                    if let Ok(_) = std::fs::copy(&old_file, &new_file) {
                        let _ = std::fs::remove_file(&old_file);
                    }
                }

                // Auch Thumbnail umbenennen falls vorhanden
                if let Ok(Some(thumb_path)) = conn.query_row(
                    "SELECT thumbnail_path FROM photos WHERE id = ?1",
                    [id],
                    |row| row.get::<_, Option<String>>(0),
                ) {
                    let new_thumb = format!("thumb_{}.jpg", uuid);
                    let old_thumb_file = app_dir.join(&thumb_path);
                    let new_thumb_file = app_dir.join(&new_thumb);
                    if old_thumb_file.exists() {
                        if let Ok(_) = std::fs::copy(&old_thumb_file, &new_thumb_file) {
                            let _ = std::fs::remove_file(&old_thumb_file);
                        }
                    }
                    conn.execute(
                        "UPDATE photos SET thumbnail_path = ?1 WHERE id = ?2",
                        rusqlite::params![new_thumb, id],
                    )?;
                }
            }
        }
    }

    // UNIQUE Constraint hinzufügen
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_photos_uuid ON photos(uuid)",
        [],
    )?;

    Ok(())
}

/// Migration v9: sync_queue Tabelle hinzufügen
/// - Verwaltet ausstehende Photo-Uploads
/// - Tracking von Sync-Status und Retry-Logik
fn apply_migration_v9(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            photo_uuid TEXT NOT NULL UNIQUE,
            status TEXT NOT NULL CHECK(status IN ('pending', 'uploading', 'uploaded', 'failed')) DEFAULT 'pending',
            retry_count INTEGER NOT NULL DEFAULT 0,
            last_attempt TEXT,
            error_message TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (photo_uuid) REFERENCES photos(uuid) ON DELETE CASCADE
        )",
        [],
    )?;

    // Index für schnelle Statusabfragen
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sync_queue_photo ON sync_queue(photo_uuid)",
        [],
    )?;

    // Trigger für updated_at in sync_queue
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_sync_queue_timestamp 
         AFTER UPDATE ON sync_queue
         BEGIN
            UPDATE sync_queue SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    Ok(())
}

/// Trigger für updated_at Timestamps
pub fn create_update_triggers(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtels_timestamp 
         AFTER UPDATE ON wachtels
         BEGIN
            UPDATE wachtels SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_egg_records_timestamp 
         AFTER UPDATE ON egg_records
         BEGIN
            UPDATE egg_records SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_individual_egg_records_timestamp 
         AFTER UPDATE ON individual_egg_records
         BEGIN
            UPDATE individual_egg_records SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    Ok(())
}

/// Migration zu Version 10: UUID Spalten zu wachtel_events hinzufügen
fn apply_migration_v10(conn: &Connection) -> Result<()> {
    eprintln!("Running migration v10: Adding UUID to wachtel_events");

    // 1) Füge uuid Spalte zu wachtel_events hinzu
    conn.execute("ALTER TABLE wachtel_events ADD COLUMN uuid TEXT", [])?;

    // 2) Generiere UUIDs für alle existierenden Events
    let mut stmt = conn.prepare("SELECT id FROM wachtel_events")?;
    let event_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for event_id in event_ids {
        let uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "UPDATE wachtel_events SET uuid = ?1 WHERE id = ?2",
            [&uuid, &event_id.to_string()],
        )?;
    }

    // 3) Mache uuid NOT NULL
    conn.execute(
        "CREATE TABLE wachtel_events_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            wachtel_id INTEGER NOT NULL,
            event_type TEXT CHECK(event_type IN ('geboren', 'am_leben', 'krank', 'gesund', 'markiert_zum_schlachten', 'geschlachtet', 'gestorben')) NOT NULL,
            event_date TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (wachtel_id) REFERENCES wachtels(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "INSERT INTO wachtel_events_new (id, uuid, wachtel_id, event_type, event_date, notes, created_at, updated_at)
         SELECT id, uuid, wachtel_id, event_type, event_date, notes, created_at, updated_at
         FROM wachtel_events",
        [],
    )?;

    conn.execute("DROP TABLE wachtel_events", [])?;
    conn.execute(
        "ALTER TABLE wachtel_events_new RENAME TO wachtel_events",
        [],
    )?;

    // Indizes erneut anlegen
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_uuid ON wachtel_events(uuid)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_wachtel_id ON wachtel_events(wachtel_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_date ON wachtel_events(event_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wachtel_events_type ON wachtel_events(event_type)",
        [],
    )?;

    // Trigger erneut anlegen
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_wachtel_events_timestamp 
         AFTER UPDATE ON wachtel_events
         BEGIN
            UPDATE wachtel_events SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    eprintln!("Migration v10 completed");
    Ok(())
}

/// Migration v11: UUID Spalte zu egg_records hinzufügen
/// - Fügt uuid TEXT UNIQUE Spalte hinzu
/// - Generiert UUIDs für alle existierenden Einträge
fn apply_migration_v11(conn: &Connection) -> Result<()> {
    eprintln!("Applying migration v11: Adding UUID to egg_records");

    // 1) Füge uuid Spalte zu egg_records hinzu
    conn.execute("ALTER TABLE egg_records ADD COLUMN uuid TEXT", [])?;

    // 2) Generiere UUIDs für alle existierenden Einträge
    let mut stmt = conn.prepare("SELECT id FROM egg_records")?;
    let record_ids: Vec<i64> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for record_id in record_ids {
        let uuid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "UPDATE egg_records SET uuid = ?1 WHERE id = ?2",
            [&uuid, &record_id.to_string()],
        )?;
    }

    // 3) Erstelle neue Tabelle mit uuid NOT NULL UNIQUE
    conn.execute(
        "CREATE TABLE egg_records_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            record_date TEXT NOT NULL UNIQUE,
            total_eggs INTEGER NOT NULL CHECK(total_eggs >= 0),
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    conn.execute(
        "INSERT INTO egg_records_new (id, uuid, record_date, total_eggs, notes, created_at, updated_at)
         SELECT id, uuid, record_date, total_eggs, notes, created_at, updated_at
         FROM egg_records",
        [],
    )?;

    conn.execute("DROP TABLE egg_records", [])?;
    conn.execute("ALTER TABLE egg_records_new RENAME TO egg_records", [])?;

    // Indizes erneut anlegen
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_egg_records_uuid ON egg_records(uuid)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_egg_records_date ON egg_records(record_date DESC)",
        [],
    )?;

    // Trigger erneut anlegen
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_egg_records_timestamp 
         AFTER UPDATE ON egg_records
         BEGIN
            UPDATE egg_records SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    eprintln!("Migration v11 completed");
    Ok(())
}
