# Test Strategy

Tests are organized by responsibility and evidence boundary rather than by implementation detail. A source test is not acceptance evidence until the unchanged exact head passes the relevant hosted gates. Parent or predecessor GREEN never transfers across a changed head.

## Baseline quality gates

Every candidate runs locked all-target Rust tests, formatting, strict Clippy, warnings-denied rustdoc, complete owned-production coverage and resolved-lock verification. OCI acceptance builds both admitted process profiles and starts them non-root with a read-only root filesystem, capabilities dropped and `no-new-privileges`. Supply Chain separately audits the dependency graph, builds/scans both images, creates SBOM evidence and binds receipts to the exact source SHA.

The generic and pg-erd load lanes use checksum-pinned k6 against release-built candidates. Controlled loopback measurements are regression bounds, not production SLO evidence. Applicable buyer paths keep their declared sample/concurrency floor and p95 `<20 ms` threshold; reducing pressure or measuring a warm-only shortcut is not acceptable.

## Admin Config and network authority

`tests/config_contract.rs` covers generic configuration version admission, listener authority, runtime budgets and one-upstream semantics. Generic version 1 remains cleartext and must reject `downstream_tls`; generic version 2 requires it.

`tests/pg_erd_admin_config_contract.rs` covers the bounded migration aggregate. Version 1 remains the historical cleartext profile, version 2 requires the response-body progress lifetime and rejects downstream TLS, and version 3 requires both the response lifetime and downstream TLS. Missing/extra/renamed authorities, unknown fields, zero budgets, listener overlap and invalid TLS references fail before listener activation.

`tests/network_authority_self_loop_contract.rs` freezes the no-self-proxy/no-metrics-recursion invariant across exact, wildcard and mapped socket aliases. Direct deserialization cannot bypass validation because public activation boundaries revalidate the aggregate.

## Downstream TLS / HTTP/2

The downstream TLS increment is tested at four layers:

- `src/downstream_tls.rs` unit tests cover deterministic absolute-path validation, non-UTF8 references and delivery failure classification;
- `src/tls_delivery.rs` unit tests cover the strict ALPN selector: H2 preference even if HTTP/1.1 is listed first, H1 fallback when H2 is absent, no-overlap rejection, empty/zero-length and overlong malformed wire vectors;
- `tests/downstream_tls_h2_config_red.rs` and the config-contract suites freeze the versioned Admin Config transition and prevent earlier versions from silently acquiring TLS;
- `tests/downstream_tls_lifecycle.rs` plus real-wire suites cover materialization, process lifecycle and the production composition roots over actual TLS sockets.

`tests/downstream_tls_h2_wire.rs` creates an ephemeral local CA and a `gateway.test` server certificate, starts the compiled generic gateway, performs certificate-verified TLS, offers ALPN `h2`, asserts that `h2` is selected, sends an actual HTTP/2 request and proves that the request traverses the gateway to the bounded upstream fixture. A second real-wire case first establishes a supported verified handshake and then connects with an ALPN-bearing client that offers only unsupported `foo`; the handshake must fail rather than continue without an application protocol. This freezes RFC 7301 no-overlap semantics at the production listener boundary.

`tests/downstream_tls_http1_fallback_wire.rs` independently proves that the same `h2_http1` policy deliberately retains HTTP/1.1 fallback when the client offers `http/1.1`. `tests/downstream_tls_pg_erd_wire.rs` proves TLS listener activation on the bounded pg-erd composition without widening its route authority. The real-wire fixtures acquire `GatewayProcess` ownership immediately after spawn, before connection/handshake, so panic and timeout paths kill and reap child processes. Negative startup uses bounded `try_wait()` and kill/reap rather than unbounded `Command::output()`.

