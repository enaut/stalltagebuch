use crate::Screen;
use dioxus::prelude::*;

#[component]
pub fn NavigationBar(current_screen: Screen, on_navigate: EventHandler<Screen>) -> Element {
    let nav_style = "display: flex; justify-content: space-around; padding: 10px; background: #f0f0f0; border-top: 1px solid #ddd;";

    rsx! {
        div {
            style: "{nav_style}",

            button {
                style: if matches!(current_screen, Screen::Home) {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #0066cc; color: #ffffff;"
                } else {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #ffffff; color: #333;"
                },
                onclick: move |_| on_navigate.call(Screen::Home),
                "🏠 Home"
            }

            button {
                style: if matches!(current_screen, Screen::ProfileList) {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #0066cc; color: #ffffff;"
                } else {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #ffffff; color: #333;"
                },
                onclick: move |_| on_navigate.call(Screen::ProfileList),
                "🐦 Profile"
            }

            button {
                style: if matches!(current_screen, Screen::EggHistory) {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #0066cc; color: #ffffff;"
                } else {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #ffffff; color: #333;"
                },
                onclick: move |_| on_navigate.call(Screen::EggHistory),
                "🥚 Eier"
            }

            button {
                style: if matches!(current_screen, Screen::Statistics) {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #0066cc; color: #ffffff;"
                } else {
                    "flex: 1; padding: 12px; margin: 0 5px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; text-align: center; background: #ffffff; color: #333;"
                },
                onclick: move |_| on_navigate.call(Screen::Statistics),
                "📊 Stats"
            }
        }
    }
}
