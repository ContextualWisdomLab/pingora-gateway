# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, upstream certificates, and deployment inputs are untrusted. Operators must protect configuration write access. V1 has no secret-bearing configuration field; credentials must not be added casually to the YAML contract.

The generic gateway contacts only its startup-admitted upstream socket. The bounded pg-erd migration profile contacts only explicit prevalidated `backend` and `frontend` authorities admitted by its compiled migration plan. Neither accepts a request-controlled upstream URI, so this runtime does not become a generic forward-proxy or SSRF primitive. Startup validation also rejects any admitted upstream whose effective socket authority overlaps the process traffic listener or metrics listener after wildcard/native/IPv4-mapped canonicalization; this prevents recursive self-proxying and prevents a misconfigured application route from exposing the process-owned metrics surface through the public proxy path.

HTTPS upstreams use certificate and hostname verification with an explicit SNI. An operator may additionally provide an absolute `trust_bundle_file`; the process reads and parses that PEM bundle before opening listeners and fails closed on missing, empty, or malformed material. This is trust-anchor consumption only. Certificate issuance, rotation, revocation workflow, downstream TLS termination, ACME, and key custody remain with their canonical owners and are not absorbed into the gateway.

Inbound forwarding identity is deleted before proxying. Generic v1 emits only `Forwarded: proto=http`; it deliberately does not claim a client IP. The pg-erd profile separately rebuilds only its characterized compatibility-forwarding fields from accepted client transport and Host/scheme authority. A future generic trusted-proxy feature must define allowed proxy CIDRs/hops and RFC 7239 semantics as a versioned contract with spoofing tests.

Request bodies and upstream connect/read/write/idle time are bounded. The Pingora HTTP parser has finite protocol/header limits, but a smaller configurable header budget remains a documented gap. Process-wide concurrent application traffic is bounded by the mandatory `max_in_flight_requests` admission budget and fails fast with 503 rather than queueing unbounded work.

The transport-neutral `http_policy` candidate characterizes edge-owned response headers without activating them in generic v1. It treats field names ASCII case-insensitively, rejects duplicate field authority and empty values, and enforces the RFC 9110 field-content boundary before activation: CR, LF, NUL, DEL and other invalid controls fail closed, as do leading or trailing SP/HTAB; valid interior SP/HTAB remains admissible. Its current name profile is deliberately narrower than the full legal HTTP field-name grammar because only observed migration contracts are admitted. The bounded pg-erd adapter may consume only the already characterized policy. Neither path absorbs product authorization/business response semantics, Wardnet/EgressWeave verdicts, or Keyverse identity.

## Logging and data minimization

The production path emits coarse status/outcome/request-body-byte access logs and label-free Prometheus request/error/body-byte counters. It does not log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, configuration credentials, arbitrary headers, route values, trust-bundle contents, or other unbounded request-derived labels. Distributed tracing and richer bounded operability evidence remain release gaps.

## Supply chain

Pingora and `pingora-prometheus` are pinned to one exact upstream commit and must be revalidated against current releases/advisories immediately before release. `Cargo.lock` is committed. Repository CI tests and lints with `--locked`, rejects lockfile mutation, and the OCI builder copies the reviewed lock and builds with `--locked`; dependency-resolution drift therefore fails closed rather than silently rewriting release inputs.

`CWL_GATEWAY_BIN` is a build-time process selector, not arbitrary executable authority: the Dockerfile allows exactly `cwl-pingora-gateway` or `cwl-pingora-pg-erd-migration` and copies the selected executable to one fixed distroless runtime path. Exact-head OCI acceptance must build both profiles and start each as uid/gid `65532` with a read-only root filesystem, all capabilities dropped and `no-new-privileges`. The supply-chain lane must build and scan both candidate images and bind both local image IDs and per-image scan receipts to the exact source SHA.

A committed lock and unreleased candidate receipts are necessary but not sufficient release evidence. The exact resolved graph still requires current vulnerability/license policy evidence, and a protected release requires SBOM and provenance bound to the published artifact digest, reproducibility evidence, signing/attestation policy, and an immutable registry image digest before consumer cutover.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.
