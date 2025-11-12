use crate::{database, models::EggRecord, services, Screen};
use chrono::Local;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn EggTrackingScreen(date: Option<String>, on_navigate: EventHandler<Screen>) -> Element {
    let mut date_str = use_signal(|| {
        date.clone()
            .unwrap_or_else(|| Local::now().format("%Y-%m-%d").to_string())
    });
    let mut total_eggs = use_signal(|| String::new());
    let mut notes = use_signal(|| String::new());
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);
    let mut existing_record = use_signal(|| None::<EggRecord>);

    // Load existing record for selected date
    let mut load_record = move || {
        let date_value = date_str();
        match database::init_database() {
            Ok(conn) => {
                match services::get_egg_record(&conn, &date_value) {
                    Ok(record) => {
                        total_eggs.set(record.total_eggs.to_string());
                        notes.set(record.notes.clone().unwrap_or_default());
                        existing_record.set(Some(record));
                    }
                    Err(_) => {
                        // Kein Eintrag für dieses Datum
                        total_eggs.set(String::new());
                        notes.set(String::new());
                        existing_record.set(None);
                    }
                }
            }
            Err(e) => {
                error.set(Some(t!("error-database-detail", error: e.to_string())));
            }
        }
    };

    // Load on mount and when date changes
    use_effect(move || {
        load_record();
    });

    let mut handle_submit = move || {
        error.set(None);
        success.set(false);

        // Validate eggs count
        let eggs_str = total_eggs();
        let eggs_trimmed = eggs_str.trim();
        if eggs_trimmed.is_empty() {
            error.set(Some(t!("error-eggs-count-empty")));
            return;
        }

        let eggs_count = match eggs_trimmed.parse::<i32>() {
            Ok(n) if n >= 0 => n,
            Ok(_) => {
                error.set(Some(t!("error-eggs-count-negative")));
                return;
            }
            Err(_) => {
                error.set(Some(t!("error-eggs-count-invalid")));
                return;
            }
        };

        // Parse date
        let date_value = date_str();
        let date_trimmed = date_value.trim();
        let record_date = match chrono::NaiveDate::parse_from_str(date_trimmed, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => {
                error.set(Some(t!("error-date-format")));
                return;
            }
        };

        // Notes
        let notes_value = notes();
        let notes_trimmed = notes_value.trim();
        let notes_opt = if notes_trimmed.is_empty() {
            None
        } else {
            Some(notes_trimmed.to_string())
        };

        // Save to database
        match database::init_database() {
            Ok(conn) => {
                let result = if eggs_count == 0 {
                    // Delete record if eggs count is 0
                    if existing_record().is_some() {
                        services::delete_egg_record(&conn, &date_trimmed)
                    } else {
                        // Nothing to delete
                        Ok(())
                    }
                } else if existing_record().is_some() {
                    // Update existing record
                    let mut record = existing_record().unwrap();
                    record.total_eggs = eggs_count;
                    record.notes = notes_opt;
                    services::update_egg_record(&conn, &record)
                } else {
                    // Create new record
                    let record = EggRecord::new(record_date, eggs_count);
                    let mut record = record;
                    record.notes = notes_opt;
                    services::add_egg_record(&conn, &record).map(|_| ())
                };

                match result {
                    Ok(_) => {
                        success.set(true);
                        load_record(); // Reload to update existing_record state
                                       // Nach erfolgreichem Speichern zur Historie zurückkehren
                        on_navigate.call(Screen::EggHistory);
                    }
                    Err(e) => {
                        error.set(Some(t!("error-save", error: e.to_string())));
                    }
                }
            }
            Err(e) => {
                error.set(Some(t!("error-database-detail", error: e.to_string())));
            }
        }
    };

    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div {
                style: "display: flex; align-items: center; margin-bottom: 24px;",
                h1 {
                    style: "color: #0066cc; font-size: 24px; font-weight: 700; margin: 0;",
                    "🥚 ",
                    { t!("egg-tracking-title") }
                }
            }

            // Error Message
            if let Some(err) = error() {
                div {
                    style: "background: #fee; border: 1px solid #fcc; color: #c33; padding: 12px; margin-bottom: 16px; border-radius: 8px; font-size: 14px;",
                    "⚠️ ",
                    { err }
                }
            }

            // Success Message
            if success() {
                div {
                    style: "background: #efe; border: 1px solid #cfc; color: #3a3; padding: 12px; margin-bottom: 16px; border-radius: 8px; font-size: 14px;",
                    "✅ ",
                    { t!("egg-tracking-success") }
                }
            }

            // Status
            if existing_record().is_some() {
                div {
                    style: "background: #e8f4f8; padding: 12px; margin-bottom: 16px; border-radius: 8px; border-left: 3px solid #0066cc; font-size: 14px; color: #333;",
                    "📝 ",
                    { t!("egg-tracking-exists-warning") }
                }
            }

            // Form
            div {
                class: "card",

                // Date Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        { t!("field-date-required") }
                    }
                    input {
                        r#type: "date",
                        class: "input",
                        value: "{date_str}",
                        oninput: move |e| {
                            date_str.set(e.value());
                            load_record();
                        },
                        autofocus: true,
                    }
                    p {
                        style: "margin: 4px 0 0 0; font-size: 12px; color: #666;",
                        { t!("field-date-format-hint") }
                    }
                }

                // Total Eggs Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        { t!("field-eggs-count-required") }
                    }
                    input {
                        r#type: "number",
                        class: "input",
                        placeholder: t!("field-eggs-count-placeholder"),
                        min: "0",
                        value: "{total_eggs}",
                        oninput: move |e| total_eggs.set(e.value()),
                    }
                }

                // Notes Field
                div {
                    style: "margin-bottom: 20px;",
                    label {
                        style: "display: block; margin-bottom: 6px; font-weight: 600; color: #333; font-size: 14px;",
                        { t!("field-notes") }
                    }
                    textarea {
                        class: "input",
                        style: "min-height: 80px; resize: vertical; font-family: inherit;",
                        placeholder: t!("field-notes-placeholder"),
                        value: "{notes}",
                        oninput: move |e| notes.set(e.value()),
                    }
                }

                // Action Buttons
                div {
                    style: "display: flex; gap: 12px; margin-top: 24px;",
                    button {
                        class: "btn-success",
                        style: "flex: 1; padding: 14px;",
                        onclick: move |_| handle_submit(),
                        "💾 ",
                        if existing_record().is_some() {
                            { t!("action-update") }
                        } else {
                            { t!("action-save") }
                        }
                    }
                }
            }

            // Quick Links
            div {
                style: "margin-top: 16px; display: flex; gap: 12px;",
                button {
                    class: "btn-primary",
                    style: "flex: 1; padding: 12px;",
                    onclick: move |_| on_navigate.call(Screen::EggHistory),
                    "📋 ",
                    { t!("egg-tracking-show-history") }
                }
            }
        }
    }
}