A test-only no-overlap oracle at exact `5aebba832692debdf99a6866b22be25deea9cdb2` was followed by the causal source repair before that queued CI job acquired execution, so the old job was cancelled rather than yielding a hosted RED receipt. The RED definition is retained in history and grounded independently by RFC 7301 plus Pingora 0.9.0 protected-source `NOACK` behavior. Acceptance requires the unchanged repaired descendant to execute the same real-wire oracle GREEN; no predecessor result is transferred.

Issue #51 owns the remaining H2 production matrix. Future GREEN must add realistic parallel streams, reset/cancellation, GOAWAY/graceful drain, header/body admission, flow-control/backpressure, origin failure and recovery, readiness/forwarding trust, and new-handshake versus reused-connection timing. Basic one-stream success must not be reused for those claims.

H2-downstream to H1-upstream Cookie and zero-length body-framing tests may become release acceptance only after supplier `cloudflare/pingora#901` and `#936` have maintainer-integrated, release-qualified identities, unless a versioned deployment contract makes the affected downgrade unreachable. Mutable contributor branches are not test dependencies.

h2c is not enabled by this increment. HTTP/3/QUIC stays explicitly unsupported until a separately governed server capability exists; any H3 test plan must cover UDP exposure, QUIC handshake/path behavior, QPACK/header limits, stream/connection flow control, amplification, loss/congestion, 0-RTT replay policy and drain.

## Upstream TLS and request policy

Existing upstream trust tests remain distinct from downstream TLS. Generic and pg-erd local-CA tests verify explicit upstream SNI and certificate trust without transferring that evidence to listener termination.

`tests/protocol_transition_policy.rs`, `tests/peer_protocol_policy.rs` and `tests/protocol_transition_traffic.rs` freeze HTTP/1 Upgrade non-support at both transport-neutral admission and Pingora peer construction. WebSocket or HTTP/2 Extended CONNECT remains a separate versioned feature rather than inferred supplier behavior.

Forwarding tests require hostile request-controlled proxy identity to be removed before the gateway rebuilds only accepted transport facts. TLS-enabled paths must derive `https` from listener transport, never from request headers.

## Runtime isolation and failure traffic

Generic and pg-erd suites cover declared/streamed body limits, process-wide in-flight saturation/recovery, health availability and upstream connect/read/write/idle behavior. Pg-erd version 2/3 response lifetime is exercised with slow-drip response-body progress and kept distinct from per-read inactivity and incomplete response-header lifetime.

Failure traffic is phase-specific: refused origin, connected-silent read, orderly partial response, pre-header TCP reset and post-commit reset have separate fixtures. After a response is committed, a later failure must terminate the incomplete response rather than fabricate a second status or silently fail over; readiness and independent routes must recover.

## Shutdown and NUMA evidence

Ordinary graceful-drain tests prove already-admitted in-flight request behavior. The Pingora 0.9.0 consumer shutdown fixture separately proved parked-read correctness on the current stack. Those tests do not close issue #46's many-core contention requirement.

Sibling PR #74 owns the representative configured-worker/NUMA harness. Its evidence contract records actual process-allowed CPUs, socket/NUMA topology, configured proxy workers and service overrides, many parked/reused H1 keep-alives, repeated SIGTERM rounds, zero survivors, shutdown tail and complete stable-TID scheduler evidence. A low-core hosted pass is characterization only and must not be reported as NUMA closure.

## Observability and secret handling

Compiled-process observability tests send non-vacuous URI/query/Host/Authorization/Cookie sentinels to an origin and prove those values reached the origin while remaining absent from canonical shared logs. Pingora-family diagnostics are tested under verbose logging to ensure supplier messages cannot bypass data minimization. TLS certificate/private-key contents and private-key paths must never become request-derived labels or payload logs.

## Release and consumer migration

No PR test substitutes for release or deployment evidence. Release-ready exact protected heads require immutable artifact identity, SBOM/provenance/reproducibility and rollback evidence after all required source/security/governance gates. Consumer migration then requires unchanged parity on the actual edge, shadow/canary, observed rollback, cutover and verified legacy Nginx/OpenResty removal.
