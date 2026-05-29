mod support;

use std::fs;

use brutui::collection::model::CollectionFormat;
use brutui::environments::{EnvironmentOption, discover};
use tempfile::tempdir;

#[test]
fn discovers_classic_collection_local_environments() {
    let collection = support::copy_fixture_collection("classic", "sample-classic");

    let environments = discover(&collection.root, CollectionFormat::ClassicJson)
        .expect("discover classic environments");

    assert_eq!(
        environments,
        vec![
            EnvironmentOption::no_environment(),
            EnvironmentOption {
                display_name: "dev".to_string(),
                cli_value: Some("dev".to_string()),
            },
            EnvironmentOption {
                display_name: "prod".to_string(),
                cli_value: Some("prod".to_string()),
            },
        ]
    );
    collection.assert_unchanged();
}

#[test]
fn discovers_open_collection_environments_and_uses_yaml_name_for_display() {
    let collection = support::copy_fixture_collection("opencollection", "sample-open");
    let custom_named = collection.root.join("environments/custom.yml");
    fs::write(
        &custom_named,
        "name: Custom Display\nvalues:\n  baseUrl: https://custom.example.com\n",
    )
    .expect("write custom env");

    let environments = discover(&collection.root, CollectionFormat::OpenCollectionYaml)
        .expect("discover open environments");

    assert_eq!(
        environments,
        vec![
            EnvironmentOption::no_environment(),
            EnvironmentOption {
                display_name: "Custom Display".to_string(),
                cli_value: Some("custom".to_string()),
            },
            EnvironmentOption {
                display_name: "dev".to_string(),
                cli_value: Some("dev".to_string()),
            },
            EnvironmentOption {
                display_name: "staging".to_string(),
                cli_value: Some("staging".to_string()),
            },
        ]
    );
}

#[test]
fn excludes_unsupported_environment_files_and_directories() {
    let workspace = tempdir().expect("temp dir");
    let root = workspace.path().join("collection");
    let env_dir = root.join("environments");
    fs::create_dir_all(env_dir.join("nested")).expect("create env dir");
    fs::write(root.join("bruno.json"), "{}\n").expect("write root marker");
    fs::write(env_dir.join("valid.bru"), "vars {}\n").expect("write valid env");
    fs::write(env_dir.join("ignored.yml"), "name: ignored\n").expect("write ignored env");
    fs::write(env_dir.join("nested/also-ignored.bru"), "vars {}\n").expect("write nested env");

    let environments =
        discover(&root, CollectionFormat::ClassicJson).expect("discover classic environments");

    assert_eq!(
        environments,
        vec![
            EnvironmentOption::no_environment(),
            EnvironmentOption {
                display_name: "valid".to_string(),
                cli_value: Some("valid".to_string()),
            },
        ]
    );
}

#[test]
fn returns_explicit_no_environment_when_environment_directory_is_missing_or_empty() {
    let workspace = tempdir().expect("temp dir");
    let classic_root = workspace.path().join("classic");
    fs::create_dir_all(&classic_root).expect("create classic root");
    fs::write(classic_root.join("bruno.json"), "{}\n").expect("write classic marker");

    assert_eq!(
        discover(&classic_root, CollectionFormat::ClassicJson).expect("discover without env dir"),
        vec![EnvironmentOption::no_environment()]
    );

    let open_root = workspace.path().join("open");
    fs::create_dir_all(open_root.join("environments")).expect("create open env dir");
    fs::write(open_root.join("opencollection.yml"), "name: demo\n").expect("write open marker");

    assert_eq!(
        discover(&open_root, CollectionFormat::OpenCollectionYaml).expect("discover empty env dir"),
        vec![EnvironmentOption::no_environment()]
    );
}
