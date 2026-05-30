use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::metadata::{MetadataDiagnostic, RequestMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionFormat {
    ClassicJson,
    OpenCollectionYaml,
}

type NodeIndexMap = BTreeMap<CollectionNodeId, usize>;
type ParentIndexMap = BTreeMap<CollectionNodeId, CollectionNodeId>;
type ChildrenIndexMap = BTreeMap<CollectionNodeId, Vec<CollectionNodeId>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    root: PathBuf,
    format: CollectionFormat,
    nodes: Vec<CollectionNode>,
    by_id: NodeIndexMap,
    parent_by_id: ParentIndexMap,
    children_by_id: ChildrenIndexMap,
}

impl Collection {
    pub fn new(root: PathBuf, format: CollectionFormat, nodes: Vec<CollectionNode>) -> Self {
        let (by_id, parent_by_id, children_by_id) = build_indexes(&nodes);
        Self {
            root,
            format,
            nodes,
            by_id,
            parent_by_id,
            children_by_id,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn format(&self) -> &CollectionFormat {
        &self.format
    }

    pub fn visible_nodes(&self) -> &[CollectionNode] {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_at(&self, index: usize) -> Option<&CollectionNode> {
        self.nodes.get(index)
    }

    pub fn node_index(&self, id: &CollectionNodeId) -> Option<usize> {
        self.by_id.get(id).copied()
    }

    pub fn node(&self, id: &CollectionNodeId) -> Option<&CollectionNode> {
        self.node_index(id).and_then(|index| self.node_at(index))
    }

    pub fn contains_node(&self, id: &CollectionNodeId) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn parent_id(&self, id: &CollectionNodeId) -> Option<&CollectionNodeId> {
        self.parent_by_id.get(id)
    }

    pub fn child_ids(&self, id: &CollectionNodeId) -> &[CollectionNodeId] {
        self.children_by_id
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

fn build_indexes(nodes: &[CollectionNode]) -> (NodeIndexMap, ParentIndexMap, ChildrenIndexMap) {
    let mut by_id = BTreeMap::new();
    let mut parent_by_id = BTreeMap::new();
    let mut children_by_id: ChildrenIndexMap = BTreeMap::new();
    let mut ancestry = Vec::<(usize, CollectionNodeId)>::new();

    for (index, node) in nodes.iter().enumerate() {
        let node_id = node.id();
        let depth = node.depth();
        children_by_id.entry(node_id.clone()).or_default();

        while ancestry
            .last()
            .is_some_and(|(ancestor_depth, _)| *ancestor_depth >= depth)
        {
            ancestry.pop();
        }

        if let Some((_, parent_id)) = ancestry.last() {
            parent_by_id.insert(node_id.clone(), parent_id.clone());
            children_by_id
                .entry(parent_id.clone())
                .or_default()
                .push(node_id.clone());
        }

        by_id.insert(node_id.clone(), index);
        ancestry.push((depth, node_id));
    }

    (by_id, parent_by_id, children_by_id)
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

    pub fn depth(&self) -> usize {
        self.relative_path()
            .map(|path| path.components().count())
            .unwrap_or(0)
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::metadata::RequestMetadata;

    use super::{
        Collection, CollectionFormat, CollectionNode, CollectionNodeId, FolderNode, RequestNode,
        RootNode,
    };

    #[test]
    fn indexed_collection_preserves_visible_order_and_relationships() {
        let root = PathBuf::from("/collections/demo");
        let collection = Collection::new(
            root.clone(),
            CollectionFormat::ClassicJson,
            vec![
                CollectionNode::Root(RootNode {
                    path: root.clone(),
                    display_name: "demo".to_string(),
                }),
                CollectionNode::Folder(FolderNode {
                    path: root.join("users"),
                    relative_path: PathBuf::from("users"),
                    display_name: "users".to_string(),
                }),
                CollectionNode::Folder(FolderNode {
                    path: root.join("users/admin"),
                    relative_path: PathBuf::from("users/admin"),
                    display_name: "admin".to_string(),
                }),
                CollectionNode::Request(RequestNode {
                    path: root.join("users/admin/delete.bru"),
                    relative_path: PathBuf::from("users/admin/delete.bru"),
                    display_name: "delete.bru".to_string(),
                    metadata: RequestMetadata::default(),
                    metadata_diagnostics: Vec::new(),
                }),
                CollectionNode::Request(RequestNode {
                    path: root.join("users/list.bru"),
                    relative_path: PathBuf::from("users/list.bru"),
                    display_name: "list.bru".to_string(),
                    metadata: RequestMetadata::default(),
                    metadata_diagnostics: Vec::new(),
                }),
            ],
        );

        let visible_ids = collection
            .visible_nodes()
            .iter()
            .map(CollectionNode::id)
            .collect::<Vec<_>>();
        assert_eq!(
            visible_ids,
            vec![
                CollectionNodeId::Root,
                CollectionNodeId::Folder(PathBuf::from("users")),
                CollectionNodeId::Folder(PathBuf::from("users/admin")),
                CollectionNodeId::Request(PathBuf::from("users/admin/delete.bru")),
                CollectionNodeId::Request(PathBuf::from("users/list.bru")),
            ]
        );

        assert_eq!(collection.node_index(&CollectionNodeId::Root), Some(0));
        assert_eq!(
            collection.parent_id(&CollectionNodeId::Folder(PathBuf::from("users"))),
            Some(&CollectionNodeId::Root)
        );
        assert_eq!(
            collection.parent_id(&CollectionNodeId::Folder(PathBuf::from("users/admin"))),
            Some(&CollectionNodeId::Folder(PathBuf::from("users")))
        );
        assert_eq!(
            collection.parent_id(&CollectionNodeId::Request(PathBuf::from("users/list.bru"))),
            Some(&CollectionNodeId::Folder(PathBuf::from("users")))
        );
        assert_eq!(
            collection.child_ids(&CollectionNodeId::Root),
            &[CollectionNodeId::Folder(PathBuf::from("users"))]
        );
        assert_eq!(
            collection.child_ids(&CollectionNodeId::Folder(PathBuf::from("users"))),
            &[
                CollectionNodeId::Folder(PathBuf::from("users/admin")),
                CollectionNodeId::Request(PathBuf::from("users/list.bru")),
            ]
        );
    }
}
