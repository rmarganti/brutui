use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BruExecutableSource {
    Environment,
    Config,
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BruExecutable {
    pub path: PathBuf,
    pub source: BruExecutableSource,
}
