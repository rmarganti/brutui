use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use crate::{
    collection::model::{CollectionNode, CollectionNodeId},
    state::{
        AppState, FocusPane, ModalState, OutputScrollMeasurements, ResponseTab, SessionState,
        StartupState, ViewMeasurements,
    },
};

use super::{text::current_tab_text, theme::Theme};

pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) -> ViewMeasurements {
    match state.session() {
        Some(session) => render_session(frame, session, theme),
        None => {
            render_startup(frame, state.startup(), theme);
            ViewMeasurements::default()
        }
    }
}

fn render_startup(frame: &mut Frame, startup: &StartupState, theme: &Theme) {
    match startup {
        StartupState::CollectionPicker(picker) => render_collection_picker(frame, picker, theme),
        StartupState::Discovering => {
            render_startup_message(frame, "Discovering Bruno collections...", theme)
        }
        StartupState::Ready => render_startup_message(frame, "Preparing session...", theme),
        StartupState::SetupMessage { message } => render_startup_message(frame, message, theme),
    }
}

fn render_startup_message(frame: &mut Frame, message: &str, theme: &Theme) {
    frame.render_widget(
        Paragraph::new(message)
            .block(themed_block("Brutui", theme))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        frame.area(),
    );
}

fn render_collection_picker(
    frame: &mut Frame,
    picker: &crate::state::CollectionPickerState,
    theme: &Theme,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(format!("Search: {}", picker.query()))
            .block(themed_block("Brutui", theme))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );

    let items = if picker.filtered_indices().is_empty() {
        vec![ListItem::new("No matching collections")]
    } else {
        picker
            .filtered_indices()
            .iter()
            .enumerate()
            .map(|(filtered_index, collection_index)| {
                let collection = &picker.collections()[*collection_index];
                let prefix = if filtered_index == picker.selected_filtered_index() {
                    "> "
                } else {
                    "  "
                };
                let style = if filtered_index == picker.selected_filtered_index() {
                    theme.focused_selected_item
                } else {
                    theme.base
                };
                ListItem::new(Line::styled(
                    format!("{prefix}{}", collection.root.display()),
                    style,
                ))
            })
            .collect::<Vec<_>>()
    };

    frame.render_widget(
        List::new(items).block(themed_block("Collection picker", theme)),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new("Type to filter • Enter to open • ↑/↓ or j/k to move • q or Esc to quit")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.panel_border),
            )
            .wrap(Wrap { trim: false }),
        chunks[2],
    );
}

fn render_session(frame: &mut Frame, session: &SessionState, theme: &Theme) -> ViewMeasurements {
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

    render_collection_tree(frame, content_chunks[0], session, theme);
    render_details_pane(frame, right_chunks[0], session, theme);
    let output = render_output_pane(frame, right_chunks[1], session, theme);
    render_footer(frame, root_chunks[1], session);
    render_modal(frame, session, theme);

    ViewMeasurements {
        output: Some(output),
    }
}

fn render_collection_tree(frame: &mut Frame, area: Rect, session: &SessionState, theme: &Theme) {
    let selected_node = session.selected_node_id();
    let items = session
        .collection()
        .visible_nodes()
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
            let style = if selected && session.focus() == &FocusPane::CollectionTree {
                theme.focused_selected_item
            } else if selected {
                theme.selected_item
            } else {
                theme.base
            };

            ListItem::new(Line::styled(
                format!("{prefix}{}", node.display_name()),
                style,
            ))
        })
        .collect::<Vec<_>>();

    let list = List::new(items).block(focused_block(
        "Collection tree",
        session.focus() == &FocusPane::CollectionTree,
        theme,
    ));

    frame.render_widget(list, area);
}

