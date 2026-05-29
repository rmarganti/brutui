use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::collection::model::{
    Collection, CollectionFormat, CollectionNode, FolderNode, RequestNode, RootNode,
};
use crate::metadata::parse_request_file;

const IGNORED_DIRECTORY_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "build",
    "dist",
    ".next",
    ".turbo",
    "environments",
];

#[derive(Debug, Error)]
pub enum CollectionScanError {
    #[error("collection root {path} does not exist")]
    MissingRoot { path: PathBuf },
    #[error("collection root {path} is not a directory")]
    RootIsNotDirectory { path: PathBuf },
    #[error("failed to read directory {path}: {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub fn scan_collection(
    root: &Path,
    format: CollectionFormat,
) -> Result<Collection, CollectionScanError> {
    let root = absolute_path(root);
    let metadata = fs::metadata(&root).map_err(|source| match source.kind() {
        io::ErrorKind::NotFound => CollectionScanError::MissingRoot { path: root.clone() },
        _ => CollectionScanError::ReadDirectory {
            path: root.clone(),
            source,
        },
    })?;

    if !metadata.is_dir() {
        return Err(CollectionScanError::RootIsNotDirectory { path: root });
    }

    let mut nodes = vec![CollectionNode::Root(RootNode {
        path: root.clone(),
        display_name: root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string()),
    })];

    scan_directory(&root, &root, &format, &mut nodes)?;

    Ok(Collection {
        root,
        format,
        nodes,
    })
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    format: &CollectionFormat,
    nodes: &mut Vec<CollectionNode>,
) -> Result<(), CollectionScanError> {
    let mut child_dirs = Vec::new();
    let mut child_requests = Vec::new();

    for entry in fs::read_dir(directory).map_err(|source| CollectionScanError::ReadDirectory {
        path: directory.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| CollectionScanError::ReadDirectory {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| CollectionScanError::ReadDirectory {
                path: path.clone(),
                source,
            })?;

        if file_type.is_dir() {
            if should_include_directory(&path) {
                child_dirs.push(path);
            }
            continue;
        }

        if file_type.is_file() && is_request_file(&path, format) {
            child_requests.push(path);
        }
    }

    child_dirs.sort();
    child_requests.sort();

    for path in child_dirs {
        let relative_path = strip_root(root, &path);
        nodes.push(CollectionNode::Folder(FolderNode {
            path: path.clone(),
            relative_path: relative_path.clone(),
            display_name: file_name(&path),
        }));
        scan_directory(root, &path, format, nodes)?;
    }

    for path in child_requests {
        let parsed = parse_request_file(&path, format.clone());
        nodes.push(CollectionNode::Request(RequestNode {
            relative_path: strip_root(root, &path),
            display_name: file_name(&path),
            metadata: parsed.metadata,
            metadata_diagnostics: parsed.diagnostics,
            path,
        }));
    }

    Ok(())
}

fn should_include_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| !IGNORED_DIRECTORY_NAMES.contains(&name))
        .unwrap_or(true)
}

fn is_request_file(path: &Path, format: &CollectionFormat) -> bool {
    match format {
        CollectionFormat::ClassicJson => {
            path.extension().and_then(|extension| extension.to_str()) == Some("bru")
        }
        CollectionFormat::OpenCollectionYaml => {
            matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml") | Some("yaml")
            ) && path.file_name().and_then(|name| name.to_str()) != Some("opencollection.yml")
        }
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn strip_root(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .expect("path should stay within collection root")
        .to_path_buf()
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
