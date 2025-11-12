# Stalltagebuch Sync Format Documentation

## Overview

The Stalltagebuch app synchronizes all data to a Nextcloud server via WebDAV. Data is organized hierarchically with TOML metadata files for structured information and JPG files for photos.

## Directory Structure

```
Stalltagebuch/                          # Root sync directory
├── wachtels/                           # All quail data
│   └── {wachtel-uuid}/                 # Individual quail folder
│       ├── profile.toml                # Quail master data
│       ├── photos/                     # Quail photos (including profile photo)
│       │   ├── {photo-uuid}.jpg        # Photo file
│       │   └── {photo-uuid}.toml       # Photo metadata
│       └── events/                     # Quail events
│           └── {event-uuid}/           # Individual event
│               ├── event.toml          # Event metadata
│               └── photos/             # Event-specific photos
│                   ├── {photo-uuid}.jpg
│                   └── {photo-uuid}.toml
├── egg_records/                        # Daily egg records
│   └── {egg-record-uuid}.toml          # Daily egg count metadata
└── orphaned_photos/                    # Photos without assignment
    ├── {photo-uuid}.jpg
    └── {photo-uuid}.toml
```

## Metadata Formats

### Quail Profile (`profile.toml`)

Contains master data for a quail.

```toml
uuid = "550e8400-e29b-41d4-a716-446655440000"
name = "Frieda"
gender = "female"  # or "male"
ring_color = "blue"  # optional
device_id = "device-12345"
created_at = "2025-01-15T10:30:00Z"
updated_at = "2025-11-12T14:22:00Z"
has_profile_photo = true  # boolean flag
```

**Key Points:**
- Profile photo is stored in `photos/` folder like any other photo
- `has_profile_photo` is just a flag indicating existence
- Profile photo is identified by `is_profile = true` in photo metadata

### Event Metadata (`event.toml`)

Describes lifecycle or health events for a quail.

```toml
uuid = "123e4567-e89b-12d3-a456-426614174000"
wachtel_uuid = "550e8400-e29b-41d4-a716-446655440000"
event_type = "geboren"  # or: am_leben, krank, gesund, markiert_zum_schlachten, geschlachtet, gestorben
event_date = "2025-01-15"
notes = "Aus Brutkasten, gesund"  # optional
device_id = "device-12345"
created_at = "2025-01-15T10:30:00Z"
```

**Event Types:**
- `geboren` - Born
- `am_leben` - Alive (status update)
- `krank` - Sick
- `gesund` - Healthy
- `markiert_zum_schlachten` - Marked for slaughter
- `geschlachtet` - Slaughtered
- `gestorben` - Died

### Photo Metadata (`{photo-uuid}.toml`)

Accompanies each photo file with contextual information.

```toml
photo_id = "a1b2c3d4-e5f6-7890-abcd-ef1234567890"
wachtel_id = 5  # optional, local DB ID
wachtel_uuid = "550e8400-e29b-41d4-a716-446655440000"  # optional
event_id = 12  # optional, local DB ID
event_uuid = "123e4567-e89b-12d3-a456-426614174000"  # optional
timestamp = "2025-11-12T14:22:00Z"
notes = "Gutes Foto vom Kopf"  # optional, from event or photo context
device_id = "device-12345"
checksum = "abc123def456..."  # SHA256 hash
is_profile = true  # or false
relative_path = "wachtels/550e8400-e29b-41d4-a716-446655440000/photos/a1b2c3d4-e5f6-7890-abcd-ef1234567890.jpg"
```

**Key Points:**
- `is_profile = true` identifies the quail's profile photo
- Profile photos are NOT stored separately; they remain in `photos/` folder
- `relative_path` indicates the photo's location in the sync structure
- Photos can be associated with a quail, an event, or neither (orphaned)

### Egg Record Metadata (`{egg-record-uuid}.toml`)

Daily egg production records.

