use std::path::PathBuf;

use thiserror::Error;

use crate::collection::model::{Collection, CollectionNode, CollectionNodeId};
use crate::discovery::DiscoveredCollection;
use crate::environments::EnvironmentOption;
use crate::report::RunReport;
use crate::runner::{RunCompletion, RunOutcome, RunToolError};

// ----------------------------------------------------------------
// AppState
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    startup: StartupState,
    session: Option<SessionState>,
}

// ----------------------------------------------------------------
// Startup and collection discovery state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupState {
    Discovering,
    CollectionPicker(CollectionPickerState),
    Ready,
    SetupMessage { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionPickerState {
    collections: Vec<DiscoveredCollection>,
    query: String,
    filtered: Vec<usize>,
    selected_filtered_index: usize,
}

// ----------------------------------------------------------------
// Loaded session state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusPane {
    CollectionTree,
    Details,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    collection: Collection,
    selected_node: CollectionNodeId,
    environments: Vec<EnvironmentOption>,
    selected_environment_index: usize,
    focus: FocusPane,
    modal: ModalState,
    run: RunState,
    response_tab: ResponseTab,
    selected_result_index: usize,
    raw_output: Vec<OutputLine>,
    latest_report_path: Option<PathBuf>,
    completed_run: Option<CompletedRun>,
    scroll: PaneScrollState,
}

// ----------------------------------------------------------------
// Modal and overlay state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalState {
    None,
    Help,
    EnvironmentPicker { highlighted_index: usize },
}

