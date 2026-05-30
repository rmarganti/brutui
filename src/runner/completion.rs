use std::path::Path;

use crate::report::{ReportParseError, parse_report_file};

use super::{RunCompletion, RunOutcome, RunToolError, RunToolErrorKind};

pub(crate) fn classify_completion(
    exit_code: Option<i32>,
    report_path: &Path,
    cancelled: bool,
) -> RunCompletion {
    let outcome = if cancelled {
        RunOutcome::Cancelled
    } else {
        match exit_code {
            Some(0) => classify_report_outcome(report_path, true),
            Some(1) => classify_report_outcome(report_path, false),
            Some(code) => RunOutcome::ToolError(RunToolError {
                kind: RunToolErrorKind::NonZeroExit,
                message: format!("Bruno exited with status {code}"),
            }),
            None => RunOutcome::ToolError(RunToolError {
                kind: RunToolErrorKind::Terminated,
                message: "Bruno terminated without an exit code".to_string(),
            }),
        }
    };

    RunCompletion {
        exit_code,
        report_path: report_path.to_path_buf(),
        outcome,
    }
}

fn classify_report_outcome(report_path: &Path, success_exit: bool) -> RunOutcome {
    match parse_report_file(report_path) {
        Ok(report) if success_exit => RunOutcome::Success(report),
        Ok(report) => RunOutcome::CompletedWithFailures(report),
        Err(error) => RunOutcome::ToolError(report_error(report_path, error)),
    }
}

fn report_error(report_path: &Path, error: ReportParseError) -> RunToolError {
    match error {
        ReportParseError::Io { path, source } if source.kind() == std::io::ErrorKind::NotFound => {
            RunToolError {
                kind: RunToolErrorKind::MissingReport,
                message: format!("Bruno did not produce a JSON report at {}", path.display()),
            }
        }
        other => RunToolError {
            kind: RunToolErrorKind::InvalidReport,
            message: format!(
                "failed to interpret Bruno JSON report at {}: {other}",
                report_path.display()
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;
    use tempfile::TempDir;

    use super::classify_completion;
    use crate::runner::{RunOutcome, RunToolErrorKind};

    #[test]
    fn valid_zero_exit_report_is_success() {
        let workspace = TempDir::new().expect("temp dir");
        let report_path = workspace.path().join("report.json");
        fs::write(&report_path, success_report().to_string()).expect("write report");

        let completion = classify_completion(Some(0), &report_path, false);

        match completion.outcome {
            RunOutcome::Success(report) => {
                assert_eq!(report.summary.total_requests, 1);
                assert_eq!(report.summary.failed_requests, 0);
            }
            other => panic!("expected success outcome, got {other:?}"),
        }
    }

    #[test]
    fn valid_exit_code_one_report_is_completed_with_failures() {
        let workspace = TempDir::new().expect("temp dir");
        let report_path = workspace.path().join("report.json");
        fs::write(&report_path, failure_report().to_string()).expect("write report");

        let completion = classify_completion(Some(1), &report_path, false);

        match completion.outcome {
            RunOutcome::CompletedWithFailures(report) => {
                assert_eq!(report.summary.failed_requests, 1);
                assert_eq!(report.requests.len(), 1);
            }
            other => panic!("expected completed-with-failures outcome, got {other:?}"),
        }
    }

    #[test]
    fn missing_report_is_classified_without_spawning_processes() {
        let completion = classify_completion(
            Some(0),
            PathBuf::from("/tmp/definitely-missing-brutui-report.json").as_path(),
            false,
        );

        match completion.outcome {
            RunOutcome::ToolError(error) => {
                assert_eq!(error.kind, RunToolErrorKind::MissingReport);
                assert!(error.message.contains("did not produce a JSON report"));
            }
            other => panic!("expected tool error, got {other:?}"),
        }
    }

    #[test]
    fn invalid_report_is_classified_without_spawning_processes() {
        let workspace = TempDir::new().expect("temp dir");
        let report_path = workspace.path().join("report.json");
        fs::write(&report_path, "{not json}\n").expect("write invalid report");

        let completion = classify_completion(Some(0), &report_path, false);

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
    }

    #[test]
    fn non_zero_exit_without_cancellation_is_a_tool_error() {
        let completion =
            classify_completion(Some(7), PathBuf::from("/tmp/report.json").as_path(), false);

        match completion.outcome {
            RunOutcome::ToolError(error) => {
                assert_eq!(error.kind, RunToolErrorKind::NonZeroExit);
                assert!(error.message.contains("status 7"));
            }
            other => panic!("expected tool error, got {other:?}"),
        }
    }

    #[test]
    fn cancellation_takes_precedence_over_exit_interpretation() {
        let completion =
            classify_completion(Some(0), PathBuf::from("/tmp/report.json").as_path(), true);

        assert_eq!(completion.outcome, RunOutcome::Cancelled);
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
                    "status": "failed"
                }
            ]
        })
    }
}
