use std::collections::HashMap;

use crate::trace_err_ret;

pub(crate) struct Profile {
    pub(crate) name: String,
    pub(crate) properties: HashMap<String, String>,
}

impl Profile {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.properties
            .get(name.to_ascii_lowercase().as_str())
            .map(|prop| prop.as_str())
    }
}

pub(crate) struct Session {
    pub(crate) name: String,
    pub(crate) properties: HashMap<String, String>,
}

impl Session {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.properties
            .get(name.to_ascii_lowercase().as_str())
            .map(|prop| prop.as_str())
    }
}

pub(crate) struct AwsConfigSections {
    pub(crate) profiles: HashMap<String, Profile>,
    pub(crate) sessions: HashMap<String, Session>,
}

impl AwsConfigSections {
    pub(crate) fn parse() -> Result<Self, anyhow::Error> {
        let mut profiles = HashMap::new();
        let mut sessions = HashMap::new();

        let home_dir =
            dirs::home_dir().ok_or_else(|| trace_err_ret("No home directory detected!"))?;
        let config_path = home_dir.join(".aws").join("config");
        if !config_path.exists() {
            return Err(trace_err_ret(&format!(
                "No config file found at {:?}. Please configure accordingly!",
                config_path
            )));
        } else {
            let config_ini = ini::Ini::load_from_file(&config_path)?;
            for (section_name, section) in config_ini.iter() {
                let mut properties = HashMap::new();
                for (key, value) in section.iter() {
                    properties.insert(key.to_string(), value.to_string());
                }
                if let Some(sec_name) = section_name {
                    if sec_name.starts_with("profile ") {
                        let profile_name = sec_name.trim_start_matches("profile ");
                        profiles.insert(
                            profile_name.to_string(),
                            Profile {
                                name: profile_name.to_string(),
                                properties,
                            },
                        );
                    } else if sec_name.starts_with("sso-session ") {
                        let session_name = sec_name.trim_start_matches("sso-session ");
                        sessions.insert(
                            session_name.to_string(),
                            Session {
                                name: session_name.to_string(),
                                properties,
                            },
                        );
                    }
                }
            }
        }
        Ok(AwsConfigSections { profiles, sessions })
    }
}
