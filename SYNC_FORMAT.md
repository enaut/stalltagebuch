# Stalltagebuch Sync Format Documentation

## Overview

The Stalltagebuch app synchronizes all data to a Nextcloud server via WebDAV. Data is organized hierarchically with TOML metadata files for structured information and JPG files for photos.


# Multi-Master Sync Format (CRDT Only)

Dieses Dokument beschreibt das aktuell allein gültige Synchronisationsformat. Das frühere hierarchische Legacy Objekt-Layout (profile.toml, event.toml, photo *.toml, egg_record *.toml) wurde vollständig entfernt.

## Ziele
- Multi-Master Betrieb ohne zentrale Autorität
- Konfliktfreie Zusammenführung via logischer Uhr + Revision
- Effiziente inkrementelle Replikation (append-only)
- Minimale Dateianzahl, einfache Serialisierung

## Verzeichnisstruktur
```
<remote_path>/
  sync/
    ops/
      <device_id>/
        YYYYMM/
          <ULID>.ndjson
      <other_device_id>/
        YYYYMM/
          ...
    photos/
      <photo_uuid>.jpg
      <photo_uuid>_thumb.jpg
      ...
```Partitionierung nach Monat (`YYYYMM`) erleichtert begrenztes Laden alter Batches.

## NDJSON Batch Dateien
Eine Datei enthält Zeilen – jede Zeile genau eine Operation:
```json
{"op_id":"01HXYZ...","rev":1,"clock":1731408000123,"entity":"quail","entity_id":"<uuid>","action":"upsert","fields":{"name":"Hilde","gender":"female"}}
{"op_id":"01HXYZ...","rev":2,"clock":1731408002123,"entity":"event","entity_id":"<uuid>","action":"upsert","fields":{"type":"weight","date":"2024-11-12","notes":"840g"}}
{"op_id":"01HXYZ...","rev":3,"clock":1731408003123,"entity":"egg_record","entity_id":"<uuid>","action":"upsert","fields":{"record_date":"2024-11-12","total_eggs":7}}
{"op_id":"01HXYZ...","rev":4,"clock":1731408005123,"entity":"photo","entity_id":"<uuid>","action":"upsert","fields":{"relative":"wachtels/<quail_uuid>/events/<event_uuid>/photos/<photo_uuid>.jpg","thumb":"..."}}
```

### Pflichtfelder
- `op_id`: ULID oder anderer eindeutig sortierbarer Identifier
- `rev`: fortlaufende Revisionsnummer pro Operation (lokal) zur Tie-Break Auflösung
- `clock`: logische Uhr in Millisekunden (Hybrid Logical Clock möglich)
- `entity`: Typ (`quail`, `event`, `egg_record`, `photo`)
- `entity_id`: UUID der Entität
- `action`: `upsert` oder `delete`
- `fields`: Key/Value Map der Änderungen (fehlt bei `delete` optional)

### Feld-Alias Toleranz
- `event`: akzeptiert `type` oder `event_type`; `date` oder `event_date`
- `photo`: akzeptiert `relative` oder `relative_path`; `thumb` oder `thumbnail_path`

### Konfliktauflösung
1. Sortierung aller ankommenden Ops nach `(clock, rev, op_id)`
2. Für jedes Feld Last-Write-Wins
3. `delete` erzeugt Tombstone (`deleted=1`) – spätere `upsert` kann wiederbeleben

### Löschungen
Operation: `{ "action":"delete" }` setzt `deleted` Flag. Physisches Entfernen via periodischer GC (noch offen).

## Upload Ablauf
1. Lokale Änderungen landen im `op_log`
2. Batch Builder sammelt bis Schwellwert (Anzahl oder Zeit)
3. Erzeugt NDJSON Datei unter `ops/<device>/<YYYYMM>/<ULID>.ndjson`
4. Remote Geräte laden neue Dateien und mergen

## Download Ablauf
1. Listen aller unbekannten Dateien unter `sync/ops/*/*/`
2. Stream Parse jeder NDJSON Zeile
3. Anwenden CRDT Regeln
4. Trigger Foto-Nachladen für neue `photo` Einträge (Binary Pfade in Feldern)

## Fotos
Foto-Einträge beinhalten relative Pfade (`relative`) und optional Thumbnail (`thumb`) in den CRDT-Operationen. Die binären Fotodateien werden separat in einem flachen Layout unter `sync/photos/<uuid>.jpg` bzw. `sync/photos/<uuid>_thumb.jpg` hochgeladen und heruntergeladen. Es gibt keine TOML Objekt-Metadateien mehr.

### Upload Prozess
1. Lokal erstellte Fotos werden mit UUID-basiertem Dateinamen gespeichert
2. CRDT Operation erfasst Metadaten (relative_path, thumbnail_path, Zuordnung)
3. Binärdatei wird zu `sync/photos/<uuid>.jpg` hochgeladen
4. Optional: Thumbnail zu `sync/photos/<uuid>_thumb.jpg`

### Download Prozess
1. CRDT-Ops liefern Foto-Metadaten mit UUID
2. System prüft ob `<uuid>.jpg` lokal existiert
3. Falls fehlend: Download von `sync/photos/<uuid>.jpg`
4. Speicherung im lokalen Foto-Verzeichnis

## Offene Punkte
- Batch Kompaktierung (Snapshotting + GC)
- Signierung/Authentizität der Batches
- Priorisierung Foto-Download vs. UI Interaktion

## Zusammenfassung
Die App speichert Entitäten in SQLite und repliziert ausschließlich über CRDT Operationen (NDJSON). Der alte hierarchische Datei-Baum ist verworfen. Synchronisation bedeutet: Ops hochladen, fremde Ops herunterladen, anwenden, Binary Assets bei Bedarf nachladen.
## UUID Usage

All entities have UUIDs (v4) for global uniqueness:
- quails
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
1. Sync quails (2 synced)
   ├─ Create quails/550e8400.../
   ├─ Create quails/550e8400.../photos/
   ├─ Create quails/550e8400.../events/
   └─ Upload profile.toml

2. Sync events (3 synced)
   ├─ Create quails/550e8400.../events/123e4567.../
   ├─ Create quails/550e8400.../events/123e4567.../photos/
   └─ Upload event.toml

3. Sync egg records (5 synced)
   ├─ Create egg_records/
   └─ Upload 789e0123....toml

4. Sync photos (10 synced)
   ├─ Upload a1b2c3d4....jpg
   ├─ Upload a1b2c3d4....toml
   └─ (repeat for each photo)

Result: ✅ 2 quails, 3 Events, 5 Eier-Einträge, 10 Fotos synchronisiert
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

Ziel: Echte, konfliktarme, geräteübergreifende Synchronisierung ohne Datenverlust, auch offline-first. Nextcloud/WebDAV dient als „dummes“ Transport- und Aufbewahrungsmedium. Konflikte werden durch ein append-only Operations-Log pro Gerät vermieden und deterministisch per CRDTs zusammengeführt.

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
- Rollback: Flag OFF → App ignoriert Logs/Snapshots

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
