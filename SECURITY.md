# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, upstream certificates, and deployment inputs are untrusted. Operators must protect configuration write access. V1 has no secret-bearing configuration field; credentials must not be added casually to the YAML contract.

The gateway contacts only the socket address admitted at startup. It is not a forward proxy and accepts no request-controlled upstream URI, preventing this runtime from becoming a generic SSRF primitive. Admitted upstream transport authority must also remain separate from the gateway's own traffic and metrics listeners. Effective overlap is checked with the same conservative wildcard/dual-stack semantics used for listener collision, so an operator cannot configure an origin that recursively points application traffic back into the gateway listener or exposes the internal Prometheus service by selecting the metrics socket as an upstream. Distinct concrete IP authorities on the same port remain valid; this invariant does not create product routing policy.

HTTPS upstreams use certificate and hostname verification with an explicit SNI. An operator may additionally provide an absolute `trust_bundle_file`; the process reads and parses that PEM bundle before opening listeners and fails closed on missing, empty, or malformed material. This is trust-anchor consumption only. Certificate issuance, rotation, revocation workflow, downstream TLS termination, ACME, and key custody remain with their canonical owners and are not absorbed into the gateway.

Inbound forwarding identity is deleted before proxying. Generic v1 strips request-controlled `Forwarded`, `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Port`, `X-Forwarded-Proto`, `X-Forwarded-Server`, and `X-Real-IP`, then emits only gateway-owned `Forwarded: proto=http`; it deliberately does not claim a client IP. A future trusted-proxy feature must define allowed proxy CIDRs/hops and RFC 7239 semantics as a versioned contract with spoofing tests.

HTTP/1 protocol transition is denied at two independent gateway-owned boundaries. The request admission layer returns 501 for an `Upgrade` field or an exact `upgrade` token in `Connection` before application admission or origin contact. The immutable Pingora peer is also built with `HttpUpstreamRequestPolicy::deny_upgrades()`, which preserves Pingora's standard hop-by-hop and connection-nomination sanitization while setting `H1UpgradePolicy::Deny`. This prevents a later callback/composition refactor from silently inheriting Pingora's default `WebSocketOnly` forwarding policy beneath the admitted contract. It does not make WebSocket a supported feature; any such capability requires a new versioned contract and supplier/consumer evidence.

Request bodies and upstream connect/read/write/idle time are bounded. The Pingora HTTP parser has finite protocol/header limits, but a smaller configurable header budget remains a documented gap. Process-wide concurrent application traffic is bounded by the mandatory `max_in_flight_requests` admission budget and fails fast with 503 rather than queueing unbounded work.

The transport-neutral `http_policy` candidate characterizes edge-owned response headers without activating them in v1. It treats field names ASCII case-insensitively, rejects duplicate field authority, empty values, and CR/LF values before activation. Its current name profile is deliberately narrower than the full legal HTTP field-name grammar because only observed migration contracts are admitted. This policy does not absorb product authorization/business response semantics, Wardnet/EgressWeave verdicts, or Keyverse identity.

## Logging and data minimization

The production path emits coarse status/outcome/request-body-byte access logs and label-free Prometheus request/error/body-byte counters. It does not log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, configuration credentials, arbitrary headers, route values, trust-bundle contents, or other unbounded request-derived labels. Distributed tracing and richer bounded operability evidence remain release gaps.

That guarantee applies to the whole gateway process, not only the CWL `observability` target. Operator-selected `RUST_LOG` verbosity is parsed normally, but records from Pingora-family dependency targets are passed through `logging_policy` before formatting: their message bodies are replaced with a static diagnostic marker while level and target remain observable. This prevents pinned-supplier debug/trace/error paths that format request URI, Host, headers, cookies, credentials, or other request-derived material from bypassing the gateway's data-minimization contract. Consumer/product loggers remain outside this process boundary and retain their own owners and policies.

## Supply chain

Pingora and `pingora-prometheus` are pinned to one exact upstream commit and must be revalidated against current releases/advisories immediately before release. `Cargo.lock` is committed. Repository CI tests and lints with `--locked`, rejects lockfile mutation, and the OCI builder copies the reviewed lock and builds with `--locked`; any dependency-resolution drift therefore fails closed rather than silently rewriting release inputs.

A committed lock is necessary but not sufficient release evidence. The exact resolved graph still requires vulnerability/license policy evidence, SBOM and provenance generation bound to the protected source SHA and artifact digest, container scanning, signing/attestation policy, and an immutable published image digest before a consumer cutover may rely on this runtime.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.
