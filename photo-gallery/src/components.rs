//! Dioxus UI components for photo gallery
//!
//! This module provides reusable photo display components.
//! Components accept image data URLs for better separation of concerns.

#[cfg(feature = "components")]
use dioxus::prelude::*;

#[cfg(feature = "components")]
/// Thumbnail image component - displays a small photo
#[component]
pub fn ThumbnailImage(
    data_url: String,
    #[props(default = "Photo".to_string())]
    alt: String,
) -> Element {
    rsx! {
        div {
            style: "width: 128px; height: 128px; border-radius: 8px; overflow: hidden; background: #f0f0f0;",
            img {
                src: "{data_url}",
                alt: "{alt}",
                style: "width: 100%; height: 100%; object-fit: cover;",
            }
        }
    }
}

#[cfg(feature = "components")]
/// Preview image component - displays a medium-sized photo
#[component]
pub fn PreviewImage(
    data_url: String,
    #[props(default = "Photo".to_string())]
    alt: String,
) -> Element {
    rsx! {
        div {
            style: "max-width: 512px; max-height: 512px; border-radius: 8px; overflow: hidden; background: #f0f0f0;",
            img {
                src: "{data_url}",
                alt: "{alt}",
                style: "max-width: 100%; max-height: 100%; object-fit: contain;",
            }
        }
    }
}

#[cfg(feature = "components")]
/// Fullscreen image component - displays a single photo in fullscreen with close button
#[component]
pub fn FullscreenImage(
    data_url: String,
    on_close: EventHandler<()>,
) -> Element {
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
                img {
                    src: "{data_url}",
                    style: "max-width: 100%; max-height: 100%; object-fit: contain;",
                }
            }
        }
    }
}

#[cfg(feature = "components")]
/// Thumbnail collection component
#[component]
pub fn ThumbnailCollection(
    preview_data_url: Option<String>,
    on_click: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div {
            style: "width: 128px; height: 128px; cursor: pointer;",
            onclick: move |_| {
                if let Some(handler) = &on_click {
                    handler.call(());
                }
            },
            if let Some(url) = preview_data_url {
                ThumbnailImage {
                    data_url: url,
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
#[component]
pub fn PreviewCollection(
    preview_data_url: Option<String>,
    on_click: Option<EventHandler<()>>,
) -> Element {
    rsx! {
        div {
            style: "max-width: 512px; cursor: pointer;",
            onclick: move |_| {
                if let Some(handler) = &on_click {
                    handler.call(());
                }
            },
            if let Some(url) = preview_data_url {
                PreviewImage {
                    data_url: url,
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
#[component]
pub fn CollectionFullscreen(
    photo_data_urls: Vec<String>,
    #[props(default = 0)]
    initial_index: usize,
    on_close: EventHandler<()>,
) -> Element {
    let mut current_index = use_signal(|| initial_index);

    let has_prev = current_index() > 0;
    let has_next = current_index() < photo_data_urls.len().saturating_sub(1);

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.95); z-index: 1000; display: flex; flex-direction: column;",
            div {
                style: "display: flex; justify-content: space-between; align-items: center; padding: 16px; background: rgba(0, 0, 0, 0.7);",
                div {
                    style: "color: white; font-size: 16px;",
                    "{current_index() + 1} / {photo_data_urls.len()}"
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
                if !photo_data_urls.is_empty() {
                    img {
                        src: "{photo_data_urls[current_index()]}",
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
                            if idx < photo_data_urls.len() - 1 {
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