```toml
uuid = "789e0123-e45b-67d8-a901-234567890abc"
record_date = "2025-11-12"
total_eggs = 15
notes = "Alle Eier in gutem Zustand"  # optional
device_id = "device-12345"
created_at = "2025-11-12T18:00:00Z"
updated_at = "2025-11-12T18:30:00Z"
```

**Key Points:**
- One record per day across all quails
- `total_eggs` must be >= 0
- No direct photo association (photos would be linked via event if needed)

## Synchronization Logic

### Full Sync Order

1. **Wachtels** - Master data and folder structure
2. **Events** - Event metadata and folders
3. **Egg Records** - Daily production data
4. **Photos** - All photos with metadata

### Photo Path Determination

Photos are placed based on their associations:

1. **Event Photo**: `wachtels/{wachtel-uuid}/events/{event-uuid}/photos/{photo-uuid}.jpg`
2. **Quail Photo (incl. profile)**: `wachtels/{wachtel-uuid}/photos/{photo-uuid}.jpg`
3. **Orphaned Photo**: `orphaned_photos/{photo-uuid}.jpg`

**Important:** Profile photos are NOT treated differently in terms of storage location. The only distinction is the `is_profile` flag in their metadata.

### Profile Photo Handling

- Profile photo is stored in `wachtels/{wachtel-uuid}/photos/` like any other photo
- Identified by `is_profile = true` in photo metadata
- `profile.toml` contains `has_profile_photo` boolean flag
- Changing profile photo only requires updating the `is_profile` flag
- No file copying or moving needed when changing profile photos

### Directory Creation

Folders are created automatically via WebDAV `MKCOL` before uploading files:
- `wachtels/{uuid}/`
- `wachtels/{uuid}/photos/`
- `wachtels/{uuid}/events/`
- `wachtels/{uuid}/events/{event-uuid}/`
- `wachtels/{uuid}/events/{event-uuid}/photos/`
- `egg_records/`
- `orphaned_photos/`

### Conflict Resolution

- UUIDs ensure uniqueness across devices
- `device_id` tracks data origin
- Timestamps (`created_at`, `updated_at`) help identify latest version
- No automatic merge - manual conflict resolution required

## UUID Usage

All entities have UUIDs (v4) for global uniqueness:
- Wachtels (quails)
- Events
- Photos
- Egg Records

UUIDs are:
- Generated on creation (not on sync)
- Immutable
- Used for file/folder naming
- Enable multi-device synchronization

## Data Consistency

### Required Fields
- All entities must have `uuid`
- All entities must have `device_id`
- Timestamps must be ISO 8601 format
- Photo checksums must be SHA256

### Validation
- Egg counts must be non-negative
- Event types must match enum values
- Dates must be valid
- Gender must be "male" or "female"

## Example Sync Session

```
1. Sync wachtels (2 synced)
   ├─ Create wachtels/550e8400.../
   ├─ Create wachtels/550e8400.../photos/
   ├─ Create wachtels/550e8400.../events/
   └─ Upload profile.toml

2. Sync events (3 synced)
   ├─ Create wachtels/550e8400.../events/123e4567.../
   ├─ Create wachtels/550e8400.../events/123e4567.../photos/
   └─ Upload event.toml

3. Sync egg records (5 synced)
   ├─ Create egg_records/
   └─ Upload 789e0123....toml

4. Sync photos (10 synced)
   ├─ Upload a1b2c3d4....jpg
   ├─ Upload a1b2c3d4....toml
   └─ (repeat for each photo)

Result: ✅ 2 Wachtels, 3 Events, 5 Eier-Einträge, 10 Fotos synchronisiert
```

## Technical Details

### WebDAV Implementation
- Uses `reqwest-dav` crate
- Basic authentication with app password
- Endpoint: `{server}/remote.php/dav/files/{username}/{remote_path}`

### TOML Serialization
- Pretty-printed format
- UTF-8 encoding
- Standard TOML tables (no inline tables for readability)

### Photo Storage
- JPG format only
- Filename: `{uuid}.jpg`
- Checksum prevents duplicate uploads
- Original files kept locally after sync
