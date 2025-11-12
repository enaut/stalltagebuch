use crate::error::AppError;
use chrono::NaiveDate;
use rusqlite::types::Type;
use rusqlite::Row;
use serde::{Deserialize, Serialize};

/// Ereignis im Leben einer Wachtel (Status-Änderung, Geburt, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WachtelEvent {
    pub id: Option<i64>,
    pub uuid: String,
    pub wachtel_id: i64,
    pub event_type: EventType,
    pub event_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Geboren,               // Birth
    AmLeben,               // Living (default state)
    Krank,                 // Sick
    Gesund,                // Recovered/Healthy
    MarkiertZumSchlachten, // Marked for slaughter
    Geschlachtet,          // Slaughtered
    Gestorben,             // Died naturally
}

impl EventType {
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Geboren => "geboren",
            EventType::AmLeben => "am_leben",
            EventType::Krank => "krank",
            EventType::Gesund => "gesund",
            EventType::MarkiertZumSchlachten => "markiert_zum_schlachten",
            EventType::Geschlachtet => "geschlachtet",
            EventType::Gestorben => "gestorben",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "geboren" => EventType::Geboren,
            "am_leben" => EventType::AmLeben,
            "krank" => EventType::Krank,
            "gesund" => EventType::Gesund,
            "markiert_zum_schlachten" => EventType::MarkiertZumSchlachten,
            "geschlachtet" => EventType::Geschlachtet,
            "gestorben" => EventType::Gestorben,
            _ => EventType::AmLeben,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            EventType::Geboren => "Geboren",
            EventType::AmLeben => "Am Leben",
            EventType::Krank => "Krank",
            EventType::Gesund => "Gesund",
            EventType::MarkiertZumSchlachten => "Markiert zum Schlachten",
            EventType::Geschlachtet => "Geschlachtet",
            EventType::Gestorben => "Gestorben",
        }
    }

    /// Returns true if this event type represents a final state (death)
    #[allow(dead_code)]
    pub fn is_final(&self) -> bool {
        matches!(self, EventType::Geschlachtet | EventType::Gestorben)
    }

    /// Returns true if this event type is a health-related status
    #[allow(dead_code)]
    pub fn is_health_status(&self) -> bool {
        matches!(self, EventType::Krank | EventType::Gesund)
    }
}

impl WachtelEvent {
    /// Creates a new event
    pub fn new(wachtel_id: i64, event_type: EventType, event_date: NaiveDate) -> Self {
        Self {
            id: None,
            uuid: uuid::Uuid::new_v4().to_string(),
            wachtel_id,
            event_type,
            event_date,
            notes: None,
        }
    }

    /// Validates the event
    pub fn validate(&self) -> Result<(), AppError> {
        // Event date should not be in the future, except for planned events
        let allows_future = matches!(self.event_type, EventType::MarkiertZumSchlachten);
        if !allows_future && self.event_date > chrono::Local::now().date_naive() {
            return Err(AppError::Validation(
                "Ereignisdatum darf nicht in der Zukunft liegen".to_string(),
            ));
        }

        // Notes should not be too long
        if let Some(notes) = &self.notes {
            if notes.len() > 1000 {
                return Err(AppError::Validation(
                    "Notizen dürfen maximal 1000 Zeichen lang sein".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl<'r> TryFrom<&Row<'r>> for WachtelEvent {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'r>) -> Result<Self, Self::Error> {
        let id: i64 = row.get(0)?;
        let uuid: String = row.get(1)?;
        let wachtel_id: i64 = row.get(2)?;
        let event_type_str: String = row.get(3)?;
        let event_date_str: String = row.get(4)?;
        let notes: Option<String> = row.get(5)?;

        let event_date = NaiveDate::parse_from_str(&event_date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(e)))?;

        Ok(WachtelEvent {
            id: Some(id),
            uuid,
            wachtel_id,
            event_type: EventType::from_str(&event_type_str),
            event_date,
            notes,
        })
    }
}
