use crate::database;
use crate::image_processing;
use crate::models::{Quail, QuailEvent};
use crate::services::{event_service, profile_service};
use crate::Screen;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn ProfileDetailScreen(quail_id: String, on_navigate: EventHandler<Screen>) -> Element {
    let mut profile = use_signal(|| None::<Quail>);
    let mut events = use_signal(|| Vec::<QuailEvent>::new());
    let mut error = use_signal(|| String::new());
    let mut photos = use_signal(|| Vec::<crate::models::Photo>::new());
    let mut current_photo_index = use_signal(|| 0usize);
    let mut show_fullscreen = use_signal(|| false);
    let mut uploading = use_signal(|| false);
    let mut upload_error = use_signal(|| String::new());

    #[cfg(target_os = "android")]
    let quail_id_for_gallery = quail_id.clone();
    #[cfg(target_os = "android")]
    let quail_id_for_camera = quail_id.clone();

    // Retry failed downloads beim Mount
    use_effect(move || {
        spawn(async move {
            if let Ok(conn) = database::init_database() {
                if let Err(e) = crate::services::photo_service::retry_failed_downloads(&conn).await
                {
                    log::warn!("Failed to retry photo downloads: {}", e);
                }
            }
        });
    });

    // Alle Bilder der Wachtel laden (using collection-based API)
    let quail_id_for_photos = quail_id.clone();
    use_effect(move || {
        if let Ok(conn) = database::init_database() {
            if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_for_photos) {
                // Try collection-based API first, fall back to old API
                let photo_list =
                    match crate::services::photo_service::get_quail_collection(&conn, &uuid) {
                        Ok(Some(collection_id)) => {
                            crate::services::photo_service::list_collection_photos(
                                &conn,
                                &collection_id,
                            )
                            .ok()
                        }
                        _ => None,
                    }
                    .or_else(|| {
                        crate::services::photo_service::list_quail_photos(&conn, &uuid).ok()
                    });

                if let Some(list) = photo_list {
                    photos.set(list);
                    // Reset photo index if it's out of bounds
                    if current_photo_index() >= list.len() && !list.is_empty() {
                        current_photo_index.set(list.len() - 1);
                    } else if list.is_empty() {
                        current_photo_index.set(0);
                    }
                }
            }
        }
    });

    // Profil und Events laden
    let quail_id_for_profile = quail_id.clone();
    use_effect(move || {
        if let Ok(conn) = database::init_database() {
            if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_for_profile) {
                match profile_service::get_profile(&conn, &uuid) {
                    Ok(p) => {
                        profile.set(Some(p));
                        error.set(String::new());
                    }
                    Err(e) => error.set(t!("error-load-failed", error: e.to_string())), // Failed to load
                }

                // Load events
                match event_service::get_events_for_quail(&conn, &uuid) {
                    Ok(evts) => events.set(evts),
                    Err(e) => log::error!("{}: {}", t!("error-load-events-failed"), e), // Failed to load events
                }
            }
        }
    });

    rsx! {
        div { style: "padding: 16px; max-width: 800px; margin: 0 auto;",
            // Header
            div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 24px;",
                button {
                    style: "padding: 8px 16px; background: #e0e0e0; color: #333; border-radius: 8px; font-size: 16px;",
                    onclick: move |_| on_navigate.call(Screen::ProfileList),
                    "← "
                    {t!("action-back")}
                }
                h1 { style: "margin: 0; font-size: 26px; color: #0066cc; font-weight: 700;",
                    {t!("profile-detail-title")} // Profile
                }
            }

            if !error().is_empty() {
                div { style: "background: #fee; border: 1px solid #fcc; color: #c33; padding: 12px; margin-bottom: 16px; border-radius: 8px; font-size: 14px;",
                    "⚠️ "
                    {error}
                }
            }

            if let Some(p) = profile() {
                div { style: "display: flex; flex-direction: column; gap: 24px;",
                    // Bild mit Plus-Button - zeigt Profilfoto, klickbar für Vollbild-Galerie
                    div { style: "width: 100%; aspect-ratio: 1/1; background: #f0f0f0; border-radius: 12px; overflow: hidden; display: flex; align-items: center; justify-content: center; position: relative;",
                        // Hauptbild (klickbar für Galerie)
                        div {
                            style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; cursor: pointer;",
                            onclick: move |_| {
                                if !photos().is_empty() {
                                    current_photo_index.set(0);
                                    show_fullscreen.set(true);
                                }
                            },
                            {
                                let profile_photo_opt = if let Some(ref profile_photo_uuid) = p.profile_photo {
                                    photos().iter().find(|ph| &ph.uuid == profile_photo_uuid).cloned()
                                } else {
                                    None
                                };
                                if let Some(profile_photo) = profile_photo_opt {
                                    let path_to_use = profile_photo
                                        .thumbnail_path
                                        .clone()
                                        .unwrap_or(profile_photo.path.clone());
                                    match image_processing::image_path_to_data_url(&path_to_use) {
                                        Ok(data_url) => rsx! {
                                            img {
                                                src: data_url,
                                                alt: p.name.clone(),
                                                style: "width:100%; height:100%; object-fit: cover;",
                                            }
                                            if photos().len() > 1 {
                                                div { style: "position:absolute; bottom:8px; right:8px; background:rgba(0,0,0,0.7); color:white; padding:6px 12px; border-radius:16px; font-size:12px;",
                                                    "📷 {photos().len()}"
                                                }
                                            }
                                        },
                                        Err(_) => rsx! {
                                            div { style: "font-size: 48px; color:#999;", "🐦" }
                                        },
                                    }
                                } else if !photos().is_empty() {
                                    let first_photo = &photos()[0];
                                    let path_to_use = first_photo
                                        .thumbnail_path
                                        .clone()
                                        .unwrap_or(first_photo.path.clone());
                                    match image_processing::image_path_to_data_url(&path_to_use) {
                                        Ok(data_url) => rsx! {
                                            img {
                                                src: data_url,
                                                alt: p.name.clone(),
                                                style: "width:100%; height:100%; object-fit: cover;",
                                            }
                                            if photos().len() > 1 {
                                                div { style: "position:absolute; bottom:8px; right:8px; background:rgba(0,0,0,0.7); color:white; padding:6px 12px; border-radius:16px; font-size:12px;",
                                                    "📷 {photos().len()}"
                                                }
                                            }
                                        },
                                        Err(_) => rsx! {
                                            div { style: "font-size: 48px; color:#999;", "🐦" }
                                        },
                                    }
                                } else {
                                    rsx! {
                                        div { style: "font-size: 48px; color:#999;", "🐦" }
                                    }
                                }
                            }
                        }
                        // Zwei halbtransparente Overlay-Buttons (Galerie Mehrfach / Kamera Einzel)
                        // Galerie (Mehrfachauswahl)
                        button {
                            style: "position:absolute; bottom:12px; left:12px; padding:10px 14px; background:rgba(0,0,0,0.45); color:white; backdrop-filter:blur(4px); border-radius:8px; font-size:14px; display:flex; align-items:center; gap:6px; cursor:pointer; z-index:11;",
                            disabled: uploading(),
                            onclick: {
                                move |e| {
                                    e.stop_propagation();
                                    uploading.set(true);
                                    upload_error.set(String::new());
                                    #[cfg(target_os = "android")]
                                    let quail_id_clone = quail_id_for_gallery.clone();
                                spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::pick_images() {
                                            Ok(paths) => {
                                                if let Ok(conn) = database::init_database() {
                                                    if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_clone) {
                                                        // Get or create collection for this quail
                                                        match crate::services::photo_service::get_or_create_quail_collection(&conn, &uuid) {
                                                            Ok(collection_id) => {
                                                                for pth in paths {
                                                                    let path_str = pth.to_string_lossy().to_string();
                                                                    match crate::services::photo_service::add_photo_to_collection(
                                                                        &conn,
                                                                        &collection_id,
                                                                        path_str,
                                                                    ).await {
                                                                        Ok(_) => {}
                                                                        Err(e) => {
                                                                            upload_error.set(format!("Fehler beim Speichern: {}", e));
                                                                            break;
                                                                        }
                                                                    }
                                                                }
                                                                // Reload photos from collection
                                                                if let Ok(photo_list) = crate::services::photo_service::list_collection_photos(
                                                                    &conn,
                                                                    &collection_id,
                                                                ) {
                                                                    photos.set(photo_list);
                                                                }
                                                            }
                                                            Err(e) => {
                                                                upload_error.set(format!("Fehler beim Erstellen der Sammlung: {}", e));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                upload_error
                                                    .set(format!("{}: {}", t!("error-selection-failed"), e))
                                            }
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        upload_error.set(t!("error-multiselect-android-only"));
                                    }
                                    uploading.set(false);
                                });
                                }
                            },
                            if uploading() {
                                "⏳"
                            } else {
                                "🖼️ "
                                {t!("action-gallery")}
                            }
                        }
                        // Kamera (Einzelfoto)
                        button {
                            style: "position:absolute; bottom:12px; right:12px; padding:10px 14px; background:rgba(0,0,0,0.45); color:white; backdrop-filter:blur(4px); border-radius:8px; font-size:14px; display:flex; align-items:center; gap:6px; cursor:pointer; z-index:11;",
                            disabled: uploading(),
                            onclick: {
                                move |e| {
                                    e.stop_propagation();
                                    uploading.set(true);
                                    upload_error.set(String::new());
                                    #[cfg(target_os = "android")]
                                    let quail_id_clone = quail_id_for_camera.clone();
                                    spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::capture_photo() {
                                            Ok(path) => {
                                                if let Ok(conn) = database::init_database() {
                                                    let path_str = path.to_string_lossy().to_string();
                                                    if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_clone) {
                                                        // Get or create collection for this quail
                                                        match crate::services::photo_service::get_or_create_quail_collection(&conn, &uuid) {
                                                            Ok(collection_id) => {
                                                                match crate::services::photo_service::add_photo_to_collection(
                                                                    &conn,
                                                                    &collection_id,
                                                                    path_str,
                                                                ).await {
                                                                    Ok(_) => {
                                                                        if let Ok(photo_list) = crate::services::photo_service::list_collection_photos(
                                                                            &conn,
                                                                            &collection_id,
                                                                        ) {
                                                                            photos.set(photo_list);
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        upload_error
                                                                            .set(format!("{}: {}", t!("error-save-failed"), e))
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                upload_error
                                                                    .set(format!("{}: {}", t!("error-collection-failed"), e))
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                upload_error
                                                    .set(format!("{}: {}", t!("error-capture-failed"), e))
                                            }
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        upload_error.set(t!("error-camera-android-only"));
                                    }
                                    uploading.set(false);
                                });
                                }
                            },
                            if uploading() {
                                "⏳"
                            } else {
                                "📷 "
                                {t!("action-photo")}
                            }
                        }
                    }

                    // Upload Error anzeigen falls vorhanden
                    if !upload_error().is_empty() {
                        div { style: "padding: 12px; background: #ffe6e6; border-radius: 8px; color: #cc0000; font-size: 14px; margin-top: 12px;",
                            "⚠️ "
                            {upload_error}
                        }
                    }

                    // Basisinfos
                    div { style: "display: flex; flex-direction: column; gap: 12px;",
                        h2 { style: "margin:0; font-size: 28px; color:#333; font-weight:600;",
                            "{p.name}"
                        }
                        div { style: "display:flex; flex-wrap:wrap; gap:8px;",
                            span { style: "padding:6px 14px; background:#e8f4f8; border-radius:16px; font-size:13px; color:#0066cc;",
                                "ID {p.uuid.to_string().chars().take(8).collect::<String>()}"
                            }
                            span { style: "padding:6px 14px; background:#fff3e0; border-radius:16px; font-size:13px; color:#ff8c00;",
                                "{p.gender.display_name()}"
                            }
                            // Status Badge basierend auf letztem Event
                            if let Some(latest_event) = events().first() {
                                match latest_event.event_type {
                                    crate::models::EventType::Born => rsx! {
                                        span { style: "padding:6px 14px; background:#e0ffe6; border-radius:16px; font-size:13px; color:#228833;",
                                            "🐣 "
                                            {t!("status-born")}
                                        }
                                    },
                                    crate::models::EventType::Alive => rsx! {
                                        span { style: "padding:6px 14px; background:#e0ffe6; border-radius:16px; font-size:13px; color:#228833;",
                                            "✅ "
                                            {t!("status-alive")}
                                        }
                                    },
                                    crate::models::EventType::Sick => rsx! {
                                        span { style: "padding:6px 14px; background:#ffe0e0; border-radius:16px; font-size:13px; color:#cc3333;",
                                            "🤒 "
                                            {t!("status-sick")}
                                        }
                                    },
                                    crate::models::EventType::Healthy => rsx! {
                                        span { style: "padding:6px 14px; background:#e0ffe6; border-radius:16px; font-size:13px; color:#228833;",
                                            "💪 "
                                            {t!("status-healthy")}
                                        }
                                    },
                                    crate::models::EventType::MarkedForSlaughter => {
                                        rsx! {
                                            span { style: "padding:6px 14px; background:#fff3e0; border-radius:16px; font-size:13px; color:#ff8800;",
                                                "🥩 "
                                                {t!("status-marked")}
                                            }
                                        }
                                    }
                                    crate::models::EventType::Slaughtered => rsx! {
                                        span { style: "padding:6px 14px; background:#f0f0f0; border-radius:16px; font-size:13px; color:#666;",
                                            "🥩 "
                                            {t!("status-slaughtered")}
                                        }
                                    },
                                    crate::models::EventType::Died => rsx! {
                                        span { style: "padding:6px 14px; background:#f0f0f0; border-radius:16px; font-size:13px; color:#666;",
                                            "🪦 "
                                            {t!("status-died")}
                                        }
                                    },
                                }
                            }
                        }
                    }
                    // Detail Grid
                    div { style: "display:grid; gap:16px;",
                        div { style: "padding:14px; background:#f5f5f5; border-radius:8px;",
                            div { style: "font-size:11px; color:#666; font-weight:600; margin-bottom:4px;",
                                "UUID"
                            }
                            div { style: "font-size:11px; color:#999; word-break:break-all; font-family:monospace;",
                                "{p.uuid}"
                            }
                        }
                    }

                    // Events Timeline
                    div { style: "margin-top:24px;",
                        div { style: "display:flex; justify-content:space-between; align-items:center; margin-bottom:12px;",
                            h3 { style: "margin:0; font-size:18px; color:#333; font-weight:600;",
                                "📅 "
                                {t!("events-timeline-title")}
                            }
                            button {
                                style: "padding:8px 16px; background:#0066cc; color:white; border-radius:8px; font-size:14px; font-weight:500;",
                                onclick: move |_| {
                                    if let Some(p) = profile() {
                                        on_navigate
                                            .call(Screen::EventAdd {
                                                quail_id: p.uuid.to_string(),
                                                quail_name: p.name.clone(),
                                            });
                                    }
                                },
                                "+ "
                                {t!("action-add-event")} // Add event
                            }
                        }

                        if events().is_empty() {
                            div { style: "padding:24px; text-align:center; background:#f5f5f5; border-radius:8px; color:#999;",
                                {t!("events-empty")} // No events available
                            }
                        } else {
                            div { style: "display:flex; flex-direction:column; gap:12px;",
                                for event in events() {
                                    div {
                                        key: "{event.uuid}",
                                        style: "padding:14px; background:white; border:1px solid #e0e0e0; border-radius:8px; cursor:pointer;",
                                        onclick: {
                                            let quail_id_for_event = quail_id.clone();
                                            move |_| {
                                                on_navigate
                                                    .call(Screen::EventEdit {
                                                        event_id: event.uuid.to_string(),
                                                        quail_id: quail_id_for_event.clone(),
                                                    });
                                            }
                                        },
                                        div { style: "display:flex; gap:10px; align-items:center; margin-bottom:8px;",
                                            span { style: "font-size:20px;",
                                                match event.event_type {
                                                    crate::models::EventType::Born => "🐣",
                                                    crate::models::EventType::Alive => "✅",
                                                    crate::models::EventType::Sick => "🤒",
                                                    crate::models::EventType::Healthy => "💪",
                                                    crate::models::EventType::MarkedForSlaughter => "🥩",
                                                    crate::models::EventType::Slaughtered => "🥩",
                                                    crate::models::EventType::Died => "🪦",
                                                }
                                            }
                                            div {
                                                div { style: "font-size:14px; font-weight:600; color:#333;",
                                                    "{event.event_type.display_name()}"
                                                }
                                                div { style: "font-size:12px; color:#666;",
                                                    {event.event_date.format("%d.%m.%Y").to_string()}
                                                }
                                            }
                                        }
                                        if let Some(notes) = &event.notes {
                                            div { style: "font-size:13px; color:#555; line-height:1.4; white-space:pre-wrap;",
                                                "{notes}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Bearbeiten Button
                    button {
                        class: "btn-primary",
                        style: "width:100%; padding:14px; font-size:16px; font-weight:600; margin-top:24px;",
                        onclick: {
                            let quail_id_for_edit = quail_id.clone();
                            move |_| on_navigate.call(Screen::ProfileEdit(quail_id_for_edit.clone()))
                        },
                        "✏️ "
                        {t!("action-edit")}
                    }
                }
            } else {
                div { style: "padding:48px; text-align:center;",
                    div { style: "font-size:48px; margin-bottom:16px;", "⏳" }
                    div { style: "color:#666;", {t!("loading-profile")} } // Loading profile...
                }
            }

            // Vollbild-Galerie Overlay
            if show_fullscreen() && !photos().is_empty() {
                div {
                    style: "position:fixed; top:0; left:0; right:0; bottom:0; background:rgba(0,0,0,0.95); z-index:9999; display:flex; flex-direction:column;",
                    onclick: move |_| show_fullscreen.set(false),
                    // Header
                    div {
                        style: "padding:16px; display:flex; justify-content:space-between; align-items:center;",
                        onclick: move |e| e.stop_propagation(),
                        div { style: "color:white; font-size:18px; font-weight:600;",
                            "{current_photo_index() + 1} / {photos().len()}"
                        }
                        button {
                            style: "background:rgba(255,255,255,0.2); color:white; padding:8px 16px; border-radius:8px; font-size:16px;",
                            onclick: move |_| show_fullscreen.set(false),
                            "✕ "
                            {t!("action-close")}
                        }
                    }
                    // Hauptbild
                    div {
                        style: "flex:1; display:flex; align-items:center; justify-content:center; padding:16px;",
                        onclick: move |e| e.stop_propagation(),
                        {
                            // Bounds check to prevent crash
                            let idx = current_photo_index();
                            let photo_vec = photos();
                            if idx < photo_vec.len() {
                                let current_photo = &photo_vec[idx];
                                let full_path = current_photo.path.clone();
                                match image_processing::image_path_to_data_url(&full_path) {
                                    Ok(data_url) => rsx! {
                                        img {
                                            src: data_url,
                                            style: "max-width:100%; max-height:100%; object-fit:contain;",
                                        }
                                    },
                                    Err(_) => rsx! {
                                        div { style: "color:white; font-size:48px;", "⚠️" }
                                    },
                                }
                            } else {
                                rsx! {
                                    div { style: "color:white; font-size:24px;",
                                        "No photo available"
                                    }
                                }
                            }
                        }
                    }
                    // Navigation
                    if photos().len() > 1 {
                        div {
                            style: "padding:16px; display:flex; gap:12px; justify-content:center;",
                            onclick: move |e| e.stop_propagation(),
                            button {
                                style: "background:rgba(255,255,255,0.3); color:white; padding:12px 24px; border-radius:8px; font-size:18px; font-weight:600;",
                                disabled: current_photo_index() == 0,
                                onclick: move |_| {
                                    if current_photo_index() > 0 {
                                        current_photo_index.set(current_photo_index() - 1);
                                    }
                                },
                                "◀"
                            }
                            button {
                                style: "background:rgba(255,255,255,0.3); color:white; padding:12px 24px; border-radius:8px; font-size:18px; font-weight:600;",
                                disabled: current_photo_index() >= photos().len() - 1,
                                onclick: move |_| {
                                    if current_photo_index() < photos().len() - 1 {
                                        current_photo_index.set(current_photo_index() + 1);
                                    }
                                },
                                "▶"
                            }
                        }
                    }
                }
            }
        }
    }
}
