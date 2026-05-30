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
    pub status_code: Option<u64>,
    pub status_text: Option<String>,
    pub duration_ms: Option<u64>,
    pub size_bytes: Option<u64>,
    pub request_body: Option<String>,
    pub response_body: Option<String>,
    #[serde(default)]
    pub request_headers: Vec<Header>,
    #[serde(default)]
    pub response_headers: Vec<Header>,
    #[serde(default)]
    pub tests: Vec<TestResult>,
    #[serde(default)]
    pub failed_tests: Vec<FailedTest>,
    #[serde(default)]
    pub failed_assertions: Vec<FailedAssertion>,
    #[serde(default)]
    pub errors: Vec<ReportError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TestResult {
    pub name: Option<String>,
    pub status: Option<String>,
    pub message: Option<String>,
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
    #[error("Bruno report root must be a JSON object or array")]
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
    let (summary, requests, diagnostics) = match &value {
        Value::Object(root) => parse_object_report(root)?,
        Value::Array(iterations) => parse_iteration_report(iterations)?,
        _ => return Err(ReportParseError::InvalidRoot),
    };

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

fn parse_object_report(
    root: &Map<String, Value>,
) -> Result<(ReportSummary, Vec<RequestResult>, Vec<ParseDiagnostic>), ReportParseError> {
    let requests_value = root.get("requests").or_else(|| root.get("results"));
    let summary = parse_summary(root.get("summary"), requests_value)?;
    let requests = parse_requests(requests_value)?;
    let mut diagnostics = Vec::new();

    if !root.contains_key("summary") {
        diagnostics.push(ParseDiagnostic {
            path: "summary".to_string(),
            message: "summary missing; counts were derived from request results when possible"
                .to_string(),
        });
    }

    if requests_value.is_none() {
        diagnostics.push(ParseDiagnostic {
            path: "requests".to_string(),
            message: "requests/results missing; failure details unavailable".to_string(),
        });
    }

    Ok((summary, requests, diagnostics))
}

fn parse_iteration_report(
    iterations: &[Value],
) -> Result<(ReportSummary, Vec<RequestResult>, Vec<ParseDiagnostic>), ReportParseError> {
    let mut summary = ReportSummary::default();
    let mut requests = Vec::new();
    let mut diagnostics = Vec::new();

    for (index, iteration) in iterations.iter().enumerate() {
        let iteration = iteration
            .as_object()
            .ok_or(ReportParseError::InvalidFieldType {
                field: "iterations[]",
                expected: "an object",
            })?;
        let requests_value = iteration
            .get("results")
            .or_else(|| iteration.get("requests"));
        let iteration_summary = parse_summary(iteration.get("summary"), requests_value)?;
        add_summary(&mut summary, &iteration_summary);
        requests.extend(parse_requests(requests_value)?);

        if !iteration.contains_key("summary") {
            diagnostics.push(ParseDiagnostic {
                path: format!("[{index}].summary"),
                message: "summary missing; counts were derived from request results when possible"
                    .to_string(),
            });
        }
        if requests_value.is_none() {
            diagnostics.push(ParseDiagnostic {
                path: format!("[{index}].results"),
                message: "results/requests missing; failure details unavailable".to_string(),
            });
        }
    }

    Ok((summary, requests, diagnostics))
}

fn add_summary(total: &mut ReportSummary, summary: &ReportSummary) {
    total.total_requests += summary.total_requests;
    total.passed_requests += summary.passed_requests;
    total.failed_requests += summary.failed_requests;
    total.total_tests += summary.total_tests;
    total.passed_tests += summary.passed_tests;
    total.failed_tests += summary.failed_tests;
    total.total_assertions += summary.total_assertions;
    total.passed_assertions += summary.passed_assertions;
    total.failed_assertions += summary.failed_assertions;
    total.error_count += summary.error_count;
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
        summary.total_tests = first_u64(map, &["total_tests", "totalTests", "tests"])
            .unwrap_or_default()
            + first_u64(map, &["totalPreRequestTests"]).unwrap_or_default()
            + first_u64(map, &["totalPostResponseTests"]).unwrap_or_default();
        summary.passed_tests = first_u64(map, &["passed_tests", "passedTests"]).unwrap_or_default()
            + first_u64(map, &["passedPreRequestTests"]).unwrap_or_default()
            + first_u64(map, &["passedPostResponseTests"]).unwrap_or_default();
        summary.failed_tests = first_u64(map, &["failed_tests", "failedTests"]).unwrap_or_default()
            + first_u64(map, &["failedPreRequestTests"]).unwrap_or_default()
            + first_u64(map, &["failedPostResponseTests"]).unwrap_or_default();
        summary.total_assertions =
            first_u64(map, &["total_assertions", "totalAssertions"]).unwrap_or_default();
        summary.passed_assertions =
            first_u64(map, &["passed_assertions", "passedAssertions"]).unwrap_or_default();
        summary.failed_assertions =
            first_u64(map, &["failed_assertions", "failedAssertions"]).unwrap_or_default();
        summary.error_count = first_u64(
            map,
            &["error_count", "errorCount", "errors", "errorRequests"],
        )
        .unwrap_or_default();
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

    let request = map.get("request").and_then(Value::as_object);
    let response = map.get("response").and_then(Value::as_object);
    let test = map.get("test").and_then(Value::as_object);

    Ok(RequestResult {
        name: first_string(map, &["name", "requestName"])
            .or_else(|| test.and_then(|map| first_string(map, &["name", "filename"])))
            .or_else(|| first_string(map, &["request"])),
        method: first_string(map, &["method"])
            .or_else(|| request.and_then(|map| first_string(map, &["method"]))),
        url: first_string(map, &["url"])
            .or_else(|| request.and_then(|map| first_string(map, &["url"]))),
        status: first_string(map, &["status", "result", "outcome"])
            .or_else(|| response.and_then(|map| first_string(map, &["statusText", "status"]))),
        status_code: response
            .and_then(|map| first_u64(map, &["status", "statusCode", "code"]))
            .or_else(|| first_u64(map, &["statusCode"])),
        status_text: response
            .and_then(|map| first_string(map, &["statusText", "statusMessage"]))
            .or_else(|| first_string(map, &["statusText"])),
        duration_ms: first_u64(map, &["duration", "durationMs", "responseTime"]).or_else(|| {
            response.and_then(|map| first_u64(map, &["duration", "durationMs", "responseTime"]))
        }),
        size_bytes: first_u64(map, &["size", "sizeBytes", "responseSize"]).or_else(|| {
            response.and_then(|map| first_u64(map, &["size", "sizeBytes", "responseSize"]))
        }),
        request_body: request.and_then(|map| first_body(map)),
        response_body: response
            .and_then(|map| first_body(map))
            .or_else(|| first_body(map)),
        request_headers: request
            .and_then(|map| parse_headers_value(map.get("headers")))
            .unwrap_or_default(),
        response_headers: response
            .and_then(|map| parse_headers_value(map.get("headers")))
            .unwrap_or_default(),
        tests: parse_tests(map),
        failed_tests: parse_failed_tests(map, index),
        failed_assertions: parse_failed_assertions(map, index),
        errors: parse_errors(map, index),
    })
}

fn parse_tests(map: &Map<String, Value>) -> Vec<TestResult> {
    collect_array_items(
        map,
        &[
            "tests",
            "testResults",
            "preRequestTestResults",
            "postResponseTestResults",
        ],
        |entry| TestResult {
            name: first_string(entry, &["name", "testName", "title"]),
            status: first_string(entry, &["status", "result"]),
            message: first_string(entry, &["message", "error", "failure"]),
        },
    )
}

fn parse_failed_tests(map: &Map<String, Value>, index: usize) -> Vec<FailedTest> {
    collect_array_items(
        map,
        &[
            "failedTests",
            "failed_tests",
            "tests",
            "testResults",
            "preRequestTestResults",
            "postResponseTestResults",
        ],
        |entry| FailedTest {
            name: first_string(entry, &["name", "testName", "title"]),
            status: first_string(entry, &["status", "result"]),
            message: first_string(entry, &["message", "error", "failure"]),
        },
    )
    .into_iter()
    .filter(|test| {
        !matches!(
            test.status.as_deref(),
            Some("passed" | "pass" | "success" | "ok")
        ) || test.message.is_some()
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
        &[
            "failedAssertions",
            "failed_assertions",
            "assertions",
            "assertionResults",
        ],
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
            || !matches!(
                status.as_deref(),
                Some("passed" | "pass" | "success" | "ok")
            )
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
    } else if let Some(error) = map.get("error").and_then(Value::as_object) {
        if let Some(message) = first_string(error, &["message", "error", "detail"]) {
            errors.push(ReportError {
                code: first_string(error, &["code", "type", "errorCode"]),
                message: Some(message),
            });
        }
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
        &[
            "failedTests",
            "failed_tests",
            "tests",
            "testResults",
            "preRequestTestResults",
            "postResponseTestResults",
        ],
        |entry| first_string(entry, &["status", "result"]).unwrap_or_else(|| "failed".to_string()),
    )
    .into_iter()
    .filter(|status| !matches!(status.as_str(), "passed" | "pass" | "success" | "ok"))
    .count() as u64
}

fn failed_assertion_count(request: &Map<String, Value>) -> u64 {
    collect_array_items(
        request,
        &[
            "failedAssertions",
            "failed_assertions",
            "assertions",
            "assertionResults",
        ],
        |entry| first_string(entry, &["status", "result"]).unwrap_or_else(|| "failed".to_string()),
    )
    .into_iter()
    .filter(|status| !matches!(status.as_str(), "passed" | "pass" | "success" | "ok"))
    .count() as u64
}

fn error_count(request: &Map<String, Value>) -> u64 {
    let count = collect_array_items(request, &["errors"], |_| ()).len() as u64;
    if count > 0 {
        return count;
    }

    match request.get("error") {
        Some(Value::Null) | None => 0,
        Some(Value::Object(error))
            if first_string(error, &["message", "error", "detail"]).is_none() =>
        {
            0
        }
        Some(_) => 1,
    }
}

fn parse_headers_value(value: Option<&Value>) -> Option<Vec<Header>> {
    match value? {
        Value::Object(headers) => Some(
            headers
                .iter()
                .map(|(name, value)| Header {
                    name: name.clone(),
                    value: value_to_display_string(value),
                })
                .collect(),
        ),
        Value::Array(headers) => Some(
            headers
                .iter()
                .filter_map(|entry| {
                    let entry = entry.as_object()?;
                    Some(Header {
                        name: first_string(entry, &["name", "key"]).unwrap_or_default(),
                        value: first_string(entry, &["value"]).unwrap_or_default(),
                    })
                })
                .filter(|header| !header.name.is_empty() || !header.value.is_empty())
                .collect(),
        ),
        _ => None,
    }
}

fn first_body(map: &Map<String, Value>) -> Option<String> {
    ["body", "data", "text", "content"]
        .iter()
        .find_map(|key| map.get(*key).map(value_to_display_string))
}

fn value_to_display_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Number(number) => number.to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
        }
    }
}

fn collect_array_items<T>(
    map: &Map<String, Value>,
    keys: &[&str],
    parser: impl Fn(&Map<String, Value>) -> T,
) -> Vec<T> {
    keys.iter()
        .filter_map(|key| map.get(*key).and_then(Value::as_array))
        .flat_map(|items| items.iter().filter_map(Value::as_object).map(&parser))
        .collect()
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
