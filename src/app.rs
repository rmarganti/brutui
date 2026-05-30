use std::path::{Path, PathBuf};
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use thiserror::Error;

use crate::cli::Cli;
use crate::collection::model::{Collection, CollectionNodeId};
use crate::collection::scanner::scan_collection;
use crate::config::{AppConfig, LoadedConfig, load};
use crate::discovery::{DiscoveredCollection, DiscoveryRequest, discover};
use crate::environments::{EnvironmentOption, discover as discover_environments};
use crate::executable::resolve as resolve_bru;
use crate::runner::{
    ProcessRunner, RunCancelError, RunCommand, RunCommandBuildError, RunEvent, RunEventReceiver,
    RunStartError, build_run_command,
};
use crate::state::{AppState, ModalState, RunState, SessionState, StartupState, StateError};
use crate::ui::{TerminalSession, UiEventResult, handle_key_event, render};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppBootstrap {
    pub collection_path: Option<PathBuf>,
}

#[derive(Debug)]
enum AppRuntime {
    Startup { state: AppState, config: AppConfig },
    Loaded(AppController),
}

#[derive(Debug)]
pub struct AppController {
    pub state: AppState,
    bru_path: PathBuf,
    runner: Box<dyn RunnerPort>,
    clipboard: Box<dyn ClipboardPort>,
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
    #[error("failed to copy response tab to clipboard: {0}")]
    Clipboard(String),
    #[error(transparent)]
    RunCancel(#[from] RunCancelError),
}

trait RunnerPort: std::fmt::Debug {
    fn start(&self, command: RunCommand) -> Result<RunEventReceiver, RunStartError>;
    fn cancel(&self) -> Result<(), RunCancelError>;
    fn has_active_run(&self) -> bool;
}

impl RunnerPort for ProcessRunner {
    fn start(&self, command: RunCommand) -> Result<RunEventReceiver, RunStartError> {
        ProcessRunner::start(self, command)
    }

    fn cancel(&self) -> Result<(), RunCancelError> {
        ProcessRunner::cancel(self)
    }

    fn has_active_run(&self) -> bool {
        ProcessRunner::has_active_run(self)
    }
}

trait ClipboardPort: std::fmt::Debug {
    fn set_text(&self, text: String) -> Result<(), String>;
}

#[derive(Debug, Default)]
struct SystemClipboard;

impl ClipboardPort for SystemClipboard {
    fn set_text(&self, text: String) -> Result<(), String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text))
            .map_err(|error| error.to_string())
    }
}

impl AppBootstrap {
    pub fn from_cli(cli: &Cli) -> Self {
        Self {
            collection_path: cli.collection_path.clone(),
        }
    }

    pub fn run(&self) -> Result<()> {
        let mut runtime = self.prepare_runtime()?;
        let mut terminal = TerminalSession::enter().context("failed to initialize terminal UI")?;

        loop {
            match &mut runtime {
                AppRuntime::Startup { state, .. } => {
                    terminal
                        .terminal_mut()
                        .draw(|frame| render(frame, state))
                        .context("failed to render startup UI")?;
                }
                AppRuntime::Loaded(controller) => {
                    controller.pump_run_events()?;
                    terminal
                        .terminal_mut()
                        .draw(|frame| render(frame, &mut controller.state))
                        .context("failed to render application UI")?;
                }
            }

            if !event::poll(Duration::from_millis(50)).context("failed to poll terminal events")? {
                continue;
            }

            let Event::Key(key_event) = event::read().context("failed to read terminal event")?
            else {
                continue;
            };

            match &mut runtime {
                AppRuntime::Startup { state, config } => {
                    match handle_key_event(state, key_event)? {
                        UiEventResult::Continue => {}
                        UiEventResult::Quit => break,
                        UiEventResult::StartupCollectionChosen => {
                            let Some(collection) = selected_startup_collection(state) else {
                                continue;
                            };
                            runtime = self.load_collection_runtime(collection, config.clone())?;
                        }
                    }
                }
                AppRuntime::Loaded(controller) => match controller.handle_key_event(key_event)? {
                    UiEventResult::Continue | UiEventResult::StartupCollectionChosen => {}
                    UiEventResult::Quit => break,
                },
            }
        }

        Ok(())
    }

