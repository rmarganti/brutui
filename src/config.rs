use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;

use crate::ui::theme::{PartialThemeConfig, ThemeConfig};

pub const CONFIG_ENV_VAR: &str = "BRUTUI_CONFIG";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub collection_dirs: Vec<PathBuf>,
    pub bru_path: Option<PathBuf>,
    pub theme: ThemeConfig,
}

#[derive(Debug, Deserialize)]
struct RawAppConfig {
    #[serde(default)]
    collection_dirs: Vec<PathBuf>,
    bru_path: Option<PathBuf>,
    #[serde(default)]
    theme: PartialThemeConfig,
}

impl<'de> Deserialize<'de> for AppConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAppConfig::deserialize(deserializer)?;
        Ok(Self {
            collection_dirs: raw.collection_dirs,
            bru_path: raw.bru_path,
            theme: ThemeConfig::default().merge(raw.theme),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: Option<PathBuf>,
    pub config: AppConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config override {path} from {env_var} does not exist")]
    ConfigOverrideNotFound {
        env_var: &'static str,
        path: PathBuf,
    },
    #[error("failed to read config file {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("configured collection directory {path} does not exist or is not a directory")]
    InvalidCollectionDir { path: PathBuf },
    #[error("configured bru executable {path} does not exist or is not a file")]
    InvalidBruPath { path: PathBuf },
}

pub fn default_config_path() -> Option<PathBuf> {
    ProjectDirs::from("dev", "rmarganti", "brutui")
        .map(|dirs| dirs.config_dir().join("config.toml"))
}

pub fn resolved_config_path() -> Option<PathBuf> {
    std::env::var_os(CONFIG_ENV_VAR)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(default_config_path)
}

pub fn load() -> Result<LoadedConfig, ConfigError> {
    if let Some(path) = std::env::var_os(CONFIG_ENV_VAR)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        if !path.exists() {
            return Err(ConfigError::ConfigOverrideNotFound {
                env_var: CONFIG_ENV_VAR,
                path,
            });
        }

        return load_from_path(&path).map(|config| LoadedConfig {
            path: Some(path),
            config,
        });
    }

    let Some(path) = default_config_path() else {
        return Ok(LoadedConfig {
            path: None,
            config: AppConfig::default(),
        });
    };

    if !path.exists() {
        return Ok(LoadedConfig {
            path: Some(path),
            config: AppConfig::default(),
        });
    }

    load_from_path(&path).map(|config| LoadedConfig {
        path: Some(path),
        config,
    })
}

pub fn load_from_path(path: impl AsRef<Path>) -> Result<AppConfig, ConfigError> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let mut config: AppConfig = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    config.collection_dirs = config
        .collection_dirs
        .into_iter()
        .map(|dir| normalize_existing_dir(base_dir, &dir))
        .collect::<Result<Vec<_>, _>>()?;
    config.bru_path = config
        .bru_path
        .map(|path| normalize_existing_file(base_dir, &path))
        .transpose()?;

    Ok(config)
}

fn normalize_existing_dir(base_dir: &Path, path: &Path) -> Result<PathBuf, ConfigError> {
    let absolute = absolutize(base_dir, path);
    let normalized = absolute
        .canonicalize()
        .map_err(|_| ConfigError::InvalidCollectionDir {
            path: absolute.clone(),
        })?;

    if normalized.is_dir() {
        Ok(normalized)
    } else {
        Err(ConfigError::InvalidCollectionDir { path: normalized })
    }
}

fn normalize_existing_file(base_dir: &Path, path: &Path) -> Result<PathBuf, ConfigError> {
    let absolute = absolutize(base_dir, path);
    let normalized = absolute
        .canonicalize()
        .map_err(|_| ConfigError::InvalidBruPath {
            path: absolute.clone(),
        })?;

    if normalized.is_file() {
        Ok(normalized)
    } else {
        Err(ConfigError::InvalidBruPath { path: normalized })
    }
}

fn absolutize(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}
