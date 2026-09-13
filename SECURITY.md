# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, upstream certificates, and deployment inputs are untrusted. Operators must protect configuration write access. V1 has no secret-bearing configuration field; credentials must not be added casually to the YAML contract.

The gateway contacts only the socket address admitted at startup. It is not a forward proxy and accepts no request-controlled upstream URI, preventing this runtime from becoming a generic SSRF primitive.

HTTPS upstreams use certificate and hostname verification with an explicit SNI. An operator may additionally provide an absolute `trust_bundle_file`; the process reads and parses that PEM bundle before opening listeners and fails closed on missing, empty, or malformed material. This is trust-anchor consumption only. Certificate issuance, rotation, revocation workflow, downstream TLS termination, ACME, and key custody remain with their canonical owners and are not absorbed into the gateway.

Inbound forwarding identity is deleted before proxying. V1 emits only `Forwarded: proto=http`; it deliberately does not claim a client IP. A future trusted-proxy feature must define allowed proxy CIDRs/hops and RFC 7239 semantics as a versioned contract with spoofing tests.

Request bodies, concurrent non-health requests, retained upstream keepalive capacity, and upstream connect/total-connect/read/write/idle time are explicitly bounded. At `max_in_flight_requests` capacity, application traffic fails fast with HTTP 503 while health remains observable. Pingora's HTTP parser also has finite protocol/header limits, but an operator-controlled smaller HTTP/1 header byte/count budget remains a separate edge/supplier gap. Representative consumer queue/origin-capacity studies are still required before production budget values can be claimed safe.

The generic v1 process performs one total upstream attempt and therefore no automatic replay/retry. Product retry, failover and idempotency semantics remain with the owning product rather than being inferred at the edge.

## Logging and data minimization

The production path emits coarse status/outcome/request-body-byte access logs and low-cardinality Prometheus counters for requests, request errors, request-body bytes, and backpressure rejections. It does not log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, configuration credentials, arbitrary headers, route values, trust-bundle contents, or other unbounded request-derived labels. Distributed tracing and richer bounded production operability evidence remain release gaps.

## Supply chain

Pingora and `pingora-prometheus` are pinned to one exact upstream commit and must be revalidated against current releases/advisories immediately before release. `Cargo.lock` is committed. Repository CI tests and lints with `--locked`, rejects lockfile mutation, and the OCI builder copies the reviewed lock and builds with `--locked`; dependency-resolution drift therefore fails closed rather than silently rewriting release inputs.

The supply-chain lane checks dependency/advisory/license/source policy, builds exact-source SPDX SBOM evidence, builds the final image and scans it fail-closed. These mechanisms exist on the migration stack, but every moved release candidate must reacquire terminal exact-head evidence. The current public Pingora graph also contains unmaintained `derivative 2.2.0` (`RUSTSEC-2024-0388`, no patched version); release policy requires a maintainer-integrated supplier repair/removal rather than a blanket advisory ignore or mutable fork pin.

A committed lock and generated SBOM are necessary but not sufficient release evidence. Provenance/reproducibility must be bound to the protected source SHA and immutable artifact digest, and rollback must be rehearsed before any consumer cutover. No Draft PR head, contributor fork head, mutable upstream branch, or predecessor check result is a releasable dependency identity.

## Container and rollout

The current image uses a digest-pinned builder and digest-pinned distroless nonroot runtime, executes as uid/gid `65532`, has no intentional application writes, and has executable acceptance for a read-only root filesystem, dropped capabilities and `no-new-privileges`. Current-head OCI evidence still has to be terminal GREEN on the exact release candidate.

SIGTERM/drain behavior is bounded by a 5-second grace period, a 10-second per-runtime shutdown timeout, and a 30-second external supervisor budget. A release and consumer deployment must prove the exact graceful-drain/rollback contract; documentation or a successful predecessor head is not rollout evidence.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.