//! Repository-level contract for GitHub Actions admission and coalescing semantics.
//!
//! Pull-request runs may cancel only an older run for the same workflow, repository, and PR.
//! Push runs are protected-main evidence and must not share that cancellation identity.

use std::fs;
use std::path::{Path, PathBuf};

const EXPECTED_GROUP: &str = "group: ${{ github.workflow }}-${{ github.repository }}-${{ github.event.pull_request.number || github.run_id }}";
const EXPECTED_CANCEL: &str =
    "cancel-in-progress: ${{ github.event_name == 'pull_request' }}";
const EXPECTED_MAIN_PUSH: &str = "push:\n    branches:\n      - main";

/// Returns all repository-owned YAML workflows in deterministic path order.
fn workflow_paths() -> Vec<PathBuf> {
    let mut paths = fs::read_dir(".github/workflows")
        .expect("workflow directory should exist")
        .map(|entry| entry.expect("workflow entry should be readable").path())
        .filter(|path| matches!(path.extension().and_then(|ext| ext.to_str()), Some("yml" | "yaml")))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

/// Reads one workflow as UTF-8 because GitHub workflow sources are text contracts.
fn read_workflow(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("workflow {} should be readable UTF-8: {error}", path.display())
    })
}

#[test]
fn pull_request_workflows_use_pr_scoped_cancellation_identity() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        if !source.contains("pull_request:") {
            continue;
        }

        assert!(
            source.contains(EXPECTED_GROUP),
            "{} must scope concurrency to workflow/repository/PR identity",
            path.display()
        );
        assert!(
            source.contains(EXPECTED_CANCEL),
            "{} must cancel only superseded pull-request runs",
            path.display()
        );
    }
}

#[test]
fn push_workflows_are_limited_to_protected_main() {
    for path in workflow_paths() {
        let source = read_workflow(&path);
        if !source.contains("push:") {
            continue;
        }

        assert!(
            source.contains(EXPECTED_MAIN_PUSH),
            "{} must not duplicate feature-branch push and pull-request evidence",
            path.display()
        );
    }
}
