mod support;

use std::fs;

use brutui::config::{
    AppConfig, CONFIG_ENV_VAR, ConfigError, default_config_path, load, load_from_path,
};
use brutui::executable::{BRU_PATH_ENV_VAR, BruExecutableError, BruExecutableSource, resolve};
use support::{FakeBruSpec, install_fake_bru, lock_process_state, set_env_var, temp_workspace};

#[test]
fn missing_default_config_returns_defaults() {
    let _lock = lock_process_state();
    let home = temp_workspace();
    let _home = set_env_var("HOME", home.path());
    let _config_override = set_env_var(CONFIG_ENV_VAR, "");

    let loaded = load().expect("load default config");

    assert_eq!(loaded.config, AppConfig::default());
    assert_eq!(loaded.path, default_config_path());
}

#[test]
fn valid_config_loads_and_normalizes_relative_paths() {
    let workspace = temp_workspace();
    let config_dir = workspace.path().join("config");
    let collections_dir = config_dir.join("collections");
    let bru_dir = config_dir.join("bin");
    let bru_path = bru_dir.join("bru");
    fs::create_dir_all(&collections_dir).expect("create collections dir");
    fs::create_dir_all(&bru_dir).expect("create bru dir");
    fs::write(&bru_path, "#!/bin/sh\nexit 0\n").expect("write fake bru path");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&bru_path).expect("bru metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bru_path, permissions).expect("bru permissions");
    }

    let config_path = config_dir.join("config.toml");
    fs::write(
        &config_path,
        "collection_dirs = [\"collections\"]\nbru_path = \"bin/bru\"\n",
    )
    .expect("write config");

    let config = load_from_path(&config_path).expect("load config from path");

    assert_eq!(
        config.collection_dirs,
        vec![collections_dir.canonicalize().unwrap()]
    );
    assert_eq!(config.bru_path, Some(bru_path.canonicalize().unwrap()));
}

#[test]
fn invalid_toml_reports_the_config_path() {
    let workspace = temp_workspace();
    let config_path = workspace.path().join("broken.toml");
    fs::write(&config_path, "collection_dirs = [\n").expect("write broken toml");

    let error = load_from_path(&config_path).expect_err("config should fail");

    match error {
        ConfigError::Parse { path, .. } => assert_eq!(path, config_path),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_collection_dir_reports_actionable_diagnostics() {
    let workspace = temp_workspace();
    let config_path = workspace.path().join("config.toml");
    fs::write(
        &config_path,
        "collection_dirs = [\"missing-collections\"]\n",
    )
    .expect("write invalid config");

    let error = load_from_path(&config_path).expect_err("config should fail");

    match error {
        ConfigError::InvalidCollectionDir { path } => {
            assert_eq!(path, workspace.path().join("missing-collections"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn invalid_bru_path_reports_actionable_diagnostics() {
    let workspace = temp_workspace();
    let config_path = workspace.path().join("config.toml");
    fs::write(&config_path, "bru_path = \"bin/missing-bru\"\n").expect("write invalid config");

    let error = load_from_path(&config_path).expect_err("config should fail");

    match error {
        ConfigError::InvalidBruPath { path } => {
            assert_eq!(path, workspace.path().join("bin/missing-bru"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn executable_resolution_prefers_environment_override() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let env_bru = install_fake_bru(&workspace, &FakeBruSpec::default());
    let config_bru = workspace.path().join("configured-bru");
    fs::write(&config_bru, "#!/bin/sh\nexit 0\n").expect("write configured bru");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&config_bru)
            .expect("config bru metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&config_bru, permissions).expect("config bru permissions");
    }
    let _env = set_env_var(BRU_PATH_ENV_VAR, &env_bru);
    let _path = set_env_var("PATH", workspace.path());

    let executable = resolve(&AppConfig {
        bru_path: Some(config_bru.clone()),
        ..AppConfig::default()
    })
    .expect("resolve bru from env");

    assert_eq!(executable.source, BruExecutableSource::Environment);
    assert_eq!(executable.path, env_bru);
}

#[test]
fn executable_resolution_prefers_config_over_path_lookup() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let path_dir = workspace.path().join("path-bin");
    fs::create_dir_all(&path_dir).expect("create path dir");
    let _path_bru = install_fake_bru(&workspace, &FakeBruSpec::default());
    let config_bru = workspace.path().join("configured-bru");
    fs::write(&config_bru, "#!/bin/sh\nexit 0\n").expect("write configured bru");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&config_bru)
            .expect("config bru metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&config_bru, permissions).expect("config bru permissions");
    }
    fs::copy(workspace.path().join("bru"), path_dir.join("bru")).expect("copy path bru");
    let _env = set_env_var(BRU_PATH_ENV_VAR, "");
    let _path = set_env_var("PATH", &path_dir);

    let executable = resolve(&AppConfig {
        bru_path: Some(config_bru.clone()),
        ..AppConfig::default()
    })
    .expect("resolve bru from config");

    assert_eq!(executable.source, BruExecutableSource::Config);
    assert_eq!(executable.path, config_bru);
}

#[test]
fn executable_resolution_falls_back_to_path() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).expect("create bin dir");
    let installed = install_fake_bru(&workspace, &FakeBruSpec::default());
    fs::copy(&installed, path_dir.join("bru")).expect("copy bru into path dir");
    let _env = set_env_var(BRU_PATH_ENV_VAR, "");
    let _path = set_env_var("PATH", &path_dir);

    let executable = resolve(&AppConfig::default()).expect("resolve bru from path");

    assert_eq!(executable.source, BruExecutableSource::Path);
    assert_eq!(executable.path, path_dir.join("bru"));
}

#[test]
fn missing_bru_returns_actionable_error() {
    let _lock = lock_process_state();
    let workspace = temp_workspace();
    let _env = set_env_var(BRU_PATH_ENV_VAR, "");
    let _path = set_env_var("PATH", workspace.path());

    let error = resolve(&AppConfig::default()).expect_err("resolution should fail");

    match error {
        BruExecutableError::NotFound { env_var } => assert_eq!(env_var, BRU_PATH_ENV_VAR),
        other => panic!("unexpected error: {other:?}"),
    }
}
