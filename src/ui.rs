use std::io::{self, Stdout};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::{
    collection::model::{CollectionNode, CollectionNodeId},
    state::{
        AppState, CompletedRunStatus, ModalState, OutputLine, OutputStream, ResponseTab, RunState,
        SessionState, StartupState, StateError,
    },
};

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusPane {
    CollectionTree,
    Details,
    Output,
}

#[derive(Debug)]
pub struct TerminalSession {
    terminal: AppTerminal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEventResult {
    Continue,
    Quit,
    StartupCollectionChosen,
}

impl TerminalSession {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.hide_cursor()?;

        Ok(Self { terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut AppTerminal {
        &mut self.terminal
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.terminal.show_cursor();
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

pub fn handle_key_event(
    state: &mut AppState,
    event: KeyEvent,
) -> Result<UiEventResult, StateError> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return Ok(UiEventResult::Continue);
    }

    let Some(session) = state.session.as_ref() else {
        return handle_startup_keys(state, event);
    };

    match &session.modal {
        ModalState::Help => handle_help_modal_keys(state, event),
        ModalState::EnvironmentPicker { .. } => handle_environment_picker_keys(state, event),
        ModalState::None => handle_session_keys(state, event),
    }
}

pub fn render(frame: &mut Frame, state: &AppState) {
    match &state.session {
        Some(session) => render_session(frame, session),
        None => render_startup(frame, &state.startup),
    }
}

fn handle_startup_keys(state: &mut AppState, event: KeyEvent) -> Result<UiEventResult, StateError> {
    match &state.startup {
        StartupState::CollectionPicker(_) => match event.code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(UiEventResult::Quit),
            KeyCode::Up | KeyCode::Char('k') => {
                state.move_collection_picker_previous();
                Ok(UiEventResult::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.move_collection_picker_next();
                Ok(UiEventResult::Continue)
            }
            KeyCode::Backspace => {
                if let StartupState::CollectionPicker(picker) = &state.startup {
                    let mut query = picker.query.clone();
                    query.pop();
                    state.set_collection_picker_query(query);
                }
                Ok(UiEventResult::Continue)
            }
            KeyCode::Enter => {
                if let StartupState::CollectionPicker(picker) = &state.startup {
                    if picker.selected_collection().is_some() {
                        return Ok(UiEventResult::StartupCollectionChosen);
                    }
                }
                Ok(UiEventResult::Continue)
            }
            KeyCode::Char(character) => {
                if let StartupState::CollectionPicker(picker) = &state.startup {
                    let mut query = picker.query.clone();
                    query.push(character);
                    state.set_collection_picker_query(query);
                }
                Ok(UiEventResult::Continue)
            }
            _ => Ok(UiEventResult::Continue),
        },
        StartupState::Discovering | StartupState::Ready | StartupState::SetupMessage { .. } => {
            Ok(match event.code {
                KeyCode::Char('q') | KeyCode::Esc => UiEventResult::Quit,
                _ => UiEventResult::Continue,
            })
        }
    }
}

fn handle_help_modal_keys(
    state: &mut AppState,
    event: KeyEvent,
) -> Result<UiEventResult, StateError> {
    match event.code {
        KeyCode::Esc | KeyCode::Char('?') => {
            state.close_modal()?;
            Ok(UiEventResult::Continue)
        }
        KeyCode::Char('q') => Ok(UiEventResult::Quit),
        _ => Ok(UiEventResult::Continue),
    }
}

fn handle_environment_picker_keys(
    state: &mut AppState,
    event: KeyEvent,
) -> Result<UiEventResult, StateError> {
    match event.code {
        KeyCode::Up | KeyCode::Char('k') => state.move_environment_highlight_previous()?,
        KeyCode::Down | KeyCode::Char('j') => state.move_environment_highlight_next()?,
        KeyCode::Enter => state.confirm_environment_selection()?,
        KeyCode::Esc => state.close_modal()?,
        KeyCode::Char('q') => return Ok(UiEventResult::Quit),
        _ => {}
    }

    Ok(UiEventResult::Continue)
}

fn handle_session_keys(state: &mut AppState, event: KeyEvent) -> Result<UiEventResult, StateError> {
    let focus = state
        .session
        .as_ref()
        .map(|session| session.focus.clone())
        .ok_or(StateError::NoCollectionLoaded)?;

    match event.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(UiEventResult::Quit),
        KeyCode::Char('?') => state.open_help()?,
        KeyCode::Tab => state.cycle_focus_forward()?,
        KeyCode::Up | KeyCode::Char('k') if focus == FocusPane::CollectionTree => {
            state.move_selection_previous()?
        }
        KeyCode::Down | KeyCode::Char('j') if focus == FocusPane::CollectionTree => {
            state.move_selection_next()?
        }
        _ => {}
    }

    Ok(UiEventResult::Continue)
}

fn render_startup(frame: &mut Frame, startup: &StartupState) {
    match startup {
        StartupState::CollectionPicker(picker) => render_collection_picker(frame, picker),
        StartupState::Discovering => {
            render_startup_message(frame, "Discovering Bruno collections...")
        }
        StartupState::Ready => render_startup_message(frame, "Preparing session..."),
        StartupState::SetupMessage { message } => render_startup_message(frame, message),
    }
}

fn render_startup_message(frame: &mut Frame, message: &str) {
    frame.render_widget(
        Paragraph::new(message)
            .block(Block::default().borders(Borders::ALL).title("Brutui"))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        frame.area(),
    );
}

fn render_collection_picker(frame: &mut Frame, picker: &crate::state::CollectionPickerState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(format!("Search: {}", picker.query))
            .block(Block::default().borders(Borders::ALL).title("Brutui"))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );

