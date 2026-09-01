# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, upstream certificates, and deployment inputs are untrusted. Operators must protect configuration write access. V1 has no secret-bearing configuration field; credentials must not be added casually to the YAML contract.

The gateway contacts only the socket address admitted at startup. It is not a forward proxy and accepts no request-controlled upstream URI, preventing this runtime from becoming a generic SSRF primitive.

HTTPS upstreams use certificate and hostname verification with an explicit SNI. An operator may additionally provide an absolute `trust_bundle_file`; the process reads and parses that PEM bundle before opening listeners and fails closed on missing, empty, or malformed material. This is trust-anchor consumption only. Certificate issuance, rotation, revocation workflow, downstream TLS termination, ACME, and key custody remain with their canonical owners and are not absorbed into the gateway.

Inbound forwarding identity is deleted before proxying. V1 emits only `Forwarded: proto=http`; it deliberately does not claim a client IP. A future trusted-proxy feature must define allowed proxy CIDRs/hops and RFC 7239 semantics as a versioned contract with spoofing tests.

Request bodies and upstream connect/read/write/idle time are bounded. The Pingora HTTP parser has finite protocol/header limits, but a smaller configurable header budget and an explicit concurrency/backpressure budget remain documented gaps.

## Logging and data minimization

The production path emits coarse status/outcome/request-body-byte access logs and label-free Prometheus request/error/body-byte counters. It does not log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, configuration credentials, arbitrary headers, route values, trust-bundle contents, or other unbounded request-derived labels. Distributed tracing and richer bounded operability evidence remain release gaps.

## Supply chain

Pingora and `pingora-prometheus` are pinned to one exact upstream commit and must be revalidated against current releases/advisories immediately before release. `Cargo.lock` is committed. Repository CI tests and lints with `--locked`, rejects lockfile mutation, and the OCI builder copies the reviewed lock and builds with `--locked`; any dependency-resolution drift therefore fails closed rather than silently rewriting release inputs.

A committed lock is necessary but not sufficient release evidence. The exact resolved graph still requires vulnerability/license policy evidence, SBOM and provenance generation bound to the protected source SHA and artifact digest, container scanning, signing/attestation policy, and an immutable published image digest before a consumer cutover may rely on this runtime.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.
