use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RequestMetadata {
    pub name: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub tags: Vec<String>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataDiagnostic {
    pub message: String,
}