// ----------------------------------------------------------------
// Active run state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    Idle,
    Running(ActiveRunState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRunState {
    pub target: CollectionNodeId,
    pub cancellation_requested: bool,
}

// ----------------------------------------------------------------
// Output and result view state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
    Tests,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLine {
    pub stream: OutputStream,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaneScrollState {
    pub output: ScrollState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScrollState {
    pub vertical_offset: usize,
    pub viewport_height: usize,
    pub content_height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewMeasurements {
    pub output: Option<OutputScrollMeasurements>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputScrollMeasurements {
    pub viewport_height: usize,
    pub content_height: usize,
}

impl ScrollState {
    pub fn max_vertical_offset(&self) -> usize {
        self.content_height.saturating_sub(self.viewport_height)
    }

    pub fn clamp(&mut self) {
        self.vertical_offset = self.vertical_offset.min(self.max_vertical_offset());
    }

    pub fn reset(&mut self) {
        self.vertical_offset = 0;
    }
}

// ----------------------------------------------------------------
// Completed run state
// ----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedRun {
    pub target: CollectionNodeId,
    pub exit_code: Option<i32>,
    pub report_path: PathBuf,
    pub status: CompletedRunStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletedRunStatus {
    Success(RunReport),
    FailedTests(RunReport),
    Cancelled,
    ToolError(RunToolError),
}

// ----------------------------------------------------------------
// State transition errors
// ----------------------------------------------------------------

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StateError {
    #[error("no collection is currently loaded")]
    NoCollectionLoaded,

    #[error("the current collection has no selectable nodes")]
    EmptyCollection,

    #[error("the requested environment index {index} is out of bounds")]
    InvalidEnvironmentIndex { index: usize },

    #[error("no environment picker is open")]
    EnvironmentPickerClosed,

    #[error("the requested collection node is not selectable in the current session")]
    InvalidSelectedNode,

    #[error("another Bruno run is already active")]
    RunAlreadyActive,

    #[error("no active Bruno run is in progress")]
    NoActiveRun,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            startup: StartupState::Discovering,
            session: None,
        }
    }

    pub fn startup(&self) -> &StartupState {
        &self.startup
    }

    pub fn session(&self) -> Option<&SessionState> {
        self.session.as_ref()
    }

    pub fn apply_view_measurements(&mut self, measurements: ViewMeasurements) {
        if let (Some(session), Some(output)) = (&mut self.session, measurements.output) {
            session.apply_output_scroll_measurements(output);
        }
    }

    // ----------------------------------------------------------------
    // Startup and collection discovery transitions
    // ----------------------------------------------------------------

    pub fn show_collection_picker(&mut self, collections: Vec<DiscoveredCollection>) {
        self.startup = StartupState::CollectionPicker(CollectionPickerState::new(collections));
    }

    pub fn show_setup_message(&mut self, message: impl Into<String>) {
        self.startup = StartupState::SetupMessage {
            message: message.into(),
        };
    }

    pub fn open_collection(
        &mut self,
        collection: Collection,
        environments: Vec<EnvironmentOption>,
    ) -> Result<(), StateError> {
        let selected_node = collection
            .visible_nodes()
            .first()
            .map(CollectionNode::id)
            .ok_or(StateError::EmptyCollection)?;
        let selected_environment_index = default_environment_index(&environments);

        self.startup = StartupState::Ready;
        self.session = Some(SessionState {
            collection,
            selected_node,
            environments,
            selected_environment_index,
            focus: FocusPane::CollectionTree,
            modal: ModalState::None,
            run: RunState::Idle,
            response_tab: ResponseTab::Body,
            selected_result_index: 0,
            raw_output: Vec::new(),
            latest_report_path: None,
            completed_run: None,
            scroll: PaneScrollState::default(),
        });

        Ok(())
    }

    pub fn set_collection_picker_query(&mut self, query: impl Into<String>) {
        if let StartupState::CollectionPicker(picker) = &mut self.startup {
            picker.set_query(query);
        }
    }

    pub fn move_collection_picker_next(&mut self) {
        if let StartupState::CollectionPicker(picker) = &mut self.startup {
            picker.move_next();
        }
    }

    pub fn move_collection_picker_previous(&mut self) {
        if let StartupState::CollectionPicker(picker) = &mut self.startup {
            picker.move_previous();
        }
    }

    // ----------------------------------------------------------------
    // Session navigation and focus transitions
    // ----------------------------------------------------------------

    pub fn cycle_focus_forward(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        session.focus = match session.focus {
            FocusPane::CollectionTree => FocusPane::Details,
            FocusPane::Details => FocusPane::Output,
            FocusPane::Output => FocusPane::CollectionTree,
        };
        Ok(())
    }

    pub fn cycle_focus_backward(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        session.focus = match session.focus {
            FocusPane::CollectionTree => FocusPane::Output,
            FocusPane::Details => FocusPane::CollectionTree,
            FocusPane::Output => FocusPane::Details,
        };
        Ok(())
    }

    pub fn move_selection_next(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let current_index = session
            .selected_node_index()
            .ok_or(StateError::EmptyCollection)?;
        let next_index = (current_index + 1).min(session.collection.node_count() - 1);
        session.selected_node = session
            .collection
            .node_at(next_index)
            .expect("next index should stay within collection bounds")
            .id();
        Ok(())
    }

    pub fn move_selection_previous(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let current_index = session
            .selected_node_index()
            .ok_or(StateError::EmptyCollection)?;
        let previous_index = current_index.saturating_sub(1);
        session.selected_node = session
            .collection
            .node_at(previous_index)
            .expect("previous index should stay within collection bounds")
            .id();
        Ok(())
    }

    pub fn select_node(&mut self, node_id: CollectionNodeId) -> Result<(), StateError> {
        self.session_mut()?.select_node(node_id)
    }

    // ----------------------------------------------------------------
    // Modal and environment-picker transitions
    // ----------------------------------------------------------------

    pub fn open_help(&mut self) -> Result<(), StateError> {
        self.session_mut()?.modal = ModalState::Help;
        Ok(())
    }

    pub fn open_environment_picker(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        session.modal = ModalState::EnvironmentPicker {
            highlighted_index: session.selected_environment_index,
        };
        Ok(())
    }

    pub fn close_modal(&mut self) -> Result<(), StateError> {
        self.session_mut()?.modal = ModalState::None;
        Ok(())
    }

    pub fn move_environment_highlight_next(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let environment_count = session.environments.len();
        match &mut session.modal {
            ModalState::EnvironmentPicker { highlighted_index } => {
                if environment_count > 0 {
                    *highlighted_index = (*highlighted_index + 1).min(environment_count - 1);
                }
                Ok(())
            }
            _ => Err(StateError::EnvironmentPickerClosed),
        }
    }

    pub fn move_environment_highlight_previous(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        match &mut session.modal {
            ModalState::EnvironmentPicker { highlighted_index } => {
                *highlighted_index = highlighted_index.saturating_sub(1);
                Ok(())
            }
            _ => Err(StateError::EnvironmentPickerClosed),
        }
    }

    pub fn confirm_environment_selection(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let index = match session.modal {
            ModalState::EnvironmentPicker { highlighted_index } => highlighted_index,
            _ => return Err(StateError::EnvironmentPickerClosed),
        };
        if index >= session.environments.len() {
            return Err(StateError::InvalidEnvironmentIndex { index });
        }

        session.selected_environment_index = index;
        session.modal = ModalState::None;
        Ok(())
    }

    // ----------------------------------------------------------------
    // Session selectors
    // ----------------------------------------------------------------

    pub fn selected_environment(&self) -> Option<&EnvironmentOption> {
        self.session
            .as_ref()
            .and_then(|session| session.environments.get(session.selected_environment_index))
    }

    pub fn selected_node(&self) -> Option<&CollectionNode> {
        self.session.as_ref().and_then(SessionState::selected_node)
    }

    // ----------------------------------------------------------------
    // Run lifecycle, output, and result-view transitions
    // ----------------------------------------------------------------

    pub fn start_run_on_selected_node(&mut self) -> Result<CollectionNodeId, StateError> {
        let session = self.session_mut()?;
        if matches!(session.run, RunState::Running(_)) {
            return Err(StateError::RunAlreadyActive);
        }

        let target = session.selected_node.clone();
        session.run = RunState::Running(ActiveRunState {
            target: target.clone(),
            cancellation_requested: false,
        });
        session.response_tab = ResponseTab::Body;
        session.selected_result_index = 0;
        session.raw_output.clear();
        session.completed_run = None;
        session.reset_output_scroll();
        Ok(target)
    }

    pub fn request_run_cancellation(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        match &mut session.run {
            RunState::Running(active) => {
                active.cancellation_requested = true;
                Ok(())
            }
            RunState::Idle => Err(StateError::NoActiveRun),
        }
    }

    pub fn append_stdout(&mut self, text: impl Into<String>) -> Result<(), StateError> {
        self.session_mut()?.raw_output.push(OutputLine {
            stream: OutputStream::Stdout,
            text: text.into(),
        });
        Ok(())
    }

    pub fn append_stderr(&mut self, text: impl Into<String>) -> Result<(), StateError> {
        self.session_mut()?.raw_output.push(OutputLine {
            stream: OutputStream::Stderr,
            text: text.into(),
        });
        Ok(())
    }

    pub fn finish_run(&mut self, completion: RunCompletion) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let target = match &session.run {
            RunState::Running(active) => active.target.clone(),
            RunState::Idle => return Err(StateError::NoActiveRun),
        };

        session.latest_report_path = Some(completion.report_path.clone());
        session.completed_run = Some(CompletedRun::from_completion(target, completion));
        session.run = RunState::Idle;
        session.response_tab = ResponseTab::Body;
        session.selected_result_index = 0;
        session.reset_output_scroll();
        Ok(())
    }

    pub fn set_response_tab(&mut self, tab: ResponseTab) -> Result<(), StateError> {
        let session = self.session_mut()?;
        if session.response_tab != tab {
            session.response_tab = tab;
            session.reset_output_scroll();
        }
        Ok(())
    }

    pub fn select_previous_result(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let previous_index = session.selected_result_index;
        session.selected_result_index = session.selected_result_index.saturating_sub(1);
        if session.selected_result_index != previous_index {
            session.reset_output_scroll();
        }
        Ok(())
    }

    pub fn select_next_result(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        let previous_index = session.selected_result_index;
        let max = session.response_result_count().saturating_sub(1);
        session.selected_result_index = (session.selected_result_index + 1).min(max);
        if session.selected_result_index != previous_index {
            session.reset_output_scroll();
        }
        Ok(())
    }

    pub fn scroll_output_by(&mut self, delta: isize) -> Result<(), StateError> {
        self.session_mut()?.scroll_output_by(delta);
        Ok(())
    }

    pub fn scroll_output_to_top(&mut self) -> Result<(), StateError> {
        self.session_mut()?.scroll.output.reset();
        Ok(())
    }

    pub fn scroll_output_to_bottom(&mut self) -> Result<(), StateError> {
        let session = self.session_mut()?;
        session.scroll.output.vertical_offset = session.scroll.output.max_vertical_offset();
        Ok(())
    }

    // ----------------------------------------------------------------
    // Internal session access
    // ----------------------------------------------------------------

    fn session_mut(&mut self) -> Result<&mut SessionState, StateError> {
        self.session.as_mut().ok_or(StateError::NoCollectionLoaded)
    }
}

// ----------------------------------------------------------------
// CollectionPickerState behavior
// ----------------------------------------------------------------

impl CollectionPickerState {
    pub fn new(collections: Vec<DiscoveredCollection>) -> Self {
        let mut picker = Self {
            collections,
            query: String::new(),
            filtered: Vec::new(),
            selected_filtered_index: 0,
        };
        picker.rebuild_filtered();
        picker
    }

    pub fn collections(&self) -> &[DiscoveredCollection] {
        &self.collections
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered
    }

    pub fn selected_filtered_index(&self) -> usize {
        self.selected_filtered_index
    }

    pub fn selected_collection(&self) -> Option<&DiscoveredCollection> {
        self.filtered
            .get(self.selected_filtered_index)
            .and_then(|index| self.collections.get(*index))
    }

    fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.rebuild_filtered();
    }

    fn move_next(&mut self) {
        if self.filtered.is_empty() {
            self.selected_filtered_index = 0;
        } else {
            self.selected_filtered_index =
                (self.selected_filtered_index + 1).min(self.filtered.len() - 1);
        }
    }

    fn move_previous(&mut self) {
        self.selected_filtered_index = self.selected_filtered_index.saturating_sub(1);
    }

    fn rebuild_filtered(&mut self) {
        let query = self.query.to_lowercase();
        self.filtered = self
            .collections
            .iter()
            .enumerate()
            .filter(|(_, collection)| {
                if query.is_empty() {
                    return true;
                }

                let root = collection.root.to_string_lossy().to_lowercase();
                root.contains(&query)
            })
            .map(|(index, _)| index)
            .collect();
        self.selected_filtered_index = 0;
    }
}

// ----------------------------------------------------------------
// SessionState selectors
// ----------------------------------------------------------------

impl SessionState {
    pub fn collection(&self) -> &Collection {
        &self.collection
    }

    pub fn selected_node_id(&self) -> &CollectionNodeId {
        &self.selected_node
    }

    pub fn environments(&self) -> &[EnvironmentOption] {
        &self.environments
    }

    pub fn selected_environment_index(&self) -> usize {
        self.selected_environment_index
    }

    pub fn selected_environment(&self) -> Option<&EnvironmentOption> {
        self.environments.get(self.selected_environment_index)
    }

    pub fn focus(&self) -> &FocusPane {
        &self.focus
    }

    pub fn modal(&self) -> &ModalState {
        &self.modal
    }

    pub fn run_state(&self) -> &RunState {
        &self.run
    }

    pub fn response_tab(&self) -> &ResponseTab {
        &self.response_tab
    }

    pub fn selected_result_index(&self) -> usize {
        self.selected_result_index
    }

    pub fn raw_output(&self) -> &[OutputLine] {
        &self.raw_output
    }

    pub fn latest_report_path(&self) -> Option<&PathBuf> {
        self.latest_report_path.as_ref()
    }

    pub fn completed_run(&self) -> Option<&CompletedRun> {
        self.completed_run.as_ref()
    }

    pub fn scroll(&self) -> &PaneScrollState {
        &self.scroll
    }

    pub fn response_result_count(&self) -> usize {
        self.completed_run
            .as_ref()
            .and_then(|run| match &run.status {
                CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) => {
                    Some(report.requests.len())
                }
                _ => None,
            })
            .unwrap_or(0)
    }

    pub fn selected_response_result(&self) -> Option<&crate::report::RequestResult> {
        self.completed_run
            .as_ref()
            .and_then(|run| match &run.status {
                CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) => {
                    report.requests.get(self.selected_result_index)
                }
                _ => None,
            })
    }

    pub fn selected_node(&self) -> Option<&CollectionNode> {
        self.collection.node(&self.selected_node)
    }

    pub fn select_node(&mut self, node_id: CollectionNodeId) -> Result<(), StateError> {
        if self.collection.contains_node(&node_id) {
            self.selected_node = node_id;
            Ok(())
        } else {
            Err(StateError::InvalidSelectedNode)
        }
    }

    pub fn apply_output_scroll_measurements(&mut self, measurements: OutputScrollMeasurements) {
        self.scroll.output.viewport_height = measurements.viewport_height;
        self.scroll.output.content_height = measurements.content_height;
        self.scroll.output.clamp();
    }

    fn reset_output_scroll(&mut self) {
        self.scroll.output.reset();
    }

    fn scroll_output_by(&mut self, delta: isize) {
        let output = &mut self.scroll.output;
        output.vertical_offset = if delta.is_negative() {
            output.vertical_offset.saturating_sub(delta.unsigned_abs())
        } else {
            output.vertical_offset.saturating_add(delta as usize)
        };
        output.clamp();
    }

    fn selected_node_index(&self) -> Option<usize> {
        self.collection.node_index(&self.selected_node)
    }
}

