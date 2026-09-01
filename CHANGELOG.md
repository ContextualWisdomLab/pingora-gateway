# Changelog

All notable changes are tracked here. No release has been published yet.

## Unreleased

- Bootstrapped an executable Rust Pingora proxy through pull-request governance.
- Added strict v1 configuration, explicit one-upstream network authority, TLS identity verification, and explicit upstream I/O budgets.
- Added fail-closed binary startup and real loopback production-path tests.
- Added `/livez` and `/readyz` through the Pingora serving path.
- Added request-body limits and a distrust-by-default forwarded-header policy.
- Added low-cardinality metrics plus credential/cookie-safe access logging through the production path.
- Overrode Pingora framework retry/drain defaults with one total upstream attempt, a 5-second SIGTERM grace period, and a 30-second graceful-shutdown timeout.
- Added non-root/read-only-root OCI packaging and executable least-privilege runtime verification.
- Added a committed dependency lock, fail-closed license/source/advisory policy, exact-source SBOM and image-vulnerability evidence.
- Added an exact-head owned-production coverage gate that requires 100% lines and regions without filename/function/branch exclusions; repaired compiler-generated generic startup coverage and structurally impossible literal-header error regions rather than weakening the gate.
- Added missing-public-rustdoc enforcement and documentation builds with warnings denied.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.

Release remains blocked on the organization decision for the exact Pingora release versus `RUSTSEC-2026-0253` (`ContextualWisdomLab/.github#1605`), exact-current-head CI/supply-chain/security/review evidence after the final source mutation, verified local-CA TLS integration including hostname failure, representative concurrency/backpressure/network-failure and benchmark evidence, an immutable registry digest with provenance and rehearsed rollback, and protected-branch integration. No consumer migration, canary, cutover, or legacy removal is claimed before those release and traffic-contract gates are satisfied.
