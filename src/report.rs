use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RunReport {
    #[serde(default)]
    pub summary: ReportSummary,
    #[serde(default)]
    pub requests: Vec<RequestResult>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReportSummary {
    pub total_requests: u64,
    pub passed_requests: u64,
    pub failed_requests: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestResult {
    pub name: Option<String>,
    pub status: Option<String>,
}
