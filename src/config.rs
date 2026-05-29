use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

pub const CONFIG_ENV_VAR: &str = "BRUTUI_CONFIG";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub collection_dirs: Vec<PathBuf>,
    pub bru_path: Option<PathBuf>,
}

pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "rmarganti", "brutui")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}
