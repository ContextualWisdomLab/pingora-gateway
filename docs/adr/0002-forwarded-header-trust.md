# ADR 0002: Distrust Downstream Forwarding Identity

**Status:** Proposed

## Context

`Forwarded` and the `X-Forwarded-*` namespace are security-sensitive because trusting arbitrary downstream values can spoof client IP, host, port, scheme, proxy provenance, path-prefix reconstruction, or client-certificate identity. Generic v1 has no trusted-proxy CIDR/hop contract and does not terminate downstream mTLS, so every inbound `X-Forwarded-*` value is untrusted caller input rather than gateway-owned forwarding evidence.

`X-Forwarded-Prefix` is a concrete example of why the trust boundary must apply to the namespace rather than a closed list. Reverse proxies use it to communicate an externally visible path prefix, and downstream frameworks may consume it while constructing external request URLs. Allowing an untrusted client to preserve that field would therefore let caller input cross the edge trust boundary even though the better-known `X-Forwarded-For` and `X-Forwarded-Proto` names were sanitized.

RFC 9110 `Via` has a different role. It records the protocol and recipient chain across intermediaries. RFC 9110 classifies a reverse proxy as a gateway: an HTTP-to-HTTP gateway must add an appropriate `Via` entry to each inbound request and may add one to forwarded responses. It is therefore preserved as protocol trace while remaining outside the trusted identity model.

RFC 9110 §7.6.2 separately defines `Max-Forwards` for TRACE and OPTIONS. An intermediary receiving either method with the field must inspect it before forwarding: zero terminates forwarding at that intermediary, while a positive value is replaced by the lesser of the received value minus one and the intermediary's supported maximum. Generic v1 previously forwarded this proxy-control field unchanged, defeating the caller's hop bound.

## Decision

Delete inbound `Forwarded`, every field whose name begins case-insensitively with `X-Forwarded-`, and `X-Real-IP` before sending upstream. Because the v1 listener is cleartext, emit only gateway-owned `Forwarded: proto=http`. Do not assert a client IP, externally visible path prefix, client-certificate identity, or downstream proxy provenance. Headers outside that forwarding namespace remain ordinary application metadata and are not removed by this policy solely because they begin with `X-`.

For forwarded requests, preserve the received `Via` chain and append one pseudonymous `cwl-pingora-gateway` entry using the HTTP protocol version actually received from downstream rather than Pingora's normalized upstream HTTP/1.1 version. For forwarded responses, deliberately preserve any upstream `Via` chain and append the same pseudonymous gateway recipient using the HTTP version actually received from the upstream response. The request entry satisfies the HTTP-to-HTTP gateway requirement; the response entry is an RFC-permitted implementation choice that keeps intermediary trace visible in both directions. Locally generated health responses are not forwarded messages and do not receive this intermediary trace. No `Via` entry is accepted as authentication, authorization, client identity, or trusted-proxy evidence.

For TRACE and OPTIONS only, accept exactly one decimal `Max-Forwards` field. Malformed or duplicate values fail closed with HTTP 400. A positive value is decremented and capped at this gateway's supported maximum of 255 before the request is forwarded. A zero value is not forwarded. Generic v1 does not implement local TRACE echo or resource-specific OPTIONS semantics, so the exhausted request receives a local HTTP 501 with an empty body. This is a deliberate final-recipient policy: it satisfies the no-forward boundary without reflecting potentially sensitive TRACE fields. `Max-Forwards` on other methods is left untouched because RFC 9110 permits recipients to ignore it there.

`X-Forwarded-Client-Cert` is treated as an identity-bearing proxy header rather than generic application metadata. Envoy documents XFCC as carrying client/proxy certificate identity and sanitizes it by default unless an explicit forwarding policy is configured. Generic v1 has no such trust policy, so preserving a caller-supplied XFCC value would cross the identity boundary owned outside this gateway.

## Consequences

Upstreams cannot obtain end-client IP, external path-prefix, or client-certificate identity from generic v1 and must not infer identity or provenance from legacy forwarding headers. The request path retains the RFC 9110-required gateway trace; the forwarded-response path deliberately retains the RFC-permitted response trace for end-to-end intermediary visibility. Neither is converted into trust evidence. TRACE/OPTIONS hop bounds now remain meaningful across this intermediary instead of being silently bypassed, while zero-hop diagnostics stop locally without request reflection. A future trusted-proxy or downstream-mTLS feature requires an explicit trust-source contract, certificate-verification ownership, RFC 7239 semantics where applicable, field-by-field justification for any admitted `X-Forwarded-*` compatibility value, spoofing/chain tests, and consumer justification before this ADR can advance beyond Proposed.
