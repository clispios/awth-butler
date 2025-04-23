#[cfg(all(desktop, not(debug_assertions)))]
use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::anyhow;

#[cfg(all(desktop, not(debug_assertions)))]
pub(crate) static APP_CONFIG_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let config_dir = dirs::config_dir().unwrap();

    let app_config_dir = config_dir.join("Awth Butler");

    if !app_config_dir.exists() {
        fs::create_dir(&app_config_dir).unwrap();
    }

    app_config_dir
});

pub(crate) fn trace_err_ret(msg: &str) -> anyhow::Error {
    tracing::error!("{}", msg);
    anyhow!(msg.to_string())
}
