# Stalltagebuch Sync Format Documentation

## Overview

The Stalltagebuch app synchronizes all data to a Nextcloud server via WebDAV. Data is organized hierarchically with TOML metadata files for structured information and JPG files for photos.

## Directory Structure (Legacy Object Layout)

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

---

## Experimental Multi-Master Sync (Nextcloud/WebDAV)

Ziel: Echte, konfliktarme, geräteübergreifende Synchronisierung ohne Datenverlust, auch offline-first. Nextcloud/WebDAV dient als „dummes“ Transport- und Aufbewahrungsmedium. Konflikte werden durch ein append-only Operations-Log pro Gerät vermieden und deterministisch per CRDTs zusammengeführt. Der bisherige „Legacy Object Layout“-Sync bleibt parallel bestehen, bis das neue Format GA ist.

### Designprinzipien
- Keine gleichzeitigen Schreibzugriffe auf dieselbe Datei: Jedes Gerät schreibt nur neue, eindeutig benannte Log-Dateien.
- Deterministische Zusammenführung: CRDT-Regeln auf Feldebene, identische Resultate auf allen Geräten.
- Idempotenz: Re-Downloads und Replays sind gefahrlos.
- Offline-first: Operationen werden lokal gepuffert und später hochgeladen.

### Remote-Verzeichnisstruktur (neu)

```
Stalltagebuch/
└── sync/                                   # neuer Namespace
      ├── ops/                                # append-only Operationen
      │   └── <device-id>/
      │       └── <YYYYMM>/
      │           └── <ULID>.ndjson           # Batch neuer Operationen
      ├── snapshots/                          # materialisierte Zustände
      │   └── <collection>/                   # z.B. quails, events, eggs
      │       └── <YYYYMMDD>/
      │           └── <ULID>.json             # Zustand + Cursor/Clock
      │
      ├── control/                            # Koordination (Leases/Marker)
      │   └── <collection>/
      │       ├── latest.json                 # zeigt auf den jüngsten Snapshot
      │       └── compactor/<ULID>.lease      # temporäre Lease-Datei
      │
      └── manifests/                          # optionale Indizes/Etags
            └── remote_index.json               # lokales Spiegelbild (optional)
```

Der Legacy-Bereich (Profile/Events/Photos/Egg Records) bleibt unverändert nutzbar; das neue „sync/“-Layout dient ausschließlich der robusten Replikation der Anwendungsdaten. Binärdaten (Fotos) verbleiben weiterhin im Legacy-Bereich; das Log referenziert sie per IDs/Checksums.

### Operationen (NDJSON Schema)
Jede Zeile ist ein JSON-Objekt.

Pflichtfelder:
- `op_id`: ULID/UUIDv7 der Operation
- `entity_type`: `quail` | `event` | `egg_record` | `photo_meta`
- `entity_id`: ULID/UUIDv7 des Zielobjekts
- `clock`: Hybrid Logical Clock `{ ts: int64_ms, lc: u32, device_id: string }`
- `op`: `upsert` | `delete` | `inc` (für Zähler)
- `payload`: diff auf Feldebene (bei `upsert`) oder Increment (bei `inc`)

Beispielzeilen:
```
{"op_id":"01HF...","entity_type":"quail","entity_id":"01HE...","clock":{"ts":1731400000000,"lc":1,"device_id":"dev-ABCD"},"op":"upsert","payload":{"name":{"set":"Frieda"},"gender":{"set":"female"}}}
{"op_id":"01HF...","entity_type":"egg_record","entity_id":"01HG...","clock":{"ts":1731400005000,"lc":2,"device_id":"dev-ABCD"},"op":"inc","payload":{"total_eggs":1}}
{"op_id":"01HF...","entity_type":"event","entity_id":"01HH...","clock":{"ts":1731400010000,"lc":1,"device_id":"dev-EFGH"},"op":"delete"}
```

Konventionen:
- Felder in `payload` nutzen CRDT-Strategien per Feldtyp.
- Keine Mehrfachschreibungen derselben Log-Datei: Jede Batch-Datei entsteht atomar (siehe Upload-Protokoll).

### CRDT-Regeln (pro Feldtyp)
- LWW-Register: Strings, strukturierte Objekte → letzter Schreiber gewinnt nach Ordnung `(ts, lc, device_id)`.
- OR-Set / LWW-Element-Set: Sammlungen (Tags, Beziehungen) → Add/Remove mit Element-UID; Remove gewinnt über Add mit höherer Ordnung.
- PN-Counter: Ganzzahlen, die additiv aggregiert werden (z. B. `total_eggs`).
- Tombstones: `delete` erzeugt Löschmarke; Wiederauferstehung nur mit höherer Ordnung.

