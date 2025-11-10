use crate::database;
use crate::models::wachtel_event::EventType;
use crate::services::event_service;
use crate::Screen;
use chrono::NaiveDate;
use dioxus::prelude::*;

#[component]
pub fn EventAdd(
    wachtel_id: i64,
    wachtel_name: String,
    on_navigate: EventHandler<Screen>,
) -> Element {
    let mut event_type = use_signal(|| EventType::AmLeben);
    let mut event_date = use_signal(|| {
        chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    });
    let mut notes = use_signal(|| String::new());
    let mut photos = use_signal(|| Vec::<String>::new());
    let mut error_message = use_signal(|| None::<String>);

    let on_save = move |_| {
        spawn(async move {
            match database::init_database() {
                Ok(conn) => {
                    // Parse date
                    let parsed_date = match NaiveDate::parse_from_str(&event_date(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(_) => {
                            error_message.set(Some("Ungültiges Datumsformat".to_string()));
                            return;
                        }
                    };

                    let notes_opt = if notes().is_empty() {
                        None
                    } else {
                        Some(notes())
                    };

                    match event_service::create_event(
                        &conn,
                        wachtel_id,
                        event_type(),
                        parsed_date,
                        notes_opt,
                    ) {
                        Ok(event_id) => {
                            // Speichere Fotos für dieses Event
                            for photo_path in photos() {
                                // Optional: Generiere Thumbnail
                                let thumbnail_opt =
                                    crate::image_processing::create_thumbnail(&photo_path).ok();
                                let _ = crate::services::photo_service::add_event_photo(
                                    &conn,
                                    event_id,
                                    photo_path,
                                    thumbnail_opt,
                                );
                            }
                            on_navigate.call(Screen::ProfileDetail(wachtel_id));
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Fehler beim Speichern: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("Datenbankfehler: {}", e)));
                }
            }
        });
    };

    rsx! {
        div {
            class: "container",
            style: "padding: 20px;",

            h2 { "Ereignis hinzufügen" }
            p { style: "color: #666; margin-bottom: 20px;", "für {wachtel_name}" }

            if let Some(error) = error_message() {
                div {
                    class: "error-message",
                    style: "background-color: #fee; color: #c00; padding: 10px; margin-bottom: 20px; border-radius: 4px;",
                    "{error}"
                }
            }

            div {
                class: "form-group",
                style: "margin-bottom: 20px;",

                label {
                    style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    "Ereignistyp"
                }
                select {
                    value: "{event_type():?}",
                    onchange: move |e| {
                        let value = e.value();
                        let et = match value.as_str() {
                            "Geboren" => EventType::Geboren,
                            "AmLeben" => EventType::AmLeben,
                            "Krank" => EventType::Krank,
                            "Gesund" => EventType::Gesund,
                            "MarkiertZumSchlachten" => EventType::MarkiertZumSchlachten,
                            "Geschlachtet" => EventType::Geschlachtet,
                            "Gestorben" => EventType::Gestorben,
                            _ => EventType::AmLeben,
                        };
                        event_type.set(et);
                    },
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",

                    option { value: "Geboren", "🐣 Geboren" }
                    option { value: "AmLeben", "✅ Am Leben" }
                    option { value: "Krank", "🤒 Krank" }
                    option { value: "Gesund", "💪 Gesund" }
                    option { value: "MarkiertZumSchlachten", "🥩 Markiert zum Schlachten" }
                    option { value: "Geschlachtet", "🥩 Geschlachtet" }
                    option { value: "Gestorben", "🪦 Gestorben" }
                }
            }

            div {
                class: "form-group",
                style: "margin-bottom: 20px;",

                label {
                    style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    "Datum"
                }
                input {
                    r#type: "date",
                    value: "{event_date}",
                    oninput: move |e| event_date.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                }
            }

            div {
                class: "form-group",
                style: "margin-bottom: 20px;",

                label {
                    style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    "Notizen (optional)"
                }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px; min-height: 100px;",
                    placeholder: "Weitere Informationen zum Ereignis..."
                }
            }

            div {
                class: "button-group",
                style: "display: flex; gap: 10px;",

                button {
                    onclick: on_save,
                    style: "flex: 1; padding: 12px; background-color: #4CAF50; color: white; border: none; border-radius: 4px; font-size: 16px; cursor: pointer;",
                    "Speichern"
                }

                button {
                    onclick: move |_| on_navigate.call(Screen::ProfileDetail(wachtel_id)),
                    style: "flex: 1; padding: 12px; background-color: #f44336; color: white; border: none; border-radius: 4px; font-size: 16px; cursor: pointer;",
                    "Abbrechen"
                }
            }
        }
    }
}
