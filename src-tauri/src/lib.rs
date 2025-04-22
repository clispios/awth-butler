use std::path::Path;

use anyhow::anyhow;
use aws::config::AwsConfigSections;
use futures::{
    SinkExt, StreamExt,
    channel::mpsc::{Receiver, channel},
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, async_runtime::spawn};
use tokio::sync::Mutex;
use utils::fetch_profiles_new;

mod aws;
mod cache;
mod error;
mod handlers;
mod utils;

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

async fn async_watch<P: AsRef<Path>>(path: P, app: AppHandle) -> notify::Result<()> {
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
                        println!("emit error: {:?}", e);
                    }
                }
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
pub(crate) struct ButlerState {
    pub(crate) aws_profiles: AwsConfigSections,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() -> Result<(), anyhow::Error> {
    let home_dir = dirs::home_dir().ok_or(anyhow!("No home directory detected!"))?;
    let config_dir = home_dir.join(".aws");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            spawn(setup(app.handle().clone()));
            spawn(async_watch(config_dir, app.handle().clone()));
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
        aws_profiles: fetch_profiles_new()?,
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
