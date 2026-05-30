use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use tempfile::TempDir;

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
    --report-file|--report-path|--output|--reporter-json)
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
