use chrono::{DateTime, Utc};
use dirs;
use ini::Ini;
use regex::Regex;
use serde::Deserialize;
use std::fs::File;
use std::io::{stdout, BufReader};
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

#[derive(Default, Deserialize, Debug, serde::Serialize)]
pub struct Profile {
    profile_name: String,
    sso_start_url: Option<String>,
    sso_region: Option<String>,
    sso_account_id: Option<String>,
    sso_role_name: Option<String>,
    region: Option<String>,
}

#[derive(Default, Deserialize, Debug, serde::Serialize)]
struct ProfileCreds {
    aws_access_key_id: Option<String>,
    aws_secret_access_key: Option<String>,
    aws_session_token: Option<String>,
    aws_security_token: Option<String>,
    aws_session_expiration: Option<String>,
}

pub struct ProcessState {
    pub running: Mutex<bool>,
    pub pid: Mutex<u32>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RoleCreds {
    access_key_id: String,
    secret_access_key: String,
    session_token: String,
    expiration: i64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CredsOutput {
    role_credentials: RoleCreds,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    start_url: String,
    region: String,
    access_token: String,
    expires_at: String,
    client_id: String,
    client_secret: String,
    registration_expires_at: String,
}

pub fn get_profile_status(prof: &str) -> String {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let filepath = homedir + "/.aws/credentials";
    if let (Ok(_exists), Ok(conf)) = (
        Path::new(&filepath).try_exists(),
        Ini::load_from_file(&filepath),
    ) {
        if let Some(sec) = conf.section(Some(prof)) {
            let prof_creds = ProfileCreds {
                aws_access_key_id: sec.get("aws_access_key_id").map(str::to_string),
                aws_secret_access_key: sec.get("aws_secret_access_key").map(str::to_string),
                aws_session_token: sec.get("aws_session_token").map(str::to_string),
                aws_security_token: sec.get("aws_security_token").map(str::to_string),
                aws_session_expiration: sec.get("aws_session_expiration").map(str::to_string),
            };
            if let Some(exp) = prof_creds.aws_session_expiration {
                return exp;
            }
        }
    };
    return "Never Authenticated".to_string();
}

pub fn get_profiles() -> Vec<Profile> {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let filepath = homedir.to_owned() + "/.aws/config";

    if let (Ok(_exists), Ok(conf)) = (
        Path::new(&filepath).try_exists(),
        Ini::load_from_file(&filepath),
    ) {
        let res = conf
            .sections()
            .map(|sec_name| (sec_name, conf.section(sec_name)))
            .filter_map(|(sec_name, some_sec)| {
                if let Some(sec) = some_sec {
                    if sec.contains_key("sso_start_url") {
                        if let Some(name) = sec_name.unwrap().split("profile ").last() {
                            return Some((name, sec));
                        }
                    }
                }
                return None;
            })
            .map(|(name, sec)| Profile {
                profile_name: name.into(),
                sso_start_url: sec.get("sso_start_url").map(str::to_string),
                sso_region: sec.get("sso_region").map(str::to_string),
                sso_account_id: sec.get("sso_account_id").map(str::to_string),
                sso_role_name: sec.get("sso_role_name").map(str::to_string),
                region: sec.get("region").map(str::to_string),
            })
            .collect::<Vec<Profile>>();
        return res;
    }
    return Vec::new();
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

fn get_auth_token(sso_url: &Option<String>) -> String {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let folder_path = homedir.to_owned() + "/.aws/sso/cache/";
    let mut auth_token: String = "".into();
    if let (Ok(files), Some(url)) = (std::fs::read_dir(folder_path), sso_url) {
        for file in files {
            let file = File::open(Path::new(file.unwrap().path().as_path())).unwrap();
            let reader = BufReader::new(file);

            let entry: Result<CacheEntry, serde_json::Error> = serde_json::from_reader(reader);
            if let Ok(sso_file) = entry {
                if url == &sso_file.start_url {
                    auth_token = sso_file.access_token;
                }
            }
        }
    }
    return auth_token;
}

fn modify_file_se(prof: &Profile, creds_out: &RoleCreds) {
    let homedir = dirs::home_dir()
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let filepath = homedir + "/.aws/credentials";
    if let (Ok(_exists), Ok(mut conf)) = (
        Path::new(&filepath).try_exists(),
        Ini::load_from_file(&filepath),
    ) {
        let dt: DateTime<Utc> = DateTime::<Utc>::from_timestamp(creds_out.expiration / 1000, 0)
            .expect("invalid timestamp");
        let formatted = format!("{}", dt.format("%Y-%m-%dT%H:%M:%S+0000"));
        conf.with_section(Some(&prof.profile_name))
            .set("aws_access_key_id", &creds_out.access_key_id)
            .set("aws_secret_access_key", &creds_out.secret_access_key)
            .set("aws_session_token", &creds_out.session_token)
            .set("aws_security_token", &creds_out.session_token)
            .set("aws_session_expiration", formatted);
        match conf.write_to_file(filepath) {
            Ok(_) => {
                println!("Wrote to file!");
            }
            Err(e) => {
                println!("Error during write! {}", e);
            }
        }
    }
}

pub fn update_creds(prof: &Profile) {
    let auth_token = get_auth_token(&prof.sso_start_url);
    let cmd = Command::new("aws")
        .args(&[
            "sso",
            "get-role-credentials",
            "--output",
            "json",
            "--profile",
            &prof.profile_name,
            "--region",
            prof.sso_region.as_ref().unwrap(),
            "--role-name",
            prof.sso_role_name.as_ref().unwrap(),
            "--account-id",
            prof.sso_account_id.as_ref().unwrap(),
            "--access-token",
            &auth_token,
        ])
        .output();

    match cmd {
        Ok(res) => {
            let creds_res: Result<CredsOutput, serde_json::Error> =
                serde_json::from_slice(&res.stdout);
            match creds_res {
                Ok(json) => {
                    modify_file_se(prof, &json.role_credentials);
                }
                Err(e) => {
                    println!("{}", e);
                }
            }
        }
        Err(_) => {
            println!("error during creds update...");
        }
    }
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
