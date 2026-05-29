mod support;

use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use brutui::collection::model::{CollectionFormat, CollectionNodeId};
use brutui::collection::scanner::scan_collection;
use brutui::environments::EnvironmentOption;
use brutui::runner::{
    ProcessRunner, RunEvent, RunOutcome, RunStartError, RunToolErrorKind, build_run_command,
};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn runner_streams_stdout_and_stderr_before_reporting_success() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = write_executable_script(
        &workspace,
        "bru",
        &format!(
            r#"#!/bin/sh
set -eu

report_path=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "report" ]; then
    report_path="$arg"
    prev=""
    continue
  fi

  case "$arg" in
    --report-file|--report-path|--output|--reporter-json)
      prev="report"
      ;;
  esac
done

printf 'booting\n'
sleep 0.2
printf 'warning\n' 1>&2
sleep 0.2
cat <<'__BRUTUI_REPORT__' > "$report_path"
{}
__BRUTUI_REPORT__
exit 0
"#,
            serde_json::to_string_pretty(&success_report()).expect("serialize report")
        ),
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command.clone()).expect("start runner");
    assert!(runner.has_active_run());
    assert_eq!(
        runner.latest_report_path().as_deref(),
        Some(command.report_path.as_path())
    );

    assert_eq!(recv_event(&events), RunEvent::Stdout("booting".to_string()));
    assert_eq!(recv_event(&events), RunEvent::Stderr("warning".to_string()));

    let completion = recv_finished(&events);
    assert_eq!(completion.report_path, command.report_path);
    assert_eq!(completion.exit_code, Some(0));
    match completion.outcome {
        RunOutcome::Success(report) => {
            assert_eq!(report.summary.total_requests, 1);
            assert_eq!(report.summary.failed_requests, 0);
        }
        other => panic!("expected success outcome, got {other:?}"),
    }

    assert!(!runner.has_active_run());
    assert_eq!(
        runner.latest_report_path().as_deref(),
        Some(command.report_path.as_path())
    );
    fixture.assert_unchanged();
}

#[test]
fn exit_code_one_with_valid_report_is_completed_with_failures() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            exit_code: 1,
            report_json: Some(failure_report()),
            ..Default::default()
        },
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Request(PathBuf::from("users/list.bru")),
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command.clone()).expect("start runner");
    let completion = recv_finished(&events);

    assert_eq!(completion.exit_code, Some(1));
    match completion.outcome {
        RunOutcome::CompletedWithFailures(report) => {
            assert_eq!(report.summary.failed_requests, 1);
            assert_eq!(report.requests.len(), 1);
        }
        other => panic!("expected completed-with-failures outcome, got {other:?}"),
    }
    assert_eq!(
        runner.latest_report_path().as_deref(),
        Some(command.report_path.as_path())
    );
    fixture.assert_unchanged();
}

#[test]
fn other_non_zero_exit_is_reported_as_tool_error() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            exit_code: 7,
            report_json: Some(success_report()),
            ..Default::default()
        },
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command).expect("start runner");
    let completion = recv_finished(&events);

    match completion.outcome {
        RunOutcome::ToolError(error) => {
            assert_eq!(error.kind, RunToolErrorKind::NonZeroExit);
            assert!(error.message.contains("status 7"));
        }
        other => panic!("expected tool error, got {other:?}"),
    }
    fixture.assert_unchanged();
}

#[test]
fn missing_report_is_reported_as_tool_error() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            exit_code: 0,
            report_json: None,
            ..Default::default()
        },
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command).expect("start runner");
    let completion = recv_finished(&events);

    match completion.outcome {
        RunOutcome::ToolError(error) => {
            assert_eq!(error.kind, RunToolErrorKind::MissingReport);
            assert!(error.message.contains("did not produce a JSON report"));
        }
        other => panic!("expected tool error, got {other:?}"),
    }
    fixture.assert_unchanged();
}

#[test]
fn invalid_report_is_reported_as_tool_error() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = write_executable_script(
        &workspace,
        "bru",
        r#"#!/bin/sh
set -eu

report_path=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "report" ]; then
    report_path="$arg"
    prev=""
    continue
  fi

  case "$arg" in
    --report-file|--report-path|--output|--reporter-json)
      prev="report"
      ;;
  esac
