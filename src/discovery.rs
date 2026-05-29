use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoverySource {
    ExplicitPath,
    CurrentWorkingDirectory,
    ConfiguredDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCollection {
    pub root: PathBuf,
    pub source: DiscoverySource,
}
