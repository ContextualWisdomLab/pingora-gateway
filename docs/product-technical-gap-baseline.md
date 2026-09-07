# Product / Technical Gap Baseline

This file is the code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. Live PR metadata remains the exact-head authority; this document intentionally does not self-hash its own containing commit. Historical RED/GREEN detail remains in PRs, commits, workflow receipts, ADRs and `CHANGELOG.md` rather than being copied indefinitely into this snapshot.

## Authority and bounded contexts

`pingora-gateway` owns shared Ingress, Edge Routing, TLS consumption, HTTP Policy, Load Balancing mechanics, Observability, Admin Config and Runtime Isolation behavior that is genuinely edge/runtime responsibility. Product authentication and business logic remain product-owned. Keyverse remains identity authority; Wardnet/EgressWeave remain their policy authorities. The gateway consumes released/versioned contracts or explicit operator transport inputs and does not copy sibling source, issue cross-service SQL, or depend on mutable sibling PR heads.

Generic v1 remains a one-upstream Rust/Pingora process. The bounded `cwl-pingora-pg-erd-migration` process is a separate composition root for the characterized `pg-erd-cloud` route/header contract; it is not a general product-routing DSL.

## Dependency and promotion root

Foundation #1 remains `0da81a93f93e869c15bb7d34c55fc87479d16522`. Compiler prerequisite #56 remains Ready/open/mergeable at exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2`; CI `33992794787` and Supply Chain `33992794799` are terminal GREEN, but an independent `APPROVED` review is still required before protected promotion. Bot/static review is not substituted for that governance gate.

Supplier-intake #54 remains Ready/open/mergeable at exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06`. Its hosted RED is intentional: committed `Cargo.lock` still contains `derivative 2.2.0`, and `tests/supply_chain_policy.rs` rejects that exact package under the CWL release criterion. Audit ignores, lock deletion, mutable supplier pins, muted regressions and scanner suppression are not admissible repairs.

Supplier-semantics #62 remains Ready/open/mergeable at exact `32e0aeedac7b0fe6234d476245f37994b1b9168f`, with exact CI `34051494660` and Supply Chain `34051494627` terminal GREEN. Its Rust measured-origin receipt records 400 requests, 800/800 status/body checks, zero HTTP failures and p95 `0.71593235 ms`; it also preserves the required `PeerOptions` Debug and `Backend` equality/hash/order semantics that an upstream repair must not regress.

Fresh supplier authority remains protected `cloudflare/pingora/main@09696b51bc59315353d96686355861604d0bb48c`. The accepted next step is a maintainer-integrated, release-qualified immutable Pingora repair removing `derivative` from the relevant workspace/core/load-balancing manifests and regenerated lock while preserving #62 semantics. Gateway promotion then requires an exact supplier bump, committed lock regeneration, unchanged #54 GREEN, preserved #62 GREEN, and #56 independent approval/governance.

## Current pg-erd stack

Parent #12 is exact `69f22265cd88881b8e14cedb32defa0a91180aa2` and is terminal hosted GREEN: CI `34161291459` and Supply Chain `34161291354` passed formatting, compile/test, strict Clippy, warning-denied rustdoc, 100% owned-production line/region coverage, dependency-lock verification, generic load and OCI runtime. Its public `PgErdMigrationConfig::build_proxy()` boundary revalidates deterministic Admin Config invariants so direct Serde deserialization cannot bypass version, listener authority, runtime, keepalive or transport-authority checks.

#14 is exact parented on #12 and reached exact `8937364909b82f50fd911aa001a8d073b517f5d9` through ordinary non-force adoption. CI `34163230667` is terminal GREEN for `load-contract 101869182479`, `test 101869182630` and `oci-runtime 101869182648`; Supply Chain `34163230576` / `candidate-evidence 101869180840` is terminal GREEN. The k6 artifact `10033390215`, digest `sha256:b098e774a469aee2e4fbf1a343ccee5579b77c08bc0d099dc3f301b82d602b90`, records 400 requests, 800/800 checks, zero HTTP failures and p95 `1.5951211 ms` against the unchanged `<20 ms` generic loopback threshold. This is controlled generic loopback evidence, not routed pg-erd or representative deployment SLO evidence.

#14's shared `edge_contract` is the sole effective socket-authority implementation. Traffic/metrics overlap rejects port zero, same-family wildcard/concrete aliases, IPv6-wildcard/IPv4 dual-stack ambiguity, native↔mapped IPv4 aliases, native/mapped wildcard aliases and mapped-to-mapped wildcard aliases. `migration_admin` reuses that boundary instead of duplicating it. Dual-profile OCI and Supply Chain evidence exists on exact #14, but registry-bound immutable image identity and release-bound provenance remain unproven.

#15 preserves the generic forwarding-trust delta while adopting current #14 without force-push or destructive rebase. Ordinary two-parent adoption `87fa98275340e205feb81b6dad208391cd619254` uses prior #15 `50c160e6d76c09559f56a1cda3b9269f720757ed` as first parent and exact #14 `8937364909b82f50fd911aa001a8d073b517f5d9` as second parent. Follow-up `4cad126a568093b31a052e0375e95a2ca8680e76` reapplies only the still-valid TRD forwarding contract; parent-supplied route-ordering and parameterized locked OCI-build reproducibility repairs are inherited rather than duplicated. #15 exact `f3dbe7afe1fa945d74145bb7852ae77882145b0a` has seven effective child paths and is mergeable. Its CI `34165264156` and Supply Chain `34165264253` are current exact-head gates; predecessor execution/review does not transfer.

