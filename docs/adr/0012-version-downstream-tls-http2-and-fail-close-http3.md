# ADR 0012: Version downstream TLS and HTTP/2; keep HTTP/3 fail-closed

- Status: Proposed
- Date: 2026-09-04
- Owners: Ingress / TLS / HTTP Policy / Runtime Isolation / Supply Chain

## Problem

The current migration stack does not terminate downstream TLS and does not admit HTTP/2. At parent exact head `969ffd7db92776c3a2389646e81a39b79705c2e0`, both production composition roots build a Pingora proxy service and call `add_tcp(&listener)`. They do not load downstream certificate/key material, configure listener ALPN, or expose a versioned HTTP/2 admission contract. Separately, `pingora_delivery` forces every characterized upstream peer to `ALPN::H1`.

This means downstream HTTP/2 cannot be added as a listener-only feature. The first admitted H2 request would immediately exercise Pingora's HTTP/2-downstream to HTTP/1.1-upstream translation path. Public supplier issue `cloudflare/pingora#935` reports that an H2 request body ending with an empty DATA frame carrying END_STREAM can cause a zero-length H1 chunk write to emit the chunk terminator and `finish()` to emit it again, poisoning a reusable upstream connection. Public repair PR #936 changes zero-length chunked writes into no-ops but remains open and unmerged. Supplier issue #892 separately reports missing RFC 9113 Cookie normalization when multiple H2 `Cookie` fields are translated to an H1 upstream; that report is a required characterization target until the exact integrated supplier proves or repairs the behavior.

The pinned public Pingora revision `09696b51bc59315353d96686355861604d0bb48c` exposes TLS listeners and HTTP/1/HTTP/2 server sessions. That capability is sufficient to design a later downstream TLS/H2 version, but capability presence is not migration parity. The inspected public supplier does not provide a first-class HTTP/3/QUIC server contract equivalent to the H1/H2 path, so a custom ALPN label or separately vendored QUIC stack cannot be credited as HTTP/3 support.

## Constraints

- `pingora-gateway` owns reusable downstream transport admission, not certificate issuance, ACME account state, private-key backup, Keyverse identity, product authentication, or consumer business policy.
- Downstream TLS evidence is distinct from #36/#37 gateway-to-upstream TLS trust/SNI evidence.
- HTTPS HTTP/2 uses TLS ALPN. Plaintext h2c is a separate protocol contract and cannot be enabled as a shortcut for HTTPS parity.
- The current upstream contract is deliberately HTTP/1.1. Switching upstream ALPN to H2 only to avoid an H2-to-H1 RED would silently introduce a separate transport decision and is not an admissible repair.
- Mutable contributor branches are not release dependencies. Supplier fixes must arrive through an immutable maintainer-integrated identity or a separately governed, provenance-bound minimal backport.
- HTTP/3 runs over QUIC and has distinct transport, TLS, flow-control, congestion, amplification, connection-migration and 0-RTT concerns. TLS/H2 completion cannot be transferred to H3.
- Existing request-body, header-admission, forwarding-trust, retry, observability, drain, OCI and supply-chain invariants remain in force under H2; protocol activation does not weaken them.

## Options considered

### Retain Nginx/Traefik in front of Pingora and count its TLS/H2/H3 behavior as shared-gateway parity

Rejected as closure. A consumer may temporarily retain a legacy edge during an evidence-preserving migration, but external protocol termination does not prove the `pingora-gateway` artifact owns or implements that protocol contract.

### Enable h2c on the existing clear-text listener

Rejected for the HTTPS migration requirement. h2c is a distinct plaintext admission mechanism and would neither prove certificate/SNI behavior nor TLS ALPN negotiation.

### Change all upstream peers to HTTP/2

Rejected as a workaround. Upstream protocol selection has origin compatibility, pooling, failure and performance consequences and therefore requires its own versioned consumer/origin evidence. It cannot be changed merely to avoid supplier translation defects.

### Vendor a QUIC/HTTP/3 stack or pin mutable Pingora contributor branches

Rejected. This would create new security-sensitive transport ownership or mutable supplier dependency outside the accepted Shared Kernel/release boundary.

### Version downstream TLS/H2 on supported Pingora APIs and keep H3 fail-closed

Selected. Issue #51 owns this product increment. It may proceed only from consumer-derived old-edge traffic characterization, explicit downstream TLS/ALPN configuration, and exact-supplier H2-to-H1 translation evidence. HTTP/3 remains unsupported and uncredited until an immutable, reviewable transport capability is separately accepted.

## Decision

Introduce a future **Downstream TLS / Protocol Admission** bounded context rather than overloading upstream TLS, HTTP Policy, Runtime Isolation or product authentication. Its aggregate will own only the versioned listener-side transport authority needed to decide whether a process may terminate TLS and admit HTTP/1.1 and/or HTTP/2. Certificate/key references are configuration values consumed read-only; issuance, renewal and custody remain with their canonical owner.

Generic v1 and the current bounded pg-erd migration profile remain clear-text HTTP/1 listeners. No existing configuration field is reinterpreted as downstream TLS. A later versioned transition must fail before listener activation when its certificate/key references, TLS policy, SNI mapping or ALPN policy are invalid.

