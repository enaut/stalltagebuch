use crate::{
    database,
    models::{Gender, Quail, RingColor},
    services, Screen,
};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn ProfileEditScreen(quail_id: String, on_navigate: EventHandler<Screen>) -> Element {
    let mut profile = use_signal(|| None::<Quail>);
    let mut name = use_signal(|| String::new());
    let mut gender = use_signal(|| "unknown".to_string());
    let mut ring_color = use_signal(|| String::new());
    let mut photos = use_signal(|| Vec::<crate::models::Photo>::new());
    let mut selected_profile_photo_id = use_signal(|| None::<String>);
    let mut show_delete_confirm = use_signal(|| false);
    let mut error = use_signal(|| String::new());
    let mut success = use_signal(|| false);
    let mut saving = use_signal(|| false);

    // Load profile and photos
    let quail_id_for_load = quail_id.clone();
    use_effect(move || {
        match database::init_database() {
            Ok(conn) => {
                if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_for_load) {
                    match services::profile_service::get_profile(&conn, &uuid) {
                        Ok(p) => {
                            name.set(p.name.clone());
                            gender.set(p.gender.as_str().to_string());
                            if let Some(rc) = &p.ring_color {
                                ring_color.set(rc.as_str().to_string());
                            }
                            profile.set(Some(p));
                        }
                        Err(e) => {
                            error.set(t!("error-load-failed", error: e.to_string()));
                            // Failed to load
                        }
                    }
                    // Lade alle Fotos
                    match crate::services::photo_service::list_quail_photos(&conn, &uuid) {
                        Ok(photo_list) => {
                            // Finde aktuelles Profilbild
                            if let Ok(Some(profile_photo)) =
                                crate::services::photo_service::get_profile_photo(&conn, &uuid)
                            {
                                selected_profile_photo_id.set(Some(profile_photo.uuid.to_string()));
                            }
                            photos.set(photo_list);
                        }
                        Err(e) => {
                            log::error!("{}: {}", t!("error-load-photos-failed"), e);
                            // Failed to load photos
                        }
                    }
                }
            }
            Err(e) => {
                error.set(format!("{}: {}", t!("error-database"), e)); // Database error
            }
        }
    });

    let quail_id_for_submit = quail_id.clone();
    let mut handle_submit = move || {
        if name().trim().is_empty() {
            error.set(t!("error-name-required")); // Name is required
            return;
        }

        saving.set(true);

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
                Some(RingColor::from_str(ring_color_trimmed))
            };

            let updated_profile_clone = updated_profile.clone();
            let quail_id_clone = quail_id_for_submit.clone();
            let selected_photo = selected_profile_photo_id();
            spawn(async move {
                match database::init_database() {
                    Ok(conn) => {
                        match services::profile_service::update_profile(
                            &conn,
                            &updated_profile_clone,
                        )
                        .await
                        {
                            Ok(_) => {
                                // Aktualisiere Profilbild falls ausgewählt
                                if let Some(photo_uuid_str) = selected_photo {
                                    if let (Ok(quail_uuid), Ok(photo_uuid)) = (
                                        uuid::Uuid::parse_str(&quail_id_clone),
                                        uuid::Uuid::parse_str(&photo_uuid_str),
                                    ) {
                                        let _ = crate::services::photo_service::set_profile_photo(
                                            &conn,
                                            &quail_uuid,
                                            &photo_uuid,
                                        )
                                        .await;
                                    }
                                }
                                success.set(true);
                                saving.set(false);
                                // Navigate back immediately
                                on_navigate.call(Screen::ProfileDetail(quail_id_clone.clone()));
                            }
                            Err(e) => {
                                error.set(format!("{}: {}", t!("error-save-failed"), e));
                                saving.set(false);
                            }
                        }
                    }
                    Err(e) => {
                        error.set(format!("{}: {}", t!("error-database"), e)); // Database error
                        saving.set(false);
                    }
                }
            });
        }
    };

    let quail_id_for_delete = quail_id.clone();
    let handle_delete = move || {
        let quail_id_clone = quail_id_for_delete.clone();
        spawn(async move {
            match database::init_database() {
                Ok(conn) => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&quail_id_clone) {
                        match services::profile_service::delete_profile(&conn, &uuid).await {
                            Ok(_) => {
                                on_navigate.call(Screen::ProfileList);
                            }
                            Err(e) => {
                                error.set(format!("{}: {}", t!("error-delete-failed"), e));
                                // Failed to delete
                            }
                        }
                    }
                }
                Err(e) => {
                    error.set(t!("error-database", error: e.to_string())); // Database error
                }
            }
        });
    };

    let quail_id_for_back = quail_id.clone();
    let quail_id_for_photo_delete = quail_id.clone();
    let quail_id_for_cancel = quail_id.clone();

    rsx! {
        div { style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 20px; padding-top: 8px;",
                button {
                    style: "padding: 8px 12px; background: #e0e0e0; color: #666; font-size: 20px; border-radius: 8px;",
                    onclick: move |_| on_navigate.call(Screen::ProfileDetail(quail_id_for_back.clone())),
                    "←"
                }
                h1 { style: "color: #0066cc; margin: 0; font-size: 24px; font-weight: 700; flex: 1;",
                    "✏️ "
                    {t!("profile-edit-title")}
                }
            }

            // Success Message
            if success() {
                div { style: "padding: 12px 16px; background: #d4edda; border-radius: 8px; color: #155724; font-size: 14px; margin-bottom: 16px; border-left: 3px solid #28a745;",
                    "✓ "
                    {t!("success-profile-updated")}
                }
            }

            // Error Message
            if !error().is_empty() {
                div { style: "padding: 12px 16px; background: #ffe6e6; border-radius: 8px; color: #cc0000; font-size: 14px; margin-bottom: 16px; border-left: 3px solid #cc0000;",
                    "⚠️ "
                    {error}
                }
            }

            // Form
            div { class: "card",

                // Name Field
                div { style: "margin-bottom: 20px;",
                    label { style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        {t!("field-name-required")} // Name *
                    }
                    input {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
                        r#type: "text",
                        placeholder: "{t!(\"field-name-placeholder\")}", // e.g. Hen 1
                        value: "{name}",
                        oninput: move |e| name.set(e.value()),
                        autofocus: true,
                    }
                }

                // Gender Field
                div { style: "margin-bottom: 20px;",
                    label { style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        {t!("field-gender")} // Gender
                    }
                    select {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
                        value: "{gender}",
                        onchange: move |e| gender.set(e.value()),
                        option { value: "unknown", {t!("gender-unknown")} } // Unknown
                        option { value: "female", {t!("gender-female")} } // Female
                        option { value: "male", {t!("gender-male")} } // Male
                    }
                }

                // Ring Color Field
                div { style: "margin-bottom: 20px;",
                    label { style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        {t!("field-ring-color")} // Ring color
                    }
                    select {
                        style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 8px; background: white;",
                        value: "{ring_color}",
                        onchange: move |e| ring_color.set(e.value()),
                        option { value: "", {t!("ring-color-none")} } // None
                        option { value: "lila", {t!("ring-color-purple")} } // Purple
                        option { value: "rosa", {t!("ring-color-pink")} } // Pink
                        option { value: "hellblau", {t!("ring-color-light-blue")} } // Light blue
                        option { value: "dunkelblau", {t!("ring-color-dark-blue")} } // Dark blue
                        option { value: "rot", {t!("ring-color-red")} } // Red
                        option { value: "orange", {t!("ring-color-orange")} } // Orange
                        option { value: "weiss", {t!("ring-color-white")} } // White
                        option { value: "gelb", {t!("ring-color-yellow")} } // Yellow
                        option { value: "schwarz", {t!("ring-color-black")} } // Black
                        option { value: "gruen", {t!("ring-color-green")} } // Green
                    }
                }

                div { style: "padding: 12px; background: #e3f2fd; border-radius: 8px; color: #0066cc; font-size: 13px; margin-bottom: 20px;",
                    "ℹ️ "
                    {t!("info-photos-detail-view")}
                }

                // Photo Gallery with Profile Selection
                div { style: "margin-bottom: 24px;",
                    label { style: "display: block; margin-bottom: 8px; font-weight: 600; color: #333; font-size: 14px;",
                        {format!("{} ({})", t!("field-photos"), photos().len())} // Photos count
                    }

                    if photos().is_empty() {
                        div { style: "padding: 24px; text-align: center; background: #f5f5f5; border-radius: 8px; color: #999;",
                            {t!("photos-empty")} // No photos available. Add photos in detail view.
                        }
                    } else {
                        div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 12px;",
                            for photo in photos() {
                                {
                                    let border_color = if selected_profile_photo_id().as_ref().map(|s| s.as_str())
                                        == Some(&photo.uuid.to_string())
                                    {
                                        "#0066cc"
                                    } else {
                                        "#e0e0e0"
                                    };
                                    let photo_style = format!(
                                        "position: relative; aspect-ratio: 1/1; border-radius: 8px; overflow: hidden; border: 2px solid {};",
                                        border_color,
                                    );
                                    rsx! {
                                        div { key: "{photo.uuid}", style: "{photo_style}",
                                            // Bild
                                            {
                                                let thumb_path = photo.thumbnail_path.clone().unwrap_or(photo.path.clone());
                                                match crate::image_processing::image_path_to_data_url(&thumb_path) {
                                                    Ok(data_url) => rsx! {
                                                        img {
                                                            src: data_url,
                                                            style: "width: 100%; height: 100%; object-fit: cover; cursor: pointer;",
                                                            onclick: move |_| {
                                                                selected_profile_photo_id.set(Some(photo.uuid.to_string()));
                                                            },
                                                        }
                                                    },
                                                    Err(_) => rsx! {
                                                        div { style: "width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: #f0f0f0; color: #999; font-size: 32px;",
                                                            "📷"
                                                        }
                                                    },
                                                }
                                            }
                                            // Löschen-Button
                                            button {
                                                style: "position: absolute; top: 4px; right: 4px; width: 28px; height: 28px; background: rgba(204, 0, 0, 0.9); color: white; border-radius: 50%; font-size: 14px; display: flex; align-items: center; justify-content: center; cursor: pointer;",
                                                onclick: {
                                                    let qid = quail_id_for_photo_delete.clone();
                                                    let photo_uuid = photo.uuid;
                                                    let photo_uuid_str = photo.uuid.to_string();
                                                    move |_| {
                                                        let qid_clone = qid.clone();
                                                        let photo_uuid_clone = photo_uuid.clone();
                                                        let photo_uuid_str_clone = photo_uuid_str.clone();
                                                        spawn(async move {
                                                            if let Ok(conn) = database::init_database() {
                                                                match crate::services::photo_service::delete_photo(&conn, &photo_uuid_clone).await {
                                                                    Ok(_) => {
                                                                        if let Ok(q_uuid) = uuid::Uuid::parse_str(&qid_clone) {
                                                                            if let Ok(photo_list) = crate::services::photo_service::list_quail_photos(
                                                                                &conn,
                                                                                &q_uuid,
                                                                            ) {
                                                                                photos.set(photo_list);
                                                                                if selected_profile_photo_id().as_ref().map(|s| s.as_str())
                                                                                    == Some(&photo_uuid_str_clone)
                                                                                {
                                                                                    selected_profile_photo_id.set(None);
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                    Err(e) => error.set(format!("{}: {}", t!("error-delete-failed"), e)),
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                                "×"
                                            }
                                            // Radio-Button für Profilbild-Auswahl
                                            if selected_profile_photo_id().as_ref().map(|s| s.as_str())
                                                == Some(&photo.uuid.to_string())
                                            {
                                                div { style: "position: absolute; bottom: 4px; right: 4px; width: 24px; height: 24px; background: #0066cc; border-radius: 50%; display: flex; align-items: center; justify-content: center; color: white; font-size: 16px;",
                                                    "✓"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        div { style: "margin-top: 12px; padding: 10px; background: #f9f9f9; border-radius: 6px; font-size: 12px; color: #666;",
                            {t!("info-tap-photo-to-mark")} // Tap a photo to mark it as profile photo.
                        }
                    }
                }

                // Buttons
                div { style: "display: flex; gap: 12px;",
                    button {
                        class: "btn-success",
                        style: "flex: 1; padding: 14px; font-size: 16px; font-weight: 600;",
                        disabled: saving(),
                        onclick: move |_| handle_submit(),
                        if saving() {
                            "⏳ "
                            {t!("action-saving")}
                        } else {
                            "✓ "
                            {t!("action-save")}
                        }
                    }
                    button {
                        style: "flex: 1; padding: 14px; background: #e0e0e0; color: #666; font-size: 16px; font-weight: 600;",
                        disabled: saving(),
                        onclick: move |_| on_navigate.call(Screen::ProfileDetail(quail_id_for_cancel.clone())),
                        "✕ "
                        {t!("action-cancel")}
                    }
                }

                // Delete Section
                div { style: "margin-top: 32px; padding-top: 24px; border-top: 2px solid #f0f0f0;",
                    if show_delete_confirm() {
                        div {
                            div { style: "margin-bottom: 16px; padding: 12px; background: #fff3cd; border-radius: 8px; color: #856404;",
                                "⚠️ "
                                {t!("confirm-delete-quail")}
                            }
                            div { style: "display: flex; gap: 12px;",
                                button {
                                    class: "btn-danger",
                                    style: "flex: 1; padding: 14px; font-size: 16px; font-weight: 600;",
                                    onclick: move |_| handle_delete(),
                                    "🗑️ "
                                    {t!("action-delete-permanently")}
                                }
                                button {
                                    style: "flex: 1; padding: 14px; background: #e0e0e0; color: #666; font-size: 16px; font-weight: 600;",
                                    onclick: move |_| show_delete_confirm.set(false),
                                    {t!("action-cancel")} // Cancel
                                }
                            }
                        }
                    } else {
                        button {
                            style: "width: 100%; padding: 12px; background: #ffe6e6; color: #cc0000; font-size: 14px; font-weight: 600; border: 1px solid #ffcccc; border-radius: 8px;",
                            onclick: move |_| show_delete_confirm.set(true),
                            "🗑️ "
                            {t!("action-delete-quail")}
                        }
                    }
                }
            }
        }
    }
}