The #15 effective forwarding invariant is: remove request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server` and `X-Real-IP`; then emit only gateway-owned `Forwarded: proto=http`. Generic v1 makes no client-IP or trusted-proxy provenance claim. Product identity authority is not introduced.

#16 is the runtime-isolation traffic child. Ordinary two-parent adoption `e087639ad2e03d36acdcb634d5081bd0b6cab343` preserves prior #16 `49d3fb5d89bf15543c7df8b433809acb8fac88eb` as first parent and exact #15 `f3dbe7afe1fa945d74145bb7852ae77882145b0a` as second parent. Exact #16 `d16f1440a05ca92a2acacd247ae2e62644df4f22` has five effective child paths and is mergeable. Its source acceptance sends a 9-byte chunked body against an 8-byte budget and requires 413 with `/readyz` still 200; it also holds the sole routed admission lease, requires a second routed request to fail 503 with backpressure telemetry visible, then proves later route recovery. CI `34165572648` and Supply Chain `34165572650` are the current exact-head gates.

#17 is the refused-origin failure child. Prior #17 `ce2b3032580fa851f9ccd34462801920be947a09` diverged after #16 moved. Ordinary two-parent adoption `49c23e509947e52e68e68ca9f2679ec979dc23fc` preserves prior #17 as first parent and exact #16 `d16f1440a05ca92a2acacd247ae2e62644df4f22` as second parent. The resolution tree takes current #16 and reapplies only `CHANGELOG.md`, `TEST_STRATEGY.md`, and `tests/pg_erd_upstream_failure_traffic.rs`; this commit projects the current failure contract into the baseline instead of replaying stale predecessor blockers.

`tests/pg_erd_upstream_failure_traffic.rs` reserves and releases the characterized backend socket before `/api/unavailable`, requiring a real loopback refusal to return 502 within a conservative one-second outer envelope around configured `connection_ms=200` / `total_connection_ms=400`, while `/readyz` stays 200 and `cwl_pingora_gateway_request_errors_total 1` remains observable. A later independent fallback request must still reach `frontend` and return 200. The one-second envelope is an end-to-end fail-closed guard, not a claim that a configured timeout elapsed. Connected read stall, TCP reset, post-commit truncation, slow-drip response, retry and failover remain separate gaps.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Implemented; #12/#14 exact hosted GREEN | Keep exact-head revalidation after every restack; no duplicate listener-authority implementations |
| Generic forwarding trust | Implemented on restacked #15 source | Exact-head hosted GREEN and fresh exact-head review required |
| Edge routing / HTTP policy | Characterized and compiled for pg-erd | Routed production-process parity must remain GREEN through descendant restacks |
| Runtime isolation | Declared/streamed body and in-flight limits implemented; #16 carries real-listener streamed overflow plus saturation/recovery acceptance | Exact descendant hosted GREEN/review; origin-capacity and in-flight SIGTERM drain remain separate tests |
| Failure recovery | #17 carries real refused-backend 502/readiness/error-telemetry/frontend-recovery acceptance | Exact hosted GREEN/review; connected read stall, reset, post-commit truncation, slow-drip/streaming failure remain unproven |
| Upstream TLS | Generic local-CA/SNI verification exists; pg-erd fail-closed trust activation exists | Successful pg-erd TLS listener/origin path and representative TLS performance remain unproven |
| Protocols | HTTP/1.1 path exists | Downstream TLS/H2, H2→H1 Cookie behavior, WebSocket/Extended CONNECT and explicit H3/QUIC disposition require separate supplier-capable contracts |
| OCI / supply chain | Dual-profile local candidate build, least privilege, audit, SPDX SBOM and scan evidence GREEN on #14 | Exact descendant revalidation; immutable registry digest, signing/attestation/provenance, release-bound SBOM, reproducibility receipt and rollback rehearsal |
| Performance | Generic loopback #14 p95 `1.5951211 ms`; supplier measured-origin #62 p95 `0.71593235 ms` | Do not treat these as production SLO proof; routed pg-erd concurrency/TLS/failure measurements must meet the same `<20 ms` objective where applicable without sample reduction or artificial warm-up |
| Observability | Low-cardinality counters/logging and separate metrics listener; #16/#17 assert bounded backpressure/error telemetry | Payload-free broader routed assertions, tracing and buyer-facing operability/recovery evidence |
| Release / migration | No protected release or consumer cutover credit | Immutable release → parity → shadow/canary → rollback rehearsal → cutover → verified Nginx/OpenResty/legacy removal |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated immutable supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + preserved #62 GREEN → #56 independent APPROVED/governance → foundation/protected integration → #12 → #14 → #15 → #16 → #17 and descendants ordinary non-force restack with exact-head GREEN → remaining routed TLS/read-stall/reset/truncation/drain/protocol acceptance → immutable gateway release/SBOM/provenance/reproducibility/rollback → shadow/canary → cutover → verified legacy removal`.

No Draft state, predecessor receipt, bot review, local image ID, mutable supplier PR, queue state or controlled loopback measurement is treated as protected merge, release, canary, cutover, rollback or legacy-removal evidence.
