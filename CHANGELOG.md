# Changelog

All notable changes are tracked here. No release has been published yet.

## Unreleased

- Bootstrapped an executable Rust Pingora proxy through pull-request governance.
- Added strict v1 configuration, explicit one-upstream network authority, TLS identity verification, and explicit upstream I/O budgets.
- Added mandatory positive `max_in_flight_requests` and `upstream_keepalive_pool_size` capacity budgets; Pingora's framework keepalive default is overridden from the validated edge contract.
- Added process-local fail-fast backpressure: non-health requests above the in-flight budget receive HTTP 503, health remains observable, rejection telemetry increments, and capacity is released after request completion or failure.
- Added optional per-upstream absolute PEM trust-bundle consumption without taking ownership of certificate issuance/rotation; trust material is loaded fail-closed before listeners open.
- Added an executable local-CA TLS test through the compiled gateway that holds CA trust constant and proves SNI/hostname mismatch is rejected.
- Added a focused transport-adapter regression proving an upstream without a custom trust bundle leaves Pingora's platform trust roots selected rather than replacing the CA store.
- Added fail-closed binary startup and real loopback production-path tests, including held-request saturation/recovery at an in-flight budget of one.
- Added `/livez` and `/readyz` through the Pingora serving path.
- Added request-body limits and a distrust-by-default forwarded-header policy.
- Added low-cardinality metrics plus credential/cookie-safe access logging through the production path.
- Overrode Pingora framework retry/drain defaults with one total upstream attempt, a 5-second SIGTERM grace period, and a 10-second per-runtime graceful-shutdown timeout inside a 30-second external termination budget.
- Added non-root/read-only-root OCI packaging on digest-pinned distroless `base-nossl-debian13:nonroot`, retaining only the gateway binary and required `libgcc_s.so.1` runtime library.
- Added a committed dependency lock, fail-closed license/source/advisory policy, exact-source SBOM and image-vulnerability evidence.
- Added an exact-head owned-production coverage gate that requires 100% lines and regions without filename/function/branch exclusions; repaired compiler-generated generic startup coverage and structurally impossible literal-header error regions rather than weakening the gate.
- Added missing-public-rustdoc enforcement and documentation builds with warnings denied.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.
- Refreshed the migration gap baseline, TRD and primary-source traceability against the current runtime/config/OCI contract and 2026-09-05 live dependency state; Rust 1.98.1 is tracked as a separately gated release-path repair rather than being claimed as already inherited by the foundation.

Release remains blocked until the then-current exact head proves all live repository/security/review gates, including the Rust 1.98.1 compiler prerequisite, workflow-admission RED→GREEN, fail-closed removal of unmaintained `derivative 2.2.0` through an immutable maintainer-integrated supplier revision, and applicable H2→H1 Cookie/body-framing supplier contracts. It also requires representative consumer-specific concurrency/origin-capacity/network-failure and benchmark evidence, an immutable registry digest with SBOM/provenance/reproducibility, and rehearsed rollback. Actual Nginx cutover additionally requires the operations owner to complete safe archive extraction and separate certificate, edge-runtime, and application/FastCGI responsibilities. No consumer migration, canary, cutover, or legacy removal is claimed before those release and traffic-contract gates are satisfied.