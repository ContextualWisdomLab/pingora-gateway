#!/usr/bin/env bash
set -euo pipefail

work="$(mktemp -d)"
gateway_pid=""
upstream_pid=""
cleanup() {
  [[ -z "$gateway_pid" ]] || kill "$gateway_pid" 2>/dev/null || true
  [[ -z "$upstream_pid" ]] || kill "$upstream_pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

printf 'production-path-fixture\n' >"$work/index.html"
python3 -m http.server 19090 --bind 127.0.0.1 --directory "$work" >"$work/upstream.log" 2>&1 &
upstream_pid=$!
cat >"$work/gateway.yaml" <<'YAML'
version: 1
listener: 127.0.0.1:18080
limits:
  max_header_count: 64
  max_header_bytes: 32768
  max_body_bytes: 16
  connect_timeout_ms: 1000
  read_timeout_ms: 5000
  write_timeout_ms: 5000
routes:
  - id: fixture
    path_prefix: /
    upstream: http://127.0.0.1:19090
    allow_private_networks: true
YAML
PINGORA_GATEWAY_CONFIG="$work/gateway.yaml" RUST_LOG=info cargo run --quiet >"$work/gateway.log" 2>&1 &
gateway_pid=$!

for _ in $(seq 1 60); do
  if curl --fail --silent http://127.0.0.1:18080/readyz >/dev/null 2>&1; then break; fi
  sleep 0.25
done
curl --fail --silent http://127.0.0.1:18080/readyz | grep -Fx 'ok'
curl --fail --silent http://127.0.0.1:18080/ | grep -F 'production-path-fixture'
curl --fail --silent http://127.0.0.1:18080/metrics | grep -F 'pingora_gateway_requests_total'
status="$(curl --silent --output /dev/null --write-out '%{http_code}' -X POST --data '0123456789abcdefg' http://127.0.0.1:18080/)"
[[ "$status" == "413" ]] || { echo "expected 413, got $status" >&2; exit 1; }
