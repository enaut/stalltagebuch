use crate::{database, services, Screen};
use dioxus::prelude::*;
use dioxus_i18n::t;

#[component]
pub fn StatisticsScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut stats = use_signal(|| None::<services::analytics_service::EggStatistics>);
    let mut trend = use_signal(|| Vec::<(String, i32)>::new());
    let mut error = use_signal(|| String::new());
    let mut selected_period = use_signal(|| "all".to_string());

    let mut load_statistics = move || {
        match database::init_database() {
            Ok(conn) => {
                // Berechne Zeitraum basierend auf Auswahl
                let (start_date, end_date) = match selected_period().as_str() {
                    "week" => {
                        let today = chrono::Local::now().date_naive();
                        let week_ago = today - chrono::Duration::days(7);
                        (
                            Some(week_ago.format("%Y-%m-%d").to_string()),
                            Some(today.format("%Y-%m-%d").to_string()),
                        )
                    }
                    "month" => {
                        let today = chrono::Local::now().date_naive();
                        let month_ago = today - chrono::Duration::days(30);
                        (
                            Some(month_ago.format("%Y-%m-%d").to_string()),
                            Some(today.format("%Y-%m-%d").to_string()),
                        )
                    }
                    "year" => {
                        let today = chrono::Local::now().date_naive();
                        let year_ago = today - chrono::Duration::days(365);
                        (
                            Some(year_ago.format("%Y-%m-%d").to_string()),
                            Some(today.format("%Y-%m-%d").to_string()),
                        )
                    }
                    _ => (None, None),
                };

                match services::analytics_service::calculate_statistics(
                    &conn,
                    start_date.as_deref(),
                    end_date.as_deref(),
                ) {
                    Ok(statistics) => {
                        stats.set(Some(statistics));
                        error.set(String::new());
                    }
                    Err(e) => {
                        error.set(format!("{}: {}", t!("error-calculation"), e));
                        // Error message when statistics calculation fails
                    }
                }

                // Load trend data (last 30 days)
                match services::analytics_service::get_recent_trend(&conn, 30) {
                    Ok(data) => trend.set(data),
                    Err(e) => error.set(format!("{}: {}", t!("error-trend-load"), e)), // Error message when loading trend data fails
                }
            }
            Err(e) => {
                error.set(t!("error-database", error: e.to_string())); // Database connection error
            }
        }
    };

    // Load on mount and when period changes
    use_effect(move || {
        load_statistics();
    });

    rsx! {
        div {
            style: "padding: 16px; max-width: 800px; margin: 0 auto; min-height: 100vh; background: #f5f5f5;",

            // Header
            div {
                style: "margin-bottom: 20px; padding-top: 8px;",
                h1 {
                    style: "color: #0066cc; margin: 0 0 16px 0; font-size: 24px; font-weight: 700;",
                    "📊 " // Statistics page title
                    {t!("stats-title")}
                }

                // Period filter
                div {
                    style: "display: flex; gap: 8px; flex-wrap: wrap;",
                    for (label, value) in [(t!("period-all"), "all"), (t!("period-week"), "week"), (t!("period-month"), "month"), (t!("period-year"), "year")] { // Time period filter buttons
                        button {
                            style: if selected_period() == value {
                                "padding: 8px 16px; background: #0066cc; color: white; border-radius: 8px; font-weight: 600;"
                            } else {
                                "padding: 8px 16px; background: white; color: #0066cc; border: 1px solid #0066cc; border-radius: 8px;"
                            },
                            onclick: move |_| selected_period.set(value.to_string()),
                            "{label}"
                        }
                    }
                }
            }

            // Error
            if !error().is_empty() {
                div {
                    style: "padding: 12px; background: #ffebee; border-radius: 8px; color: #c62828; margin-bottom: 16px;",
                    "{error}"
                }
            }

            // Statistiken
            if let Some(s) = stats() {
                div {
                    style: "display: flex; flex-direction: column; gap: 12px;",

                    // Overview card
                    div {
                        class: "card",
                        h2 {
                            style: "margin: 0 0 16px 0; font-size: 18px; color: #333;",
                            "📈 " // Statistics overview section heading
                            {t!("stats-overview")}
                        }
                        div {
                            style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px;",

                            StatCard { label: t!("stats-total-records"), value: format!("{}", s.total_records), icon: "📋" } // Total number of egg records
                            StatCard { label: t!("stats-total-eggs"), value: format!("{}", s.total_eggs), icon: "🥚" } // Total number of eggs collected
                            StatCard { label: t!("stats-min"), value: format!("{}", s.min_eggs), icon: "⬇️" } // Minimum eggs in a single day
                            StatCard { label: t!("stats-max"), value: format!("{}", s.max_eggs), icon: "⬆️" } // Maximum eggs in a single day
                        }
                    }

                    // Averages card
                    div {
                        class: "card",
                        h2 {
                            style: "margin: 0 0 16px 0; font-size: 18px; color: #333;",
                            "📊 " // Averages section heading
                            {t!("stats-averages")}
                        }
                        div {
                            style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px;",

                            StatCard { label: t!("stats-daily-avg"), value: format!("{:.1}", s.daily_average), icon: "📅" } // Daily average eggs
                            StatCard { label: t!("stats-weekly-avg"), value: format!("{:.1}", s.weekly_average), icon: "📆" } // Weekly average eggs
                            StatCard { label: t!("stats-monthly-avg"), value: format!("{:.1}", s.monthly_average), icon: "🗓️" } // Monthly average eggs
                        }
                    }

                    // Date range info
                    if let (Some(first), Some(last)) = (&s.first_date, &s.last_date) {
                        div {
                            class: "card",
                            style: "background: #e3f2fd;",
                            p {
                                style: "margin: 0; font-size: 14px; color: #1565c0;",
                                "📅 " // Date range display (from/to)
                                {t!("stats-period")}
                                ": {first} "
                                {t!("stats-until")}
                                " {last}"
                            }
                        }
                    }

                    // Trend (simple list of last 10 days)
                    if !trend().is_empty() {
                        div {
                            class: "card",
                            h2 {
                                style: "margin: 0 0 16px 0; font-size: 18px; color: #333;",
                                "📈 " // Last 10 days trend section heading
                                {t!("stats-last-10-days")}
                            }
                            div {
                                style: "display: flex; flex-direction: column; gap: 8px;",
                                for (date, eggs) in trend().iter().take(10) {
                                    div {
                                        style: "display: flex; justify-content: space-between; padding: 8px; background: #f8f9fa; border-radius: 6px;",
                                        span { style: "color: #666;", "{date}" }
                                        span { style: "font-weight: 600; color: #ff8c00;", "🥚 {eggs}" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div {
                    class: "card",
                    style: "text-align: center; padding: 40px; color: #999;",
                    {t!("stats-no-data")} // Empty state when no statistics data available
                }
            }

            // Navigation
            div {
                style: "margin-top: 20px;",
                button {
                    class: "btn-primary",
                    style: "width: 100%;",
                    onclick: move |_| on_navigate.call(Screen::EggTracking(None)),
                    "➕ " // Button to navigate to egg entry form
                    {t!("stats-add-entry")}
                }
            }
        }
    }
}

#[component]
fn StatCard(label: String, value: String, icon: String) -> Element {
    rsx! {
        div {
            style: "background: #f8f9fa; padding: 12px; border-radius: 8px; text-align: center;",
            div {
                style: "font-size: 24px; margin-bottom: 4px;",
                "{icon}"
            }
            div {
                style: "font-size: 20px; font-weight: 700; color: #0066cc; margin-bottom: 4px;",
                "{value}"
            }
            div {
                style: "font-size: 12px; color: #666;",
                "{label}"
            }
        }
    }
}
