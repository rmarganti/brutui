use std::fs;

use brutui::report::{
    FailedAssertion, FailedTest, ReportError, ReportParseError, ReportSummary, parse_report_file,
    parse_report_str,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn parses_successful_report_with_summary_and_source_path() {
    let workspace = tempdir().expect("temp dir");
    let report_path = workspace.path().join("report.json");
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&json!({
            "summary": {
                "total_requests": 1,
                "passed_requests": 1,
                "failed_requests": 0,
                "total_tests": 2,
                "passed_tests": 2,
                "failed_tests": 0,
                "total_assertions": 2,
                "passed_assertions": 2,
                "failed_assertions": 0,
                "error_count": 0
            },
            "requests": [
                {
                    "name": "List users",
                    "method": "GET",
                    "url": "https://example.test/users",
                    "status": "passed",
                    "tests": [
                        {"name": "status code", "status": "passed"},
                        {"name": "body schema", "status": "passed"}
                    ]
                }
            ]
        }))
        .expect("serialize report"),
    )
    .expect("write report");

    let report = parse_report_file(&report_path).expect("parse report");

    assert_eq!(report.source_path.as_deref(), Some(report_path.as_path()));
    assert_eq!(report.summary.total_requests, 1);
    assert_eq!(report.summary.passed_requests, 1);
    assert_eq!(report.requests[0].name.as_deref(), Some("List users"));
    assert!(report.requests[0].failed_tests.is_empty());
    assert!(report.requests[0].failed_assertions.is_empty());
    assert!(report.requests[0].errors.is_empty());
    assert!(report.diagnostics.is_empty());
}

#[test]
fn parses_failed_requests_tests_assertions_and_errors() {
    let report = parse_report_str(
        &serde_json::to_string(&json!({
            "summary": {
                "totalRequests": 2,
                "passedRequests": 1,
                "failedRequests": 1,
                "failedTests": 1,
                "failedAssertions": 1,
                "errorCount": 1
            },
            "requests": [
                {
                    "requestName": "Create user",
                    "method": "POST",
                    "url": "https://example.test/users",
                    "result": "failed",
                    "failedTests": [
                        {"testName": "status code", "message": "expected 201"}
                    ],
                    "failedAssertions": [
                        {
                            "assertion": "body.id",
                            "message": "id missing",
                            "expected": "present",
                            "actual": "null"
                        }
                    ],
                    "errors": [
                        {"code": "ASSERTION_ERROR", "message": "request assertions failed"}
                    ]
                },
                {
                    "requestName": "Health",
                    "result": "passed"
                }
            ]
        }))
        .expect("serialize report"),
        None,
    )
    .expect("parse report");

    assert_eq!(report.summary.total_requests, 2);
    assert_eq!(report.summary.failed_requests, 1);
    assert_eq!(report.summary.failed_tests, 1);
    assert_eq!(report.summary.failed_assertions, 1);
    assert_eq!(report.summary.error_count, 1);
    assert_eq!(report.requests.len(), 2);
    assert_eq!(
        report.requests[0].failed_tests,
        vec![FailedTest {
            name: Some("status code".to_string()),
            status: Some("failed@requests[0]".to_string()),
            message: Some("expected 201".to_string()),
        }]
    );
    assert_eq!(
        report.requests[0].failed_assertions,
        vec![FailedAssertion {
            name: Some("body.id".to_string()),
            message: Some("id missing".to_string()),
            expected: Some("present".to_string()),
            actual: Some("null".to_string()),
        }]
    );
    assert_eq!(
        report.requests[0].errors,
        vec![ReportError {
            code: Some("ASSERTION_ERROR".to_string()),
            message: Some("request assertions failed".to_string()),
        }]
    );
}

#[test]
fn missing_optional_fields_and_partial_summary_are_derived_defensively() {
    let report = parse_report_str(
        &serde_json::to_string(&json!({
            "requests": [
                {
                    "status": "passed"
                },
                {
                    "status": "failed",
                    "assertions": [
                        {"name": "response body", "message": "mismatch"}
                    ],
                    "error": "socket hang up"
                }
            ]
        }))
        .expect("serialize report"),
        None,
    )
    .expect("parse report");

    assert_eq!(
        report.summary,
        ReportSummary {
            total_requests: 2,
            passed_requests: 1,
            failed_requests: 1,
            total_tests: 0,
            passed_tests: 0,
            failed_tests: 0,
            total_assertions: 1,
            passed_assertions: 0,
            failed_assertions: 1,
            error_count: 1,
        }
    );
    assert_eq!(report.requests[0].name, None);
    assert_eq!(report.requests[1].failed_assertions.len(), 1);
    assert_eq!(
        report.requests[1].errors[0].message.as_deref(),
        Some("socket hang up")
    );
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].path, "summary");
}

