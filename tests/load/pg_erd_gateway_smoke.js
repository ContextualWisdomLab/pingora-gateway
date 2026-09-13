import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 4,
  iterations: 400,
  thresholds: {
    checks: ['rate==1'],
    http_req_failed: ['rate==0'],
    http_req_duration: ['p(95)<20'],
    'http_req_duration{route:backend}': ['p(95)<20'],
    'http_req_duration{route:frontend}': ['p(95)<20'],
    'http_reqs{route:backend}': ['count>=198'],
    'http_reqs{route:frontend}': ['count>=198'],
  },
};

const gatewayUrl = __ENV.PG_ERD_GATEWAY_URL || 'http://127.0.0.1:18180';

export default function () {
  const backendRoute = (__VU + __ITER) % 2 === 0;
  const route = backendRoute ? 'backend' : 'frontend';
  const path = backendRoute ? '/api/load-contract' : '/load-contract';
  const expectedBody = backendRoute ? 'backend-ok' : 'frontend-ok';
  const response = http.get(`${gatewayUrl}${path}`, { tags: { route } });

  check(response, {
    'pg-erd gateway returns 200': (result) => result.status === 200,
    'pg-erd gateway preserves characterized route body': (result) => result.body === expectedBody,
  });
}

export function handleSummary(data) {
  return {
    'k6-pg-erd-summary.json': JSON.stringify(data, null, 2),
  };
}
