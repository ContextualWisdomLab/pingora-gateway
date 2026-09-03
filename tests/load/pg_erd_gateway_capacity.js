import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 16,
  iterations: 1600,
  thresholds: {
    checks: ['rate==1'],
    http_req_failed: ['rate==0'],
    http_req_duration: ['p(95)<20'],
  },
};

const gatewayUrl = __ENV.PG_ERD_GATEWAY_URL || 'http://127.0.0.1:18280';

export default function () {
  const backendRoute = (__VU + __ITER) % 2 === 0;
  const path = backendRoute ? '/api/capacity-contract' : '/capacity-contract';
  const expectedBody = backendRoute ? 'backend-capacity-ok' : 'frontend-capacity-ok';
  const response = http.get(`${gatewayUrl}${path}`);

  check(response, {
    'bounded-origin pg-erd gateway returns 200': (result) => result.status === 200,
    'bounded-origin pg-erd gateway preserves characterized route body': (result) =>
      result.body === expectedBody,
  });
}

export function handleSummary(data) {
  return {
    'k6-pg-erd-capacity-summary.json': JSON.stringify(data, null, 2),
  };
}
