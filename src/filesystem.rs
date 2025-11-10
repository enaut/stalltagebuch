use std::path::PathBuf;
use std::fs;
use std::io::Result;

#[cfg(target_os = "android")]
fn android_files_dir() -> Option<PathBuf> {
    use jni::{objects::{JObject, JString}, JavaVM};
    unsafe {
        let ctx = ndk_context::android_context();
        let vm = JavaVM::from_raw(ctx.vm().cast()).ok()?;
        let mut env = vm.attach_current_thread().ok()?; // mutable for JNI calls
        let activity = JObject::from_raw(ctx.context().cast());
        let files_dir = env
            .call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])
            .ok()?
            .l()
            .ok()?;
        let abs_path_obj = env
            .call_method(files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        let abs_path_jstring: JString = JString::from(abs_path_obj);
        let abs_path: String = env.get_string(&abs_path_jstring).ok()?.into();
        Some(PathBuf::from(abs_path))
    }
}

/// Get the app data directory for the current platform
pub fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "android")]
    {
        if let Some(dir) = android_files_dir() { return dir; }
        // Fallbacks
        for d in [
            "/data/user/0/de.teilgedanken.stalltagebuch/files",
            "/data/data/de.teilgedanken.stalltagebuch/files",
        ] {
            let p = PathBuf::from(d);
            if p.exists() { return p; }
        }
        PathBuf::from("./data")
    }
    
    #[cfg(not(target_os = "android"))]
    {
        // On desktop, use ./data directory
        PathBuf::from("./data")
    }
}

/// Write test file to app storage
pub fn write_test_file(filename: &str, content: &[u8]) -> Result<PathBuf> {
    let dir = get_app_data_dir();
    fs::create_dir_all(&dir)?;
    
    let filepath = dir.join(filename);
    fs::write(&filepath, content)?;
    
    Ok(filepath)
}

/// Read test file from app storage
pub fn read_test_file(filename: &str) -> Result<Vec<u8>> {
    let filepath = get_app_data_dir().join(filename);
    fs::read(filepath)
}

/// Check if test file exists
#[allow(dead_code)]
pub fn test_file_exists(filename: &str) -> bool {
    get_app_data_dir().join(filename).exists()
}

/// List all files in app data directory
pub fn list_files() -> Result<Vec<String>> {
    let dir = get_app_data_dir();
    
    if !dir.exists() {
        return Ok(Vec::new());
    }
    
    let entries = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| {
            entry.file_name().to_str().map(|s| s.to_string())
        })
        .collect();
    
    Ok(entries)
}
