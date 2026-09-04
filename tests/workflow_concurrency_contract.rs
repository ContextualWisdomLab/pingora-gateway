//! Repository-level contract for GitHub Actions admission and coalescing semantics.
//!
//! Pull-request runs may cancel only an older run for the same workflow, repository, and PR.
//! Push runs are protected-main evidence and must not share that cancellation identity.

use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_GROUP_LINE: &str =
    "  group: ${{ github.workflow }}-${{ github.repository }}-${{ github.event.pull_request.number || github.run_id }}";
const EXPECTED_CANCEL_LINE: &str =
    "  cancel-in-progress: ${{ github.event_name == 'pull_request' }}";

/// Returns all repository-owned YAML workflows in deterministic path order.
fn workflow_paths() -> Vec<PathBuf> {
    let mut paths = fs::read_dir(".github/workflows")
        .expect("workflow directory should exist")
        .map(|entry| entry.expect("workflow entry should be readable").path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|ext| ext.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

/// Reads one workflow as UTF-8 because GitHub workflow sources are text contracts.
fn read_workflow(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workflow {} should be readable UTF-8: {error}",
            path.display()
        )
    })
}

/// Counts leading spaces so event parsing cannot be satisfied by comments or unrelated job keys.
fn indentation(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

/// Returns one block-style event nested directly under the top-level `on` mapping.
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

/// Returns the top-level concurrency mapping without accepting a job-local look-alike.
fn concurrency_block<'a>(source: &'a str) -> Option<Vec<&'a str>> {
    let mut in_block = false;
    let mut block = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        let ignorable = trimmed.is_empty() || trimmed.starts_with('#');

        if !in_block {
            if line == "concurrency:" {
                in_block = true;
            }
            continue;
        }

        if !ignorable && indentation(line) == 0 {
            break;
        }
        block.push(line);
    }

    in_block.then_some(block)
}

/// Extracts the exact branch allow-list from a block-style `push` event.
fn push_branches(source: &str) -> Option<Vec<String>> {
    let block = event_block(source, "push")?;
    let mut in_branches = false;
    let mut branches = Vec::new();

    for line in block {
        let trimmed = line.trim();
        let ignorable = trimmed.is_empty() || trimmed.starts_with('#');

        if !in_branches {
            if line == "    branches:" {
                in_branches = true;
            }
            continue;
        }

        if !ignorable && indentation(line) <= 4 {
            break;
        }
        if indentation(line) == 6 {
            if let Some(branch) = trimmed.strip_prefix("- ") {
                let normalized = branch
                    .trim()
                    .trim_matches(|character| character == '\'' || character == '"');
                branches.push(normalized.to_owned());
            }
        }
    }

    in_branches.then_some(branches)
}

#[test]
fn pull_request_workflows_use_pr_scoped_cancellation_identity() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        if event_block(&source, "pull_request").is_none() {
            continue;
        }

        let concurrency = concurrency_block(&source).unwrap_or_else(|| {
            panic!(
                "{} must define top-level concurrency for pull-request admission",
                path.display()
            )
        });
        assert_eq!(
            concurrency
                .iter()
                .filter(|line| **line == EXPECTED_GROUP_LINE)
                .count(),
            1,
            "{} must scope concurrency exactly once to workflow/repository/PR identity",
            path.display()
        );
        assert_eq!(
            concurrency
                .iter()
                .filter(|line| **line == EXPECTED_CANCEL_LINE)
                .count(),
            1,
            "{} must cancel only superseded pull-request runs",
            path.display()
        );
    }
}

#[test]
fn push_workflows_are_limited_to_protected_main() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        if event_block(&source, "push").is_none() {
            continue;
        }

        assert_eq!(
            push_branches(&source),
            Some(vec!["main".to_owned()]),
            "{} must admit push evidence for protected main and no other branch pattern",
            path.display()
        );
    }
}

#[test]
fn push_branch_parser_rejects_additional_feature_branch_patterns() {
    let source = "on:\n  push:\n    branches:\n      - main\n      - develop\n  pull_request:\n\nconcurrency:\n  group: ignored\n";

    assert_eq!(
        push_branches(source),
        Some(vec!["main".to_owned(), "develop".to_owned()])
    );
}

#[test]
fn event_parser_ignores_same_named_job_outside_on_mapping() {
    let source = "on:\n  pull_request:\n\njobs:\n  push:\n    runs-on: ubuntu-24.04\n";

    assert!(event_block(source, "push").is_none());
    assert!(event_block(source, "pull_request").is_some());
}