Before H2 can become GREEN, the exact supplier used by the candidate must pass mixed-protocol translation traffic while upstream peers remain on the characterized H1 contract. `cloudflare/pingora#935/#936` is a release prerequisite for the empty-DATA/END_STREAM keep-alive-desynchronization path unless exact integrated source and wire evidence independently prove it absent. `cloudflare/pingora#892` remains a mandatory multiple-Cookie characterization/disposition path. Callback shims are not preferred closure and must be minimal, versioned, removable and covered by the same RED/GREEN evidence if a temporary compatibility boundary is explicitly approved.

HTTP/3/QUIC remains fail-closed after TLS/H2 ships. It requires a separate ADR and immutable transport dependency with real UDP/QUIC/HTTP3 integration and deployment evidence.

## Effects and risks

The decision prevents a buyer-visible protocol checkbox from silently widening several ownership boundaries at once. It preserves certificate lifecycle, product identity and upstream protocol authority while making downstream TLS/H2 a reusable edge concern only when concrete consumers prove the need.

The main commercial cost is sequencing: H2 activation waits on old-edge characterization plus supplier translation disposition, and H3 remains unavailable from the shared artifact. The alternative would move unresolved wire-compatibility and supply-chain risk into production traffic, which is not acceptable for a canonical edge runtime.

## Verification

- RED first proves both exact current composition roots are clear-text-only: the application service is bound with `add_tcp`, no downstream TLS material is loaded, no ALPN policy is configured, and no test earns TLS/H2 parity credit.
- Consumer characterization records actual SNI/vhost, public certificate identity, ALPN result, HTTP/1.1 fallback, HTTP/2 concurrency/header/body/idle behavior, forwarding trust, health, drain and rollback. Defaults are not inferred from Nginx, Traefik or Pingora.
- GREEN uses an ephemeral local CA/certificate against the compiled candidate listener. Valid identity/SNI negotiates only admitted ALPN protocols; invalid or unrecognized authority fails closed according to the versioned contract; HTTP/1.1 fallback is deliberate.
- H2 traffic covers concurrent streams, decoded-header-list boundaries, request-body limits, cancellation/reset, GOAWAY/drain, origin failure, readiness and independent post-failure recovery.
- H2-to-H1 wire acceptance sends a streamed body ending with empty DATA+END_STREAM, requires exactly one H1 chunk terminator, then reuses the same H1 keep-alive connection for an unrelated request. Multiple H2 `Cookie` fields must arrive in the H1 context with the RFC 9113-required `; ` concatenation; pseudo/hop-by-hop translation is also observed on the wire.
- Performance evidence separates new-connection/TLS-handshake and reused-connection measurements, keeps realistic concurrency/sample sizes, includes applicable handshake/routing I/O, and does not rely on artificial warm-up to satisfy the p95 contract.
- HTTP/3 receives no GREEN from TLS/H2 tests. A future H3 candidate must independently prove UDP exposure/network policy, QUIC TLS/ALPN, QPACK/header limits, stream/connection flow control, loss/congestion behavior, amplification protection, connection migration, explicit 0-RTT replay policy, drain/close, OCI/Kubernetes exposure and rollback.
- Exact-head fmt/compile/test/clippy/warning-denied rustdoc, 100% owned-production coverage, OCI/SBOM/provenance/security/supply-chain evidence, independent review and terminal hosted execution remain mandatory.
- This ADR stays Proposed until the versioned downstream TLS/H2 source contract and its exact-head traffic evidence are integrated. Documentation and supplier issue comments alone cannot make it Accepted.

## References

Cloudflare. (n.d.). *Pingora public source* [Source code, commit `09696b51bc59315353d96686355861604d0bb48c`]. GitHub.

songhieu. (2026, July 17). *H1 upstream: chunked terminator written twice when an H2 downstream ends the request body with an empty DATA frame (END_STREAM)* [GitHub issue #935]. Cloudflare Pingora.

songhieu. (2026, July 17). *Fix: don't emit the chunked terminator for zero-length body writes* [GitHub pull request #936]. Cloudflare Pingora.

MyLittleLuckyDog. (2026, May 29). *HTTP/2 multiple Cookie headers not concatenated when proxied to HTTP/1.1 upstream (RFC 9113 §8.2.3)* [GitHub issue #892]. Cloudflare Pingora.

Internet Engineering Task Force. (2022). *HTTP/2* (RFC 9113).

Internet Engineering Task Force. (2022). *HTTP/3* (RFC 9114).

Internet Engineering Task Force. (2021). *QUIC: A UDP-Based Multiplexed and Secure Transport* (RFC 9000).

Internet Engineering Task Force. (2021). *Using TLS to Secure QUIC* (RFC 9001).

Internet Engineering Task Force. (2024). *Service Identity in TLS* (RFC 9525).

Internet Engineering Task Force. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846).

ContextualWisdomLab. (2026, September 3). *Protocol: establish downstream TLS/HTTP/2 parity and keep HTTP/3 fail-closed until supplier support exists* [GitHub issue #51].