    let items = if picker.filtered.is_empty() {
        vec![ListItem::new("No matching collections")]
    } else {
        picker
            .filtered
            .iter()
            .enumerate()
            .map(|(filtered_index, collection_index)| {
                let collection = &picker.collections[*collection_index];
                let prefix = if filtered_index == picker.selected_filtered_index {
                    "> "
                } else {
                    "  "
                };
                let style = if filtered_index == picker.selected_filtered_index {
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::styled(
                    format!("{prefix}{}", collection.root.display()),
                    style,
                ))
            })
            .collect::<Vec<_>>()
    };

    frame.render_widget(
        List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Collection picker"),
        ),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Type to filter • Enter to open • ↑/↓ or j/k to move • q or Esc to quit")
            .block(Block::default().borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        chunks[2],
    );
}

fn render_session(frame: &mut Frame, session: &SessionState) {
    let root_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(root_chunks[0]);
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(content_chunks[1]);

    render_collection_tree(frame, content_chunks[0], session);
    render_details_pane(frame, right_chunks[0], session);
    render_output_pane(frame, right_chunks[1], session);
    render_footer(frame, root_chunks[1], session);
    render_modal(frame, session);
}

fn render_collection_tree(frame: &mut Frame, area: Rect, session: &SessionState) {
    let selected_node = &session.selected_node;
    let items = session
        .collection
        .nodes
        .iter()
        .map(|node| {
            let prefix = match node {
                CollectionNode::Root(_) => "◉ ".to_string(),
                CollectionNode::Folder(folder) => {
                    format!(
                        "{}📁 ",
                        "  ".repeat(folder.relative_path.components().count())
                    )
                }
                CollectionNode::Request(request) => {
                    format!(
                        "{}↳ ",
                        "  ".repeat(request.relative_path.components().count())
                    )
                }
            };
            let selected = node.id() == *selected_node;
            let style = if selected && session.focus == FocusPane::CollectionTree {
                Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
            } else if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            ListItem::new(Line::styled(
                format!("{prefix}{}", node.display_name()),
                style,
            ))
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(focused_block(
        "Collection tree",
        session.focus == FocusPane::CollectionTree,
    ));

    frame.render_widget(list, area);
}

fn render_details_pane(frame: &mut Frame, area: Rect, session: &SessionState) {
    let lines = if let Some(node) = session.selected_node() {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                selected_node_label(&node.id()),
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(format!("Name: {}", node.display_name())),
            Line::from(format!("Path: {}", node.path().display())),
            Line::from(format!(
                "Environment: {}",
                selected_environment_label(session)
            )),
        ];

        if let Some(relative_path) = node.relative_path() {
            lines.push(Line::from(format!("Relative: {}", relative_path.display())));
        }

        if let CollectionNode::Request(request) = node {
            if let Some(name) = &request.metadata.name {
                lines.push(Line::from(format!("Request name: {name}")));
            }

            let mut method_url_parts = Vec::new();
            if let Some(method) = &request.metadata.method {
                method_url_parts.push(format!("Method: {method}"));
            }
            if let Some(url) = &request.metadata.url {
                method_url_parts.push(format!("URL: {url}"));
            }
            if !method_url_parts.is_empty() {
                lines.push(Line::from(method_url_parts.join("  •  ")));
            }

            if !request.metadata.tags.is_empty() {
                lines.push(Line::from(format!(
                    "Tags (read-only): {}",
                    request.metadata.tags.join(", ")
                )));
            }
            if !request.metadata_diagnostics.is_empty() {
                lines.push(Line::from(format!(
                    "Metadata notes: {}",
                    request
                        .metadata_diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )));
            }
        }

        lines.push(Line::from(
            "e environment • r run • c cancel • 1 body • 2 headers • 3 tests • [/] results • y copy",
        ));
        lines
    } else {
        vec![Line::from("No collection node selected.")]
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(focused_block(
                "Details",
                session.focus == FocusPane::Details,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_output_pane(frame: &mut Frame, area: Rect, session: &SessionState) {
    let result_count = session.response_result_count();
    let result_position = if result_count == 0 {
        "no result".to_string()
    } else {
        format!(
            "result {}/{}",
            session.selected_result_index + 1,
            result_count
        )
    };
    let tab = match session.response_tab {
        ResponseTab::Body => "Body",
        ResponseTab::Headers => "Headers",
        ResponseTab::Tests => "Tests",
    };
    let title = format!("Output: Response viewer ({tab}, {result_position})");
    let lines = current_tab_text(session)
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines)
            .block(focused_block(&title, session.focus == FocusPane::Output))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, session: &SessionState) {
    let environment = selected_environment_label(session);
    let run_state = match &session.run {
        crate::state::RunState::Running(active) if active.cancellation_requested => "cancelling",
        crate::state::RunState::Running(_) => "running",
        crate::state::RunState::Idle => "idle",
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::raw(format!("Focus: {}", focus_label(&session.focus))),
        Span::raw("  •  "),
        Span::raw(format!("Env: {environment}")),
        Span::raw("  •  "),
        Span::raw(format!("Run: {run_state}")),
        Span::raw(
            "  •  e env  r run  c cancel  1 body  2 headers  3 tests  [/] result  y copy  ? help  q quit",
        ),
    ]));

    frame.render_widget(footer, area);
}

fn render_modal(frame: &mut Frame, session: &SessionState) {
    match &session.modal {
        ModalState::None => {}
        ModalState::Help => {
            let area = centered_rect(frame.area(), 70, 60);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from("Keybindings"),
                    Line::default(),
                    Line::from("↑/k, ↓/j  Move selection in the collection tree"),
                    Line::from("Tab        Cycle focus between tree, details, and output"),
                    Line::from("e          Open the environment picker"),
                    Line::from("r          Run the selected root, folder, or request"),
                    Line::from("c          Cancel the active Bruno run"),
                    Line::from("1/2/3      Show response body, headers, or tests"),
                    Line::from("[/]        Move to previous/next request result"),
                    Line::from("y          Copy the current response tab"),
                    Line::from("?          Toggle this help overlay"),
                    Line::from("Esc        Close modal"),
                    Line::from("q          Quit Brutui"),
                ])
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: false }),
                area,
            );
        }
        ModalState::EnvironmentPicker { highlighted_index } => {
            let area = centered_rect(frame.area(), 60, 60);
            frame.render_widget(Clear, area);
            let items = session
                .environments
                .iter()
                .enumerate()
                .map(|(index, environment)| {
                    let highlighted = index == *highlighted_index;
                    let selected = index == session.selected_environment_index;
                    let prefix = if highlighted { "> " } else { "  " };
                    let selected_suffix = if selected { " (current)" } else { "" };
                    let style = if highlighted {
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Line::styled(
                        format!("{prefix}{}{}", environment.display_name, selected_suffix),
                        style,
                    ))
                })
                .collect::<Vec<_>>();
            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Environment picker • Enter choose • Esc cancel"),
                ),
                area,
            );
        }
    }
}

fn selected_environment_label(session: &SessionState) -> &str {
    session
        .environments
        .get(session.selected_environment_index)
        .map(|environment| environment.display_name.as_str())
        .unwrap_or("unknown")
}

pub fn current_tab_text(session: &SessionState) -> String {
    if let RunState::Running(active) = &session.run {
        return if active.cancellation_requested {
            "Cancellation requested; waiting for Bruno to exit.".to_string()
        } else {
            let mut text = format!("Run in progress: {}", selected_node_label(&active.target));
            if !session.raw_output.is_empty() {
                text.push_str("\n\n");
                text.push_str(&raw_output_text(&session.raw_output));
            }
            text
        };
    }

    let Some(request) = session.selected_response_result() else {
        return match &session.completed_run {
            Some(run) => match &run.status {
                CompletedRunStatus::Cancelled => {
                    "Status: cancelled\nRun was cancelled.".to_string()
                }
                CompletedRunStatus::ToolError(error) => {
                    format!(
                        "Status: tool error\nExecution/tool error: {}",
                        error.message
                    )
                }
                CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) => {
                    let mut text = completed_run_summary_text(run);
                    if report.requests.is_empty() {
                        text.push_str(
                            "\nThe Bruno report did not include request/response results.",
                        );
                    } else {
                        text.push_str("\nNo response selected.");
                    }
                    text
                }
            },
            None => {
                "No Bruno run started yet. Press r on the selected node to start a run.".to_string()
            }
        };
    };

    let mut text = session
        .completed_run
        .as_ref()
        .map(completed_run_summary_text)
        .unwrap_or_default();
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    if matches!(session.response_tab, ResponseTab::Tests) && !session.raw_output.is_empty() {
        text.push_str("Raw output:\n");
        text.push_str(&raw_output_text(&session.raw_output));
        text.push_str("\n\n");
    }
    text.push_str(&match session.response_tab {
        ResponseTab::Body => response_body_text(request),
        ResponseTab::Headers => response_headers_text(request),
        ResponseTab::Tests => response_tests_text(request),
    });
    text
}

fn completed_run_summary_text(run: &crate::state::CompletedRun) -> String {
    let mut lines = vec![
        format!("Status: {}", completed_status_label(&run.status)),
        format!("Exit code: {}", display_exit_code(run.exit_code)),
        format!("Report: {}", run.report_path.display()),
    ];
    if let CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) =
        &run.status
    {
        lines.push(format!(
            "Requests: {} total, {} passed, {} failed",
            report.summary.total_requests,
            report.summary.passed_requests,
            report.summary.failed_requests
        ));
        lines.push(format!(
            "Tests: {} total, {} passed, {} failed",
            report.summary.total_tests, report.summary.passed_tests, report.summary.failed_tests
        ));
        lines.push(format!(
            "Assertions: {} total, {} passed, {} failed",
            report.summary.total_assertions,
            report.summary.passed_assertions,
            report.summary.failed_assertions
        ));
    }
    lines.join("\n")
}

fn response_body_text(request: &crate::report::RequestResult) -> String {
    let mut lines = response_heading(request);
    lines.push(String::new());
    lines.push(
        request
            .response_body
            .clone()
            .unwrap_or_else(|| "No response body captured by bru --reporter-json.".to_string()),
    );
    lines.join("\n")
}

fn response_headers_text(request: &crate::report::RequestResult) -> String {
    let mut lines = response_heading(request);
    if !request.failed_tests.is_empty()
        || !request.failed_assertions.is_empty()
        || !request.errors.is_empty()
    {
        lines.push(String::new());
        lines.push("Failure details:".to_string());
        append_failure_lines(&mut lines, request);
    }
    lines.push(String::new());
    lines.push("Response headers:".to_string());
    if request.response_headers.is_empty() {
        lines.push("  No response headers captured.".to_string());
    } else {
        for header in &request.response_headers {
            lines.push(format!("  {}: {}", header.name, header.value));
        }
    }
    lines.push(String::new());
    lines.push("Request headers:".to_string());
    if request.request_headers.is_empty() {
        lines.push("  No request headers captured.".to_string());
    } else {
        for header in &request.request_headers {
            lines.push(format!("  {}: {}", header.name, header.value));
        }
    }
    lines.join("\n")
}

fn response_tests_text(request: &crate::report::RequestResult) -> String {
    let mut lines = response_heading(request);
    lines.push(String::new());
    if request.tests.is_empty()
        && request.failed_tests.is_empty()
        && request.failed_assertions.is_empty()
        && request.errors.is_empty()
    {
        lines.push("No tests, assertions, or errors captured.".to_string());
    }
    for test in &request.tests {
        lines.push(format!(
            "{} {}{}",
            status_icon(test.status.as_deref()),
            test.name.as_deref().unwrap_or("unnamed test"),
            test.message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for failed_test in &request.failed_tests {
        lines.push(format!(
            "✗ Failed test: {}{}",
            failed_test.name.as_deref().unwrap_or("unnamed"),
            failed_test
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for assertion in &request.failed_assertions {
        lines.push(format!(
            "✗ assertion {}{}",
            assertion.name.as_deref().unwrap_or("unnamed"),
            assertion
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for error in &request.errors {
        lines.push(format!(
            "✗ error [{}]: {}",
            error.code.as_deref().unwrap_or("error"),
            error.message.as_deref().unwrap_or("no details")
        ));
    }
    lines.join("\n")
}

fn append_failure_lines(lines: &mut Vec<String>, request: &crate::report::RequestResult) {
    for failed_test in &request.failed_tests {
        lines.push(format!(
            "Failed test: {}{}",
            failed_test.name.as_deref().unwrap_or("unnamed"),
            failed_test
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for assertion in &request.failed_assertions {
        lines.push(format!(
            "Failed assertion: {}{}",
            assertion.name.as_deref().unwrap_or("unnamed"),
            assertion
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
        if assertion.expected.is_some() || assertion.actual.is_some() {
            lines.push(format!(
                "expected: {} • actual: {}",
                assertion.expected.as_deref().unwrap_or("unknown"),
                assertion.actual.as_deref().unwrap_or("unknown")
            ));
        }
    }
}

fn response_heading(request: &crate::report::RequestResult) -> Vec<String> {
    let label = request
        .url
        .as_deref()
        .or(request.name.as_deref())
        .unwrap_or("Unnamed request");
    let mut lines = vec![
        format!("Request: {label}"),
        format!("{} {label}", request.method.as_deref().unwrap_or("REQUEST")),
    ];
    let status = request
        .status_code
        .map(|code| code.to_string())
        .or_else(|| request.status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut meta = format!("Status: {status}");
    if let Some(text) = &request.status_text {
        meta.push_str(&format!(" {text}"));
    }
    if let Some(duration) = request.duration_ms {
        meta.push_str(&format!(" • {duration} ms"));
    }
    if let Some(size) = request.size_bytes {
        meta.push_str(&format!(" • {size} B"));
    }
    lines.push(meta);
    lines
}

fn status_icon(status: Option<&str>) -> &'static str {
    match status {
        Some("passed" | "pass" | "success" | "ok") => "✓",
        Some("skipped" | "skip") => "-",
        _ => "✗",
    }
}

fn raw_output_text(output: &[OutputLine]) -> String {
    output
        .iter()
        .map(|line| {
            let stream = match line.stream {
                OutputStream::Stdout => "stdout",
                OutputStream::Stderr => "stderr",
            };
            format!("[{stream}] {}", line.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn selected_node_label(node: &CollectionNodeId) -> &'static str {
    match node {
        CollectionNodeId::Root => "Collection root",
        CollectionNodeId::Folder(_) => "Folder",
        CollectionNodeId::Request(_) => "Request",
    }
}

fn completed_status_label(status: &CompletedRunStatus) -> &'static str {
    match status {
        CompletedRunStatus::Success(_) => "success",
        CompletedRunStatus::FailedTests(_) => "completed with failures",
        CompletedRunStatus::Cancelled => "cancelled",
        CompletedRunStatus::ToolError(_) => "tool error",
    }
}

fn display_exit_code(code: Option<i32>) -> String {
    code.map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn focus_label(focus: &FocusPane) -> &'static str {
    match focus {
        FocusPane::CollectionTree => "tree",
        FocusPane::Details => "details",
        FocusPane::Output => "output",
    }
}

fn focused_block(title: &str, focused: bool) -> Block<'static> {
    let title = if focused {
        format!("{title} [focused]")
    } else {
        title.to_string()
    };

    let block = Block::default().borders(Borders::ALL).title(title);
    if focused {
        block.border_style(Style::default().yellow())
    } else {
        block
    }
}

fn centered_rect(area: Rect, horizontal_percent: u16, vertical_percent: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical_percent) / 2),
            Constraint::Percentage(vertical_percent),
            Constraint::Percentage((100 - vertical_percent) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal_percent) / 2),
            Constraint::Percentage(horizontal_percent),
            Constraint::Percentage((100 - horizontal_percent) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        collection::model::{
            Collection, CollectionFormat, CollectionNode, CollectionNodeId, FolderNode,
            RequestNode, RootNode,
        },
        discovery::{DiscoveredCollection, DiscoverySource},
        environments::EnvironmentOption,
        metadata::{MetadataDiagnostic, RequestMetadata},
        state::{AppState, ModalState},
    };

    use super::{FocusPane, UiEventResult, handle_key_event, render};

    #[test]
    fn startup_collection_picker_filters_and_selects_with_keyboard() {
        let mut state = AppState::new();
        state.show_collection_picker(vec![
            discovered_collection("/collections/payments"),
            discovered_collection("/collections/catalog"),
        ]);

        handle_key_event(&mut state, press(KeyCode::Char('c'))).expect("type c");
        handle_key_event(&mut state, press(KeyCode::Char('a'))).expect("type a");
        handle_key_event(&mut state, press(KeyCode::Char('t'))).expect("type t");

        let outcome =
            handle_key_event(&mut state, press(KeyCode::Enter)).expect("choose collection");

        assert_eq!(outcome, UiEventResult::StartupCollectionChosen);
        let crate::state::StartupState::CollectionPicker(picker) = &state.startup else {
            panic!("expected collection picker");
        };
        assert_eq!(picker.query, "cat");
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
    fn focused_tree_keys_drive_selection_and_focus() {
        let mut state = loaded_state();

        let outcome = handle_key_event(&mut state, press(KeyCode::Down)).expect("handle down");
        assert_eq!(outcome, UiEventResult::Continue);
        assert_eq!(
            state.selected_node().expect("selected node").id(),
            CollectionNodeId::Folder(PathBuf::from("users"))
        );

        handle_key_event(&mut state, press(KeyCode::Tab)).expect("handle tab");
        assert_eq!(
            state.session.as_ref().expect("session").focus,
            FocusPane::Details
        );

        handle_key_event(&mut state, press(KeyCode::Down)).expect("ignore down outside tree");
        assert_eq!(
            state.selected_node().expect("selected node").id(),
            CollectionNodeId::Folder(PathBuf::from("users"))
        );
    }

    #[test]
    fn help_overlay_opens_and_closes_with_expected_keys() {
        let mut state = loaded_state();

        handle_key_event(&mut state, press(KeyCode::Char('?'))).expect("open help");
        assert!(matches!(
            state.session.as_ref().expect("session").modal,
            ModalState::Help
        ));

        handle_key_event(&mut state, press(KeyCode::Esc)).expect("close help");
        assert!(matches!(
            state.session.as_ref().expect("session").modal,
            ModalState::None
        ));
    }

    #[test]
    fn quit_keys_request_application_exit() {
        let mut state = loaded_state();

        let outcome = handle_key_event(&mut state, press(KeyCode::Char('q'))).expect("quit");

        assert_eq!(outcome, UiEventResult::Quit);
    }

    #[test]
    fn render_smoke_supports_small_and_normal_terminal_sizes() {
        for (width, height) in [(70, 18), (120, 36)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).expect("test terminal");
            let state = loaded_state();

            terminal
                .draw(|frame| render(frame, &state))
                .expect("render UI");

            let buffer = terminal.backend().buffer();
            let rendered = buffer_string(buffer);
            assert!(rendered.contains("Collection tree"));
            assert!(rendered.contains("Details"));
            assert!(rendered.contains("Output"));
        }
    }

    #[test]
    fn details_pane_renders_request_metadata_tags_and_diagnostics() {
        let backend = TestBackend::new(120, 36);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        let mut state = loaded_state();
        state.session.as_mut().expect("session").selected_node =
            CollectionNodeId::Request(PathBuf::from("users/list.bru"));

        terminal
            .draw(|frame| render(frame, &state))
            .expect("render UI");

        let rendered = buffer_string(terminal.backend().buffer());
        assert!(rendered.contains("Environment: No environment"));
        assert!(rendered.contains("Request name: List users"));
        assert!(rendered.contains("Method: GET"));
        assert!(rendered.contains("URL: https://example.com/users"));
        assert!(rendered.contains("Tags (read-only): smoke, team:platform"));
        assert!(rendered.contains("Metadata notes:"));
        assert!(rendered.contains("best-effort parse kept the request runnable"));
    }

    fn loaded_state() -> AppState {
        let mut state = AppState::new();
        state
            .open_collection(sample_collection(), sample_environments())
            .expect("open collection");
        state
    }

    fn sample_collection() -> Collection {
        Collection {
            root: PathBuf::from("/collections/demo"),
            format: CollectionFormat::ClassicJson,
            nodes: vec![
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
                    metadata: RequestMetadata {
                        name: Some("List users".to_string()),
                        method: Some("GET".to_string()),
                        url: Some("https://example.com/users".to_string()),
                        tags: vec!["smoke".to_string(), "team:platform".to_string()],
                        source_path: Some(PathBuf::from("/collections/demo/users/list.bru")),
                    },
                    metadata_diagnostics: vec![MetadataDiagnostic {
                        message: "best-effort parse kept the request runnable".to_string(),
                    }],
                }),
            ],
        }
    }

    fn discovered_collection(path: &str) -> DiscoveredCollection {
        DiscoveredCollection {
            root: PathBuf::from(path),
            format: CollectionFormat::ClassicJson,
            source: DiscoverySource::ConfiguredDirectory,
        }
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

    fn press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn buffer_string(buffer: &ratatui::buffer::Buffer) -> String {
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
}
