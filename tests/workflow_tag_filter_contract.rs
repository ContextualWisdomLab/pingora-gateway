//! Regression contract for semantic `push` tag-filter keys in GitHub Actions YAML.
//!
//! GitHub Actions treats `tags` and `tags-ignore` as push-ref selectors. This test parses
//! workflow YAML through the repository's YAML implementation so presentation-equivalent
//! mapping keys cannot bypass the protected-main-only duplicate-evidence policy.

use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Returns repository-owned workflow paths in deterministic order.
fn workflow_paths() -> Vec<PathBuf> {
    let mut paths = fs::read_dir(".github/workflows")
        .expect("workflow directory should exist")
        .map(|entry| entry.expect("workflow entry should be readable").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

/// Reads a repository workflow as UTF-8 text.
fn read_workflow(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workflow {} should be readable UTF-8: {error}",
            path.display()
        )
    })
}

/// Detects semantic tag selectors only in workflows that serve both PR and push events.
fn pull_request_push_workflow_has_tag_filter(source: &str) -> bool {
    let document: Value = serde_yaml::from_str(source).unwrap_or_else(|error| {
        panic!("workflow YAML should parse before policy validation: {error}")
    });
    let Some(on) = document.get("on") else {
        return false;
    };
    if on.get("pull_request").is_none() {
        return false;
    }
    let Some(push) = on.get("push").and_then(Value::as_mapping) else {
        return false;
    };

    push.keys()
        .any(|key| matches!(key.as_str(), Some("tags" | "tags-ignore")))
}

#[test]
fn pull_request_push_workflows_reject_semantic_tag_filters() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        assert!(
            !pull_request_push_workflow_has_tag_filter(&source),
            "{} must not add tag selectors to a PR workflow's protected-main duplicate-evidence push lane",
            path.display()
        );
    }
}

#[test]
fn yaml_separation_space_before_tag_colon_is_still_a_tag_filter() {
    for key in ["tags :", "tags-ignore :"] {
        let source = format!(
            "on:\n  push:\n    branches:\n      - main\n    {key}\n      - 'v*'\n  pull_request:\n"
        );
        assert!(pull_request_push_workflow_has_tag_filter(&source));
    }
}

#[test]
fn quoted_tag_filter_keys_are_still_tag_filters() {
    for key in ["'tags':", "\"tags-ignore\":"] {
        let source = format!(
            "on:\n  push:\n    branches:\n      - main\n    {key}\n      - 'v*'\n  pull_request:\n"
        );
        assert!(pull_request_push_workflow_has_tag_filter(&source));
    }
}

#[test]
fn escaped_double_quoted_tag_filter_keys_are_still_tag_filters() {
    let escaped_tags = r#"on:
  push:
    branches:
      - main
    "t\u0061gs":
      - "v*"
  pull_request:
"#;
    let escaped_tags_ignore = r#"on:
  push:
    branches:
      - main
    "tags\u002dignore":
      - "v*"
  pull_request:
"#;

    assert!(pull_request_push_workflow_has_tag_filter(escaped_tags));
    assert!(pull_request_push_workflow_has_tag_filter(
        escaped_tags_ignore
    ));
}

#[test]
fn unrelated_push_filters_and_push_only_tag_workflows_remain_out_of_scope() {
    let unrelated = "on:\n  push:\n    branches:\n      - main\n    paths:\n      - 'src/**'\n  pull_request:\n";
    let push_only_tag_release = "on:\n  push:\n    tags:\n      - 'v*'\n";

    assert!(!pull_request_push_workflow_has_tag_filter(unrelated));
    assert!(!pull_request_push_workflow_has_tag_filter(
        push_only_tag_release
    ));
}
