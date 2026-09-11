# Downstream TLS/H2 performance traceability

## Decision boundary

This evidence belongs to the gateway's downstream transport and runtime boundary. It does not create product authentication, client identity, certificate lifecycle, Wardnet/EgressWeave verdict, Keyverse identity, or consumer deployment authority.

The purpose is narrower than a production SLO: prove on an exact release-built candidate that new downstream TLS/H2 connections and reused downstream H2 connections are measured separately, both carry real routed traffic, and neither can be reported by substituting cleartext traffic or a warm-only shortcut. Controlled GitHub-hosted loopback remains a regression gate; representative CPU/socket/NUMA evidence and production shadow/canary remain separate promotion gates.

## Evidence contract

`tests/load/gateway_tls_h2_performance.js` retains the repository's established generic load floor of 4 VUs and 400 iterations. It does not reduce concurrency or sample count to satisfy the threshold.

Two isolated executions use fresh gateway and origin processes and perform health readiness only; the measured application route is not pre-warmed.

- `PERF_MODE=fresh` sets k6 `noConnectionReuse=true`, requires every measured response to negotiate HTTP/2, requires a non-zero TLS handshake timing, and records `connecting + tls_handshaking + duration` as `tls_h2_fresh_buyer_path_ms`. Both fresh buyer-path p95 and TLS-handshake p95 must remain below 20 ms.
- `PERF_MODE=reuse` permits connection reuse, requires HTTP/2, records only requests for which both connection and TLS-handshake timings are zero as `tls_h2_reused_buyer_path_ms`, requires at least one observed reused request, and requires reused buyer-path p95 below 20 ms.
- Both modes require zero HTTP request failures, 100% functional checks, status 200 and the exact bounded-origin payload.

The custom fresh buyer-path metric deliberately adds connection establishment and TLS negotiation because k6 `http_req_duration` covers sending, waiting and receiving but excludes connection setup. The reused metric deliberately measures only requests that did not pay connection or TLS setup. This separation prevents a large population of reused requests from hiding handshake cost in aggregate p95.

`tests/load/run_tls_h2_performance.sh` issues an ephemeral local CA and an IP-SAN server certificate, starts version-2 generic gateway candidates with `h2_http1`, and runs k6 against `https://127.0.0.1`. Certificate verification remains enabled through the local CA; the harness does not use `insecureSkipTLSVerify`. The origin remains the bounded Rust load fixture with keep-alive enabled so the comparison changes downstream connection behavior rather than intentionally adding upstream churn.

`.github/workflows/tls-h2-performance.yml` builds the exact source SHA in release mode, compiles the bounded Rust origin with optimization, installs the same checksum-pinned k6 2.2.0 identity used by the repository load lane, validates the harness, runs both modes, and uploads both JSON summaries under an exact-SHA artifact name.

## Interpretation and non-claims

A hosted GREEN proves a reproducible loopback regression bound for the exact candidate. It does not prove Internet/WAN latency, production hardware tail latency, representative NUMA behavior, session-resumption policy, zero-RTT policy, consumer deployment parity, or cutover readiness. It also does not replace the separate protocol correctness, supplier framing, immutable-release, rollback, shadow/canary, and legacy-removal gates.

No sample is discarded as warm-up. Health checks use `/livez` and never exercise the measured application route. Fresh and reuse modes restart the gateway and origin independently, so the reuse result cannot inherit application-route state from the fresh-handshake run.

## Standards and primary documentation

HTTP/2 over TLS uses protocol negotiation and then multiplexes exchanges over the established connection. The performance contract therefore distinguishes the cost of establishing a new TLS/H2 connection from requests carried on an already-established H2 connection rather than treating both populations as one latency distribution.

TLS 1.3 handshake work is part of secure-channel establishment and is not application-response time. The current TLS 1.3 standards-track authority is RFC 9846, which obsoletes RFC 8446.

k6 exposes `connecting`, `tls_handshaking`, and `duration` independently on `Response.timings`; its documentation states that `duration` excludes initial connection setup. k6 also exposes `noConnectionReuse` as a supported option and thresholds as executable pass/fail criteria. Those primary contracts are why the harness constructs its fresh buyer-path metric explicitly rather than treating `http_req_duration` as end-to-end connection establishment latency.

### References

Grafana Labs. (n.d.). *Get timings for an HTTP metric*. k6 documentation. https://grafana.com/docs/k6/latest/examples/get-timings-for-an-http-metric/

Grafana Labs. (n.d.). *How to use options*. k6 documentation. https://grafana.com/docs/k6/latest/using-k6/k6-options/how-to/

Grafana Labs. (n.d.). *Thresholds*. k6 documentation. https://grafana.com/docs/k6/latest/using-k6/thresholds/

Rescorla, E. (2026). *The Transport Layer Security (TLS) Protocol Version 1.3* (RFC 9846). RFC Editor. https://www.rfc-editor.org/rfc/rfc9846

Thomson, M., & Benfield, C. (2022). *HTTP/2* (RFC 9113). RFC Editor. https://www.rfc-editor.org/rfc/rfc9113