#[test]
fn parses_bruno_iteration_report_format() {
    let report = parse_report_str(
        &serde_json::to_string(&json!([
            {
                "iterationIndex": 0,
                "summary": {
                    "totalRequests": 1,
                    "passedRequests": 1,
                    "failedRequests": 0,
                    "errorRequests": 0,
                    "totalTests": 1,
                    "passedTests": 1,
                    "failedTests": 0,
                    "totalPreRequestTests": 1,
                    "passedPreRequestTests": 1,
                    "failedPreRequestTests": 0,
                    "totalAssertions": 1,
                    "passedAssertions": 1,
                    "failedAssertions": 0
                },
                "results": [
                    {
                        "test": {"filename": "Morty Rick.yml"},
                        "request": {
                            "method": "GET",
                            "url": "https://rickandmortyapi.com/api/character/231"
                        },
                        "response": {"status": 200, "statusText": "OK"},
                        "status": "pass",
                        "testResults": [{"name": "status code", "status": "pass"}],
                        "assertionResults": [{"name": "response body", "status": "pass"}],
                        "error": null
                    }
                ]
            }
        ]))
        .expect("serialize report"),
        None,
    )
    .expect("parse report");

    assert_eq!(report.summary.total_requests, 1);
    assert_eq!(report.summary.passed_requests, 1);
    assert_eq!(report.summary.total_tests, 2);
    assert_eq!(report.requests.len(), 1);
    assert_eq!(report.requests[0].name.as_deref(), Some("Morty Rick.yml"));
    assert_eq!(report.requests[0].method.as_deref(), Some("GET"));
    assert_eq!(
        report.requests[0].url.as_deref(),
        Some("https://rickandmortyapi.com/api/character/231")
    );
    assert!(report.requests[0].failed_tests.is_empty());
    assert!(report.requests[0].failed_assertions.is_empty());
    assert!(report.requests[0].errors.is_empty());
}

#[test]
fn parses_bruno_object_report_format_with_results_key() {
    let report = parse_report_str(
        &serde_json::to_string(&json!({
            "summary": {
                "totalRequests": 1,
                "passedRequests": 0,
                "failedRequests": 1,
                "failedTests": 1,
                "failedAssertions": 1,
                "errorRequests": 1
            },
            "results": [
                {
                    "name": "Create user",
                    "request": {"method": "POST", "url": "https://example.test/users"},
                    "response": {"status": 500, "statusText": "Internal Server Error"},
                    "testResults": [{"name": "status code", "status": "fail", "message": "expected 201"}],
                    "assertionResults": [{"name": "body.id", "status": "fail", "message": "missing"}],
                    "error": {"message": "request failed"}
                }
            ]
        }))
        .expect("serialize report"),
        None,
    )
    .expect("parse report");

    assert_eq!(report.summary.failed_requests, 1);
    assert_eq!(report.summary.error_count, 1);
    assert_eq!(report.requests[0].method.as_deref(), Some("POST"));
    assert_eq!(report.requests[0].failed_tests.len(), 1);
    assert_eq!(report.requests[0].failed_assertions.len(), 1);
    assert_eq!(
        report.requests[0].errors[0].message.as_deref(),
        Some("request failed")
    );
}

#[test]
fn rejects_malformed_json_and_invalid_shapes() {
    let malformed = parse_report_str("{", None).expect_err("invalid json");
    assert!(matches!(malformed, ReportParseError::InvalidJson(_)));

    let invalid_requests = parse_report_str(
        &serde_json::to_string(&json!({
            "summary": {"total_requests": 1},
            "requests": {}
        }))
        .expect("serialize report"),
        None,
    )
    .expect_err("invalid requests shape");
    assert!(matches!(
        invalid_requests,
        ReportParseError::InvalidFieldType {
            field: "requests",
            expected: "an array"
        }
    ));
}

#[test]
fn rejects_reports_missing_required_semantics() {
    let error = parse_report_str(
        &serde_json::to_string(&json!({
            "summary": {},
            "requests": []
        }))
        .expect("serialize report"),
        None,
    )
    .expect_err("missing semantics");

    assert!(matches!(
        error,
        ReportParseError::MissingSemantics("expected summary counts or request results")
    ));
}
