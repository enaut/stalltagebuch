use dioxus::prelude::*;
use rusqlite::Connection;

mod camera;
mod components;
mod database;
mod error;
mod filesystem;
mod image_processing;
mod models;
mod services;

use components::{
    AddProfileScreen, EggHistoryScreen, EggTrackingScreen, EventAdd, EventEditScreen, HomeScreen,
    NavigationBar, ProfileDetailScreen, ProfileEditScreen, ProfileListScreen, StatisticsScreen,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

/// Screen-Navigation für die App
#[derive(Clone, PartialEq, Debug)]
pub enum Screen {
    Home,
    ProfileList,
    ProfileDetail(i64),
    ProfileEdit(i64),
    AddProfile,
    EventAdd {
        wachtel_id: i64,
        wachtel_name: String,
    },
    EventEdit {
        event_id: i64,
        wachtel_id: i64,
    },
    EggTracking,
    EggHistory,
    Statistics,
    Settings,
}

#[component]
fn App() -> Element {
    let mut current_screen = use_signal(|| Screen::Home);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        div {
            style: "display: flex; flex-direction: column; height: 100vh; font-family: sans-serif;",

            // Main Content
            div {
                style: "flex: 1; overflow-y: auto;",
                match current_screen() {
                    Screen::Home => rsx! { HomeScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::ProfileList => rsx! { ProfileListScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::ProfileDetail(id) => rsx! { ProfileDetailScreen { wachtel_id: id, on_navigate: move |s| current_screen.set(s) } },
                    Screen::ProfileEdit(id) => rsx! { ProfileEditScreen { wachtel_id: id, on_navigate: move |s| current_screen.set(s) } },
                    Screen::AddProfile => rsx! { AddProfileScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::EventAdd { wachtel_id, wachtel_name } => rsx! { EventAdd { wachtel_id, wachtel_name, on_navigate: move |s| current_screen.set(s) } },
                    Screen::EventEdit { event_id, wachtel_id } => rsx! { EventEditScreen { event_id, wachtel_id, on_navigate: move |s| current_screen.set(s) } },
                    Screen::EggTracking => rsx! { EggTrackingScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::EggHistory => rsx! { EggHistoryScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::Statistics => rsx! { StatisticsScreen { on_navigate: move |s| current_screen.set(s) } },
                    Screen::Settings => rsx! { div { "Einstellungen (TODO)" } },
                }
            }

            // Bottom Navigation Bar
            NavigationBar {
                current_screen: current_screen(),
                on_navigate: move |screen| current_screen.set(screen),
            }
        }
    }
}

// Legacy PoC component - keeping for reference
#[allow(dead_code)]
#[component]
pub fn MobileTestApp() -> Element {
    // State
    let mut counter = use_signal(|| 0);
    let mut db_status = use_signal(|| "Not initialized".to_string());
    let mut db_entries = use_signal(|| Vec::<(i64, String, String)>::new());
    let mut fs_status = use_signal(|| "Not tested".to_string());
    let mut db_conn = use_signal(|| None::<Connection>);

    // Initialize database on mount
    use_effect(move || match database::init_database() {
        Ok(conn) => {
            db_status.set("✅ Database initialized".to_string());
            db_conn.set(Some(conn));
        }
        Err(e) => {
            db_status.set(format!("❌ DB Error: {}", e));
        }
    });

    rsx! {
        div {
            style: "padding: 20px; max-width: 600px; margin: 0 auto; font-family: sans-serif;",

            h1 {
                style: "color: #0066cc; text-align: center;",
                "🥚 Stalltagebuch PoC"
            }

            // Counter Test (Signal State)
            div {
                style: "background: #f0f0f0; padding: 20px; margin: 20px 0; border-radius: 8px;",
                h2 { "📊 Counter Test (Signals)" }
                p {
                    style: "font-size: 24px; font-weight: bold; color: #333;",
                    "Count: {counter}"
                }
                div {
                    style: "display: flex; gap: 10px; flex-wrap: wrap;",
                    button {
                        style: "padding: 10px 20px; font-size: 16px; background: #0066cc; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| *counter.write() += 1,
                        "➕ Increment"
                    }
                    button {
                        style: "padding: 10px 20px; font-size: 16px; background: #cc6600; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| *counter.write() -= 1,
                        "➖ Decrement"
                    }
                }
            }

            // SQLite Test
            div {
                style: "background: #e8f4f8; padding: 20px; margin: 20px 0; border-radius: 8px;",
                h2 { "� SQLite Test" }
                p {
                    style: "color: #666; margin: 10px 0;",
                    "{db_status}"
                }

                div {
                    style: "display: flex; gap: 10px; flex-wrap: wrap; margin: 10px 0;",
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #00aa00; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            if let Some(ref conn) = *db_conn.read() {
                                let content = format!("Entry at {}", chrono::Utc::now().format("%H:%M:%S"));
                                match database::add_test_entry(conn, &content) {
                                    Ok(id) => {
                                        db_status.set(format!("✅ Added entry #{}", id));
                                        // Reload entries
                                        if let Ok(entries) = database::get_test_entries(conn) {
                                            db_entries.set(entries);
                                        }
                                    }
                                    Err(e) => db_status.set(format!("❌ Error: {}", e)),
                                }
                            }
                        },
                        "➕ Add Entry"
                    }
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #0066cc; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            if let Some(ref conn) = *db_conn.read() {
                                match database::get_test_entries(conn) {
                                    Ok(entries) => {
                                        db_entries.set(entries);
                                        db_status.set("✅ Entries loaded".to_string());
                                    }
                                    Err(e) => db_status.set(format!("❌ Error: {}", e)),
                                }
                            }
                        },
                        "🔄 Load Entries"
                    }
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #cc0000; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            if let Some(ref conn) = *db_conn.read() {
                                match database::clear_test_entries(conn) {
                                    Ok(_) => {
                                        db_entries.set(Vec::new());
                                        db_status.set("✅ All entries cleared".to_string());
                                    }
                                    Err(e) => db_status.set(format!("❌ Error: {}", e)),
                                }
                            }
                        },
                        "🗑️ Clear All"
                    }
                }

                div {
                    style: "margin-top: 15px;",
                    h3 {
                        style: "font-size: 14px; color: #666;",
                        "Entries: {db_entries.read().len()}"
                    }
                    for (id, content, created_at) in db_entries.read().iter() {
                        div {
                            style: "background: white; padding: 8px; margin: 5px 0; border-radius: 4px; font-size: 12px;",
                            "#{id}: {content} ({created_at})"
                        }
                    }
                }
            }

            // File System Test
            div {
                style: "background: #f8f0e8; padding: 20px; margin: 20px 0; border-radius: 8px;",
                h2 { "📁 File System Test" }
                p {
                    style: "color: #666; margin: 10px 0;",
                    "{fs_status}"
                }

                div {
                    style: "display: flex; gap: 10px; flex-wrap: wrap;",
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #aa6600; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            let test_data = b"Test file content from Stalltagebuch PoC";
                            match filesystem::write_test_file("test.txt", test_data) {
                                Ok(path) => fs_status.set(format!("✅ File written to: {}", path.display())),
                                Err(e) => fs_status.set(format!("❌ Write error: {}", e)),
                            }
                        },
                        "💾 Write File"
                    }
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #0066aa; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            match filesystem::read_test_file("test.txt") {
                                Ok(data) => {
                                    let content = String::from_utf8_lossy(&data);
                                    fs_status.set(format!("✅ Read {} bytes: {}", data.len(), content));
                                }
                                Err(e) => fs_status.set(format!("❌ Read error: {}", e)),
                            }
                        },
                        "� Read File"
                    }
                    button {
                        style: "padding: 10px 20px; font-size: 14px; background: #6600aa; color: white; border: none; border-radius: 5px; cursor: pointer;",
                        onclick: move |_| {
                            match filesystem::list_files() {
                                Ok(files) => fs_status.set(format!("✅ Files: {}", files.join(", "))),
                                Err(e) => fs_status.set(format!("❌ List error: {}", e)),
                            }
                        },
                        "📋 List Files"
                    }
                }
            }

            // Platform Info
            div {
                style: "background: #f0f0f0; padding: 15px; margin: 20px 0; border-radius: 8px; font-size: 12px; color: #666;",
                h3 { "ℹ️ Platform Info" }
                p { "OS: {std::env::consts::OS}" }
                p { "Arch: {std::env::consts::ARCH}" }
                p {
                    "DB Path: {database::get_database_path().display()}"
                }
                p {
                    "Data Dir: {filesystem::get_app_data_dir().display()}"
                }
            }
        }
    }
}
