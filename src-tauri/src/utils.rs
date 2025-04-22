use anyhow::anyhow;
use chrono::{DateTime, Utc};

use crate::aws::config::AwsConfigSections;

pub(crate) fn fetch_profiles_new() -> Result<AwsConfigSections, anyhow::Error> {
    AwsConfigSections::parse()
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
