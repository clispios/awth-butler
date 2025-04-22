use anyhow::anyhow;
use chrono::{DateTime, Utc};

pub(crate) async fn fetch_profiles()
-> Result<aws_runtime::env_config::section::EnvConfigSections, anyhow::Error> {
    let fs = aws_types::os_shim_internal::Fs::real();
    let env = aws_types::os_shim_internal::Env::real();
    let profile_files = aws_runtime::env_config::file::EnvConfigFiles::default();
    let conf = aws_config::profile::load(&fs, &env, &profile_files, None).await;
    match conf {
        Ok(profiles) => Ok(profiles),
        Err(e) => {
            if let Some(home) = dirs::home_dir() {
                let aws_folder = home.join(".aws");
                if aws_folder.exists() {
                    let config_file = aws_folder.join("config");
                    if config_file.exists() {
                        let config_file_str = std::fs::read_to_string(config_file);
                        if let Err(e) = config_file_str {
                            return Err(anyhow!(
                                "Failed to read AWS config file: {}",
                                e.to_string()
                            ));
                        }
                    } else {
                        return Err(anyhow!("AWS config file does not exist"));
                    }
                } else {
                    return Err(anyhow!("AWS folder does not exist in home directory"));
                }
            } else {
                return Err(anyhow!("Failed to get home directory"));
            };
            Err(anyhow!("Failed to load AWS profiles: {}", e.to_string()))
        }
    }
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
