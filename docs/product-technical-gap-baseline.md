# Product / Technical Gap Baseline

State captured during PR #1 development on 2026-09-01. The last exact branch head before this documentation batch was `2662e663038111fa6bbeff13bdcedb6ed854255f`; PR #1 was draft, mergeable, based on live `main` `f8b4c99b8e5d3de79af1ff0c00c0c8fd63b52991`, and had no qualifying independent approval. Exact-head CI/SAST/Security runs had not yet produced passing hosted evidence after the latest code change, so nothing in this file marks the runtime releasable.

## Shared runtime

| Area | State | Evidence / gap |
| --- | --- | --- |
| Executable Pingora path | Implemented on branch | Production binary composes `GatewayCommand` -> `GatewayConfig` -> `GatewayProxy` -> `http_proxy_service`; hosted GREEN pending |
| DDD ownership | Implemented | Edge invariants live in `edge_contract`; Pingora types stay in delivery modules; one-upstream v1 invariant was moved out of the adapter |
| Fail-closed config | Implemented | Strict YAML, version/body/upstream/TLS/timeout validation; production process tests exist |
| HTTP/HTTPS upstream | Implemented in adapter | `HttpPeer` verifies cert and hostname for TLS; local TLS integration fixture still missing |
| Hop-by-hop / forwarding trust | Implemented on branch | Pingora standard policy plus explicit deletion of downstream forwarding identity; production-path test added; hosted GREEN pending |
| Request limits | Partial | Body size bounded; Pingora protocol/header bounds are finite; configurable smaller header/concurrency budgets missing |
| Health | Implemented on branch | `/livez` and `/readyz`; readiness does not probe upstream; hosted GREEN pending |
| Graceful drain | Framework lifecycle wired | Repository-specific SIGTERM/in-flight GREEN missing |
| Logs / metrics | Missing | No credential-safe low-cardinality access log/metrics implementation yet |
| OCI | Scaffold only | Dockerfile is non-root/read-only-root-compatible by design; no hosted build/runtime test and no published digest |
| Reproducibility | Blocked | No committed `Cargo.lock` |
| SBOM / provenance | Missing | No protected artifact evidence |
| Benchmark | Missing | No representative Pingora-vs-replaced latency/throughput/CPU/RSS/connection-reuse/TLS evidence; no 20 ms claim permitted |
| Rollback | Documented, not rehearsed | Consumer digest/manifest rollback can only be tested after publication |

## Organization Nginx/OpenResty inventory

Fresh organization search in this run found no `openresty` or literal `ingress-nginx` hits. The following Nginx evidence remains actionable until an owner proves otherwise:

| Repository / path | Classification | Ownership / next evidence |
| --- | --- | --- |
| `scopeweave/Dockerfile`, `scopeweave/infra/nginx/default.conf` | ACTIVE_RUNTIME / ACTIVE_DEPLOYMENT | No enabled scopeweave-specific writer observed; wait for a real shared artifact, then characterize static behavior before migration |
| `inkspan/Dockerfile` | ACTIVE_RUNTIME | No enabled inkspan-specific writer observed; characterize runtime before migration |
| `LineageWeave/frontend/Dockerfile`, `frontend/nginx.conf` | ACTIVE_RUNTIME | More-specific LineageWeave writer enabled; read-only from this loop, advance that owner path |
| `naruon` ingress/live-E2E references | ACTIVE_DEPLOYMENT / TEST_RUNTIME | More-specific naruon writer enabled; read-only from this loop |
| `linux-cluster-ops/scripts/nginx-backup.sh`, Nginx routing/recovery docs | ACTIVE_RUNTIME / CURRENT_OPERATOR_DOC | Must separate reverse-proxy replacement from Certbot/certificate ownership before any change |
| central `.github` scanner fixtures/policy text | NEGATIVE_POLICY_FIXTURE / THIRD_PARTY_TEXT as applicable | Central writer owns it; do not delete legitimate fixtures/history |

No consumer is marked migrated. Protected/default branch must contain the replacement plus runtime/security/deployment/rollback evidence before that classification changes.

## Release blockers in dependency order

1. Obtain exact-head hosted GREEN for build/test/clippy and required security/check paths; fix only verified current-head defects.
2. Commit a reproducible dependency lock and audit the exact resolved graph, including the pinned Pingora revision.
3. Add redacted low-cardinality logs/metrics, graceful SIGTERM/drain integration, local verified-TLS fixture, chunked-over-limit and recovery/concurrency tests.
4. Build and exercise the OCI image as uid/gid 65532 with a read-only root filesystem.
5. Add SBOM/provenance/container-security evidence and publish an immutable image digest under protected release governance.
6. Revalidate Pingora releases/security advisories, obtain required independent review, and merge only an unchanged policy-clean exact head.
7. Characterize and migrate the highest-impact owned consumer without absorbing its product domain semantics.
