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
- Added non-root/read-only-root-compatible OCI packaging scaffold.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.

Release remains blocked on exact-head CI/security/review evidence, a committed dependency lock and reproducibility audit, in-flight graceful-drain and verified-TLS integration tests, chunked/concurrency/failure coverage, OCI runtime verification, SBOM/provenance, immutable image digest/rollback evidence, benchmark evidence, and protected-branch integration.
