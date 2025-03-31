use anyhow::anyhow;
use tauri::{AppHandle, Manager, async_runtime::spawn};
use tokio::sync::Mutex;
use utils::fetch_profiles;

mod cache;
mod credentials;
mod error;
mod handlers;
mod utils;

pub(crate) struct ButlerState {
    pub(crate) aws_profiles: aws_runtime::env_config::section::EnvConfigSections,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), anyhow::Error> {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            spawn(setup(app.handle().clone()));
            Ok(())
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            handlers::authenticate_aws,
            handlers::refresh_profiles,
            handlers::fetch_butler_config,
        ])
        // allow rust-analyzer error E0308
        .run(tauri::generate_context!())
        .map_err(Into::into)
}

async fn setup(app: AppHandle) -> Result<(), anyhow::Error> {
    let state = ButlerState {
        aws_profiles: fetch_profiles().await?,
    };
    app.manage(Mutex::new(state));
    let splash_win = app
        .get_webview_window("splashscreen")
        .ok_or(anyhow!("No splashscreen!"))?;
    let main_win = app
        .get_webview_window("main")
        .ok_or(anyhow!("No main window!"))?;
    splash_win.close()?;
    main_win.show()?;
    Ok(())
}
