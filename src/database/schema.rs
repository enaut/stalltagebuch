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
