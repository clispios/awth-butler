// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tauri::async_runtime::set(tokio::runtime::Handle::current());
    awth_butler_lib::run().await
}
