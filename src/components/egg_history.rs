use crate::{database, models::EggRecord, services, Screen};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn EggHistoryScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut records = use_signal(|| Vec::<EggRecord>::new());
    let mut status_message = use_signal(|| String::new());

    // Load records
    let mut load_records = move || match database::init_database() {
        Ok(conn) => match services::list_egg_records(&conn, None, None) {
            Ok(list) => {
                records.set(list);
                status_message.set(format!(
                    "✅ {}",
                    t!("egg-history-loaded", count: records().len())
                ));
            }
            Err(e) => {
                status_message.set(format!(
                    "❌ {}",
                    t!("error-load-failed", error: e.to_string())
                ));
            }
        },
        Err(e) => {
            status_message.set(format!("❌ {}", t!("error-database", error: e.to_string())));
        }
    };

    // Load on mount
    use_effect(move || {
        load_records();
    });

    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; padding-top: 8px;",
                h1 {
                    style: "color: #0066cc; margin: 0; font-size: 24px; font-weight: 700;",
                    "📋 ",
                    { t!("egg-history-title") }
                }
                button {
                    class: "btn-success",
                    style: "padding: 10px 20px; font-size: 16px; font-weight: 500;",
                    onclick: move |_| on_navigate.call(Screen::EggTracking(None)),
                    "+ ",
                    { t!("action-new") }
                }
            }

            // Status
            if !status_message().is_empty() {
                div {
                    style: "padding: 12px 16px; background: #e8f4f8; border-radius: 8px; color: #333; font-size: 14px; margin-bottom: 12px; border-left: 3px solid #0066cc;",
                    "{status_message}"
                }
            }

            // Records List
            if records().is_empty() {
                div {
                    style: "text-align: center; padding: 40px; color: #999;",
                    { t!("egg-history-empty") }
                }
            } else {
                for record in records() {
                    EggRecordCard {
                        record: record.clone(),
                        on_edit: move |date| on_navigate.call(Screen::EggTracking(Some(date))),
                    }
                }
            }
        }
    }
}

#[component]
fn EggRecordCard(record: EggRecord, on_edit: EventHandler<String>) -> Element {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();
    let display_date = record.record_date.format("%d.%m.%Y").to_string();
    use chrono::Datelike;
    let weekday_num = record.record_date.weekday().num_days_from_monday();
    let weekday = match weekday_num {
        0 => t!("weekday-mon"),
        1 => t!("weekday-tue"),
        2 => t!("weekday-wed"),
        3 => t!("weekday-thu"),
        4 => t!("weekday-fri"),
        5 => t!("weekday-sat"),
        6 => t!("weekday-sun"),
        _ => String::new(),
    };

    rsx! {
        div {
            class: "card",
            style: "padding: 16px; margin: 8px 0; border-left: 4px solid #ff8c00; cursor: pointer;",
            onclick: move |_| on_edit.call(date_str.clone()),

            div {
                style: "display: flex; justify-content: space-between; align-items: start;",

                div {
                    style: "flex: 1; min-width: 0;",
                    div {
                        style: "display: flex; align-items: center; gap: 8px; margin-bottom: 8px;",
                        h3 {
                            style: "margin: 0; font-size: 18px; color: #333; font-weight: 600;",
                            "📅 {display_date} ({weekday})"
                        }
                    }
                    div {
                        style: "display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px;",
                        span {
                            style: "display: inline-block; padding: 6px 14px; background: #fff3e0; border-radius: 12px; font-size: 16px; color: #ff8c00; font-weight: 600;",
                            "🥚 ",
                            { t!("egg-history-eggs-count", count: record.total_eggs) }
                        }
                    }
                    if let Some(notes) = &record.notes {
                        if !notes.trim().is_empty() {
                            div {
                                style: "margin-top: 12px; padding: 8px; background: #f8f9fa; border-radius: 6px; font-size: 13px; color: #666;",
                                "💬 {notes}"
                            }
                        }
                    }
                }

                div {
                    style: "margin-left: 12px; color: #999; font-size: 18px;",
                    "✏️"
                }
            }
        }
    }
}
