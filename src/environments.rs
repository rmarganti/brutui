use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::collection::model::CollectionFormat;

const ENVIRONMENTS_DIR: &str = "environments";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentOption {
    pub display_name: String,
    pub cli_value: Option<String>,
}

impl EnvironmentOption {
    pub fn no_environment() -> Self {
        Self {
            display_name: "No environment".to_string(),
            cli_value: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum EnvironmentDiscoveryError {
    #[error("failed to read environments directory {path}: {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to inspect environment entry in {path}: {source}")]
    ReadDirectoryEntry {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("failed to read environment file {path}: {source}")]
    ReadEnvironmentFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub fn discover(
    collection_root: &Path,
    format: CollectionFormat,
) -> Result<Vec<EnvironmentOption>, EnvironmentDiscoveryError> {
    let environments_dir = collection_root.join(ENVIRONMENTS_DIR);
    let mut options = vec![EnvironmentOption::no_environment()];

    if !environments_dir.is_dir() {
        return Ok(options);
    }

    let mut discovered = match format {
        CollectionFormat::ClassicJson => discover_classic(&environments_dir)?,
        CollectionFormat::OpenCollectionYaml => discover_open_collection(&environments_dir)?,
    };
    discovered.sort_by(|left, right| {
        left.display_name
            .cmp(&right.display_name)
            .then(left.cli_value.cmp(&right.cli_value))
    });
    options.extend(discovered);

    Ok(options)
}

fn discover_classic(
    environments_dir: &Path,
) -> Result<Vec<EnvironmentOption>, EnvironmentDiscoveryError> {
    let mut options = Vec::new();

    for entry in fs::read_dir(environments_dir).map_err(|source| {
        EnvironmentDiscoveryError::ReadDirectory {
            path: environments_dir.to_path_buf(),
            source,
        }
    })? {
        let entry = entry.map_err(|source| EnvironmentDiscoveryError::ReadDirectoryEntry {
            path: environments_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("bru") {
            continue;
        }

        let Some(name) = file_stem_string(&path) else {
            continue;
        };
        options.push(EnvironmentOption {
            display_name: name.clone(),
            cli_value: Some(name),
        });
    }

    Ok(options)
}

fn discover_open_collection(
    environments_dir: &Path,
) -> Result<Vec<EnvironmentOption>, EnvironmentDiscoveryError> {
    let mut options = Vec::new();

    for entry in fs::read_dir(environments_dir).map_err(|source| {
        EnvironmentDiscoveryError::ReadDirectory {
            path: environments_dir.to_path_buf(),
            source,
        }
    })? {
        let entry = entry.map_err(|source| EnvironmentDiscoveryError::ReadDirectoryEntry {
            path: environments_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() || !is_yaml_path(&path) {
            continue;
        }

        let Some(cli_value) = file_stem_string(&path) else {
            continue;
        };
        let display_name =
            parse_open_collection_env_name(&path)?.unwrap_or_else(|| cli_value.clone());
        options.push(EnvironmentOption {
            display_name,
            cli_value: Some(cli_value),
        });
    }

    Ok(options)
}

fn parse_open_collection_env_name(
    path: &Path,
) -> Result<Option<String>, EnvironmentDiscoveryError> {
    let contents = fs::read_to_string(path).map_err(|source| {
        EnvironmentDiscoveryError::ReadEnvironmentFile {
            path: path.to_path_buf(),
            source,
        }
    })?;

    Ok(contents.lines().find_map(parse_top_level_name_line))
}

fn parse_top_level_name_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') || trimmed.starts_with('-') {
        return None;
    }

    let (key, value) = trimmed.split_once(':')?;
    if key.trim() != "name" {
        return None;
    }

    let value = value.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn file_stem_string(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
}

fn is_yaml_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("yml" | "yaml")
    )
}

#[cfg(test)]
mod tests {
    use super::{EnvironmentOption, parse_top_level_name_line};

    #[test]
    fn no_environment_option_is_explicit() {
        let env = EnvironmentOption::no_environment();

        assert_eq!(env.display_name, "No environment");
        assert_eq!(env.cli_value, None);
    }

    #[test]
    fn parses_top_level_yaml_name_lines() {
        assert_eq!(
            parse_top_level_name_line("name: staging"),
            Some("staging".to_string())
        );
        assert_eq!(
            parse_top_level_name_line("name: \"quoted\""),
            Some("quoted".to_string())
        );
        assert_eq!(parse_top_level_name_line("  values:"), None);
        assert_eq!(parse_top_level_name_line("- name: nested"), None);
    }
}
