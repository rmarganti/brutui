use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RunReport {
    pub summary: ReportSummary,
    #[serde(default)]
    pub requests: Vec<RequestResult>,
    #[serde(default)]
    pub diagnostics: Vec<ParseDiagnostic>,
    pub source_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReportSummary {
    pub total_requests: u64,
    pub passed_requests: u64,
    pub failed_requests: u64,
    pub total_tests: u64,
    pub passed_tests: u64,
    pub failed_tests: u64,
    pub total_assertions: u64,
    pub passed_assertions: u64,
    pub failed_assertions: u64,
    pub error_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RequestResult {
    pub name: Option<String>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub failed_tests: Vec<FailedTest>,
    #[serde(default)]
    pub failed_assertions: Vec<FailedAssertion>,
    #[serde(default)]
    pub errors: Vec<ReportError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FailedTest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FailedAssertion {
    pub name: Option<String>,
    pub message: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReportError {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseDiagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ReportParseError {
    #[error("failed to read Bruno report at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse Bruno report JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("Bruno report root must be a JSON object")]
    InvalidRoot,
    #[error("Bruno report is missing required semantics: {0}")]
    MissingSemantics(&'static str),
    #[error("Bruno report field `{field}` must be {expected}")]
    InvalidFieldType {
        field: &'static str,
        expected: &'static str,
    },
}

pub fn parse_report_file(path: impl AsRef<Path>) -> Result<RunReport, ReportParseError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| ReportParseError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    parse_report_str(&content, Some(path.to_path_buf()))
}

pub fn parse_report_str(
    input: &str,
    source_path: Option<PathBuf>,
) -> Result<RunReport, ReportParseError> {
    let value: Value = serde_json::from_str(input)?;
    let root = value.as_object().ok_or(ReportParseError::InvalidRoot)?;

    let summary = parse_summary(root.get("summary"), root.get("requests"))?;
    let requests = parse_requests(root.get("requests"))?;
    let mut diagnostics = Vec::new();

    if !root.contains_key("summary") {
        diagnostics.push(ParseDiagnostic {
            path: "summary".to_string(),
            message: "summary missing; counts were derived from request results when possible"
                .to_string(),
        });
    }

    if !root.contains_key("requests") {
        diagnostics.push(ParseDiagnostic {
            path: "requests".to_string(),
            message: "requests missing; failure details unavailable".to_string(),
        });
    }

    if summary.total_requests == 0 && requests.is_empty() {
        return Err(ReportParseError::MissingSemantics(
            "expected summary counts or request results",
        ));
    }

    Ok(RunReport {
        summary,
        requests,
        diagnostics,
        source_path,
    })
}

fn parse_summary(
    summary_value: Option<&Value>,
    requests_value: Option<&Value>,
) -> Result<ReportSummary, ReportParseError> {
    let mut summary = ReportSummary::default();

    if let Some(value) = summary_value {
        let map = value
            .as_object()
            .ok_or(ReportParseError::InvalidFieldType {
                field: "summary",
                expected: "an object",
            })?;

        summary.total_requests =
            first_u64(map, &["total_requests", "totalRequests", "requests"]).unwrap_or_default();
        summary.passed_requests =
            first_u64(map, &["passed_requests", "passedRequests"]).unwrap_or_default();
        summary.failed_requests =
            first_u64(map, &["failed_requests", "failedRequests"]).unwrap_or_default();
        summary.total_tests =
            first_u64(map, &["total_tests", "totalTests", "tests"]).unwrap_or_default();
        summary.passed_tests = first_u64(map, &["passed_tests", "passedTests"]).unwrap_or_default();
        summary.failed_tests = first_u64(map, &["failed_tests", "failedTests"]).unwrap_or_default();
        summary.total_assertions =
            first_u64(map, &["total_assertions", "totalAssertions"]).unwrap_or_default();
        summary.passed_assertions =
            first_u64(map, &["passed_assertions", "passedAssertions"]).unwrap_or_default();
        summary.failed_assertions =
            first_u64(map, &["failed_assertions", "failedAssertions"]).unwrap_or_default();
        summary.error_count =
            first_u64(map, &["error_count", "errorCount", "errors"]).unwrap_or_default();
    }

    if let Some(requests) = requests_value {
        let array = requests
            .as_array()
            .ok_or(ReportParseError::InvalidFieldType {
                field: "requests",
                expected: "an array",
            })?;

        if summary.total_requests == 0 {
            summary.total_requests = array.len() as u64;
        }

        let mut derived_failed_requests = 0_u64;
        let mut derived_failed_tests = 0_u64;
        let mut derived_failed_assertions = 0_u64;
        let mut derived_error_count = 0_u64;

        for request in array.iter().filter_map(Value::as_object) {
            let failed_tests = failed_test_count(request);
            let failed_assertions = failed_assertion_count(request);
            let errors = error_count(request);
            let failed = request_failed(request, failed_tests, failed_assertions, errors);

            if failed {
                derived_failed_requests += 1;
            }
            derived_failed_tests += failed_tests;
            derived_failed_assertions += failed_assertions;
            derived_error_count += errors;
        }

        if summary.failed_requests == 0 {
            summary.failed_requests = derived_failed_requests;
        }
        if summary.passed_requests == 0 && summary.total_requests >= summary.failed_requests {
            summary.passed_requests = summary.total_requests - summary.failed_requests;
        }
        if summary.failed_tests == 0 {
            summary.failed_tests = derived_failed_tests;
        }
        if summary.total_tests == 0 {
            summary.total_tests = summary.passed_tests + summary.failed_tests;
        }
        if summary.failed_assertions == 0 {
            summary.failed_assertions = derived_failed_assertions;
        }
        if summary.total_assertions == 0 {
            summary.total_assertions = summary.passed_assertions + summary.failed_assertions;
        }
        if summary.error_count == 0 {
            summary.error_count = derived_error_count;
        }
    }

    Ok(summary)
}

fn parse_requests(requests_value: Option<&Value>) -> Result<Vec<RequestResult>, ReportParseError> {
    let Some(requests_value) = requests_value else {
        return Ok(Vec::new());
    };

    let requests = requests_value
        .as_array()
        .ok_or(ReportParseError::InvalidFieldType {
            field: "requests",
            expected: "an array",
        })?;

    requests
        .iter()
        .enumerate()
        .map(|(index, request)| parse_request(index, request))
        .collect()
}

fn parse_request(index: usize, request: &Value) -> Result<RequestResult, ReportParseError> {
    let map = request
        .as_object()
        .ok_or(ReportParseError::InvalidFieldType {
            field: "requests[]",
            expected: "an object",
        })?;

    Ok(RequestResult {
        name: first_string(map, &["name", "requestName", "request"]),
        method: first_string(map, &["method"]),
        url: first_string(map, &["url"]),
        status: first_string(map, &["status", "result", "outcome"]),
        failed_tests: parse_failed_tests(map, index),
        failed_assertions: parse_failed_assertions(map, index),
        errors: parse_errors(map, index),
    })
}

fn parse_failed_tests(map: &Map<String, Value>, index: usize) -> Vec<FailedTest> {
    collect_array_items(map, &["failedTests", "failed_tests", "tests"], |entry| {
        FailedTest {
            name: first_string(entry, &["name", "testName", "title"]),
            status: first_string(entry, &["status", "result"]),
            message: first_string(entry, &["message", "error", "failure"]),
        }
    })
    .into_iter()
    .filter(|test| {
        !matches!(test.status.as_deref(), Some("passed" | "success" | "ok"))
            || test.message.is_some()
    })
    .map(|mut test| {
        if test.status.is_none() {
            test.status = Some(format!("failed@requests[{index}]"));
        }
        test
    })
    .collect()
}

fn parse_failed_assertions(map: &Map<String, Value>, _index: usize) -> Vec<FailedAssertion> {
    collect_array_items(
        map,
        &["failedAssertions", "failed_assertions", "assertions"],
        |entry| {
            (
                FailedAssertion {
                    name: first_string(entry, &["name", "assertion", "title"]),
                    message: first_string(entry, &["message", "error", "failure"]),
                    expected: first_string(entry, &["expected"]),
                    actual: first_string(entry, &["actual"]),
                },
                first_string(entry, &["status", "result"]),
            )
        },
    )
    .into_iter()
    .filter(|(assertion, status)| {
        assertion.message.is_some()
            || assertion.expected.is_some()
            || assertion.actual.is_some()
            || !matches!(status.as_deref(), Some("passed" | "success" | "ok"))
    })
    .map(|(assertion, _)| assertion)
    .collect()
}

fn parse_errors(map: &Map<String, Value>, _index: usize) -> Vec<ReportError> {
    let collected = collect_array_items(map, &["errors"], |entry| ReportError {
        code: first_string(entry, &["code", "type"]),
        message: first_string(entry, &["message", "error", "detail"]),
    })
    .into_iter()
    .filter(|error| error.code.is_some() || error.message.is_some())
    .collect::<Vec<_>>();

    if !collected.is_empty() {
        return collected;
    }

    let mut errors = Vec::new();
    if let Some(message) = first_string(map, &["error", "message"]) {
        errors.push(ReportError {
            code: first_string(map, &["code", "errorCode"]),
            message: Some(message),
        });
    }
    errors
}

fn request_failed(
    request: &Map<String, Value>,
    failed_tests: u64,
    failed_assertions: u64,
    errors: u64,
) -> bool {
    if failed_tests > 0 || failed_assertions > 0 || errors > 0 {
        return true;
    }

    first_string(request, &["status", "result", "outcome"])
        .map(|status| matches!(status.as_str(), "failed" | "failure" | "error"))
        .unwrap_or(false)
}

fn failed_test_count(request: &Map<String, Value>) -> u64 {
    collect_array_items(
        request,
        &["failedTests", "failed_tests", "tests"],
        |entry| first_string(entry, &["status", "result"]).unwrap_or_else(|| "failed".to_string()),
    )
    .into_iter()
    .filter(|status| !matches!(status.as_str(), "passed" | "success" | "ok"))
    .count() as u64
}

fn failed_assertion_count(request: &Map<String, Value>) -> u64 {
    collect_array_items(
        request,
        &["failedAssertions", "failed_assertions", "assertions"],
        |entry| entry.clone(),
    )
    .len() as u64
}

fn error_count(request: &Map<String, Value>) -> u64 {
    let count = collect_array_items(request, &["errors"], |_| ()).len() as u64;
    if count > 0 {
        return count;
    }

    if request.contains_key("error") { 1 } else { 0 }
}

fn collect_array_items<T>(
    map: &Map<String, Value>,
    keys: &[&str],
    parser: impl Fn(&Map<String, Value>) -> T,
) -> Vec<T> {
    keys.iter()
        .find_map(|key| map.get(*key))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_object)
                .map(parser)
                .collect()
        })
        .unwrap_or_default()
}

fn first_u64(map: &Map<String, Value>, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            Value::Number(number) => number.as_u64(),
            _ => None,
        })
    })
}

fn first_string(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        map.get(*key).and_then(|value| match value {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            Value::Bool(boolean) => Some(boolean.to_string()),
            _ => None,
        })
    })
}
