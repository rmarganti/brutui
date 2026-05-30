use crate::{
    collection::model::CollectionNodeId,
    report::RequestResult,
    state::{
        CompletedRun, CompletedRunStatus, OutputLine, OutputStream, ResponseTab, RunState,
        SessionState,
    },
};

pub fn current_tab_text(session: &SessionState) -> String {
    if let RunState::Running(active) = session.run_state() {
        return if active.cancellation_requested {
            "Cancellation requested; waiting for Bruno to exit.".to_string()
        } else {
            let mut text = format!("Run in progress: {}", selected_node_label(&active.target));
            if !session.raw_output().is_empty() {
                text.push_str("\n\n");
                text.push_str(&raw_output_text(session.raw_output()));
            }
            text
        };
    }

    let Some(request) = session.selected_response_result() else {
        return match session.completed_run() {
            Some(run) => match &run.status {
                CompletedRunStatus::Cancelled => {
                    "Status: cancelled\nRun was cancelled.".to_string()
                }
                CompletedRunStatus::ToolError(error) => {
                    format!(
                        "Status: tool error\nExecution/tool error: {}",
                        error.message
                    )
                }
                CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) => {
                    let mut text = completed_run_summary_text(run);
                    if report.requests.is_empty() {
                        text.push_str(
                            "\nThe Bruno report did not include request/response results.",
                        );
                    } else {
                        text.push_str("\nNo response selected.");
                    }
                    text
                }
            },
            None => {
                "No Bruno run started yet. Press r on the selected node to start a run.".to_string()
            }
        };
    };

    let mut text = session
        .completed_run()
        .map(completed_run_summary_text)
        .unwrap_or_default();
    if !text.is_empty() {
        text.push_str("\n\n");
    }
    if matches!(session.response_tab(), ResponseTab::Tests) && !session.raw_output().is_empty() {
        text.push_str("Raw output:\n");
        text.push_str(&raw_output_text(session.raw_output()));
        text.push_str("\n\n");
    }
    text.push_str(&match session.response_tab() {
        ResponseTab::Body => response_body_text(request),
        ResponseTab::Headers => response_headers_text(request),
        ResponseTab::Tests => response_tests_text(request),
    });
    text
}

fn completed_run_summary_text(run: &CompletedRun) -> String {
    let mut lines = vec![
        format!("Status: {}", completed_status_label(&run.status)),
        format!("Exit code: {}", display_exit_code(run.exit_code)),
        format!("Report: {}", run.report_path.display()),
    ];
    if let CompletedRunStatus::Success(report) | CompletedRunStatus::FailedTests(report) =
        &run.status
    {
        lines.push(format!(
            "Requests: {} total, {} passed, {} failed",
            report.summary.total_requests,
            report.summary.passed_requests,
            report.summary.failed_requests
        ));
        lines.push(format!(
            "Tests: {} total, {} passed, {} failed",
            report.summary.total_tests, report.summary.passed_tests, report.summary.failed_tests
        ));
        lines.push(format!(
            "Assertions: {} total, {} passed, {} failed",
            report.summary.total_assertions,
            report.summary.passed_assertions,
            report.summary.failed_assertions
        ));
    }
    lines.join("\n")
}

fn response_body_text(request: &RequestResult) -> String {
    let mut lines = response_heading(request);
    lines.push(String::new());
    lines.push(
        request
            .response_body
            .clone()
            .unwrap_or_else(|| "No response body captured by bru --reporter-json.".to_string()),
    );
    lines.join("\n")
}

fn response_headers_text(request: &RequestResult) -> String {
    let mut lines = response_heading(request);
    if !request.failed_tests.is_empty()
        || !request.failed_assertions.is_empty()
        || !request.errors.is_empty()
    {
        lines.push(String::new());
        lines.push("Failure details:".to_string());
        append_failure_lines(&mut lines, request);
    }
    lines.push(String::new());
    lines.push("Response headers:".to_string());
    if request.response_headers.is_empty() {
        lines.push("  No response headers captured.".to_string());
    } else {
        for header in &request.response_headers {
            lines.push(format!("  {}: {}", header.name, header.value));
        }
    }
    lines.push(String::new());
    lines.push("Request headers:".to_string());
    if request.request_headers.is_empty() {
        lines.push("  No request headers captured.".to_string());
    } else {
        for header in &request.request_headers {
            lines.push(format!("  {}: {}", header.name, header.value));
        }
    }
    lines.join("\n")
}

