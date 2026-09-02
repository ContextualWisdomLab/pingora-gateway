# Changelog

All notable changes are tracked here. No release has been published yet.

## Unreleased

- Bootstrapped an executable Rust Pingora proxy through pull-request governance.
- Added strict v1 configuration, explicit one-upstream network authority, TLS identity verification, and explicit upstream I/O budgets.
- Added a transport-neutral Edge Routing characterization for the live `pg-erd-cloud` Traefik contract: exact `/healthz`, raw-prefix `/api`, fallback `/`, explicit numeric precedence, and fail-closed ambiguous-priority/malformed-route rejection. This does not yet activate multi-upstream production traffic or claim traffic cutover.
- Added a separate transport-neutral HTTP Policy characterization for the live `pg-erd-cloud` Traefik response-security middleware: exact `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, and `Permissions-Policy` values, ASCII case-insensitive field identity, duplicate authority rejection, and fail-closed CR/LF values.
- Added a transport-neutral `EdgeMigrationPlan` that composes the characterized route and HTTP-policy contracts with an explicit normalized upstream-authority set. The pg-erd-cloud plan admits only `backend` and `frontend`, rejects undeclared route targets, and remains pre-runtime evidence rather than shadow/canary, cutover, or legacy-removal proof.
- Added `MigrationDeliveryPlan` to bind every characterized migration upstream identity to exactly one explicit, prevalidated Pingora `HttpPeer`; missing, duplicate, and undeclared concrete transport authority fails closed. This does not widen `GatewayConfig` v1 or activate multi-route traffic in the production startup path.
- Added `MigrationGatewayProxy` as a pre-listener Pingora callback adapter over the characterized route/header/peer contracts. It rejects unmatched routes, enforces shared body/concurrency isolation, and applies only the characterized response policy. Production startup still selects the single-upstream `GatewayProxy`; no shadow/canary/cutover is claimed.
- Added a separate Ingress forwarding-policy boundary for the pg-erd migration. Request-controlled `Forwarded`, `X-Forwarded-*` and `X-Real-IP` values are removed, then the compatibility `X-Forwarded-For`, `X-Real-IP`, `X-Forwarded-Host`, `X-Forwarded-Port` and `X-Forwarded-Proto` fields are rebuilt from accepted transport metadata. The current characterized Traefik `web` entryPoint remains explicitly HTTP; TLS-derived scheme behavior is not claimed before a TLS listener contract exists.
- Added a shared `observability` bounded context so both `GatewayProxy` and `MigrationGatewayProxy` use the same low-cardinality request/error/body/backpressure counters and coarse access-log shape instead of duplicating telemetry. The public observation vocabulary contains only response status, `ok`/`error`, and observed request-body bytes; paths, query strings, headers/cookies, credentials, customer payloads and product identifiers stay out of the shared telemetry contract.
- Added mandatory positive `max_in_flight_requests` and `upstream_keepalive_pool_size` capacity budgets; Pingora's framework keepalive default is overridden from the validated edge contract.
- Added process-local fail-fast backpressure: non-health requests above the in-flight budget receive HTTP 503, health remains observable, rejection telemetry increments, and capacity is released after request completion or failure.
- Added optional per-upstream absolute PEM trust-bundle consumption without taking ownership of certificate issuance/rotation; trust material is loaded fail-closed before listeners open.
- Added an executable local-CA TLS test through the compiled gateway that holds CA trust constant and proves SNI/hostname mismatch is rejected.
- Added a focused transport-adapter regression proving an upstream without a custom trust bundle leaves Pingora's platform trust roots selected rather than replacing the CA store.
- Added fail-closed binary startup and real loopback production-path tests, including held-request saturation/recovery at an in-flight budget of one.
- Added `/livez` and `/readyz` through the Pingora serving path.
- Added request-body limits and a distrust-by-default forwarded-header policy.
- Added low-cardinality metrics plus credential/cookie-safe access logging through the production path.
- Overrode Pingora framework retry/drain defaults with one total upstream attempt, a 5-second SIGTERM grace period, and a 30-second graceful-shutdown timeout.
- Added non-root/read-only-root OCI packaging and executable least-privilege runtime verification.
- Added a committed dependency lock, fail-closed license/source/advisory policy, exact-source SBOM and image-vulnerability evidence.
- Added an exact-head owned-production coverage gate that requires 100% lines and regions without filename/function/branch exclusions; repaired compiler-generated generic startup coverage and structurally impossible literal-header error regions rather than weakening the gate.
- Added missing-public-rustdoc enforcement and documentation builds with warnings denied.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.

Release remains blocked on the organization decision for the exact Pingora release versus `RUSTSEC-2026-0253` and the separate time-bounded disposition of unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388` (`ContextualWisdomLab/.github#1605`), restoration of authoritative public non-fork Dependency Review evidence (`ContextualWisdomLab/.github#810`), terminal exact-current-head CI/supply-chain/security/review evidence, representative consumer-specific concurrency/origin-capacity/network-failure and benchmark evidence, an immutable registry digest with provenance and rehearsed rollback, and protected-branch integration. No consumer migration, canary, cutover, or legacy removal is claimed before those release and traffic-contract gates are satisfied.
