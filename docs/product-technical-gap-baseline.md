# Product / Technical Gap Baseline

State refreshed during PR #1 development on 2026-09-01. Live protected base is `main@f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; this PR remains Draft and has no qualifying review. Treat every workflow result as valid only for its exact current head.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; exact-current-head hosted GREEN remains required after every change |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types stay in delivery/application modules; one-upstream v1 invariant is transport-neutral |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/timeout validation; production process tests exist |
| HTTP/HTTPS upstream | Implemented in adapter | `HttpPeer` verifies certificate and hostname for TLS; local verified-TLS integration fixture is still missing |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard policy plus explicit deletion of downstream forwarding identity; production-path test exists |
| Retry policy | Implemented, intentionally minimal | `runtime_policy` overrides Pingora's framework default and sets `max_retries=1`, which the pinned proxy loop interprets as one total upstream attempt and therefore zero automatic retries; product-specific retry/failover is out of v1 scope |
| Request limits | Partial | Body size bounded; configurable smaller header/concurrency/backpressure budgets are still missing |
| Health | Implemented on branch | `/livez` and `/readyz`; readiness does not probe upstream |
| Graceful drain | Partial | SIGTERM policy is explicitly bounded to 5 s grace + 30 s runtime-shutdown timeout; repository-specific in-flight SIGTERM GREEN is still missing |
| Logs / metrics | Implemented initial vertical | Label-free request/error/body-byte counters and credential/cookie-safe coarse access logging are exercised through the production path; tracing and richer bounded operability remain gaps |
| OCI | Scaffold only | Dockerfile is non-root/read-only-root-compatible by design; no hosted build/runtime test and no published digest |
| Reproducibility | Blocked | Hosted CI generates a lock artifact, but `Cargo.lock` is not committed and image build is not `--locked` |
| SBOM / provenance | Missing | No protected artifact evidence |
| Benchmark | Missing | No representative Pingora-vs-replaced latency/throughput/CPU/RSS/connection-reuse/TLS evidence; no 20 ms p95 claim is permitted |
| Rollback | Documented, not rehearsed | Consumer digest/manifest rollback can only be tested after publication |

## Organization Nginx/OpenResty inventory

Fresh organization search in this run found no actionable OpenResty hit. The following Nginx evidence remains actionable until an owner proves otherwise:

| Repository / path | Classification | Ownership / next evidence |
| --- | --- | --- |
| `scopeweave/Dockerfile`, `scopeweave/infra/nginx/default.conf` | ACTIVE_RUNTIME / ACTIVE_DEPLOYMENT | Static-content serving is not automatically a shared reverse-proxy responsibility; characterize behavior and ownership before migration |
| `inkspan/Dockerfile` | ACTIVE_RUNTIME | Nginx serves the built demo bundle; characterize static-server semantics before deciding whether Pingora or another managed static boundary is correct |
| `LineageWeave/frontend/Dockerfile`, `frontend/nginx.conf` | ACTIVE_RUNTIME | More-specific LineageWeave writer owns mutations; read-only from this loop |
| `naruon` ingress/live-E2E references | ACTIVE_DEPLOYMENT / TEST_RUNTIME | More-specific naruon writer owns mutations; read-only from this loop |
| `linux-cluster-ops/scripts/nginx-backup.sh`, Nginx routing/recovery docs | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | Separate reverse-proxy replacement from Certbot/certificate authority and backup/recovery ownership before any change |
| central `.github` scanner fixtures/policy text | NEGATIVE_POLICY_FIXTURE / THIRD_PARTY_TEXT as applicable | Central writer owns it; do not delete legitimate fixtures/history |

No consumer is marked migrated. A replacement requires executable traffic parity plus canary/shadow, rollback, security and protected deployment evidence before legacy removal.

## Context Fabric / EA dependency

`context-graph-contracts` and `enterprise-architecture-core` remain read-only to this writer. Their live metadata still reports `develop` as default while the accepted Context Fabric owner path is repairing the protected-`main` integration transition through central governance. Edge migration must not bind an open Context Graph PR head as a contract dependency. Until an immutable released contract exists, EA projection is RED: a migration cannot claim authoritative `current technology/interface -> initiative/scenario -> target technology/interface -> validated execution` evidence. GREEN requires a released complete-bundle Context Assertion/CloudEvent/API profile, admission/conformance proof, and an EA owner implementation that stores canonical refs/provenance rather than runtime request/log/customer data.

## Release blockers in dependency order

1. Obtain exact-head hosted GREEN for build/test/clippy and required security/check paths; fix only verified current-head defects.
2. Commit a reproducible dependency lock, switch build/CI paths to locked resolution, and audit the exact resolved graph including the pinned Pingora revision.
3. Add real in-flight SIGTERM/drain, verified-TLS hostname-failure, chunked-over-limit, recovery/concurrency/backpressure and applicable fuzz/property evidence.
4. Build and exercise the OCI image as uid/gid 65532 with a read-only root filesystem.
5. Add SBOM/provenance/container-security evidence and publish an immutable image digest under protected release governance.
6. Benchmark representative owned gateway traffic before deciding whether 20 ms p95 is a realistic edge-path target; report the measured bottleneck rather than forcing an unsupported SLO.
7. Revalidate Pingora/Rust/security standards, obtain then-required review evidence, and merge only an unchanged policy-clean exact head.
8. Characterize and migrate the highest-impact owned reverse-proxy/edge consumer whose bounded responsibility actually matches the shared runtime; keep static hosting, certificate authority and product routing with their canonical owners when they do not.
