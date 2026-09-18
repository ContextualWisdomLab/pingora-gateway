# WebSocket fast-101 supplier RED

Status: Draft characterization evidence for #112. The durable product/technical gap and repository-wide references remain owned by #61; this file only binds the executable supplier RED to its exact inputs and expected failure.

## Scope and authority

This lane is a test-only child of #70 at `df5d2f05fc5fbdd94bbfb487283bf2a6d73a55bf`. That parent pins `pingora = 0.9.0` and `pingora-prometheus = 0.9.0`. Production gateway policy remains fail-closed for HTTP/1 Upgrade: no route, timeout, authentication, authorization, subprotocol, or consumer deployment policy is added here.

The test proxy deliberately uses `HttpUpstreamRequestPolicy::standard()` only inside `tests/websocket_fast_101_supplier_red.rs` so the released supplier's upgrade path can be exercised without weakening the production `deny_upgrades()` boundary.

## Supplier finding

Fresh supplier verification on 2026-09-18:

- protected `cloudflare/pingora/main` is `4487f7b2ab50f159e4a2cf4f6a6b813f61bb6e19`;
- `cloudflare/pingora#946` remains open and reports that an empty-request-body event processed after `101 Switching Protocols` can mark both request and response complete, terminating the upgraded tunnel;
- the proposed repair `cloudflare/pingora#947@1e8488b0627370831832744fc6e65614396c310d` remains open/unmerged and therefore is not dependency authority;
- latest public Pingora release remains 0.9.0, published 2026-09-09. No later release-qualified identity contains a maintainer disposition for #946.

The upstream reproducer controls scheduler ordering by delaying the request-body callback. The CWL RED keeps that causal mechanism but uses its own loopback origin and RFC 6455 opening handshake/frame exchange so it does not copy the supplier implementation fix.

## Executable RED

`released_pingora_keeps_fast_101_upgrade_tunnel_bidirectional` launches a test-only Pingora process and a loopback origin. The fixed RFC 6455 sample key `dGhlIHNhbXBsZSBub25jZQ==` is answered with the corresponding `Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`. The origin returns that valid `101` immediately while the proxy delays request-body handling by 200 ms, forcing the response to win the ordering race.

After the 101 is observed, the client waits 300 ms and sends a FIN text frame carrying `cwl-fast-101` with a client mask as required by RFC 6455. The origin unmasks and validates that payload, then sends an unmasked server FIN text frame containing the same bytes. The client must receive and validate the echoed frame. This proves post-handshake bidirectional tunnel survival at WebSocket frame level rather than crediting a header-only smoke test.

On an affected supplier, the delayed original request-body completion is misclassified as tunnel completion and the post-101 frame write/read fails. A future GREEN is valid only against a maintainer-integrated, release-qualified Pingora identity, with this test unchanged apart from dependency/version evidence required by that release transition.

This RED is not WebSocket enablement. It does not provide H2 Extended CONNECT, H3/RFC 9220, long-lived timeout policy, production backpressure, consumer parity, canary, rollback, or cutover evidence. Those remain #112 acceptance gates.

## Primary evidence

- Cloudflare Pingora issue #946, WebSocket upgrade race.
- Cloudflare Pingora pull request #947, proposed request-body-end accounting repair and deterministic ordering test; open/unmerged as of 2026-09-18.
- RFC 6455, *The WebSocket Protocol*.
- RFC 8441, *Bootstrapping WebSockets with HTTP/2*.
- RFC 9220, *Bootstrapping WebSockets with HTTP/3*.
- RFC 9931, *Security Considerations for Optimistic Protocol Transitions in HTTP/1.1*.
