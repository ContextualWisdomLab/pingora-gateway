# Incomplete upstream response-header lifetime trace

**Status:** Open supplier prerequisite  
**Gateway owner finding:** `ContextualWisdomLab/pingora-gateway#40`  
**Supplier owner path:** `cloudflare/pingora#992`  
**Pinned supplier source:** `cloudflare/pingora@09696b51bc59315353d96686355861604d0bb48c`

## Problem and exact evidence

Pg-erd Admin Config v2 bounds continuously progressing **response bodies** after the first non-informational upstream response header. It does not bound the time spent receiving an **incomplete response header**.

At the pinned Pingora source, HTTP/1 `HttpSession::read_response()` retains partial bytes in `response_header_read_buf`. When parsing remains partial, each `underlying_stream.read_buf(...)` is wrapped independently in `read_timeout`; a successful read returns to the parse loop and the next read receives a fresh timeout. Pingora's public peer documentation describes the same contract: `read_timeout` applies to each individual read, resets on progress, and is not a total response deadline.

`ProxyHttp::upstream_response_filter` cannot close this gap because it is called only after a complete upstream response header arrives. `upstream_response_body_filter` is later still. `PeerOptions::total_connection_timeout` is explicitly connection-establishment time, including TLS, and is therefore not a response-header lifetime.

A backend that sends an incomplete HTTP response header one byte or fragment at a time with every fragment arriving before `read_ms` can consequently retain an admitted request without completing the header and without triggering the existing per-read inactivity timeout or PR #39's body-progress lifetime.

## Alternatives

- **Lower `read_ms`: rejected.** This changes inactivity tolerance and still resets after each successful read; it does not create an overall header deadline.
- **Reuse `total_connection_timeout`: rejected.** The supplier defines this over connection establishment, not response receipt.
- **Start a gateway wall-clock task outside Pingora: rejected for current source.** Without an exposed cancellation-safe handle to the pending upstream header read, a side timer cannot prove that the supplier read task is terminated or the pooled connection state remains coherent.
- **Vendor/copy Pingora code locally: rejected as the routine path.** Edge-runtime truth belongs at the supplier boundary; an emergency backport/fork would require a separate immutable, provenance-bound supply-chain decision compatible with `.github#1605`.
- **Supplier capability: selected.** An additive overall upstream response-header deadline, or an equivalent cancellation-safe hook, must span repeated successful reads and remain distinct from `read_timeout`.

## Required semantics before gateway integration

The supplier capability must document HTTP/1 and HTTP/2 behavior, define how informational 1xx response blocks consume or reset the budget, terminate a pending header read at the overall deadline without corrupting parser/pool state, and preserve current behavior when unset. After an immutable supplier revision is available, the gateway can add a versioned positive pg-erd Admin Config field and real-listener RED→GREEN traffic for incomplete-header slow-drip, readiness, admission recovery, low-cardinality failure telemetry, and independent-route recovery.

Until then, this remains a release/cutover gap. PR #39's response-body progress guard must not be described as a whole-response deadline.

## References

Cloudflare. (n.d.). *Pingora HTTP/1 client session* [Source code, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/pingora-core/src/protocols/http/v1/client.rs

Cloudflare. (n.d.). *Peer: how to connect to upstream* [Documentation, commit 09696b51bc59315353d96686355861604d0bb48c]. GitHub. https://github.com/cloudflare/pingora/blob/09696b51bc59315353d96686355861604d0bb48c/docs/user_guide/peer.md

Bae, S. (2026, September 3). *Add an overall upstream response-header deadline distinct from read_timeout* [GitHub issue #992]. Cloudflare Pingora. https://github.com/cloudflare/pingora/issues/992
