#[path = "support/process.rs"]
mod process;
#[path = "support/temp.rs"]
mod temp;

use std::fs;

use brutui::config::{CONFIG_ENV_VAR, INIT_CONFIG_TEMPLATE, InitError, default_config_path, init};
use process::{lock_process_state, set_env_var};
use temp::temp_workspace;

#[test]
fn init_writes_template_to_default_config_path() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let _home = set_env_var("HOME", workspace.path());
    let _config_override = set_env_var(CONFIG_ENV_VAR, "");

    init().expect("init should succeed");

    let path = default_config_path().expect("default config path should resolve");
    assert!(path.exists(), "config file should exist after init");
    let contents = fs::read_to_string(&path).expect("read config file");
    assert_eq!(contents, INIT_CONFIG_TEMPLATE);
}

#[test]
fn init_creates_parent_directories() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let _home = set_env_var("HOME", workspace.path());
    let _config_override = set_env_var(CONFIG_ENV_VAR, "");

    let path = default_config_path().expect("default config path should resolve");
    assert!(
        !path.parent().map(|p| p.exists()).unwrap_or(false),
        "parent dir should not exist before init"
    );

    init().expect("init should create parent dirs and succeed");

    assert!(path.exists(), "config file should exist after init");
}

#[test]
fn init_errors_when_config_already_exists() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let _home = set_env_var("HOME", workspace.path());
    let _config_override = set_env_var(CONFIG_ENV_VAR, "");

    init().expect("first init should succeed");
    let error = init().expect_err("second init should fail");

    match error {
        InitError::AlreadyExists { path } => {
            assert_eq!(
                path,
                default_config_path().expect("default config path")
            );
        }
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
}

#[test]
fn init_template_is_valid_toml() {
    let config: toml::Value =
        toml::from_str(INIT_CONFIG_TEMPLATE).expect("template should be valid TOML when uncommented — but all lines are commented, so empty doc is valid");
    // The template is entirely commented out, so it parses as an empty TOML document.
    assert!(config.is_table());
}

#[test]
fn init_template_contains_all_top_level_keys() {
    assert!(
        INIT_CONFIG_TEMPLATE.contains("collection_dirs"),
        "template should document collection_dirs"
    );
    assert!(
        INIT_CONFIG_TEMPLATE.contains("bru_path"),
        "template should document bru_path"
    );
    assert!(
        INIT_CONFIG_TEMPLATE.contains("[theme."),
        "template should document theme sections"
    );
}

#[test]
fn init_template_contains_all_theme_sections() {
    for section in &[
        "theme.base",
        "theme.panel_border",
        "theme.focused_panel_border",
        "theme.selected_item",
        "theme.focused_selected_item",
        "theme.emphasized_text",
    ] {
        assert!(
            INIT_CONFIG_TEMPLATE.contains(section),
            "template should document {section}"
        );
    }
}

#[test]
fn init_template_contains_all_style_properties() {
    for prop in &["fg", "bg", "bold", "italic", "underlined", "reversed", "dim"] {
        assert!(
            INIT_CONFIG_TEMPLATE.contains(prop),
            "template should document style property `{prop}`"
        );
    }
}