done

printf '{not json}\n' > "$report_path"
exit 0
"#,
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command).expect("start runner");
    let completion = recv_finished(&events);

    match completion.outcome {
        RunOutcome::ToolError(error) => {
            assert_eq!(error.kind, RunToolErrorKind::InvalidReport);
            assert!(
                error
                    .message
                    .contains("failed to interpret Bruno JSON report")
            );
        }
        other => panic!("expected tool error, got {other:?}"),
    }
    fixture.assert_unchanged();
}

#[test]
fn runner_cancels_active_process() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = write_executable_script(
        &workspace,
        "bru",
        r#"#!/bin/sh
set -eu
printf 'started\n'
while true; do
  sleep 1
done
"#,
    );

    let command = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build run command");
    let runner = ProcessRunner::new();

    let events = runner.start(command).expect("start runner");
    assert_eq!(recv_event(&events), RunEvent::Stdout("started".to_string()));

    runner.cancel().expect("cancel active run");
    let completion = recv_finished(&events);

    assert_eq!(completion.outcome, RunOutcome::Cancelled);
    assert!(!runner.has_active_run());
    fixture.assert_unchanged();
}

#[test]
fn overlapping_runs_are_rejected() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let workspace = support::temp_workspace();
    let bru = write_executable_script(
        &workspace,
        "bru",
        r#"#!/bin/sh
set -eu
while true; do
  sleep 1
done
"#,
    );

    let first = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Root,
        &EnvironmentOption::no_environment(),
    )
    .expect("build first command");
    let second = build_run_command(
        &bru,
        &collection,
        &CollectionNodeId::Request(PathBuf::from("users/list.bru")),
        &EnvironmentOption::no_environment(),
    )
    .expect("build second command");
    let runner = ProcessRunner::new();

    let events = runner.start(first).expect("start first runner");
    let error = runner
        .start(second)
        .expect_err("second run should be rejected");
    assert_eq!(error, RunStartError::ActiveRunInProgress);

    runner.cancel().expect("cancel first run");
    let completion = recv_finished(&events);
    assert_eq!(completion.outcome, RunOutcome::Cancelled);
    fixture.assert_unchanged();
}

fn recv_event(events: &Receiver<RunEvent>) -> RunEvent {
    events
        .recv_timeout(Duration::from_secs(3))
        .expect("event before timeout")
}

fn recv_finished(events: &Receiver<RunEvent>) -> brutui::runner::RunCompletion {
    loop {
        match recv_event(events) {
            RunEvent::Finished(completion) => return completion,
            RunEvent::Stdout(_) | RunEvent::Stderr(_) => {}
        }
    }
}

fn write_executable_script(workspace: &TempDir, name: &str, script: &str) -> PathBuf {
    let path = workspace.path().join(name);
    fs::write(&path, script).expect("write script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&path).expect("script metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("script permissions");
    }

    path
}

fn success_report() -> serde_json::Value {
    json!({
        "summary": {
            "total_requests": 1,
            "passed_requests": 1,
            "failed_requests": 0,
            "total_tests": 1,
            "passed_tests": 1,
            "failed_tests": 0,
            "total_assertions": 1,
            "passed_assertions": 1,
            "failed_assertions": 0,
            "error_count": 0
        },
        "requests": [
            {
                "name": "list users",
                "status": "passed"
            }
        ]
    })
}

fn failure_report() -> serde_json::Value {
    json!({
        "summary": {
            "total_requests": 1,
            "passed_requests": 0,
            "failed_requests": 1,
            "total_tests": 1,
            "passed_tests": 0,
            "failed_tests": 1,
            "total_assertions": 1,
            "passed_assertions": 0,
            "failed_assertions": 1,
            "error_count": 0
        },
        "requests": [
            {
                "name": "list users",
                "status": "failed",
                "failedTests": [
                    {
                        "name": "status code",
                        "status": "failed",
                        "message": "expected 200"
                    }
                ],
                "failedAssertions": [
                    {
                        "name": "status code",
                        "message": "expected 200",
                        "expected": "200",
                        "actual": "500"
                    }
                ]
            }
        ]
    })
}
