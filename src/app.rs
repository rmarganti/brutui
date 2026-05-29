use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;

use anyhow::{Result, bail};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use thiserror::Error;

use crate::cli::Cli;
use crate::collection::model::{Collection, CollectionNodeId};
use crate::environments::EnvironmentOption;
use crate::runner::{
    ProcessRunner, RunCancelError, RunCommandBuildError, RunEvent, RunEventReceiver, RunStartError,
    build_run_command,
};
use crate::state::{AppState, ModalState, RunState, StateError};
use crate::ui::{UiEventResult, handle_key_event};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupMode {
    CollectionPathProvided,
    AutomaticDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppBootstrap {
    pub mode: StartupMode,
}

#[derive(Debug)]
pub struct AppController {
    pub state: AppState,
    bru_path: PathBuf,
    runner: ProcessRunner,
    active_events: Option<RunEventReceiver>,
}

#[derive(Debug, Error)]
pub enum AppControllerError {
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    RunCommandBuild(#[from] RunCommandBuildError),
    #[error(transparent)]
    RunStart(#[from] RunStartError),
    #[error(transparent)]
    RunCancel(#[from] RunCancelError),
}

impl AppBootstrap {
    pub fn from_cli(cli: &Cli) -> Self {
        let mode = if cli.collection_path.is_some() {
            StartupMode::CollectionPathProvided
        } else {
            StartupMode::AutomaticDiscovery
        };

        Self { mode }
    }

    pub fn run(self) -> Result<()> {
        let mode = match self.mode {
            StartupMode::CollectionPathProvided => "explicit collection path",
            StartupMode::AutomaticDiscovery => "automatic collection discovery",
        };

        bail!(
            "Brutui scaffold complete, but startup flow for {mode} is not implemented yet. Follow-on ishes will add discovery, TUI rendering, and Bruno execution."
        )
    }
}

impl AppController {
    pub fn new_loaded(
        collection: Collection,
        environments: Vec<EnvironmentOption>,
        bru_path: impl Into<PathBuf>,
    ) -> Result<Self, StateError> {
        let mut state = AppState::new();
        state.open_collection(collection, environments)?;

        Ok(Self {
            state,
            bru_path: bru_path.into(),
            runner: ProcessRunner::new(),
            active_events: None,
        })
    }

    pub fn handle_key_event(
        &mut self,
        event: KeyEvent,
    ) -> Result<UiEventResult, AppControllerError> {
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return Ok(UiEventResult::Continue);
        }

        if self.session_modal_is_clear() {
            match event.code {
                KeyCode::Char('r') => {
                    self.start_selected_run()?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('c') => {
                    self.cancel_active_run()?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('1') => {
                    self.state
                        .set_result_view(crate::state::ResultView::Summary)?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('2') => {
                    self.state
                        .set_result_view(crate::state::ResultView::Failures)?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('3') => {
                    self.state
                        .set_result_view(crate::state::ResultView::RawOutput)?;
                    return Ok(UiEventResult::Continue);
                }
                _ => {}
            }
        }

        Ok(handle_key_event(&mut self.state, event)?)
    }

    pub fn pump_run_events(&mut self) -> Result<bool, AppControllerError> {
        let mut finished = false;

        while let Some(event) = self.try_recv_event()? {
            match event {
                RunEvent::Stdout(line) => self.state.append_stdout(line)?,
                RunEvent::Stderr(line) => self.state.append_stderr(line)?,
                RunEvent::Finished(completion) => {
                    self.state.finish_run(completion)?;
                    self.active_events = None;
                    finished = true;
                }
            }
        }

        Ok(finished)
    }

    pub fn has_active_run(&self) -> bool {
        self.active_events.is_some() || self.runner.has_active_run()
    }

    fn start_selected_run(&mut self) -> Result<(), AppControllerError> {
        if matches!(
            self.state.session.as_ref().map(|session| &session.run),
            Some(RunState::Running(_))
        ) {
            self.state
                .append_stderr("Run already active; cancel it before starting another run.")?;
            self.state
                .set_result_view(crate::state::ResultView::RawOutput)?;
            return Ok(());
        }

        let (collection, target, environment) = self.selected_run_context()?;
        let command = build_run_command(&self.bru_path, collection, &target, environment)?;
        let events = self.runner.start(command)?;

        self.state.start_run_on_selected_node()?;
        self.active_events = Some(events);
        Ok(())
    }

    fn cancel_active_run(&mut self) -> Result<(), AppControllerError> {
        if !matches!(
            self.state.session.as_ref().map(|session| &session.run),
            Some(RunState::Running(_))
        ) {
            self.state.append_stderr("No active run to cancel.")?;
            self.state
                .set_result_view(crate::state::ResultView::RawOutput)?;
            return Ok(());
        }

        self.state.request_run_cancellation()?;
        self.runner.cancel()?;
        Ok(())
    }

    fn session_modal_is_clear(&self) -> bool {
        matches!(
            self.state.session.as_ref().map(|session| &session.modal),
            Some(ModalState::None)
        )
    }

    fn selected_run_context(
        &self,
    ) -> Result<(&Collection, CollectionNodeId, &EnvironmentOption), StateError> {
        let session = self
            .state
            .session
            .as_ref()
            .ok_or(StateError::NoCollectionLoaded)?;
        let environment = session
            .environments
            .get(session.selected_environment_index)
            .ok_or(StateError::InvalidEnvironmentIndex {
                index: session.selected_environment_index,
            })?;

        Ok((
            &session.collection,
            session.selected_node.clone(),
            environment,
        ))
    }

    fn try_recv_event(&mut self) -> Result<Option<RunEvent>, AppControllerError> {
        let Some(receiver) = self.active_events.as_ref() else {
            return Ok(None);
        };

        match receiver.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                self.active_events = None;
                self.state
                    .append_stderr("Run event stream disconnected before completion.")?;
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::Cli;

    use super::{AppBootstrap, StartupMode};

    #[test]
    fn bootstrap_detects_automatic_discovery_mode() {
        let cli = Cli::parse_from(["brutui"]);

        let bootstrap = AppBootstrap::from_cli(&cli);

        assert_eq!(bootstrap.mode, StartupMode::AutomaticDiscovery);
    }

    #[test]
    fn bootstrap_detects_explicit_collection_mode() {
        let cli = Cli::parse_from(["brutui", "fixtures/demo"]);

        let bootstrap = AppBootstrap::from_cli(&cli);

        assert_eq!(bootstrap.mode, StartupMode::CollectionPathProvided);
    }
}
