use serde::{Deserialize, Serialize};

/// Metadata für ein Photo, wird als TOML auf den Server hochgeladen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub photo_id: String,          // UUID des Fotos
    pub wachtel_id: Option<i64>,   // Wachtel-ID falls zugeordnet
    pub wachtel_uuid: Option<String>, // Wachtel-UUID falls zugeordnet
    pub event_id: Option<i64>,     // Event-ID falls zugeordnet
    pub event_uuid: Option<String>, // Event-UUID falls zugeordnet
    pub timestamp: String,         // ISO 8601 Zeitstempel
    pub notes: Option<String>,     // Notizen zum Foto
    pub device_id: String,         // Eindeutige Geräte-ID
    pub checksum: String,          // SHA256 Hash des Fotos
    pub is_profile: bool,          // Profilbild?
    pub relative_path: String,     // z.B. "wachtels/{uuid}/photos/{photo_uuid}.jpg"
}

/// Metadata für eine Wachtel (Stammdaten)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WachtelMetadata {
    pub uuid: String,
    pub name: String,
    pub gender: String,
    pub ring_color: Option<String>,
    pub device_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub has_profile_photo: bool,
}

/// Metadata für ein Event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub uuid: String,
    pub wachtel_uuid: String,
    pub event_type: String,
    pub event_date: String,
    pub notes: Option<String>,
    pub device_id: String,
    pub created_at: String,
}

/// Metadata für einen Eier-Eintrag
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
    /// Konvertiert zu TOML String
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Lädt von TOML String
    pub fn from_toml(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }
}

impl WachtelMetadata {
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
