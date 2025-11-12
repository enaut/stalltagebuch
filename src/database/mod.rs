pub mod schema;

use rusqlite::Connection;
use std::path::PathBuf;
use crate::error::AppError;

#[cfg(target_os = "android")]
use jni::objects::JObject;
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use ndk_context::android_context;

/// Gibt das App-Verzeichnis zurück (für Fotos etc.)
#[cfg(target_os = "android")]
pub fn get_app_directory() -> Option<PathBuf> {
    android_files_dir().ok()
}

#[cfg(not(target_os = "android"))]
pub fn get_app_directory() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

/// Gibt den Pfad zum Datenbank-Verzeichnis zurück
pub fn get_database_path() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        android_files_dir()
            .unwrap_or_else(|_| PathBuf::from("/data/local/tmp/stalltagebuch"))
            .join("stalltagebuch.db")
    }
    
    #[cfg(not(target_os = "android"))]
    {
        PathBuf::from("./data/stalltagebuch.db")
    }
}

#[cfg(target_os = "android")]
fn android_files_dir() -> Result<PathBuf, AppError> {
    use jni::JavaVM;
    
    let vm_ptr = android_context()
        .vm()
        as *mut jni::sys::JavaVM;
    
    let vm = unsafe { JavaVM::from_raw(vm_ptr) }
        .map_err(|e| AppError::Other(format!("JavaVM creation failed: {}", e)))?;
    
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| AppError::Other(format!("Failed to attach thread: {}", e)))?;
    
    let context_ptr = android_context().context();
    let context = unsafe { JObject::from_raw(context_ptr as jni::sys::jobject) };
    
    get_files_dir(&mut env, &context)
}

#[cfg(target_os = "android")]
fn get_files_dir(env: &mut JNIEnv, context: &JObject) -> Result<PathBuf, AppError> {
    let file = env
        .call_method(context, "getFilesDir", "()Ljava/io/File;", &[])
        .map_err(|e| AppError::Other(format!("getFilesDir failed: {}", e)))?;
    
    let file_obj = file.l()
        .map_err(|e| AppError::Other(format!("Failed to get file object: {}", e)))?;
    
    let path_jstring = env
        .call_method(file_obj, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .map_err(|e| AppError::Other(format!("getAbsolutePath failed: {}", e)))?;
    
    let path_obj = path_jstring.l()
        .map_err(|e| AppError::Other(format!("Failed to get path object: {}", e)))?;
    
    let path_str: String = env
        .get_string(&path_obj.into())
        .map_err(|e| AppError::Other(format!("Failed to get string: {}", e)))?
        .into();
    
    Ok(PathBuf::from(path_str))
}

/// Initialisiert die Datenbank mit vollständigem Schema
pub fn init_database() -> Result<Connection, AppError> {
    let db_path = get_database_path();
    
    // Sicherstellen dass das Verzeichnis existiert
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let conn = Connection::open(&db_path)?;
    
    // Schema initialisieren
    schema::init_schema(&conn)?;
    
    // Update-Triggers erstellen
    schema::create_update_triggers(&conn)?;
    
    Ok(conn)
}

/// Testet die Datenbankverbindung
#[allow(dead_code)]
pub fn test_connection() -> Result<(), AppError> {
    let conn = init_database()?;
    
    // Einfache Query zum Testen
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
        [],
        |row| row.get(0),
    )?;
    
    if count < 3 {
        return Err(AppError::Database(rusqlite::Error::QueryReturnedNoRows));
    }
    
    Ok(())
}
