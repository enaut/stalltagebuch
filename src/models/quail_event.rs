use crate::error::AppError;
use chrono::NaiveDate;
use rusqlite::types::Type;
use rusqlite::Row;
use serde::{Deserialize, Serialize};

/// Event in the life of a quail (status change, birth, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuailEvent {
    pub id: Option<i64>,
    pub uuid: String,
    pub quail_id: i64,
    pub event_type: EventType,
    pub event_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventType {
    Born,               // Birth
    Alive,              // Living (default state)
    Sick,               // Sick
    Healthy,            // Recovered/Healthy
    MarkedForSlaughter, // Marked for slaughter
    Slaughtered,        // Slaughtered
    Died,               // Died naturally
}

impl EventType {
    pub fn as_str(&self) -> &str {
        match self {
            EventType::Born => "born",
            EventType::Alive => "alive",
            EventType::Sick => "sick",
            EventType::Healthy => "healthy",
            EventType::MarkedForSlaughter => "marked_for_slaughter",
            EventType::Slaughtered => "slaughtered",
            EventType::Died => "died",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "born" | "geboren" => EventType::Born,
            "alive" | "am_leben" => EventType::Alive,
            "sick" | "krank" => EventType::Sick,
            "healthy" | "gesund" => EventType::Healthy,
            "marked_for_slaughter" | "markiert_zum_schlachten" => EventType::MarkedForSlaughter,
            "slaughtered" | "geschlachtet" => EventType::Slaughtered,
            "died" | "gestorben" => EventType::Died,
            _ => EventType::Alive,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            EventType::Born => "Geboren",
            EventType::Alive => "Am Leben",
            EventType::Sick => "Krank",
            EventType::Healthy => "Gesund",
            EventType::MarkedForSlaughter => "Markiert zum Schlachten",
            EventType::Slaughtered => "Geschlachtet",
            EventType::Died => "Gestorben",
        }
    }

    /// Returns true if this event type represents a final state (death)
    #[allow(dead_code)]
    pub fn is_final(&self) -> bool {
        matches!(self, EventType::Slaughtered | EventType::Died)
    }

    /// Returns true if this event type is a health-related status
    #[allow(dead_code)]
    pub fn is_health_status(&self) -> bool {
        matches!(self, EventType::Sick | EventType::Healthy)
    }
}

impl QuailEvent {
    /// Creates a new event
    pub fn new(quail_id: i64, event_type: EventType, event_date: NaiveDate) -> Self {
        Self {
            id: None,
            uuid: uuid::Uuid::new_v4().to_string(),
            quail_id,
            event_type,
            event_date,
            notes: None,
        }
    }

    /// Validates the event
    pub fn validate(&self) -> Result<(), AppError> {
        // Event date should not be in the future, except for planned events
        let allows_future = matches!(self.event_type, EventType::MarkedForSlaughter);
        if !allows_future && self.event_date > chrono::Local::now().date_naive() {
            return Err(AppError::Validation(
                "Event date must not be in the future".to_string(),
            ));
        }

        // Notes should not be too long
        if let Some(notes) = &self.notes {
            if notes.len() > 1000 {
                return Err(AppError::Validation(
                    "Notes must not exceed 1000 characters".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl<'r> TryFrom<&Row<'r>> for QuailEvent {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'r>) -> Result<Self, Self::Error> {
        let id: i64 = row.get(0)?;
        let uuid: String = row.get(1)?;
        let quail_id: i64 = row.get(2)?;
        let event_type_str: String = row.get(3)?;
        let event_date_str: String = row.get(4)?;
        let notes: Option<String> = row.get(5)?;

        let event_date = NaiveDate::parse_from_str(&event_date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, Type::Text, Box::new(e)))?;

        Ok(QuailEvent {
            id: Some(id),
            uuid,
            quail_id,
            event_type: EventType::from_str(&event_type_str),
            event_date,
            notes,
        })
    }
}
