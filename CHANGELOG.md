# Changelog

All notable changes are tracked here. No release has been published yet.

## Unreleased

- Bootstrapped an executable Rust Pingora proxy through pull-request governance.
- Added strict v1 configuration, explicit one-upstream network authority, TLS identity verification, and explicit upstream I/O budgets.
- Added fail-closed binary startup and real loopback production-path tests.
- Added `/livez` and `/readyz` through the Pingora serving path.
- Added request-body limits and a distrust-by-default forwarded-header policy.
- Added non-root/read-only-root-compatible OCI packaging scaffold.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.

Release remains blocked on exact-head CI/security/review evidence, dependency lock/reproducibility, metrics/redacted logging, graceful-drain and TLS integration tests, OCI runtime verification, SBOM/provenance, immutable image digest, and protected-branch integration.