fn render_details_pane(frame: &mut Frame, area: Rect, session: &SessionState, theme: &Theme) {
    let lines = if let Some(node) = session.selected_node() {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                selected_node_label(&node.id()),
                theme.emphasized_text,
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
                session.focus() == &FocusPane::Details,
                theme,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_output_pane(
    frame: &mut Frame,
    area: Rect,
    session: &SessionState,
    theme: &Theme,
) -> OutputScrollMeasurements {
    let result_count = session.response_result_count();
    let result_position = if result_count == 0 {
        "no result".to_string()
    } else {
        format!(
            "result {}/{}",
            session.selected_result_index() + 1,
            result_count
        )
    };
    let tab = match session.response_tab() {
        ResponseTab::Body => "Body",
        ResponseTab::Headers => "Headers",
        ResponseTab::Tests => "Tests",
    };
    let title = format!("Output: Response viewer ({tab}, {result_position})");
    let text = current_tab_text(session);
    let lines = text
        .lines()
        .map(|line| Line::from(line.to_string()))
        .collect::<Vec<_>>();
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let content_height = wrapped_visual_line_count(&text, inner.width);
    let measurements = OutputScrollMeasurements {
        viewport_height: inner.height as usize,
        content_height,
    };
    let vertical_offset = session
        .scroll()
        .output
        .vertical_offset
        .min(u16::MAX as usize) as u16;

    frame.render_widget(
        Paragraph::new(lines)
            .block(focused_block(
                &title,
                session.focus() == &FocusPane::Output,
                theme,
            ))
            .wrap(Wrap { trim: false })
            .scroll((vertical_offset, 0)),
        area,
    );

    measurements
}

fn render_footer(frame: &mut Frame, area: Rect, session: &SessionState) {
    let environment = selected_environment_label(session);
    let run_state = match session.run_state() {
        crate::state::RunState::Running(active) if active.cancellation_requested => "cancelling",
        crate::state::RunState::Running(_) => "running",
        crate::state::RunState::Idle => "idle",
    };

    let footer = Paragraph::new(Line::from(vec![
        Span::raw(format!("Env: {environment}")),
        Span::raw("  •  "),
        Span::raw(format!("Run: {run_state}")),
        Span::raw(
            "  •  e env  r run  c cancel  1 body  2 headers  3 tests  [/] result  y copy  ? help  q quit",
        ),
    ]));

    frame.render_widget(footer, area);
}

fn render_modal(frame: &mut Frame, session: &SessionState, theme: &Theme) {
    match session.modal() {
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
                .block(themed_block("Help", theme))
                .wrap(Wrap { trim: false }),
                area,
            );
        }
        ModalState::EnvironmentPicker { highlighted_index } => {
            let area = centered_rect(frame.area(), 60, 60);
            frame.render_widget(Clear, area);
            let items = session
                .environments()
                .iter()
                .enumerate()
                .map(|(index, environment)| {
                    let highlighted = index == *highlighted_index;
                    let selected = index == session.selected_environment_index();
                    let prefix = if highlighted { "> " } else { "  " };
                    let selected_suffix = if selected { " (current)" } else { "" };
                    let style = if highlighted {
                        theme.focused_selected_item
                    } else {
                        theme.base
                    };
                    ListItem::new(Line::styled(
                        format!("{prefix}{}{}", environment.display_name, selected_suffix),
                        style,
                    ))
                })
                .collect::<Vec<_>>();
            frame.render_widget(
                List::new(items).block(themed_block(
                    "Environment picker • Enter choose • Esc cancel",
                    theme,
                )),
                area,
            );
        }
    }
}

fn wrapped_visual_line_count(text: &str, width: u16) -> usize {
    let width = width as usize;
    if width == 0 {
        return 0;
    }

    let line_count = text.lines().count();
    if line_count == 0 {
        return 1;
    }

    text.lines()
        .map(|line| {
            let width_chars = line.chars().count();
            width_chars.div_ceil(width).max(1)
        })
        .sum()
}

fn selected_environment_label(session: &SessionState) -> &str {
    session
        .selected_environment()
        .map(|environment| environment.display_name.as_str())
        .unwrap_or("unknown")
}

fn selected_node_label(node: &CollectionNodeId) -> &'static str {
    match node {
        CollectionNodeId::Root => "Collection root",
        CollectionNodeId::Folder(_) => "Folder",
        CollectionNodeId::Request(_) => "Request",
    }
}

fn themed_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(theme.panel_border)
}

fn focused_block<'a>(title: &'a str, focused: bool, theme: &Theme) -> Block<'a> {
    let block = themed_block(title, theme);
    if focused {
        block.border_style(theme.focused_panel_border)
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
