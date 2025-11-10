use crate::{
    database,
    models::{Gender, Ringfarbe, Wachtel},
    services, Screen,
};
use dioxus::prelude::*;
use std::path::PathBuf;

#[component]
pub fn AddProfileScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut name = use_signal(|| String::new());
    let mut gender = use_signal(|| "unknown".to_string());
    let mut ring_color = use_signal(|| String::new());
    let mut photo_path = use_signal(|| None::<PathBuf>);
    let mut uploading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    let mut handle_submit = move || {
        error.set(None);
        success.set(false);

        let name_value = name();
        let name_trimmed = name_value.trim();
        if name_trimmed.is_empty() {
            error.set(Some("Name darf nicht leer sein".to_string()));
            return;
        }

        let mut wachtel = Wachtel::new(name_trimmed.to_string());

        wachtel.gender = match gender().as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            _ => Gender::Unknown,
        };

        let ring_color_value = ring_color();
        let ring_color_trimmed = ring_color_value.trim();
        wachtel.ring_color = if ring_color_trimmed.is_empty() {
            None
        } else {
            Some(Ringfarbe::from_str(ring_color_trimmed))
        };

        match database::init_database() {
            Ok(conn) => {
                match services::create_profile(&conn, &wachtel) {
                    Ok(wachtel_id) => {
                        // Speichere Profilfoto falls vorhanden
                        if let Some(path) = photo_path() {
                            let path_str = path.to_string_lossy().to_string();
                            let thumbnail_opt =
                                crate::image_processing::create_thumbnail(&path_str).ok();
                            let _ = crate::services::photo_service::add_wachtel_photo(
                                &conn,
                                wachtel_id,
                                path_str,
                                thumbnail_opt,
                                true, // is_profile
                            );
                        }
                        success.set(true);
                        on_navigate.call(Screen::ProfileList);
                    }
                    Err(e) => {
                        error.set(Some(format!("Fehler beim Speichern: {}", e)));
                    }
                }
            }
            Err(e) => {
                error.set(Some(format!("Datenbankfehler: {}", e)));
            }
        }
    };

    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            div {
                style: "display: flex; align-items: center; margin-bottom: 24px;",
                button {
                    class: "btn-secondary",
                    style: "margin-right: 12px; padding: 8px 16px;",
                    onclick: move |_| on_navigate.call(Screen::ProfileList),
                    "← Zurück"
                }
                h1 {
                    style: "color: #0066cc; font-size: 24px; font-weight: 700; margin: 0;",
                    "Neues Profil"
                }
            }

            if let Some(err) = error() {
                div {
                    style: "background: #fee; border: 1px solid #fcc; color: #c33; padding: 12px; margin-bottom: 16px; border-radius: 8px; font-size: 14px;",
                    "⚠️ {err}"
                }
            }

            if success() {
                div {
                    style: "background: #efe; border: 1px solid #cfc; color: #3a3; padding: 12px; margin-bottom: 16px; border-radius: 8px; font-size: 14px;",
                    "✅ Profil erfolgreich erstellt!"
                }
            }

            div {
                class: "card",

                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        "Name *"
                    }
                    input {
                        r#type: "text",
                        class: "input",
                        placeholder: "z.B. Flecki",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                        autofocus: true,
                    }
                }

                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        "Geschlecht"
                    }
                    select {
                        class: "input",
                        value: "{gender}",
                        onchange: move |e| gender.set(e.value()),
                        option { value: "unknown", "Unbekannt" }
                        option { value: "female", "Weiblich" }
                        option { value: "male", "Männlich" }
                    }
                }

                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        "Ringfarbe"
                    }
                    select {
                        class: "input",
                        value: "{ring_color}",
                        onchange: move |e| ring_color.set(e.value()),
                        option { value: "", "Keine" }
                        option { value: "lila", "Lila" }
                        option { value: "rosa", "Rosa" }
                        option { value: "hellblau", "Hellblau" }
                        option { value: "dunkelblau", "Dunkelblau" }
                        option { value: "rot", "Rot" }
                        option { value: "orange", "Orange" }
                        option { value: "weiss", "Weiß" }
                        option { value: "gelb", "Gelb" }
                        option { value: "schwarz", "Schwarz" }
                        option { value: "gruen", "Grün" }
                    }
                }

                div {
                    style: "padding: 12px; background: #e3f2fd; border-radius: 8px; color: #0066cc; font-size: 13px; margin-bottom: 20px;",
                    "ℹ️ Geburtsdatum und Notizen können nach dem Erstellen als Ereignisse hinzugefügt werden."
                }

                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        "Foto"
                    }

                    div {
                        style: "margin-bottom: 12px;",
                        if let Some(path) = photo_path() {
                            div {
                                style: "display: flex; align-items: center; gap: 12px; padding: 12px; background: #f0f0f0; border-radius: 8px;",
                                div {
                                    style: "width: 60px; height: 60px; background: #ddd; border-radius: 8px; display: flex; align-items: center; justify-content: center; font-size: 32px;",
                                    "📷"
                                }
                                div {
                                    style: "flex: 1;",
                                    div {
                                        style: "font-size: 14px; font-weight: 600; color: #333;",
                                        "Foto ausgewählt"
                                    }
                                    div {
                                        style: "font-size: 12px; color: #666; word-break: break-all;",
                                        "{path.file_name().and_then(|n| n.to_str()).unwrap_or(\"Unbekannt\")}"
                                    }
                                }
                                button {
                                    class: "btn-secondary",
                                    style: "padding: 6px 12px; font-size: 12px;",
                                    onclick: move |_| photo_path.set(None),
                                    "🗑️"
                                }
                            }
                        } else {
                            div {
                                style: "width: 100%; height: 120px; border: 2px dashed #ccc; border-radius: 8px; display: flex; align-items: center; justify-content: center; color: #999; font-size: 14px;",
                                "Kein Foto ausgewählt"
                            }
                        }
                    }

                    div {
                        style: "display: flex; gap: 8px;",
                        button {
                            class: "btn-secondary",
                            style: "flex: 1; padding: 10px; font-size: 14px;",
                            disabled: uploading(),
                            onclick: move |_| {
                                uploading.set(true);
                                error.set(None);
                                spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::pick_image() {
                                            Ok(path) => photo_path.set(Some(path)),
                                            Err(e) => error.set(Some(format!("Fehler: {}", e))),
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        error.set(Some("Nur auf Android verfügbar".to_string()));
                                    }
                                    uploading.set(false);
                                });
                            },
                            if uploading() { "⏳ Lädt..." } else { "🖼️ Galerie" }
                        }
                        button {
                            class: "btn-secondary",
                            style: "flex: 1; padding: 10px; font-size: 14px;",
                            disabled: uploading(),
                            onclick: move |_| {
                                uploading.set(true);
                                error.set(None);
                                spawn(async move {
                                    #[cfg(target_os = "android")]
                                    {
                                        match crate::camera::capture_photo() {
                                            Ok(path) => photo_path.set(Some(path)),
                                            Err(e) => error.set(Some(format!("Fehler: {}", e))),
                                        }
                                    }
                                    #[cfg(not(target_os = "android"))]
                                    {
                                        error.set(Some("Nur auf Android verfügbar".to_string()));
                                    }
                                    uploading.set(false);
                                });
                            },
                            if uploading() { "⏳ Lädt..." } else { "📷 Kamera" }
                        }
                    }
                }

                div {
                    style: "display: flex; gap: 12px; margin-top: 24px;",
                    button {
                        class: "btn-primary",
                        style: "flex: 1; padding: 14px;",
                        onclick: move |_| handle_submit(),
                        "💾 Speichern"
                    }
                    button {
                        class: "btn-secondary",
                        style: "flex: 1; padding: 14px;",
                        onclick: move |_| on_navigate.call(Screen::ProfileList),
                        "❌ Abbrechen"
                    }
                }
            }
        }
    }
}
