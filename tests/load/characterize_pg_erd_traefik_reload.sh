#!/usr/bin/env bash
set -euo pipefail

consumer_dir="${1:?usage: characterize_pg_erd_traefik_reload.sh <pg-erd-cloud checkout>}"
: "${PG_ERD_SOURCE_SHA:?PG_ERD_SOURCE_SHA must be set}"
: "${PG_ERD_TRAEFIK_RELOAD_EVIDENCE:?PG_ERD_TRAEFIK_RELOAD_EVIDENCE must be set}"
: "${PG_ERD_TRAEFIK_LOG:?PG_ERD_TRAEFIK_LOG must be set}"

expected_source_sha="8dc746920c12988f082e914879d95e13c9693535"
expected_traefik_image="traefik:v3.5.4@sha256:4df0a50fcf71b454c0d7ad17675776dc8d37359deae3291895bdaa008c1b9972"
if [[ "$PG_ERD_SOURCE_SHA" != "$expected_source_sha" ]]; then
  echo "unexpected pg-erd-cloud source identity: $PG_ERD_SOURCE_SHA" >&2
  exit 1
fi

cd "$consumer_dir"
test "$(git rev-parse HEAD)" = "$PG_ERD_SOURCE_SHA"

compose_file="compose.prod.yaml"
dynamic_file="deploy/traefik/dynamic.yaml"
test -f "$compose_file"
test -f "$dynamic_file"
grep -F -- "image: $expected_traefik_image" "$compose_file" >/dev/null
grep -F -- "--providers.file.filename=/etc/traefik/dynamic.yaml" "$compose_file" >/dev/null
grep -F -- "--providers.file.watch=true" "$compose_file" >/dev/null
grep -F -- "./deploy/traefik/dynamic.yaml:/etc/traefik/dynamic.yaml:ro" "$compose_file" >/dev/null
git diff --exit-code -- compose.prod.yaml deploy/traefik/dynamic.yaml

dynamic_mode="$(stat -c '%a' "$dynamic_file")"
baseline="$(mktemp "$RUNNER_TEMP/pg-erd-dynamic-baseline.XXXXXX.yaml")"
cp "$dynamic_file" "$baseline"
probe_log="$RUNNER_TEMP/pg-erd-traefik-concurrent-probes.txt"
probe_stop="$RUNNER_TEMP/pg-erd-traefik-probe.stop"
rm -f "$probe_stop"
: >"$probe_log"
: >"$PG_ERD_TRAEFIK_RELOAD_EVIDENCE"
: >"$PG_ERD_TRAEFIK_LOG"

compose_project="cwl_pg_erd_reload_${GITHUB_RUN_ID:-manual}_${GITHUB_RUN_ATTEMPT:-1}"
export COMPOSE_PROJECT_NAME="$compose_project"
read -r edge_port postgres_port < <(python3 - <<'PY'
import socket
ports = []
for _ in range(2):
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        ports.append(sock.getsockname()[1])
print(*ports)
PY
)

cp .env.example .env
python3 - "$edge_port" "$postgres_port" <<'PY'
from pathlib import Path
import sys
path = Path(".env")
lines = []
for line in path.read_text().splitlines():
    if line.startswith("APP_SECRET="):
        continue
    if line.startswith("POSTGRES_PASSWORD="):
        line = "POSTGRES_PASSWORD=cwl-characterization-only-password"
    elif line.startswith("POSTGRES_PORT="):
        line = f"POSTGRES_PORT={sys.argv[2]}"
    elif line.startswith("TRAEFIK_HTTP_PORT="):
        line = f"TRAEFIK_HTTP_PORT={sys.argv[1]}"
    lines.append(line)
path.write_text("\n".join(lines) + "\n")
PY
mkdir -p secrets
printf '%s\n' 'cwl-characterization-only-app-secret-00000000000000000000' >secrets/app_secret
chmod 600 secrets/app_secret

base_url="http://127.0.0.1:${edge_port}"
probe_pid=""

record() {
  local key="$1"
  local value="$2"
  printf '%s=%s\n' "$key" "$value" >>"$PG_ERD_TRAEFIK_RELOAD_EVIDENCE"
}

capture_traefik_log() {
  local phase="${1:-snapshot}"
  {
    printf '=== traefik-log-snapshot phase=%s timestamp_ms=%s ===\n' "$phase" "$(date +%s%3N)"
    container_id="$(docker compose -f "$compose_file" ps -q traefik 2>/dev/null || true)"
    printf 'container_id=%s\n' "${container_id:-none}"
    docker compose -f "$compose_file" logs --no-color traefik 2>&1 || true
  } >>"$PG_ERD_TRAEFIK_LOG"
}

cleanup() {
  local status=$?
  if [[ -n "$probe_pid" ]]; then
    touch "$probe_stop"
    wait "$probe_pid" >/dev/null 2>&1 || true
  fi
  if [[ -f "$baseline" && -f "$dynamic_file" ]]; then
    cat "$baseline" >"$dynamic_file" || true
    chmod "$dynamic_mode" "$dynamic_file" || true
  fi
  capture_traefik_log cleanup
  docker compose -f "$compose_file" down -v --remove-orphans >/dev/null 2>&1 || true
  rm -f .env
  rm -rf secrets
  trap - EXIT
  exit "$status"
}
trap cleanup EXIT

