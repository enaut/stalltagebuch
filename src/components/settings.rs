use crate::database;
use crate::models::SyncSettings;
use crate::services::sync_service;
use crate::Screen;
use dioxus::prelude::*;
use dioxus_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq)]
enum NetworkStatus {
    Checking,
    Online,
    Offline(String),
}

#[component]
fn NetworkCheckCard() -> Element {
    let mut network_status = use_signal(|| NetworkStatus::Checking);

    // Check network connectivity on mount
    use_effect(move || {
        spawn(async move {
            // Try to connect to a reliable service
            match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
            {
                Ok(client) => {
                    match client
                        .get("https://www.google.com/generate_204")
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if response.status().is_success() || response.status().as_u16() == 204 {
                                network_status.set(NetworkStatus::Online);
                            } else {
                                network_status.set(NetworkStatus::Offline(format!(
                                    "HTTP Status: {}",
                                    response.status()
                                )));
                            }
                        }
                        Err(e) => {
                            network_status.set(NetworkStatus::Offline(format!(
                                "{}: {}",
                                t!("error-network"),
                                e
                            )));
                        }
                    }
                }
                Err(e) => {
                    network_status.set(NetworkStatus::Offline(format!(
                        "{}: {}",
                        t!("error-client"),
                        e
                    )));
                }
            }
        });
    });

    let recheck = move |_| {
        network_status.set(NetworkStatus::Checking);
        spawn(async move {
            match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
            {
                Ok(client) => {
                    match client
                        .get("https://www.google.com/generate_204")
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if response.status().is_success() || response.status().as_u16() == 204 {
                                network_status.set(NetworkStatus::Online);
                            } else {
                                network_status.set(NetworkStatus::Offline(format!(
                                    "HTTP Status: {}",
                                    response.status()
                                )));
                            }
                        }
                        Err(e) => {
                            network_status.set(NetworkStatus::Offline(format!(
                                "{}: {}",
                                t!("error-network"),
                                e
                            )));
                        }
                    }
                }
                Err(e) => {
                    network_status.set(NetworkStatus::Offline(format!(
                        "{}: {}",
                        t!("error-client"),
                        e
                    )));
                }
            }
        });
    };

    rsx! {
        match network_status() {
            NetworkStatus::Checking => rsx! {
                div { class: "card", style: "margin-bottom: 16px;",
                    div { style: "display: flex; align-items: center; gap: 12px;",
                        div { style: "font-size: 24px;", "🔄" }
                        div {
                            p { style: "margin: 0; font-weight: 600; font-size: 14px;", {t!("network-checking")} } // Network connectivity check in progress
                        }
                    }
                }
            },
            NetworkStatus::Online => rsx! {},
            NetworkStatus::Offline(error) => rsx! {
                div { class: "card", style: "margin-bottom: 16px;",
                    div {
                        div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 12px;",
                            div { style: "font-size: 24px;", "❌" }
                            div {
                                p { style: "margin: 0; font-weight: 600; font-size: 14px; color: #c62828;",
                                    {t!("network-offline")} // No internet connection message
                                }
                                p { style: "margin: 0; font-size: 12px; color: #666;", "{error}" }
                            }
                        }
                        button { class: "btn-primary", style: "width: 100%;", onclick: recheck,
                            "🔄 "
                            {t!("action-retry")}
                        }
                    }
                }
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginFlowInit {
    poll: PollInfo,
    login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PollInfo {
    token: String,
    endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoginFlowResult {
    server: String,
    #[serde(rename = "loginName")]
    login_name: String,
    #[serde(rename = "appPassword")]
    app_password: String,
}

#[derive(Clone, PartialEq)]
enum LoginState {
    NotStarted,
    InitiatingFlow,
    WaitingForUser {
        poll_url: String,
        token: String,
        login_url: String,
    },
    Success,
    Error(String),
}

#[derive(Clone, PartialEq)]
enum ConnectionStatus {
    Checking,
    Connected,
    Failed(String),
}

#[component]
pub fn SettingsScreen(on_navigate: EventHandler<Screen>) -> Element {
    let mut server_url = use_signal(|| String::from("https://"));
    let mut remote_path = use_signal(|| String::from("/Stalltagebuch"));
    let mut login_state = use_signal(|| LoginState::NotStarted);
    let mut current_settings = use_signal(|| None::<SyncSettings>);
    let mut status_message = use_signal(|| String::new());
    let mut connection_status = use_signal(|| None::<ConnectionStatus>);

    // Load existing settings on mount
    use_effect(move || {
        match database::init_database() {
            Ok(conn) => match sync_service::load_sync_settings(&conn) {
                Ok(Some(settings)) => {
                    server_url.set(settings.server_url.clone());
                    remote_path.set(settings.remote_path.clone());
                    current_settings.set(Some(settings.clone()));

                    // Test connection in background
                    let settings_clone = settings.clone();
                    spawn(async move {
                        connection_status.set(Some(ConnectionStatus::Checking));

                        let webdav_url = format!(
                            "{}/remote.php/dav/files/{}",
                            settings_clone.server_url.trim_end_matches('/'),
                            settings_clone.username
                        );

                        match reqwest_dav::ClientBuilder::new()
                            .set_host(webdav_url)
                            .set_auth(reqwest_dav::Auth::Basic(
                                settings_clone.username.clone(),
                                settings_clone.app_password.clone(),
                            ))
                            .build()
                        {
                            Ok(client) => {
                                match client
                                    .list(
                                        &settings_clone.remote_path,
                                        reqwest_dav::Depth::Number(0),
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        connection_status.set(Some(ConnectionStatus::Connected));
                                    }
                                    Err(e) => {
                                        connection_status.set(Some(ConnectionStatus::Failed(
                                            format!("{}: {:?}", t!("error-access-failed"), e),
                                        )));
                                    }
                                }
                            }
                            Err(e) => {
                                connection_status.set(Some(ConnectionStatus::Failed(format!(
                                    "{}: {:?}",
                                    t!("error-client"),
                                    e
                                ))));
                            }
                        }
                    });
                }
                Ok(None) => {
                    status_message.set(format!("\u{2139}\u{fe0f} {}", t!("sync-not-configured")));
                }
                Err(e) => {
                    status_message.set(format!("\u{26a0}\u{fe0f} {}: {}", t!("error-loading"), e));
                }
            },
            Err(e) => {
                status_message.set(format!("\u{274c} {}: {}", t!("error-database"), e));
            }
        }
    });

    // Start Nextcloud Login Flow v2
    let start_login = move |_| {
        let server = server_url();
        let remote_path_value = remote_path();
        login_state.set(LoginState::InitiatingFlow);

        spawn(async move {
            let url = format!("{}/index.php/login/v2", server.trim_end_matches('/'));

            match reqwest::Client::new()
                .post(&url)
                .header("User-Agent", "Stalltagebuch/0.1.0")
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<LoginFlowInit>().await {
                            Ok(flow) => {
                                let poll_url = flow.poll.endpoint.clone();
                                let token = flow.poll.token.clone();
                                let login_url = flow.login.clone();

                                // Set state to show login URL
                                login_state.set(LoginState::WaitingForUser {
                                    poll_url: poll_url.clone(),
                                    token: token.clone(),
                                    login_url: login_url.clone(),
                                });

                                // Start polling immediately in background
                                spawn(async move {
                                    // Poll up to 60 times (5 minutes, every 5 seconds)
                                    for _ in 0..60 {
                                        match reqwest::Client::new()
                                            .post(&poll_url)
                                            .form(&[("token", &token)])
                                            .header("User-Agent", "Stalltagebuch/0.1.0")
                                            .send()
                                            .await
                                        {
                                            Ok(response) => {
                                                if response.status().as_u16() == 200 {
                                                    match response.json::<LoginFlowResult>().await {
                                                        Ok(result) => {
                                                            // Create WebDAV client and folder
                                                            let webdav_url = format!(
                                                                "{}/remote.php/dav/files/{}",
                                                                result.server.trim_end_matches('/'),
                                                                result.login_name
                                                            );

                                                            match reqwest_dav::ClientBuilder::new()
                                                                .set_host(webdav_url)
                                                                .set_auth(reqwest_dav::Auth::Basic(
                                                                    result.login_name.clone(),
                                                                    result.app_password.clone(),
                                                                ))
                                                                .build()
                                                            {
                                                                Ok(client) => {
                                                                    // Try to create the folder
                                                                    match client
                                                                        .mkcol(&remote_path_value)
                                                                        .await
                                                                    {
                                                                        Ok(_) => {
                                                                            // Folder created successfully
                                                                        }
                                                                        Err(e) => {
                                                                            // Folder might already exist (405)
                                                                            eprintln!("Folder creation note: {}", e);
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    login_state.set(LoginState::Error(
                                                                        format!("{}: {:?}", t!("error-webdav-client"), e),
                                                                    ));
                                                                    return;
                                                                }
                                                            }

                                                            // Save credentials
                                                            let settings = SyncSettings::new(
                                                                result.server,
                                                                result.login_name,
                                                                result.app_password,
                                                                remote_path_value.clone(),
                                                            );

                                                            match database::init_database() {
                                                                Ok(conn) => {
                                                                    match sync_service::save_sync_settings(
                                                                        &conn, &settings,
                                                                    ) {
                                                                        Ok(_) => {
                                                                            current_settings
                                                                                .set(Some(settings));
                                                                            login_state
                                                                                .set(LoginState::Success);
                                                                            status_message.set(
                                                                                format!("\u{2705} {}", t!("sync-login-success-folder"))
                                                                            );
                                                                            return;
                                                                        }
                                                                        Err(e) => {
                                                                            login_state.set(
                                                                                LoginState::Error(format!(
                                                                                    "{}: {}",
                                                                                    t!("error-save"), e
                                                                                )),
                                                                            );
                                                                            return;
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    login_state.set(LoginState::Error(
                                                                        format!("{}: {}", t!("error-database"), e),
                                                                    ));
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            login_state.set(LoginState::Error(
                                                                format!(
                                                                    "{}: {}",
                                                                    t!("error-json"),
                                                                    e
                                                                ),
                                                            ));
                                                            return;
                                                        }
                                                    }
                                                } else if response.status().as_u16() != 404 {
                                                    login_state.set(LoginState::Error(format!(
                                                        "{}: {}",
                                                        t!("error-unexpected-status"),
                                                        response.status()
                                                    )));
                                                    return;
                                                }
                                                // 404 means waiting, continue polling
                                            }
                                            Err(e) => {
                                                login_state.set(LoginState::Error(format!(
                                                    "{}: {}",
                                                    t!("error-poll"),
                                                    e
                                                )));
                                                return;
                                            }
                                        }

                                        // Wait 5 seconds before next poll
                                        #[cfg(not(target_arch = "wasm32"))]
                                        std::thread::sleep(std::time::Duration::from_secs(5));
                                        #[cfg(target_arch = "wasm32")]
                                        gloo_timers::future::sleep(std::time::Duration::from_secs(
                                            5,
                                        ))
                                        .await;
                                    }

                                    login_state.set(LoginState::Error(
                                        t!("error-login-timeout").to_string(),
                                    ));
                                });
                            }
                            Err(e) => {
                                login_state.set(LoginState::Error(format!(
                                    "{}: {}",
                                    t!("error-json"),
                                    e
                                )));
                            }
                        }
                    } else {
                        login_state.set(LoginState::Error(format!(
                            "{}: {}",
                            t!("error-server"),
                            response.status()
                        )));
                    }
                }
                Err(e) => {
                    login_state.set(LoginState::Error(format!(
                        "{}: {}",
                        t!("error-connection"),
                        e
                    )));
                }
            }
        });
    };

    let delete_settings = move |_| match database::init_database() {
        Ok(conn) => match sync_service::delete_sync_settings(&conn) {
            Ok(_) => {
                current_settings.set(None);
                login_state.set(LoginState::NotStarted);
                status_message.set(format!("\u{2705} {}", t!("sync-settings-deleted")));
            }
            Err(e) => {
                status_message.set(format!("\u{26a0}\u{fe0f} {}: {}", t!("error-deleting"), e));
            }
        },
        Err(e) => {
            status_message.set(format!("\u{274c} {}: {}", t!("error-database"), e));
        }
    };

    rsx! {
        div { style: "padding: 16px; max-width: 600px; margin: 0 auto;",
            // Header
            div { style: "display: flex; align-items: center; margin-bottom: 24px;",
                button {
                    class: "btn-back",
                    onclick: move |_| on_navigate.call(Screen::Home),
                    "← "
                    {t!("action-back")}
                }
                h1 { style: "flex: 1; text-align: center; margin: 0; font-size: 24px; color: #0066cc;",
                    "⚙️ "
                    {t!("settings-title")}
                }
                div { style: "width: 80px;" }
            }

            // Status message
            if !status_message().is_empty() {
                div { style: "padding: 12px; margin-bottom: 16px; background: #f0f0f0; border-radius: 8px; border-left: 4px solid #0066cc;",
                    "{status_message}"
                }
            }

            // Network connectivity check
            NetworkCheckCard {}

            // Current settings display
            if let Some(settings) = current_settings() {
                div {
                    class: "card",
                    style: "margin-bottom: 16px; background: #e8f5e9;",
                    h2 { style: "margin: 0 0 12px 0; font-size: 18px; color: #2e7d32;",
                        "\u{2705} " // Sync configured successfully heading
                        {t!("sync-configured")}
                    }
                    p { style: "margin: 4px 0; font-size: 14px;",
                        strong {
                            {t!("sync-server")}
                            ": "
                        } // Server URL label
                        "{settings.server_url}"
                    }
                    p { style: "margin: 4px 0; font-size: 14px;",
                        strong {
                            {t!("sync-username")}
                            ": "
                        } // Username label
                        "{settings.username}"
                    }
                    p { style: "margin: 4px 0; font-size: 14px;",
                        strong {
                            {t!("sync-path")}
                            ": "
                        } // Remote path label
                        "{settings.remote_path}"
                        " "
                        match connection_status() {
                            Some(ConnectionStatus::Checking) => rsx! {
                                span { class: "spinner", style: "font-size: 12px;", "⏳" }
                            },
                            Some(ConnectionStatus::Connected) => rsx! {
                                span { style: "color: green; font-weight: bold;", "✓" }
                            },
                            Some(ConnectionStatus::Failed(ref err)) => rsx! {
                                span { style: "color: red; font-weight: bold;", title: "{err}", "⚠️" }
                            },
                            None => rsx! {
                                span {}
                            },
                        }
                    }
                    if let Some(last_sync) = settings.last_sync {
                        p { style: "margin: 4px 0; font-size: 14px;",
                            strong {
                                {t!("sync-last-sync")}
                                ": "
                            } // Last sync timestamp label
                            "{last_sync}"
                        }
                    }

                    div { style: "display: flex; gap: 12px; margin-top: 12px;",
                        button {
                            class: "btn-primary",
                            style: "flex: 1;",
                            onclick: move |_| {
                                spawn(async move {
                                    status_message
                                        .set("🔄 Vollständige Synchronisierung läuft...".to_string());
                                    match database::init_database() {
                                        Ok(conn) => {
                                            match crate::services::upload_service::sync_all(&conn).await {
                                                Ok((quails, events, egg_records, photos)) => {
                                                    status_message
                                                        .set(
                                                            format!(
                                                                "\u{2705} {}",
                                                                t!(
                                                                    "sync-success", quails : quails, events : events, eggs :
                                                                    egg_records, photos : photos
                                                                ),
                                                            ),
                                                        );
                                                    if let Ok(Some(updated)) = crate::services::sync_service::load_sync_settings(
                                                        &conn,
                                                    ) {
                                                        current_settings.set(Some(updated));
                                                    }
                                                }
                                                Err(e) => {
                                                    status_message
                                                        .set(format!("\u{274c} {}: {}", t!("sync-failed"), e));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            status_message
                                                .set(format!("\u{274c} {}", t!("error-database", error: e.to_string())));
                                        }
                                    }
                                });
                            },
                            {format!("🔄 {}", t!("sync-now"))}
                        }
                        button {
                            class: "btn-danger",
                            style: "flex: 1;",
                            onclick: delete_settings,
                            "🗑️ "
                            {t!("sync-delete-config")}
                        }
                    }
                }
            } else {
                // Setup form
                div { class: "card",
                    h2 { style: "margin: 0 0 16px 0; font-size: 18px; color: #333;",
                        {t!("sync-setup-title")} // Setup sync heading
                    }

                    // Server URL
                    div { style: "margin-bottom: 16px;",
                        label { style: "display: block; margin-bottom: 4px; font-weight: 600; font-size: 14px;",
                            {t!("sync-server-url")} // Server URL input label
                        }
                        input {
                            r#type: "url",
                            value: "{server_url}",
                            oninput: move |e| server_url.set(e.value()),
                            placeholder: "https://cloud.example.com",
                            style: "width: 100%; padding: 10px; font-size: 16px; border: 1px solid #ccc; border-radius: 4px;",
                        }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: #666;",
                            {t!("sync-server-hint")} // Server URL hint text
                        }
                    }

                    // Remote Path
                    div { style: "margin-bottom: 16px;",
                        label { style: "display: block; margin-bottom: 4px; font-weight: 600; font-size: 14px;",
                            {t!("sync-path-label")} // Remote path input label
                        }
                        input {
                            r#type: "text",
                            value: "{remote_path}",
                            oninput: move |e| remote_path.set(e.value()),
                            placeholder: "/Stalltagebuch",
                            style: "width: 100%; padding: 10px; font-size: 16px; border: 1px solid #ccc; border-radius: 4px;",
                        }
                        p { style: "margin: 4px 0 0 0; font-size: 12px; color: #666;",
                            {t!("sync-path-hint")} // Remote path hint text
                        }
                    }

                    // Login button and status
                    match login_state() {
                        LoginState::NotStarted => rsx! {
                            button {
                                class: "btn-primary",
                                onclick: start_login,
                                disabled: server_url().trim().is_empty() || !server_url().starts_with("http"),
                                "🔐 "
                                {t!("sync-login")}
                            }
                        },
                        LoginState::InitiatingFlow => rsx! {
                            div { style: "padding: 12px; background: #fff3cd; border-radius: 4px; text-align: center;",
                                "🔄 "
                                {t!("sync-connecting")}
                            }
                        },
                        LoginState::WaitingForUser { login_url, poll_url: _, token: _ } => {
                            rsx! {
                                div { style: "padding: 12px; background: #d1ecf1; border-radius: 4px;",
                                    div { style: "display: flex; align-items: center; gap: 12px; margin-bottom: 12px;",
                                        div { style: "font-size: 32px; animation: spin 2s linear infinite;", "💠" }
                                        div {
                                            p { style: "margin: 0; font-weight: 600; font-size: 16px;", {t!("sync-waiting")} } // Waiting for login message
                                            p { style: "margin: 4px 0 0 0; font-size: 12px; color: #666;",
                                                {t!("sync-polling-background")} // Polling in background message
                                            }
                                        }
                                    }
                                    p { style: "margin: 0 0 12px 0; font-size: 14px;", {t!("sync-login-instructions")} } // Login instructions
                                    a {
                                        href: "{login_url}",
                                        target: "_blank",
                                        style: "display: block; padding: 12px; background: #0066cc; color: white; text-decoration: none; border-radius: 4px; text-align: center; font-weight: 600;",
                                        "🌐 "
                                        {t!("sync-login-browser")}
                                    }
                                }
                            }
                        }
                        LoginState::Success => rsx! {
                            div { style: "padding: 12px; background: #d4edda; border-radius: 4px; text-align: center; color: #155724;",
                                "\u{2705} " // Login success message
                                {t!("sync-login-success")}
                            }
                        },
                        LoginState::Error(error) => rsx! {
                            div { style: "padding: 12px; background: #f8d7da; border-radius: 4px; color: #721c24;",
                                p { style: "margin: 0 0 12px 0; font-weight: 600;",
                                    "\u{274c} "
                                    {t!("sync-error")}
                                } // Login error heading
                                p { style: "margin: 0; font-size: 14px;", "{error}" }
                                button {
                                    class: "btn-primary",
                                    style: "margin-top: 12px;",
                                    onclick: move |_| login_state.set(LoginState::NotStarted),
                                    "🔄 Erneut versuchen"
                                }
                            }
                        },
                    }

                    // Info box
                    div { style: "margin-top: 16px; padding: 12px; background: #f8f9fa; border-radius: 4px; border-left: 4px solid #0066cc;",
                        p { style: "margin: 0 0 8px 0; font-size: 14px; font-weight: 600;",
                            "\u{2139}\u{fe0f} " // How login works heading
                            {t!("sync-login-info-title")}
                        }
                        ul { style: "margin: 0; padding-left: 20px; font-size: 13px; color: #555;",
                            li { {t!("sync-login-step1")} } // Step 1: Click login button
                            li { {t!("sync-login-step2")} } // Step 2: Open browser link
                            li { {t!("sync-login-step3")} } // Step 3: Login to Nextcloud
                            li { {t!("sync-login-step4")} } // Step 4: Confirm access
                            li { {t!("sync-login-step5")} } // Step 5: Return to app
                        }
                    }
                }
            }
        }
    }
}
