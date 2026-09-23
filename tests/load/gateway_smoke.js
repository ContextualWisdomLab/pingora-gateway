import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 4,
  iterations: 400,
  thresholds: {
    checks: ['rate==1'],
    http_req_failed: ['rate==0'],
    http_req_duration: ['p(95)<20'],
  },
};

const gatewayUrl = __ENV.GATEWAY_URL || 'http://127.0.0.1:18080';

export default function () {
  const response = http.get(`${gatewayUrl}/load-contract`);
  check(response, {
    'gateway returns 200': (result) => result.status === 200,
    'gateway preserves upstream body': (result) => result.body === 'upstream-ok',
  });
}

export function handleSummary(data) {
  return {
    'k6-summary.json': JSON.stringify(data, null, 2),
  };
}
