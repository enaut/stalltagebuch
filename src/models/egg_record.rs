use crate::error::AppError;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use rusqlite::types::Type;
use rusqlite::Row;
use uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EggRecord {
    pub id: Option<i64>,
    pub uuid: String,
    pub record_date: NaiveDate,
    pub total_eggs: i32,
    pub notes: Option<String>,
}

impl EggRecord {
    /// Erstellt einen neuen Eier-Eintrag
    #[allow(dead_code)]
    pub fn new(record_date: NaiveDate, total_eggs: i32) -> Self {
        Self {
            id: None,
            uuid: uuid::Uuid::new_v4().to_string(),
            record_date,
            total_eggs,
            notes: None,
        }
    }

    /// Validiert den Eier-Eintrag
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), AppError> {
        // Anzahl darf nicht negativ sein
        if self.total_eggs < 0 {
            return Err(AppError::Validation(
                "Anzahl der Eier darf nicht negativ sein".to_string(),
            ));
        }

        // Realistische Obergrenze (z.B. max 100 Eier pro Tag)
        if self.total_eggs > 100 {
            return Err(AppError::Validation(
                "Anzahl scheint unrealistisch hoch zu sein".to_string(),
            ));
        }

        // Datum darf nicht zu weit in der Zukunft liegen
        let today = chrono::Local::now().date_naive();
        if self.record_date > today {
            return Err(AppError::Validation(
                "Datum darf nicht in der Zukunft liegen".to_string(),
            ));
        }

        Ok(())
    }
}

impl<'r> TryFrom<&Row<'r>> for EggRecord {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'r>) -> Result<Self, Self::Error> {
        let id: i64 = row.get(0)?;
        let uuid: String = row.get(1)?;
        let date_str: String = row.get(2)?;
        let total_eggs: i32 = row.get(3)?;
        let notes: Option<String> = row.get(4)?;

        let record_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, Type::Text, Box::new(e)))?;

        Ok(EggRecord {
            id: Some(id),
            uuid,
            record_date,
            total_eggs,
            notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_egg_record() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 9).unwrap();
        let record = EggRecord::new(date, 5);
        assert_eq!(record.total_eggs, 5);
        assert_eq!(record.record_date, date);
    }

    #[test]
    fn test_validate_negative_eggs() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 9).unwrap();
        let mut record = EggRecord::new(date, 5);
        record.total_eggs = -1;
        assert!(record.validate().is_err());
    }

    #[test]
    fn test_validate_too_many_eggs() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 9).unwrap();
        let record = EggRecord::new(date, 150);
        assert!(record.validate().is_err());
    }
}