    fn prepare_runtime(&self) -> Result<AppRuntime> {
        let loaded_config = load().context("failed to load Brutui configuration")?;
        let cwd =
            std::env::current_dir().context("failed to determine current working directory")?;
        self.prepare_runtime_with(loaded_config, cwd)
    }

    fn prepare_runtime_with(
        &self,
        loaded_config: LoadedConfig,
        cwd: PathBuf,
    ) -> Result<AppRuntime> {
        let discovered = discover(&DiscoveryRequest {
            explicit_path: self.collection_path.clone(),
            cwd,
            configured_dirs: loaded_config.config.collection_dirs.clone(),
        })
        .context("failed to discover Bruno collections")?;

        self.runtime_from_discovery(discovered, loaded_config)
    }

    fn runtime_from_discovery(
        &self,
        discovered: Vec<DiscoveredCollection>,
        loaded_config: LoadedConfig,
    ) -> Result<AppRuntime> {
        let config = loaded_config.config;
        match discovered.as_slice() {
            [] => Ok(AppRuntime::Startup {
                state: setup_message_state(no_collection_message(loaded_config.path.as_deref())),
                config,
            }),
            [collection] => self.load_collection_runtime(collection.clone(), config),
            _ => {
                let mut state = AppState::new();
                state.show_collection_picker(discovered);
                Ok(AppRuntime::Startup { state, config })
            }
        }
    }

    fn load_collection_runtime(
        &self,
        discovered: DiscoveredCollection,
        config: AppConfig,
    ) -> Result<AppRuntime> {
        let bru = match resolve_bru(&config) {
            Ok(bru) => bru,
            Err(error) => {
                return Ok(AppRuntime::Startup {
                    state: setup_message_state(missing_bru_message(&discovered.root, &error)),
                    config,
                });
            }
        };

        let collection = scan_collection(&discovered.root, discovered.format.clone())
            .with_context(|| {
                format!("failed to scan collection at {}", discovered.root.display())
            })?;
        let environments = discover_environments(&discovered.root, discovered.format)
            .with_context(|| {
                format!(
                    "failed to discover environments for {}",
                    discovered.root.display()
                )
            })?;

        Ok(AppRuntime::Loaded(AppController::new_loaded(
            collection,
            environments,
            bru.path,
        )?))
    }
}

impl AppController {
    pub fn new_loaded(
        collection: Collection,
        environments: Vec<EnvironmentOption>,
        bru_path: impl Into<PathBuf>,
    ) -> Result<Self, StateError> {
        Self::new_loaded_with_ports(
            collection,
            environments,
            bru_path,
            Box::new(ProcessRunner::new()),
            Box::new(SystemClipboard),
        )
    }

