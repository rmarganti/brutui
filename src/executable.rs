use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::AppConfig;

pub const BRU_PATH_ENV_VAR: &str = "BRUTUI_BRU_PATH";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BruExecutableSource {
    Environment,
    Config,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BruExecutable {
    pub path: PathBuf,
    pub source: BruExecutableSource,
}

#[derive(Debug, Error)]
pub enum BruExecutableError {
    #[error("{env_var} points to {path}, but it does not exist or is not executable")]
    InvalidEnvironmentOverride {
        env_var: &'static str,
        path: PathBuf,
    },
    #[error("configured bru executable {path} does not exist or is not executable")]
    InvalidConfiguredPath { path: PathBuf },
    #[error("could not find `bru`; set {env_var}, configure `bru_path`, or add `bru` to PATH")]
    NotFound { env_var: &'static str },
}

pub fn resolve(config: &AppConfig) -> Result<BruExecutable, BruExecutableError> {
    if let Some(path) = env::var_os(BRU_PATH_ENV_VAR)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return validate_candidate(path, BruExecutableSource::Environment).map_err(|_| {
            BruExecutableError::InvalidEnvironmentOverride {
                env_var: BRU_PATH_ENV_VAR,
                path: env::var_os(BRU_PATH_ENV_VAR)
                    .map(PathBuf::from)
                    .unwrap_or_default(),
            }
        });
    }

    if let Some(path) = config.bru_path.clone() {
        return validate_candidate(path.clone(), BruExecutableSource::Config)
            .map_err(|_| BruExecutableError::InvalidConfiguredPath { path });
    }

    for directory in env::var_os("PATH")
        .as_deref()
        .map(env::split_paths)
        .into_iter()
        .flatten()
    {
        for candidate in bru_binary_names().iter().map(|name| directory.join(name)) {
            if let Ok(executable) = validate_candidate(candidate, BruExecutableSource::Path) {
                return Ok(executable);
            }
        }
    }

    Err(BruExecutableError::NotFound {
        env_var: BRU_PATH_ENV_VAR,
    })
}

fn validate_candidate(
    path: PathBuf,
    source: BruExecutableSource,
) -> Result<BruExecutable, BruExecutableError> {
    if is_executable(&path) {
        Ok(BruExecutable { path, source })
    } else {
        Err(BruExecutableError::NotFound {
            env_var: BRU_PATH_ENV_VAR,
        })
    }
}

fn bru_binary_names() -> &'static [&'static str] {
    #[cfg(windows)]
    {
        &["bru.exe", "bru.bat", "bru.cmd", "bru"]
    }

    #[cfg(not(windows))]
    {
        &["bru"]
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };

    if !metadata.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(unix))]
    {
        true
    }
}