// ----------------------------------------------------------------
// CompletedRun construction
// ----------------------------------------------------------------

impl CompletedRun {
    fn from_completion(target: CollectionNodeId, completion: RunCompletion) -> Self {
        let status = match completion.outcome {
            RunOutcome::Success(report) => CompletedRunStatus::Success(report),
            RunOutcome::CompletedWithFailures(report) => CompletedRunStatus::FailedTests(report),
            RunOutcome::Cancelled => CompletedRunStatus::Cancelled,
            RunOutcome::ToolError(error) => CompletedRunStatus::ToolError(error),
        };

        Self {
            target,
            exit_code: completion.exit_code,
            report_path: completion.report_path,
            status,
        }
    }
}

// ----------------------------------------------------------------
// Environment defaults
// ----------------------------------------------------------------

fn default_environment_index(environments: &[EnvironmentOption]) -> usize {
    environments
        .iter()
        .position(|option| option.cli_value.is_none())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::collection::model::{
        Collection, CollectionFormat, CollectionNode, CollectionNodeId, FolderNode, RequestNode,
        RootNode,
    };
    use crate::discovery::{DiscoveredCollection, DiscoverySource};
    use crate::environments::EnvironmentOption;
    use crate::metadata::RequestMetadata;
    use crate::report::{ReportSummary, RunReport};
    use crate::runner::{RunCompletion, RunOutcome, RunToolError, RunToolErrorKind};

    use super::{
        AppState, CompletedRunStatus, FocusPane, ModalState, OutputScrollMeasurements,
        OutputStream, ResponseTab, RunState, StartupState, StateError, ViewMeasurements,
    };

    #[test]
    fn collection_picker_filters_and_tracks_selection() {
        let mut state = AppState::new();
        state.show_collection_picker(vec![
            discovered("/collections/payments"),
            discovered("/collections/catalog"),
        ]);

        state.set_collection_picker_query("cat");
        state.move_collection_picker_next();

        let StartupState::CollectionPicker(picker) = &state.startup else {
            panic!("expected picker state");
        };
        assert_eq!(picker.filtered.len(), 1);
        assert_eq!(
            picker
                .selected_collection()
                .expect("selected collection")
                .root,
            PathBuf::from("/collections/catalog")
        );
    }

    #[test]
    fn selection_movement_clamps_at_collection_edges() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        state.move_selection_next().expect("move to folder");
        state.move_selection_next().expect("move to request");
        state.move_selection_next().expect("stay on request");

        assert_eq!(
            state.selected_node().expect("selected node").id(),
            CollectionNodeId::Request(PathBuf::from("users/list.bru"))
        );

        state.move_selection_previous().expect("move to folder");
        state.move_selection_previous().expect("move to root");
        state.move_selection_previous().expect("stay on root");

        assert_eq!(
            state.selected_node().expect("selected node").id(),
            CollectionNodeId::Root
        );
    }

    #[test]
    fn environment_picker_changes_selected_environment() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        state
            .open_environment_picker()
            .expect("open environment picker");
        state
            .move_environment_highlight_next()
            .expect("highlight next environment");
        state
            .confirm_environment_selection()
            .expect("confirm environment");

        assert_eq!(
            state
                .selected_environment()
                .expect("selected environment")
                .cli_value,
            Some("dev".to_string())
        );
        assert_eq!(state.session().expect("session").modal(), &ModalState::None);
    }

    #[test]
    fn invalid_node_selection_is_rejected_and_keeps_current_selection() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        let error = state
            .select_node(CollectionNodeId::Request(PathBuf::from("missing.bru")))
            .expect_err("invalid node selection should be rejected");

        assert_eq!(error, StateError::InvalidSelectedNode);
        assert_eq!(
            state.selected_node().expect("selected node").id(),
            CollectionNodeId::Root
        );
    }

    #[test]
    fn focus_cycles_between_panes() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        state.cycle_focus_forward().expect("focus details");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::Details
        );
        state.cycle_focus_forward().expect("focus output");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::Output
        );
        state.cycle_focus_forward().expect("focus tree");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::CollectionTree
        );
    }

    #[test]
    fn focus_cycles_backward_between_panes() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        // Starts at CollectionTree, going backward should go to Output
        state.cycle_focus_backward().expect("focus output");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::Output
        );
        state.cycle_focus_backward().expect("focus details");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::Details
        );
        state.cycle_focus_backward().expect("focus tree");
        assert_eq!(
            state.session().expect("session").focus(),
            &FocusPane::CollectionTree
        );
    }

    #[test]
    fn run_start_blocks_overlapping_runs_and_resets_output() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");
        state.append_stdout("stale line").expect("append output");

        let target = state.start_run_on_selected_node().expect("start run");
        let error = state
            .start_run_on_selected_node()
            .expect_err("second run should be rejected");

        assert_eq!(target, CollectionNodeId::Root);
        assert_eq!(error, StateError::RunAlreadyActive);
        assert!(matches!(
            state.session.as_ref().expect("session").run,
            RunState::Running(_)
        ));
        assert!(
            state
                .session
                .as_ref()
                .expect("session")
                .raw_output
                .is_empty()
        );
        assert_eq!(
            state.session.as_ref().expect("session").response_tab,
            ResponseTab::Body
        );
    }

    #[test]
    fn cancellation_is_recorded_before_completion_and_surface_status_is_cancelled() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");
        state.start_run_on_selected_node().expect("start run");

        state
            .request_run_cancellation()
            .expect("request cancellation");

        let RunState::Running(active) = &state.session.as_ref().expect("session").run else {
            panic!("expected running state");
        };
        assert!(active.cancellation_requested);

        state
            .finish_run(RunCompletion {
                exit_code: Some(0),
                report_path: PathBuf::from("/tmp/cancelled.json"),
                outcome: RunOutcome::Cancelled,
            })
            .expect("finish run");

        assert!(matches!(
            state
                .session
                .as_ref()
                .and_then(|session| session.completed_run.as_ref())
                .map(|run| &run.status),
            Some(CompletedRunStatus::Cancelled)
        ));
    }

    #[test]
    fn run_completion_surfaces_success_failure_and_tool_error_classifications() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        for completion in [
            RunCompletion {
                exit_code: Some(0),
                report_path: PathBuf::from("/tmp/success.json"),
                outcome: RunOutcome::Success(sample_report()),
            },
            RunCompletion {
                exit_code: Some(1),
                report_path: PathBuf::from("/tmp/failures.json"),
                outcome: RunOutcome::CompletedWithFailures(sample_report()),
            },
            RunCompletion {
                exit_code: Some(7),
                report_path: PathBuf::from("/tmp/error.json"),
                outcome: RunOutcome::ToolError(RunToolError {
                    kind: RunToolErrorKind::NonZeroExit,
                    message: "Bruno exited with status 7".to_string(),
                }),
            },
        ] {
            state.start_run_on_selected_node().expect("start run");
            state.finish_run(completion).expect("finish run");
        }

        match &state
            .session
            .as_ref()
            .expect("session")
            .completed_run
            .as_ref()
            .expect("completed run")
            .status
        {
            CompletedRunStatus::ToolError(error) => {
                assert_eq!(error.kind, RunToolErrorKind::NonZeroExit);
            }
            other => panic!("expected tool error classification, got {other:?}"),
        }
    }

    #[test]
    fn raw_output_and_latest_report_are_retained_after_completion() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");
        state.start_run_on_selected_node().expect("start run");
        state.append_stdout("stdout line").expect("append stdout");
        state.append_stderr("stderr line").expect("append stderr");

        state
            .finish_run(RunCompletion {
                exit_code: Some(1),
                report_path: PathBuf::from("/tmp/latest-report.json"),
                outcome: RunOutcome::CompletedWithFailures(sample_report()),
            })
            .expect("finish run");

        let session = state.session.as_ref().expect("session");
        assert_eq!(session.raw_output.len(), 2);
        assert_eq!(session.raw_output[0].stream, OutputStream::Stdout);
        assert_eq!(session.raw_output[1].stream, OutputStream::Stderr);
        assert_eq!(
            session.latest_report_path,
            Some(PathBuf::from("/tmp/latest-report.json"))
        );
        assert_eq!(session.response_tab, ResponseTab::Body);
    }

    #[test]
    fn applying_view_measurements_clamps_output_scroll_offsets() {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");

        state.apply_view_measurements(ViewMeasurements {
            output: Some(OutputScrollMeasurements {
                viewport_height: 4,
                content_height: 20,
            }),
        });
        state.scroll_output_to_bottom().expect("scroll to bottom");

        state.apply_view_measurements(ViewMeasurements {
            output: Some(OutputScrollMeasurements {
                viewport_height: 5,
                content_height: 6,
            }),
        });

        assert_eq!(
            state
                .session()
                .expect("session")
                .scroll()
                .output
                .vertical_offset,
            1
        );
        assert_eq!(
            state
                .session()
                .expect("session")
                .scroll()
                .output
                .max_vertical_offset(),
            1
        );
    }

    #[test]
    fn loading_a_collection_requires_a_selectable_root_node() {
        let mut state = AppState::new();
        let error = state
            .open_collection(
                Collection::new(
                    PathBuf::from("/tmp/empty"),
                    CollectionFormat::ClassicJson,
                    Vec::new(),
                ),
                sample_environments(),
            )
            .expect_err("empty collections should be rejected");

        assert_eq!(error, StateError::EmptyCollection);
    }

    fn discovered(path: &str) -> DiscoveredCollection {
        DiscoveredCollection {
            root: PathBuf::from(path),
            format: CollectionFormat::ClassicJson,
            source: DiscoverySource::ConfiguredDirectory,
        }
    }

    fn sample_collection() -> Collection {
        Collection::new(
            PathBuf::from("/collections/demo"),
            CollectionFormat::ClassicJson,
            vec![
                CollectionNode::Root(RootNode {
                    path: PathBuf::from("/collections/demo"),
                    display_name: "demo".to_string(),
                }),
                CollectionNode::Folder(FolderNode {
                    path: PathBuf::from("/collections/demo/users"),
                    relative_path: PathBuf::from("users"),
                    display_name: "users".to_string(),
                }),
                CollectionNode::Request(RequestNode {
                    path: PathBuf::from("/collections/demo/users/list.bru"),
                    relative_path: PathBuf::from("users/list.bru"),
                    display_name: "list.bru".to_string(),
                    metadata: RequestMetadata::default(),
                    metadata_diagnostics: Vec::new(),
                }),
            ],
        )
    }

    fn sample_environments() -> Vec<EnvironmentOption> {
        vec![
            EnvironmentOption::no_environment(),
            EnvironmentOption {
                display_name: "dev".to_string(),
                cli_value: Some("dev".to_string()),
            },
        ]
    }

    fn sample_report() -> RunReport {
        RunReport {
            summary: ReportSummary {
                total_requests: 1,
                passed_requests: 1,
                failed_requests: 0,
                total_tests: 1,
                passed_tests: 1,
                failed_tests: 0,
                total_assertions: 1,
                passed_assertions: 1,
                failed_assertions: 0,
                error_count: 0,
            },
            requests: Vec::new(),
            diagnostics: Vec::new(),
            source_path: None,
        }
    }
}
