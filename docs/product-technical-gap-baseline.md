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

The parent chain through #25 is ordinary/non-force and has exact hosted evidence at each retained final head. Those contracts cover bounded Admin Config/socket authority, forwarding-trust reconstruction, body/in-flight isolation, refused and connected-silent origins, post-header inactivity, OCI metrics identity, routed SIGTERM drain, routed latency, payload-free observability, and pre-/post-commit TCP reset phases. Predecessor receipts are never transferred to changed descendants.

### `#21` post-header partial-response phase

This phase owns the characterized backend contract that declares `Content-Length: 20`, commits only the seven-byte `partial` prefix, waits until that prefix has been observed downstream, then closes. Acceptance preserves the committed 200/framing rather than inventing a second status or failover, requires `/readyz` 200, an exact single request-error Prometheus sample, and an independent `frontend` recovery request. The framing oracle parses field lines semantically, matches `Content-Length` case-insensitively, trims field-value whitespace, requires exactly one value equal to `20`, and rejects lookalike or duplicate/conflicting fields. #21 has terminal exact-head CI/Supply Chain evidence; descendants must independently revalidate.

### `#22` routed concurrency/latency acceptance

This phase has closed its unchanged exact-head hosted and changed-range technical-review gates. Its measured origin is Rust-only: `tests/load/load_origin.rs` is a bounded std-only HTTP/1.1 loopback origin with finite worker and queue budgets, a 64 KiB request-header cap, deterministic Content-Length framing, startup validation and direct fixture tests. Routed k6 uses four VUs and 400 total iterations, alternates characterized `/api/load-contract` and `/load-contract`, requires exact 200/body identity and zero HTTP failures, independently gates aggregate/backend/frontend p95 below 20 ms, and requires at least 198 samples per route. This remains controlled loopback evidence, not representative deployment, TLS, multi-hop or origin-capacity SLO proof.

### `#23` payload-free shared-observability acceptance

This phase was ordinarily/non-force restacked on the final #22 tree and has independently closed exact-head hosted CI/Supply Chain plus current-range technical review. `tests/pg_erd_payload_free_observability.rs` sends a real routed request carrying unique URI/query, Host, Authorization, Cookie and product-context sentinels. The backend must receive the exact case-sensitive request target and semantically exact HTTP fields, while shared gateway stderr must emit only the exact bounded completion message and none of the sentinels. Reliability oracles keep traffic/metrics reservations concurrent, bound origin request reads to five seconds/64 KiB, reject `X-Forwarded-Host` as a substitute for `Host`, and require exact Prometheus sample-line equality. Bot/static technical review remains technical evidence only and does not replace #56 independent human approval.

### `#24` pre-header upstream TCP reset

This phase owns the distinct case where the characterized backend connection succeeds, the complete routed request reaches `backend`, and that backend then performs an abortive Linux `SO_LINGER(0)` close before sending any response header. It was ordinary/non-force restacked from the final #23 tree while preserving only this valid child test delta and code-current documentation; no stale parent blobs or predecessor receipts were replayed.

`tests/pg_erd_upstream_reset_traffic.rs` requires HTTP 502 within two seconds despite `read_ms=5000`, no silent frontend failover, `/readyz` 200, exactly one `cwl_pingora_gateway_request_errors_total 1` sample, and a later independent frontend HTTP 200. Traffic and metrics listener reservations remain simultaneous until process startup; origin header reads fail closed after five seconds or 64 KiB; exact Prometheus sample-line matching rejects numeric-prefix values such as `10`; exact HTTP/1.1 status parsing rejects protocol-case and numeric-prefix lookalikes. This phase has closed unchanged-head CI/Supply Chain and a fresh exact-head technical review. Bot/owner technical evidence does not replace #56 independent human approval.

### `#25` post-commit upstream TCP reset

This phase owns the later transport case where the characterized backend first commits HTTP 200 with exactly one `Content-Length: 20` field and the seven-byte `partial` body prefix, waits until the downstream has actually observed that committed header and prefix, then applies Linux `SO_LINGER(0)` and aborts the established connection. Ordinary/non-force succession starts from the final #24 tree and preserves the historical child as ancestry while replaying only the valid post-commit-reset delta.

`tests/pg_erd_post_commit_reset_traffic.rs` preserves the already-committed 200 and exact framing, terminates before the declared 20-byte body completes, avoids a fabricated second status or silent failover, keeps `/readyz` 200, exposes exactly one low-cardinality request-error sample, and allows an independent frontend recovery request. Listener reservations are concurrent, origin header reads are bounded to five seconds/64 KiB, `Content-Length` is matched by exact field identity so `X-Content-Length` cannot satisfy the oracle, Prometheus samples use exact whole-line equality, and response status uses an exact HTTP/1.1 three-digit parser. Final #25 has terminal exact-head CI/Supply Chain; a fresh owner technical sweep found no new actionable source/documentation defect. The included CodeRabbit review was temporarily rate-limited, so no bot review credit is inferred or transferred.

