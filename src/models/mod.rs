pub mod egg_record;
pub mod photo;
pub mod photo_metadata;
pub mod quail;
pub mod quail_event;
pub mod sync_settings;

pub use egg_record::EggRecord;
pub use photo::Photo;
pub use photo_metadata::{EggRecordMetadata, EventMetadata, PhotoMetadata, QuailMetadata};
pub use quail::{Gender, Quail, RingColor};
pub use quail_event::{EventType, QuailEvent};
pub use sync_settings::SyncSettings;
