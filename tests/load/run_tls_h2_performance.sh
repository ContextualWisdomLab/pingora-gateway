#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"
gateway_bin="${GATEWAY_BIN:-${repo_root}/target/release/cwl-pingora-gateway}"
origin_bin="${LOAD_ORIGIN_BIN:-/tmp/load_origin}"
k6_script="${repo_root}/tests/load/gateway_tls_h2_performance.js"
workdir="$(mktemp -d)"

cleanup() {
  rm -rf "${workdir}"
}
trap cleanup EXIT

for command in openssl curl k6; do
  command -v "${command}" >/dev/null 2>&1 || {
    echo "required command is unavailable: ${command}" >&2
    exit 1
  }
done
for executable in "${gateway_bin}" "${origin_bin}"; do
  test -x "${executable}" || {
    echo "required executable is unavailable: ${executable}" >&2
    exit 1
  }
done

ca_key="${workdir}/ca.key"
ca_cert="${workdir}/ca.crt"
server_key="${workdir}/server.key"
server_csr="${workdir}/server.csr"
server_cert="${workdir}/server.crt"
server_ext="${workdir}/server.ext"

openssl req -x509 -newkey rsa:2048 -nodes \
  -keyout "${ca_key}" \
  -out "${ca_cert}" \
  -subj '/CN=CWL TLS H2 Performance Test CA' \
  -days 1 -sha256 >/dev/null 2>&1
openssl req -newkey rsa:2048 -nodes \
  -keyout "${server_key}" \
  -out "${server_csr}" \
  -subj '/CN=127.0.0.1' \
  -sha256 >/dev/null 2>&1
cat >"${server_ext}" <<'EOF'
subjectAltName=IP:127.0.0.1
basicConstraints=CA:FALSE
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EOF
openssl x509 -req \
  -in "${server_csr}" \
  -CA "${ca_cert}" \
  -CAkey "${ca_key}" \
  -CAcreateserial \
  -out "${server_cert}" \
  -days 1 -sha256 \
  -extfile "${server_ext}" >/dev/null 2>&1

wait_for_origin() {
  local port="$1"
  local pid="$2"
  for _ in $(seq 1 80); do
    if curl --fail --silent --show-error --max-time 1 \
      "http://127.0.0.1:${port}/fixture-ready" >/dev/null; then
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      echo "Rust origin exited before becoming ready on ${port}" >&2
      return 1
    fi
    sleep 0.05
  done
  echo "Rust origin never became ready on ${port}" >&2
  return 1
}

wait_for_gateway() {
  local port="$1"
  local pid="$2"
  for _ in $(seq 1 80); do
    if curl --fail --silent --show-error --max-time 1 \
      --cacert "${ca_cert}" \
      "https://127.0.0.1:${port}/livez" >/dev/null; then
      return 0
    fi
    if ! kill -0 "${pid}" 2>/dev/null; then
      echo "gateway exited before becoming live on ${port}" >&2
      return 1
    fi
    sleep 0.1
  done
  echo "gateway never became live on ${port}" >&2
  return 1
}

run_mode() (
  set -euo pipefail
  local mode="$1"
  local traffic_port="$2"
  local origin_port="$3"
  local metrics_port="$4"
  local mode_dir="${workdir}/${mode}"
  local gateway_config="${mode_dir}/gateway.yaml"
  local gateway_log="${mode_dir}/gateway.log"
  local origin_log="${mode_dir}/origin.log"
  local origin_pid=""
  local gateway_pid=""
  mkdir -p "${mode_dir}"

  stop_mode() {
    local status=$?
    trap - EXIT
    if [ -n "${gateway_pid}" ]; then
      kill "${gateway_pid}" >/dev/null 2>&1 || true
      wait "${gateway_pid}" >/dev/null 2>&1 || true
    fi
    if [ -n "${origin_pid}" ]; then
      kill "${origin_pid}" >/dev/null 2>&1 || true
      wait "${origin_pid}" >/dev/null 2>&1 || true
    fi
    if [ "${status}" -ne 0 ]; then
      echo "--- ${mode} gateway log ---"
      cat "${gateway_log}" || true
      echo "--- ${mode} origin log ---"
      cat "${origin_log}" || true
    fi
    exit "${status}"
  }
  trap stop_mode EXIT

  UPSTREAM_PORT="${origin_port}" \
    UPSTREAM_WORKERS=32 \
    UPSTREAM_CONNECTION_MODE=keep-alive \
    "${origin_bin}" >"${origin_log}" 2>&1 &
  origin_pid=$!
  wait_for_origin "${origin_port}" "${origin_pid}"

  cat >"${gateway_config}" <<EOF
version: 2
listener: 127.0.0.1:${traffic_port}
metrics_listener: 127.0.0.1:${metrics_port}
max_request_body_bytes: 1048576
max_in_flight_requests: 128
service_threads: 4
upstream_keepalive_pool_size: 32
downstream_tls:
  certificate_chain_file: ${server_cert}
  private_key_file: ${server_key}
  alpn: h2_http1
upstreams:
  - name: load-fixture
    address: 127.0.0.1:${origin_port}
    tls: false
    timeouts:
      connection_ms: 500
      total_connection_ms: 1000
      read_ms: 2000
      write_ms: 2000
      idle_ms: 5000
EOF

  "${gateway_bin}" --config "${gateway_config}" >"${gateway_log}" 2>&1 &
  gateway_pid=$!
  wait_for_gateway "${traffic_port}" "${gateway_pid}"

  # Health readiness is deliberately outside the measured application route. Each mode starts a
  # fresh gateway and origin, so neither handshake nor reuse evidence depends on a prior app warm-up.
  SSL_CERT_FILE="${ca_cert}" \
    PERF_MODE="${mode}" \
    GATEWAY_URL="https://127.0.0.1:${traffic_port}" \
    k6 run --quiet "${k6_script}"

  test -s "k6-tls-h2-${mode}-summary.json"
)

run_mode fresh 18280 18281 18282
run_mode reuse 18380 18381 18382
