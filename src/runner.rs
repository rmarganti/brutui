use std::path::PathBuf;

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
