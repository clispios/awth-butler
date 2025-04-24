use std::path::PathBuf;

use aws::config::AwsConfigSections;
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{Receiver, channel},
};
use global::trace_err_ret;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, async_runtime::spawn};
use tokio::sync::Mutex;
use tracing::{Level, info};
use utils::fetch_profiles_new;

mod aws;
mod cache;
mod error;
mod global;
mod handlers;
mod utils;

fn setup_logging() {
    #[cfg(all(desktop, not(debug_assertions)))]
    let writer = {
        use crate::global::APP_CONFIG_DIR;
        use std::{fs::File, sync::Mutex};
        let log_file =
            File::create(APP_CONFIG_DIR.join("butler.log")).expect("Failed to create the log file");
        Mutex::new(log_file)
    };

    #[cfg(any(debug_assertions, mobile))]
    let writer = std::io::stderr;

    let builder = tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        .with_file(true)
        .with_line_number(true)
        .with_env_filter("awth_butler_lib")
        .with_target(false)
        .with_writer(writer);

    if cfg!(debug_assertions) {
        builder.init();
    } else {
        builder.json().init();
    }
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch(path: PathBuf, app: AppHandle) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(event) => match event.kind {
                notify::EventKind::Create(_)
                | notify::EventKind::Modify(_)
                | notify::EventKind::Remove(_) => {
                    if let Err(e) = app.emit_to("main", "configs-change", "change") {
                        tracing::error!("emit error: {:?}", e);
                    }
                }
                _ => {}
            },
            Err(e) => tracing::error!("watch error: {:?}", e),
        }
    }

    Ok(())
}

pub(crate) struct ButlerState {
    pub(crate) aws_profiles: AwsConfigSections,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), anyhow::Error> {
    setup_logging();
    info!("Logging initialized");

    let home_dir = dirs::home_dir().ok_or_else(|| trace_err_ret("home directory not found..."))?;
    let path = home_dir.join(".aws");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            spawn(setup(app.handle().clone()));
            spawn(async_watch(path, app.handle().clone()));
            Ok(())
        })
        .manage(Mutex::new(ButlerState {
            aws_profiles: fetch_profiles_new()?,
        }))
        .invoke_handler(tauri::generate_handler![
            handlers::authenticate_aws,
            handlers::refresh_profiles,
            handlers::fetch_butler_config,
        ])
        // NOTE: This error is fine
        .run(tauri::generate_context!())
        .map_err(Into::into)
}

async fn setup(app: AppHandle) -> Result<(), anyhow::Error> {
    let main_win = app
        .get_webview_window("main")
        .ok_or_else(|| trace_err_ret("main window not found"))?;
    main_win.center()?;
    Ok(())
}
