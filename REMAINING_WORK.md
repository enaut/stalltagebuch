# Remaining Work for Photo Gallery Refactoring

This document outlines the remaining tasks to complete the full refactoring of photo management into the photo-gallery crate.

## Completed ✅

1. **Photo collections schema** - Created `photo_collections` table
2. **Migration logic** - Automatically migrates old photos to collections on startup
3. **Basic Dioxus components** - Created ThumbnailImage, PreviewImage, FullscreenImage, etc. (but they accept data URLs)
4. **Schema v6 migration** - Added to main crate, runs automatically
5. **Collection FK setup** - Added collection_id to quails and events tables

## Remaining Tasks

### 1. Move Photo Service Logic to photo-gallery ❌

**Current state:**
- `src/services/photo_service.rs` is a thin wrapper over PhotoGalleryService
- It handles app-specific concerns (CRDT operation capture, error conversion)

**What needs to be done:**
- Move all CRUD operations fully to photo-gallery crate
- Remove dependency on main crate's AppError
- Make photo-gallery crate handle database operations directly
- Keep only CRDT operation capture in main crate

**Files to modify:**
- `photo-gallery/src/service.rs` - Expand with full CRUD operations
- `src/services/photo_service.rs` - Reduce to minimal CRDT wrapper

### 2. Move Upload/Download Logic to photo-gallery ❌

**Current state:**
- `src/services/upload_service.rs` contains photo upload logic with WebDAV
- Uses app-specific error handling and sync settings
- Parallel upload with JoinSet for 3 concurrent uploads

**What needs to be done:**
- Move `upload_photos_batch()` to photo-gallery crate
- Move `count_pending_photos()` to photo-gallery
- Move `list_remote_photos_simple()` and related WebDAV helpers
- Create sync settings struct in photo-gallery (or accept as parameter)
- Return results that main crate can log/track

**Files to move/modify:**
- Create `photo-gallery/src/upload.rs` with upload logic
- Create `photo-gallery/src/download.rs` with download logic  
- Update `src/services/upload_service.rs` to call photo-gallery
- Update `src/services/download_service.rs` (if exists) similarly

### 3. Move Camera/Gallery Picking to photo-gallery ✅

**Completed:**
- Created `photo-gallery/src/picker.rs` with full Android JNI implementation
- Moved all camera/gallery picking logic to photo-gallery crate
- Main crate's `src/camera.rs` is now a thin wrapper for backward compatibility
- Made MainActivity class name configurable via `AndroidPickerConfig`
- All platform-specific code now in photo-gallery
- Comprehensive `PICKER_API.md` documentation

**Implementation:**
- `photo-gallery/src/picker.rs` - Full Android JNI picker implementation
- `src/camera.rs` - Thin compatibility wrapper
- Configurable via `AndroidPickerConfig { main_activity_class }` 
- Functions: `pick_image()`, `pick_images()`, `capture_photo()`, `has_camera_permission()`
- Platform-agnostic error handling with `PickerError` enum
- Stub implementations for non-Android platforms

### 4. Components That Load Their Own Data ❌

**Current state:**
- Components accept data URLs (base64-encoded images)
- Caller must load photo from database and convert to data URL
- This was a design decision to avoid Rust ownership issues

**What was originally requested:**
- Components should accept `database_connection: &Connection` and `photo_uuid: Uuid`
- Components load their own data from database
- Data retrieved once from remote, then loaded from disk

**Challenge:**
- Dioxus components can't easily hold `&Connection` (not Clone, not Send)
- Need alternative approach: global service, Arc<Mutex<Connection>>, or channels

**Possible solutions:**

**Solution A: Global connection pool**
```rust
// In photo-gallery
pub fn init_photo_connection(conn: Connection);

// Component uses global connection
ThumbnailImage {
    photo_uuid: uuid,
    storage_path: path,
}
```

**Solution B: Pass connection via context**
```rust
// Set up context in app
use_context_provider(|| PhotoGalleryContext::new(conn));

// Component gets from context
ThumbnailImage {
    photo_uuid: uuid,
}
```

**Solution C: Async data loading with signals**
```rust
ThumbnailImage {
    photo_uuid: uuid,
    storage_path: path,
    // Component spawns async task to load data
}
```

**Files to modify:**
- `photo-gallery/src/components.rs` - Refactor all components
- May need new `photo-gallery/src/context.rs` for shared state

### 5. Collection-Based API in Main Crate ❌

**Current state:**
- Main crate still uses quail_id/event_id in photo operations
- `add_quail_photo()`, `add_event_photo()` functions exist

**What should change:**
- Main crate creates collections for quails/events
- Photos added to collections, not directly to quails/events
- Functions like `add_photo_to_collection(collection_id, path)`

**Example new API:**
```rust
// Instead of:
photo_service::add_quail_photo(conn, quail_id, path).await?;

// Use:
let collection_id = get_or_create_quail_collection(conn, quail_id)?;
photo_service::add_photo_to_collection(conn, collection_id, path).await?;
```

**Files to modify:**
- `src/services/photo_service.rs` - Update API
- `src/components/*.rs` - Update photo adding code
- Any component that adds photos to quails/events

## Testing Plan

Once the above is complete:

1. **Unit tests** - Test photo-gallery crate in isolation
2. **Integration tests** - Test main crate with photo-gallery
3. **Migration testing** - Test schema v6 migration with real data
4. **Android testing** - Test on physical Android device
5. **Sync testing** - Test upload/download with Nextcloud

## Estimated Scope

This is a **significant refactoring** that touches many parts of the codebase:

- **High complexity items**: Component data loading, camera/gallery picking
- **Medium complexity items**: Upload/download service migration
- **Lower complexity items**: API updates to use collections

**Recommendation**: Consider a **phased approach**:

Phase 1: Complete upload/download migration ✨ (most valuable)
Phase 2: Refactor components to load their own data
Phase 3: Handle camera/gallery picking (callback vs. full move)
Phase 4: Update all usage to collection-based API

## Questions for Discussion

1. **Camera picking**: Full move or callback API?
2. **Component data loading**: Which solution (global pool, context, or signals)?
3. **API breaking changes**: Ok to require collection_id instead of quail_id/event_id?
4. **Priority**: Which phase should be tackled first?

## Notes

- The schema migration (v6) is already complete and working
- Collections are created automatically from existing photos
- Backward compatibility is maintained (old columns still exist)
- New code should use collections; old code still works
