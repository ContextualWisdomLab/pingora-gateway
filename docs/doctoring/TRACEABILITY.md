# Primary-Source and APA-7 Traceability

This document maps material edge-runtime, protocol, toolchain, container, and supplier claims to primary standards or first-party upstream evidence. Version, advisory, protected-head, and release claims are revalidated before promotion. Mutable contributor PRs are evidence candidates, not release authority.

## Current supplier and implementation traceability

| Claim | Primary evidence | Acceptance consequence |
| --- | --- | --- |
| Pingora 0.9.0 is the current published supplier release observed on 2026-09-10 KST | Cloudflare Pingora GitHub Release `0.9.0`, published 2026-09-09T23:34:48Z; protected release-source commit `702f69015e53f7244d6ad2e743de571d859a70a4` | Release-name existence is not blanket migration credit. The exact consumer dependency identity and unchanged gateway behavior must still be proven. |
| Pingora 0.9.0 contains the graceful-shutdown lost-wakeup repair | Pingora 0.9.0 release notes: proxy shutdown notifications are sharded and the graceful-shutdown lost-wakeup race is closed | Historical #844/#969 becomes provenance for correctness; gateway #70 moves to exact registry pin/lock transition and unchanged downstream GREEN. |
| Released 0.9.0 does not expose CWL's required configurable H1 request-header byte/count admission | Released `pingora-core` HTTP/1 source at `702f690...`; downstream #72 executable real-socket RED | Callback-only rejection is not equivalent to parser admission. A later release-qualified supplier capability is required. |
| Current upstream parser-admission candidate repairs the seven CWL findings at mutable-candidate scope | `cloudflare/pingora#1000@6a90c79b61fbbc70b518709de6802165668cba2c`, four-file current range; exact-current CWL COMMENT review `5162454757` | Candidate source may advance owner-path confidence but cannot be pinned or treated as maintainer-integrated/released authority. Exact build, maintainer integration, later release, then unchanged #72 GREEN remain required. |
| Released 0.9.0 still lacks a supported monotonic whole-request-header deadline | Released H1 request-read path at `702f690...`; open upstream #447; downstream #71 fresh/reused real-socket RED | Per-read inactivity timeout does not satisfy a whole-header lifetime contract. |
| Pingora 0.9.0 still carries `derivative 2.2.0` in the relevant graph | Released/tagged workspace source at `702f690...`; open upstream #889; RustSec RUSTSEC-2024-0388 | #54 remains intentionally RED until a later release-qualified supplier identity removes the package and the consumer lock is regenerated. |
| Released 0.9.0 does not close H2→H1 Cookie coalescing | Released proxy sanitizer at `702f690...`; downstream #53 real TLS/H2→H1 wire RED; open contributor #901 | Multiple H2 Cookie fields must be reconstructed as one H1 Cookie field using the protocol-defined delimiter before release credit. |
| Zero-length application writes must not own the H1 chunk terminator | Pingora issue/PR lineage #935/#936; contributor #936 remains open | `finish()` must remain the sole chunk-terminator owner; async and cancel-safe write paths require regression evidence before release credit. |

## Protocol and security standards

| Area | Primary standard | Gateway use |
| --- | --- | --- |
| HTTP semantics | RFC 9110 | Method/status/header semantics and intermediary obligations. |
| HTTP/1.1 | RFC 9112 | Message parsing/framing, connection handling, chunked coding and hop-by-hop correctness. |
| HTTP/2 | RFC 9113 | H2 framing/connection rules; Section 8.2.3 permits Cookie field splitting and requires recombination before a non-H2 hop. |
| QUIC transport | RFC 9000 | Transport basis for HTTP/3 acceptance. |
| HTTP/3 | RFC 9114 | HTTP semantics over QUIC; no H3 product credit without executable interoperability evidence. |
| WebSocket | RFC 6455 | Upgrade/framing semantics for HTTP/1.1 WebSocket acceptance. |
| WebSocket over HTTP/2 | RFC 8441 | Extended CONNECT path when H2 WebSocket support is in scope. |
| WebSocket over HTTP/3 | RFC 9220 | Extended CONNECT/bootstrap path when H3 WebSocket support is in scope. |
| Forwarded header | RFC 7239 | Forwarding grammar; request-supplied client identity remains untrusted until an explicit trust contract admits it. |
| TLS 1.3 | RFC 9846 (July 2026) | Current TLS 1.3 protocol specification; it obsoletes RFC 8446 and leaves application identity verification to the application protocol profile. |
| TLS service identity | RFC 9525 | Certificate/service identity verification guidance for TLS applications. |
| New-protocol TLS baseline | RFC 9852 / BCP 195 (July 2026) | New protocols using TLS must require TLS 1.3; existing gateway compatibility still requires explicit product/deployment evidence rather than silent policy widening. |

## Rust, Cargo, OCI, and supply-chain authority

