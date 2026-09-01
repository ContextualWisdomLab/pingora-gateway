# Product / Technical Gap Baseline

State refreshed during PR #1 development on 2026-09-01. Live protected base is `main@f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`; this PR remains Draft and has no qualifying review. Treat every workflow result as valid only for its exact head. The streamed-body characterization commit `1040580cfa9f4b962a999b84504386c60404a31f` completed repository CI `33487091545` successfully; later documentation commits require their own current-head gates before release readiness can be claimed.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; exact-current-head hosted GREEN remains required after every change |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types stay in delivery/application modules; one-upstream v1 invariant is transport-neutral |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/timeout validation; production process tests exist |
| HTTP/HTTPS upstream | Implemented in adapter | `HttpPeer` verifies certificate and hostname for TLS; local verified-TLS integration fixture is still missing |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard policy plus explicit deletion of downstream forwarding identity; production-path test exists |
| Retry policy | Implemented, intentionally minimal | `runtime_policy` overrides Pingora's framework default and sets `max_retries=1`, which the pinned proxy loop interprets as one total upstream attempt and therefore zero automatic retries; product-specific retry/failover is out of v1 scope |
| Request limits | Partial | Declared and streamed/chunked body size are bounded; compiled-binary evidence rejects an over-limit chunked body and remains ready afterward. Configurable smaller header/concurrency/backpressure budgets are still missing |
| Health | Implemented on branch | `/livez` and `/readyz`; readiness does not probe upstream |
| Graceful drain | Implemented candidate behavior | SIGTERM policy is explicitly bounded to 5 s grace + 10 s per-runtime graceful timeout inside a 30 s external termination budget; real in-flight process test exists. Exact release-candidate success remains mandatory |
| Logs / metrics | Implemented initial vertical | Label-free request/error/body-byte counters and credential/cookie-safe coarse access logging are exercised through the production path; tracing and richer bounded operability remain gaps |
| OCI | Scaffold only | Dockerfile is non-root/read-only-root-compatible by design; no hosted build/runtime test and no published digest |
| Reproducibility | Blocked | Hosted CI generates a lock artifact, but `Cargo.lock` is not committed and image build is not `--locked` |
| SBOM / provenance | Missing | No protected artifact evidence |
| Benchmark | Missing | No representative Pingora-vs-replaced latency/throughput/CPU/RSS/connection-reuse/TLS evidence; no 20 ms p95 claim is permitted |
| Rollback | Documented, not rehearsed | Consumer digest/manifest rollback can only be tested after publication |

## Organization Nginx/OpenResty inventory

Fresh organization search in this run found no actionable literal OpenResty deployment. The following edge evidence remains actionable until an owner proves otherwise:

| Repository / path | Classification | Ownership / next evidence |
| --- | --- | --- |
| `linux-cluster-ops/docs/architecture/nginx-routing-inventory.md` and recovery/backup scripts | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | Current evidence describes host-native Nginx plus static roots, PHP-FPM and Certbot-managed TLS. Separate HTTP edge routing from static/FastCGI hosting, certificate authority and backup/recovery before migration; do not absorb those responsibilities into the shared gateway |
| `pg-erd-cloud/deploy/traefik/dynamic.yaml`, `compose.prod.yaml` | ACTIVE_DEPLOYMENT / PLAUSIBLE_CONSUMER | Traefik v3.5.4 currently owns precedence for `/healthz` -> backend, `/api*` -> backend and `/` -> SPA plus response-security headers. Pingora v1 has one upstream and no route table, so parity is RED until routing/multiple-upstream behavior is explicitly modeled and characterized by the repository owner |
| `scopeweave/Dockerfile`, `scopeweave/infra/nginx/default.conf` | ACTIVE_RUNTIME / ACTIVE_DEPLOYMENT | Static-content serving is not automatically a shared reverse-proxy responsibility; characterize behavior and ownership before migration |
| `inkspan/Dockerfile` | ACTIVE_RUNTIME | Nginx serves the built demo bundle; characterize static-server semantics before deciding whether Pingora or another managed static boundary is correct |
| `LineageWeave/frontend/Dockerfile`, `frontend/nginx.conf` | ACTIVE_RUNTIME | More-specific LineageWeave writer owns mutations; read-only from this loop |
| `naruon` ingress/live-E2E references | ACTIVE_DEPLOYMENT / TEST_RUNTIME | More-specific naruon writer owns mutations; read-only from this loop |
| central `.github` scanner fixtures/policy text | NEGATIVE_POLICY_FIXTURE / THIRD_PARTY_TEXT as applicable | Central writer owns it; do not delete legitimate fixtures/history |

