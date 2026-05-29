use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use crate::collection::model::CollectionFormat;

pub const CLASSIC_ROOT_MARKER: &str = "bruno.json";
pub const OPEN_COLLECTION_ROOT_MARKER: &str = "opencollection.yml";
pub const CONFIG_DISCOVERY_MAX_DEPTH: usize = 4;
const IGNORED_DIRECTORY_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "dist",
    ".next",
    ".turbo",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverySource {
    ExplicitPath,
    CurrentWorkingDirectory,
    ConfiguredDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCollection {
    pub root: PathBuf,
    pub format: CollectionFormat,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveryRequest {
    pub explicit_path: Option<PathBuf>,
    pub cwd: PathBuf,
    pub configured_dirs: Vec<PathBuf>,
}

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("failed to inspect collection path {path}: {source}")]
    InspectPath {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("explicit collection path {path} is not a Bruno collection root")]
    ExplicitPathNotCollectionRoot { path: PathBuf },
    #[error("current working directory {path} does not exist")]
    MissingCurrentWorkingDirectory { path: PathBuf },
    #[error("failed to walk configured directory {path}: {source}")]
    WalkConfiguredDirectory {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
}

pub fn discover(request: &DiscoveryRequest) -> Result<Vec<DiscoveredCollection>, DiscoveryError> {
    if let Some(explicit_path) = &request.explicit_path {
        return discover_explicit(explicit_path).map(|collection| vec![collection]);
    }

    if let Some(collection) = discover_from_cwd(&request.cwd)? {
        return Ok(vec![collection]);
    }

    discover_from_configured_dirs(&request.configured_dirs)
}

pub fn detect_collection_root(path: &Path) -> Result<Option<CollectionFormat>, io::Error> {
    if !path.is_dir() {
        return Ok(None);
    }

    let classic_marker = path.join(CLASSIC_ROOT_MARKER);
    if fs::metadata(&classic_marker)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return Ok(Some(CollectionFormat::ClassicJson));
    }

    let open_marker = path.join(OPEN_COLLECTION_ROOT_MARKER);
    if fs::metadata(&open_marker)
        .map(|metadata| metadata.is_file())
        .unwrap_or(false)
    {
        return Ok(Some(CollectionFormat::OpenCollectionYaml));
    }

    Ok(None)
}

fn discover_explicit(path: &Path) -> Result<DiscoveredCollection, DiscoveryError> {
    let absolute = absolute_path(path);
    let Some(format) =
        detect_collection_root(&absolute).map_err(|source| DiscoveryError::InspectPath {
            path: absolute.clone(),
            source,
        })?
    else {
        return Err(DiscoveryError::ExplicitPathNotCollectionRoot { path: absolute });
    };

    Ok(DiscoveredCollection {
        root: absolute,
        format,
        source: DiscoverySource::ExplicitPath,
    })
}

fn discover_from_cwd(cwd: &Path) -> Result<Option<DiscoveredCollection>, DiscoveryError> {
    let cwd = absolute_path(cwd);
    if !cwd.exists() {
        return Err(DiscoveryError::MissingCurrentWorkingDirectory { path: cwd });
    }

    for candidate in cwd.ancestors() {
        if let Some(format) =
            detect_collection_root(candidate).map_err(|source| DiscoveryError::InspectPath {
                path: candidate.to_path_buf(),
                source,
            })?
        {
            return Ok(Some(DiscoveredCollection {
                root: candidate.to_path_buf(),
                format,
                source: DiscoverySource::CurrentWorkingDirectory,
            }));
        }
    }

    Ok(None)
}

fn discover_from_configured_dirs(
    configured_dirs: &[PathBuf],
) -> Result<Vec<DiscoveredCollection>, DiscoveryError> {
    let mut discovered = Vec::new();
    let mut seen = BTreeSet::new();

    for configured_dir in configured_dirs {
        let configured_dir = absolute_path(configured_dir);
        let walker = WalkDir::new(&configured_dir)
            .follow_links(false)
            .max_depth(CONFIG_DISCOVERY_MAX_DEPTH + 1)
            .into_iter()
            .filter_entry(|entry| should_visit(entry, &configured_dir));

        for entry in walker {
            let entry = entry.map_err(|source| DiscoveryError::WalkConfiguredDirectory {
                path: configured_dir.clone(),
                source,
            })?;
            if !entry.file_type().is_dir() {
                continue;
            }

            let path = entry.into_path();
            let Some(format) =
                detect_collection_root(&path).map_err(|source| DiscoveryError::InspectPath {
                    path: path.clone(),
                    source,
                })?
            else {
                continue;
            };

            if seen.insert(path.clone()) {
                discovered.push(DiscoveredCollection {
                    root: path,
                    format,
                    source: DiscoverySource::ConfiguredDirectory,
                });
            }
        }
    }

    discovered.sort_by(|left, right| left.root.cmp(&right.root));
    Ok(discovered)
}

