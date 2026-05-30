use tempfile::{TempDir, tempdir};

pub fn temp_workspace() -> TempDir {
    tempdir().expect("temp workspace")
}
