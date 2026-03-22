#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="${ROOT_DIR}/tmp/perf"
CLG_BIN="${ROOT_DIR}/target/release/clg"

STARTUP_LATENCY_MAX_S="${STARTUP_LATENCY_MAX_S:-2.0}"
PKG_RESOLUTION_LATENCY_MAX_S="${PKG_RESOLUTION_LATENCY_MAX_S:-2.5}"
RUNTIME_LINK_STARTUP_LATENCY_MAX_S="${RUNTIME_LINK_STARTUP_LATENCY_MAX_S:-3.0}"
MAX_RSS_KB="${MAX_RSS_KB:-400000}"
MAX_CPU_PERCENT="${MAX_CPU_PERCENT:-400}"

mkdir -p "${TMP_DIR}"

if [[ ! -x "${CLG_BIN}" && -x "${CLG_BIN}.exe" ]]; then
  CLG_BIN="${CLG_BIN}.exe"
fi
if [[ ! -x "${CLG_BIN}" ]]; then
  echo "missing release binary at ${CLG_BIN}" >&2
  exit 1
fi
if ! command -v wasm-tools >/dev/null 2>&1; then
  echo "missing wasm-tools in PATH" >&2
  exit 1
fi
if ! command -v openssl >/dev/null 2>&1; then
  echo "missing openssl in PATH" >&2
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

canonical_json_sha256() {
  local json_file="$1"
  python - "$json_file" <<'PY'
import hashlib
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
value = json.loads(path.read_text(encoding="utf-8"))
canonical = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
print(hashlib.sha256(canonical).hexdigest())
PY
}

setup_runtime_loader_fixture() {
  local fixture_dir="${TMP_DIR}/runtime_loader"
  rm -rf "${fixture_dir}"
  mkdir -p "${fixture_dir}/store"

  cat > "${fixture_dir}/provider.wat" <<'WAT'
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
)
WAT
  wasm-tools parse "${fixture_dir}/provider.wat" -o "${fixture_dir}/store/pkg-a.wasm"

  cat > "${fixture_dir}/app.wat" <<'WAT'
(module
  (import "pkg::a" "add" (func $add (param i32 i32) (result i32)))
  (func (export "main") (result i32)
    i32.const 40
    i32.const 2
    call $add)
)
WAT
  wasm-tools parse "${fixture_dir}/app.wat" -o "${fixture_dir}/app.wasm"

  local digest
  digest="sha256:$(sha256sum "${fixture_dir}/store/pkg-a.wasm" | awk '{print $1}')"

  cat > "${fixture_dir}/clg.runtime-link.json" <<EOF
{
  "schema_version": 0,
  "resolver_version": 1,
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "digest": "${digest}",
      "artifact_path": "store/pkg-a.wasm",
      "abi_id": "abi:pkg::a:1.0.0"
    }
  ],
  "bindings": [
    {
      "import_module": "pkg::a",
      "import_name": "add",
      "provider_package_id": "pkg::a@1.0.0",
      "provider_symbol": "add"
    }
  ]
}
EOF
  canonical_json_sha256 "${fixture_dir}/clg.runtime-link.json" > "${fixture_dir}/clg.runtime-link.sha256"

  cat > "${fixture_dir}/clg.package-store-index.json" <<EOF
{
  "schema_version": 0,
  "artifacts": [
    {
      "id": "pkg::a@1.0.0",
      "digest": "${digest}",
      "path": "store/pkg-a.wasm"
    }
  ]
}
EOF

  cat > "${fixture_dir}/clg.lock.json" <<EOF
{
  "schema_version": 1,
  "resolver_version": 1,
  "roots": [],
  "packages": [
    {
      "id": "pkg::a@1.0.0",
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "${digest}",
      "abi_id": "abi:pkg::a:1.0.0",
      "dependencies": []
    }
  ]
}
EOF

  openssl genpkey -algorithm ED25519 -out "${fixture_dir}/signing.key" >/dev/null 2>&1
  local pub_der_hex
  pub_der_hex="$(openssl pkey -in "${fixture_dir}/signing.key" -pubout -outform DER | xxd -p -c 1000 | tr -d '\n')"
  local pub_hex="${pub_der_hex: -64}"
  local signed_at="2026-06-01T00:00:00Z"

  printf "clg-package-signature-v0\npkg::a\n1.0.0\n%s\n%s\n" "${digest}" "${signed_at}" > "${fixture_dir}/payload.txt"
  openssl pkeyutl -sign -inkey "${fixture_dir}/signing.key" -rawin -in "${fixture_dir}/payload.txt" -out "${fixture_dir}/sig.bin" >/dev/null 2>&1
  local sig_hex
  sig_hex="$(xxd -p -c 1000 "${fixture_dir}/sig.bin" | tr -d '\n')"

  cat > "${fixture_dir}/clg.trust-policy.json" <<EOF
{
  "schema_version": 0,
  "trusted_signers": [
    {
      "key_id": "k1",
      "scheme": "ed25519",
      "public_key": "hex:${pub_hex}",
      "not_before": "2026-01-01T00:00:00Z",
      "not_after": "2027-01-01T00:00:00Z"
    }
  ],
  "revoked_key_ids": []
}
EOF

  cat > "${fixture_dir}/clg.package-signatures.json" <<EOF
{
  "schema_version": 0,
  "signatures": [
    {
      "name": "pkg::a",
      "version": "1.0.0",
      "digest": "${digest}",
      "key_id": "k1",
      "signed_at": "${signed_at}",
      "signature_format": "ed25519",
      "signature": "${sig_hex}"
    }
  ]
}
EOF
}

