//! Semantic regression contract for pull-request job admission in GitHub Actions YAML.
//!
//! The lexical concurrency oracle intentionally requires a narrow repository style, while this
//! companion contract parses YAML semantics so presentation-only comments on `jobs:` cannot hide
//! a direct job from the Draft admission invariant.

use serde_yaml::Value;
use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_ACTIVE_PR_JOB_LINE: &str =
    "github.event_name != 'pull_request' || github.event.pull_request.draft == false";

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

/// Reads one repository workflow as UTF-8 text.
fn read_workflow(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "workflow {} should be readable UTF-8: {error}",
            path.display()
        )
    })
}

/// Returns direct PR job identities whose semantic `if` guard does not exclude Draft PRs.
fn pull_request_jobs_without_draft_guard(source: &str) -> Option<Vec<String>> {
    let document: Value = serde_yaml::from_str(source).unwrap_or_else(|error| {
        panic!("workflow YAML should parse before admission validation: {error}")
    });
    let Some(on) = document.get("on") else {
        return None;
    };
    if on.get("pull_request").is_none() {
        return None;
    }

    let jobs = document
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("pull-request workflow must define a semantic top-level `jobs` mapping");
    assert!(
        !jobs.is_empty(),
        "pull-request workflow must define at least one direct job"
    );

    let mut violations = Vec::new();
    for (job_key, job_definition) in jobs {
        let job_id = job_key
            .as_str()
            .expect("direct workflow job identity must be a string");
        assert!(
            job_definition.as_mapping().is_some(),
            "direct workflow job `{job_id}` must be a mapping"
        );
        let guard = job_definition.get("if").and_then(Value::as_str);
        if guard != Some(EXPECTED_ACTIVE_PR_JOB_LINE) {
            violations.push(job_id.to_owned());
        }
    }
    violations.sort();
    Some(violations)
}

#[test]
fn repository_pull_request_jobs_semantically_exclude_draft_prs() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        let Some(violations) = pull_request_jobs_without_draft_guard(&source) else {
            continue;
        };
        assert!(
            violations.is_empty(),
            "{} has direct PR jobs without the exact Draft admission guard: {violations:?}",
            path.display()
        );
    }
}

#[test]
fn inline_comment_on_jobs_mapping_cannot_hide_an_unguarded_direct_job() {
    let source = format!(
        "on:\n  pull_request:\njobs: # runner-facing work\n  build:\n    runs-on: ubuntu-24.04\n  test:\n    if: {EXPECTED_ACTIVE_PR_JOB_LINE}\n    runs-on: ubuntu-24.04\n"
    );

    assert_eq!(
        pull_request_jobs_without_draft_guard(&source),
        Some(vec!["build".to_owned()])
    );
}

#[test]
fn inline_comment_on_jobs_mapping_still_sees_all_guarded_jobs() {
    let source = format!(
        "on:\n  pull_request:\njobs: # runner-facing work\n  build:\n    if: {EXPECTED_ACTIVE_PR_JOB_LINE}\n    runs-on: ubuntu-24.04\n  test:\n    if: {EXPECTED_ACTIVE_PR_JOB_LINE}\n    runs-on: ubuntu-24.04\n"
    );

    assert_eq!(
        pull_request_jobs_without_draft_guard(&source),
        Some(Vec::new())
    );
}
