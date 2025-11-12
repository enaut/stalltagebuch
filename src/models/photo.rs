use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Photo {
    pub uuid: Uuid,
    pub quail_id: Option<Uuid>,
    pub event_id: Option<Uuid>,
    pub path: String,
    pub thumbnail_path: Option<String>,
}
