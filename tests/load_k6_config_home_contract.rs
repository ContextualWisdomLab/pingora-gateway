//! Fail-closed contract for k6's implicit user configuration precedence.
//!
//! k6 reads its default user `config.json` when no explicit config file is supplied. Script options
//! outrank that lower-precedence config, so routed release evidence must pin config-file options that
//! can make the nominal 4-VU / 400-iteration run materially easier without changing the canonical
//! command. An inherited `rps` cap or `minIterationDuration` can pace requests while the required
//! counts and per-request latency thresholds still pass. The checked-in script therefore pins both
//! pacing controls alongside its VU and iteration counts.

use std::fs;

const ROUTED_SCRIPT: &str = "tests/load/pg_erd_gateway_smoke.js";

fn active_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
}

fn count_exact_line(source: &str, expected: &str) -> usize {
    active_lines(source)
        .filter(|line| *line == expected)
        .count()
}

fn routed_script_pins_lower_precedence_pacing(source: &str) -> bool {
    [
        "vus: 4,",
        "iterations: 400,",
        "rps: 0,",
        "minIterationDuration: '0s',",
    ]
    .iter()
    .all(|expected| count_exact_line(source, expected) == 1)
}

#[test]
fn live_routed_script_pins_lower_precedence_pacing() {
    let source = fs::read_to_string(ROUTED_SCRIPT).expect("routed k6 script should be readable UTF-8");
    assert!(
        routed_script_pins_lower_precedence_pacing(&source),
        "routed release evidence must explicitly override config-file pacing in the checked-in script"
    );
}

#[test]
fn inherited_rps_cap_must_not_remain_effective() {
    let source = r#"
export const options = {
  vus: 4,
  iterations: 400,
  minIterationDuration: '0s',
};
"#;

    assert!(
        !routed_script_pins_lower_precedence_pacing(source),
        "without script-level rps: 0, an inherited config.json can pace a nominal concurrent run to an easier request rate"
    );
}

#[test]
fn inherited_minimum_iteration_duration_must_not_remain_effective() {
    let source = r#"
export const options = {
  vus: 4,
  iterations: 400,
  rps: 0,
};
"#;

    assert!(
        !routed_script_pins_lower_precedence_pacing(source),
        "without script-level minIterationDuration: '0s', inherited config can force each VU to sleep between iterations"
    );
}

#[test]
fn canonical_execution_shape_is_admitted() {
    let source = r#"
export const options = {
  vus: 4,
  iterations: 400,
  rps: 0,
  minIterationDuration: '0s',
};
"#;

    assert!(routed_script_pins_lower_precedence_pacing(source));
}
