# Product / Technical Gap Baseline

This file is the durable, code-current migration baseline for `ContextualWisdomLab/pingora-gateway`. Live protected refs, PR heads/bases, reviews, workflow runs, releases and supplier metadata remain the exact authority. Mutable exact-head SHAs and in-flight run IDs belong in canonical issues #51 and #58 rather than here; historical RED/GREEN detail belongs in PRs, commits, workflow receipts, ADRs and `CHANGELOG.md`.

## Authority and bounded contexts

`pingora-gateway` owns reusable edge/runtime behavior that belongs to Ingress, Edge Routing, downstream TLS consumption, HTTP Policy, Load Balancing mechanics, Observability, Admin Config and Runtime Isolation. Product authentication and business logic stay product-owned. Keyverse remains identity authority; Wardnet and EgressWeave remain their policy authorities. Certificate issuance, renewal, revocation, ACME and private-key custody remain outside this gateway.

The gateway consumes released/versioned contracts or explicit operator transport inputs. It does not copy sibling source, query sibling databases, depend on mutable sibling PR heads, or turn a migration adapter into a product-domain authority.

Generic v1 remains a one-upstream Rust/Pingora composition root. `cwl-pingora-pg-erd-migration` remains the bounded composition root for the characterized `pg-erd-cloud` route/header contract; it is not a general product-routing DSL. `PgErdMigrationConfig` denies unknown fields and admits only concrete listener/runtime budgets plus characterized `backend` and `frontend` transport bindings. Route authority remains compiled into the migration plan.

## Domain invariants

The durable edge model is organized around the following bounded contexts and invariants.

| Bounded context | Durable invariant |
| --- | --- |
| Ingress | Listener authority is explicit, non-zero, non-overlapping and cannot recurse into gateway-owned traffic/metrics sockets. |
| Edge Routing | A request is routed only through characterized gateway-owned route/transport contracts; product routing semantics are not invented here. |
| TLS | Downstream TLS consumes explicit certificate/key material and a versioned security profile; upstream TLS consumes explicit trust/SNI. Certificate lifecycle authority stays external. |
| HTTP Policy | Uncharacterized protocol transitions fail closed; header/body admission and forwarding identity are protocol-aware but do not become product auth. |
| Load Balancing | Peer selection and pool mechanics remain transport concerns; retry/failover semantics are not silently broadened. |
| Observability | Shared telemetry is low-cardinality and payload-minimized; supplier diagnostics are redacted before shared formatting when they can expose request material. |
| Admin Config | Versioned config rejects unknown/ambiguous authority and zero budgets; new semantics require a version bump rather than hidden defaults. |
| Runtime Isolation | In-flight/body/lifetime budgets, graceful shutdown, connection reuse and worker topology are explicit and fail closed. |

DB ownership is currently not part of the gateway hot path. If persistent Admin Config or translation resources are introduced later, they must remain normalized/versioned and may not hold an explicit database transaction or lock across LLM, network, TLS, routing or other long-running computation.

## Foundation and supplier boundary

Rust 1.98.1 foundation/governance remains a prerequisite for protected promotion. Owner/bot technical COMMENTs are evidence only and do not replace a ruleset-valid independent `APPROVED` review.

Released dependency authority is Pingora 0.9.0, with h2 0.4.19 in the resolved graph. Mutable upstream PR heads are never dependency authority. Publication alone is not sufficient supplier qualification: the gateway still requires exact commit/package identity, lockfile reproducibility, unchanged executable acceptance and downstream supply-chain evidence.

Open supplier concerns remain separate from gateway-local policy. They include the `derivative 2.2.0` / RUSTSEC-2024-0388 path, downstream HTTP/1 parser/lifetime controls, and mixed-protocol H2→H1 Cookie/body-framing behavior. These must be closed by maintainer-integrated, release-qualified supplier behavior or an explicitly versioned alternative owner contract; gateway-local source copies, audit suppression and mutable pins are not acceptable substitutes.

HTTP/3/QUIC remains fail closed until a maintainer-supported release-qualified server path exists and the gateway adds its own executable admission, failure, performance and rollback evidence.

## Retained pg-erd and generic runtime evidence

