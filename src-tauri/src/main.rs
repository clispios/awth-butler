#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod configure;
use regex::Regex;
use std::{process::Command, sync::Mutex};
use sysinfo::{Pid, PidExt, ProcessExt, System, SystemExt};
use tauri::{
    AppHandle, CustomMenuItem, Manager, State, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

fn init_aws_version() -> Option<String> {
    let re = Regex::new(r"aws-cli/(\d+)\.(\d+)\.(\d+)").unwrap();
    let cmd;
    #[cfg(target_os = "windows")]
    {
        cmd = Command::new("aws")
            .arg("--version")
            .creation_flags(0x08000000) //hide windows console
            .output();
    }

    #[cfg(not(target_os = "windows"))]
    {
        cmd = Command::new("aws").arg("--version").output();
    }

    match cmd {
        Ok(res) => {
            if let Ok(out) = String::from_utf8(res.stdout) {
                if let Some(vzn) = re.find(&out) {
                    return Some(
                        vzn.as_str()
                            .to_string()
                            .split("/")
                            .last()
                            .unwrap_or("0.0.0")
                            .to_string(),
                    );
                }
            }
            return None;
        }
        Err(e) => {
            println!("error AWS version check... {}", e);
            None
        }
    }
}

#[tauri::command]
fn get_aws_cli_ver(process_state: State<configure::ProcessState>) -> String {
    let ver = process_state.cli_ver.lock().unwrap();
    return ver.clone();
}

#[tauri::command]
fn get_full_profiles() -> Vec<configure::Profile> {
    return configure::get_profiles();
}

#[tauri::command(rename_all = "snake_case")]
fn update_creds(full_prof: configure::Profile) {
    configure::update_creds(&full_prof);
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
                #[cfg(not(target_os = "macos"))]
                {
                    event.window().hide().unwrap();
                }

                #[cfg(target_os = "macos")]
                {
                    tauri::AppHandle::hide(&event.window().app_handle()).unwrap();
                }
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
            cli_ver: Mutex::from("0.0.0".to_string()),
            running: Mutex::from(false),
            pid: Mutex::from(0),
        })
        .invoke_handler(tauri::generate_handler![
            get_aws_cli_ver,
            do_sso_login,
            do_sso_cancel,
            update_creds,
            get_profile_exp,
            get_full_profiles
        ])
        .setup(|app| {
            let ver = init_aws_version();
            if let Some(v) = ver {
                let app_state: State<configure::ProcessState> = app.state();
                let mut cli_ver = app_state.cli_ver.lock().unwrap();
                *cli_ver = v;
            }
            let window = app.get_window("main").unwrap();
            window.center().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