No consumer is marked migrated. A replacement requires executable traffic parity plus canary/shadow, rollback, security and protected deployment evidence before legacy removal.

## Context Fabric / EA dependency

`context-graph-contracts` and `enterprise-architecture-core` remain read-only to this writer. Fresh repository metadata still reports `develop` as default for both while their accepted owner path is repairing a protected-`main` transition; this writer does not mutate either repository or treat remembered branch topology as authoritative.

The current Context Graph tail is Draft PR #21 on actual head `a3a3125619ed6e777818811b1c0b97f3a4574b73`, base `#20@b5397e0e9e0184105250046a19be02c422644081`. Its current hosted conformance/package/reproducibility runs are queued, and it packages the Context Assertion event semantic profile on top of the structured CloudEvent-envelope repair. This is useful candidate evidence, not an immutable dependency. Edge migration remains RED until a protected release provides the complete bundle, exact package/provenance identity, Context Assertion plus structured CloudEvent semantics, and admission/conformance evidence.

The current EA consumer-mapping tail is Draft PR #40 on `2b14e008a11712c840d0bf6c8c5d3a1d6e9ec1ba`, stacked on `#39@f6ed5b0c565975927c5dac558b89d0efca8ed9fa`; its current CI/runtime-readiness/supply-chain runs are queued. Its intended fail-closed boundary is correct: Context Fabric projections bind the single Context Graph dependency manifest, preserve canonical/source references, truth status, effective/system time and provenance, and reject a `provisional-pr-head` as a released contract. The EA owner path must eventually project `current technology/interface -> migration initiative/scenario -> target technology/interface -> validated execution` from immutable released contract identity. Runtime requests, logs and customer data do not belong in Context Graph/EA authoritative tables, and cross-service application-table SQL remains prohibited.

For an edge migration, GREEN therefore requires the EA owner path to record the affected application/service/API, current edge technology/provider/version and interface behavior, migration initiative/scenario, target immutable Pingora artifact/version/digest, lifecycle/security/operability risk, accountable owner, and canary/cutover/rollback/validated-execution state, all with released Context Graph provenance rather than copied runtime data.

## Release blockers in dependency order

1. Obtain exact-current-head hosted GREEN for build/test/clippy and every required security/check path; fix only verified current-head defects.
2. Commit a reproducible dependency lock, switch build/CI/container paths to locked resolution, and audit the exact resolved graph including the pinned Pingora revision.
3. Add verified-TLS hostname-failure, realistic concurrency/backpressure/load, broader upstream/network failure recovery and applicable fuzz/property evidence. Streamed/chunked over-limit rejection, post-rejection readiness recovery and in-flight SIGTERM drain are now characterized in compiled-process tests.
4. Build and exercise the OCI image as uid/gid 65532 with a read-only root filesystem and least-privilege runtime options.
5. Add SBOM/provenance/container-security evidence and publish an immutable image digest under protected release governance.
6. Benchmark representative owned gateway traffic before deciding whether 20 ms p95 is a realistic edge-path target; report the measured bottleneck rather than forcing an unsupported SLO.
7. Revalidate Pingora/Rust/IETF/HTTP/TLS/OCI security standards, obtain then-required review evidence, and merge only an unchanged policy-clean exact head.
8. Characterize and migrate the highest-impact owned reverse-proxy/edge consumer whose bounded responsibility actually matches the shared runtime. `pg-erd-cloud` is a plausible future consumer but is not parity-ready against one-upstream v1; keep static hosting, certificate authority and product routing with their canonical owners when they do not belong in the shared boundary.