### `#27` recursive gateway network-authority separation

This phase prevents operator-configured upstream transport authority from aliasing either gateway-owned listener. Generic `GatewayConfig` and bounded `PgErdMigrationConfig` reuse one `socket_authorities_overlap` model for exact addresses, IPv4/IPv6 wildcard aliases, IPv4-mapped IPv6 aliases and the conservative IPv6-wildcard/IPv4 dual-stack case. An upstream that aliases the public traffic listener fails with `UpstreamListenerCollision`; one that aliases the internal metrics listener fails with `UpstreamMetricsListenerCollision`. Distinct concrete IP authorities on the same port remain configurable, so the invariant prevents recursive self-proxying and accidental metrics exposure without becoming product routing policy.

The current child ordinarily/non-force adopted final #25 with a two-parent ancestry commit whose resolution tree was exactly the final #25 tree, then reapplied only the shared production invariant, bounded migration-admin use of that invariant, and dedicated current-parent acceptance. `tests/network_authority_self_loop_contract.rs` proves generic and pg-erd exact/wildcard/dual-stack collision rejection, distinct-concrete same-port admission, public `build_proxy` revalidation after direct deserialization, and compiled fail-closed startup for both composition roots. Current-head hosted and current-range review evidence must be generated on the resulting exact child; historical #27 receipts are not transferred.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Characterized pg-erd transport binding is fail closed; routes remain compiled; current #27 source now rejects upstream aliases of traffic/metrics listener authority in both composition roots | Exact #27 hosted/review closure and revalidation on every descendant restack |
| Generic forwarding trust | Sanitizer/reconstruction invariant is inherited through the current parent stack | No client-IP/trusted-proxy claim until separately characterized |
| Runtime isolation / recovery | Body/in-flight rejection, refused origin, read stall, partial response, graceful drain, pre-header reset and post-commit reset have retained exact-head evidence | Broader streaming/upgraded failure, slow-drip/whole-response lifetime and rollback traffic remain unproven |
| Upstream TLS | Generic local-CA/SNI verification and pg-erd fail-closed trust activation exist | Successful pg-erd TLS origin path plus representative TLS performance remain unproven |
| Protocols | HTTP/1.1 migration path exists | Downstream TLS/H2, H2→H1 Cookie behavior, WebSocket/Extended CONNECT and explicit H3/QUIC disposition require separate supplier-capable contracts |
| OCI / supply chain | #25 exact dual-profile OCI and Supply Chain evidence are GREEN | #27 and every changed descendant must independently revalidate; immutable registry digest, signing/attestation/provenance, release-bound SBOM, reproducibility receipt and rollback rehearsal remain gaps |
| Performance | #22 routed Rust-origin traffic passes aggregate/per-route/sample-floor acceptance | Controlled loopback is not production SLO proof; TLS/multi-hop/container scheduling/origin-capacity deployment measurements remain required |
| Observability | Low-cardinality shared counters/logging exist; #23 proves payload-free observability and reset phases preserve exact error telemetry | Preserve semantics through #27 and later failure/protocol children; tracing remains separately unproven |
| Documentation / review | This baseline is aligned to final #25 → current #27 ordinary succession and avoids mutable run IDs | Current #27 changed head needs its own hosted/current-range review evidence; bot/owner technical comments do not replace #56 independent human approval |
| Release / migration | No protected release or consumer cutover credit | Immutable release → parity → shadow/canary → rollback rehearsal → cutover → verified Nginx/OpenResty/legacy removal |

## Execution order

The current dependency order is `#54 derivative RED + #62 exact semantics/load GREEN → maintainer-integrated release-qualified supplier repair → gateway supplier bump and committed lock regeneration → #54 GREEN + preserved #62 GREEN → #56 independent APPROVED/governance → protected foundation integration → #12 → #14 → #15 → #16 → #17 → #18 → #19 → #20 → #21 exact hosted GREEN → #22 exact hosted/technical-review GREEN → #23 exact hosted/technical-review GREEN → #24 exact hosted/technical-review GREEN → #25 exact hosted/owner-technical closure → #27 exact hosted/current-range review closure → remaining TLS/failure/protocol acceptance → immutable gateway release/SBOM/provenance/reproducibility/rollback → shadow/canary → cutover → verified legacy removal`.

No Draft state, predecessor receipt, bot review, local image ID, mutable supplier PR, queue state or controlled loopback measurement is treated as protected merge, release, canary, cutover, rollback or legacy-removal evidence.
