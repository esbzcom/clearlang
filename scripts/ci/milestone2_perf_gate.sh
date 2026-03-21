#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="${ROOT_DIR}/tmp/perf"
CLG_BIN="${ROOT_DIR}/target/release/clg"

STARTUP_LATENCY_MAX_S="${STARTUP_LATENCY_MAX_S:-2.0}"
PKG_RESOLUTION_LATENCY_MAX_S="${PKG_RESOLUTION_LATENCY_MAX_S:-2.5}"
MAX_RSS_KB="${MAX_RSS_KB:-400000}"
MAX_CPU_PERCENT="${MAX_CPU_PERCENT:-400}"

mkdir -p "${TMP_DIR}"

if [[ ! -x "${CLG_BIN}" ]]; then
  echo "missing release binary at ${CLG_BIN}" >&2
  exit 1
fi

measure() {
  local time_file="$1"
  shift
  /usr/bin/time -f "%e %M %P" -o "${time_file}" "$@" >/dev/null
}

assert_le_float() {
  local name="$1"
  local value="$2"
  local max="$3"
  awk -v v="${value}" -v m="${max}" 'BEGIN { exit !(v <= m) }' || {
    echo "${name} budget exceeded: value=${value}, max=${max}" >&2
    exit 1
  }
}

assert_le_int() {
  local name="$1"
  local value="$2"
  local max="$3"
  if (( value > max )); then
    echo "${name} budget exceeded: value=${value}, max=${max}" >&2
    exit 1
  fi
}

measure \
  "${TMP_DIR}/startup.time" \
  "${CLG_BIN}" run "${ROOT_DIR}/clearlang-tests/16_namespaced_call.clear"

measure \
  "${TMP_DIR}/pkg_resolution.time" \
  "${CLG_BIN}" build "${ROOT_DIR}/clearlang-tests/perf/pkg_resolution/main.clear" \
  -o "${TMP_DIR}/pkg_resolution.wasm"

read -r startup_s startup_rss startup_cpu_pct < "${TMP_DIR}/startup.time"
read -r pkg_s pkg_rss pkg_cpu_pct < "${TMP_DIR}/pkg_resolution.time"

startup_cpu="${startup_cpu_pct%\%}"
pkg_cpu="${pkg_cpu_pct%\%}"

assert_le_float "startup_latency_s" "${startup_s}" "${STARTUP_LATENCY_MAX_S}"
assert_le_float "pkg_resolution_latency_s" "${pkg_s}" "${PKG_RESOLUTION_LATENCY_MAX_S}"
assert_le_int "startup_rss_kb" "${startup_rss}" "${MAX_RSS_KB}"
assert_le_int "pkg_resolution_rss_kb" "${pkg_rss}" "${MAX_RSS_KB}"
assert_le_float "startup_cpu_percent" "${startup_cpu}" "${MAX_CPU_PERCENT}"
assert_le_float "pkg_resolution_cpu_percent" "${pkg_cpu}" "${MAX_CPU_PERCENT}"

cat > "${TMP_DIR}/milestone2-performance.json" <<EOF
{
  "schema_version": 1,
  "thresholds": {
    "startup_latency_s_max": ${STARTUP_LATENCY_MAX_S},
    "pkg_resolution_latency_s_max": ${PKG_RESOLUTION_LATENCY_MAX_S},
    "max_rss_kb": ${MAX_RSS_KB},
    "max_cpu_percent": ${MAX_CPU_PERCENT}
  },
  "measurements": {
    "startup": {
      "latency_s": ${startup_s},
      "rss_kb": ${startup_rss},
      "cpu_percent": ${startup_cpu}
    },
    "pkg_resolution": {
      "latency_s": ${pkg_s},
      "rss_kb": ${pkg_rss},
      "cpu_percent": ${pkg_cpu}
    }
  }
}
EOF

echo "milestone_2 performance gate passed"
echo "artifact: ${TMP_DIR}/milestone2-performance.json"
