use crate::database;
use crate::services;
use crate::Screen;
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn HomeScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut db_status = use_signal(|| Err(t!("status-initializing")));
    let mut profile_count = use_signal(|| 0i32);

    // Initialize database on mount
    use_effect(move || match database::init_database() {
        Ok(conn) => match services::count_profiles(&conn) {
            Ok(count) => {
                profile_count.set(count);
                db_status.set(Ok(format!("✅ {}", t!( "status-db-ready", count: count))));
            }
            Err(e) => {
                db_status.set(Err(format!("⚠️ {}", e)));
            }
        },
        Err(e) => {
            db_status.set(Err(format!(
                "❌ {}",
                t!( "status-db-error", error: e.to_string())
            )));
        }
    });

    rsx! {
        div { style: "padding: 16px; max-width: 600px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",
            h1 { style: "color: #0066cc; text-align: center; margin-bottom: 24px; margin-top: 48px; font-size: 28px; font-weight: 700;",
                {format!("🥚 {}", t!("app-title"))}
            }
            if let Err(db_status) = db_status() {
                // Status Card
                div { class: "card-header",
                    h2 { style: "margin: 0 0 12px 0; font-size: 18px; color: #333;",
                        "Status"
                    }
                    p { style: "font-size: 14px; color: #555; margin: 0;", "{db_status}" }
                }
            }
            // Quick Actions
            div { class: "card", style: "margin-bottom: 128px;",
                h2 { style: "margin: 0 0 16px 0; font-size: 18px; color: #333;", "Schnellzugriff" }
                div { style: "display: flex; flex-direction: column; gap: 12px;",
                    button {
                        class: "btn-primary",
                        style: "padding: 16px; font-size: 16px; display: flex; align-items: center; justify-content: center;",
                        onclick: move |_| on_navigate.call(Screen::ProfileList),
                        {format!("🐦 {}", t!("profile-list-title"))}
                    }
                    button {
                        class: "btn-success",
                        style: "padding: 16px; font-size: 16px; display: flex; align-items: center; justify-content: center;",
                        onclick: move |_| on_navigate.call(Screen::EggTracking(None)),
                        {format!("🥚 {}", t!("egg-tracking-title"))}
                    }
                    button {
                        style: "padding: 16px; font-size: 16px; background: #ff8c00; color: white; display: flex; align-items: center; justify-content: center;",
                        onclick: move |_| on_navigate.call(Screen::Statistics),
                        {format!("📊 {}", t!("stats-title"))}
                    }
                }
            }
            // Settings button
            div { class: "card", style: "margin-bottom: 16px;",
                button {
                    class: "btn-secondary",
                    style: "width: 100%; padding: 16px; font-size: 16px; display: flex; align-items: center; justify-content: center;",
                    onclick: move |_| on_navigate.call(Screen::Settings),
                    {format!("⚙️ {}", t!("settings-title"))}
                }
            }

            // Info Card
            div { style: "background: #f8f9fa; padding: 16px; margin: 16px 0; border-radius: 8px; border: 1px solid #e0e0e0;",
                h3 { style: "margin: 0 0 12px 0; font-size: 14px; color: #666; font-weight: 600;",
                    "ℹ️ System-Info"
                }
                p { style: "font-size: 12px; color: #666; margin: 4px 0;",
                    "OS: {std::env::consts::OS}"
                }
                p { style: "font-size: 12px; color: #666; margin: 4px 0;",
                    "Arch: {std::env::consts::ARCH}"
                }
                p { style: "font-size: 11px; color: #888; margin: 4px 0; word-break: break-all;",
                    {t!("info-db-path", path : database::get_database_path().display().to_string())} // DB: {path}
                }
            }
        }
    }
}