The retained stack already characterizes the following durable behavior. Exact promotion status remains in #58.

- bounded Admin Config and socket authority, including recursive self-proxy prevention;
- route/header preservation for the characterized pg-erd migration plan;
- explicit upstream TLS trust and hostname verification;
- in-flight and request-body isolation with recovery;
- refused origin, connected-silent origin, pre-header reset, post-commit reset and truncated-response behavior;
- response-body progress lifetime as a versioned pg-erd Runtime Isolation semantic;
- graceful in-flight SIGTERM drain and parked-read/shutdown correctness acceptance;
- process-wide payload-safe Pingora-family diagnostic handling;
- HTTP/1 Upgrade fail-close at request admission plus immutable peer defense in depth;
- routed latency and bounded-origin capacity harnesses with Rust origins and exact route/sample-floor contracts;
- OCI/Supply Chain/reproducibility workflow contracts that bind evidence to the exact source under test.

Controlled loopback latency/capacity results are regression evidence only. They are not a production SLO or representative TLS, multi-hop, Kubernetes, NUMA or origin-capacity claim.

## Downstream TLS / HTTP/2 migration state

The active stack deliberately separates transport capabilities instead of treating “TLS/H2 works” as one acceptance claim. Canonical current heads and workflow receipts are tracked in #51 and #58.

| Phase | Accepted responsibility | State boundary |
| --- | --- | --- |
| #75 | Explicit downstream certificate/key materialization, TLS listener activation, strict ALPN selection and verified HTTP/1.1 fallback/no-ALPN behavior | Does not own certificate lifecycle or later H2 semantics. |
| #76 | TLS 1.2–1.3 security profile and real-wire version/cipher acceptance | Keeps TLS 1.2 only for the documented compatibility boundary; does not imply future TLS-1.3-only policy. |
| #77 | One verified H2 connection dispatching multiple live streams before either origin response | Proves multiplexing, not cancellation, drain or admission limits. |
| #78 | Cancelling/resetting one H2 stream releases its origin and preserves the live sibling | Proves sibling survival, not graceful process shutdown. |
| #79 | SIGTERM-aware H2 graceful GOAWAY/drain, including the h2 0.4.19 graceful-shutdown PING/ACK behavior | Exact-head GREEN/Ready; independent approval is still governance authority. |
| #80 | Decoded HTTP/2 header-list admission: advertised 64 KiB limit, legal HEADERS/CONTINUATION oversize traffic, pre-origin stream-local rejection and same-connection sibling recovery | Draft until its current exact-head gates terminate. Exact 431 is not independently claimed by the raw fixture. |
| #81 | Protocol-neutral request-body budget parity on negotiated H2 for declared and streamed overflow plus same-connection recovery | Draft until its current exact-head gates terminate. Exact H2 413 is not independently claimed by the raw fixture; HTTP semantic authority remains the production policy plus RFC 9110. |

The raw H2 fixtures intentionally distinguish frame-size admission from decoded-header admission and connection-level failure from stream-local failure. They also keep control-frame handling protocol-correct: non-ACK PING is acknowledged with the identical opaque payload, SETTINGS is acknowledged, and GOAWAY/RST_STREAM are not reinterpreted as generic application errors.

## Capability state and buyer-visible gaps