fn response_tests_text(request: &RequestResult) -> String {
    let mut lines = response_heading(request);
    lines.push(String::new());
    if request.tests.is_empty()
        && request.failed_tests.is_empty()
        && request.failed_assertions.is_empty()
        && request.errors.is_empty()
    {
        lines.push("No tests, assertions, or errors captured.".to_string());
    }
    for test in &request.tests {
        lines.push(format!(
            "{} {}{}",
            status_icon(test.status.as_deref()),
            test.name.as_deref().unwrap_or("unnamed test"),
            test.message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for failed_test in &request.failed_tests {
        lines.push(format!(
            "✗ Failed test: {}{}",
            failed_test.name.as_deref().unwrap_or("unnamed"),
            failed_test
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for assertion in &request.failed_assertions {
        lines.push(format!(
            "✗ assertion {}{}",
            assertion.name.as_deref().unwrap_or("unnamed"),
            assertion
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for error in &request.errors {
        lines.push(format!(
            "✗ error [{}]: {}",
            error.code.as_deref().unwrap_or("error"),
            error.message.as_deref().unwrap_or("no details")
        ));
    }
    lines.join("\n")
}

fn append_failure_lines(lines: &mut Vec<String>, request: &RequestResult) {
    for failed_test in &request.failed_tests {
        lines.push(format!(
            "Failed test: {}{}",
            failed_test.name.as_deref().unwrap_or("unnamed"),
            failed_test
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
    }
    for assertion in &request.failed_assertions {
        lines.push(format!(
            "Failed assertion: {}{}",
            assertion.name.as_deref().unwrap_or("unnamed"),
            assertion
                .message
                .as_deref()
                .map(|message| format!(" — {message}"))
                .unwrap_or_default()
        ));
        if assertion.expected.is_some() || assertion.actual.is_some() {
            lines.push(format!(
                "expected: {} • actual: {}",
                assertion.expected.as_deref().unwrap_or("unknown"),
                assertion.actual.as_deref().unwrap_or("unknown")
            ));
        }
    }
}

fn response_heading(request: &RequestResult) -> Vec<String> {
    let label = request
        .url
        .as_deref()
        .or(request.name.as_deref())
        .unwrap_or("Unnamed request");
    let mut lines = vec![
        format!("Request: {label}"),
        format!("{} {label}", request.method.as_deref().unwrap_or("REQUEST")),
    ];
    let status = request
        .status_code
        .map(|code| code.to_string())
        .or_else(|| request.status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut meta = format!("Status: {status}");
    if let Some(text) = &request.status_text {
        meta.push_str(&format!(" {text}"));
    }
    if let Some(duration) = request.duration_ms {
        meta.push_str(&format!(" • {duration} ms"));
    }
    if let Some(size) = request.size_bytes {
        meta.push_str(&format!(" • {size} B"));
    }
    lines.push(meta);
    lines
}

fn status_icon(status: Option<&str>) -> &'static str {
    match status {
        Some("passed" | "pass" | "success" | "ok") => "✓",
        Some("skipped" | "skip") => "-",
        _ => "✗",
    }
}

fn raw_output_text(output: &[OutputLine]) -> String {
    output
        .iter()
        .map(|line| {
            let stream = match line.stream {
                OutputStream::Stdout => "stdout",
                OutputStream::Stderr => "stderr",
            };
            format!("[{stream}] {}", line.text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn selected_node_label(node: &CollectionNodeId) -> &'static str {
    match node {
        CollectionNodeId::Root => "Collection root",
        CollectionNodeId::Folder(_) => "Folder",
        CollectionNodeId::Request(_) => "Request",
    }
}

fn completed_status_label(status: &CompletedRunStatus) -> &'static str {
    match status {
        CompletedRunStatus::Success(_) => "success",
        CompletedRunStatus::FailedTests(_) => "completed with failures",
        CompletedRunStatus::Cancelled => "cancelled",
        CompletedRunStatus::ToolError(_) => "tool error",
    }
}

fn display_exit_code(code: Option<i32>) -> String {
    code.map(|code| code.to_string())
        .unwrap_or_else(|| "none".to_string())
}