### Identität und Ordnung
- Entitäten: ULID/UUIDv7 als stabile IDs (bevorzugt ULID für zeitliche Sortierbarkeit).
- HLC: `ts` aus Systemzeit, `lc` als logischer Zähler bei Uhr-Rückgängen; Tie-Breaker ist `device_id` (lexikographisch).

### Download-Protokoll
1) Indexieren: PROPFIND Depth=1 je `ops/<device>/<YYYYMM>/` und `snapshots/<collection>/<YYYYMMDD>/`.
2) Deltas bestimmen: lokales Manifest `{path -> etag}` vergleichen; neue oder veränderte Dateien herunterladen (GET).
3) Replay: NDJSON Operationen in deterministischer Reihenfolge anwenden (sortiert nach `(ts, lc, device_id, op_id)`).
4) Snapshot nutzen: Wenn Snapshot vorhanden, nur nachfolgenden Zeitraum replays durchführen.

Hinweis: WebDAV REPORT `sync-collection` (RFC 6578) ist für File-Collections häufig nicht verfügbar. Falls unterstützt, statt PROPFIND einsetzen.

### Upload-Protokoll
- Batching: Lokale Änderungen in neue Datei schreiben: `ops/<device>/<YYYYMM>/<ULID>.ndjson`.
- Atomare Erstellung: `PUT ...` mit Header `If-None-Match: *` (verhindert Überschreiben).
- Ordneraufbau: `MKCOL`/`X-NC-WebDAV-AutoMkcol` nutzen, falls Ordner fehlen.
- Checksummen optional via `OC-Checksum`.

### Snapshots und Kompaktierung
- Snapshot-Datei: `snapshots/<collection>/<YYYYMMDD>/<ULID>.json` enthält materialisierten Zustand, Versionsinfo (z. B. letzter `clock`) und ggf. Prüfsumme.
- `latest.json` Update nur per `If-Match` (ETag-Vergleich), um Rennen zu verhindern.
- Compaction-Lease: temporäre Datei `control/<collection>/compactor/<ULID>.lease` mit Ablaufzeit; nur aktiver Lease-Besitzer kompaktiert.

### WebDAV Flows (Beispiele)

REPORT sync-collection (falls verfügbar):
```
REPORT /remote.php/dav/files/<user>/Stalltagebuch/sync/ops/ HTTP/1.1
Depth: 0
Content-Type: text/xml; charset="utf-8"
<?xml version="1.0"?>
<D:sync-collection xmlns:D="DAV:">
   <D:sync-token/>
   <D:sync-level>infinite</D:sync-level>
   <D:prop><D:getetag/></D:prop>
</D:sync-collection>
```

PROPFIND für ETag/Meta:
```
PROPFIND /remote.php/dav/files/<user>/Stalltagebuch/sync/ops/<device>/<YYYYMM>/ HTTP/1.1
Depth: 1
Content-Type: text/xml
<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns">
   <d:prop>
      <d:getetag/>
      <d:getlastmodified/>
      <d:getcontentlength/>
      <oc:fileid/>
   </d:prop>
   </d:propfind>
```

Konditionelles Erstellen (atomar):
```
PUT /remote.php/dav/files/<user>/Stalltagebuch/sync/ops/<device>/<YYYYMM>/<ULID>.ndjson HTTP/1.1
If-None-Match: *
Content-Type: application/x-ndjson
```

`latest.json` Update (verhindert Race):
```
PUT /remote.php/dav/files/<user>/Stalltagebuch/sync/snapshots/<collection>/latest.json HTTP/1.1
If-Match: "<etag-alt>"
Content-Type: application/json
```

### Fehler- und Retry-Strategie
- HTTP 409/412: neuen Dateinamen (ULID) generieren und erneut versuchen.
- Netzfehler/Timeout: exponentielles Backoff, begrenzte Parallelität, maximale Retry-Dauer.
- Datenvalidierung: korrupten Download in Quarantäne verschieben und überspringen; Telemetrie-Eintrag.

### Rollout und Versionierung
- `format_version`: globale Versionsnummer in Snapshots; v1 initial.
- Feature-Flag `experimental_sync` steuert Aktivierung.
- Schattenbetrieb: zunächst nur Schreiben der Logs und Download-Dry-Run; später Pull→Apply→Push aktivieren.
- Rollback: Flag OFF → App ignoriert Logs/Snapshots und nutzt Legacy-Sync weiter.

### Sicherheit & Datenschutz
- Keine Credentials in Logs/Dateiinhalten.
- `device_id`: zufällig/stabil; lokal gespeichert.
- Optional Signaturen/Checksums für Datenintegrität.

### Referenzen
- RFC 6578 WebDAV Sync: https://www.rfc-editor.org/rfc/rfc6578
- RFC 4918 WebDAV: https://www.rfc-editor.org/rfc/rfc4918
- Nextcloud WebDAV: https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html
- Bulk/Chunked Upload: https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/
- MDN ETag: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag
