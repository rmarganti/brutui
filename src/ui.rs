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
        AppState, CompletedRunStatus, ModalState, OutputLine, OutputStream, ResultView,
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
        return Ok(match event.code {
            KeyCode::Char('q') | KeyCode::Esc => UiEventResult::Quit,
            _ => UiEventResult::Continue,
        });
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
    let block = Block::default().borders(Borders::ALL).title("Brutui");
    let message = match startup {
        StartupState::Discovering => "Discovering Bruno collections...",
        StartupState::Ready => "Preparing session...",
        StartupState::SetupMessage { message } => message,
        StartupState::CollectionPicker(picker) => {
            let selected = picker
                .selected_collection()
                .map(|collection| collection.root.display().to_string())
                .unwrap_or_else(|| "No matching collections".to_string());
            return frame.render_widget(
                Paragraph::new(format!(
                    "Collection picker placeholder\n\nQuery: {}\nSelected: {}",
                    picker.query, selected
                ))
                .block(block)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false }),
                frame.area(),
            );
        }
    };

    frame.render_widget(
        Paragraph::new(message)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        frame.area(),
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
        .constraints([Constraint::Length(9), Constraint::Min(0)])
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
        ];

        if let Some(relative_path) = node.relative_path() {
            lines.push(Line::from(format!("Relative: {}", relative_path.display())));
        }

        lines.push(Line::default());
        lines.push(Line::from("Request details panel ready."));
        lines.push(Line::from(
            "Metadata and environment picker land in follow-on ishes.",
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
    let title = match session.result_view {
        ResultView::Summary => "Output & results (summary)",
        ResultView::Failures => "Output & results (failures)",
        ResultView::RawOutput => "Output & results (raw output)",
    };

    let mut lines = render_output_lines(&session.raw_output);
    if lines.is_empty() {
        lines = match &session.completed_run {
            Some(run) => vec![
                Line::from(format!(
                    "Last run target: {}",
                    selected_node_label(&run.target)
                )),
                Line::from(format!("Status: {}", completed_status_label(&run.status))),
                Line::from(format!("Exit code: {}", display_exit_code(run.exit_code))),
                Line::from(format!("Report: {}", run.report_path.display())),
            ],
            None => vec![
                Line::from("No Bruno run started yet."),
                Line::from("Run/output wiring lands in follow-on ishes."),
            ],
        };
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(focused_block(title, session.focus == FocusPane::Output))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_footer(frame: &mut Frame, area: Rect, session: &SessionState) {
    let environment = session
        .environments
        .get(session.selected_environment_index)
        .map(|environment| environment.display_name.as_str())
        .unwrap_or("unknown");
    let run_state = if matches!(session.run, crate::state::RunState::Running(_)) {
        "running"
    } else {
        "idle"
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::raw(format!("Focus: {}", focus_label(&session.focus))),
        Span::raw("  •  "),
        Span::raw(format!("Env: {environment}")),
        Span::raw("  •  "),
        Span::raw(format!("Run: {run_state}")),
        Span::raw("  •  ↑/↓ move  Tab focus  ? help  q quit"),
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
                    let prefix = if index == *highlighted_index {
                        "> "
                    } else {
                        "  "
                    };
                    ListItem::new(format!("{prefix}{}", environment.display_name))
                })
                .collect::<Vec<_>>();
            frame.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Environment picker"),
                ),
                area,
            );
        }
    }
}

fn render_output_lines(output: &[OutputLine]) -> Vec<Line<'static>> {
    output
        .iter()
        .map(|line| {
            let stream = match line.stream {
                OutputStream::Stdout => "stdout",
                OutputStream::Stderr => "stderr",
            };
            Line::from(format!("[{stream}] {}", line.text))
        })
        .collect()
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
        environments::EnvironmentOption,
        metadata::RequestMetadata,
        state::{AppState, ModalState},
    };

    use super::{FocusPane, UiEventResult, handle_key_event, render};

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
                    metadata: RequestMetadata::default(),
                    metadata_diagnostics: Vec::new(),
                }),
            ],
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
