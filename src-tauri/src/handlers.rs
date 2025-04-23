use std::{
    str::FromStr,
    time::{Duration, SystemTime},
};

use anyhow::anyhow;
use aws_config::Region;
use aws_sdk_sso::operation::get_role_credentials::GetRoleCredentialsOutput;
use aws_sdk_ssooidc::operation::{
    register_client::RegisterClientOutput,
    start_device_authorization::StartDeviceAuthorizationOutput,
};
use serde::{Deserialize, Serialize};
use tauri::{State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tokio::sync::{
    Mutex,
    mpsc::{self, error::TryRecvError},
};

use crate::{
    ButlerState,
    aws::{
        config::Profile,
        credentials::{get_credentials_for_profile, store_credentials_for_profile},
    },
    cache::{get_token_from_cache, store_token_in_cache},
    fetch_profiles_new, trace_err_ret,
};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SsoToken {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expiration: SystemTime,
}

async fn generate_aws_config(region: Region) -> aws_config::SdkConfig {
    aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region)
        .load()
        .await
}

async fn create_registered_client(
    sso_oidc_client: &aws_sdk_ssooidc::Client,
) -> Result<RegisterClientOutput, anyhow::Error> {
    Ok(sso_oidc_client
        .register_client()
        .client_name("aws-awth-butler")
        .client_type("public")
        .scopes("sso:account:access")
        .send()
        .await?)
}

async fn run_client_authorization(
    sso_oidc_client: &aws_sdk_ssooidc::Client,
    reg_cli: &RegisterClientOutput,
    start_url: &str,
) -> Result<StartDeviceAuthorizationOutput, anyhow::Error> {
    Ok(sso_oidc_client
        .start_device_authorization()
        .client_id(
            reg_cli
                .client_id()
                .ok_or(trace_err_ret("Client ID not found!"))?,
        )
        .client_secret(
            reg_cli
                .client_secret()
                .ok_or(trace_err_ret("Client Secret not found!"))?,
        )
        .start_url(start_url)
        .send()
        .await?)
}

