mod support;

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use brutui::app::AppController;
use brutui::collection::model::CollectionFormat;
use brutui::collection::scanner::scan_collection;
use brutui::environments::discover as discover_environments;
use brutui::state::{CompletedRunStatus, ModalState, OutputStream};
use brutui::ui::render;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn run_key_streams_output_and_summary_and_raw_views_render() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let environments = discover_environments(&fixture.root, CollectionFormat::ClassicJson)
        .expect("discover environments");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            stdout: vec!["booting".to_string()],
            stderr: vec!["warning".to_string()],
            exit_code: 0,
            report_json: Some(success_report()),
        },
    );
    let mut controller =
        AppController::new_loaded(collection, environments, bru).expect("create controller");

    controller
        .handle_key_event(press_char('r'))
        .expect("start run with r");
    wait_for_completion(&mut controller);

    let session = controller.state.session.as_ref().expect("session");
    assert!(matches!(
        session.completed_run.as_ref().map(|run| &run.status),
        Some(CompletedRunStatus::Success(_))
    ));
    assert_eq!(session.raw_output.len(), 2);
    assert!(
        session
            .raw_output
            .iter()
            .any(|line| line.stream == OutputStream::Stdout && line.text == "booting")
    );
    assert!(
        session
            .raw_output
            .iter()
            .any(|line| line.stream == OutputStream::Stderr && line.text == "warning")
    );

    let summary = render_to_string(&controller.state, 120, 32);
    assert!(summary.contains("Status: success"));
    assert!(summary.contains("Requests: 1 total, 1 passed, 0 failed"));

    controller
        .handle_key_event(press_char('3'))
        .expect("switch to raw view");
    let raw = render_to_string(&controller.state, 120, 32);
    assert!(raw.contains("[stdout] booting"));
    assert!(raw.contains("[stderr] warning"));

    fixture.assert_unchanged();
}

#[test]
fn failure_view_renders_failed_requests_tests_and_assertions() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let environments = discover_environments(&fixture.root, CollectionFormat::ClassicJson)
        .expect("discover environments");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            exit_code: 1,
            report_json: Some(failure_report()),
            ..Default::default()
        },
    );
    let mut controller =
        AppController::new_loaded(collection, environments, bru).expect("create controller");

    controller
        .handle_key_event(press_char('r'))
        .expect("start run with r");
    wait_for_completion(&mut controller);
    controller
        .handle_key_event(press_char('2'))
        .expect("switch to failures view");

    let rendered = render_to_string(&controller.state, 120, 32);
    assert!(rendered.contains("completed with failures"));
    assert!(rendered.contains("Request: list users"));
    assert!(rendered.contains("Failed test: status code — expected 200"));
    assert!(rendered.contains("expected: 200 • actual: 500"));

    fixture.assert_unchanged();
}

#[test]
fn missing_report_is_shown_as_a_tool_error() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let environments = discover_environments(&fixture.root, CollectionFormat::ClassicJson)
        .expect("discover environments");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(
        &workspace,
        &support::FakeBruSpec {
            exit_code: 0,
            report_json: None,
            ..Default::default()
        },
    );
    let mut controller =
        AppController::new_loaded(collection, environments, bru).expect("create controller");

    controller
        .handle_key_event(press_char('r'))
        .expect("start run with r");
    wait_for_completion(&mut controller);

    let rendered = render_to_string(&controller.state, 120, 32);
    assert!(rendered.contains("Status: tool error"));
    assert!(rendered.contains("did not produce a JSON report"));

    fixture.assert_unchanged();
}

