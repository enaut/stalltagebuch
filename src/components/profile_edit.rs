use crate::{
    database,
    models::{Gender, Ringfarbe, Wachtel},
    services, Screen,
};
use dioxus::prelude::*;

#[component]
pub fn ProfileEditScreen(wachtel_id: i64, on_navigate: EventHandler<Screen>) -> Element {
    let mut profile = use_signal(|| None::<Wachtel>);
    let mut name = use_signal(|| String::new());
    let mut gender = use_signal(|| "unknown".to_string());
    let mut ring_color = use_signal(|| String::new());
    let mut photos = use_signal(|| Vec::<crate::models::Photo>::new());
    let mut selected_profile_photo_id = use_signal(|| None::<i64>);
    let mut show_delete_confirm = use_signal(|| false);
    let mut error = use_signal(|| String::new());
    let mut success = use_signal(|| false);

    // Load profile and photos
    use_effect(move || {
        match database::init_database() {
            Ok(conn) => {
                match services::profile_service::get_profile(&conn, wachtel_id) {
                    Ok(p) => {
                        name.set(p.name.clone());
                        gender.set(p.gender.as_str().to_string());
                        if let Some(rc) = &p.ring_color {
                            ring_color.set(rc.as_str().to_string());
                        }
                        profile.set(Some(p));
                    }
                    Err(e) => {
                        error.set(format!("Fehler beim Laden: {}", e));
                    }
                }
                // Lade alle Fotos
                match crate::services::photo_service::list_wachtel_photos(&conn, wachtel_id) {
                    Ok(photo_list) => {
                        // Finde aktuelles Profilbild
                        if let Some(profile_photo) = photo_list.iter().find(|p| p.is_profile) {
                            selected_profile_photo_id.set(profile_photo.id);
                        }
                        photos.set(photo_list);
                    }
                    Err(e) => {
                        eprintln!("Fehler beim Laden der Fotos: {}", e);
                    }
                }
            }
            Err(e) => {
                error.set(format!("DB-Fehler: {}", e));
            }
        }
    });

    let mut handle_submit = move || {
        if name().trim().is_empty() {
            error.set("Name darf nicht leer sein".to_string());
            return;
        }

        if let Some(mut updated_profile) = profile() {
            updated_profile.name = name().trim().to_string();
            updated_profile.gender = match gender().as_str() {
                "male" => Gender::Male,
                "female" => Gender::Female,
                _ => Gender::Unknown,
            };

            // Ring Color
            let ring_color_value = ring_color();
            let ring_color_trimmed = ring_color_value.trim();
            updated_profile.ring_color = if ring_color_trimmed.is_empty() {
                None
            } else {
                Some(Ringfarbe::from_str(ring_color_trimmed))
            };

            match database::init_database() {
                Ok(conn) => {
                    match services::profile_service::update_profile(&conn, &updated_profile) {
                        Ok(_) => {
                            // Aktualisiere Profilbild falls ausgewählt
                            if let Some(photo_id) = selected_profile_photo_id() {
                                let _ = crate::services::photo_service::set_profile_photo(
                                    &conn, wachtel_id, photo_id,
                                );
                            }
                            success.set(true);
                            // Navigate back immediately
                            on_navigate.call(Screen::ProfileDetail(wachtel_id));
                        }
                        Err(e) => {
                            error.set(format!("Fehler beim Speichern: {}", e));
                        }
                    }
                }
                Err(e) => {
                    error.set(format!("DB-Fehler: {}", e));
                }
            }
        }
    };

    let mut handle_delete = move || match database::init_database() {
        Ok(conn) => match services::profile_service::delete_profile(&conn, wachtel_id) {
            Ok(_) => {
                on_navigate.call(Screen::ProfileList);
            }
            Err(e) => {
                error.set(format!("Fehler beim Löschen: {}", e));
            }
        },
        Err(e) => {
            error.set(format!("DB-Fehler: {}", e));
        }
    };

    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div {
                style: "display: flex; align-items: center; gap: 12px; margin-bottom: 20px; padding-top: 8px;",
                button {
                    style: "padding: 8px 12px; background: #e0e0e0; color: #666; font-size: 20px; border-radius: 8px;",
                    onclick: move |_| on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                    "←"
                }
                h1 {
                    style: "color: #0066cc; margin: 0; font-size: 24px; font-weight: 700; flex: 1;",
                    "✏️ Profil bearbeiten"
                }
            }

            // Success Message
            if success() {
                div {
                    style: "padding: 12px 16px; background: #d4edda; border-radius: 8px; color: #155724; font-size: 14px; margin-bottom: 16px; border-left: 3px solid #28a745;",
                    "✓ Profil erfolgreich aktualisiert!"
                }
            }

            // Error Message
            if !error().is_empty() {
                div {
                    style: "padding: 12px 16px; background: #ffe6e6; border-radius: 8px; color: #cc0000; font-size: 14px; margin-bottom: 16px; border-left: 3px solid #cc0000;",
                    "⚠️ {error}"
                }
            }

            // Form
            div {
                class: "card",

                // Name Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        "Name *"
                    }
                    input {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
                        r#type: "text",
                        placeholder: "z.B. Henne 1",
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                        autofocus: true,
                    }
                }

                // Gender Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        "Geschlecht"
                    }
                    select {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
                        value: "{gender}",
                        onchange: move |e| gender.set(e.value()),
                        option { value: "unknown", "Unbekannt" }
                        option { value: "female", "Weiblich" }
                        option { value: "male", "Männlich" }
                    }
                }

                // Ring Color Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        "Ringfarbe"
                    }
                    select {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
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
                    "ℹ️ Fotos werden in der Detailansicht hinzugefügt. Hier können Sie nur das Profilbild auswählen oder Fotos löschen."
                }

                // Photo Gallery with Profile Selection
                div {
                    style: "margin-bottom: 24px;",
                    label {
                        style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        "Fotos ({photos().len()})"
                    }

                    if photos().is_empty() {
                        div {
                            style: "padding: 24px; text-align: center; background: #f5f5f5; border-radius: 8px; color: #999;",
                            "Keine Fotos vorhanden. Fügen Sie Fotos in der Detailansicht hinzu."
                        }
                    } else {
                        div {
                            style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 12px;",
                            for photo in photos() {
                                {
                                    let border_color = if selected_profile_photo_id() == photo.id { "#0066cc" } else { "#e0e0e0" };
                                    let photo_style = format!("position: relative; aspect-ratio: 1/1; border-radius: 8px; overflow: hidden; border: 2px solid {};", border_color);
                                    rsx! {
                                div {
                                    key: "{photo.id.unwrap_or(0)}",
                                    style: "{photo_style}",
                                    // Bild
                                    {
                                        let thumb_path = photo.thumbnail_path.clone().unwrap_or(photo.path.clone());
                                        match crate::image_processing::image_path_to_data_url(&thumb_path) {
                                            Ok(data_url) => rsx! {
                                                img {
                                                    src: data_url,
                                                    style: "width: 100%; height: 100%; object-fit: cover; cursor: pointer;",
                                                    onclick: move |_| {
                                                        selected_profile_photo_id.set(photo.id);
                                                    }
                                                }
                                            },
                                            Err(_) => rsx! {
                                                div {
                                                    style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: #f0f0f0; color: #999; font-size: 32px;",
                                                    "📷"
                                                }
                                            }
                                        }
                                    }
                                    // Profilbild-Badge
                                    if photo.is_profile {
                                        div {
                                            style: "position: absolute; top: 4px; left: 4px; background: rgba(0, 102, 204, 0.9); color: white; padding: 4px 8px; border-radius: 4px; font-size: 10px; font-weight: 600;",
                                            "PROFIL"
                                        }
                                    }
                                    // Löschen-Button
                                    button {
                                        style: "position: absolute; top: 4px; right: 4px; width: 28px; height: 28px; background: rgba(204, 0, 0, 0.9); color: white; border-radius: 50%; font-size: 14px; display: flex; align-items: center; justify-content: center; cursor: pointer;",
                                        onclick: move |_| {
                                            if let Ok(conn) = database::init_database() {
                                                if let Some(id) = photo.id {
                                                    match crate::services::photo_service::delete_photo(&conn, id) {
                                                        Ok(_) => {
                                                            // Reload photos
                                                            if let Ok(photo_list) = crate::services::photo_service::list_wachtel_photos(&conn, wachtel_id) {
                                                                photos.set(photo_list);
                                                                // Aktualisiere selected_profile_photo_id
                                                                if selected_profile_photo_id() == Some(id) {
                                                                    selected_profile_photo_id.set(None);
                                                                }
                                                            }
                                                        }
                                                        Err(e) => error.set(format!("Fehler beim Löschen: {}", e)),
                                                    }
                                                }
                                            }
                                        },
                                        "×"
                                    }
                                    // Radio-Button für Profilbild-Auswahl
                                    if selected_profile_photo_id() == photo.id {
                                        div {
                                            style: "position: absolute; bottom: 4px; right: 4px; width: 24px; height: 24px; background: #0066cc; border-radius: 50%; display: flex; align-items: center; justify-content: center; color: white; font-size: 16px;",
                                            "✓"
                                        }
                                    }
                                }
                                    }
                                }
                            }
                        }
                        div {
                            style: "margin-top: 12px; padding: 10px; background: #f9f9f9; border-radius: 6px; font-size: 12px; color: #666;",
                            "Tippen Sie auf ein Foto, um es als Profilbild zu markieren."
                        }
                    }
                }

                // Buttons
                div {
                    style: "display: flex; gap: 12px;",
                    button {
                        class: "btn-success",
                        style: "flex: 1; padding: 14px; font-size: 16px; font-weight: 600;",
                        onclick: move |_| handle_submit(),
                        "✓ Speichern"
                    }
                    button {
                        style: "flex: 1; padding: 14px; background: #e0e0e0; color: #666; font-size: 16px; font-weight: 600;",
                        onclick: move |_| on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                        "✕ Abbrechen"
                    }
                }

                // Delete Section
                div {
                    style: "margin-top: 32px; padding-top: 24px; border-top: 2px solid #f0f0f0;",
                    if show_delete_confirm() {
                        div {
                            div {
                                style: "margin-bottom: 16px; padding: 12px; background: #fff3cd; border-radius: 8px; color: #856404;",
                                "⚠️ Möchten Sie diese Wachtel wirklich löschen? Diese Aktion kann nicht rückgängig gemacht werden."
                            }
                            div {
                                style: "display: flex; gap: 12px;",
                                button {
                                    class: "btn-danger",
                                    style: "flex: 1; padding: 14px; font-size: 16px; font-weight: 600;",
                                    onclick: move |_| handle_delete(),
                                    "🗑️ Endgültig löschen"
                                }
                                button {
                                    style: "flex: 1; padding: 14px; background: #e0e0e0; color: #666; font-size: 16px; font-weight: 600;",
                                    onclick: move |_| show_delete_confirm.set(false),
                                    "Abbrechen"
                                }
                            }
                        }
                    } else {
                        button {
                            style: "width: 100%; padding: 12px; background: #ffe6e6; color: #cc0000; font-size: 14px; font-weight: 600; border: 1px solid #ffcccc; border-radius: 8px;",
                            onclick: move |_| show_delete_confirm.set(true),
                            "🗑️ Wachtel löschen"
                        }
                    }
                }
            }
        }
    }
}
