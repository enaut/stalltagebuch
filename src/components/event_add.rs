use crate::database;
use crate::models::quail_event::EventType;
use crate::services::event_service;
use crate::Screen;
use chrono::NaiveDate;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn EventAdd(
    quail_id: String,
    quail_name: String,
    on_navigate: EventHandler<Screen>,
) -> Element {
    let mut event_type = use_signal(|| EventType::Alive);
    let mut event_date = use_signal(|| {
        chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    });
    let mut notes = use_signal(|| String::new());
    let photos = use_signal(|| Vec::<String>::new());
    let error_message = use_signal(|| None::<String>);

    let quail_id_for_save = quail_id.clone();
    let error_message_signal = error_message.clone();
    let event_date_signal = event_date.clone();
    let notes_signal = notes.clone();
    let event_type_signal = event_type.clone();
    let photos_signal = photos.clone();
    let on_save = move |_| {
        let quail_id = quail_id_for_save.clone();
        let mut error_message = error_message_signal.clone();
        let event_date = event_date_signal.clone();
        let notes = notes_signal.clone();
        let event_type = event_type_signal.clone();
        let photos = photos_signal.clone();
        spawn(async move {
            match database::init_database() {
                Ok(conn) => {
                    // Parse date
                    let parsed_date = match NaiveDate::parse_from_str(&event_date(), "%Y-%m-%d") {
                        Ok(date) => date,
                        Err(_) => {
                            error_message.set(Some(t!("error-invalid-date")));
                            return;
                        }
                    };

                    let notes_opt = if notes().is_empty() {
                        None
                    } else {
                        Some(notes())
                    };

                    if let Ok(q_uuid) = uuid::Uuid::parse_str(&quail_id) {
                        match event_service::create_event(
                            &conn,
                            q_uuid,
                            event_type(),
                            parsed_date,
                            notes_opt,
                        ) {
                            Ok(event_id) => {
                                // Save photos for this event
                                for photo_path in photos() {
                                    let thumbnail_opt =
                                        crate::image_processing::create_thumbnail(&photo_path).ok();
                                    let _ = crate::services::photo_service::add_event_photo(
                                        &conn,
                                        event_id,
                                        photo_path,
                                        thumbnail_opt,
                                    );
                                }
                                on_navigate.call(Screen::ProfileDetail(quail_id.clone()));
                            }
                            Err(e) => {
                                error_message
                                    .set(Some(t!("error-event-save", error: e.to_string())));
                            }
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(t!("error-database-detail", error: e.to_string())));
                }
            }
        });
    };

    rsx! {
        div { class: "container", style: "padding: 20px;",

            h2 { { t!("event-add-title") } }
            p { style: "color: #666; margin-bottom: 20px;", { t!("event-add-for", name: quail_name.clone()) } }

            if let Some(error) = error_message() {
                div {
                    class: "error-message",
                    style: "background-color: #fee; color: #c00; padding: 10px; margin-bottom: 20px; border-radius: 4px;",
                    "{error}"
                }
            }

            div { class: "form-group", style: "margin-bottom: 20px;",

                label { style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    { t!("field-event-type") }
                }
                select {
                    value: "{event_type().as_str()}",
                    onchange: move |e| {
                        let value = e.value();
                        let et = EventType::from_str(value.as_str());
                        event_type.set(et);
                    },
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                    option { value: "born", { t!("event-type-born") } }
                    option { value: "alive", { t!("event-type-alive") } }
                    option { value: "sick", { t!("event-type-sick") } }
                    option { value: "healthy", { t!("event-type-healthy") } }
                    option { value: "marked_for_slaughter", { t!("event-type-marked") } }
                    option { value: "slaughtered", { t!("event-type-slaughtered") } }
                    option { value: "died", { t!("event-type-died") } }
                }
            }

            div { class: "form-group", style: "margin-bottom: 20px;",

                label { style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    { t!("field-date") }
                }
                input {
                    r#type: "date",
                    value: "{event_date}",
                    oninput: move |e| event_date.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                }
            }

            div { class: "form-group", style: "margin-bottom: 20px;",

                label { style: "display: block; margin-bottom: 8px; font-weight: bold;",
                    { t!("field-notes-optional") }
                }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px; min-height: 100px;",
                    placeholder: t!("placeholder-event-notes"),
                }
            }

            div { class: "button-group", style: "display: flex; gap: 10px;",

                button {
                    onclick: on_save,
                    style: "flex: 1; padding: 12px; background-color: #4CAF50; color: white; border: none; border-radius: 4px; font-size: 16px; cursor: pointer;",
                    { t!("action-save") }
                }

                button {
                    onclick: {
                        let quail_id_for_cancel = quail_id.clone();
                        move |_| on_navigate.call(Screen::ProfileDetail(quail_id_for_cancel.clone()))
                    },
                    style: "flex: 1; padding: 12px; background-color: #f44336; color: white; border: none; border-radius: 4px; font-size: 16px; cursor: pointer;",
                    { t!("action-cancel") }
                }
            }
        }
    }
}