#[test]
fn environment_picker_can_be_opened_selected_and_cancelled() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let environments = discover_environments(&fixture.root, CollectionFormat::ClassicJson)
        .expect("discover environments");
    let workspace = support::temp_workspace();
    let bru = support::install_fake_bru(&workspace, &support::FakeBruSpec::default());
    let mut controller =
        AppController::new_loaded(collection, environments, bru).expect("create controller");

    controller
        .handle_key_event(press_char('e'))
        .expect("open env picker");
    assert!(matches!(
        controller.state.session.as_ref().expect("session").modal,
        ModalState::EnvironmentPicker {
            highlighted_index: 0
        }
    ));

    controller
        .handle_key_event(press_key(KeyCode::Down))
        .expect("move env highlight");
    controller
        .handle_key_event(press_key(KeyCode::Enter))
        .expect("confirm env");
    assert_eq!(
        controller
            .state
            .selected_environment()
            .expect("selected environment")
            .display_name,
        "dev"
    );
    assert!(matches!(
        controller.state.session.as_ref().expect("session").modal,
        ModalState::None
    ));

    let rendered = render_to_string(&controller.state, 120, 32);
    assert!(rendered.contains("Environment: dev"));
    assert!(rendered.contains("Env: dev"));

    controller
        .handle_key_event(press_char('e'))
        .expect("reopen env picker");
    controller
        .handle_key_event(press_key(KeyCode::Esc))
        .expect("cancel env picker");
    assert_eq!(
        controller
            .state
            .selected_environment()
            .expect("selected environment")
            .display_name,
        "dev"
    );

    fixture.assert_unchanged();
}

#[test]
fn active_run_can_be_cancelled_and_rejects_overlapping_run_requests() {
    let fixture = support::copy_fixture_collection("classic", "sample-classic");
    let collection = scan_collection(&fixture.root, CollectionFormat::ClassicJson)
        .expect("scan classic fixture");
    let environments = discover_environments(&fixture.root, CollectionFormat::ClassicJson)
        .expect("discover environments");
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
    let mut controller =
        AppController::new_loaded(collection, environments, bru).expect("create controller");

    controller
        .handle_key_event(press_char('r'))
        .expect("start run with r");
    controller
        .handle_key_event(press_char('r'))
        .expect("reject overlapping run");
    wait_for_output_lines(&mut controller, 1);

    assert!(
        controller
            .state
            .session
            .as_ref()
            .expect("session")
            .raw_output
            .iter()
            .any(|line| line.text.contains("Run already active"))
    );

    controller
        .handle_key_event(press_char('c'))
        .expect("cancel active run");
    wait_for_completion(&mut controller);

    assert!(matches!(
        controller
            .state
            .session
            .as_ref()
            .and_then(|session| session.completed_run.as_ref())
            .map(|run| &run.status),
        Some(CompletedRunStatus::Cancelled)
    ));

    let rendered = render_to_string(&controller.state, 120, 32);
    assert!(rendered.contains("Status: cancelled"));

    fixture.assert_unchanged();
}

fn wait_for_completion(controller: &mut AppController) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        controller.pump_run_events().expect("pump run events");
        let done = controller
            .state
            .session
            .as_ref()
            .and_then(|session| session.completed_run.as_ref())
            .is_some()
            && !controller.has_active_run();
        if done {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for run completion"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_output_lines(controller: &mut AppController, minimum_lines: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        controller.pump_run_events().expect("pump run events");
        let count = controller
            .state
            .session
            .as_ref()
            .expect("session")
            .raw_output
            .len();
        if count >= minimum_lines {
            return;
        }
        assert!(Instant::now() < deadline, "timed out waiting for output");
        thread::sleep(Duration::from_millis(10));
    }
}

fn render_to_string(state: &brutui::state::AppState, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    let mut state = state.clone();
    terminal
        .draw(|frame| render(frame, &mut state))
        .expect("render UI");

    let buffer = terminal.backend().buffer();
    let area = buffer.area();
    let mut rendered = String::new();
    for y in 0..area.height {
        for x in 0..area.width {
            rendered.push_str(buffer.cell((x, y)).expect("cell").symbol());
        }
        rendered.push('\n');
    }
    rendered
}

fn press_char(character: char) -> KeyEvent {
    press_key(KeyCode::Char(character))
}

fn press_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
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
