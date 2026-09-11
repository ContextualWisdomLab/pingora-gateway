import http from 'k6/http';
import { check } from 'k6';
import { Counter, Trend } from 'k6/metrics';

const mode = __ENV.PERF_MODE || 'reuse';
if (mode !== 'fresh' && mode !== 'reuse') {
  throw new Error(`PERF_MODE must be fresh or reuse, received ${mode}`);
}

const freshBuyerPath = new Trend('tls_h2_fresh_buyer_path_ms', true);
const freshHandshake = new Trend('tls_h2_fresh_handshake_ms', true);
const reusedBuyerPath = new Trend('tls_h2_reused_buyer_path_ms', true);
const reusedRequests = new Counter('tls_h2_reused_requests');

export const options = {
  vus: 4,
  iterations: 400,
  noConnectionReuse: mode === 'fresh',
  thresholds: mode === 'fresh'
    ? {
        checks: ['rate==1'],
        http_req_failed: ['rate==0'],
        tls_h2_fresh_buyer_path_ms: ['p(95)<20'],
        tls_h2_fresh_handshake_ms: ['p(95)<20'],
      }
    : {
        checks: ['rate==1'],
        http_req_failed: ['rate==0'],
        tls_h2_reused_buyer_path_ms: ['p(95)<20'],
        tls_h2_reused_requests: ['count>0'],
      },
};

const gatewayUrl = __ENV.GATEWAY_URL || 'https://127.0.0.1:18280';

export default function () {
  const response = http.get(`${gatewayUrl}/tls-h2-performance`, {
    tags: { transport_mode: mode },
  });
  const timings = response.timings;

  check(response, {
    'gateway returns 200': (result) => result.status === 200,
    'gateway preserves upstream body': (result) => result.body === 'upstream-ok',
    'gateway negotiates HTTP/2': (result) => result.proto === 'HTTP/2.0',
    ...(mode === 'fresh'
      ? {
          'fresh request pays a TLS handshake': () => timings.tls_handshaking > 0,
        }
      : {}),
  });

  if (mode === 'fresh') {
    freshHandshake.add(timings.tls_handshaking);
    freshBuyerPath.add(
      timings.connecting + timings.tls_handshaking + timings.duration,
    );
    return;
  }

  if (timings.connecting === 0 && timings.tls_handshaking === 0) {
    reusedRequests.add(1);
    reusedBuyerPath.add(timings.duration);
  }
}

export function handleSummary(data) {
  return {
    [`k6-tls-h2-${mode}-summary.json`]: JSON.stringify(data, null, 2),
  };
}
