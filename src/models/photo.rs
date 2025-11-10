use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Photo {
    pub id: Option<i64>,
    pub wachtel_id: Option<i64>,
    pub event_id: Option<i64>,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub is_profile: bool,
}
