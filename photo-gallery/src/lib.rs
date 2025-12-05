//! # Photo Gallery
//!
//! A reusable photo gallery management library with thumbnail generation and storage.
//!
//! This crate provides cross-platform photo management functionality, including:
//! - Photo storage and retrieval
//! - Automatic thumbnail generation (WebP format)
//! - Database integration with SQLite
//! - Support for multiple photo sizes (small, medium, original)
//!
//! ## Platform Separation
//!
//! This crate focuses on cross-platform photo logic. Platform-specific code
//! (e.g., Android JNI camera integration) should remain in the application crate.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use photo_gallery::{PhotoGalleryService, PhotoGalleryConfig};
//!
//! let config = PhotoGalleryConfig {
//!     storage_path: "/path/to/photos".to_string(),
//!     enable_thumbnails: true,
//!     thumbnail_small_size: 128,
//!     thumbnail_medium_size: 512,
//! };
//!
//! let service = PhotoGalleryService::new(config);
//! ```

pub mod models;
pub mod service;
pub mod thumbnail;

#[cfg(feature = "sync")]
pub mod sync;

pub use models::{Photo, PhotoGalleryConfig, PhotoResult, PhotoSize};
pub use service::{PhotoGalleryError, PhotoGalleryService};
pub use thumbnail::{ThumbnailError, create_thumbnails, rename_photo_with_uuid};

#[cfg(feature = "sync")]
pub use sync::{PhotoSyncConfig, PhotoSyncService};
