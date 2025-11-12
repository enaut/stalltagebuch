use crate::{
    database, image_processing,
    models::{EventType, WachtelEvent},
    services::{event_service, photo_service},
    Screen,
};
use chrono::NaiveDate;
use dioxus::prelude::*;

#[component]
pub fn EventEditScreen(
    event_id: i64,
    wachtel_id: i64,
    on_navigate: EventHandler<Screen>,
) -> Element {
    let mut event = use_signal(|| None::<WachtelEvent>);
    let mut event_type = use_signal(|| EventType::AmLeben);
    let mut event_date_str = use_signal(|| {
        chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    });
    let mut notes = use_signal(|| String::new());
    let mut photos = use_signal(|| Vec::<crate::models::Photo>::new());
    let mut error = use_signal(|| String::new());
    let mut success = use_signal(|| false);
    let mut uploading = use_signal(|| false);

    // Load event + photos
    use_effect(move || {
        if let Ok(conn) = database::init_database() {
            match event_service::get_event_by_id(&conn, event_id) {
                Ok(Some(e)) => {
                    event.set(Some(e.clone()));
                    event_type.set(e.event_type.clone());
                    event_date_str.set(e.event_date.format("%Y-%m-%d").to_string());
                    notes.set(e.notes.unwrap_or_default());
                }
                Ok(None) => error.set("Ereignis nicht gefunden".to_string()),
                Err(e) => error.set(format!("Fehler beim Laden: {}", e)),
            }
            match photo_service::list_event_photos(&conn, event_id) {
                Ok(list) => photos.set(list),
                Err(e) => eprintln!("Fehler beim Laden der Event-Fotos: {}", e),
            }
        }
    });

    // Save handler
    let mut handle_save = move || {
        if event_date_str().is_empty() {
            error.set("Datum darf nicht leer sein".to_string());
            return;
        }
        let parsed_date = match NaiveDate::parse_from_str(&event_date_str(), "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                error.set("Ungültiges Datum".to_string());
                return;
            }
        };
        if let Ok(conn) = database::init_database() {
            match event_service::update_event_full(
                &conn,
                event_id,
                event_type(),
                parsed_date,
                if notes().is_empty() {
                    None
                } else {
                    Some(notes())
                },
            ) {
                Ok(_) => {
                    success.set(true);
                    on_navigate.call(Screen::ProfileDetail(wachtel_id));
                }
                Err(e) => error.set(format!("Speicherfehler: {}", e)),
            }
        } else {
            error.set("DB nicht verfügbar".to_string());
        }
    };

    // Delete handler
    let mut handle_delete = move || {
        if let Ok(conn) = database::init_database() {
            match event_service::delete_event(&conn, event_id) {
                Ok(_) => on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                Err(e) => error.set(format!("Löschen fehlgeschlagen: {}", e)),
            }
        }
    };

    rsx! {
        div { style: "padding:16px; max-width:600px; margin:0 auto;",
            // Header
            div { style: "display:flex; align-items:center; gap:12px; margin-bottom:20px;",
                button {
                    style: "padding:8px 12px; background:#e0e0e0; border-radius:8px;",
                    onclick: move |_| on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                    "←"
                }
                h1 { style: "margin:0; font-size:22px; color:#0066cc;", "Ereignis bearbeiten" }
            }
            if !error().is_empty() {
                div { style: "background:#ffe6e6; padding:12px; border-radius:8px; color:#c00; margin-bottom:16px;",
                    "⚠️ {error}"
                }
            }
            if success() {
                div { style: "background:#e6ffe6; padding:12px; border-radius:8px; color:#060; margin-bottom:16px;",
                    "✓ Aktualisiert"
                }
            }
            if let Some(_) = event() {
                // Event type
                div { style: "margin-bottom:16px;",
                    label { style: "display:block; font-weight:600; margin-bottom:6px;",
                        "Typ"
                    }
                    select {
                        value: "{event_type():?}",
                        onchange: move |ev| {
                            let v = ev.value();
                            let t = match v.as_str() {
                                "Geboren" => EventType::Geboren,
                                "AmLeben" => EventType::AmLeben,
                                "Krank" => EventType::Krank,
                                "Gesund" => EventType::Gesund,
                                "MarkiertZumSchlachten" => EventType::MarkiertZumSchlachten,
                                "Geschlachtet" => EventType::Geschlachtet,
                                "Gestorben" => EventType::Gestorben,
                                _ => EventType::AmLeben,
                            };
                            event_type.set(t);
                        },
                        style: "width:100%; padding:10px; border:1px solid #ccc; border-radius:8px;",
                        option { value: "Geboren", "🐣 Geboren" }
                        option { value: "AmLeben", "✅ Am Leben" }
                        option { value: "Krank", "🤒 Krank" }
                        option { value: "Gesund", "💪 Gesund" }
                        option { value: "MarkiertZumSchlachten", "🥩 Markiert" }
                        option { value: "Geschlachtet", "🥩 Geschlachtet" }
                        option { value: "Gestorben", "🪦 Gestorben" }
                    }
                }
                // Date
                div { style: "margin-bottom:16px;",
                    label { style: "display:block; font-weight:600; margin-bottom:6px;",
                        "Datum"
                    }
                    input {
                        r#type: "date",
                        value: "{event_date_str}",
                        oninput: move |ev| event_date_str.set(ev.value()),
                        style: "width:100%; padding:10px; border:1px solid #ccc; border-radius:8px;",
                    }
                }
                // Notes
                div { style: "margin-bottom:16px;",
                    label { style: "display:block; font-weight:600; margin-bottom:6px;",
                        "Notizen"
                    }
                    textarea {
                        value: "{notes}",
                        oninput: move |ev| notes.set(ev.value()),
                        style: "width:100%; padding:10px; border:1px solid #ccc; border-radius:8px; min-height:120px;",
                    }
                }
                // Photos grid
                div { style: "margin-bottom:20px;",
                    label { style: "display:block; font-weight:600; margin-bottom:6px;",
                        "Fotos ({photos().len()})"
                    }
                    if !photos().is_empty() {
                        div { style: "display:grid; grid-template-columns:repeat(auto-fill,minmax(110px,1fr)); gap:10px; margin-bottom:12px;",
                            for photo in photos() {
                                {
                                    let thumb = photo.thumbnail_path.clone().unwrap_or(photo.path.clone());
                                    let style_border = "border:2px solid #e0e0e0;";
                                    rsx! {
                                        div {
                                            key: "{photo.id.unwrap_or(0)}",
                                            style: "position:relative; aspect-ratio:1/1; border-radius:8px; overflow:hidden; {style_border}",
                                            {
                                                match image_processing::image_path_to_data_url(&thumb) {
                                                    Ok(data_url) => rsx! {
                                                        img { src: data_url, style: "width:100%; height:100%; object-fit:cover;" }
                                                    },
                                                    Err(_) => rsx! {
                                                        div { style: "width:100%; height:100%; display:flex; align-items:center; justify-content:center; background:#ddd; color:#666;",
                                                            "⚠️"
                                                        }
                                                    },
                                                }
                                            }
                                            button {
                                                style: "position:absolute; top:4px; right:4px; width:28px; height:28px; background:rgba(204,0,0,0.85); color:white; border-radius:50%; font-size:14px; cursor:pointer;",
                                                onclick: move |_| {
                                                    if let Some(pid) = photo.id {
                                                        if let Ok(conn) = database::init_database() {
                                                            let _ = photo_service::delete_photo(&conn, pid);
                                                            if let Ok(list) = photo_service::list_event_photos(&conn, event_id) {
                                                                photos.set(list);
                                                            }
                                                        }
                                                    }
                                                },
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Add buttons (always visible)
                    div { style: "display:flex; gap:12px;",
                        button {
                            disabled: uploading(),
                            style: "flex:1; padding:10px; background:rgba(0,0,0,0.6); color:white; border-radius:8px;",
                            onclick: move |_| {
                                uploading.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::pick_images() {
                                            Ok(paths) => {
                                                if let Ok(conn) = database::init_database() {
                                                    for p in paths {
                                                        let ps = p.to_string_lossy().to_string();
                                                        let th = image_processing::create_thumbnail(&ps).ok();
                                                        let _ = photo_service::add_event_photo(
                                                            &conn,
                                                            event_id,
                                                            ps,
                                                            th,
                                                        );
                                                    }
                                                    if let Ok(list) = photo_service::list_event_photos(
                                                        &conn,
                                                        event_id,
                                                    ) {
                                                        photos.set(list);
                                                    }
                                                }
                                            }
                                            Err(e) => error.set(format!("Auswahlfehler: {}", e)),
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        error.set("Nur Android unterstützt Mehrfachauswahl".to_string());
                                    }
                                    uploading.set(false);
                                });
                            },
                            if uploading() {
                                "⏳"
                            } else {
                                "🖼️ Galerie"
                            }
                        }
                        button {
                            disabled: uploading(),
                            style: "flex:1; padding:10px; background:rgba(0,0,0,0.6); color:white; border-radius:8px;",
                            onclick: move |_| {
                                uploading.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::capture_photo() {
                                            Ok(p) => {
                                                if let Ok(conn) = database::init_database() {
                                                    let ps = p.to_string_lossy().to_string();
                                                    let th = image_processing::create_thumbnail(&ps).ok();
                                                    let _ = photo_service::add_event_photo(
                                                        &conn,
                                                        event_id,
                                                        ps,
                                                        th,
                                                    );
                                                    if let Ok(list) = photo_service::list_event_photos(
                                                        &conn,
                                                        event_id,
                                                    ) {
                                                        photos.set(list);
                                                    }
                                                }
                                            }
                                            Err(e) => error.set(format!("Aufnahmefehler: {}", e)),
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        error.set("Nur Android Kamera verfügbar".to_string());
                                    }
                                    uploading.set(false);
                                });
                            },
                            if uploading() {
                                "⏳"
                            } else {
                                "📷 Foto"
                            }
                        }
                    }
                }
                // Action buttons
                div { style: "display:flex; gap:12px;",
                    button {
                        style: "flex:1; padding:14px; background:#0066cc; color:white; border-radius:8px; font-weight:600;",
                        onclick: move |_| handle_save(),
                        "✓ Speichern"
                    }
                    button {
                        style: "flex:1; padding:14px; background:#e0e0e0; color:#333; border-radius:8px; font-weight:600;",
                        onclick: move |_| on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                        "Abbrechen"
                    }
                    button {
                        style: "flex:1; padding:14px; background:#ffdddd; color:#cc0000; border-radius:8px; font-weight:600;",
                        onclick: move |_| handle_delete(),
                        "🗑️ Löschen"
                    }
                }
            } else {
                div { style: "padding:40px; text-align:center; color:#666;", "Lade Ereignis..." }
            }
        }
    }
}