async fn execute_login_flow(
    app_handle: tauri::AppHandle,
    auth_out: &StartDeviceAuthorizationOutput,
    sso_oidc_client: &aws_sdk_ssooidc::Client,
    reg_cli: &RegisterClientOutput,
) -> Result<SsoToken, anyhow::Error> {
    let auth_window = WebviewWindowBuilder::new(
        &app_handle,
        "aws_authenticate",
        WebviewUrl::External(tauri::Url::from_str(
            auth_out.verification_uri_complete().ok_or(trace_err_ret(
                "No verification uri after client registration!",
            ))?,
        )?),
    )
    .title("AWS Authenticate")
    .inner_size(650.0, 800.0)
    .min_inner_size(650.0, 800.0)
    .build()?;

    let (tx, mut rx) = mpsc::channel(1);
    auth_window.on_window_event(move |event| {
        let tx = tx.clone();
        if let WindowEvent::CloseRequested { .. } = event {
            tx.try_send(()).unwrap_or(());
        }
    });

    let mut token: Option<SsoToken> = None;
    let interval = auth_out.interval() as u64;
    for _ in 0..(60 / interval) {
        match rx.try_recv() {
            Ok(_) => {
                return Err(trace_err_ret("User closed window before authenticating!"));
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                return Err(trace_err_ret(
                    "Process exited mid authentication for some reason!",
                ));
            }
        }
        if let Ok(output) = sso_oidc_client
            .create_token()
            .client_id(
                reg_cli
                    .client_id()
                    .ok_or(trace_err_ret("Client ID not found!"))?,
            )
            .client_secret(
                reg_cli
                    .client_secret()
                    .ok_or(trace_err_ret("Client Secret not found!"))?,
            )
            .grant_type("urn:ietf:params:oauth:grant-type:device_code")
            .device_code(
                auth_out
                    .device_code()
                    .ok_or(trace_err_ret("Device code not found!"))?,
            )
            .send()
            .await
        {
            token = Some(SsoToken {
                access_token: output
                    .access_token()
                    .ok_or(trace_err_ret("Access token missing from completed auth!"))?
                    .to_string(),
                refresh_token: output.refresh_token().map(|s| s.to_string()),
                expiration: SystemTime::now() + Duration::from_secs(output.expires_in() as u64),
            });
            break;
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }

    if auth_window.is_closable()? {
        auth_window.close()?
    }

    token.ok_or(trace_err_ret("Unable to complete SSO login flow!"))
}

async fn inner_sso_session_login(
    app_handle: tauri::AppHandle,
    state: State<'_, Mutex<ButlerState>>,
    session_name: &str,
) -> Result<(), anyhow::Error> {
    // grab session information from config, if it exists
    let profile_set = &state.lock().await.aws_profiles;
    // println!("{:#?}", profile_set);
    let session = profile_set
        .sessions
        .get(session_name)
        .ok_or(trace_err_ret("Session not found!"))?;
    let sso_region = session
        .get("sso_region")
        .ok_or(trace_err_ret("No region found for session!"))?;
    let region = Region::new(sso_region.to_string());

    // generate AWS config and clients
    let config = generate_aws_config(region).await;
    let sso_oidc_client = aws_sdk_ssooidc::Client::new(&config);
    let reg_cli = create_registered_client(&sso_oidc_client).await?;

    // run the client authorization flow
    let sso_start_url = session
        .get("sso_start_url")
        .ok_or(trace_err_ret("No start URL found for session!"))?;
    let response = run_client_authorization(&sso_oidc_client, &reg_cli, sso_start_url).await?;
    let token = execute_login_flow(app_handle, &response, &sso_oidc_client, &reg_cli).await?;

    // store the token in the cache
    store_token_in_cache(
        Some(session_name),
        sso_start_url,
        sso_region,
        &reg_cli,
        &token,
    )?;

    // find all profiles that use this session
    let sso_client = aws_sdk_sso::Client::new(&config);
    let profiles = &profile_set.profiles;
    let profiles_using_session: Vec<&Profile> = profiles
        .iter()
        .flat_map(|prof_name| profile_set.profiles.get(prof_name.0))
        .filter(|prof| prof.get("sso_session") == Some(session_name))
        .collect();

    // spawn off tasks to get role credentials for each profile in parallel
    let tasks: Vec<_> = profiles_using_session
        .iter()
        .map(|p| async {
            let creds = sso_client
                .get_role_credentials()
                .account_id(
                    p.get("sso_account_id")
                        .ok_or(trace_err_ret("No account ID found for profile!"))?,
                )
                .role_name(
                    p.get("sso_role_name")
                        .ok_or(trace_err_ret("No role name found for profile!"))?,
                )
                .access_token(&token.access_token)
                .send()
                .await?;
            Ok::<(&str, GetRoleCredentialsOutput), anyhow::Error>((p.name(), creds))
        })
        .collect();

    // collect the results from the tasks
    let prof_creds = futures::future::join_all(tasks)
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // update the credentials file with the new role credentials
    // don't want to do this in parallel in case of file contention
    for (prof_name, creds) in prof_creds {
        store_credentials_for_profile(prof_name, &creds)?;
    }

    // println!("finished doing auth thing!");
    Ok(())
}

async fn inner_legacy_profile_login(
    app_handle: tauri::AppHandle,
    state: State<'_, Mutex<ButlerState>>,
    profile_name: &str,
) -> Result<(), anyhow::Error> {
    let profile_set = &state.lock().await.aws_profiles;
    let prof = profile_set
        .profiles
        .get(profile_name)
        .ok_or(trace_err_ret("Profile not found!"))?;
    let sso_region = prof
        .get("sso_region")
        .ok_or(trace_err_ret("No sso_region found for profile!"))?;
    let region = Region::new(sso_region.to_string());
    let config = generate_aws_config(region).await;
    let sso_oidc_client = aws_sdk_ssooidc::Client::new(&config);
    let reg_cli = create_registered_client(&sso_oidc_client).await?;
    let sso_start_url = prof
        .get("sso_start_url")
        .ok_or(trace_err_ret("No sso_start_url found for profile!"))?;
    let response = run_client_authorization(
        &sso_oidc_client,
        &reg_cli,
        prof.get("sso_start_url")
            .ok_or(trace_err_ret("No sso_start_url found for profile!"))?,
    )
    .await?;

    let token = execute_login_flow(app_handle, &response, &sso_oidc_client, &reg_cli).await?;
    store_token_in_cache(None, sso_start_url, sso_region, &reg_cli, &token)?;
    let sso_client = aws_sdk_sso::Client::new(&config);
    let creds = sso_client
        .get_role_credentials()
        .account_id(prof.get("sso_account_id").unwrap())
        .role_name(prof.get("sso_role_name").unwrap())
        .access_token(token.access_token)
        .send()
        .await?;
    store_credentials_for_profile(profile_name, &creds)?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn refresh_profiles(state: State<'_, Mutex<ButlerState>>) -> Result<(), String> {
    let mut state = state.lock().await;
    state.aws_profiles = fetch_profiles_new().map_err(|e| e.to_string())?;
    // println!("{:#?}", state.aws_profiles);
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum LoginType {
    SsoSession,
    LegacyProfile,
}

#[tauri::command]
pub(crate) async fn authenticate_aws(
    app_handle: tauri::AppHandle,
    state: State<'_, Mutex<ButlerState>>,
    login_type: LoginType,
    name: &str,
) -> Result<(), String> {
    match login_type {
        LoginType::SsoSession => inner_sso_session_login(app_handle, state, name)
            .await
            .map_err(|e| e.to_string()),
        LoginType::LegacyProfile => inner_legacy_profile_login(app_handle, state, name)
            .await
            .map_err(|e| e.to_string()),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ButlerSsoSession {
    session_name: String,
    session_expiration: Option<chrono::DateTime<chrono::Utc>>,
    fresh: bool,
    profile_names: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ButlerSsoProfile {
    profile_name: String,
    session_name: String,
    profile_expiration: Option<chrono::DateTime<chrono::Utc>>,
    fresh: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ButlerSsoLegacyProfile {
    profile_name: String,
    profile_expiration: Option<chrono::DateTime<chrono::Utc>>,
    fresh: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ButlerSsoConfig {
    sessions: Vec<ButlerSsoSession>,
    sso_profiles: Vec<ButlerSsoProfile>,
    legacy_profiles: Vec<ButlerSsoLegacyProfile>,
}

#[tauri::command]
pub(crate) async fn fetch_butler_config(
    state: State<'_, Mutex<ButlerState>>,
) -> Result<ButlerSsoConfig, String> {
    // println!("Fetching butler config...");
    let state = &state.lock().await.aws_profiles;
    let sessions = &state
        .sessions
        .iter()
        .map(|s| s.1.name())
        .collect::<Vec<_>>();
    let profiles = state
        .profiles
        .iter()
        .flat_map(|pn| state.profiles.get(pn.1.name()))
        .collect::<Vec<_>>();

    let session_profiles = profiles
        .iter()
        .filter(|prof| prof.get("sso_session").is_some())
        .collect::<Vec<_>>();

    let legacy_profiles = profiles
        .iter()
        .filter(|prof| {
            prof.get("sso_session").is_none()
                && prof.get("sso_region").is_some()
                && prof.get("sso_start_url").is_some()
        })
        .collect::<Vec<_>>();

    // println!("fetched sessions, session profiles and legacy profiles... Now making config");
    let config = ButlerSsoConfig {
        sessions: sessions
            .iter()
            .map(|sn| {
                let cached_token = get_token_from_cache(sn)?;
                let sso_exp = cached_token.map(|tok| tok.expiration);
                let sso_fresh = sso_exp.map(|exp| exp > SystemTime::now()).unwrap_or(false);
                Ok::<_, anyhow::Error>(ButlerSsoSession {
                    session_name: sn.to_string(),
                    session_expiration: sso_exp.map(|exp| exp.into()),
                    fresh: sso_fresh,
                    profile_names: session_profiles
                        .iter()
                        .filter(|prof| prof.get("sso_session") == Some(sn))
                        .map(|prof| prof.name().to_string())
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()
            .map_err(|e| e.to_string())?,
        sso_profiles: session_profiles
            .iter()
            .map(|prof| {
                let cached_creds = get_credentials_for_profile(prof.name())?;
                let prof_exp = cached_creds.map(|creds| creds.expiration);
                let sess_name = prof
                    .get("sso_session")
                    .ok_or(trace_err_ret("No session name found!"))?;
                let prof_fresh = prof_exp
                    .map(|exp| exp > chrono::Utc::now())
                    .unwrap_or(false);
                Ok::<_, anyhow::Error>(ButlerSsoProfile {
                    profile_name: prof.name().to_string(),
                    session_name: sess_name.to_string(),
                    profile_expiration: prof_exp,
                    fresh: prof_fresh,
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()
            .map_err(|e| e.to_string())?,
        legacy_profiles: legacy_profiles
            .iter()
            .map(|prof| {
                let cached_creds = get_credentials_for_profile(prof.name())?;
                let prof_exp = cached_creds.map(|creds| creds.expiration);
                let prof_fresh = prof_exp
                    .map(|exp| exp > chrono::Utc::now())
                    .unwrap_or(false);
                Ok::<_, anyhow::Error>(ButlerSsoLegacyProfile {
                    profile_name: prof.name().to_string(),
                    profile_expiration: prof_exp,
                    fresh: prof_fresh,
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()
            .map_err(|e| e.to_string())?,
    };
    Ok(config)
}
