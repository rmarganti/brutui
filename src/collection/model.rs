use std::path::{Path, PathBuf};

use crate::metadata::{MetadataDiagnostic, RequestMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionFormat {
    ClassicJson,
    OpenCollectionYaml,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub root: PathBuf,
    pub format: CollectionFormat,
    pub nodes: Vec<CollectionNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionNode {
    Root(RootNode),
    Folder(FolderNode),
    Request(RequestNode),
}

impl CollectionNode {
    pub fn id(&self) -> CollectionNodeId {
        match self {
            Self::Root(_) => CollectionNodeId::Root,
            Self::Folder(node) => CollectionNodeId::Folder(node.relative_path.clone()),
            Self::Request(node) => CollectionNodeId::Request(node.relative_path.clone()),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Root(node) => &node.path,
            Self::Folder(node) => &node.path,
            Self::Request(node) => &node.path,
        }
    }

    pub fn relative_path(&self) -> Option<&Path> {
        match self {
            Self::Root(_) => None,
            Self::Folder(node) => Some(&node.relative_path),
            Self::Request(node) => Some(&node.relative_path),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Root(node) => &node.display_name,
            Self::Folder(node) => &node.display_name,
            Self::Request(node) => &node.display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CollectionNodeId {
    Root,
    Folder(PathBuf),
    Request(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootNode {
    pub path: PathBuf,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderNode {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestNode {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub display_name: String,
    pub metadata: RequestMetadata,
    pub metadata_diagnostics: Vec<MetadataDiagnostic>,
}
