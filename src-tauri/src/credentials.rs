use anyhow::anyhow;
use aws_sdk_sso::operation::get_role_credentials::GetRoleCredentialsOutput;

use crate::utils::parse_aws_date_robust;

pub(crate) struct ButlerRoleCreds {
    // pub(crate) access_key_id: String,
    // pub(crate) secret_access_key: String,
    // pub(crate) session_token: String,
    pub(crate) expiration: chrono::DateTime<chrono::Utc>,
}

pub(crate) fn get_credentials_for_profile(
    profile_name: &str,
) -> Result<Option<ButlerRoleCreds>, anyhow::Error> {
    // Get the credentials file path
    let home_dir = dirs::home_dir().ok_or(anyhow!("No home directory detected!"))?;
    let credentials_path = home_dir.join(".aws").join("credentials");

    // Ensure file exists, if not return None
    if !credentials_path.exists() {
        return Ok(None);
    }

    // Parse the existing credentials file
    let ini = ini::Ini::load_from_file(&credentials_path)?;

    // Get the profile section
    let section = ini.section(Some(profile_name));
    if let Some(section) = section {
        // Extract the credentials
        let brc = ButlerRoleCreds {
            // access_key_id: section
            //     .get("aws_access_key_id")
            //     .ok_or(anyhow!("Missing access key ID!"))?
            //     .to_string(),
            // secret_access_key: section
            //     .get("aws_secret_access_key")
            //     .ok_or(anyhow!("Missing secret access key!"))?
            //     .to_string(),
            // session_token: section
            //     .get("aws_session_token")
            //     .ok_or(anyhow!("Missing session token!"))?
            //     .to_string(),
            expiration: parse_aws_date_robust(
                section
                    .get("aws_session_expiration")
                    .ok_or(anyhow!("Missing expiration timestamp!"))?,
            )?,
        };
        Ok(Some(brc))
    } else {
        Ok(None)
    }
}

pub(crate) fn store_credentials_for_profile(
    profile_name: &str,
    creds: &GetRoleCredentialsOutput,
) -> Result<(), anyhow::Error> {
    // Get the credentials file path
    let home_dir = dirs::home_dir().ok_or(anyhow!("No home directory detected!"))?;
    let credentials_path = home_dir.join(".aws").join("credentials");

    // Ensure the directory exists
    if let Some(parent) = credentials_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Parse the existing credentials file or create a new one
    let mut ini = if credentials_path.exists() {
        ini::Ini::load_from_file(&credentials_path)?
    } else {
        ini::Ini::new()
    };

    // Update credentials for this profile
    let role_creds = creds
        .role_credentials
        .as_ref()
        .ok_or(anyhow!("Missing role credentials after login!"))?;

    // Format expiration as ISO 8601
    let expiration = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
        role_creds.expiration, // Convert milliseconds to seconds
    )
    .ok_or(anyhow!("Invalid expiration timestamp!"))?
    .to_rfc3339();

    let session_token = role_creds
        .session_token()
        .ok_or(anyhow!("Missing session token!"))?;
    ini.with_section(Some(profile_name.to_string()))
        .set(
            "aws_access_key_id",
            role_creds
                .access_key_id()
                .ok_or(anyhow!("Missing access key ID!"))?,
        )
        .set(
            "aws_secret_access_key",
            role_creds
                .secret_access_key()
                .ok_or(anyhow!("Missing secret access key!"))?,
        )
        .set("aws_session_token", session_token)
        .set("aws_security_token", session_token)
        .set("aws_session_expiration", expiration);

    // Write back to the file
    ini.write_to_file(&credentials_path)?;

    Ok(())
}