measure \
  "${TMP_DIR}/startup.time" \
  "${CLG_BIN}" run "${ROOT_DIR}/clearlang-tests/16_namespaced_call.clear"

measure \
  "${TMP_DIR}/pkg_resolution.time" \
  "${CLG_BIN}" build "${ROOT_DIR}/clearlang-tests/perf/pkg_resolution/main.clear" \
  -o "${TMP_DIR}/pkg_resolution.wasm"

setup_runtime_loader_fixture
measure \
  "${TMP_DIR}/runtime_link.time" \
  "${CLG_BIN}" run "${TMP_DIR}/runtime_loader/app.wasm"

read -r startup_s startup_rss startup_cpu_pct < "${TMP_DIR}/startup.time"
read -r pkg_s pkg_rss pkg_cpu_pct < "${TMP_DIR}/pkg_resolution.time"
read -r runtime_s runtime_rss runtime_cpu_pct < "${TMP_DIR}/runtime_link.time"

startup_cpu="${startup_cpu_pct%\%}"
pkg_cpu="${pkg_cpu_pct%\%}"
runtime_cpu="${runtime_cpu_pct%\%}"

assert_le_float "startup_latency_s" "${startup_s}" "${STARTUP_LATENCY_MAX_S}"
assert_le_float "pkg_resolution_latency_s" "${pkg_s}" "${PKG_RESOLUTION_LATENCY_MAX_S}"
assert_le_float "runtime_link_startup_latency_s" "${runtime_s}" "${RUNTIME_LINK_STARTUP_LATENCY_MAX_S}"
assert_le_int "startup_rss_kb" "${startup_rss}" "${MAX_RSS_KB}"
assert_le_int "pkg_resolution_rss_kb" "${pkg_rss}" "${MAX_RSS_KB}"
assert_le_int "runtime_link_rss_kb" "${runtime_rss}" "${MAX_RSS_KB}"
assert_le_float "startup_cpu_percent" "${startup_cpu}" "${MAX_CPU_PERCENT}"
assert_le_float "pkg_resolution_cpu_percent" "${pkg_cpu}" "${MAX_CPU_PERCENT}"
assert_le_float "runtime_link_cpu_percent" "${runtime_cpu}" "${MAX_CPU_PERCENT}"

cat > "${TMP_DIR}/milestone2-performance.json" <<EOF
{
  "schema_version": 1,
  "thresholds": {
    "startup_latency_s_max": ${STARTUP_LATENCY_MAX_S},
    "pkg_resolution_latency_s_max": ${PKG_RESOLUTION_LATENCY_MAX_S},
    "runtime_link_startup_latency_s_max": ${RUNTIME_LINK_STARTUP_LATENCY_MAX_S},
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
    },
    "runtime_link_startup": {
      "latency_s": ${runtime_s},
      "rss_kb": ${runtime_rss},
      "cpu_percent": ${runtime_cpu}
    }
  }
}
EOF

echo "milestone_2 performance gate passed"
echo "artifact: ${TMP_DIR}/milestone2-performance.json"
