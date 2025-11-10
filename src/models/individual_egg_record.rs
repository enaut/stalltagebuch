use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use crate::error::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndividualEggRecord {
    pub id: Option<i64>,
    pub wachtel_id: i64,
    pub record_date: NaiveDate,
    pub eggs_laid: i32,
    pub notes: Option<String>,
}

impl IndividualEggRecord {
    /// Erstellt einen neuen individuellen Eier-Eintrag
    pub fn new(wachtel_id: i64, record_date: NaiveDate, eggs_laid: i32) -> Self {
        Self {
            id: None,
            wachtel_id,
            record_date,
            eggs_laid,
            notes: None,
        }
    }
    
    /// Erstellt einen Eintrag für heute
    pub fn today(wachtel_id: i64, eggs_laid: i32) -> Self {
        Self::new(wachtel_id, chrono::Local::now().date_naive(), eggs_laid)
    }
    
    /// Validiert den individuellen Eier-Eintrag
    pub fn validate(&self) -> Result<(), AppError> {
        // Anzahl darf nicht negativ sein
        if self.eggs_laid < 0 {
            return Err(AppError::Validation("Anzahl der Eier darf nicht negativ sein".to_string()));
        }
        
        // Eine Wachtel kann maximal 1-2 Eier pro Tag legen
        if self.eggs_laid > 2 {
            return Err(AppError::Validation("Eine Wachtel kann maximal 2 Eier pro Tag legen".to_string()));
        }
        
        // Datum darf nicht in der Zukunft liegen
        let today = chrono::Local::now().date_naive();
        if self.record_date > today {
            return Err(AppError::Validation("Datum darf nicht in der Zukunft liegen".to_string()));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_new_individual_egg_record() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 9).unwrap();
        let record = IndividualEggRecord::new(1, date, 1);
        assert_eq!(record.wachtel_id, 1);
        assert_eq!(record.eggs_laid, 1);
        assert_eq!(record.record_date, date);
    }
    
    #[test]
    fn test_validate_negative_eggs() {
        let mut record = IndividualEggRecord::today(1, 1);
        record.eggs_laid = -1;
        assert!(record.validate().is_err());
    }
    
    #[test]
    fn test_validate_too_many_eggs() {
        let record = IndividualEggRecord::today(1, 5);
        assert!(record.validate().is_err());
    }
}
