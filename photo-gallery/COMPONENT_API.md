# Photo Gallery Components API

The photo-gallery crate provides Dioxus UI components for displaying photos and collections. Components support both pre-loaded data URLs (backward compatible) and automatic loading from storage via context.

## Setup

Enable the `components` feature in your `Cargo.toml`:

```toml
[dependencies]
photo-gallery = { path = "../photo-gallery", features = ["components"] }
```

### Context Provider

Components use `PhotoGalleryContext` to load photos from storage. Provide the context at your app root:

```rust
use dioxus::prelude::*;
use photo_gallery::PhotoGalleryContext;

fn App() -> Element {
    // Provide storage path via context
    use_context_provider(|| PhotoGalleryContext::new("/path/to/photos".to_string()));
    
    rsx! {
        MyComponent {}
    }
}
```

## Components

### ThumbnailImage

Displays a small thumbnail (128px × 128px).

**Props:**
- `data_url: Option<String>` - Pre-loaded data URL (backward compatible)
- `relative_path: Option<String>` - Photo path relative to storage (loads automatically)
- `alt: String` - Alt text (default: "Photo")

**Example:**
```rust
use photo_gallery::{ThumbnailImage, get_photo_path};

#[component]
fn PhotoThumbnail(photo_uuid: String) -> Element {
    let conn = /* database connection */;
    let relative_path = get_photo_path(&conn, &photo_uuid).ok();
    
    rsx! {
        ThumbnailImage {
            relative_path: relative_path,
            alt: "Profile photo".to_string(),
        }
    }
}
```

### PreviewImage

Displays a medium-sized preview (max 512px).

**Props:**
- `data_url: Option<String>` - Pre-loaded data URL
- `relative_path: Option<String>` - Photo path (loads automatically)
- `alt: String` - Alt text (default: "Photo")

**Example:**
```rust
PreviewImage {
    relative_path: Some("photos/abc123.jpg".to_string()),
    alt: "Event photo".to_string(),
}
```

### FullscreenImage

Displays a photo in fullscreen with a close button.

**Props:**
- `data_url: Option<String>` - Pre-loaded data URL
- `relative_path: Option<String>` - Photo path (loads automatically)
- `on_close: EventHandler<()>` - Close button callback

**Example:**
```rust
let mut show_fullscreen = use_signal(|| false);

rsx! {
    button {
        onclick: move |_| show_fullscreen.set(true),
        "View Photo"
    }
    
    if show_fullscreen() {
        FullscreenImage {
            relative_path: Some("photos/abc123.jpg".to_string()),
            on_close: move |_| show_fullscreen.set(false),
        }
    }
}
```

### ThumbnailCollection

Displays a collection preview thumbnail that opens fullscreen viewer on click.

**Props:**
- `preview_data_url: Option<String>` - Pre-loaded preview data URL
- `preview_relative_path: Option<String>` - Preview photo path (loads automatically)
- `on_click: Option<EventHandler<()>>` - Click handler

**Example:**
```rust
use photo_gallery::{ThumbnailCollection, get_collection_preview_path};

let preview_path = get_collection_preview_path(&conn, &collection_id).ok().flatten();

rsx! {
    ThumbnailCollection {
        preview_relative_path: preview_path,
        on_click: move |_| open_collection(),
    }
}
```

### PreviewCollection

Medium-sized collection preview.

**Props:**
- `preview_data_url: Option<String>` - Pre-loaded preview data URL
- `preview_relative_path: Option<String>` - Preview photo path
- `on_click: Option<EventHandler<()>>` - Click handler

### CollectionFullscreen

Fullscreen collection viewer with navigation (prev/next).

**Props:**
- `photo_data_urls: Vec<String>` - Pre-loaded data URLs (default: empty)
- `photo_relative_paths: Vec<String>` - Photo paths (loads automatically, default: empty)
- `initial_index: usize` - Starting photo index (default: 0)
- `on_close: EventHandler<()>` - Close button callback

**Example:**
```rust
use photo_gallery::{CollectionFullscreen, get_collection_photos};

let photo_paths = get_collection_photos(&conn, &collection_id).ok().unwrap_or_default();

rsx! {
    CollectionFullscreen {
        photo_relative_paths: photo_paths,
        initial_index: 0,
        on_close: move |_| close_viewer(),
    }
}
```

## Helper Functions

### get_photo_path

Query photo path from database by UUID.

```rust
pub fn get_photo_path(conn: &Connection, photo_uuid: &str) -> Result<String, String>
```

**Example:**
```rust
let path = photo_gallery::get_photo_path(&conn, "photo-uuid")?;
```

### get_collection_preview_path

Get preview photo path for a collection. Returns the collection's preview_photo_uuid if set, otherwise the first photo in the collection.

```rust
pub fn get_collection_preview_path(conn: &Connection, collection_id: &str) -> Result<Option<String>, String>
```

**Example:**
```rust
let preview = photo_gallery::get_collection_preview_path(&conn, "collection-uuid")?;
```

### get_collection_photos

Get all photo paths in a collection, ordered by creation date.

```rust
pub fn get_collection_photos(conn: &Connection, collection_id: &str) -> Result<Vec<String>, String>
```

**Example:**
```rust
let photos = photo_gallery::get_collection_photos(&conn, "collection-uuid")?;
```

## Migration from Data URL Approach

If you're currently passing data URLs to components, you can migrate gradually:

### Before:
```rust
// Load and encode photo manually
let photo_bytes = std::fs::read(&photo_path)?;
let data_url = format!("data:image/jpeg;base64,{}", base64::encode(&photo_bytes));

rsx! {
    ThumbnailImage {
        data_url: data_url,
    }
}
```

### After:
```rust
// Just pass the relative path
let relative_path = get_photo_path(&conn, &photo_uuid)?;

rsx! {
    ThumbnailImage {
        relative_path: Some(relative_path),
    }
}
```

Components handle loading and encoding automatically using the `PhotoGalleryContext`.

## Notes

- Components show a loading indicator (⏳) while loading photos
- Failed loads show a placeholder (📷)
- Thumbnails use WebP format when available, falling back to original
- Context must be provided at app root for automatic loading to work
- Components remain backward compatible with data URLs for gradual migration
