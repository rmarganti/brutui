mod support;

use std::fs;

use serde_json::json;

use support::{
    FakeBruSpec, copy_fixture_collection, install_fake_bru, lock_process_state, report_path,
    run_command, set_current_dir, set_env_var, temp_workspace,
};

#[test]
fn fixture_collections_copy_into_isolated_temp_workspaces() {
    let classic = copy_fixture_collection("classic", "sample-classic");
    let open = copy_fixture_collection("opencollection", "sample-open");

    assert!(classic.root.join("bruno.json").is_file());
    assert!(classic.root.join("users/list.bru").is_file());
    assert!(classic.root.join("environments/dev.bru").is_file());
    assert!(classic.root.join("broken/malformed.bru").is_file());
    assert!(
        classic
            .root
            .join("node_modules/ignored/ignored.bru")
            .is_file()
    );

    assert!(open.root.join("opencollection.yml").is_file());
    assert!(
        open.root
            .join("requests/catalog/get-products.yml")
            .is_file()
    );
    assert!(open.root.join("environments/staging.yml").is_file());
}

#[test]
fn cwd_guard_restores_the_previous_working_directory() {
    let _lock = lock_process_state();
    let original = std::env::current_dir().expect("original cwd");
    let workspace = temp_workspace();

    {
        let _guard = set_current_dir(workspace.path());
        assert_eq!(
            std::env::current_dir()
                .expect("temp cwd")
                .canonicalize()
                .expect("canonical temp cwd"),
            workspace
                .path()
                .canonicalize()
                .expect("canonical workspace")
        );
    }

    assert_eq!(std::env::current_dir().expect("restored cwd"), original);
}

#[test]
fn env_var_guard_restores_previous_values() {
    let _lock = lock_process_state();
    let key = "BRUTUI_TEST_SCOPED_ENV";
    let original = std::env::var_os(key);

    {
        let _guard = set_env_var(key, "fake-bru");
        assert_eq!(std::env::var(key).expect("set env"), "fake-bru");
    }

    assert_eq!(std::env::var_os(key), original);
}

#[test]
fn fake_bru_emits_output_and_writes_a_configurable_json_report() {
    let workspace = temp_workspace();
    let fake_bru = install_fake_bru(
        &workspace,
        &FakeBruSpec {
            stdout: vec!["running request".into()],
            stderr: vec!["warning: flaky test".into()],
            exit_code: 1,
            report_json: Some(json!({
                "summary": {"total_requests": 1, "failed_requests": 1},
                "requests": [{"name": "get users", "status": "failed"}]
            })),
        },
    );
    let report = report_path(&workspace, "reports/result.json");

    let output = run_command(
        &fake_bru,
        &[
            "run",
            "collection",
            "--report-file",
            report.to_str().expect("report path"),
        ],
    );

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "running request\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "warning: flaky test\n"
    );

    let written_report = fs::read_to_string(&report).expect("report written");
    let parsed: serde_json::Value =
        serde_json::from_str(&written_report).expect("valid report json");
    assert_eq!(parsed["requests"][0]["status"], "failed");
}

#[test]
fn unchanged_assertion_catches_collection_mutations() {
    let collection = copy_fixture_collection("classic", "sample-classic");
    let report_workspace = temp_workspace();
    let fake_bru = install_fake_bru(
        &report_workspace,
        &FakeBruSpec {
            report_json: Some(json!({"summary": {"total_requests": 0}})),
            ..FakeBruSpec::default()
        },
    );
    let report = report_path(&report_workspace, "result.json");

    let output = run_command(
        &fake_bru,
        &[
            "run",
            collection.root.to_str().expect("collection path"),
            "--report-file",
            report.to_str().expect("report path"),
        ],
    );

    assert!(output.status.success());
    collection.assert_unchanged();
}
