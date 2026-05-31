use std::path::{Path, PathBuf};

use crate::collection::model::{Collection, CollectionNodeId};
use crate::environments::EnvironmentOption;

use super::report_path::next_report_path;
use super::{RunCommand, RunCommandBuildError};

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
        working_dir: collection.root().to_path_buf(),
        report_path,
    })
}

fn resolve_target_path(
    collection: &Collection,
    target: &CollectionNodeId,
) -> Result<PathBuf, RunCommandBuildError> {
    match target {
        CollectionNodeId::Root => Ok(collection.root().to_path_buf()),
        CollectionNodeId::Folder(_) | CollectionNodeId::Request(_) => collection
            .node(target)
            .map(|node| node.path().to_path_buf())
            .ok_or_else(|| RunCommandBuildError::UnknownSelection(target.clone())),
    }
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::collection::model::{Collection, CollectionFormat, CollectionNode, RootNode};
    use crate::environments::EnvironmentOption;

    use super::build_run_command;
    use crate::runner::RunCommandBuildError;

    #[test]
    fn missing_non_root_selection_is_reported() {
        let collection = Collection::new(
            PathBuf::from("/tmp/demo"),
            CollectionFormat::ClassicJson,
            vec![CollectionNode::Root(RootNode {
                path: PathBuf::from("/tmp/demo"),
                display_name: "demo".to_string(),
            })],
        );

        let error = build_run_command(
            "/usr/bin/bru",
            &collection,
            &crate::collection::model::CollectionNodeId::Request(PathBuf::from("missing.bru")),
            &EnvironmentOption::no_environment(),
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
