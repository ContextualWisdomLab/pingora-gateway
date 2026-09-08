# Product / Technical Gap Baseline

This file is the code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. Live protected-branch, PR, review, workflow, release and supplier metadata remain the exact authority. Historical RED/GREEN detail belongs in PRs, commits, workflow receipts, ADRs and `CHANGELOG.md`; this snapshot keeps durable architecture, acceptance and remaining buyer-visible gaps. Mutable current-head SHAs and in-flight run IDs are deliberately not copied here because an evidence-refresh commit would immediately stale its own statement.

## Authority and bounded contexts

`pingora-gateway` owns shared Ingress, Edge Routing, TLS consumption, HTTP Policy, Load Balancing mechanics, Observability, Admin Config and Runtime Isolation behavior that is genuinely edge/runtime responsibility. Product authentication and business logic remain product-owned. Keyverse remains identity authority; Wardnet/EgressWeave remain their policy authorities. The gateway consumes released/versioned contracts or explicit operator transport inputs and does not copy sibling source, issue cross-service SQL, or depend on mutable sibling PR heads.

Generic v1 remains a one-upstream Rust/Pingora process. The bounded `cwl-pingora-pg-erd-migration` process is a separate composition root for the characterized `pg-erd-cloud` route/header contract; it is not a general product-routing DSL. `PgErdMigrationConfig` deliberately denies unknown fields and admits only concrete listener, runtime budget and characterized `backend`/`frontend` transport bindings. Route authority stays compiled into the migration plan rather than becoming operator-configurable product routing.

## Dependency and promotion root

Foundation #1 and compiler prerequisite #56 remain the earlier promotion root. #56 has terminal CI/Supply Chain evidence but protected promotion still requires an independent `APPROVED` review; owner or bot technical comments are not substituted for that gate.

Supplier-intake #54 remains intentional hosted RED because the committed gateway dependency graph still contains unmaintained `derivative 2.2.0` / `RUSTSEC-2024-0388`. Audit ignores, lock deletion, mutable supplier pins, scanner suppression and muted regressions are not admissible repairs. Supplier-semantics #62 remains exact hosted/technical GREEN for the required `PeerOptions` Debug surface, `Backend` equality/hash/order semantics and bounded Rust-origin load contract.

The supplier promotion path remains: maintainer-integrated, release-qualified Pingora repair removing `derivative` from the relevant workspace/core/load-balancing manifests and regenerated lock while preserving #62 semantics → exact gateway supplier bump and committed lock regeneration → unchanged #54 absence regression GREEN and preserved #62 GREEN → #56 independent approval/governance → protected foundation promotion.

## Current pg-erd stack

The parent chain through #20 is ordinary/non-force and has exact hosted evidence at each retained current head. Those contracts cover bounded Admin Config/socket authority, forwarding-trust reconstruction, body/in-flight isolation, refused and connected-silent origins, post-header inactivity, OCI metrics identity and routed SIGTERM drain. Predecessor receipts are never transferred to changed descendants.

### `#21` post-header partial-response phase

This phase owns the characterized backend contract that declares `Content-Length: 20`, commits only the seven-byte `partial` prefix, waits until that prefix has been observed downstream, then closes. Acceptance preserves the committed 200/framing rather than inventing a second status or failover, requires `/readyz` 200, an exact single request-error Prometheus sample, and an independent `frontend` recovery request. The framing oracle parses field lines semantically, matches `Content-Length` case-insensitively, trims field-value whitespace, requires exactly one value equal to `20`, and rejects lookalike or duplicate/conflicting fields. #21 has terminal exact-head CI/Supply Chain evidence; descendants must independently revalidate.

### `#22` routed concurrency/latency acceptance

This phase has closed its unchanged exact-head hosted and changed-range technical-review gates. Its measured origin is Rust-only: `tests/load/load_origin.rs` is a bounded std-only HTTP/1.1 loopback origin with finite worker and queue budgets, a 64 KiB request-header cap, deterministic Content-Length framing, startup validation and direct fixture tests. The load lane compiles that fixture with `rustc -D warnings`, runs its tests, builds an optimized origin, and contains no Python invocation.

Routed k6 acceptance uses four VUs and 400 total iterations, alternates characterized `/api/load-contract` and `/load-contract` requests, tags every request with `backend` or `frontend`, requires exact 200/body identity and zero HTTP failures, and independently gates aggregate, backend and frontend `http_req_duration` at p95 `<20 ms`. Each route must contribute at least 198 measured samples. The workflow starts distinct bounded Rust origins for the characterized `backend` and `frontend` authorities and the compiled migration process with only admitted transport/runtime configuration; it does not add an operator route DSL. Current exact hosted traffic meets this controlled-loopback contract, but those measurements are not representative deployment, TLS, multi-hop or origin-capacity SLO evidence.

