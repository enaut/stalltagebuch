use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EggRecord {
    pub id: Option<i64>,
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
            record_date,
            total_eggs,
            notes: None,
        }
    }
    
    /// Erstellt einen Eintrag für heute
    #[allow(dead_code)]
    pub fn today(total_eggs: i32) -> Self {
        Self::new(chrono::Local::now().date_naive(), total_eggs)
    }
    
    /// Validiert den Eier-Eintrag
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<(), AppError> {
        // Anzahl darf nicht negativ sein
        if self.total_eggs < 0 {
            return Err(AppError::Validation("Anzahl der Eier darf nicht negativ sein".to_string()));
        }
        
        // Realistische Obergrenze (z.B. max 100 Eier pro Tag)
        if self.total_eggs > 100 {
            return Err(AppError::Validation("Anzahl scheint unrealistisch hoch zu sein".to_string()));
        }
        
        // Datum darf nicht zu weit in der Zukunft liegen
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
    fn test_new_egg_record() {
        let date = NaiveDate::from_ymd_opt(2025, 11, 9).unwrap();
        let record = EggRecord::new(date, 5);
        assert_eq!(record.total_eggs, 5);
        assert_eq!(record.record_date, date);
    }
    
    #[test]
    fn test_validate_negative_eggs() {
        let mut record = EggRecord::today(5);
        record.total_eggs = -1;
        assert!(record.validate().is_err());
    }
    
    #[test]
    fn test_validate_too_many_eggs() {
        let record = EggRecord::today(150);
        assert!(record.validate().is_err());
    }
}
