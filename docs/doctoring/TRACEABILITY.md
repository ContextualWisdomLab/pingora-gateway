# Primary-Source and APA-7 Traceability

This file links material technical/security claims to primary standards or upstream sources. Revalidate version/advisory claims immediately before release.

| Claim | Source |
| --- | --- |
| Pingora server/proxy composition and graceful server lifecycle | Cloudflare Pingora source at pinned commit `09696b51bc59315353d96686355861604d0bb48c`, the protected upstream `main` head revalidated on 2026-09-03 |
| Pinned Pingora diagnostics can contain request-derived data outside CWL's callback logging vocabulary: proxy TRACE formats the full downstream `RequestHeader`, the HTTP/1 client TRACE formats the serialized upstream request header, and Pingora request summaries contain method/path/Host | `pingora-proxy/src/lib.rs`, `pingora-core/src/protocols/http/v1/client.rs`, and `pingora-core/src/protocols/http/v1/server.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; this is the source basis for process-wide Pingora diagnostic message redaction in `logging_policy` |
| Pingora downstream sessions expose accepted client/server socket addresses; Pingora socket addresses expose IP socket values through `as_inet()` | `pingora-core/src/protocols/http/server.rs`, `pingora-proxy/tests/utils/server_utils.rs`, and `pingora-core/src/protocols/l4/socket.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; these are the transport observations used by the pg-erd forwarding adapter |
| Pingora's server default `max_retries` is 16, while the proxy loop copies that field and loops while its attempt counter is below the value | `pingora-core/src/server/configuration/mod.rs` and `pingora-proxy/src/lib.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; CWL v1 therefore sets the field to `1` for one total attempt |
| Graceful SIGTERM uses `grace_period_seconds` and `graceful_shutdown_timeout_seconds`, with framework fallbacks when unset | `pingora-core/src/server/mod.rs` and `pingora-core/src/server/configuration/mod.rs` at the pinned commit; CWL v1 sets 5 s grace and 10 s per-runtime graceful timeout explicitly inside a 30 s external termination budget |
| Pingora standard upstream request policy contains HTTP/1 hop-by-hop/connection-nominated handling and supplier WebSocket-upgrade support, but supplier capability is not itself an admitted CWL protocol-transition contract | Cloudflare Pingora `HttpUpstreamRequestPolicy` / peer implementation at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; generic v1 and the bounded pg-erd candidate reject uncharacterized HTTP/1 Upgrade before origin contact |
| At the pinned Pingora revision, `HttpUpstreamRequestPolicy::default()`/`standard()` strips standard hop-by-hop and connection-nominated fields, rejects malformed nominations, but sets `H1UpgradePolicy::WebSocketOnly`; `deny_upgrades()` preserves those default sanitization fields while setting the upgrade policy to `Deny` | `pingora-core/src/upstreams/peer.rs` and `pingora-proxy/src/proxy_common.rs` at commit `09696b51bc59315353d96686355861604d0bb48c`; this is the exact supplier basis for CWL peer-level Upgrade denial in `pingora_delivery` |
| Pingora 0.8.1 has an open scheduler-dependent HTTP/1 Upgrade/WebSocket tunnel teardown where an upstream `101` can be observed before the request's empty-body completion task; the reported 2-CPU Linux reproduction survived only 34/40 upgrades while idle 10-core macOS survived 40/40 | Cloudflare Pingora issue #946, opened 2026-07-30 and revalidated open on 2026-09-03 |
| Proposed supplier repair for the HTTP/1 Upgrade tunnel race exists but is not merged | Cloudflare Pingora PR #947, head `1e8488b0627370831832744fc6e65614396c310d`, open/non-Draft and unmerged when revalidated 2026-09-03 |
| HTTP/1.1 `Upgrade` is an optional connection-wide protocol transition; a server may decline it | RFC 9110 §7.8 |
| WebSocket over HTTP/1.1 uses a GET opening handshake with `Upgrade: websocket` / `Connection: Upgrade` and requires a successful `101 Switching Protocols` before the connection enters the WebSocket protocol | RFC 6455 §§1.2, 4 |
| WebSocket over HTTP/2 is a distinct Extended CONNECT mechanism rather than HTTP/1 connection-wide Upgrade | RFC 8441 §§3-5 |
| Current IETF security guidance for optimistic HTTP/1.1 protocol transitions identifies request-smuggling/parser risks, confirms rejected upgrades are normal, and specifically notes that RFC 6455 forbids optimistic WebSocket data before the server response | RFC 9931, March 2026, especially §§3-8 |
| Pingora `read_timeout` is a per-individual-read inactivity budget and resets after each successful upstream `read()`; it is not a total-response lifetime bound | Cloudflare Pingora `docs/user_guide/peer.md` and `pingora-proxy/src/proxy_h1.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; the pg-erd read-stall acceptance therefore characterizes a connected origin that sends no response bytes and deliberately does not claim slow-drip/whole-response bounding |
| A proxy failure after the upstream response header has already been sent downstream cannot be replaced with a new error response or failover; Pingora logs/surfaces the error and gives up that request | Cloudflare Pingora `docs/user_guide/failover.md` and `pingora-proxy/src/proxy_h1.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; this phase boundary is the basis of the dedicated pg-erd partial-response traffic contract |
| Pingora HTTP/1 body framing treats a body that ends before its declared `Content-Length` as `PREMATURE_BODY_END`, while upstream read failures are propagated as failed proxy tasks | `pingora-core/src/protocols/http/v1/body.rs`, `pingora-core/src/protocols/http/v1/client.rs`, and `pingora-proxy/src/proxy_h1.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c`; RFC 9112 defines HTTP/1.1 message framing requirements |
| Pingora OpenSSL peers support a per-peer CA store; when configured it replaces the verification store for that peer while certificate and hostname verification remain separately enabled | `pingora-core/src/upstreams/peer.rs`, `pingora-core/src/connectors/tls/boringssl_openssl/mod.rs`, and `pingora-core/src/protocols/tls/boringssl_openssl/mod.rs` at pinned commit `09696b51bc59315353d96686355861604d0bb48c` |
| Traefik normally adds `X-Forwarded-For`, `X-Real-Ip`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, and `X-Forwarded-Server` when proxying HTTP | Traefik official Getting Started FAQ, current documentation revalidated 2026-09-02 |
| Incoming Traefik `X-Forwarded-*` identity is trusted only when an EntryPoint explicitly configures trusted IPs or insecure trust; insecure mode is not recommended for production | Traefik official EntryPoints documentation, current documentation revalidated 2026-09-02 |
| `pg-erd-cloud` can use `X-Forwarded-For` for rate-limit/observability client identity only under an explicit trust switch and tells operators to enable it only behind a sanitizing ingress | `ContextualWisdomLab/pg-erd-cloud@8dc746920c12988f082e914879d95e13c9693535`: `.env.example`, `backend/app/rate_limit.py`, `backend/app/observability.py`, `docs/api-security-checklist.md` |
| Forwarded-header grammar and trust semantics | RFC 7239 |
| HTTP semantics | RFC 9110 |
| HTTP/1.1 message framing/hop-by-hop requirements | RFC 9112 |
| HTTP/2 framing and connection semantics | RFC 9113 |
| HTTP/3 semantics over QUIC | RFC 9114; HTTP/3 is not claimed implemented by this v1 candidate until executable listener/interoperability evidence exists |
| Current TLS 1.3 protocol semantics and application identity-verification responsibility | RFC 9846, published July 2026, which obsoletes RFC 8446 and points applications to RFC 9525 for identity verification |
| New protocols using TLS must require TLS 1.3 | RFC 9852, BCP 195, July 2026; this gateway is not claiming a new application protocol and still requires explicit migration-time protocol compatibility evidence |
| March 2026 Pingora request-smuggling/cache-key advisories are patched in 0.8.0 | GitHub Security Advisories GHSA-xq2h-p299-vjwv, GHSA-hj7x-879w-vrp7, GHSA-f93w-pcj3-rggc |
| Pingora 0.8.1 remains the latest GitHub release revalidated on 2026-09-03 and bounds default HTTP/2 server limits | Cloudflare Pingora GitHub Releases, 0.8.1, 2026-06-04 |
| The pinned upstream head is seven commits after the prior security-resolution pin `6463ad6407a1d3fe256f1951dd0ecb054477e3f6`; the relevant retry/grace configuration remains unchanged at the new head | GitHub compare `6463ad6...09696b5` plus the exact `ServerConf` source at `09696b5` |
| Rust 1.98.0 is the latest stable toolchain observed on 2026-09-01 | Rust Release Team, Rust 1.98.0 announcement, 2026-08-20 |
| OCI runtime-spec 1.3.0 is the latest released runtime specification observed on 2026-09-01 | Open Container Initiative runtime-spec v1.3.0 release notice, 2025-11-04; runtime hardening claims still require executable container evidence |
| `lru` versions before 0.18.2 are affected by RUSTSEC-2026-0253 | RustSec advisory RUSTSEC-2026-0253; the upstream pin includes the first-fixed `lru` dependency change, but release must use a committed audited lock |