### `#23` payload-free shared-observability acceptance

This phase has been ordinarily/non-force restacked on the final #22 tree. The restack preserves the historical child as first parent and final #22 as second parent while using the final #22 tree as the resolution basis, then reapplies only the valid child contract rather than stale parent source or documentation.

`tests/pg_erd_payload_free_observability.rs` drives a real routed request carrying unique URI/query, Host, Authorization, Cookie and product-context sentinels through `cwl-pingora-pg-erd-migration`. The backend must receive those values, proving the fixture is non-vacuous, while the shared `cwl_pingora_gateway::observability` target may emit only bounded transport completion facts and none of the sentinels. The test-only reliability repairs retained on this child are:

- traffic and metrics listener reservations remain held concurrently until immediately before process start so ephemeral-port reuse cannot manufacture an Admin Config collision;
- origin request-header reads are bounded by a five-second socket timeout and 64 KiB byte cap so forwarding failure becomes finite RED instead of a hanging suite;
- HTTP field lookup uses case-insensitive exact field-name parsing plus OWS trimming so `X-Forwarded-Host` cannot satisfy `Host`;
- Prometheus validation uses exact complete sample-line matching and a focused regression so a counter value such as `10` cannot satisfy an expected value of `1`.

These repairs change no production Rust, observability vocabulary, routing, authentication/TLS behavior or product authority. The current #23 source/docs must independently acquire exact-head hosted CI/Supply Chain and current-range review evidence after the restack before observability parity is credited.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Characterized pg-erd transport binding is fail closed; routes remain compiled rather than operator-configurable | Revalidate on every descendant restack |
| Generic forwarding trust | Sanitizer/reconstruction invariant is inherited through the current parent stack | No client-IP/trusted-proxy claim until separately characterized |
| Runtime isolation / recovery | Body/in-flight rejection, refused origin, read stall, partial response and graceful drain are covered on current ancestry | Explicit TCP reset, broader streaming/upgraded failure, slow-drip/whole-response lifetime and rollback traffic remain unproven |
| Upstream TLS | Generic local-CA/SNI verification and pg-erd fail-closed trust activation exist | Successful pg-erd TLS origin path plus representative TLS performance remain unproven |
| Protocols | HTTP/1.1 migration path exists | Downstream TLS/H2, H2→H1 Cookie behavior, WebSocket/Extended CONNECT and explicit H3/QUIC disposition require separate supplier-capable contracts |
| OCI / supply chain | #22 exact dual-profile OCI and Supply Chain evidence are GREEN | #23 and every changed descendant must independently revalidate; immutable registry digest, signing/attestation/provenance, release-bound SBOM, reproducibility receipt and rollback rehearsal remain gaps |
| Performance | #22 exact routed Rust-origin traffic passes aggregate/per-route/sample-floor acceptance | Controlled loopback is not production SLO proof; TLS/multi-hop/container scheduling/origin-capacity deployment measurements remain required |
| Observability | Low-cardinality shared counters/logging exist; #23 now contains non-vacuous payload-free compiled-process acceptance with exact header and metric oracles | Obtain unchanged #23 exact hosted/review closure and preserve semantics through later failure/protocol children |
| Documentation / review | Changelog, Test Strategy and this baseline are aligned to the #22→#23 succession and avoid self-staling run IDs | Bot/static technical review is not independent human approval; later changed heads need their own review evidence |
| Release / migration | No protected release or consumer cutover credit | Immutable release → parity → shadow/canary → rollback rehearsal → cutover → verified Nginx/OpenResty/legacy removal |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated release-qualified supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + preserved #62 GREEN → #56 independent APPROVED/governance → protected foundation integration → #12 → #14 → #15 → #16 → #17 → #18 → #19 → #20 → #21 exact hosted GREEN → #22 exact hosted/technical-review GREEN → #23 exact hosted/review closure → later descendants parent-first ordinary/non-force → remaining TLS/failure/protocol acceptance → immutable gateway release/SBOM/provenance/reproducibility/rollback → shadow/canary → cutover → verified legacy removal`.

No Draft state, predecessor receipt, bot review, local image ID, mutable supplier PR, queue state or controlled loopback measurement is treated as protected merge, release, canary, cutover, rollback or legacy-removal evidence.
