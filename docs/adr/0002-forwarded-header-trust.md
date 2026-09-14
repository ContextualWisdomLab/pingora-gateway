# ADR 0002: Distrust Downstream Forwarding Identity

**Status:** Proposed

## Context

`Forwarded` and the `X-Forwarded-*` namespace are security-sensitive because trusting arbitrary downstream values can spoof client IP, host, port, scheme, proxy provenance, path-prefix reconstruction, or client-certificate identity. Generic v1 has no trusted-proxy CIDR/hop contract and does not terminate downstream mTLS, so every inbound `X-Forwarded-*` value is untrusted caller input rather than gateway-owned forwarding evidence.

`X-Forwarded-Prefix` is a concrete example of why the trust boundary must apply to the namespace rather than a closed list. Reverse proxies use it to communicate an externally visible path prefix, and downstream frameworks may consume it while constructing external request URLs. Allowing an untrusted client to preserve that field would therefore let caller input cross the edge trust boundary even though the better-known `X-Forwarded-For` and `X-Forwarded-Proto` names were sanitized.

RFC 9110 `Via` has a different role. It records the protocol and recipient chain across intermediaries, and an HTTP-to-HTTP gateway is required to add its own entry when forwarding. It is therefore preserved as protocol trace while remaining outside the trusted identity model.

## Decision

Delete inbound `Forwarded`, every field whose name begins case-insensitively with `X-Forwarded-`, and `X-Real-IP` before sending upstream. Because the v1 listener is cleartext, emit only gateway-owned `Forwarded: proto=http`. Do not assert a client IP, externally visible path prefix, client-certificate identity, or downstream proxy provenance. Headers outside that forwarding namespace remain ordinary application metadata and are not removed by this policy solely because they begin with `X-`.

Preserve the received `Via` chain and append one pseudonymous `cwl-pingora-gateway` entry using the HTTP protocol version actually received from downstream rather than Pingora's normalized upstream HTTP/1.1 version. No `Via` entry is accepted as authentication, authorization, client identity, or trusted-proxy evidence.

`X-Forwarded-Client-Cert` is treated as an identity-bearing proxy header rather than generic application metadata. Envoy documents XFCC as carrying client/proxy certificate identity and sanitizes it by default unless an explicit forwarding policy is configured. Generic v1 has no such trust policy, so preserving a caller-supplied XFCC value would cross the identity boundary owned outside this gateway.

## Consequences

Upstreams cannot obtain end-client IP, external path-prefix, or client-certificate identity from generic v1 and must not infer identity or provenance from legacy forwarding headers. They can inspect the standards-required `Via` trace for intermediary/protocol diagnostics, but not as a trust source. A future trusted-proxy or downstream-mTLS feature requires an explicit trust-source contract, certificate-verification ownership, RFC 7239 semantics where applicable, field-by-field justification for any admitted `X-Forwarded-*` compatibility value, spoofing/chain tests, and consumer justification before this ADR can advance beyond Proposed.
