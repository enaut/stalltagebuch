//! Dioxus UI components for photo gallery
//!
//! This module provides reusable photo display components.
//! Components can either accept pre-loaded data URLs or load their own data
//! from the database using photo UUIDs.

#[cfg(feature = "components")]
use dioxus::prelude::*;

#[cfg(feature = "components")]
use rusqlite::Connection;

#[cfg(feature = "components")]
use std::path::PathBuf;

#[cfg(feature = "components")]
/// Configuration for photo gallery components
#[derive(Clone)]
pub struct PhotoGalleryContext {
    pub storage_path: String,
}

#[cfg(feature = "components")]
impl PhotoGalleryContext {
    pub fn new(storage_path: String) -> Self {
        Self { storage_path }
    }

    /// Load photo data from storage and convert to data URL
    fn load_photo_data(&self, relative_path: &str, size: PhotoSize) -> Option<String> {
        use base64::{engine::general_purpose, Engine as _};

        let abs_path = if relative_path.starts_with('/') {
            PathBuf::from(relative_path)
        } else {
            PathBuf::from(&self.storage_path).join(relative_path)
        };

        let file_path = match size {
            PhotoSize::Original => abs_path,
            PhotoSize::Small => {
                let mut thumb_path = abs_path.clone();
                if let Some(name) = abs_path.file_stem() {
                    thumb_path
                        .set_file_name(format!("{}_thumb_small.webp", name.to_string_lossy()));
                }
                thumb_path
            }
            PhotoSize::Medium => {
                let mut thumb_path = abs_path.clone();
                if let Some(name) = abs_path.file_stem() {
                    thumb_path
                        .set_file_name(format!("{}_thumb_medium.webp", name.to_string_lossy()));
                }
                thumb_path
            }
        };

        if file_path.exists() {
            if let Ok(bytes) = std::fs::read(&file_path) {
                let mime_type = if file_path.extension().and_then(|s| s.to_str()) == Some("webp") {
                    "image/webp"
                } else {
                    "image/jpeg"
                };
                let encoded = general_purpose::STANDARD.encode(&bytes);
                return Some(format!("data:{};base64,{}", mime_type, encoded));
            }
        }
        None
    }
}

#[cfg(feature = "components")]
#[derive(Debug, Clone, Copy)]
enum PhotoSize {
    Original,
    Small,
    Medium,
}

#[cfg(feature = "components")]
#[derive(Debug, Clone)]
enum ImageLoadState {
    Loading,
    Loaded(String),
    Failed,
}

#[cfg(feature = "components")]
/// Helper function to query photo path from database
/// This is meant to be called by the parent component before rendering
pub fn get_photo_path(conn: &Connection, photo_uuid: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT COALESCE(relative_path, path) FROM photos WHERE uuid = ?1 AND deleted = 0",
        [photo_uuid],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to load photo {}: {}", photo_uuid, e))
}

