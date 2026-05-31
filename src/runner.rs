use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

use thiserror::Error;

use crate::collection::model::CollectionNodeId;
use crate::report::RunReport;

mod command;
mod completion;
mod process;
mod report_path;

pub use command::build_run_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
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
pub(crate) struct RunnerState {
    active: Option<ActiveRunHandle>,
    latest_report_path: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct ActiveRunHandle {
    cancel_tx: mpsc::Sender<()>,
}

pub type RunEventReceiver = mpsc::Receiver<RunEvent>;

impl ProcessRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self, command: RunCommand) -> Result<RunEventReceiver, RunStartError> {
        process::start(command, Arc::clone(&self.state))
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
