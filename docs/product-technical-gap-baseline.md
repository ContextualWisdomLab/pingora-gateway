# Product / Technical Gap Baseline

This file is the code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. Live protected-branch, PR, review, workflow, release and supplier metadata remain the exact authority. Historical RED/GREEN detail belongs in PRs, commits, workflow receipts, ADRs and `CHANGELOG.md`; this snapshot keeps only evidence needed to understand the current migration graph and remaining buyer-visible gaps.

## Authority and bounded contexts

`pingora-gateway` owns shared Ingress, Edge Routing, TLS consumption, HTTP Policy, Load Balancing mechanics, Observability, Admin Config and Runtime Isolation behavior that is genuinely edge/runtime responsibility. Product authentication and business logic remain product-owned. Keyverse remains identity authority; Wardnet/EgressWeave remain their policy authorities. The gateway consumes released/versioned contracts or explicit operator transport inputs and does not copy sibling source, issue cross-service SQL, or depend on mutable sibling PR heads.

Generic v1 remains a one-upstream Rust/Pingora process. The bounded `cwl-pingora-pg-erd-migration` process is a separate composition root for the characterized `pg-erd-cloud` route/header contract; it is not a general product-routing DSL. `PgErdMigrationConfig` deliberately denies unknown fields and admits only concrete listener, runtime budget and characterized `backend`/`frontend` transport bindings. Route authority stays compiled into the migration plan rather than becoming operator-configurable product routing.

## Dependency and promotion root

Foundation #1 remains `0da81a93f93e869c15bb7d34c55fc87479d16522`. Compiler prerequisite #56 remains exact `18fb38b1ba70c4bf222642ef347f3d57a98379a2` with terminal CI/Supply Chain GREEN, but protected promotion still requires an independent `APPROVED` review; owner or bot technical comments are not substituted for that gate.

Supplier-intake #54 remains exact `50b0516a9249c4066e3a0f305dbf2759eae3ae06` with intentional hosted RED because committed `Cargo.lock` still contains `derivative 2.2.0`. Audit ignores, lock deletion, mutable supplier pins, scanner suppression and muted regressions are not admissible repairs. Supplier-semantics #62 remains exact `32e0aeedac7b0fe6234d476245f37994b1b9168f` with exact hosted GREEN and the required `PeerOptions` Debug plus `Backend` equality/hash/order semantics.

The supplier promotion path remains: maintainer-integrated, release-qualified Pingora repair removing `derivative` from the relevant workspace/core/load-balancing manifests and regenerated lock while preserving #62 semantics → exact gateway supplier bump and committed lock regeneration → unchanged #54 GREEN and preserved #62 GREEN → #56 independent approval/governance → protected foundation promotion.

## Current pg-erd stack

The parent chain through #20 is ordinary/non-force and has exact hosted evidence at each current head. Relevant current heads are #12 `69f22265cd88881b8e14cedb32defa0a91180aa2`, #14 `8937364909b82f50fd911aa001a8d073b517f5d9`, #15 `bb65f2810b178bda46ed7eb5c6a62aae9fe36403`, #16 `356f3f250043d71c9bb1c0655481cd6984f3ae57`, #17 `7a7e1f1ca4c8310220b7ff2fb96e01027a7e89f3`, #18 `9749d01ae0e9aae027d7fce1a2c15e6a8358acd9`, #19 `86a6eb1b8fd5777b578cdbce49f40d52e916cc9b`, and #20 `d4d4565854cc924a2214de2b67a966d2f253da3e`. Their retained contracts cover bounded Admin Config/socket authority, forwarding-trust reconstruction, body/in-flight isolation, refused and connected-silent origins, post-header inactivity, OCI metrics identity and routed SIGTERM drain. Predecessor receipts are not transferred to changed descendants.

#21 owns the post-header partial-response phase. Ordinary two-parent commit `bff40ec74448a6e63bac8c408962aad7f7309d4e` adopted exact #20 while retaining the valid child test delta. The final exact head is `51f1242663ccbf164efc50ca2ac74c4d0a1c7126`. Its backend declares `Content-Length: 20`, commits only the seven-byte `partial` prefix, waits until that prefix has been observed downstream, then closes. Acceptance preserves the committed 200/framing rather than inventing a second status or failover, requires `/readyz` 200, exact `cwl_pingora_gateway_request_errors_total 1`, and an independent `frontend` recovery request. The prior substring-based Content-Length oracle was replaced by semantic field parsing that matches the field name case-insensitively, trims field-value whitespace, requires exactly `vec!["20"]`, and rejects lookalike or duplicate/conflicting fields.

Exact #21 CI `34185538078` is terminal GREEN for `test`, `load-contract` and dual-profile `oci-runtime`; Supply Chain `34185538063` is terminal GREEN on the same exact SHA. The test lane passed exact checkout, Rust 1.98.0 formatting, locked compile/test, strict Clippy, warning-denied public rustdoc, complete owned-production line/region coverage enforcement and resolved-lock verification. Existing owner and CodeRabbit comments are technical evidence only; there is no independent human `APPROVED` review credit.

