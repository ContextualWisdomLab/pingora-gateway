//! Regression for Bash startup-file injection into routed load evidence.
//!
//! GitHub Actions executes an explicit `shell: bash` step as non-interactive Bash. Bash reads the
//! file named by `BASH_ENV` before the step script, even with `--noprofile --norc`. The existing
//! routed provenance predicate must therefore reject workflow/job/step `BASH_ENV`; otherwise a
//! startup file can predefine functions or command-resolution state before the canonical evidence
//! tail begins.

mod existing_contract {
    include!("load_binary_provenance_contract.rs");

    pub(super) fn admits(source: &str) -> bool {
        routed_binary_provenance_is_fresh(source)
    }

    pub(super) fn routed_tail() -> &'static str {
        ROUTED_PROVENANCE_TAIL
    }
}

#[test]
fn step_level_bash_env_startup_hook_must_not_claim_routed_release_evidence() {
    let source = format!(
        r#"
jobs:
  load-contract:
    runs-on: ubuntu-24.04
    steps:
      - name: Run routed pg-erd loopback traffic
        shell: bash
        env:
          BASH_ENV: /tmp/provenance-startup.sh
        run: |
          set -euo pipefail
          {tail}
"#,
        tail = existing_contract::routed_tail().replace('\n', "\n          ")
    );

    assert!(
        !existing_contract::admits(&source),
        "BASH_ENV is read by non-interactive Bash before the routed script and can pre-poison provenance commands"
    );
}
