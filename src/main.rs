use dioxus::prelude::*;
use dioxus_i18n::prelude::*;

mod camera;
mod components;
mod database;
mod error;
mod i18n;
mod image_processing;
mod models;
mod services;

use components::{
    AddProfileScreen, EggHistoryScreen, EggTrackingScreen, EventAdd, EventEditScreen, HomeScreen,
    NavigationBar, ProfileDetailScreen, ProfileEditScreen, ProfileListScreen, SettingsScreen,
    StatisticsScreen,
};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

/// Screen navigation for the app
#[derive(Clone, PartialEq, Debug)]
pub enum Screen {
    Home,
    ProfileList,
    ProfileDetail(i64),
    ProfileEdit(i64),
    AddProfile,
    EventAdd { quail_id: i64, quail_name: String },
    EventEdit { event_id: i64, quail_id: i64 },
    EggTracking(Option<String>), // Date in YYYY-MM-DD format
    EggHistory,
    Statistics,
    Settings,
}

#[component]
fn App() -> Element {
    let mut current_screen = use_signal(|| Screen::Home);
    use_init_i18n(i18n::init_i18n);

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        div { style: "display: flex; flex-direction: column; height: 100vh; font-family: sans-serif;",

            // Main Content
            div { style: "flex: 1; overflow-y: auto;",
                match current_screen() {
                    Screen::Home => rsx! {
                        HomeScreen { on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::ProfileList => rsx! {
                        ProfileListScreen { on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::ProfileDetail(id) => rsx! {
                        ProfileDetailScreen { quail_id: id, on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::ProfileEdit(id) => rsx! {
                        ProfileEditScreen { quail_id: id, on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::AddProfile => rsx! {
                        AddProfileScreen { on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::EventAdd { quail_id, quail_name } => {
                        rsx! {
                            EventAdd {
                                quail_id,
                                quail_name,
                                on_navigate: move |s| current_screen.set(s),
                            }
                        }
                    }
                    Screen::EventEdit { event_id, quail_id } => rsx! {
                        EventEditScreen {
                            event_id,
                            quail_id,
                            on_navigate: move |s| current_screen.set(s),
                        }
                    },
                    Screen::EggTracking(date_opt) => rsx! {
                        EggTrackingScreen { date: date_opt, on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::EggHistory => rsx! {
                        EggHistoryScreen { on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::Statistics => rsx! {
                        StatisticsScreen { on_navigate: move |s| current_screen.set(s) }
                    },
                    Screen::Settings => rsx! {
                        SettingsScreen { on_navigate: move |s| current_screen.set(s) }
                    },
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
