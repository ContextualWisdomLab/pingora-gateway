# Changelog

All notable changes are tracked here. No release has been published yet.

## Unreleased

- Bootstrapped an executable Rust Pingora proxy through pull-request governance.
- Added strict v1 configuration, explicit one-upstream network authority, TLS identity verification, and explicit upstream I/O budgets.
- Added a transport-neutral Edge Routing characterization for the live `pg-erd-cloud` Traefik contract: exact `/healthz`, raw-prefix `/api`, fallback `/`, explicit numeric precedence, and fail-closed ambiguous-priority/malformed-route rejection. This does not by itself claim traffic cutover.
- Added a separate transport-neutral HTTP Policy characterization for the live `pg-erd-cloud` Traefik response-security middleware: exact `X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`, and `Permissions-Policy` values, ASCII case-insensitive field identity, duplicate authority rejection, and RFC 9110 field-value admission that rejects invalid controls plus leading/trailing SP or HTAB while preserving valid interior whitespace.
- Added a transport-neutral `EdgeMigrationPlan` application composition over the characterized route and HTTP-policy contracts plus an explicit normalized upstream-authority set. The pg-erd-cloud plan admits only `backend` and `frontend`, rejects undeclared route targets, and does not create product-domain or service-discovery authority.
- Added `MigrationDeliveryPlan` to bind every characterized migration upstream identity to exactly one explicit, prevalidated Pingora `HttpPeer`; missing, duplicate, and undeclared concrete transport authority fails closed. This does not widen generic `GatewayConfig` v1.
- Added `MigrationGatewayProxy` over characterized routing/HTTP policy, explicit peer binding, runtime isolation, transport-derived forwarding trust, failure-response policy, and shared observability. It selects only prevalidated peers, rejects unmatched routes, and enforces body/in-flight budgets.
- Added bounded `PgErdMigrationConfig` and a separate `cwl-pingora-pg-erd-migration` composition root. Operators may configure listener/metrics sockets, non-zero runtime/keepalive budgets, and concrete `backend`/`frontend` transport/TLS values, but routes, response-policy fields, product auth/business logic, service discovery, and new migration authorities remain non-configurable. Generic `GatewayConfig` v1 and `cwl-pingora-gateway` remain unchanged.
- Kept migration admin parsing side-effect free for custom TLS trust material: deterministic authority validation happens during parse, while peer/trust-bundle materialization occurs once during `build_proxy` before listener creation, avoiding a validate-then-reload trust-file window.
- Added a dedicated compiled pg-erd listener contract: `/livez` and `/readyz` remain process-local, consumer `/healthz` and raw `/api` prefix traffic follow the characterized origins, response policy is replaced at the edge, forwarding identity is rebuilt from accepted client transport plus Host/scheme authority, and declared oversize bodies fail before origin delivery. This is executable candidate evidence, not canary/cutover evidence.
- Added an Ingress Forwarding Policy that removes request-controlled `Forwarded`, `X-Forwarded-*`, and `X-Real-IP` authority before rebuilding only the characterized pg-erd compatibility fields from accepted client transport and validated Host/scheme authority. The listener bind socket is not treated as external-port truth. The current clear-text `web` characterization emits `http`; HTTPS remains a separate listener/TLS contract.
- Added shared payload-free observability for both Pingora adapters: low-cardinality completion status/outcome/body-byte facts plus backpressure counters, with paths, query strings, headers, cookies, credentials, product identifiers, and customer payloads excluded.
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
- Added a load-workflow contract that proves the measured loopback origin is ready before gateway startup so fixture races cannot be counted as gateway latency or availability behavior.
- Added DDD, product, technical, security, threat, test, operability, configuration, migration-gap, and primary-source traceability documentation.

Release remains blocked on a maintainer-integrated and release-qualified disposition of unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388`, the exact Pingora supplier/protocol gates tracked by the foundation stack, the Rust compiler promotion owned by #56, terminal exact-current CI/supply-chain/security/review evidence, central required-workflow convergence and independent approval, representative pg-erd routed concurrency/origin-capacity/network-failure and benchmark evidence, an immutable package/image identity with SBOM/provenance/reproducibility and rehearsed rollback, and protected-branch integration. No consumer migration, shadow/canary, cutover, or legacy removal is claimed before those release and traffic-contract gates are satisfied.
