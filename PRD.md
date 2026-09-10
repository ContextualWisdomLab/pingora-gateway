# Product Requirements — CWL Pingora Gateway

## Problem

CWL repositories contain CWL-managed Nginx/OpenResty/legacy reverse-proxy boundaries with duplicated proxy/ingress behavior and uneven security evidence. Replacing those runtimes safely requires one reusable edge-runtime owner that is narrow enough not to absorb consumer business policy, identity authority or security-verdict ownership.

## Users

Primary users are CWL product maintainers and platform operators migrating an owned reverse-proxy boundary. Security, architecture and governance reviewers consume the same configuration, test, release and operational evidence.

## First release-quality vertical

The first release-quality vertical SHALL:

- parse a strict versioned configuration before opening network authority;
- run a Rust/Pingora HTTP proxy as a non-root process on an explicit listener;
- admit exactly one explicit upstream per generic process, with HTTP or verified HTTPS upstream transport;
- require positive connect/total-connect/read/write/idle budgets;
- bound request bodies and process-wide admitted concurrency;
- make data-plane worker topology explicit and bounded rather than inferred from host CPU count;
- serve liveness/readiness through the production Pingora path;
- use Pingora's standards-oriented hop-by-hop policy while distrusting inbound forwarding identity;
- shut down through Pingora's graceful lifecycle and preserve explicit external recovery budgets;
- support a read-only-root container layout; and
- ship code-current architecture, security, test and operating evidence sufficient to evaluate a consumer migration without reading implementation internals first.

## Versioned downstream TLS / HTTP/2 increment

The HTTPS migration increment SHALL preserve cleartext semantics of earlier Admin Config versions and add downstream TLS only through an explicit new version. Generic version 2 and pg-erd version 3 SHALL:

- consume an already-provisioned certificate chain and private key by read-only filesystem reference;
- fail before listener activation on invalid references, unreadable material or certificate/private-key mismatch;
- keep certificate issuance, renewal, ACME, revocation workflow, backup and key custody outside the gateway;
- use a maintainer-supported Pingora TLS listener API;
- explicitly negotiate HTTP/2 over TLS while retaining only the intended HTTP/1.1 fallback;
- keep h2c and HTTP/3/QUIC fail-closed;
- preserve product auth/business routing, Keyverse identity and Wardnet/EgressWeave authority boundaries; and
- prove the admitted path with real TLS sockets and exact-head evidence.

Basic TLS/H2 listener capability is not complete HTTP/2 parity. Issue #51 remains the product acceptance authority for parallel streams, reset/cancellation, GOAWAY/drain, header/body admission, flow control/backpressure, origin failure/recovery, forwarding trust and cutover observability. H2-downstream to H1-upstream Cookie and zero-length body-framing correctness remain gated by release-qualified supplier disposition of `cloudflare/pingora#901` and `#936`.

## Non-goals

The gateway does not own product-specific routes except bounded migration characterization, CDN/business logic, certificate lifecycle, product authentication/authorization, Keyverse identity, Wardnet/EgressWeave verdicts, arbitrary forward-proxy destinations, generic service discovery, or user-controlled upstream selection.

WebSocket/HTTP Upgrade, HTTP/2 Extended CONNECT, h2c, HTTP/3/QUIC, generic load balancing, dynamic reload and Kubernetes Gateway API are separate versioned increments. Their existence in Pingora or another dependency is not admission into this product contract.

## Performance outcome

Applicable buyer-visible paths target p95 `<20 ms` under realistic declared load. Controlled loopback is regression evidence only. TLS/H2 performance evidence must distinguish new TLS handshakes from reused connections and retain the actual routing/TLS I/O in the measurement. Worker/NUMA claims require the representative topology evidence tracked by issue #46; sample/concurrency reduction or warm-only measurement cannot manufacture acceptance.

## Release and migration gates

A release is blocked until the exact protected head passes required tests/checks, independent review/governance is satisfied, dependency/advisory roots are release-qualified, OCI candidates run non-root/read-only-root, and version/CHANGELOG/tag/package plus immutable artifact digest, SBOM/provenance/reproducibility and rollback evidence exist.

A consumer migration is blocked until that immutable release is pinned by the deployment owner and the concrete edge proves parity, shadow/canary behavior and observed rollback before cutover. Legacy Nginx/OpenResty removal is credited only after the migrated traffic state is verified; deleting configuration or landing source code is not migration completion.
