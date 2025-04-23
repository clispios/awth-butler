use anyhow::anyhow;
use aws_sdk_ssooidc::operation::register_client::RegisterClientOutput;
use serde_json::json;
use sha1::{Digest, Sha1};
use walkdir::WalkDir;

use crate::{handlers::SsoToken, trace_err_ret};
use std::fs;

pub(crate) fn store_token_in_cache(
    session_name: Option<&str>,
    sso_start_url: &str,
    sso_region: &str,
    reg_cli: &RegisterClientOutput,
    token: &SsoToken,
) -> Result<(), anyhow::Error> {
    // Calculate hash for the cache file name
    let mut hasher = Sha1::new();
    hasher.update(sso_start_url.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    // Determine cache directory
    let cache_dir = dirs::home_dir()
        .ok_or(trace_err_ret("home directory not found..."))?
        .join(".aws")
        .join("sso")
        .join("cache");

    // Create directory if it doesn't exist
    fs::create_dir_all(&cache_dir)?;

    // Create cache file path
    let cache_file = cache_dir.join(format!("{}.json", hash));

    // Create token object for serialization
    let cache_entry = json!({
        "startUrl": sso_start_url,
        "region": sso_region,
        "accessToken": token.access_token,
        "clientId": reg_cli.client_id,
        "clientSecret": reg_cli.client_secret,
        "registrationExpiresAt": format!("{}",
            chrono::DateTime::<chrono::Utc>::from_timestamp(reg_cli.client_secret_expires_at(), 0)
                .ok_or(trace_err_ret("Invalid timestamp!"))?
                .format("%Y-%m-%dT%H:%M:%SZ")),
        "expiresAt": format!("{}",
            chrono::DateTime::<chrono::Utc>::from(token.expiration)
                .format("%Y-%m-%dT%H:%M:%SZ")),
        "refreshToken": token.refresh_token,
        "sessionName": session_name,
    });

    // Write to file
    fs::write(cache_file, serde_json::to_string_pretty(&cache_entry)?)?;

    if let Some(sn) = session_name {
        let mut name_haser = Sha1::new();
        name_haser.update(sn.as_bytes());
        let name_hash = format!("{:x}", name_haser.finalize());
        let name_cache_file = cache_dir.join(format!("{}.json", name_hash));

        fs::write(name_cache_file, serde_json::to_string_pretty(&cache_entry)?)?;
    }
    Ok(())
}

pub(crate) fn get_token_from_cache(session_name: &str) -> Result<Option<SsoToken>, anyhow::Error> {
    // Get the cache directory
    let cache_dir = dirs::home_dir()
        .ok_or(trace_err_ret("home directory not found..."))?
        .join(".aws")
        .join("sso")
        .join("cache");

    // Ensure the directory exists
    if !cache_dir.exists() {
        return Ok(None);
    }
    // Look through all files in the directory
    for entry in WalkDir::new(&cache_dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Try to read and parse the file
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                // Check if this is the right session
                if json.get("sessionName").and_then(|v| v.as_str()) == Some(session_name) {
                    // Parse the token
                    let access_token = json
                        .get("accessToken")
                        .and_then(|v| v.as_str())
                        .ok_or(trace_err_ret("Invalid cache entry!"))?
                        .to_string();

                    let refresh_token = json
                        .get("refreshToken")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let expires_at = json
                        .get("expiresAt")
                        .and_then(|v| v.as_str())
                        .ok_or(trace_err_ret("Invalid cache entry!"))?;

                    // Parse expiration date
                    let expiration = chrono::DateTime::parse_from_rfc3339(expires_at)?
                        .with_timezone(&chrono::Utc)
                        .into();

                    return Ok(Some(SsoToken {
                        access_token,
                        refresh_token,
                        expiration,
                    }));
                }
            }
        }
    }

    Ok(None)
}