## References

Cloudflare. (2026, June 4). *Pingora 0.8.1*. GitHub. https://github.com/cloudflare/pingora/releases/tag/0.8.1

Cloudflare. (n.d.). *Pingora upstream peer options* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/upstreams/peer.rs

Cloudflare. (n.d.). *Pingora HTTP/1 upstream request sanitization* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/proxy_common.rs

Cloudflare. (n.d.). *Peer: how to connect to upstream* [Documentation, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/docs/user_guide/peer.md

Cloudflare. (n.d.). *Handling failures and failover* [Documentation, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/docs/user_guide/failover.md

Cloudflare. (n.d.). *Pingora HTTP/1 proxy implementation* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/proxy_h1.rs

Cloudflare. (n.d.). *Pingora HTTP/1 client session* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/v1/client.rs

Cloudflare. (n.d.). *Pingora HTTP/1 server session* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/v1/server.rs

Cloudflare. (n.d.). *Pingora HTTP/1 body framing* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/v1/body.rs

Cloudflare. (n.d.). *Pingora OpenSSL upstream TLS connector* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/connectors/tls/boringssl_openssl/mod.rs

Cloudflare. (n.d.). *Pingora downstream HTTP session* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/server.rs

Cloudflare. (n.d.). *Pingora L4 socket address* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/l4/socket.rs

Cloudflare. (n.d.). *Pingora server configuration* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/configuration/mod.rs

Cloudflare. (n.d.). *Pingora server lifecycle* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/server/mod.rs

Cloudflare. (n.d.). *Pingora proxy implementation* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-proxy/src/lib.rs

dorianverlaine. (2026, July 30). *HTTP/1 upgrade torn down when the upstream's 101 is read before the request's empty body* [GitHub issue #946]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/946

dorianverlaine. (2026, August 4). *Keep an upgraded tunnel open when the request body ends after 101* [GitHub pull request #947]. Cloudflare Pingora. https://github.com/cloudflare/pingora/pull/947

Cloudflare. (2026). *HTTP request smuggling via premature upgrade* (GHSA-xq2h-p299-vjwv). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-xq2h-p299-vjwv

Cloudflare. (2026). *HTTP request smuggling via HTTP/1.0 and Transfer-Encoding misparsing* (GHSA-hj7x-879w-vrp7). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-hj7x-879w-vrp7

Cloudflare. (2026). *Cache key poisoning advisory* (GHSA-f93w-pcj3-rggc). GitHub Security Advisories. https://github.com/cloudflare/pingora/security/advisories/GHSA-f93w-pcj3-rggc

Traefik Labs. (n.d.). *Traefik getting started FAQ: Forwarded headers when proxying HTTP requests*. https://doc.traefik.io/traefik/getting-started/faq/

Traefik Labs. (n.d.). *Traefik EntryPoints: Forwarded headers*. https://doc.traefik.io/traefik/reference/install-configuration/entrypoints/

Fielding, R., Nottingham, M., & Reschke, J. (2022). *HTTP semantics* (RFC 9110). RFC Editor. https://www.rfc-editor.org/rfc/rfc9110

Nottingham, M. (2022). *HTTP/1.1* (RFC 9112). RFC Editor. https://www.rfc-editor.org/rfc/rfc9112

Fette, I., & Melnikov, A. (2011). *The WebSocket protocol* (RFC 6455). RFC Editor. https://www.rfc-editor.org/rfc/rfc6455

McManus, P. (2018). *Bootstrapping WebSockets with HTTP/2* (RFC 8441). RFC Editor. https://www.rfc-editor.org/rfc/rfc8441

Schwartz, B. M. (2026). *Security considerations for optimistic protocol transitions in HTTP/1.1* (RFC 9931). RFC Editor. https://www.rfc-editor.org/rfc/rfc9931

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113

Bishop, M. (2022). *HTTP/3* (RFC 9114). RFC Editor. https://www.rfc-editor.org/rfc/rfc9114

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Salz, R., & Aviram, N. (2026). *New protocols using TLS must require TLS 1.3* (RFC 9852, BCP 195). RFC Editor. https://www.rfc-editor.org/rfc/rfc9852

Petersson, A., & Nilsson, M. (2014). *Forwarded HTTP extension* (RFC 7239). RFC Editor. https://www.rfc-editor.org/rfc/rfc7239

Open Container Initiative. (2025, November 4). *OCI runtime-spec v1.3.0 release notice*. https://opencontainers.org/release-notices/v1-3-0-runtime-spec/

Rust Release Team. (2026, August 20). *Announcing Rust 1.98.0*. Rust Blog. https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/

Rust Secure Code Working Group. (2026, August 11). *RUSTSEC-2026-0253: lru—memory safety issue under panic*. RustSec Advisory Database. https://rustsec.org/advisories/RUSTSEC-2026-0253.html
