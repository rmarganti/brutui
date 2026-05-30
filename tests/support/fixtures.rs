use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
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
    let workspace = tempdir().expect("temp workspace");
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