#[cfg(feature = "components")]
/// Helper function to get preview photo path for a collection
pub fn get_collection_preview_path(
    conn: &Connection,
    collection_id: &str,
) -> Result<Option<String>, String> {
    // Get preview photo UUID from collection
    let preview_uuid: Option<String> = conn
        .query_row(
            "SELECT preview_photo_uuid FROM photo_collections WHERE uuid = ?1",
            [collection_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to load collection {}: {}", collection_id, e))?;

    if let Some(uuid) = preview_uuid {
        get_photo_path(conn, &uuid).map(Some)
    } else {
        // No preview set, try to get first photo in collection
        let first_photo: Option<String> = conn
            .query_row(
                "SELECT COALESCE(relative_path, path) FROM photos 
                 WHERE collection_id = ?1 AND deleted = 0 
                 ORDER BY created_at ASC LIMIT 1",
                [collection_id],
                |row| row.get(0),
            )
            .ok();
        Ok(first_photo)
    }
}

#[cfg(feature = "components")]
/// Helper function to get all photo paths in a collection
pub fn get_collection_photos(
    conn: &Connection,
    collection_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(relative_path, path) FROM photos 
             WHERE collection_id = ?1 AND deleted = 0 
             ORDER BY created_at ASC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let paths = stmt
        .query_map([collection_id], |row| row.get(0))
        .map_err(|e| format!("Failed to query photos: {}", e))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Failed to collect photos: {}", e))?;

    Ok(paths)
}

#[cfg(feature = "components")]
/// Thumbnail image component - displays a small photo
///
/// Can be used in two ways:
/// 1. With pre-loaded data_url (backward compatible)
/// 2. With relative_path (loads from storage)
#[component]
pub fn ThumbnailImage(
    #[props(default = None)] data_url: Option<String>,
    #[props(default = None)] relative_path: Option<String>,
    #[props(default = "Photo".to_string())] alt: String,
) -> Element {
    let mut image_state = use_signal(|| ImageLoadState::Loading);
    let context = use_context::<PhotoGalleryContext>();

    // Load photo data
    use_effect(move || {
        if let Some(path) = relative_path.clone() {
            // Load from storage using context
            if let Some(data) = context.load_photo_data(&path, PhotoSize::Small) {
                image_state.set(ImageLoadState::Loaded(data));
            } else {
                image_state.set(ImageLoadState::Failed);
            }
        } else if let Some(url) = data_url.clone() {
            // Use pre-loaded data URL
            image_state.set(ImageLoadState::Loaded(url));
        } else {
            image_state.set(ImageLoadState::Failed);
        }
    });

    rsx! {
        div {
            style: "width: 128px; height: 128px; border-radius: 8px; overflow: hidden; background: #f0f0f0;",
            match image_state() {
                ImageLoadState::Loading => rsx! {
                    div {
                        style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: #999;",
                        "⏳"
                    }
                },
                ImageLoadState::Loaded(url) => rsx! {
                    img {
                        src: "{url}",
                        alt: "{alt}",
                        style: "width: 100%; height: 100%; object-fit: cover;",
                    }
                },
                ImageLoadState::Failed => rsx! {
                    div {
                        style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; color: #999;",
                        "📷"
                    }
                },
            }
        }
    }
}

#[cfg(feature = "components")]
/// Preview image component - displays a medium-sized photo
///
/// Can be used with data_url or relative_path
#[component]
pub fn PreviewImage(
    #[props(default = None)] data_url: Option<String>,
    #[props(default = None)] relative_path: Option<String>,
    #[props(default = "Photo".to_string())] alt: String,
) -> Element {
    let mut image_state = use_signal(|| ImageLoadState::Loading);
    let context = use_context::<PhotoGalleryContext>();

    use_effect(move || {
        if let Some(path) = relative_path.clone() {
            if let Some(data) = context.load_photo_data(&path, PhotoSize::Medium) {
                image_state.set(ImageLoadState::Loaded(data));
            } else {
                image_state.set(ImageLoadState::Failed);
            }
        } else if let Some(url) = data_url.clone() {
            image_state.set(ImageLoadState::Loaded(url));
        } else {
            image_state.set(ImageLoadState::Failed);
        }
    });

    rsx! {
        div {
            style: "max-width: 512px; max-height: 512px; border-radius: 8px; overflow: hidden; background: #f0f0f0;",
            match image_state() {
                ImageLoadState::Loading => rsx! {
                    div {
                        style: "width: 100%; height: 400px; display: flex; align-items: center; justify-content: center; color: #999;",
                        "⏳"
                    }
                },
                ImageLoadState::Loaded(url) => rsx! {
                    img {
                        src: "{url}",
                        alt: "{alt}",
                        style: "max-width: 100%; max-height: 100%; object-fit: contain;",
                    }
                },
                ImageLoadState::Failed => rsx! {
                    div {
                        style: "width: 100%; height: 400px; display: flex; align-items: center; justify-content: center; color: #999;",
                        "📷"
                    }
                },
            }
        }
    }
}

#[cfg(feature = "components")]
/// Fullscreen image component - displays a single photo in fullscreen with close button
///
/// Can be used with data_url or relative_path
#[component]
pub fn FullscreenImage(
    #[props(default = None)] data_url: Option<String>,
    #[props(default = None)] relative_path: Option<String>,
    on_close: EventHandler<()>,
) -> Element {
    let mut image_state = use_signal(|| ImageLoadState::Loading);
    let context = use_context::<PhotoGalleryContext>();

    use_effect(move || {
        if let Some(path) = relative_path.clone() {
            if let Some(data) = context.load_photo_data(&path, PhotoSize::Original) {
                image_state.set(ImageLoadState::Loaded(data));
            } else {
                image_state.set(ImageLoadState::Failed);
            }
        } else if let Some(url) = data_url.clone() {
            image_state.set(ImageLoadState::Loaded(url));
        } else {
            image_state.set(ImageLoadState::Failed);
        }
    });

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.95); z-index: 1000; display: flex; flex-direction: column;",
            div {
                style: "display: flex; justify-content: flex-end; padding: 16px; background: rgba(0, 0, 0, 0.7);",
                button {
                    style: "width: 40px; height: 40px; background: rgba(255, 255, 255, 0.2); color: white; border-radius: 50%; font-size: 24px; cursor: pointer; border: none;",
                    onclick: move |_| on_close.call(()),
                    "×"
                }
            }
            div {
                style: "flex: 1; display: flex; align-items: center; justify-content: center; padding: 20px;",
                match image_state() {
                    ImageLoadState::Loading => rsx! {
                        div {
                            style: "color: white; font-size: 48px;",
                            "⏳"
                        }
                    },
                    ImageLoadState::Loaded(url) => rsx! {
                        img {
                            src: "{url}",
                            style: "max-width: 100%; max-height: 100%; object-fit: contain;",
                        }
                    },
                    ImageLoadState::Failed => rsx! {
                        div {
                            style: "color: white; font-size: 48px;",
                            "📷"
                        }
                    },
                }
            }
        }
    }
}

