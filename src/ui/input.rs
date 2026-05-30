use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::state::{AppState, FocusPane, ModalState, StartupState, StateError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiEventResult {
    Continue,
    Quit,
    StartupCollectionChosen,
}

pub fn handle_key_event(
    state: &mut AppState,
    event: KeyEvent,
) -> Result<UiEventResult, StateError> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return Ok(UiEventResult::Continue);
    }

    let Some(session) = state.session() else {
        return handle_startup_keys(state, event);
    };

    match session.modal() {
        ModalState::Help => handle_help_modal_keys(state, event),
        ModalState::EnvironmentPicker { .. } => handle_environment_picker_keys(state, event),
        ModalState::None => handle_session_keys(state, event),
    }
}

fn handle_startup_keys(state: &mut AppState, event: KeyEvent) -> Result<UiEventResult, StateError> {
    match state.startup() {
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
                if let StartupState::CollectionPicker(picker) = state.startup() {
                    let mut query = picker.query().to_string();
                    query.pop();
                    state.set_collection_picker_query(query);
                }
                Ok(UiEventResult::Continue)
            }
            KeyCode::Enter => {
                if let StartupState::CollectionPicker(picker) = state.startup() {
                    if picker.selected_collection().is_some() {
                        return Ok(UiEventResult::StartupCollectionChosen);
                    }
                }
                Ok(UiEventResult::Continue)
            }
            KeyCode::Char(character) => {
                if let StartupState::CollectionPicker(picker) = state.startup() {
                    let mut query = picker.query().to_string();
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
        .session()
        .map(|session| session.focus().clone())
        .ok_or(StateError::NoCollectionLoaded)?;

    match event.code {
        KeyCode::Char('q') | KeyCode::Esc => return Ok(UiEventResult::Quit),
        KeyCode::Char('?') => state.open_help()?,
        KeyCode::Tab => state.cycle_focus_forward()?,
        KeyCode::BackTab => state.cycle_focus_backward()?,
        KeyCode::Up | KeyCode::Char('k') if focus == FocusPane::CollectionTree => {
            state.move_selection_previous()?
        }
        KeyCode::Down | KeyCode::Char('j') if focus == FocusPane::CollectionTree => {
            state.move_selection_next()?
        }
        KeyCode::Up | KeyCode::Char('k') if focus == FocusPane::Output => {
            state.scroll_output_by(-1)?
        }
        KeyCode::Down | KeyCode::Char('j') if focus == FocusPane::Output => {
            state.scroll_output_by(1)?
        }
        KeyCode::PageUp if focus == FocusPane::Output => {
            state.scroll_output_by(-output_half_page_height(state))?
        }
        KeyCode::PageDown if focus == FocusPane::Output => {
            state.scroll_output_by(output_half_page_height(state))?
        }
        KeyCode::Char('u')
            if focus == FocusPane::Output && event.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            state.scroll_output_by(-output_half_page_height(state))?
        }
        KeyCode::Char('d')
            if focus == FocusPane::Output && event.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            state.scroll_output_by(output_half_page_height(state))?
        }
        KeyCode::Home | KeyCode::Char('g') if focus == FocusPane::Output => {
            state.scroll_output_to_top()?
        }
        KeyCode::End | KeyCode::Char('G') if focus == FocusPane::Output => {
            state.scroll_output_to_bottom()?
        }
        _ => {}
    }

    Ok(UiEventResult::Continue)
}

fn output_half_page_height(state: &AppState) -> isize {
    state
        .session()
        .map(|session| (session.scroll().output.viewport_height / 2).max(1) as isize)
        .unwrap_or(1)
}