observe_health() {
  local output code generation
  output="$(
    curl --silent --show-error --max-time 2 \
      --dump-header - \
      --output /dev/null \
      --write-out 'CWL_STATUS:%{http_code}\n' \
      "$base_url/healthz" 2>/dev/null || true
  )"
  code="$(printf '%s\n' "$output" | awk -F: '$1 == "CWL_STATUS" {print $2}' | tail -n 1)"
  generation="$(
    printf '%s\n' "$output" \
      | tr -d '\r' \
      | awk -F': *' 'tolower($1) == "x-cwl-reload-generation" {print $2; exit}'
  )"
  [[ -n "$code" ]] || code="000"
  [[ -n "$generation" ]] || generation="none"
  printf '%s %s\n' "$code" "$generation"
}

wait_for_status_200() {
  local deadline=$((SECONDS + 120)) code generation
  while (( SECONDS < deadline )); do
    read -r code generation < <(observe_health)
    if [[ "$code" == "200" ]]; then
      return 0
    fi
    sleep 1
  done
  return 1
}

wait_for_generation() {
  local expected="$1"
  local seconds="${2:-15}"
  local deadline=$((SECONDS + seconds)) code generation
  while (( SECONDS < deadline )); do
    read -r code generation < <(observe_health)
    if [[ "$code" == "200" && "$generation" == "$expected" ]]; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

wait_for_generation_absent() {
  local seconds="${1:-15}"
  local deadline=$((SECONDS + seconds)) code generation
  while (( SECONDS < deadline )); do
    read -r code generation < <(observe_health)
    if [[ "$code" == "200" && "$generation" == "none" ]]; then
      return 0
    fi
    sleep 0.2
  done
  return 1
}

render_generation() {
  local generation="$1"
  local output="$2"
  python3 - "$baseline" "$output" "$generation" <<'PY'
from pathlib import Path
import sys
source = Path(sys.argv[1]).read_text()
needle = "        permissionsPolicy: geolocation=(), microphone=(), camera=()\n"
if source.count(needle) != 1:
    raise SystemExit("consumer dynamic.yaml security-header anchor changed")
addition = (
    needle
    + "        customResponseHeaders:\n"
    + f"          X-CWL-Reload-Generation: {sys.argv[3]}\n"
)
Path(sys.argv[2]).write_text(source.replace(needle, addition, 1))
PY
}

write_in_place() {
  local source="$1"
  cat "$source" >"$dynamic_file"
}

force_recreate_traefik() {
  capture_traefik_log pre-recreate
  docker compose -f "$compose_file" up -d --no-deps --force-recreate traefik
  capture_traefik_log post-recreate
}

activate_generation_with_fallback() {
  local candidate="$1"
  local generation="$2"
  local receipt_key="$3"
  write_in_place "$candidate"
  if wait_for_generation "$generation" 15; then
    record "$receipt_key" false
    return 0
  fi
  record "$receipt_key" true
  force_recreate_traefik
  wait_for_generation "$generation" 30
}

start_concurrent_probe() {
  (
    set +e
    while [[ ! -e "$probe_stop" ]]; do
      now_ms="$(date +%s%3N)"
      read -r code generation < <(observe_health)
      printf 'timestamp_ms=%s status=%s generation=%s\n' "$now_ms" "$code" "$generation" >>"$probe_log"
      sleep 0.05
    done
  ) &
  probe_pid=$!
}

record consumer_source_sha "$PG_ERD_SOURCE_SHA"
record compose_sha256 "$(sha256sum "$compose_file" | awk '{print $1}')"
record dynamic_config_sha256 "$(sha256sum "$baseline" | awk '{print $1}')"
record traefik_image "$expected_traefik_image"
record bind_mount_shape 'single-file-read-only'
record provider_mode 'filename+watch'

docker compose -f "$compose_file" up -d --build
wait_for_status_200
record baseline_ready true
start_concurrent_probe

in_place_candidate="$(mktemp "$RUNNER_TEMP/pg-erd-in-place.XXXXXX.yaml")"
render_generation "in-place" "$in_place_candidate"
in_place_started_ms="$(date +%s%3N)"
write_in_place "$in_place_candidate"
if wait_for_generation "in-place" 15; then
  in_place_reload_detected=true
  in_place_detected_ms="$(date +%s%3N)"
  record in_place_reload_latency_ms "$((in_place_detected_ms - in_place_started_ms))"
else
  in_place_reload_detected=false
  record in_place_reload_latency_ms unavailable
fi
record in_place_reload_detected "$in_place_reload_detected"

lkg_candidate="$(mktemp "$RUNNER_TEMP/pg-erd-lkg.XXXXXX.yaml")"
render_generation "last-known-good" "$lkg_candidate"
activate_generation_with_fallback "$lkg_candidate" "last-known-good" "lkg_generation_requires_recreate"
lkg_generation_requires_recreate="$(awk -F= '$1 == "lkg_generation_requires_recreate" {print $2}' "$PG_ERD_TRAEFIK_RELOAD_EVIDENCE" | tail -n 1)"
if [[ "$lkg_generation_requires_recreate" == false ]]; then
  live_reload_observation_path=true
  printf 'http:\n  routers: [\n' >"$dynamic_file"
  sleep 2
  read -r malformed_status malformed_generation < <(observe_health)
  if [[ "$malformed_status" == "200" && "$malformed_generation" == "last-known-good" ]]; then
    malformed_last_known_good=true
  else
    malformed_last_known_good=false
  fi
else
  live_reload_observation_path=false
  malformed_status=not-observable
  malformed_generation=not-observable
  malformed_last_known_good=not-observable
fi
record live_reload_observation_path "$live_reload_observation_path"
record malformed_status "$malformed_status"
record malformed_generation "$malformed_generation"
record malformed_last_known_good "$malformed_last_known_good"

recovery_candidate="$(mktemp "$RUNNER_TEMP/pg-erd-recovery.XXXXXX.yaml")"
render_generation "recovery" "$recovery_candidate"
activate_generation_with_fallback "$recovery_candidate" "recovery" "recovery_requires_recreate"
record recovery_detected true
recovery_requires_recreate="$(awk -F= '$1 == "recovery_requires_recreate" {print $2}' "$PG_ERD_TRAEFIK_RELOAD_EVIDENCE" | tail -n 1)"

if [[ "$live_reload_observation_path" == true && "$recovery_requires_recreate" == false ]]; then
  semantic_invalid="$(mktemp "$RUNNER_TEMP/pg-erd-semantic-invalid.XXXXXX.yaml")"
  python3 - "$recovery_candidate" "$semantic_invalid" <<'PY'
from pathlib import Path
import sys
text = Path(sys.argv[1]).read_text()
needle = "      service: backend\n"
if text.count(needle) < 1:
    raise SystemExit("healthz service anchor changed")
Path(sys.argv[2]).write_text(text.replace(needle, "      service: missing-cwl-characterization-service\n", 1))
PY
  write_in_place "$semantic_invalid"
  sleep 2
  read -r semantic_invalid_status semantic_invalid_generation < <(observe_health)
  record semantic_invalid_observable true
else
  semantic_invalid_status=not-observable
  semantic_invalid_generation=not-observable
  record semantic_invalid_observable false
fi
record semantic_invalid_status "$semantic_invalid_status"
record semantic_invalid_generation "$semantic_invalid_generation"
activate_generation_with_fallback "$recovery_candidate" "recovery" "post_invalid_recovery_requires_recreate"

atomic_candidate="$(mktemp "$(dirname "$dynamic_file")/.cwl-dynamic-replacement.XXXXXX")"
render_generation "atomic-replace" "$atomic_candidate"
chmod "$dynamic_mode" "$atomic_candidate"
replacement="$atomic_candidate"
atomic_started_ms="$(date +%s%3N)"
mv "$replacement" "$dynamic_file"
if wait_for_generation "atomic-replace" 5; then
  atomic_replace_detected=true
  atomic_detected_ms="$(date +%s%3N)"
  record atomic_replace_latency_ms "$((atomic_detected_ms - atomic_started_ms))"
else
  atomic_replace_detected=false
  record atomic_replace_latency_ms unavailable
fi
record atomic_replace_detected "$atomic_replace_detected"

# Recreate only the Traefik container against the replaced host path. This is a
# characterization control proving whether controlled recreation consumes the new inode.
force_recreate_traefik
if wait_for_generation "atomic-replace" 30; then
  atomic_replace_after_recreate_detected=true
else
  atomic_replace_after_recreate_detected=false
fi
record atomic_replace_after_recreate_detected "$atomic_replace_after_recreate_detected"
[[ "$atomic_replace_after_recreate_detected" == true ]]

write_in_place "$baseline"
chmod "$dynamic_mode" "$dynamic_file"
if wait_for_generation_absent 15; then
  final_baseline_requires_recreate=false
else
  final_baseline_requires_recreate=true
  force_recreate_traefik
  wait_for_generation_absent 30
fi
record final_baseline_requires_recreate "$final_baseline_requires_recreate"
record final_baseline_recovered true

touch "$probe_stop"
wait "$probe_pid" >/dev/null 2>&1 || true
probe_pid=""
concurrent_probe_samples="$(wc -l <"$probe_log" | tr -d ' ')"
concurrent_probe_failures="$(awk '$0 !~ / status=200 / {count++} END {print count+0}' "$probe_log")"
record concurrent_probe_samples "$concurrent_probe_samples"
record concurrent_probe_failures "$concurrent_probe_failures"
record concurrent_probe_log_sha256 "$(sha256sum "$probe_log" | awk '{print $1}')"
[[ "$concurrent_probe_samples" -ge 20 ]]

capture_traefik_log final
record traefik_log_sha256 "$(sha256sum "$PG_ERD_TRAEFIK_LOG" | awk '{print $1}')"
record result characterization-complete

git diff --exit-code -- compose.prod.yaml deploy/traefik/dynamic.yaml
