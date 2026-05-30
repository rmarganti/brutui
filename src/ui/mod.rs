mod input;
mod render;
mod terminal;
mod text;

pub use input::{UiEventResult, handle_key_event};
pub use render::render;
pub use terminal::{AppTerminal, TerminalSession};
pub use text::current_tab_text;

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
        state::{AppState, FocusPane, ModalState},
    };

    use super::{UiEventResult, handle_key_event, render};

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
        let crate::state::StartupState::CollectionPicker(picker) = state.startup() else {
            panic!("expected collection picker");
        };
        assert_eq!(picker.query(), "cat");
        assert_eq!(picker.filtered_indices().len(), 1);
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
            state.session().expect("session").focus(),
            &FocusPane::Details
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
            state.session().expect("session").modal(),
            ModalState::Help
        ));

        handle_key_event(&mut state, press(KeyCode::Esc)).expect("close help");
        assert!(matches!(
            state.session().expect("session").modal(),
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
            let mut state = loaded_state();

            terminal
                .draw(|frame| render(frame, &mut state))
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
        state
            .select_node(CollectionNodeId::Request(PathBuf::from("users/list.bru")))
            .expect("select request node");

        terminal
            .draw(|frame| render(frame, &mut state))
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
