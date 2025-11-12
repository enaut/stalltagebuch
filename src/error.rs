use std::fmt;

/// Zentrale Error-Typen für die Stalltagebuch-App
#[derive(Debug)]
pub enum AppError {
    /// Datenbankfehler (rusqlite)
    Database(rusqlite::Error),
    /// Dateisystem-Fehler
    Filesystem(std::io::Error),
    /// Validierungsfehler (z.B. ungültige Eingaben)
    Validation(String),
    /// Ressource nicht gefunden
    NotFound(String),
    /// Berechtigung fehlt (z.B. Kamera)
    PermissionDenied(String),
    /// Bildverarbeitungsfehler
    ImageProcessing(String),
    /// Allgemeiner Fehler
    #[allow(dead_code)]
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Database(e) => write!(f, "Datenbankfehler: {}", e),
            AppError::Filesystem(e) => write!(f, "Dateisystem-Fehler: {}", e),
            AppError::Validation(msg) => write!(f, "Validierungsfehler: {}", msg),
            AppError::NotFound(msg) => write!(f, "Nicht gefunden: {}", msg),
            AppError::PermissionDenied(msg) => write!(f, "Berechtigung fehlt: {}", msg),
            AppError::ImageProcessing(msg) => write!(f, "Bildverarbeitungsfehler: {}", msg),
            AppError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Conversions von anderen Error-Typen
impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Filesystem(e)
    }
}

/// User-friendly Fehlermeldungen für UI
impl AppError {
    #[allow(dead_code)]
    pub fn user_message(&self) -> String {
        match self {
            AppError::Database(_) => {
                "Ein Datenbankfehler ist aufgetreten. Bitte versuchen Sie es erneut.".to_string()
            }
            AppError::Filesystem(_) => {
                "Fehler beim Zugriff auf Dateien. Bitte prüfen Sie die App-Berechtigungen."
                    .to_string()
            }
            AppError::Validation(msg) => msg.clone(),
            AppError::NotFound(msg) => format!("{} wurde nicht gefunden.", msg),
            AppError::PermissionDenied(msg) => format!("Berechtigung erforderlich: {}", msg),
            AppError::ImageProcessing(_) => "Fehler beim Verarbeiten des Bildes.".to_string(),
            AppError::Other(msg) => msg.clone(),
        }
    }
}