| Area | Current state | Remaining acceptance |
| --- | --- | --- |
| Admin Config / network authority | Versioned generic and pg-erd contracts are fail closed; recursive traffic/metrics authority is rejected. | Revalidate on every changed release candidate; do not turn migration config into product routing. |
| Forwarding trust | Client-controlled proxy identity is sanitized/reconstructed for characterized paths. | Add explicit trusted-proxy/client-IP chain acceptance before claiming production forwarding provenance. |
| HTTP/1 request-header admission / lifetime | Supplier parser ceilings and per-read inactivity exist, but operator-controlled parser byte/count and monotonic whole-header lifetime are not fully versioned CWL controls. | Close parser-phase and whole-header lifetime supplier roots on released source; keep distinct from H2 decoded-header accounting. |
| Downstream TLS | Verified certificate material, ALPN, HTTP/1.1 fallback, TLS 1.2–1.3 profile and real-wire H2 are characterized through the active stack. | Certificate lifecycle remains external; add representative handshake/reuse performance and release/cutover evidence. |
| HTTP/2 semantics | Multiplexing, reset/sibling survival, graceful GOAWAY/drain and current header/body-admission work are separated into executable phases. | Finish current #80/#81 exact gates, then connection/stream flow control/backpressure, partial-body upstream cancellation and origin failure/recovery. |
| WebSocket / CONNECT | HTTP/1 Upgrade is intentionally rejected at admission and peer policy. | Add a separately versioned WebSocket/Extended CONNECT product form only if a real migration target requires it. |
| HTTP/3 / QUIC | Fail closed. | Require released supplier support, explicit config/ALPN authority, real traffic, failure/security/load evidence and rollback before admission. |
| Runtime isolation / recovery | In-flight/body budgets, origin failures, response lifetime, graceful drain and shutdown correctness are characterized. | Representative worker-count/NUMA contention, connection reuse and long-lived streaming/cancellation remain. |
| OCI / supply chain | Exact-source workflows, lockfile/reproducibility contracts and candidate evidence exist; mutable supplier PRs are excluded. | Resolve remaining supplier advisory roots and produce immutable signed release/SBOM/provenance/rollback receipts. |
| Performance | Routed and bounded-origin loopback p95 contracts exist and remain <20 ms gates where applicable. | Representative TLS handshake vs reused-connection, multi-hop/container scheduling, origin-capacity and CPU/off-CPU/NUMA profiling. |
| Observability | Payload-safe shared telemetry and exact request-error evidence exist. | Add release/canary/rollback dashboards and tracing evidence without importing product-domain payload authority. |
| Documentation / review | ADR/TRACEABILITY and this baseline separate durable architecture from exact mutable evidence. | Keep #51/#58 and this baseline synchronized with every head movement; independent approval remains distinct from technical COMMENT evidence. |
| Release / migration | Source-level parity is advancing, but no protected immutable gateway release or consumer cutover is credited. | Protected integration → version/CHANGELOG/tag/package → immutable artifact + SBOM/provenance/reproducibility → parity → shadow/canary → observed rollback → cutover → verified Nginx/OpenResty/legacy removal. |

## Next commercial gaps after the current H2 admission stack

The next protocol/runtime work is dependency-ordered rather than feature-count driven:

1. finish #80 decoded-header and #81 request-body exact-head GREEN plus independent governance without transferring predecessor receipts;
2. prove connection-level and stream-level H2 flow control/backpressure under realistic concurrent traffic, including bounded memory and sibling fairness;
3. characterize partial-body cancellation after upstream contact so a rejected/cancelled H2 stream cannot leak origin work or poison a sibling;
4. exercise pre-commit and post-commit origin failure/recovery on H2 without inventing retries or second statuses;
5. characterize trusted forwarding/client-IP provenance on TLS/H2;
6. consume only release-qualified supplier fixes for H2→H1 Cookie/body framing and HTTP/1 parser/lifetime roots;
7. measure TLS handshake versus connection reuse, configured worker counts and representative CPU/NUMA/container scheduling before commercial performance credit;
8. promote the protected dependency chain, build an immutable gateway release with SBOM/provenance/reproducibility/rollback evidence, then run parity → shadow/canary → rollback → cutover and verify legacy proxy removal.

## Evidence and promotion rules

A changed exact head invalidates predecessor GREEN for promotion. A Draft, bot/owner COMMENT, local image ID, mutable supplier PR, queued run, release name, controlled loopback result or source-level capability is not protected merge/release/canary/cutover evidence.

Concurrent commits are inspected and adopted/adapted through normal forward integration. Force-push, destructive rebase, self-approval, gate weakening and silent deletion of valid delta are not migration tools. A PR may disappear only by normal merge or by a verified successor that completely inherits its valid source/test/fixture/contract/evidence.

The promotion order is therefore: foundation/supplier/governance prerequisites → exact current #75–#81 dependency chain → remaining H2/runtime/trust/performance acceptance → protected integration in dependency order → immutable gateway release → parity → shadow/canary → observed rollback → cutover → verified Nginx/OpenResty/legacy reverse-proxy removal. Until the last evidence exists, legacy removal is not complete.
