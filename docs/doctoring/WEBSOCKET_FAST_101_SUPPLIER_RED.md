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

## Fixture integrity repairs

Current-head review first found a test-infrastructure failure path independent of the supplier race: the spawned proxy was wrapped in its cleanup guard only after `/readyz` succeeded, while the origin accept thread was started before readiness. A readiness timeout or early child exit could therefore escape the child cleanup guard and leave an origin thread blocked in `accept()`. The fixture also released its selected listener reservation before child command construction, unnecessarily widening the local reservation-to-bind race.

Ordinary repair `75a42783b26c31e52d3f5325de395e2cf392ff93` wraps the child process before readiness polling, keeps the loopback listener reservation through child command construction and releases it only immediately before spawn, and starts the origin accept thread only after `/readyz` has completed. `/readyz` is handled locally by the test proxy, so this ordering does not require an origin connection. The fast-101 delay, RFC 6455 handshake/frame oracle, supplier dependency, timeout values, and production fail-closed Upgrade policy are unchanged.

A second review found that the WebSocket frame readers bounded each blocking socket read but did not bound the complete frame. `read_exact()` can make multiple reads for the prefix, mask, and payload; with a fixed five-second socket timeout, partial progress could renew the wall-clock evidence window and allow a slow-drip frame to exceed the intended five-second fixture budget. Header evidence already uses one absolute deadline, so the frame path was weaker than the surrounding fixture contract.

Ordinary repair `d3ad47b860d35efc3a4803d327a4c01d1791a6cc` adds one absolute deadline per client or server WebSocket frame. Every partial read receives only the remaining duration, and the outer renewable socket timeout assignments are removed. Follow-up `f01e9b14df1e861f18b84615b7d4fe9c4785fd4c` preserves the retry semantics that `read_exact()` provided for interrupted system calls and treats timeout/would-block as terminal only once the absolute deadline has actually elapsed. The five-second budget, RFC 6455 masking and payload oracles, fast-101 ordering control, supplier dependency, and production fail-closed Upgrade policy are unchanged. This is evidence-boundedness repair only; it is not a gateway timeout policy.

## Ordinary-CI isolation repair

Exact `26dea63a60a1961edfe4cb7f2f0a0da096aa5b7c` completed after runner acquisition. Supply Chain `35409928080`, PgErd bounded-origin capacity `35409928052`, CI `oci-runtime 105807343621`, and CI `load-contract 105807343835` were successful. CI `test 105807343713` passed exact checkout, Rust 1.98.0 installation, and `cargo fmt --all -- --check`, then failed at the ordinary `Compile and test` step; all later lint/rustdoc/coverage/lock steps were skipped.

At that exact, `websocket_fast_101_supplier_red.rs` was an auto-discovered integration test with no required feature. Ordinary CI intentionally executes `cargo test --all-targets --locked --no-fail-fast`, so the known affected-supplier acceptance was part of the ordinary product test graph. That mixes two incompatible meanings: ordinary CI should stay GREEN when repository code is healthy, while a dedicated supplier characterization should become GREEN only when it proves the released dependency still reproduces the narrowly fingerprinted RED.

Ordinary repair `5bcdaed9359d3f207fbeedaa29e4f2e7f1b0c5ee` adds an empty-default `supplier-red` feature and requires it for the `websocket_fast_101_supplier_red` target. `76c9cfc6af55ab80dfa1514dd8e24278cbec4ea7` adds a dedicated exact-SHA characterization workflow. `79d9bd512a0f6f6165a8dac9bf5c3f87751080a1` binds this doctoring file to that workflow's path admission.

The dedicated workflow first compiles the ordinary all-target graph with the default feature set and inspects Cargo metadata to prove that the supplier target is gated by exactly `supplier-red`. It then compiles the target explicitly under that feature and executes only `released_pingora_keeps_fast_101_upgrade_tunnel_bidirectional`. A nonzero test result is credited only when the log contains the ordinary Rust test failure marker and one of the bounded post-101 tunnel-teardown fingerprints. Compile/setup/runtime or unrelated assertion failures do not count as supplier RED. If a release-qualified supplier repair makes the unchanged acceptance pass, the characterization workflow fails because the RED receipt is no longer valid.

This separation does not skip a product regression, use `continue-on-error`, or weaken the WebSocket assertion. It prevents an intentionally failing supplier oracle from poisoning ordinary CI while making the supplier defect independently executable and fail-closed.

## Primary evidence

- Cloudflare Pingora issue #946, WebSocket upgrade race.
- Cloudflare Pingora pull request #947, proposed request-body-end accounting repair and deterministic ordering test; open/unmerged as of 2026-09-18.
- RFC 6455, *The WebSocket Protocol*.
- RFC 8441, *Bootstrapping WebSockets with HTTP/2*.
- RFC 9220, *Bootstrapping WebSockets with HTTP/3*.
- RFC 9931, *Security Considerations for Optimistic Protocol Transitions in HTTP/1.1*.
