use dioxus::prelude::*;
use crate::{database, services, models::EggRecord, Screen};

#[component]
pub fn EggHistoryScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut records = use_signal(|| Vec::<EggRecord>::new());
    let mut status_message = use_signal(|| String::new());
    let mut show_confirm_delete = use_signal(|| None::<String>);
    
    // Load records
    let mut load_records = move || {
        match database::init_database() {
            Ok(conn) => {
                match services::list_egg_records(&conn, None, None) {
                    Ok(list) => {
                        records.set(list);
                        status_message.set(format!("✅ {} Einträge geladen", records().len()));
                    }
                    Err(e) => {
                        status_message.set(format!("❌ Fehler: {}", e));
                    }
                }
            }
            Err(e) => {
                status_message.set(format!("❌ DB-Fehler: {}", e));
            }
        }
    };
    
    // Load on mount
    use_effect(move || {
        load_records();
    });
    
    let mut handle_delete = move |date: String| {
        match database::init_database() {
            Ok(conn) => {
                if let Err(e) = services::delete_egg_record(&conn, &date) {
                    status_message.set(format!("❌ Löschen fehlgeschlagen: {}", e));
                } else {
                    show_confirm_delete.set(None);
                    load_records();
                }
            }
            Err(e) => {
                status_message.set(format!("❌ DB-Fehler: {}", e));
            }
        }
    };
    
    rsx! {
        div {
            style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",
            
            // Header
            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; padding-top: 8px;",
                h1 {
                    style: "color: #0066cc; margin: 0; font-size: 24px; font-weight: 700;",
                    "📋 Eier-Historie"
                }
                button {
                    class: "btn-success",
                    style: "padding: 10px 20px; font-size: 16px; font-weight: 500;",
                    onclick: move |_| on_navigate.call(Screen::EggTracking),
                    "+ Neu"
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
                    "Keine Einträge vorhanden"
                }
            } else {
                for record in records() {
                    EggRecordCard {
                        record: record.clone(),
                        on_edit: move |_date| on_navigate.call(Screen::EggTracking),
                        on_delete_confirm: move |date| show_confirm_delete.set(Some(date)),
                        show_confirm: show_confirm_delete() == Some(record.record_date.format("%Y-%m-%d").to_string()),
                        on_delete: move |date| handle_delete(date),
                        on_cancel_delete: move || show_confirm_delete.set(None),
                    }
                }
            }
        }
    }
}

#[component]
fn EggRecordCard(
    record: EggRecord,
    on_edit: EventHandler<String>,
    on_delete_confirm: EventHandler<String>,
    show_confirm: bool,
    on_delete: EventHandler<String>,
    on_cancel_delete: EventHandler<()>,
) -> Element {
    let date_str = record.record_date.format("%Y-%m-%d").to_string();
    let display_date = record.record_date.format("%d.%m.%Y").to_string();
    use chrono::Datelike;
    let weekday_num = record.record_date.weekday().num_days_from_monday();
    let weekday = match weekday_num {
        0 => "Mo",
        1 => "Di",
        2 => "Mi",
        3 => "Do",
        4 => "Fr",
        5 => "Sa",
        6 => "So",
        _ => "",
    };
    
    rsx! {
        div {
            class: "card",
            style: "padding: 16px; margin: 8px 0; border-left: 4px solid #ff8c00;",
            
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
                            "🥚 {record.total_eggs} Eier"
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
                    style: "display: flex; gap: 8px; margin-left: 12px;",
                    
                    if show_confirm {
                        button {
                            class: "btn-danger",
                            style: "padding: 8px 16px; font-size: 14px; font-weight: 600;",
                            onclick: move |_| on_delete.call(date_str.clone()),
                            "✔ Bestätigen"
                        }
                        button {
                            style: "padding: 8px 16px; background: #e0e0e0; color: #666; font-size: 14px; font-weight: 600;",
                            onclick: move |_| on_cancel_delete.call(()),
                            "✕"
                        }
                    } else {
                        button {
                            style: "padding: 8px 16px; background: #ffe6e6; color: #cc0000; font-size: 20px; border-radius: 8px;",
                            onclick: move |_| on_delete_confirm.call(date_str.clone()),
                            "🗑️"
                        }
                    }
                }
            }
        }
    }
}
