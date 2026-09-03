#!/usr/bin/env bash
set -euo pipefail

wait_for_origin() {
  local url="$1"
  local pid="$2"
  for _ in $(seq 1 80); do
    if curl --fail --silent --show-error --max-time 1 "$url" >/dev/null; then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "bounded load origin exited before becoming reachable: $url" >&2
      return 1
    fi
    sleep 0.05
  done
  echo "bounded load origin did not become reachable: $url" >&2
  return 1
}

validate_bounded_fixture() {
  UPSTREAM_PORT=18291 \
  UPSTREAM_WORKERS=1 \
  UPSTREAM_CONNECTION_MODE=close \
  UPSTREAM_RESPONSE_DELAY_MS=150 \
    /tmp/load_origin >/tmp/bounded-origin-self-check.log 2>&1 &
  local origin_pid=$!
  local status=0
  local first_fd
  local second_fd

  cleanup_validation() {
    if [ -n "${first_fd:-}" ]; then
      eval "exec ${first_fd}>&-" || true
    fi
    if [ -n "${second_fd:-}" ]; then
      eval "exec ${second_fd}>&-" || true
    fi
    kill "$origin_pid" >/dev/null 2>&1 || true
    wait "$origin_pid" >/dev/null 2>&1 || true
  }
  trap cleanup_validation RETURN

  wait_for_origin http://127.0.0.1:18291/ready "$origin_pid"

  exec {first_fd}<>/dev/tcp/127.0.0.1/18291
  exec {second_fd}<>/dev/tcp/127.0.0.1/18291

  local started_ms
  local finished_ms
  local elapsed_ms
  started_ms=$(date +%s%3N)
  printf 'GET /a HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n' >&"$first_fd"
  printf 'GET /b HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n' >&"$second_fd"

  timeout 2s cat <&"$first_fd" >/tmp/bounded-origin-a.raw || status=$?
  timeout 2s cat <&"$second_fd" >/tmp/bounded-origin-b.raw || status=$?
  finished_ms=$(date +%s%3N)
  elapsed_ms=$((finished_ms - started_ms))

  eval "exec ${first_fd}>&-"
  first_fd=""
  eval "exec ${second_fd}>&-"
  second_fd=""

  if [ "$status" -ne 0 ]; then
    echo "bounded-origin fixture did not close both responses after the exact body: status=$status" >&2
    return "$status"
  fi

  printf 'HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 11\r\nConnection: close\r\n\r\nupstream-ok' \
    >/tmp/bounded-origin-expected.raw
  if ! cmp --silent /tmp/bounded-origin-expected.raw /tmp/bounded-origin-a.raw; then
    echo "bounded-origin first response did not match the exact close-framed upstream-ok contract" >&2
    cat -v /tmp/bounded-origin-a.raw >&2 || true
    return 1
  fi
  if ! cmp --silent /tmp/bounded-origin-expected.raw /tmp/bounded-origin-b.raw; then
    echo "bounded-origin second response did not match the exact close-framed upstream-ok contract" >&2
    cat -v /tmp/bounded-origin-b.raw >&2 || true
    return 1
  fi

  if [ "$elapsed_ms" -lt 250 ] || [ "$elapsed_ms" -gt 1500 ]; then
    echo "bounded-origin fixture did not serialize simultaneously admitted one-worker requests: elapsed_ms=$elapsed_ms" >&2
    return 1
  fi

  cleanup_validation
  trap - RETURN
}

validate_bounded_fixture

UPSTREAM_PORT=18281 \
UPSTREAM_PAYLOAD=backend-capacity-ok \
UPSTREAM_WORKERS=4 \
UPSTREAM_CONNECTION_MODE=close \
UPSTREAM_RESPONSE_DELAY_MS=1 \
  /tmp/load_origin >/tmp/pg-erd-capacity-backend.log 2>&1 &
backend_pid=$!
UPSTREAM_PORT=18283 \
UPSTREAM_PAYLOAD=frontend-capacity-ok \
UPSTREAM_WORKERS=4 \
UPSTREAM_CONNECTION_MODE=close \
UPSTREAM_RESPONSE_DELAY_MS=1 \
  /tmp/load_origin >/tmp/pg-erd-capacity-frontend.log 2>&1 &
frontend_pid=$!

gateway_pid=""
cleanup() {
  status=$?
  if [ -n "$gateway_pid" ]; then
    kill "$gateway_pid" >/dev/null 2>&1 || true
    wait "$gateway_pid" >/dev/null 2>&1 || true
  fi
  kill "$backend_pid" "$frontend_pid" >/dev/null 2>&1 || true
  wait "$backend_pid" >/dev/null 2>&1 || true
  wait "$frontend_pid" >/dev/null 2>&1 || true
  if [ "$status" -ne 0 ]; then
    echo "--- pg-erd capacity gateway log ---"
    cat /tmp/pg-erd-capacity-gateway.log || true
    echo "--- bounded backend log ---"
    cat /tmp/pg-erd-capacity-backend.log || true
    echo "--- bounded frontend log ---"
    cat /tmp/pg-erd-capacity-frontend.log || true
  fi
}
trap cleanup EXIT

wait_for_origin http://127.0.0.1:18281/ready "$backend_pid"
wait_for_origin http://127.0.0.1:18283/ready "$frontend_pid"

cat >/tmp/pg-erd-capacity.yaml <<'EOF'
version: 1
listener: 127.0.0.1:18280
metrics_listener: 127.0.0.1:18282
max_request_body_bytes: 1048576
max_in_flight_requests: 128
upstream_keepalive_pool_size: 32
upstreams:
  - name: backend
    address: 127.0.0.1:18281
    tls: false
    timeouts:
      connection_ms: 500
      total_connection_ms: 1000
      read_ms: 2000
      write_ms: 2000
      idle_ms: 5000
  - name: frontend
    address: 127.0.0.1:18283
    tls: false
    timeouts:
      connection_ms: 500
      total_connection_ms: 1000
      read_ms: 2000
      write_ms: 2000
      idle_ms: 5000
EOF

target/release/cwl-pingora-pg-erd-migration --config /tmp/pg-erd-capacity.yaml \
  >/tmp/pg-erd-capacity-gateway.log 2>&1 &
gateway_pid=$!

for _ in $(seq 1 80); do
  if curl --fail --silent --show-error --max-time 1 http://127.0.0.1:18280/livez >/dev/null; then
    break
  fi
  if ! kill -0 "$gateway_pid" 2>/dev/null; then
    echo "pg-erd gateway exited before the bounded-origin capacity contract became live" >&2
    exit 1
  fi
  sleep 0.1
done
curl --fail --silent --show-error --max-time 1 http://127.0.0.1:18280/livez >/dev/null

PG_ERD_GATEWAY_URL=http://127.0.0.1:18280 \
  k6 run --quiet tests/load/pg_erd_gateway_capacity.js
