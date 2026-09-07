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

#14 is exact parented on #12 and reached exact `8937364909b82f50fd911aa001a8d073b517f5d9` through ordinary non-force adoption. CI `34163230667` is terminal GREEN for `load-contract 101869182479`, `test 101869182630` and `oci-runtime 101869182648`; Supply Chain `34163230576` / `candidate-evidence 101869180840` is terminal GREEN. The k6 artifact `10033390215`, digest `sha256:b098e774a469aee2e4fbf1a343ccee5579b77c08bc0d099dc3f301b82d602b90`, records 400 requests, 800/800 checks, zero HTTP failures and p95 `1.5951211 ms` against the unchanged `<20 ms` generic loopback threshold. This is controlled generic loopback evidence, not routed pg-erd or representative deployment SLO evidence. OCI proves both admitted image profiles under uid/gid 65532, read-only root, dropped capabilities and `no-new-privileges`; Supply Chain builds/scans both profiles, produces SPDX dependency SBOM evidence and binds receipts to exact source. Registry-bound immutable image identity and release-bound provenance remain unproven.

#14's shared `edge_contract` is the sole effective socket-authority implementation. Traffic/metrics overlap rejects port zero, same-family wildcard/concrete aliases, IPv6-wildcard/IPv4 dual-stack ambiguity, native↔mapped IPv4 aliases, native/mapped wildcard aliases and mapped-to-mapped wildcard aliases. `migration_admin` reuses that boundary instead of duplicating it.

#15 preserves the generic forwarding-trust delta while adopting current #14 without force-push or destructive rebase. Its exact current head is `bb65f2810b178bda46ed7eb5c6a62aae9fe36403`. The effective forwarding invariant removes request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server` and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http`; generic v1 makes no client-IP or trusted-proxy provenance claim.

#15 exact CI `34166547052` and Supply Chain `34166547010` are terminal GREEN after the executed coverage RED on predecessor `f3dbe7afe1fa945d74145bb7852ae77882145b0a`. The predecessor had only one uncovered production region: the impossible error-propagation branch from inserting the static gateway-owned forwarding literal. Minimum source repair `abeca67680bc218781209f2ffe065c1091dd55e4` replaces only that impossible branch with an explicit literal invariant; routing, sanitizer output, client-visible error behavior and coverage policy remain unchanged. Current CI passed formatting, locked compile/test, strict Clippy, warning-denied public rustdoc, 100% owned-production line/region coverage, dependency-lock verification, generic load and dual-profile OCI runtime. Current Supply Chain passed committed dependency audit, both candidate-image builds, SPDX dependency SBOM, both image scans, exact-source binding and success-only evidence upload. Load artifact `10034472458`, digest `sha256:5e2e7f5a85b83c01b49f7767b63b830ae0b65b26b5695fc21b55fc6beaadce37`, records 400 requests, 800/800 checks, zero HTTP failures and p95 `1.31409845 ms` against the unchanged `<20 ms` generic loopback threshold. Fresh exact-range CodeRabbit review reports no actionable finding; it remains technical review rather than independent human approval.

#16 is the next traffic-evidence child and now adopts exact #15 `bb65f2810b178bda46ed7eb5c6a62aae9fe36403` by ordinary two-parent succession, preserving the existing #16 traffic delta as the first-parent lineage and taking current #15 as the second parent. Parent source and evidence are not copied into child-owned modules, and no force-push or destructive rebase is used.

`tests/pg_erd_runtime_isolation_traffic.rs` exercises the compiled `cwl-pingora-pg-erd-migration` process over real loopback listeners. One test sends a 9-byte chunked body against an 8-byte budget and requires 413 while `/readyz` remains 200. A second configures `max_in_flight_requests=1`, holds one routed backend request, requires the next routed request to fail fast 503, requires `/readyz` and `cwl_pingora_gateway_backpressure_rejections_total 1` to remain observable, releases the held request, and requires a later backend route to succeed.

The previous #16 exact `d16f1440a05ca92a2acacd247ae2e62644df4f22` executed CI `34165572648`: load and OCI were GREEN and Supply Chain `34165572650` was GREEN, while test `101875828235` failed deterministically at rustfmt before compile/test. Owner-local repair `abee6af1ecfc1d2f44de1dc25917c2eb84a6bb38` applies exactly the hosted formatter output to this traffic fixture without changing traffic volume, status, timeout, metric, routing, TLS, auth or product semantics. The newly restacked #16 head must reacquire all exact-head CI/Supply Chain/review evidence; none of the predecessor GREEN transfers.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Implemented; #12/#14 exact hosted GREEN | Keep exact-head revalidation after every restack; no duplicate listener-authority implementations |
| Generic forwarding trust | #15 exact source/review/CI/Supply Chain GREEN | Preserve invariant through descendants; no client-IP/trusted-proxy claim until separately characterized |
| Edge routing / HTTP policy | Characterized and compiled for pg-erd | Routed production-process parity must remain GREEN through descendant restacks |
| Runtime isolation | Declared/streamed body and in-flight limits implemented; #16 carries real-listener streamed overflow plus saturation/recovery acceptance | Exact restacked #16 hosted GREEN/review; routed timeout/reset/post-commit truncation/slow-drip, origin-capacity and SIGTERM drain remain later stack acceptance |
| Upstream TLS | Generic local-CA/SNI verification exists; pg-erd fail-closed trust activation exists | Successful pg-erd TLS listener/origin path and representative TLS performance remain unproven |
| Protocols | HTTP/1.1 path exists | Downstream TLS/H2, H2→H1 Cookie behavior, WebSocket/Extended CONNECT and explicit H3/QUIC disposition require separate supplier-capable contracts |
| OCI / supply chain | #14 and #15 dual-profile local candidate build, least privilege, audit, SPDX SBOM and scan evidence GREEN | Exact descendant revalidation; immutable registry digest, signing/attestation/provenance, release-bound SBOM, reproducibility receipt and rollback rehearsal |
| Performance | Generic loopback #15 p95 `1.31409845 ms`; supplier measured-origin #62 p95 `0.71593235 ms` | Do not treat these as production SLO proof; routed pg-erd concurrency/TLS/failure measurements must meet the `<20 ms` objective where applicable without sample reduction or artificial warm-up |
| Observability | Low-cardinality counters/logging and separate metrics listener; #16 asserts saturation telemetry | Payload-free routed assertions beyond backpressure, tracing and buyer-facing operability/recovery evidence |
| Release / migration | No protected release or consumer cutover credit | Immutable release → parity → shadow/canary → rollback rehearsal → cutover → verified Nginx/OpenResty/legacy removal |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated immutable supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + preserved #62 GREEN → #56 independent APPROVED/governance → foundation/protected integration → #12 → #14 → #15 exact hosted GREEN → #16 current restack exact-head GREEN → #17/#18/#19/#20/#21 and later descendants parent-first ordinary non-force succession → remaining routed TLS/failure/drain/protocol acceptance → immutable gateway release/SBOM/provenance/reproducibility/rollback → shadow/canary → cutover → verified legacy removal`.

No Draft state, predecessor receipt, bot review, local image ID, mutable supplier PR, queue state or controlled loopback measurement is treated as protected merge, release, canary, cutover, rollback or legacy-removal evidence.
