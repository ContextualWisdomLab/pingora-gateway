//! Regression contract for semantic `push` tag-filter keys in GitHub Actions YAML.
//!
//! GitHub Actions treats `tags` and `tags-ignore` as push-ref selectors. This test keeps
//! presentation-equivalent YAML key spellings from bypassing the protected-main-only
//! duplicate-evidence policy enforced by the workflow concurrency contract.

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

/// Counts leading spaces so only direct `push` children are classified as filters.
fn indentation(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

/// Returns one canonical block-style event beneath top-level `on:`.
fn event_block<'a>(source: &'a str, event: &str) -> Option<Vec<&'a str>> {
    let target = format!("  {event}:");
    let mut in_on = false;
    let mut in_event = false;
    let mut block = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        let ignorable = trimmed.is_empty() || trimmed.starts_with('#');

        if !in_on {
            if line == "on:" {
                in_on = true;
            }
            continue;
        }

        if !in_event {
            if !ignorable && indentation(line) == 0 {
                return None;
            }
            if line == target {
                in_event = true;
            }
            continue;
        }

        if !ignorable && indentation(line) <= 2 {
            break;
        }
        block.push(line);
    }

    in_event.then_some(block)
}

/// Detects tag selectors using the predecessor lexical classifier.
fn direct_push_filter_is_tag_selector(line: &str) -> bool {
    if indentation(line) != 4 {
        return false;
    }

    matches!(
        line.trim().split_once(':').map(|(key, _)| key),
        Some("tags" | "tags-ignore")
    )
}

#[test]
fn pull_request_push_workflows_reject_semantic_tag_filters() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        if event_block(&source, "pull_request").is_none() {
            continue;
        }
        let Some(push) = event_block(&source, "push") else {
            continue;
        };

        assert!(
            !push
                .into_iter()
                .any(direct_push_filter_is_tag_selector),
            "{} must not add tag selectors to a PR workflow's protected-main duplicate-evidence push lane",
            path.display()
        );
    }
}

#[test]
fn yaml_separation_space_before_tag_colon_is_still_a_tag_filter() {
    assert!(direct_push_filter_is_tag_selector("    tags :"));
    assert!(direct_push_filter_is_tag_selector("    tags-ignore :"));
}

#[test]
fn quoted_tag_filter_keys_are_still_tag_filters() {
    assert!(direct_push_filter_is_tag_selector("    'tags':"));
    assert!(direct_push_filter_is_tag_selector("    \"tags-ignore\":"));
}

#[test]
fn unrelated_direct_push_filters_are_not_tag_filters() {
    for line in [
        "    branches:",
        "    paths:",
        "    paths-ignore:",
        "    tags-extra:",
    ] {
        assert!(!direct_push_filter_is_tag_selector(line));
    }
}
