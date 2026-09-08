# ADR 0009: Deny HTTP/1 Upgrade at the Pingora peer boundary

- Status: Proposed
- Date: 2026-09-03
- Owner: ContextualWisdomLab/pingora-gateway

## Problem

Generic v1 and the bounded pg-erd migration candidate already reject uncharacterized HTTP/1 protocol transitions before application admission, route/upstream selection, or origin contact. However, the Pingora delivery adapter still constructed every immutable `HttpPeer` with `HttpUpstreamRequestPolicy::standard()`.

At pinned supplier revision `09696b51bc59315353d96686355861604d0bb48c`, `standard()`/`default()` retains `H1UpgradePolicy::WebSocketOnly`. Pingora therefore retains WebSocket forwarding capability below a CWL contract that deliberately admits no WebSocket policy. The current callbacks prevent that capability from being reached, but the transport object itself did not encode the same invariant. A later callback/composition refactor could consequently widen the protocol surface without changing the transport-neutral admission contract.

Fresh supplier verification on 2026-09-09 still finds protected `cloudflare/pingora/main` at `09696b51bc59315353d96686355861604d0bb48c`, issue `cloudflare/pingora#946` open, and proposed repair `cloudflare/pingora#947` open/unmerged. That supplier state reinforces fail-closed admission; it is not consumed as mutable dependency authority.

## Constraints

- WebSocket is a v1 non-goal and cannot be inferred from supplier capability.
- Standard hop-by-hop and `Connection`-nominated field sanitization must remain enabled.
- Product authentication, authorization, business routing, retry/idempotency, Wardnet/EgressWeave verdicts, and Keyverse identity remain outside this decision.
- The repair must not depend on a mutable supplier branch or weaken exact-head security/release gates.
- A lightly loaded WebSocket happy path is not acceptable parity evidence while the supplier upgrade race remains unresolved on protected public ancestry.

## Alternatives

### Keep `HttpUpstreamRequestPolicy::standard()` and rely only on request admission

Rejected. The current behavior is safe only while every composition root invokes the admission guard before transport acquisition. It leaves an avoidable mismatch between the admitted protocol contract and the immutable transport policy.

### Use `HttpUpstreamRequestPolicy::preserve()` and sanitize in application callbacks

Rejected. That would transfer hop-by-hop correctness from the transport adapter into application callbacks without a consumer requirement and would weaken the current standard sanitization boundary.

### Use `HttpUpstreamRequestPolicy::deny_upgrades()`

Selected. At the pinned supplier revision it preserves the standard hop-by-hop/connection-nomination sanitization behavior while changing the HTTP/1 upgrade policy from `WebSocketOnly` to `Deny`.

## Decision

`pingora_delivery::build_peer_from_validated` SHALL construct every generic and migration `HttpPeer` with `HttpUpstreamRequestPolicy::deny_upgrades()` for the current contract. The transport-neutral request admission rule remains authoritative for HTTP 501 behavior and the origin-contact prohibition; the peer setting is defense in depth at the Pingora anti-corruption boundary.

`tests/peer_protocol_policy.rs` SHALL lock the exact peer option independently from the existing real-listener protocol-transition traffic test. Any future WebSocket version must deliberately change both the admitted contract and the immutable peer policy with consumer-derived traffic evidence rather than inherit supplier defaults.

## Evidence

Historical RED `b0719a893c0136083efc12918c6e906a28f39319` requires a peer returned by public `build_peer()` to equal `HttpUpstreamRequestPolicy::deny_upgrades()`; its parent still configured `standard()`. Historical source fix `0109031b97ce1816a7936987a4d63330a2d19cba` demonstrated the minimal semantic repair.

The current ordinary/non-force succession must preserve final #31 as the resolution basis, reapply only this valid peer-policy source/test/documentation delta, and reacquire exact-head hosted and current-range review evidence. Predecessor execution does not transfer.

This ADR remains Proposed until the final exact PR head has terminal formatting, compile/test, Clippy, warning-denied rustdoc, 100% owned-production coverage, applicable security/supply-chain/OCI/load checks, and required review evidence. Source presence is not GREEN evidence.

## Risks and effects

Ordinary HTTP/1 proxy traffic keeps the same supplier sanitization contract. An accidental path that bypasses the application admission callback no longer forwards a WebSocket Upgrade handshake upstream. A consumer that later proves a real WebSocket requirement will need an explicit versioned change rather than relying on an implicit default.

## Follow-up

- Reacquire exact-head hosted evidence without gate weakening or no-op retrigger churn.
- Keep `TRD.md`, `ARCHITECTURE.md`, `SECURITY.md`, `TEST_STRATEGY.md`, `CHANGELOG.md`, TRACEABILITY, and the product/technical gap baseline synchronized with the final source head.
- Do not claim WebSocket, HTTP/2 Extended CONNECT, HTTP/3/QUIC, parity, canary, cutover, or legacy removal from this decision.
