#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod configure;
use std::sync::Mutex;
use sysinfo::{Pid, PidExt, ProcessExt, System, SystemExt};
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

#[tauri::command]
fn get_profiles() -> Vec<String> {
    return configure::get_profile_names();
}

#[tauri::command]
fn get_profile_exp(prof: &str) -> String {
    return configure::get_profile_status(prof);
}

#[tauri::command]
fn do_sso_login(app: AppHandle, process_state: State<configure::ProcessState>, prof: &str) -> u32 {
    configure::sso_login(app, process_state, prof)
}

#[tauri::command]
fn do_sso_cancel(process_state: State<configure::ProcessState>, running_login_pid: u32) {
    configure::kill_running_login(process_state, running_login_pid);
}

fn main() {

    let _ = fix_path_env::fix();
    let open = CustomMenuItem::new("open".to_string(), "Open");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let tray_menu = SystemTrayMenu::new()
        .add_item(open)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    let tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .system_tray(tray)
        .on_window_event(|event| match event.event() {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                event.window().hide().unwrap();
                api.prevent_close();
            }
            _ => {}
        })
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "open" => {
                    let window = app.get_window("main").unwrap();
                    if window.is_minimized().unwrap() {
                        if std::env::consts::OS == "linux" {
                            window.hide().unwrap();
                            window.show().unwrap();
                            window.center().unwrap();
                        } else {
                            window.unminimize().unwrap();
                        }
                    }
                    if !window.is_visible().unwrap() {
                        window.show().unwrap();
                        window.center().unwrap();
                    }
                    window.set_focus().unwrap();
                }
                "quit" => {
                    let app_state: State<configure::ProcessState> = app.state();
                    let s = System::new_all();
                    let process_status = app_state.running.lock().unwrap();
                    let process_pid = app_state.pid.lock().unwrap();
                    if *process_status {
                        if let Some(process) = s.process(Pid::from_u32(*process_pid)) {
                            process.kill();
                        }
                    }
                    std::process::exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .manage(configure::ProcessState {
            running: Mutex::from(false),
            pid: Mutex::from(0),
        })
        .invoke_handler(tauri::generate_handler![
            do_sso_login,
            do_sso_cancel,
            get_profiles,
            get_profile_exp
        ])
        .setup(|app| {
            let window = app.get_window("main").unwrap();
            window.center().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
