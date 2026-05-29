use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::collection::model::{Collection, CollectionNodeId};
use crate::environments::EnvironmentOption;
use crate::report::{ReportParseError, RunReport, parse_report_file};

static REPORT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub report_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvent {
    Stdout(String),
    Stderr(String),
    Finished(RunCompletion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCompletion {
    pub exit_code: Option<i32>,
    pub report_path: PathBuf,
    pub outcome: RunOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    Success(RunReport),
    CompletedWithFailures(RunReport),
    Cancelled,
    ToolError(RunToolError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunToolError {
    pub kind: RunToolErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunToolErrorKind {
    NonZeroExit,
    MissingReport,
    InvalidReport,
    Terminated,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunCommandBuildError {
    #[error("collection node not found for selection {0:?}")]
    UnknownSelection(CollectionNodeId),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunStartError {
    #[error("another Bruno run is already active")]
    ActiveRunInProgress,
    #[error("failed to launch `{program}`: {message}")]
    SpawnFailed { program: PathBuf, message: String },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunCancelError {
    #[error("no active Bruno run to cancel")]
    NoActiveRun,
    #[error("failed to signal Bruno cancellation")]
    SignalFailed,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessRunner {
    state: Arc<Mutex<RunnerState>>,
}

#[derive(Debug, Default)]
struct RunnerState {
    active: Option<ActiveRunHandle>,
    latest_report_path: Option<PathBuf>,
}

#[derive(Debug)]
struct ActiveRunHandle {
    cancel_tx: mpsc::Sender<()>,
}

pub type RunEventReceiver = mpsc::Receiver<RunEvent>;

impl ProcessRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self, command: RunCommand) -> Result<RunEventReceiver, RunStartError> {
        let mut child = Command::new(&command.program)
            .args(&command.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| RunStartError::SpawnFailed {
                program: command.program.clone(),
                message: error.to_string(),
            })?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (event_tx, event_rx) = mpsc::channel();
        let (cancel_tx, cancel_rx) = mpsc::channel();

        {
            let mut state = self.state.lock().expect("runner state lock");
            if state.active.is_some() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunStartError::ActiveRunInProgress);
            }

            state.latest_report_path = Some(command.report_path.clone());
            state.active = Some(ActiveRunHandle { cancel_tx });
        }

        let state = Arc::clone(&self.state);
        thread::spawn(move || {
            let stdout_thread = stdout.map(|stream| spawn_reader(stream, event_tx.clone(), true));
            let stderr_thread = stderr.map(|stream| spawn_reader(stream, event_tx.clone(), false));

            let mut cancelled = false;
            let exit_code = loop {
                if cancel_rx.try_recv().is_ok() {
                    cancelled = true;
                    let _ = child.kill();
                }

                match child.try_wait() {
                    Ok(Some(status)) => break status.code(),
                    Ok(None) => thread::sleep(Duration::from_millis(10)),
                    Err(error) => {
                        let completion = RunCompletion {
                            exit_code: None,
                            report_path: command.report_path.clone(),
                            outcome: RunOutcome::ToolError(RunToolError {
                                kind: RunToolErrorKind::Terminated,
                                message: format!("failed while waiting for Bruno process: {error}"),
                            }),
                        };
                        let _ = event_tx.send(RunEvent::Finished(completion));
                        clear_active_run(&state);
                        return;
                    }
                }
            };

            if let Some(thread) = stdout_thread {
                let _ = thread.join();
            }
            if let Some(thread) = stderr_thread {
                let _ = thread.join();
            }

            let completion = classify_completion(exit_code, &command.report_path, cancelled);
            let _ = event_tx.send(RunEvent::Finished(completion));
            clear_active_run(&state);
        });

        Ok(event_rx)
    }

    pub fn cancel(&self) -> Result<(), RunCancelError> {
        let cancel_tx = {
            let state = self.state.lock().expect("runner state lock");
            state
                .active
                .as_ref()
                .map(|active| active.cancel_tx.clone())
                .ok_or(RunCancelError::NoActiveRun)?
        };

        cancel_tx.send(()).map_err(|_| RunCancelError::SignalFailed)
    }

    pub fn has_active_run(&self) -> bool {
        self.state
            .lock()
            .expect("runner state lock")
            .active
            .is_some()
    }

    pub fn latest_report_path(&self) -> Option<PathBuf> {
        self.state
            .lock()
            .expect("runner state lock")
            .latest_report_path
            .clone()
    }
}

pub fn build_run_command(
    program: impl Into<PathBuf>,
    collection: &Collection,
    target: &CollectionNodeId,
    environment: &EnvironmentOption,
) -> Result<RunCommand, RunCommandBuildError> {
    let target_path = resolve_target_path(collection, target)?;
    let report_path = next_report_path();
    let mut args = vec!["run".to_string(), path_arg(&target_path)];

    if matches!(target, CollectionNodeId::Root | CollectionNodeId::Folder(_)) {
        args.push("-r".to_string());
    }

    if let Some(env) = &environment.cli_value {
        args.push("--env".to_string());
        args.push(env.clone());
    }

    args.push("--reporter-json".to_string());
    args.push(path_arg(&report_path));

    Ok(RunCommand {
        program: program.into(),
        args,
        report_path,
    })
}

fn classify_completion(
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

fn spawn_reader(
    stream: impl std::io::Read + Send + 'static,
    event_tx: mpsc::Sender<RunEvent>,
    stdout: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let event = if stdout {
                        RunEvent::Stdout(line)
                    } else {
                        RunEvent::Stderr(line)
                    };
                    if event_tx.send(event).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = event_tx.send(RunEvent::Stderr(format!(
                        "failed to read Bruno process output: {error}"
                    )));
                    break;
                }
            }
        }
    })
}

fn clear_active_run(state: &Arc<Mutex<RunnerState>>) {
    state.lock().expect("runner state lock").active = None;
}

fn resolve_target_path(
    collection: &Collection,
    target: &CollectionNodeId,
) -> Result<PathBuf, RunCommandBuildError> {
    match target {
        CollectionNodeId::Root => Ok(collection.root.clone()),
        CollectionNodeId::Folder(_) | CollectionNodeId::Request(_) => collection
            .nodes
            .iter()
            .find(|node| node.id() == *target)
            .map(|node| node.path().to_path_buf())
            .ok_or_else(|| RunCommandBuildError::UnknownSelection(target.clone())),
    }
}

fn next_report_path() -> PathBuf {
    let unique = REPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!(
        "brutui-report-{}-{}-{}.json",
        std::process::id(),
        timestamp,
        unique
    ))
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::collection::model::{Collection, CollectionFormat, CollectionNode, RootNode};

    use super::{
        RunCommandBuildError, RunOutcome, RunToolErrorKind, build_run_command, classify_completion,
    };

    #[test]
    fn missing_non_root_selection_is_reported() {
        let collection = Collection {
            root: PathBuf::from("/tmp/demo"),
            format: CollectionFormat::ClassicJson,
            nodes: vec![CollectionNode::Root(RootNode {
                path: PathBuf::from("/tmp/demo"),
                display_name: "demo".to_string(),
            })],
        };

        let error = build_run_command(
            "/usr/bin/bru",
            &collection,
            &crate::collection::model::CollectionNodeId::Request(PathBuf::from("missing.bru")),
            &crate::environments::EnvironmentOption::no_environment(),
        )
        .expect_err("missing selection should error");

        assert_eq!(
            error,
            RunCommandBuildError::UnknownSelection(
                crate::collection::model::CollectionNodeId::Request(PathBuf::from("missing.bru"))
            )
        );
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
}
