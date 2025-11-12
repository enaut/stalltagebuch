use crate::error::AppError;
use rusqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Wachtel {
    pub id: Option<i64>,
    pub uuid: String,
    pub name: String,
    pub gender: Gender,
    pub ring_color: Option<Ringfarbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Ringfarbe {
    Lila,
    Rosa,
    Hellblau,
    Dunkelblau,
    Rot,
    Orange,
    Weiss, // Speicherung als weiss (ASCII) – Anzeige als Weiß
    Gelb,
    Schwarz,
    Gruen, // Speicherung als gruen (ASCII) – Anzeige als Grün
}

impl Ringfarbe {
    pub fn as_str(&self) -> &str {
        match self {
            Ringfarbe::Lila => "lila",
            Ringfarbe::Rosa => "rosa",
            Ringfarbe::Hellblau => "hellblau",
            Ringfarbe::Dunkelblau => "dunkelblau",
            Ringfarbe::Rot => "rot",
            Ringfarbe::Orange => "orange",
            Ringfarbe::Weiss => "weiss",
            Ringfarbe::Gelb => "gelb",
            Ringfarbe::Schwarz => "schwarz",
            Ringfarbe::Gruen => "gruen",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "lila" => Ringfarbe::Lila,
            "rosa" => Ringfarbe::Rosa,
            "hellblau" => Ringfarbe::Hellblau,
            "dunkelblau" => Ringfarbe::Dunkelblau,
            "rot" => Ringfarbe::Rot,
            "orange" => Ringfarbe::Orange,
            "weiß" | "weiss" => Ringfarbe::Weiss,
            "gelb" => Ringfarbe::Gelb,
            "schwarz" => Ringfarbe::Schwarz,
            "grün" | "gruen" => Ringfarbe::Gruen,
            _ => Ringfarbe::Lila, // Fallback – sollte eigentlich nicht passieren
        }
    }

    #[allow(dead_code)]
    pub fn display_name(&self) -> &str {
        match self {
            Ringfarbe::Lila => "Lila",
            Ringfarbe::Rosa => "Rosa",
            Ringfarbe::Hellblau => "Hellblau",
            Ringfarbe::Dunkelblau => "Dunkelblau",
            Ringfarbe::Rot => "Rot",
            Ringfarbe::Orange => "Orange",
            Ringfarbe::Weiss => "Weiß",
            Ringfarbe::Gelb => "Gelb",
            Ringfarbe::Schwarz => "Schwarz",
            Ringfarbe::Gruen => "Grün",
        }
    }

    #[allow(dead_code)]
    pub fn all() -> &'static [Ringfarbe] {
        static ALL: [Ringfarbe; 10] = [
            Ringfarbe::Lila,
            Ringfarbe::Rosa,
            Ringfarbe::Hellblau,
            Ringfarbe::Dunkelblau,
            Ringfarbe::Rot,
            Ringfarbe::Orange,
            Ringfarbe::Weiss,
            Ringfarbe::Gelb,
            Ringfarbe::Schwarz,
            Ringfarbe::Gruen,
        ];
        &ALL
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Unknown,
}

impl Gender {
    pub fn as_str(&self) -> &str {
        match self {
            Gender::Male => "male",
            Gender::Female => "female",
            Gender::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "male" => Gender::Male,
            "female" => Gender::Female,
            _ => Gender::Unknown,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Gender::Male => "Männlich",
            Gender::Female => "Weiblich",
            Gender::Unknown => "Unbekannt",
        }
    }
}

impl Wachtel {
    /// Erstellt eine neue Wachtel mit generierten UUID
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            uuid: uuid::Uuid::new_v4().to_string(),
            name,
            gender: Gender::Unknown,
            ring_color: None,
        }
    }

    /// Validiert alle Felder der Wachtel
    pub fn validate(&self) -> Result<(), AppError> {
        // Name darf nicht leer sein
        if self.name.trim().is_empty() {
            return Err(AppError::Validation(
                "Name darf nicht leer sein".to_string(),
            ));
        }

        // Name sollte nicht zu lang sein
        if self.name.len() > 100 {
            return Err(AppError::Validation(
                "Name darf maximal 100 Zeichen lang sein".to_string(),
            ));
        }

        Ok(())
    }
}

impl<'r> TryFrom<&Row<'r>> for Wachtel {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'r>) -> Result<Self, Self::Error> {
        let id: i64 = row.get(0)?;
        let uuid: String = row.get(1)?;
        let name: String = row.get(2)?;
        let gender_str: String = row.get(3)?;
        let ring_color_opt: Option<String> = row.get(4)?;

        Ok(Wachtel {
            id: Some(id),
            uuid,
            name,
            gender: Gender::from_str(&gender_str),
            ring_color: ring_color_opt.map(|s| Ringfarbe::from_str(&s)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_wachtel() {
        let wachtel = Wachtel::new("Test".to_string());
        assert_eq!(wachtel.name, "Test");
        assert_eq!(wachtel.gender, Gender::Unknown);
        assert!(wachtel.uuid.len() > 0);
    }

    #[test]
    fn test_validate_empty_name() {
        let mut wachtel = Wachtel::new("".to_string());
        wachtel.name = "   ".to_string();
        assert!(wachtel.validate().is_err());
    }

    #[test]
    fn test_gender_conversion() {
        assert_eq!(Gender::from_str("male"), Gender::Male);
        assert_eq!(Gender::from_str("female"), Gender::Female);
        assert_eq!(Gender::from_str("invalid"), Gender::Unknown);
    }
}
