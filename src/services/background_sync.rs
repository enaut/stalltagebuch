use crate::database;
use crate::error::AppError;
use crate::services::{download_service, sync_service, upload_service};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Background sync configuration
const SYNC_INTERVAL_SECONDS: u64 = 300; // 5 minutes
const RETRY_DELAY_SECONDS: u64 = 60; // 1 minute on error

/// Global flag to control background sync
static SYNC_ENABLED: AtomicBool = AtomicBool::new(false);

/// Starts the background sync loop
///
/// This will continuously sync in the background at regular intervals.
/// Call `stop_background_sync()` to stop it.
pub fn start_background_sync() {
    if SYNC_ENABLED.swap(true, Ordering::SeqCst) {
        eprintln!("Background sync already running");
        return;
    }

    eprintln!("Starting background sync with {} second interval", SYNC_INTERVAL_SECONDS);

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        while SYNC_ENABLED.load(Ordering::SeqCst) {
            runtime.block_on(async {
                // Perform sync cycle
                match perform_sync_cycle().await {
                    Ok(stats) => {
                        eprintln!("Background sync completed: {:?}", stats);
                    }
                    Err(e) => {
                        eprintln!("Background sync error: {}", e);
                        // Wait shorter time before retry on error
                        tokio::time::sleep(Duration::from_secs(RETRY_DELAY_SECONDS)).await;
                        return;
                    }
                }

                // Wait for next sync interval
                tokio::time::sleep(Duration::from_secs(SYNC_INTERVAL_SECONDS)).await;
            });
        }

        eprintln!("Background sync stopped");
    });
}

/// Stops the background sync loop
pub fn stop_background_sync() {
    if SYNC_ENABLED.swap(false, Ordering::SeqCst) {
        eprintln!("Stopping background sync");
    }
}

/// Checks if background sync is running
pub fn is_background_sync_running() -> bool {
    SYNC_ENABLED.load(Ordering::SeqCst)
}

#[derive(Debug, Clone)]
pub struct SyncStats {
    pub quails_uploaded: usize,
    pub events_uploaded: usize,
    pub egg_records_uploaded: usize,
    pub photos_uploaded: usize,
    pub operations_downloaded: usize,
}

/// Performs one complete sync cycle: download remote changes first, then upload local changes
/// 
/// Download-first strategy ensures we get the latest remote state before uploading,
/// reducing conflicts and ensuring we're working with up-to-date data.
async fn perform_sync_cycle() -> Result<SyncStats, AppError> {
    let conn = database::init_database()?;

    // Check if sync is configured and enabled
    let settings = sync_service::load_sync_settings(&conn)?
        .ok_or_else(|| AppError::NotFound("Sync not configured".to_string()))?;

    if !settings.enabled {
        return Err(AppError::Validation("Sync disabled".to_string()));
    }

    // Phase 1: Download remote changes first (new multi-master sync)
    let ops_downloaded = download_service::download_and_merge_ops(&conn).await?;

    // Phase 2: Upload local changes (legacy sync)
    let (quails, events, egg_records, photos) = upload_service::sync_all(&conn).await?;

    Ok(SyncStats {
        quails_uploaded: quails,
        events_uploaded: events,
        egg_records_uploaded: egg_records,
        photos_uploaded: photos,
        operations_downloaded: ops_downloaded,
    })
}

/// Triggers an immediate sync (in addition to scheduled background syncs)
pub async fn sync_now() -> Result<SyncStats, AppError> {
    perform_sync_cycle().await
}
