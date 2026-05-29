use std::path::PathBuf;

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
}
