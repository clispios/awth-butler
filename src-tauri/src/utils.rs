use anyhow::anyhow;
use chrono::{DateTime, Utc};

pub(crate) async fn fetch_profiles()
-> Result<aws_runtime::env_config::section::EnvConfigSections, anyhow::Error> {
    let fs = aws_types::os_shim_internal::Fs::real();
    let env = aws_types::os_shim_internal::Env::real();
    let profile_files = aws_runtime::env_config::file::EnvConfigFiles::default();
    Ok(aws_config::profile::load(&fs, &env, &profile_files, None).await?)
}

pub(crate) fn parse_aws_date_robust(date_str: &str) -> Result<DateTime<Utc>, anyhow::Error> {
    let formats = [
        "%Y-%m-%dT%H:%M:%S%:z", // Standard RFC3339
        "%Y-%m-%dT%H:%M:%S%z",  // Without colon in timezone
        "%Y-%m-%dT%H:%M:%SZ",   // UTC/Zulu time format
        "%Y-%m-%d %H:%M:%S%:z", // Space instead of T
        "%Y-%m-%d %H:%M:%S%z",  // Space instead of T, no colon in timezone
    ];

    for format in &formats {
        if let Ok(dt) = DateTime::parse_from_str(date_str, format) {
            return Ok(dt.with_timezone(&Utc));
        }
    }

    Err(anyhow!(
        "Failed to parse date: '{}'. Tried formats: {:?}",
        date_str,
        formats.iter().map(|f| f.to_string()).collect::<Vec<_>>()
    ))
}
