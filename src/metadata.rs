use std::fs;
use std::path::{Path, PathBuf};

use crate::collection::model::CollectionFormat;

const HTTP_METHODS: &[&str] = &[
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedRequestMetadata {
    pub metadata: RequestMetadata,
    pub diagnostics: Vec<MetadataDiagnostic>,
}

pub fn parse_request_file(path: &Path, format: CollectionFormat) -> ParsedRequestMetadata {
    let source_path = Some(path.to_path_buf());

    match fs::read_to_string(path) {
        Ok(contents) => parse_request_str(&contents, format, source_path),
        Err(error) => ParsedRequestMetadata {
            metadata: RequestMetadata {
                source_path,
                ..RequestMetadata::default()
            },
            diagnostics: vec![MetadataDiagnostic {
                message: format!("failed to read request metadata: {error}"),
            }],
        },
    }
}

pub fn parse_request_str(
    input: &str,
    format: CollectionFormat,
    source_path: Option<PathBuf>,
) -> ParsedRequestMetadata {
    match format {
        CollectionFormat::ClassicJson => parse_classic_bru(input, source_path),
        CollectionFormat::OpenCollectionYaml => parse_open_collection_yaml(input, source_path),
    }
}

fn parse_classic_bru(input: &str, source_path: Option<PathBuf>) -> ParsedRequestMetadata {
    let mut metadata = RequestMetadata {
        source_path,
        ..RequestMetadata::default()
    };
    let mut diagnostics = Vec::new();
    let mut current_block: Option<String> = None;

    for (line_number, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        if trimmed == "}" {
            current_block = None;
            continue;
        }

        if trimmed.ends_with('{') {
            let block_name = trimmed.trim_end_matches('{').trim().to_lowercase();
            if block_name.is_empty() {
                diagnostics.push(diagnostic(
                    line_number + 1,
                    "encountered an unnamed Bruno block",
                ));
                continue;
            }

            if let Some(previous_block) = current_block.replace(block_name.clone()) {
                diagnostics.push(diagnostic(
                    line_number + 1,
                    format!(
                        "starting `{block_name}` before closing `{previous_block}`; recovering best-effort"
                    ),
                ));
            }
            if HTTP_METHODS.contains(&block_name.as_str()) {
                metadata
                    .method
                    .get_or_insert_with(|| block_name.to_uppercase());
            }
            continue;
        }

        let Some(block_name) = current_block.as_deref() else {
            continue;
        };

        let Some((key, raw_value)) = trimmed.split_once(':') else {
            diagnostics.push(diagnostic(
                line_number + 1,
                format!("could not interpret `{trimmed}` inside `{block_name}` block"),
            ));
            continue;
        };
        let key = key.trim();
        let value = clean_value(raw_value);

        match block_name {
            "meta" if key == "name" && !value.is_empty() => {
                metadata.name = Some(value);
            }
            method if HTTP_METHODS.contains(&method) && key == "url" && !value.is_empty() => {
                metadata.method.get_or_insert_with(|| method.to_uppercase());
                metadata.url = Some(value);
            }
            "tags" if !key.is_empty() => {
                if value.eq_ignore_ascii_case("true") || value.is_empty() {
                    metadata.tags.push(key.to_string());
                } else {
                    metadata.tags.push(format!("{key}:{value}"));
                }
            }
            _ => {}
        }
    }

    if current_block.is_some() {
        diagnostics.push(MetadataDiagnostic {
            message: "request file ended before a Bruno block was closed".to_string(),
        });
    }

    ParsedRequestMetadata {
        metadata,
        diagnostics,
    }
}

fn parse_open_collection_yaml(input: &str, source_path: Option<PathBuf>) -> ParsedRequestMetadata {
    let mut metadata = RequestMetadata {
        source_path,
        ..RequestMetadata::default()
    };
    let mut diagnostics = Vec::new();
    let mut in_tags_block = false;

    for (line_number, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if in_tags_block {
            if trimmed.starts_with('-') {
                let tag = clean_value(trimmed.trim_start_matches('-'));
                if !tag.is_empty() {
                    metadata.tags.push(tag);
                }
                continue;
            }

            if line.starts_with(' ') || line.starts_with('\t') {
                diagnostics.push(diagnostic(
                    line_number + 1,
                    format!("unsupported nested tags shape `{trimmed}`"),
                ));
                continue;
            }

            in_tags_block = false;
        }

        let Some((key, raw_value)) = trimmed.split_once(':') else {
            diagnostics.push(diagnostic(
                line_number + 1,
                format!("could not interpret YAML metadata line `{trimmed}`"),
            ));
            continue;
        };
        let key = key.trim();
        let value = raw_value.trim();

        match key {
            "name" if !value.is_empty() => metadata.name = Some(clean_value(value)),
            "method" if !value.is_empty() => {
                metadata.method = Some(clean_value(value).to_uppercase())
            }
            "url" if !value.is_empty() => metadata.url = Some(clean_value(value)),
            "tags" if value.is_empty() => in_tags_block = true,
            "tags" => {
                if let Some(tags) = parse_inline_yaml_tags(value) {
                    metadata.tags.extend(tags);
                } else {
                    diagnostics.push(diagnostic(
                        line_number + 1,
                        format!("unsupported tags value `{value}`"),
                    ));
                }
            }
            _ => {}
        }
    }

    ParsedRequestMetadata {
        metadata,
        diagnostics,
    }
}

fn parse_inline_yaml_tags(value: &str) -> Option<Vec<String>> {
    let trimmed = value.trim();
    if !(trimmed.starts_with('[') && trimmed.ends_with(']')) {
        return None;
    }

    let inner = &trimmed[1..trimmed.len() - 1];
    Some(
        inner
            .split(',')
            .map(clean_value)
            .filter(|tag| !tag.is_empty())
            .collect(),
    )
}

fn clean_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn diagnostic(line_number: usize, message: impl Into<String>) -> MetadataDiagnostic {
    MetadataDiagnostic {
        message: format!("line {line_number}: {}", message.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::collection::model::CollectionFormat;

    use super::parse_request_str;

    #[test]
    fn parses_common_classic_metadata() {
        let parsed = parse_request_str(
            r#"meta {
  name: Create User
}

post {
  url: {{baseUrl}}/users
}

tags {
  smoke: true
  team: platform
}
"#,
            CollectionFormat::ClassicJson,
            Some(PathBuf::from("request.bru")),
        );

        assert_eq!(parsed.metadata.name.as_deref(), Some("Create User"));
        assert_eq!(parsed.metadata.method.as_deref(), Some("POST"));
        assert_eq!(parsed.metadata.url.as_deref(), Some("{{baseUrl}}/users"));
        assert_eq!(parsed.metadata.tags, vec!["smoke", "team:platform"]);
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn malformed_classic_requests_return_partial_metadata_with_diagnostics() {
        let parsed = parse_request_str(
            r#"meta {
  name: Broken Request

get {
  url: https://example.com/bad
}
"#,
            CollectionFormat::ClassicJson,
            None,
        );

        assert_eq!(parsed.metadata.name.as_deref(), Some("Broken Request"));
        assert_eq!(parsed.metadata.method.as_deref(), Some("GET"));
        assert_eq!(
            parsed.metadata.url.as_deref(),
            Some("https://example.com/bad")
        );
        assert!(!parsed.diagnostics.is_empty());
    }

    #[test]
    fn parses_common_open_collection_metadata() {
        let parsed = parse_request_str(
            r#"name: Get Products
method: get
url: "{{baseUrl}}/products"
tags:
  - catalog
  - smoke
"#,
            CollectionFormat::OpenCollectionYaml,
            None,
        );

        assert_eq!(parsed.metadata.name.as_deref(), Some("Get Products"));
        assert_eq!(parsed.metadata.method.as_deref(), Some("GET"));
        assert_eq!(parsed.metadata.url.as_deref(), Some("{{baseUrl}}/products"));
        assert_eq!(parsed.metadata.tags, vec!["catalog", "smoke"]);
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn unsupported_yaml_tag_shapes_are_reported_without_losing_other_metadata() {
        let parsed = parse_request_str(
            r#"name: Login
method: post
url: /login
tags: smoke
"#,
            CollectionFormat::OpenCollectionYaml,
            None,
        );

        assert_eq!(parsed.metadata.name.as_deref(), Some("Login"));
        assert_eq!(parsed.metadata.method.as_deref(), Some("POST"));
        assert_eq!(parsed.metadata.url.as_deref(), Some("/login"));
        assert!(parsed.metadata.tags.is_empty());
        assert_eq!(parsed.diagnostics.len(), 1);
    }
}
