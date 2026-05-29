use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use thiserror::Error;

use crate::collection::model::{Collection, CollectionNodeId};
use crate::environments::EnvironmentOption;

static REPORT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub report_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvent {
    Stdout(String),
    Stderr(String),
    Finished(RunCompletion),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCompletion {
    pub exit_code: i32,
    pub report_path: PathBuf,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RunCommandBuildError {
    #[error("collection node not found for selection {0:?}")]
    UnknownSelection(CollectionNodeId),
}

pub fn build_run_command(
    program: impl Into<PathBuf>,
    collection: &Collection,
    target: &CollectionNodeId,
    environment: &EnvironmentOption,
) -> Result<RunCommand, RunCommandBuildError> {
    let target_path = resolve_target_path(collection, target)?;
    let report_path = next_report_path();
    let mut args = vec!["run".to_string(), path_arg(&target_path)];

    if matches!(target, CollectionNodeId::Root | CollectionNodeId::Folder(_)) {
        args.push("-r".to_string());
    }

    if let Some(env) = &environment.cli_value {
        args.push("--env".to_string());
        args.push(env.clone());
    }

    args.push("--reporter-json".to_string());
    args.push(path_arg(&report_path));

    Ok(RunCommand {
        program: program.into(),
        args,
        report_path,
    })
}

fn resolve_target_path(
    collection: &Collection,
    target: &CollectionNodeId,
) -> Result<PathBuf, RunCommandBuildError> {
    match target {
        CollectionNodeId::Root => Ok(collection.root.clone()),
        CollectionNodeId::Folder(_) | CollectionNodeId::Request(_) => collection
            .nodes
            .iter()
            .find(|node| node.id() == *target)
            .map(|node| node.path().to_path_buf())
            .ok_or_else(|| RunCommandBuildError::UnknownSelection(target.clone())),
    }
}

fn next_report_path() -> PathBuf {
    let unique = REPORT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    std::env::temp_dir().join(format!(
        "brutui-report-{}-{}-{}.json",
        std::process::id(),
        timestamp,
        unique
    ))
}

fn path_arg(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::collection::model::{Collection, CollectionFormat, CollectionNode, RootNode};

    use super::{RunCommandBuildError, build_run_command};

    #[test]
    fn missing_non_root_selection_is_reported() {
        let collection = Collection {
            root: PathBuf::from("/tmp/demo"),
            format: CollectionFormat::ClassicJson,
            nodes: vec![CollectionNode::Root(RootNode {
                path: PathBuf::from("/tmp/demo"),
                display_name: "demo".to_string(),
            })],
        };

        let error = build_run_command(
            "/usr/bin/bru",
            &collection,
            &crate::collection::model::CollectionNodeId::Request(PathBuf::from("missing.bru")),
            &crate::environments::EnvironmentOption::no_environment(),
        )
        .expect_err("missing selection should error");

        assert_eq!(
            error,
            RunCommandBuildError::UnknownSelection(
                crate::collection::model::CollectionNodeId::Request(PathBuf::from("missing.bru"))
            )
        );
    }
}
