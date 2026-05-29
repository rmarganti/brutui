#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

pub fn temp_workspace() -> TempDir {
    tempdir().expect("temp workspace")
}

pub struct CurrentDirGuard {
    previous: PathBuf,
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).expect("restore current dir");
    }
}

pub fn set_current_dir(path: &Path) -> CurrentDirGuard {
    let previous = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(path).expect("set current dir");
    CurrentDirGuard { previous }
}

pub struct EnvVarGuard {
    key: String,
    previous: Option<OsString>,
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => {
                // SAFETY: Test helper restores the original process env value before teardown.
                unsafe { std::env::set_var(&self.key, value) }
            }
            None => {
                // SAFETY: Test helper removes only the key it created during the test.
                unsafe { std::env::remove_var(&self.key) }
            }
        }
    }
}

pub fn set_env_var(key: impl Into<String>, value: impl AsRef<std::ffi::OsStr>) -> EnvVarGuard {
    let key = key.into();
    let previous = std::env::var_os(&key);
    // SAFETY: Tests use this helper in a scoped manner and restore the original value on drop.
    unsafe { std::env::set_var(&key, value) };
    EnvVarGuard { key, previous }
}

#[derive(Debug)]
pub struct TempCollection {
    _workspace: TempDir,
    pub root: PathBuf,
    original_dirs: BTreeSet<PathBuf>,
    original_files: BTreeMap<PathBuf, Vec<u8>>,
}

impl TempCollection {
    pub fn assert_unchanged(&self) {
        let (dirs, files) = snapshot_paths(&self.root);
        assert_eq!(dirs, self.original_dirs, "collection directories changed");
        assert_eq!(files, self.original_files, "collection files changed");
    }
}

pub fn copy_fixture_collection(format_dir: &str, collection_name: &str) -> TempCollection {
    let source = fixture_root().join(format_dir).join(collection_name);
    let workspace = temp_workspace();
    let root = workspace.path().join(collection_name);
    copy_dir_all(&source, &root);
    let (original_dirs, original_files) = snapshot_paths(&root);

    TempCollection {
        _workspace: workspace,
        root,
        original_dirs,
        original_files,
    }
}

pub fn report_path(workspace: &TempDir, file_name: &str) -> PathBuf {
    workspace.path().join(file_name)
}

#[derive(Debug, Clone, Default)]
pub struct FakeBruSpec {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
    pub exit_code: i32,
    pub report_json: Option<Value>,
}

pub fn install_fake_bru(workspace: &TempDir, spec: &FakeBruSpec) -> PathBuf {
    let script_path = workspace.path().join("bru");
    let report_block = spec
        .report_json
        .as_ref()
        .map(|report| {
            format!(
                "cat <<'__BRUTUI_REPORT__' > \"$report_path\"\n{}\n__BRUTUI_REPORT__",
                serde_json::to_string_pretty(report).expect("serialize report")
            )
        })
        .unwrap_or_default();
    let stdout_block = shell_echo_block("stdout", &spec.stdout);
    let stderr_block = shell_echo_block("stderr", &spec.stderr);
    let script = format!(
        r#"#!/bin/sh
set -eu

report_path=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "report" ]; then
    report_path="$arg"
    prev=""
    continue
  fi

  case "$arg" in
    --report-file|--report-path|--output)
      prev="report"
      ;;
  esac
done

{stdout_block}
{stderr_block}

if [ -n "$report_path" ] && [ -n "{report_guard}" ]; then
  mkdir -p "$(dirname "$report_path")"
  {report_block}
fi

exit {exit_code}
"#,
        stdout_block = stdout_block,
        stderr_block = stderr_block,
        report_guard = if spec.report_json.is_some() {
            "write"
        } else {
            ""
        },
        report_block = report_block,
        exit_code = spec.exit_code,
    );

    fs::write(&script_path, script).expect("write fake bru");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&script_path)
            .expect("fake bru metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script_path, permissions).expect("fake bru permissions");
    }

    script_path
}

pub fn run_command(program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .expect("run command")
}

fn shell_echo_block(stream: &str, lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| {
            format!(
                "printf '%s\\n' '{}' {}",
                shell_single_quote(line),
                if stream == "stderr" { "1>&2" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn shell_single_quote(value: &str) -> String {
    value.replace('\'', "'\"'\"'")
}

fn snapshot_paths(root: &Path) -> (BTreeSet<PathBuf>, BTreeMap<PathBuf, Vec<u8>>) {
    let mut dirs = BTreeSet::new();
    let mut files = BTreeMap::new();

    for entry in WalkDir::new(root).sort_by_file_name() {
        let entry = entry.expect("walk fixture");
        let path = entry.path();
        if path == root {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("relative path")
            .to_path_buf();
        if entry.file_type().is_dir() {
            dirs.insert(relative);
        } else if entry.file_type().is_file() {
            files.insert(relative, fs::read(path).expect("read snapshot file"));
        }
    }

    (dirs, files)
}

fn copy_dir_all(source: &Path, destination: &Path) {
    for entry in WalkDir::new(source).sort_by_file_name() {
        let entry = entry.expect("walk source fixture");
        let relative = entry
            .path()
            .strip_prefix(source)
            .expect("relative source path");
        let target = destination.join(relative);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).expect("create fixture dir");
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("create fixture parent");
            }
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}
