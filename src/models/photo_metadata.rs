use serde::{Deserialize, Serialize};

/// Metadata for a photo, uploaded to the server as TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub photo_id: String,           // UUID des Fotos
    pub quail_uuid: Option<String>, // Quail-UUID falls zugeordnet
    pub event_uuid: Option<String>, // Event-UUID falls zugeordnet
    pub timestamp: String,          // ISO 8601 Zeitstempel
    pub notes: Option<String>,      // Notizen zum Foto
    pub device_id: String,          // Eindeutige Geräte-ID
    pub checksum: String,           // SHA256 Hash des Fotos
    pub relative_path: String,      // e.g. "quails/{uuid}/photos/{photo_uuid}.jpg"
}

/// Metadata for a quail (master data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuailMetadata {
    pub uuid: String,
    pub name: String,
    pub gender: String,
    pub ring_color: Option<String>,
    pub device_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub has_profile_photo: bool,
}

/// Metadata for an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub uuid: String,
    pub quail_uuid: String,
    pub event_type: String,
    pub event_date: String,
    pub notes: Option<String>,
    pub device_id: String,
    pub created_at: String,
}

/// Metadata for an egg record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EggRecordMetadata {
    pub uuid: String,
    pub record_date: String,
    pub total_eggs: i32,
    pub notes: Option<String>,
    pub device_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl PhotoMetadata {
    /// Converts to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Lädt von TOML String
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

impl QuailMetadata {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

impl EventMetadata {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

impl EggRecordMetadata {
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}
