//! Fail-closed contract for k6's implicit user configuration precedence.
//!
//! k6 reads its default user `config.json` when no explicit config file is supplied. Script options
//! outrank that lower-precedence config, so routed release evidence must pin every config-file option
//! that could make the nominal 4-VU / 400-iteration run materially easier without changing the
//! canonical command. In particular, an inherited `rps` cap or `minIterationDuration` can serialize
//! or pace requests while all count and latency thresholds still pass, and an inherited `hosts`
//! mapping can retarget a named origin. The checked-in script therefore pins those execution-shaping
//! options alongside its VU and iteration counts.

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

fn routed_script_pins_lower_precedence_execution_shape(source: &str) -> bool {
    [
        "vus: 4,",
        "iterations: 400,",
        "rps: 0,",
        "minIterationDuration: '0s',",
        "hosts: {},",
    ]
    .iter()
    .all(|expected| count_exact_line(source, expected) == 1)
}

#[test]
fn live_routed_script_pins_lower_precedence_execution_shape() {
    let source = fs::read_to_string(ROUTED_SCRIPT).expect("routed k6 script should be readable UTF-8");
    assert!(
        routed_script_pins_lower_precedence_execution_shape(&source),
        "routed release evidence must explicitly override config-file pacing and host mappings in the checked-in script"
    );
}

#[test]
fn inherited_rps_cap_must_not_remain_effective() {
    let source = r#"
export const options = {
  vus: 4,
  iterations: 400,
  minIterationDuration: '0s',
  hosts: {},
};
"#;

    assert!(
        !routed_script_pins_lower_precedence_execution_shape(source),
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
  hosts: {},
};
"#;

    assert!(
        !routed_script_pins_lower_precedence_execution_shape(source),
        "without script-level minIterationDuration: '0s', inherited config can force each VU to sleep between iterations"
    );
}

#[test]
fn inherited_host_mapping_must_not_remain_effective() {
    let source = r#"
export const options = {
  vus: 4,
  iterations: 400,
  rps: 0,
  minIterationDuration: '0s',
};
"#;

    assert!(
        !routed_script_pins_lower_precedence_execution_shape(source),
        "without an explicit empty hosts map, lower-precedence user config can retain host rewrites"
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
  hosts: {},
};
"#;

    assert!(routed_script_pins_lower_precedence_execution_shape(source));
}