#[cfg(feature = "components")]
/// Thumbnail collection component
///
/// Can be used with preview_data_url, preview_relative_path, or neither (shows placeholder)
#[component]
pub fn ThumbnailCollection(
    #[props(default = None)] preview_data_url: Option<String>,
    #[props(default = None)] preview_relative_path: Option<String>,
    #[props(default = None)] on_click: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div {
            style: "width: 128px; height: 128px; cursor: pointer;",
            onclick: move |_| {
                if let Some(handler) = &on_click {
                    handler.call(());
                }
            },
            if preview_data_url.is_some() || preview_relative_path.is_some() {
                ThumbnailImage {
                    data_url: preview_data_url.clone(),
                    relative_path: preview_relative_path.clone(),
                    alt: "Collection preview".to_string(),
                }
            } else {
                div {
                    style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: #f0f0f0; border-radius: 8px; color: #999;",
                    "📷"
                }
            }
        }
    }
}

#[cfg(feature = "components")]
/// Preview collection component
///
/// Can be used with preview_data_url, preview_relative_path, or neither (shows placeholder)
#[component]
pub fn PreviewCollection(
    #[props(default = None)] preview_data_url: Option<String>,
    #[props(default = None)] preview_relative_path: Option<String>,
    #[props(default = None)] on_click: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div {
            style: "max-width: 512px; cursor: pointer;",
            onclick: move |_| {
                if let Some(handler) = &on_click {
                    handler.call(());
                }
            },
            if preview_data_url.is_some() || preview_relative_path.is_some() {
                PreviewImage {
                    data_url: preview_data_url.clone(),
                    relative_path: preview_relative_path.clone(),
                    alt: "Collection preview".to_string(),
                }
            } else {
                div {
                    style: "width: 100%; height: 400px; display: flex; align-items: center; justify-content: center; background: #f0f0f0; border-radius: 8px; color: #999;",
                    "📷"
                }
            }
        }
    }
}

#[cfg(feature = "components")]
/// Fullscreen collection viewer
///
/// Can be used with photo_data_urls or photo_relative_paths
#[component]
pub fn CollectionFullscreen(
    #[props(default = vec![])] photo_data_urls: Vec<String>,
    #[props(default = vec![])] photo_relative_paths: Vec<String>,
    #[props(default = 0)] initial_index: usize,
    on_close: EventHandler<()>,
) -> Element {
    let mut current_index = use_signal(|| initial_index);
    let mut loaded_urls = use_signal(|| vec![]);
    let context = use_context::<PhotoGalleryContext>();

    // Load photos from relative paths if provided
    use_effect(move || {
        if !photo_data_urls.is_empty() {
            loaded_urls.set(photo_data_urls.clone());
        } else if !photo_relative_paths.is_empty() {
            let urls: Vec<String> = photo_relative_paths
                .iter()
                .filter_map(|path| context.load_photo_data(path, PhotoSize::Original))
                .collect();
            loaded_urls.set(urls);
        }
    });

    let photo_count = loaded_urls.read().len();
    let has_prev = current_index() > 0;
    let has_next = current_index() < photo_count.saturating_sub(1);

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.95); z-index: 1000; display: flex; flex-direction: column;",
            div {
                style: "display: flex; justify-content: space-between; align-items: center; padding: 16px; background: rgba(0, 0, 0, 0.7);",
                div {
                    style: "color: white; font-size: 16px;",
                    "{current_index() + 1} / {photo_count}"
                }
                button {
                    style: "width: 40px; height: 40px; background: rgba(255, 255, 255, 0.2); color: white; border-radius: 50%; font-size: 24px; cursor: pointer; border: none;",
                    onclick: move |_| on_close.call(()),
                    "×"
                }
            }
            div {
                style: "flex: 1; display: flex; align-items: center; justify-content: center; padding: 20px; position: relative;",
                if has_prev {
                    button {
                        style: "position: absolute; left: 20px; width: 50px; height: 50px; background: rgba(255, 255, 255, 0.3); color: white; border-radius: 50%; font-size: 24px; cursor: pointer; border: none;",
                        onclick: move |_| {
                            let idx = current_index();
                            if idx > 0 {
                                current_index.set(idx - 1);
                            }
                        },
                        "‹"
                    }
                }
                if photo_count > 0 {
                    img {
                        src: "{loaded_urls.read()[current_index()]}",
                        style: "max-width: 100%; max-height: 100%; object-fit: contain;",
                    }
                } else {
                    div {
                        style: "color: white; font-size: 24px;",
                        "No photos in collection"
                    }
                }
                if has_next {
                    button {
                        style: "position: absolute; right: 20px; width: 50px; height: 50px; background: rgba(255, 255, 255, 0.3); color: white; border-radius: 50%; font-size: 24px; cursor: pointer; border: none;",
                        onclick: move |_| {
                            let idx = current_index();
                            if idx < photo_count - 1 {
                                current_index.set(idx + 1);
                            }
                        },
                        "›"
                    }
                }
            }
        }
    }
}
