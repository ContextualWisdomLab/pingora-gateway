# Security

## Trust boundaries

Configuration files, downstream requests, forwarding headers, upstream certificates, and deployment inputs are untrusted. Operators must protect configuration write access. V1 has no secret-bearing configuration field; credentials must not be added casually to the YAML contract.

The gateway contacts only the socket address admitted at startup. It is not a forward proxy and accepts no request-controlled upstream URI, preventing this runtime from becoming a generic SSRF primitive.

HTTPS upstreams use certificate and hostname verification with an explicit SNI. Certificate issuance/rotation and downstream TLS termination are separate operational responsibilities; this repository does not acquire ACME ownership merely because legacy Nginx/Certbot deployments co-located them.

Inbound forwarding identity is deleted before proxying. V1 emits only `Forwarded: proto=http`; it deliberately does not claim a client IP. A future trusted-proxy feature must define allowed proxy CIDRs/hops and RFC 7239 semantics as a versioned contract with spoofing tests.

Request bodies and upstream connect/read/write/idle time are bounded. The Pingora HTTP parser has finite protocol/header limits, but a smaller configurable header budget remains a documented gap.

## Logging and data minimization

No implementation should log Authorization, Proxy-Authorization, Cookie, Set-Cookie, request/response bodies, access tokens, configuration credentials, or arbitrary high-cardinality header/route values. Low-cardinality operational metrics and redacted access logging are release requirements but are not implemented yet.

## Supply chain

Pingora is pinned to an exact upstream commit and must be revalidated against current releases/advisories immediately before release. The repository currently lacks a committed `Cargo.lock`; therefore dependency resolution is not yet reproducible and release is blocked. SBOM, provenance, container scanning, signing/attestation policy, and immutable image digest are also release gates.

Report security issues privately through the organization security channel rather than publishing exploit details in a public issue.
