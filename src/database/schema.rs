use rusqlite::{Connection, Result};

/// Initialize complete database schema for the Quail Diary app
pub fn init_schema(conn: &Connection) -> Result<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Schema version table for future migrations
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Check if schema already exists
    let current_version: i32 = conn
        .query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current_version < 1 {
        create_schema(conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    Ok(())
}

/// Create the complete schema (version 1) - clean slate with English naming
fn create_schema(conn: &Connection) -> Result<()> {
    // Table: quails (Quail profiles)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quails (
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

    // Indexes for quails
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quails_name ON quails(name)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quails_uuid ON quails(uuid)",
        [],
    )?;

    // Trigger for updated_at in quails
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_quails_timestamp 
         AFTER UPDATE ON quails
         BEGIN
            UPDATE quails SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Table: quail_events (Life events for quails)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quail_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            quail_id INTEGER NOT NULL,
            event_type TEXT CHECK(event_type IN ('born', 'alive', 'sick', 'healthy', 'marked_for_slaughter', 'slaughtered', 'died')) NOT NULL,
            event_date TEXT NOT NULL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (quail_id) REFERENCES quails(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Indexes for quail_events
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quail_events_quail_id ON quail_events(quail_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quail_events_date ON quail_events(event_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quail_events_type ON quail_events(event_type)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quail_events_uuid ON quail_events(uuid)",
        [],
    )?;

    // Trigger for updated_at in quail_events
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_quail_events_timestamp 
         AFTER UPDATE ON quail_events
         BEGIN
            UPDATE quail_events SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Table: photos (Photo storage with optional linking to quails OR events)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS photos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            quail_id INTEGER,
            event_id INTEGER,
            path TEXT NOT NULL,
            thumbnail_path TEXT,
            is_profile INTEGER NOT NULL DEFAULT 0 CHECK(is_profile IN (0,1)),
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (quail_id) REFERENCES quails(id) ON DELETE CASCADE,
            FOREIGN KEY (event_id) REFERENCES quail_events(id) ON DELETE CASCADE,
            CHECK( (quail_id IS NOT NULL AND event_id IS NULL) OR (quail_id IS NULL AND event_id IS NOT NULL) )
        )",
        [],
    )?;

    // Indexes for photos
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photos_quail ON photos(quail_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photos_event ON photos(event_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_photos_uuid ON photos(uuid)",
        [],
    )?;

    // Trigger for updated_at in photos
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_photos_timestamp 
         AFTER UPDATE ON photos
         BEGIN
            UPDATE photos SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Table: egg_records (Daily egg production tracking)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS egg_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            record_date TEXT NOT NULL UNIQUE,
            total_eggs INTEGER NOT NULL DEFAULT 0 CHECK(total_eggs >= 0),
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Indexes for egg_records
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_egg_records_date ON egg_records(record_date DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_egg_records_uuid ON egg_records(uuid)",
        [],
    )?;

    // Trigger for updated_at in egg_records
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_egg_records_timestamp 
         AFTER UPDATE ON egg_records
         BEGIN
            UPDATE egg_records SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Table: sync_settings (Nextcloud WebDAV synchronization settings)
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

    // Trigger for updated_at in sync_settings
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS update_sync_settings_timestamp 
         AFTER UPDATE ON sync_settings
         BEGIN
            UPDATE sync_settings SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
         END",
        [],
    )?;

    // Table: sync_queue (Queue for pending photo uploads)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            photo_id INTEGER NOT NULL,
            status TEXT CHECK(status IN ('pending', 'uploading', 'completed', 'failed')) NOT NULL DEFAULT 'pending',
            retry_count INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (photo_id) REFERENCES photos(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Indexes for sync_queue
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sync_queue_status ON sync_queue(status)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_sync_queue_photo ON sync_queue(photo_id)",
        [],
    )?;

    // Trigger for updated_at in sync_queue
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
