#!/usr/bin/env bash
set -euo pipefail
if grep -RInE '\bpingora(_|::|-)' src/edge_routing; then
  echo 'Edge Routing domain must not depend on Pingora transport types' >&2
  exit 1
fi
