#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::PathBuf;

use brutui::collection::model::{CollectionFormat, CollectionNode, CollectionNodeId};
use brutui::collection::scanner::scan_collection;
use fixtures::copy_fixture_collection;

#[test]
fn classic_scanner_builds_a_deterministic_filesystem_first_tree() {
    let fixture = copy_fixture_collection("classic", "sample-classic");

    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");

    let summary = node_summary(collection.visible_nodes());
    assert_eq!(
        summary,
        vec![
            ("root", None, fixture.root.clone()),
            (
                "folder",
                Some(PathBuf::from("broken")),
                fixture.root.join("broken"),
            ),
            (
                "request",
                Some(PathBuf::from("broken/malformed.bru")),
                fixture.root.join("broken/malformed.bru"),
            ),
            (
                "folder",
                Some(PathBuf::from("users")),
                fixture.root.join("users"),
            ),
            (
                "folder",
                Some(PathBuf::from("users/admin")),
                fixture.root.join("users/admin"),
            ),
            (
                "request",
                Some(PathBuf::from("users/admin/delete.bru")),
                fixture.root.join("users/admin/delete.bru"),
            ),
            (
                "request",
                Some(PathBuf::from("users/create.bru")),
                fixture.root.join("users/create.bru"),
            ),
            (
                "request",
                Some(PathBuf::from("users/list.bru")),
                fixture.root.join("users/list.bru"),
            ),
        ]
    );

    assert_eq!(
        collection.visible_nodes()[0].display_name(),
        "sample-classic"
    );
    assert_eq!(collection.visible_nodes()[1].display_name(), "broken");
    assert_eq!(
        collection.visible_nodes()[2].display_name(),
        "malformed.bru"
    );
    assert!(!summary.iter().any(|(_, relative_path, _)| {
        relative_path
            .as_ref()
            .is_some_and(|path| path.starts_with("environments"))
    }));
    assert!(!summary.iter().any(|(_, relative_path, _)| {
        relative_path
            .as_ref()
            .is_some_and(|path| path.starts_with("node_modules") || path.starts_with("target"))
    }));

    fixture.assert_unchanged();
}

#[test]
fn open_collection_scanner_includes_yaml_requests_but_not_root_or_environment_files() {
    let fixture = copy_fixture_collection("opencollection", "sample-open");

    let collection =
        scan_collection(&fixture.root, CollectionFormat::OpenCollectionYaml).expect("scan open");

    let summary = node_summary(collection.visible_nodes());
    assert_eq!(
        summary,
        vec![
            ("root", None, fixture.root.clone()),
            (
                "folder",
                Some(PathBuf::from("requests")),
                fixture.root.join("requests"),
            ),
            (
                "folder",
                Some(PathBuf::from("requests/auth")),
                fixture.root.join("requests/auth"),
            ),
            (
                "request",
                Some(PathBuf::from("requests/auth/login.yml")),
                fixture.root.join("requests/auth/login.yml"),
            ),
            (
                "folder",
                Some(PathBuf::from("requests/catalog")),
                fixture.root.join("requests/catalog"),
            ),
            (
                "request",
                Some(PathBuf::from("requests/catalog/get-products.yml")),
                fixture.root.join("requests/catalog/get-products.yml"),
            ),
        ]
    );

    assert!(summary.iter().all(|(_, relative_path, _)| {
        relative_path.as_ref() != Some(&PathBuf::from("opencollection.yml"))
    }));
    assert!(collection.visible_nodes().iter().any(|node| {
        node.id() == CollectionNodeId::Request(PathBuf::from("requests/auth/login.yml"))
    }));

    fixture.assert_unchanged();
}

fn node_summary(nodes: &[CollectionNode]) -> Vec<(&'static str, Option<PathBuf>, PathBuf)> {
    nodes
        .iter()
        .map(|node| match node {
            CollectionNode::Root(_) => ("root", None, node.path().to_path_buf()),
            CollectionNode::Folder(_) => (
                "folder",
                node.relative_path().map(|path| path.to_path_buf()),
                node.path().to_path_buf(),
            ),
            CollectionNode::Request(_) => (
                "request",
                node.relative_path().map(|path| path.to_path_buf()),
                node.path().to_path_buf(),
            ),
        })
        .collect()
}
