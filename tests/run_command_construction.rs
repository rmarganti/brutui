#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::PathBuf;

use brutui::collection::model::{CollectionFormat, CollectionNodeId};
use brutui::collection::scanner::scan_collection;
use brutui::environments::EnvironmentOption;
use brutui::runner::build_run_command;
use fixtures::copy_fixture_collection;

#[test]
fn root_run_uses_recursive_collection_target_without_env() {
    let fixture = copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");

    let command = build_run_command(
        "/usr/local/bin/bru",
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build root command");

    assert_eq!(command.program, PathBuf::from("/usr/local/bin/bru"));
    assert_eq!(command.args[0], "run");
    assert_eq!(command.args[1], fixture.root.to_string_lossy());
    assert!(command.args.contains(&"-r".to_string()));
    assert!(!command.args.iter().any(|arg| arg == "--env"));
    assert_reporter_json_arg(&command.args, &command.report_path);
    assert_report_path_is_temp(&fixture.root, &command.report_path);
    fixture.assert_unchanged();
}

#[test]
fn folder_run_is_recursive_and_request_run_is_not() {
    let fixture = copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");

    let folder_command = build_run_command(
        "bru",
        &collection,
        &CollectionNodeId::Folder(PathBuf::from("users")),
        &EnvironmentOption::no_environment(),
    )
    .expect("build folder command");
    let request_command = build_run_command(
        "bru",
        &collection,
        &CollectionNodeId::Request(PathBuf::from("users/list.bru")),
        &EnvironmentOption::no_environment(),
    )
    .expect("build request command");

    assert_eq!(folder_command.args[0], "run");
    assert_eq!(
        folder_command.args[1],
        fixture.root.join("users").to_string_lossy()
    );
    assert!(folder_command.args.contains(&"-r".to_string()));

    assert_eq!(request_command.args[0], "run");
    assert_eq!(
        request_command.args[1],
        fixture.root.join("users/list.bru").to_string_lossy()
    );
    assert!(!request_command.args.contains(&"-r".to_string()));

    fixture.assert_unchanged();
}

#[test]
fn selected_environment_is_mapped_to_bru_env_flag() {
    let fixture = copy_fixture_collection("opencollection", "sample-open");
    let collection = scan_collection(&fixture.root, CollectionFormat::OpenCollectionYaml)
        .expect("scan open fixture");
    let env = EnvironmentOption {
        display_name: "staging".to_string(),
        cli_value: Some("staging".to_string()),
    };

    let command = build_run_command(
        "bru",
        &collection,
        &CollectionNodeId::Request(PathBuf::from("requests/catalog/get-products.yml")),
        &env,
    )
    .expect("build request command with env");

    let env_index = command
        .args
        .iter()
        .position(|arg| arg == "--env")
        .expect("--env flag present");
    assert_eq!(command.args[env_index + 1], "staging");
    assert_reporter_json_arg(&command.args, &command.report_path);
    assert_report_path_is_temp(&fixture.root, &command.report_path);
}

fn assert_reporter_json_arg(args: &[String], report_path: &std::path::Path) {
    let report_index = args
        .iter()
        .position(|arg| arg == "--reporter-json")
        .expect("--reporter-json flag present");
    assert_eq!(args[report_index + 1], report_path.to_string_lossy());
}

fn assert_report_path_is_temp(collection_root: &std::path::Path, report_path: &std::path::Path) {
    assert!(report_path.is_absolute());
    assert!(report_path.starts_with(std::env::temp_dir()));
    assert!(!report_path.starts_with(collection_root));
}