| Claim | Primary evidence | Acceptance consequence |
| --- | --- | --- |
| Rust 1.98.1 repairs the Rust 1.98.0 vtable-generation miscompilation | Rust Release Team, *Announcing Rust 1.98.1*, 2026-09-03 | Release-producing gateway paths select/verify 1.98.1; #56 remains a separately governed prerequisite until normally integrated. |
| `derivative` is unmaintained and RUSTSEC-2024-0388 has no patched versions | RustSec Advisory Database, RUSTSEC-2024-0388 | Commercial supplier-intake policy requires removal/replacement, not an audit ignore. |
| Cargo lock/source/checksum evidence must be resolver-generated | Cargo Book: dependency resolution, lockfiles, registries and `--locked` behavior | A manifest-only Pingora transition or hand-authored `Cargo.lock` is invalid; registry source/checksum authority must come from Cargo resolution. |
| OCI runtime-spec v1.3.0 is the latest runtime-spec release observed during the 2026-09-10 refresh | Open Container Initiative, `runtime-spec` releases | Rootless/read-only/capability/seccomp/AppArmor/SELinux and lifecycle claims are tested against actual runtime behavior; the spec version alone is not runtime hardening evidence. |
| OCI image-spec v1.1.1 is the latest image-spec release observed during the 2026-09-10 refresh | Open Container Initiative, `image-spec` releases | Immutable image digest/config/layer semantics use the current image format authority. |
| OCI distribution-spec v1.1.1 is the latest distribution-spec release observed during the 2026-09-10 refresh | Open Container Initiative, `distribution-spec` releases | Registry publication/retrieval claims require digest-bound artifacts; tag names alone are not immutable deployment identity. |

## Gateway evidence doctrine

The repository distinguishes four evidence classes:

1. **source characterization** — exact code/spec evidence explaining why a RED or invariant exists;
2. **candidate execution** — exact-head tests/checks on a mutable branch;
3. **release authority** — maintainer/protected integration plus immutable/versioned dependency or gateway artifact identity;
4. **deployment evidence** — exact artifact exercised in parity, shadow/canary, rollback and cutover traffic.

A later class is never inferred solely from an earlier one. In particular, contributor PR CI is not release authority, a GitHub Release label is not a consumer lock, a loopback p95 is not WAN/TLS production SLO evidence, and disappearance of Nginx/OpenResty strings is not migration completion.

## References

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Cloudflare. (2026, September 9). *Pingora 0.9.0* [Software release]. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.9.0

Cloudflare. (n.d.). *Pingora configurable H1 request-header limits* (Pull request #1000). GitHub. https://github.com/cloudflare/pingora/pull/1000

Cloudflare. (n.d.). *Support for configurable timeout for downstream connections in HTTP/1.1* (Issue #447). GitHub. https://github.com/cloudflare/pingora/issues/447

Cloudflare. (n.d.). *RUSTSEC-2024-0388: derivative is unmaintained* (Issue #889). GitHub. https://github.com/cloudflare/pingora/issues/889

Cloudflare. (n.d.). *Concatenate HTTP/2 Cookie headers when proxying to HTTP/1.1* (Pull request #901). GitHub. https://github.com/cloudflare/pingora/pull/901

Cloudflare. (n.d.). *Do not emit the chunked terminator for zero-length body writes* (Pull request #936). GitHub. https://github.com/cloudflare/pingora/pull/936

Fette, I., & Melnikov, A. (2011). *The WebSocket protocol* (RFC 6455). RFC Editor. https://www.rfc-editor.org/rfc/rfc6455

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Internet Security Research Group. (n.d.). *RustSec advisory database: RUSTSEC-2024-0388*. https://rustsec.org/advisories/RUSTSEC-2024-0388.html

McManus, P. (2018). *Bootstrapping WebSockets with HTTP/2* (RFC 8441). RFC Editor. https://www.rfc-editor.org/rfc/rfc8441

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Open Container Initiative. (2025, March 3). *OCI image format specification v1.1.1* [Software specification release]. GitHub. https://github.com/opencontainers/image-spec/releases/tag/v1.1.1

Open Container Initiative. (2025, November 4). *OCI runtime specification v1.3.0* [Software specification release]. GitHub. https://github.com/opencontainers/runtime-spec/releases/tag/v1.3.0

Open Container Initiative. (n.d.). *OCI distribution specification v1.1.1* [Software specification release]. GitHub. https://github.com/opencontainers/distribution-spec/releases/tag/v1.1.1

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Rust Release Team. (2026, September 3). *Announcing Rust 1.98.1*. Rust Blog. https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/

Salowey, J., Zhou, H., Eronen, P., & Tschofenig, H. (2022). *Bootstrapping WebSockets with HTTP/3* (RFC 9220). RFC Editor. https://www.rfc-editor.org/rfc/rfc9220

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Thomson, M., & Turner, S. (2024). *Recommendations for secure use of transport layer security (TLS) and datagram transport layer security (DTLS)* (RFC 9525). RFC Editor. https://www.rfc-editor.org/rfc/rfc9525

Thomson, M., & Turner, S. (2021). *QUIC: A UDP-based multiplexed and secure transport* (RFC 9000). RFC Editor. https://www.rfc-editor.org/rfc/rfc9000

The Cargo Project Developers. (n.d.). *Cargo Book*. https://doc.rust-lang.org/cargo/
