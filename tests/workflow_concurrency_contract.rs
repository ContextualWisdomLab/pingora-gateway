//! Repository-level contract for GitHub Actions admission and coalescing semantics.
//!
//! Pull-request runs may cancel only an older run for the same workflow, repository, and PR.
//! Workflows that also run on pushes must keep that duplicate-evidence path on protected main;
//! push-only release/tag workflows remain outside this PR-coalescing contract.

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

/// Requires the event declaration shape understood by this fail-closed repository contract.
fn assert_block_style_on_mapping(path: &Path, source: &str) {
    let lines = source.lines().collect::<Vec<_>>();
    let top_level_on = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| indentation(line) == 0 && line.trim_start().starts_with("on:"))
        .collect::<Vec<_>>();

    assert_eq!(
        top_level_on.len(),
        1,
        "{} must define exactly one top-level `on:` key",
        path.display()
    );
    let (on_index, on_line) = top_level_on[0];
    assert_eq!(
        *on_line,
        "on:",
        "{} must use a block-style top-level `on:` mapping so admission checks cannot skip inline/alternate event syntax",
        path.display()
    );

    let mut saw_direct_event = false;
    for line in lines.iter().skip(on_index + 1) {
        let trimmed = line.trim();
        let ignorable = trimmed.is_empty() || trimmed.starts_with('#');
        if !ignorable && indentation(line) == 0 {
            break;
        }
        if ignorable {
            continue;
        }

        if indentation(line) == 2 {
            let direct_event_key = line
                .strip_prefix("  ")
                .expect("two-space indentation must have a two-space prefix");
            let canonical_event_key = direct_event_key
                .strip_suffix(':')
                .is_some_and(|event_name| {
                    !event_name.is_empty()
                        && event_name
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
                });
            assert!(
                !direct_event_key.starts_with('-') && canonical_event_key,
                "{} must declare direct children of `on:` as canonical block mapping event keys without alternate YAML spelling",
                path.display()
            );
            saw_direct_event = true;
            continue;
        }

        assert!(
            saw_direct_event && indentation(line) > 2,
            "{} must begin the `on:` mapping with a two-space direct event key; alternate direct-child indentation can bypass event detection",
            path.display()
        );
    }

    assert!(
        saw_direct_event,
        "{} must declare at least one two-space direct event key under `on:`",
        path.display()
    );
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
fn repository_workflows_keep_fail_closed_event_syntax() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        assert_block_style_on_mapping(&path, &source);
    }
}

#[test]
fn pull_request_workflows_use_pr_scoped_cancellation_identity() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        assert_block_style_on_mapping(&path, &source);
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
fn pull_request_workflows_with_push_are_limited_to_protected_main() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        assert_block_style_on_mapping(&path, &source);
        if event_block(&source, "pull_request").is_none() || event_block(&source, "push").is_none() {
            continue;
        }

        assert_eq!(
            push_branches(&source),
            Some(vec!["main".to_owned()]),
            "{} must admit duplicate push evidence for protected main and no feature-branch pattern",
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

#[test]
#[should_panic(expected = "block-style top-level `on:` mapping")]
fn inline_on_syntax_cannot_bypass_event_contract() {
    let path = Path::new("synthetic-inline-workflow.yml");
    assert_block_style_on_mapping(path, "on: [push, pull_request]\njobs:\n");
}

#[test]
#[should_panic(expected = "canonical block mapping event keys")]
fn sequence_on_syntax_cannot_bypass_event_contract() {
    let path = Path::new("synthetic-sequence-workflow.yml");
    assert_block_style_on_mapping(path, "on:\n  - push\n  - pull_request\njobs:\n");
}

#[test]
#[should_panic(expected = "two-space direct event key")]
fn alternate_direct_event_indentation_cannot_bypass_event_contract() {
    let path = Path::new("synthetic-four-space-workflow.yml");
    assert_block_style_on_mapping(path, "on:\n    push:\n    pull_request:\njobs:\n");
}

#[test]
#[should_panic(expected = "canonical block mapping event keys")]
fn alternate_direct_event_key_spelling_cannot_bypass_event_contract() {
    let path = Path::new("synthetic-spaced-event-key-workflow.yml");
    assert_block_style_on_mapping(path, "on:\n  push :\n  pull_request :\njobs:\n");
}

#[test]
#[should_panic(expected = "canonical block mapping event keys")]
fn trailing_whitespace_after_direct_event_colon_cannot_bypass_event_contract() {
    let path = Path::new("synthetic-trailing-event-space-workflow.yml");
    assert_block_style_on_mapping(path, "on:\n  push:   \n  pull_request:\njobs:\n");
}
