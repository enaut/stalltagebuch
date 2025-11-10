use crate::database;
use crate::image_processing;
use crate::models::{Ringfarbe, Wachtel};
use crate::services;
use crate::Screen;
use dioxus::prelude::*;

#[component]
pub fn ProfileListScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut profiles = use_signal(|| Vec::<Wachtel>::new());
    let mut search_filter = use_signal(|| String::new());
    // Toggle zeigt "nur Tote" an; Standard (false) zeigt Lebende + Markierte
    let mut show_dead = use_signal(|| false);

    // Load profiles
    let mut load_profiles = move || match database::init_database() {
        Ok(conn) => {
            let search_value = search_filter();
            let filter = if search_value.is_empty() {
                None
            } else {
                Some(search_value.as_str())
            };

            match services::profile_service::list_profiles_with_status(&conn, filter, !show_dead())
            {
                Ok(list) => profiles.set(list),
                Err(e) => eprintln!("Profile laden fehlgeschlagen: {}", e),
            }
        }
        Err(e) => eprintln!("DB-Fehler: {}", e),
    };

    // Load on mount
    use_effect(move || {
        load_profiles();
    });

    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; padding-top: 8px;",
                h1 {
                    style: "color: #0066cc; margin: 0; font-size: 24px; font-weight: 700;",
                    "🐦 Wachtel-Profile"
                }
                div { style: "display: flex; gap: 8px; align-items: center;",
                    // Toggle: nur tote anzeigen
                    button {
                        style: format!(
                            "padding: 8px 10px; font-size: 16px; border-radius: 8px; {}",
                            if show_dead() { "background:#ffe6e6; color:#a00; border:1px solid #f5b5b5;" } else { "background:#f0f0f0; color:#666; border:1px solid #ddd;" }
                        ),
                        onclick: move |_| { show_dead.set(!show_dead()); load_profiles(); },
                        // Tombstone Emoji
                        "🪦"
                    }
                    button {
                        class: "btn-success",
                        style: "padding: 10px 16px; font-size: 16px; font-weight: 500;",
                        onclick: move |_| on_navigate.call(Screen::AddProfile),
                        "+ Neu"
                    }
                }
            }

            // Search & Filter
            div {
                style: "margin: 12px 0 16px;",
                input {
                    style: "width: 100%; padding: 14px 16px; font-size: 16px; border: 2px solid #e0e0e0; border-radius: 10px; background: white; margin-bottom: 12px;",
                    r#type: "text",
                    placeholder: "🔍 Nach Name suchen...",
                    value: "{search_filter}",
                    oninput: move |e| {
                        search_filter.set(e.value());
                        load_profiles();
                    },
                }
            }

            // Profile Grid
            if profiles().is_empty() {
                div {
                    style: "text-align: center; padding: 40px; color: #999;",
                    "Keine Profile vorhanden"
                }
            } else {
                div {
                    class: "profile-grid",
                    for profile in profiles() {
                        ProfileCard {
                            profile: profile.clone(),
                            on_click: move |_| {
                                if let Some(id) = profile.id {
                                    on_navigate.call(Screen::ProfileDetail(id));
                                }
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProfileCard(profile: Wachtel, on_click: EventHandler<()>) -> Element {
    let profile_id = profile.id.unwrap_or(0);

    // Lade Profilfoto über photo_service
    let image_data = use_resource(move || async move {
        if let Ok(conn) = database::init_database() {
            if let Ok(Some(photo)) = services::photo_service::get_profile_photo(&conn, profile_id) {
                let path = photo.thumbnail_path.unwrap_or(photo.path);
                return image_processing::image_path_to_data_url(&path).ok();
            }
        }
        None
    });

    // Load current status from events
    let mut current_status = use_signal(|| None::<crate::models::EventType>);
    use_effect(move || {
        if let Ok(conn) = database::init_database() {
            if let Ok(status) =
                services::profile_service::get_profile_current_status(&conn, profile_id)
            {
                current_status.set(status);
            }
        }
    });

    // Convert ring color to light version for overlay background
    let overlay_bg = if let Some(ring_color) = &profile.ring_color {
        get_light_color_for(ring_color)
    } else {
        "rgba(255, 255, 255, 0.9)".to_string()
    };

    // Image Data URL (Base64)
    // let has_image = image_data().is_some(); // aktuell nicht genutzt

    rsx! {
        div {
            class: "profile-card",
            onclick: move |_| on_click.call(()),

            // Square Image Container
            div {
                class: "profile-image",
                if let Some(data_url) = image_data() {
                    // data_url ist ein String, direkt zuweisen statt Format-Interpolation
                    img {
                        src: data_url,
                        alt: profile.name.clone(),
                        style: "width: 100%; height: 100%; object-fit: cover;"
                    }
                } else {
                    div {
                        class: "profile-image-placeholder",
                        "🐦"
                    }
                }

                // Overlay with name and gender
                div {
                    class: "profile-overlay",
                    style: format!("background: {};", overlay_bg),
                    div {
                        class: "profile-name",
                        "{profile.name}"
                    }
                    div {
                        class: "profile-gender",
                        "{profile.gender.display_name()}"
                    }
                }

                // Status Overlay Emoji (top right corner)
                {
                    if let Some(status) = current_status() {
                        match status {
                            crate::models::EventType::Krank => rsx! {
                                div {
                                    style: "position: absolute; top: 8px; right: 8px; font-size: 32px; background: rgba(255,255,255,0.9); border-radius: 50%; width: 48px; height: 48px; display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 8px rgba(0,0,0,0.3);",
                                    "🤒"
                                }
                            },
                            crate::models::EventType::MarkiertZumSchlachten => rsx! {
                                div {
                                    style: "position: absolute; top: 8px; right: 8px; font-size: 32px; background: rgba(255,255,255,0.9); border-radius: 50%; width: 48px; height: 48px; display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 8px rgba(0,0,0,0.3);",
                                    "🥩"
                                }
                            },
                            crate::models::EventType::Gestorben => rsx! {
                                div {
                                    style: "position: absolute; top: 8px; right: 8px; font-size: 32px; background: rgba(255,255,255,0.9); border-radius: 50%; width: 48px; height: 48px; display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 8px rgba(0,0,0,0.3);",
                                    "🪦"
                                }
                            },
                            crate::models::EventType::Geschlachtet => rsx! {
                                div {
                                    style: "position: absolute; top: 8px; right: 8px; font-size: 32px; background: rgba(255,255,255,0.9); border-radius: 50%; width: 48px; height: 48px; display: flex; align-items: center; justify-content: center; box-shadow: 0 2px 8px rgba(0,0,0,0.3);",
                                    "🥩"
                                }
                            },
                            _ => rsx! { }
                        }
                    } else {
                        rsx! { }
                    }
                }

            }
        }
    }
}

/// Helper function to convert color names to light versions
fn get_light_color_for(color: &Ringfarbe) -> String {
    match color {
        Ringfarbe::Rot => "rgba(255, 200, 200, 0.9)".to_string(),
        Ringfarbe::Dunkelblau => "rgba(200, 210, 245, 0.9)".to_string(),
        Ringfarbe::Hellblau => "rgba(210, 230, 255, 0.9)".to_string(),
        Ringfarbe::Gruen => "rgba(200, 255, 200, 0.9)".to_string(),
        Ringfarbe::Gelb => "rgba(255, 255, 200, 0.9)".to_string(),
        Ringfarbe::Orange => "rgba(255, 230, 200, 0.9)".to_string(),
        Ringfarbe::Lila => "rgba(230, 200, 255, 0.9)".to_string(),
        Ringfarbe::Rosa => "rgba(255, 200, 230, 0.9)".to_string(),
        Ringfarbe::Schwarz => "rgba(220, 220, 220, 0.9)".to_string(),
        Ringfarbe::Weiss => "rgba(255, 255, 255, 0.9)".to_string(),
    }
}