    fn new_loaded_with_ports(
        collection: Collection,
        environments: Vec<EnvironmentOption>,
        bru_path: impl Into<PathBuf>,
        runner: Box<dyn RunnerPort>,
        clipboard: Box<dyn ClipboardPort>,
    ) -> Result<Self, StateError> {
        let mut state = AppState::new();
        state.open_collection(collection, environments)?;

        Ok(Self {
            state,
            bru_path: bru_path.into(),
            runner,
            clipboard,
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
                KeyCode::Char('e') => {
                    self.state.open_environment_picker()?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('1') => {
                    self.state
                        .set_response_tab(crate::state::ResponseTab::Body)?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('2') => {
                    self.state
                        .set_response_tab(crate::state::ResponseTab::Headers)?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('3') => {
                    self.state
                        .set_response_tab(crate::state::ResponseTab::Tests)?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('[') => {
                    self.state.select_previous_result()?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char(']') => {
                    self.state.select_next_result()?;
                    return Ok(UiEventResult::Continue);
                }
                KeyCode::Char('y') => {
                    self.copy_current_response_tab()?;
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
            self.state.session().map(SessionState::run_state),
            Some(RunState::Running(_))
        ) {
            self.state
                .append_stderr("Run already active; cancel it before starting another run.")?;
            self.state
                .set_response_tab(crate::state::ResponseTab::Body)?;
            return Ok(());
        }

        let (collection, target, environment) = self.selected_run_context()?;
        let command = build_run_command(&self.bru_path, collection, &target, environment)?;
        let events = self.runner.start(command)?;

        self.state.start_run_on_selected_node()?;
        self.active_events = Some(events);
        Ok(())
    }

    fn copy_current_response_tab(&mut self) -> Result<(), AppControllerError> {
        let text = {
            let session = self.state.session().ok_or(StateError::NoCollectionLoaded)?;
            crate::ui::current_tab_text(session)
        };
        match self.clipboard.set_text(text) {
            Ok(()) => Ok(()),
            Err(error) => {
                self.state
                    .append_stderr(format!("Failed to copy response tab: {error}"))?;
                Ok(())
            }
        }
    }

    fn cancel_active_run(&mut self) -> Result<(), AppControllerError> {
        if !matches!(
            self.state.session().map(SessionState::run_state),
            Some(RunState::Running(_))
        ) {
            self.state.append_stderr("No active run to cancel.")?;
            self.state
                .set_response_tab(crate::state::ResponseTab::Body)?;
            return Ok(());
        }

        self.state.request_run_cancellation()?;
        self.runner.cancel()?;
        Ok(())
    }

    fn session_modal_is_clear(&self) -> bool {
        matches!(
            self.state.session().map(SessionState::modal),
            Some(ModalState::None)
        )
    }

    fn selected_run_context(
        &self,
    ) -> Result<(&Collection, CollectionNodeId, &EnvironmentOption), StateError> {
        let session = self.state.session().ok_or(StateError::NoCollectionLoaded)?;
        let environment =
            session
                .selected_environment()
                .ok_or(StateError::InvalidEnvironmentIndex {
                    index: session.selected_environment_index(),
                })?;

        Ok((
            session.collection(),
            session.selected_node_id().clone(),
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

fn selected_startup_collection(state: &AppState) -> Option<DiscoveredCollection> {
    match state.startup() {
        StartupState::CollectionPicker(picker) => picker.selected_collection().cloned(),
        _ => None,
    }
}

fn setup_message_state(message: String) -> AppState {
    let mut state = AppState::new();
    state.show_setup_message(message);
    state
}

fn no_collection_message(config_path: Option<&Path>) -> String {
    let mut message = String::from(
        "No Bruno collection was discovered.\n\nTry one of:\n- launch Brutui from inside a Bruno collection\n- pass a collection path directly: brutui /path/to/collection\n",
    );

    if let Some(config_path) = config_path {
        message.push_str(&format!(
            "- add `collection_dirs` entries to {}\n",
            config_path.display()
        ));
    }

    message.push_str(
        "\nBrutui recognizes Bruno roots containing `bruno.json` or `opencollection.yml`.",
    );
    message
}

fn missing_bru_message(collection_root: &Path, error: &impl std::fmt::Display) -> String {
    format!(
        "Found Bruno collection at {} but could not resolve `bru`.\n\n{}",
        collection_root.display(),
        error
    )
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock, mpsc};

    use clap::Parser;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    use tempfile::tempdir;

    use crate::cli::Cli;
    use crate::collection::{model::CollectionFormat, scanner::scan_collection};
    use crate::config::{AppConfig, LoadedConfig};
    use crate::environments::discover as discover_environments;
    use crate::executable::BRU_PATH_ENV_VAR;
    use crate::runner::{RunCommand, RunCompletion, RunEvent, RunOutcome};
    use crate::state::StartupState;

    use super::{
        AppBootstrap, AppController, AppRuntime, ClipboardPort, RunCancelError, RunEventReceiver,
        RunStartError, RunnerPort,
    };

    static PROCESS_STATE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[test]
    fn explicit_collection_path_wins_at_startup() {
        let workspace = tempdir().expect("temp dir");
        let explicit = classic_collection(workspace.path().join("explicit"));
        let cwd_collection = classic_collection(workspace.path().join("cwd-root"));
        let configured = classic_collection(workspace.path().join("configured"));
        let cwd_nested = cwd_collection.join("requests/users");
        fs::create_dir_all(&cwd_nested).expect("create cwd nested");
        let fake_bru = install_fake_bru(workspace.path().join("bru"));

        let cli = Cli::parse_from([
            "brutui",
            explicit.to_str().expect("explicit path should be utf-8"),
        ]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(
                    Some(workspace.path().join("config.toml")),
                    &[configured.as_path()],
                    Some(fake_bru.as_path()),
                ),
                cwd_nested,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Loaded(controller) => assert_eq!(
                controller
                    .state
                    .session()
                    .expect("session")
                    .collection()
                    .root
                    .canonicalize()
                    .expect("canonical loaded root"),
                explicit.canonicalize().expect("canonical explicit root")
            ),
            other => panic!("expected loaded runtime, got {other:?}"),
        }
    }

    #[test]
    fn current_working_directory_discovery_is_config_optional_and_beats_configured_dirs() {
        let workspace = tempdir().expect("temp dir");
        let cwd_collection = classic_collection(workspace.path().join("cwd-root"));
        let configured = classic_collection(workspace.path().join("configured"));
        let cwd_nested = cwd_collection.join("requests/users");
        fs::create_dir_all(&cwd_nested).expect("create cwd nested");
        let fake_bru = install_fake_bru(workspace.path().join("bru"));
        let _configured = configured;

        let cli = Cli::parse_from(["brutui"]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(None, &[], Some(fake_bru.as_path())),
                cwd_nested,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Loaded(controller) => assert_eq!(
                controller
                    .state
                    .session()
                    .expect("session")
                    .collection()
                    .root
                    .canonicalize()
                    .expect("canonical loaded root"),
                cwd_collection.canonicalize().expect("canonical cwd root")
            ),
            other => panic!("expected loaded runtime, got {other:?}"),
        }
    }

    #[test]
    fn configured_single_collection_auto_opens() {
        let workspace = tempdir().expect("temp dir");
        let configured = classic_collection(workspace.path().join("configured"));
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");
        let fake_bru = install_fake_bru(workspace.path().join("bru"));

        let cli = Cli::parse_from(["brutui"]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(
                    Some(workspace.path().join("config.toml")),
                    &[configured.as_path()],
                    Some(fake_bru.as_path()),
                ),
                outside,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Loaded(controller) => assert_eq!(
                controller
                    .state
                    .session()
                    .expect("session")
                    .collection()
                    .root
                    .canonicalize()
                    .expect("canonical loaded root"),
                configured
                    .canonicalize()
                    .expect("canonical configured root")
            ),
            other => panic!("expected loaded runtime, got {other:?}"),
        }
    }

    #[test]
    fn configured_multiple_collections_show_startup_picker() {
        let workspace = tempdir().expect("temp dir");
        let first = classic_collection(workspace.path().join("collections/alpha"));
        let second = classic_collection(workspace.path().join("collections/beta"));
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");
        let _first = first;
        let _second = second;

        let cli = Cli::parse_from(["brutui"]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(
                    Some(workspace.path().join("config.toml")),
                    &[workspace.path().join("collections").as_path()],
                    None,
                ),
                outside,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Startup { state, .. } => match state.startup() {
                StartupState::CollectionPicker(picker) => {
                    assert_eq!(picker.collections().len(), 2);
                    assert_eq!(picker.filtered_indices().len(), 2);
                }
                other => panic!("expected collection picker, got {other:?}"),
            },
            other => panic!("expected startup runtime, got {other:?}"),
        }
    }

    #[test]
    fn startup_shows_setup_message_when_no_collection_is_found() {
        let workspace = tempdir().expect("temp dir");
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");

        let cli = Cli::parse_from(["brutui"]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(Some(workspace.path().join("config.toml")), &[], None),
                outside,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Startup { state, .. } => match state.startup() {
                StartupState::SetupMessage { message } => {
                    assert!(message.contains("No Bruno collection was discovered"));
                    assert!(message.contains("collection_dirs"));
                    assert!(message.contains("bruno.json"));
                }
                other => panic!("expected setup message, got {other:?}"),
            },
            other => panic!("expected startup runtime, got {other:?}"),
        }
    }

    #[test]
    fn startup_surfaces_missing_bru_as_setup_message() {
        let _lock = lock_process_state();
        let workspace = tempdir().expect("temp dir");
        let collection = classic_collection(workspace.path().join("sample"));
        let outside = workspace.path().join("outside");
        fs::create_dir_all(&outside).expect("create outside dir");
        let _bru_override = remove_env_var(BRU_PATH_ENV_VAR);
        let _path = set_env_var("PATH", "");

        let cli = Cli::parse_from(["brutui"]);
        let bootstrap = AppBootstrap::from_cli(&cli);
        let runtime = bootstrap
            .prepare_runtime_with(
                loaded_config(
                    Some(workspace.path().join("config.toml")),
                    &[collection.as_path()],
                    None,
                ),
                outside,
            )
            .expect("prepare runtime");

        match runtime {
            AppRuntime::Startup { state, .. } => match state.startup() {
                StartupState::SetupMessage { message } => {
                    assert!(message.contains("could not resolve `bru`"));
                    assert!(message.contains("BRUTUI_BRU_PATH"));
                    assert!(message.contains(&collection.display().to_string()));
                }
                other => panic!("expected setup message, got {other:?}"),
            },
            other => panic!("expected startup runtime, got {other:?}"),
        }
    }

    #[test]
    fn controller_uses_injected_runner_and_clipboard_ports() {
        let workspace = tempdir().expect("temp dir");
        let root = classic_collection(workspace.path().join("collection"));
        let collection =
            scan_collection(&root, CollectionFormat::ClassicJson).expect("scan test collection");
        let environments =
            discover_environments(&root, CollectionFormat::ClassicJson).expect("discover envs");
        let (runner, runner_state, event_tx) = FakeRunner::boxed();
        let (clipboard, clipboard_state) = FakeClipboard::boxed();
        let mut controller = AppController::new_loaded_with_ports(
            collection,
            environments,
            "/tmp/fake-bru",
            runner,
            clipboard,
        )
        .expect("create controller");

        controller
            .handle_key_event(press_char('r'))
            .expect("start run through fake runner");
        assert_eq!(
            runner_state
                .lock()
                .expect("runner state")
                .started_commands
                .len(),
            1
        );

        runner_state.lock().expect("runner state").active = false;
        event_tx
            .send(RunEvent::Stdout("booting".to_string()))
            .expect("send stdout event");
        event_tx
            .send(RunEvent::Finished(RunCompletion {
                exit_code: None,
                report_path: PathBuf::from("/tmp/report.json"),
                outcome: RunOutcome::Cancelled,
            }))
            .expect("send completion event");
        controller.pump_run_events().expect("pump events");

        controller
            .handle_key_event(press_char('y'))
            .expect("copy through fake clipboard");
        let copied = clipboard_state.lock().expect("clipboard state");
        assert_eq!(copied.len(), 1);
        assert!(copied[0].contains("Status: cancelled"));
    }

    fn press_char(ch: char) -> KeyEvent {
        let mut event = KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE);
        event.kind = KeyEventKind::Press;
        event
    }

    fn classic_collection(path: PathBuf) -> PathBuf {
        fs::create_dir_all(&path).expect("create collection dir");
        fs::create_dir_all(path.join("environments")).expect("create env dir");
        fs::write(path.join("bruno.json"), "{}\n").expect("write bruno marker");
        fs::write(path.join("request.bru"), "meta {\n  name: sample\n}\n").expect("write request");
        fs::write(path.join("environments/dev.bru"), "vars: {}\n").expect("write env");
        path
    }

    fn loaded_config(
        path: Option<PathBuf>,
        collection_dirs: &[&Path],
        bru_path: Option<&Path>,
    ) -> LoadedConfig {
        LoadedConfig {
            path,
            config: AppConfig {
                collection_dirs: collection_dirs
                    .iter()
                    .map(|path| path.to_path_buf())
                    .collect(),
                bru_path: bru_path.map(Path::to_path_buf),
            },
        }
    }

    fn install_fake_bru(path: PathBuf) -> PathBuf {
        fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write fake bru");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut permissions = fs::metadata(&path)
                .expect("fake bru metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("set fake bru permissions");
        }

        path
    }

    fn lock_process_state() -> MutexGuard<'static, ()> {
        PROCESS_STATE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[derive(Debug)]
    struct EnvVarGuard {
        key: String,
        previous: Option<OsString>,
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(&self.key, value) },
                None => unsafe { std::env::remove_var(&self.key) },
            }
        }
    }

    fn set_env_var(key: impl Into<String>, value: impl AsRef<OsStr>) -> EnvVarGuard {
        let key = key.into();
        let previous = std::env::var_os(&key);
        unsafe { std::env::set_var(&key, value) };
        EnvVarGuard { key, previous }
    }

    fn remove_env_var(key: impl Into<String>) -> EnvVarGuard {
        let key = key.into();
        let previous = std::env::var_os(&key);
        unsafe { std::env::remove_var(&key) };
        EnvVarGuard { key, previous }
    }

    #[derive(Debug, Default)]
    struct FakeClipboard {
        copied: Arc<Mutex<Vec<String>>>,
    }

    impl FakeClipboard {
        fn boxed() -> (Box<dyn ClipboardPort>, Arc<Mutex<Vec<String>>>) {
            let clipboard = Self::default();
            let state = Arc::clone(&clipboard.copied);
            (Box::new(clipboard), state)
        }
    }

    impl ClipboardPort for FakeClipboard {
        fn set_text(&self, text: String) -> Result<(), String> {
            self.copied.lock().expect("clipboard state").push(text);
            Ok(())
        }
    }

    #[derive(Debug, Default)]
    struct FakeRunnerState {
        started_commands: Vec<RunCommand>,
        active: bool,
        event_rx: Option<RunEventReceiver>,
    }

    #[derive(Debug)]
    struct FakeRunner {
        state: Arc<Mutex<FakeRunnerState>>,
    }

    impl FakeRunner {
        fn boxed() -> (
            Box<dyn RunnerPort>,
            Arc<Mutex<FakeRunnerState>>,
            mpsc::Sender<RunEvent>,
        ) {
            let (event_tx, event_rx) = mpsc::channel();
            let state = Arc::new(Mutex::new(FakeRunnerState {
                event_rx: Some(event_rx),
                ..FakeRunnerState::default()
            }));
            (
                Box::new(Self {
                    state: Arc::clone(&state),
                }),
                state,
                event_tx,
            )
        }
    }

    impl RunnerPort for FakeRunner {
        fn start(&self, command: RunCommand) -> Result<RunEventReceiver, RunStartError> {
            let mut state = self.state.lock().expect("runner state");
            state.started_commands.push(command);
            state.active = true;
            state
                .event_rx
                .take()
                .ok_or(RunStartError::ActiveRunInProgress)
        }

        fn cancel(&self) -> Result<(), RunCancelError> {
            self.state.lock().expect("runner state").active = false;
            Ok(())
        }

        fn has_active_run(&self) -> bool {
            self.state.lock().expect("runner state").active
        }
    }
}