fn should_visit(entry: &DirEntry, _configured_dir: &Path) -> bool {
    if entry.depth() == 0 {
        return true;
    }

    if entry.depth() > CONFIG_DISCOVERY_MAX_DEPTH {
        return false;
    }

    if !entry.file_type().is_dir() {
        return true;
    }

    entry
        .file_name()
        .to_str()
        .map(|name| !IGNORED_DIRECTORY_NAMES.contains(&name))
        .unwrap_or(true)
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("current working directory")
            .join(path)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{
        CLASSIC_ROOT_MARKER, CONFIG_DISCOVERY_MAX_DEPTH, DiscoveryRequest, DiscoverySource,
        OPEN_COLLECTION_ROOT_MARKER, detect_collection_root, discover,
    };
    use crate::collection::model::CollectionFormat;

    #[test]
    fn detects_supported_collection_root_markers() {
        let workspace = tempdir().expect("temp dir");
        let classic = workspace.path().join("classic");
        let open = workspace.path().join("open");
        fs::create_dir_all(&classic).expect("create classic");
        fs::create_dir_all(&open).expect("create open");
        fs::write(classic.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write classic marker");
        fs::write(open.join(OPEN_COLLECTION_ROOT_MARKER), "name: sample\n")
            .expect("write open marker");

        assert_eq!(
            detect_collection_root(&classic).expect("detect classic"),
            Some(CollectionFormat::ClassicJson)
        );
        assert_eq!(
            detect_collection_root(&open).expect("detect open"),
            Some(CollectionFormat::OpenCollectionYaml)
        );
    }

    #[test]
    fn explicit_path_overrides_other_discovery_sources() {
        let workspace = tempdir().expect("temp dir");
        let explicit = workspace.path().join("explicit");
        let configured = workspace.path().join("configured");
        fs::create_dir_all(&explicit).expect("create explicit");
        fs::create_dir_all(&configured).expect("create configured");
        fs::write(explicit.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write explicit marker");
        fs::write(configured.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write configured marker");

        let discovered = discover(&DiscoveryRequest {
            explicit_path: Some(explicit.clone()),
            cwd: configured.clone(),
            configured_dirs: vec![workspace.path().to_path_buf()],
        })
        .expect("discover explicit collection");

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].root, explicit);
        assert_eq!(discovered[0].source, DiscoverySource::ExplicitPath);
    }

    #[test]
    fn cwd_ancestry_overrides_configured_directory_discovery() {
        let workspace = tempdir().expect("temp dir");
        let cwd_root = workspace.path().join("collections/current");
        let nested = cwd_root.join("requests/users");
        let configured = workspace.path().join("configured/other");
        fs::create_dir_all(&nested).expect("create cwd nested");
        fs::create_dir_all(&configured).expect("create configured");
        fs::write(cwd_root.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write cwd marker");
        fs::write(configured.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write configured marker");

        let discovered = discover(&DiscoveryRequest {
            explicit_path: None,
            cwd: nested,
            configured_dirs: vec![workspace.path().join("configured")],
        })
        .expect("discover cwd collection");

        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].root, cwd_root);
        assert_eq!(
            discovered[0].source,
            DiscoverySource::CurrentWorkingDirectory
        );
    }

    #[test]
    fn configured_directory_discovery_returns_zero_one_or_many_results() {
        let workspace = tempdir().expect("temp dir");
        let empty = workspace.path().join("empty");
        fs::create_dir_all(&empty).expect("create empty");

        let none = discover(&DiscoveryRequest {
            explicit_path: None,
            cwd: empty.clone(),
            configured_dirs: vec![empty.clone()],
        })
        .expect("discover zero collections");
        assert!(none.is_empty());

        let single_root = workspace.path().join("single/sample");
        fs::create_dir_all(&single_root).expect("create single root");
        fs::write(single_root.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write single marker");
        let one = discover(&DiscoveryRequest {
            explicit_path: None,
            cwd: empty.clone(),
            configured_dirs: vec![workspace.path().join("single")],
        })
        .expect("discover one collection");
        assert_eq!(one.len(), 1);

        let second_root = workspace.path().join("many/open");
        let third_root = workspace.path().join("many/classic");
        fs::create_dir_all(&second_root).expect("create second root");
        fs::create_dir_all(&third_root).expect("create third root");
        fs::write(
            second_root.join(OPEN_COLLECTION_ROOT_MARKER),
            "name: sample\n",
        )
        .expect("write open marker");
        fs::write(third_root.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write classic marker");
        let many = discover(&DiscoveryRequest {
            explicit_path: None,
            cwd: empty,
            configured_dirs: vec![workspace.path().join("many")],
        })
        .expect("discover many collections");
        assert_eq!(many.len(), 2);
        assert!(
            many.iter()
                .all(|collection| collection.source == DiscoverySource::ConfiguredDirectory)
        );
    }

    #[test]
    fn configured_discovery_is_bounded_and_ignores_irrelevant_directories() {
        let workspace = tempdir().expect("temp dir");
        let root = workspace.path().join("search-root");
        let allowed = root.join("level1/level2/within-depth");
        let too_deep = root
            .join("a")
            .join("b")
            .join("c")
            .join("d")
            .join("beyond-depth");
        let ignored = root.join("node_modules/ignored-collection");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&allowed).expect("create allowed root");
        fs::create_dir_all(&too_deep).expect("create deep root");
        fs::create_dir_all(&ignored).expect("create ignored root");
        fs::create_dir_all(&outside).expect("create outside dir");
        fs::write(allowed.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write allowed marker");
        fs::write(too_deep.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write deep marker");
        fs::write(ignored.join(CLASSIC_ROOT_MARKER), "{}\n").expect("write ignored marker");

        let discovered = discover(&DiscoveryRequest {
            explicit_path: None,
            cwd: outside,
            configured_dirs: vec![root],
        })
        .expect("discover configured collections");

        assert_eq!(CONFIG_DISCOVERY_MAX_DEPTH, 4);
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].root, allowed);
    }

    #[test]
    fn discovery_rejects_directories_without_supported_root_markers() {
        let workspace = tempdir().expect("temp dir");
        let almost = workspace.path().join("almost");
        fs::create_dir_all(&almost).expect("create almost root");
        fs::write(almost.join("collection.json"), "{}\n").expect("write wrong marker");

        assert_eq!(detect_collection_root(&almost).expect("detect root"), None);
    }
}
