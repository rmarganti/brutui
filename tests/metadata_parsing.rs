mod support;

use std::fs;

use brutui::collection::model::{CollectionFormat, CollectionNode};
use brutui::collection::scanner::scan_collection;
use brutui::metadata::parse_request_file;
use support::copy_fixture_collection;

#[test]
fn parses_classic_request_metadata_from_fixture_files() {
    let fixture = copy_fixture_collection("classic", "sample-classic");
    let parsed = parse_request_file(
        &fixture.root.join("users/list.bru"),
        CollectionFormat::ClassicJson,
    );

    assert_eq!(parsed.metadata.name.as_deref(), Some("List Users"));
    assert_eq!(parsed.metadata.method.as_deref(), Some("GET"));
    assert_eq!(parsed.metadata.url.as_deref(), Some("{{baseUrl}}/users"));
    assert_eq!(parsed.metadata.tags, vec!["team:platform", "smoke"]);
    assert!(parsed.diagnostics.is_empty());

    fixture.assert_unchanged();
}

#[test]
fn parses_open_collection_metadata_and_missing_fields_best_effort() {
    let fixture = copy_fixture_collection("opencollection", "sample-open");
    let parsed = parse_request_file(
        &fixture.root.join("requests/auth/login.yml"),
        CollectionFormat::OpenCollectionYaml,
    );

    assert_eq!(parsed.metadata.name.as_deref(), Some("Login"));
    assert_eq!(parsed.metadata.method.as_deref(), Some("POST"));
    assert_eq!(parsed.metadata.url.as_deref(), Some("{{baseUrl}}/login"));
    assert!(parsed.metadata.tags.is_empty());
    assert!(parsed.diagnostics.is_empty());

    fixture.assert_unchanged();
}

#[test]
fn malformed_request_metadata_returns_diagnostics_without_hiding_the_request() {
    let fixture = copy_fixture_collection("classic", "sample-classic");
    let parsed = parse_request_file(
        &fixture.root.join("broken/malformed.bru"),
        CollectionFormat::ClassicJson,
    );

    assert_eq!(parsed.metadata.name.as_deref(), Some("Broken Request"));
    assert_eq!(parsed.metadata.method.as_deref(), Some("GET"));
    assert_eq!(
        parsed.metadata.url.as_deref(),
        Some("https://example.com/bad")
    );
    assert!(!parsed.diagnostics.is_empty());

    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let broken_request = collection
        .nodes
        .iter()
        .find_map(|node| match node {
            CollectionNode::Request(request)
                if request.relative_path == std::path::PathBuf::from("broken/malformed.bru") =>
            {
                Some(request)
            }
            _ => None,
        })
        .expect("malformed request should remain visible");

    assert_eq!(broken_request.display_name, "malformed.bru");
    assert_eq!(
        broken_request.metadata.name.as_deref(),
        Some("Broken Request")
    );
    assert!(!broken_request.metadata_diagnostics.is_empty());

    fixture.assert_unchanged();
}

#[test]
fn unsupported_yaml_tag_shapes_report_diagnostics_but_keep_other_fields() {
    let fixture = copy_fixture_collection("opencollection", "sample-open");
    let request_path = fixture.root.join("requests/catalog/unsupported-tags.yml");
    fs::write(
        &request_path,
        "name: Unsupported Tags\nmethod: GET\nurl: /tags\ntags: smoke\n",
    )
    .expect("write request fixture");

    let parsed = parse_request_file(&request_path, CollectionFormat::OpenCollectionYaml);

    assert_eq!(parsed.metadata.name.as_deref(), Some("Unsupported Tags"));
    assert_eq!(parsed.metadata.method.as_deref(), Some("GET"));
    assert_eq!(parsed.metadata.url.as_deref(), Some("/tags"));
    assert!(parsed.metadata.tags.is_empty());
    assert_eq!(parsed.diagnostics.len(), 1);
}
