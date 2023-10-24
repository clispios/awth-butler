use dirs;
use ini::Ini;
use regex::Regex;
use std::io::stdout;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
use sysinfo::{Pid, PidExt, ProcessExt, System, SystemExt};
use tauri::{AppHandle, Manager, State};
use wait_timeout::ChildExt;

#[derive(Clone, serde::Serialize)]
struct Payload {
    event_type: String,
    auth_code: String,
    auth_url: String,
    process_pid: u32,
}

pub struct ProcessState {
    pub running: Mutex<bool>,
    pub pid: Mutex<u32>,
}

pub fn get_profile_status(prof: &str) -> String {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let filepath = homedir + "/.aws/credentials";
    let file_existence = Path::new(&filepath).try_exists();
    let mut exp_val = "";
    let creds = Ini::load_from_file(&filepath);
    let conf;
    if file_existence.is_ok() && file_existence.unwrap() {
        conf = creds.unwrap();
        let sec = conf.section(Some(prof));
        if sec.is_some() {
            let expiration = sec.unwrap().get("aws_session_expiration");
            if expiration.is_some() {
                exp_val = expiration.unwrap();
            }
        } else {
            exp_val = "never authenticated"
        }
    }
    return exp_val.to_string();
}

pub fn get_profile_names() -> Vec<String> {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let filepath = homedir.to_owned() + "/.aws/config";
    let file_existence = Path::new(&filepath).try_exists();
    let mut strvec = Vec::new();

    if file_existence.is_ok() && file_existence.unwrap() {
        let conf = Ini::load_from_file(&filepath).unwrap();
        for (sec, ks) in &conf {
            if sec.is_some() && ks.contains_key("sso_role_name") {
                strvec.push(sec.unwrap().to_string());
            }
        }
    }
    return strvec;
}

pub fn kill_running_login(process_state: State<ProcessState>, id: u32) {
    let s = System::new_all();
    let mut process_status = process_state.running.lock().unwrap();
    let mut process_pid = process_state.pid.lock().unwrap();
    if *process_status && (*process_pid == id) {
        if let Some(process) = s.process(Pid::from_u32(id)) {
            process.kill();
        }
    }
    *process_status = false;
    *process_pid = 0;
}

pub fn sso_login(app: AppHandle, process_state: State<ProcessState>, prof: &str) -> u32 {
    let profile = prof.to_string();
    let re = Regex::new(r"[A-Z]{4}-[A-Z]{4}").unwrap();
    let re_url = Regex::new(r"https?:\/\/(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*)").unwrap();
    let mut child = Command::new("aws")
        .args(&["sso", "login", "--profile", &profile, "--no-browser"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to execute process: {}", e));

    let command_timeout = Duration::from_secs(60);
    let out = child.stdout.as_mut().unwrap();
    let mut read_buf = [0u8; 64];
    let mut owned_string: String = "\n".to_owned();
    while let Ok(size) = out.read(&mut read_buf) {
        if size == 0 {
            break;
        }
        stdout().write_all(&read_buf).unwrap();
        owned_string.push_str(String::from_utf8(read_buf.into()).unwrap().as_str());
        if re.find(&owned_string).is_some() {
            let auth_url_prefix = re_url.find(&owned_string).unwrap().as_str();
            let auth_code = re.find(&owned_string).unwrap().as_str();
            let auth_url: String = auth_url_prefix.to_string() + "?user_code=" + auth_code;
            let _ = app.emit_all(
                "sso-login",
                Payload {
                    event_type: "start".into(),
                    auth_code: auth_code.into(),
                    auth_url,
                    process_pid: child.id().clone(),
                },
            );
            break;
        }
    }
    let mut process_status = process_state.running.lock().unwrap();
    let mut process_pid = process_state.pid.lock().unwrap();
    let child_id = child.id();
    *process_status = true;
    *process_pid = child.id();
    std::thread::spawn(move || {
        match child.wait_timeout(command_timeout).unwrap() {
            Some(status) => {
                if status.success() {
                    let _ = app.emit_all(
                        "sso-login",
                        Payload {
                            event_type: "success".into(),
                            auth_code: "".into(),
                            auth_url: "".into(),
                            process_pid: 0,
                        },
                    );
                }
                status.code()
            }
            None => {
                // child hasn't exited yet
                child.kill().unwrap();
                let _ = app.emit_all(
                    "sso-login",
                    Payload {
                        event_type: "timeout".into(),
                        auth_code: "".into(),
                        auth_url: "".into(),
                        process_pid: 0,
                    },
                );
                let status = child.wait().unwrap().code();
                println!("\nExiting timeout shell command.");
                status
            }
        };
    });
    return child_id;
}