#22 owns the next routed concurrency/latency acceptance. It no longer carries stale #21 ancestry. Ordinary two-parent commit `bc6d50fce4e7a94b928dc91dce4e299e558a4c93` keeps historical #22 `c71bd51fec222508cbf98931e76a98f0830c312b` as first parent and exact final #21 `51f1242663ccbf164efc50ca2ac74c4d0a1c7126` as second parent while using the exact #21 tree as the resolution tree. Valid child semantics were then reapplied without replaying stale parent source/docs/workflow blobs.

The #22 measured path is Rust-only. `tests/load/load_origin.rs` is a bounded std-only HTTP/1.1 loopback origin with finite worker and queue budgets, a 64 KiB request-header cap, deterministic Content-Length framing, startup validation and direct fixture tests. The measured `load-contract` compiles that fixture with `rustc -D warnings`, runs its tests, builds an optimized origin, and contains no Python invocation. The historical Python origin file is removed from this child.

Routed k6 acceptance uses 4 VUs and 400 total iterations, alternates characterized `/api/load-contract` and `/load-contract` requests, tags every request with `backend` or `frontend`, requires exact 200/body identity and zero HTTP failures, and independently gates aggregate, backend and frontend `http_req_duration` at p95 `<20 ms`. Each route must contribute at least 198 measured samples. The workflow starts distinct bounded Rust origins for the characterized `backend` and `frontend` authorities, starts the compiled `cwl-pingora-pg-erd-migration` binary with only its admitted transport/runtime configuration, then records `k6-pg-erd-summary.json`. Route selection remains compiled into the pg-erd migration plan; workflow configuration does not add a route DSL.

Current #22 exact head is `162f29c83cef032787359195d7468640485b488e`. The PR is Draft/open/mergeable, base SHA is exactly final #21, and its effective delta is limited to the current CI load lane, bounded Rust origin, routed k6 script, routed-latency regression contract, Rust-origin workflow regression contract, removal of the obsolete Python measured-origin fixture, and this baseline. Exact #22 CI `34191044793` and Supply Chain `34191044772` have been dispatched on this SHA but were still pre-execution/queued at the latest evidence read. No predecessor GREEN, production p95, protected merge, release, canary or cutover credit is transferred to this changed head.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Characterized pg-erd transport binding is fail closed; routes remain compiled rather than operator-configurable | Revalidate on every descendant restack |
| Generic forwarding trust | #15 invariant inherited through the current parent stack | Preserve exact sanitizer/reconstruction invariant; no client-IP/trusted-proxy claim until separately characterized |
| Runtime isolation / recovery | #16–#21 cover body/in-flight rejection, refused origin, read stall, partial response and graceful drain on current ancestry | Explicit reset, broader streaming/upgraded failure, slow-drip/whole-response lifetime and rollback traffic remain unproven |
| Upstream TLS | Generic local-CA/SNI verification and pg-erd fail-closed trust activation exist | Successful pg-erd TLS origin path plus representative TLS performance remain unproven |
| Protocols | HTTP/1.1 migration path exists | Downstream TLS/H2, H2→H1 Cookie behavior, WebSocket/Extended CONNECT and explicit H3/QUIC disposition require separate supplier-capable contracts |
| OCI / supply chain | #21 exact dual-profile OCI and Supply Chain evidence GREEN | Revalidate changed #22; immutable registry digest, signing/attestation/provenance, release-bound SBOM, reproducibility receipt and rollback rehearsal remain gaps |
| Performance | #22 now contains routed Rust-origin aggregate/per-route/sample-floor acceptance | Obtain unchanged exact-head hosted k6 evidence; controlled loopback is not representative production SLO proof; TLS/multi-hop/origin-capacity deployment measurements remain required |
| Observability | Low-cardinality counters/logging and separate metrics listener are covered by parent traffic tests | Preserve exact telemetry semantics through #22 and later failure/protocol children |
| Documentation / review | This baseline reflects final #21 and current #22 ancestry and semantics | Reacquire exact #22 hosted closure and fresh exact-range review; bot/static review is not independent human approval |
| Release / migration | No protected release or consumer cutover credit | Immutable release → parity → shadow/canary → rollback rehearsal → cutover → verified Nginx/OpenResty/legacy removal |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated immutable supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + preserved #62 GREEN → #56 independent APPROVED/governance → protected foundation integration → #12 → #14 → #15 → #16 → #17 → #18 → #19 → #20 → #21 exact hosted GREEN → #22 exact hosted/review closure → parent-first ordinary/non-force succession of later descendants → remaining TLS/failure/protocol acceptance → immutable gateway release/SBOM/provenance/reproducibility/rollback → shadow/canary → cutover → verified legacy removal`.

No Draft state, predecessor receipt, bot review, local image ID, mutable supplier PR, queue state or controlled loopback measurement is treated as protected merge, release, canary, cutover, rollback or legacy-removal evidence.
