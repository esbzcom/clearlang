#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TMP_DIR="${ROOT_DIR}/tmp/perf"
CLG_BIN="${ROOT_DIR}/target/release/clg"
WASM_TOOLS_BIN="${WASM_TOOLS_BIN:-}"
PERF_GATE_MODE="${1:-run}"

STARTUP_LATENCY_MAX_S="${STARTUP_LATENCY_MAX_S:-2.0}"
PKG_RESOLUTION_LATENCY_MAX_S="${PKG_RESOLUTION_LATENCY_MAX_S:-2.5}"
RUNTIME_LINK_STARTUP_LATENCY_MAX_S="${RUNTIME_LINK_STARTUP_LATENCY_MAX_S:-3.0}"
MAX_RSS_KB="${MAX_RSS_KB:-400000}"
MAX_CPU_PERCENT="${MAX_CPU_PERCENT:-400}"
PERF_SAMPLE_RUNS="${PERF_SAMPLE_RUNS:-3}"

mkdir -p "${TMP_DIR}"

if [[ "${PERF_GATE_MODE}" != "run" && "${PERF_GATE_MODE}" != "--portability-smoke" && "${PERF_GATE_MODE}" != "--self-test" ]]; then
  echo "usage: $0 [--portability-smoke|--self-test]" >&2
  exit 1
fi
if ! [[ "${PERF_SAMPLE_RUNS}" =~ ^[1-9][0-9]*$ ]]; then
  echo "PERF_SAMPLE_RUNS must be a positive integer" >&2
  exit 1
fi

to_tool_path() {
  local exe="$1"
  local path="$2"
  if [[ "${exe}" == *.exe ]] && command -v cygpath >/dev/null 2>&1; then
    cygpath -w "${path}"
    return
  fi
  printf "%s\n" "${path}"
}

run_tool() {
  local exe="$1"
  shift
  local args=()
  local arg
  for arg in "$@"; do
    case "${arg}" in
      /*|./*|../*)
        args+=("$(to_tool_path "${exe}" "${arg}")")
        ;;
      *)
        args+=("${arg}")
        ;;
    esac
  done
  "${exe}" "${args[@]}"
}

run_clg() {
  run_tool "${CLG_BIN}" "$@"
}

run_wasm_tools() {
  run_tool "${WASM_TOOLS_BIN}" "$@"
}

run_python() {
  if command -v python >/dev/null 2>&1; then
    python "$@"
    return
  fi
  if command -v python3 >/dev/null 2>&1; then
    python3 "$@"
    return
  fi
  if command -v py >/dev/null 2>&1; then
    py "$@"
    return
  fi
  echo "missing python interpreter (python/python3/py)" >&2
  exit 1
}

portability_smoke() {
  local sample_unix="/tmp/clearlang/perf/sample.wasm"
  local converted_exe
  converted_exe="$(to_tool_path "tool.exe" "${sample_unix}")"
  if command -v cygpath >/dev/null 2>&1; then
    local expected_exe
    expected_exe="$(cygpath -w "${sample_unix}")"
    if [[ "${converted_exe}" != "${expected_exe}" ]]; then
      echo "portability smoke failed: .exe path conversion mismatch" >&2
      exit 1
    fi
  elif [[ "${converted_exe}" != "${sample_unix}" ]]; then
    echo "portability smoke failed: non-cygpath .exe conversion should be passthrough" >&2
    exit 1
  fi

  local converted_native
  converted_native="$(to_tool_path "tool" "${sample_unix}")"
  if [[ "${converted_native}" != "${sample_unix}" ]]; then
    echo "portability smoke failed: native tool path should be passthrough" >&2
    exit 1
  fi

  local passthrough
  passthrough="$(run_tool printf "%s" "${sample_unix}")"
  if [[ "${passthrough}" != "${sample_unix}" ]]; then
    echo "portability smoke failed: run_tool path passthrough mismatch" >&2
    exit 1
  fi

  echo "milestone_2 perf portability smoke passed"
}

write_performance_artifact() {
  local out_file="$1"
  local startup_s="$2"
  local startup_rss="$3"
  local startup_cpu="$4"
  local pkg_s="$5"
  local pkg_rss="$6"
  local pkg_cpu="$7"
  local runtime_s="$8"
  local runtime_rss="$9"
  local runtime_cpu="${10}"
  cat > "${out_file}" <<EOF
{
  "schema_version": 1,
  "sample_runs": ${PERF_SAMPLE_RUNS},
  "aggregation_mode": "median",
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
}

validate_performance_artifact_schema() {
  local file="$1"
  run_python - "$file" <<'PY'
import json
import sys
from pathlib import Path

value = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert value["schema_version"] == 1
assert isinstance(value["sample_runs"], int) and value["sample_runs"] >= 1
assert value["aggregation_mode"] == "median"
for threshold in (
    "startup_latency_s_max",
    "pkg_resolution_latency_s_max",
    "runtime_link_startup_latency_s_max",
    "max_rss_kb",
    "max_cpu_percent",
):
    assert threshold in value["thresholds"], threshold
for metric in ("startup", "pkg_resolution", "runtime_link_startup"):
    entry = value["measurements"][metric]
    assert "latency_s" in entry, metric
    assert "rss_kb" in entry, metric
    assert "cpu_percent" in entry, metric
PY
}

self_test() {
  local self_dir="${TMP_DIR}/self-test"
  mkdir -p "${self_dir}"

  assert_le_float "self_float_pass" "1.0" "1.0"
  if (assert_le_float "self_float_fail" "2.0" "1.0" >/dev/null 2>&1); then
    echo "perf self-test failed: assert_le_float should fail when value > max" >&2
    exit 1
  fi

  assert_le_int "self_int_pass" "1" "2"
  if (assert_le_int "self_int_fail" "3" "2" >/dev/null 2>&1); then
    echo "perf self-test failed: assert_le_int should fail when value > max" >&2
    exit 1
  fi

  local artifact_file="${self_dir}/milestone2-performance.json"
  write_performance_artifact "${artifact_file}" \
    "0.10" "1000" "10" \
    "0.20" "1100" "20" \
    "0.30" "1200" "30"
  validate_performance_artifact_schema "${artifact_file}"

  cat > "${self_dir}/sample.1.time" <<EOF
1.00 100 10%
EOF
  cat > "${self_dir}/sample.2.time" <<EOF
3.00 300 30%
EOF
  cat > "${self_dir}/sample.3.time" <<EOF
2.00 200 20%
EOF
  aggregate_measurements "${self_dir}/sample.median.time" \
    "${self_dir}/sample.1.time" \
    "${self_dir}/sample.2.time" \
    "${self_dir}/sample.3.time"
  local median_line
  median_line="$(cat "${self_dir}/sample.median.time")"
  if [[ "${median_line}" != "2.000000 200 20.000000" ]]; then
    echo "perf self-test failed: median aggregation mismatch: ${median_line}" >&2
    exit 1
  fi

  echo "milestone_2 perf self-test passed"
}

measure() {
  local time_file="$1"
  shift
  /usr/bin/time -f "%e %M %P" -o "${time_file}" "$@" >/dev/null
}

aggregate_measurements() {
  local out_file="$1"
  shift
  run_python - "$out_file" "$@" <<'PY'
import statistics
import sys
from pathlib import Path

out_path = Path(sys.argv[1])
sample_paths = [Path(p) for p in sys.argv[2:]]
if not sample_paths:
    raise SystemExit("no sample measurements provided")

latencies = []
rss_values = []
cpu_values = []
for sample in sample_paths:
    raw = sample.read_text(encoding="utf-8").strip().split()
    if len(raw) != 3:
        raise SystemExit(f"invalid time sample format: {sample}")
    latencies.append(float(raw[0]))
    rss_values.append(int(raw[1]))
    cpu_values.append(float(raw[2].rstrip("%")))

out_path.write_text(
    f"{statistics.median(latencies):.6f} "
    f"{int(statistics.median(rss_values))} "
    f"{statistics.median(cpu_values):.6f}\n",
    encoding="utf-8",
)
PY
}

measure_samples() {
  local prefix="$1"
  shift
  local sample_files=()
  local i
  for ((i = 1; i <= PERF_SAMPLE_RUNS; i++)); do
    local sample_file="${TMP_DIR}/${prefix}.${i}.time"
    measure "${sample_file}" "$@"
    sample_files+=("${sample_file}")
  done
  aggregate_measurements "${TMP_DIR}/${prefix}.time" "${sample_files[@]}"
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

sha256_file() {
  local file="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${file}" | awk '{print $1}'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "${file}" | awk '{print $1}'
    return
  fi
  openssl dgst -sha256 "${file}" | awk '{print $2}'
}

canonical_json_sha256() {
  local json_file="$1"
  run_python - "$json_file" <<'PY'
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
  run_wasm_tools parse "${fixture_dir}/provider.wat" -o "${fixture_dir}/store/pkg-a.wasm"

  cat > "${fixture_dir}/app.wat" <<'WAT'
(module
  (import "pkg::a" "add" (func $add (param i32 i32) (result i32)))
  (func (export "main") (result i32)
    i32.const 40
    i32.const 2
    call $add)
)
WAT
  run_wasm_tools parse "${fixture_dir}/app.wat" -o "${fixture_dir}/app.wasm"

  local digest
  digest="sha256:$(sha256_file "${fixture_dir}/store/pkg-a.wasm")"

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

if [[ "${PERF_GATE_MODE}" == "--portability-smoke" ]]; then
  portability_smoke
  exit 0
fi
if [[ "${PERF_GATE_MODE}" == "--self-test" ]]; then
  self_test
  exit 0
fi

if [[ ! -x "${CLG_BIN}" && -x "${CLG_BIN}.exe" ]]; then
  CLG_BIN="${CLG_BIN}.exe"
fi
if [[ ! -x "${CLG_BIN}" ]]; then
  echo "missing release binary at ${CLG_BIN}" >&2
  exit 1
fi
if [[ -z "${WASM_TOOLS_BIN}" ]]; then
  if command -v wasm-tools >/dev/null 2>&1; then
    WASM_TOOLS_BIN="$(command -v wasm-tools)"
  elif command -v wasm-tools.exe >/dev/null 2>&1; then
    WASM_TOOLS_BIN="$(command -v wasm-tools.exe)"
  fi
fi
if [[ -z "${WASM_TOOLS_BIN}" ]]; then
  echo "missing wasm-tools in PATH" >&2
  exit 1
fi
if ! command -v openssl >/dev/null 2>&1; then
  echo "missing openssl in PATH" >&2
  exit 1
fi

measure_samples \
  "startup" \
  "${CLG_BIN}" run "${ROOT_DIR}/clearlang-tests/16_namespaced_call.clear"

measure_samples \
  "pkg_resolution" \
  "${CLG_BIN}" build "${ROOT_DIR}/clearlang-tests/perf/pkg_resolution/main.clear" \
  -o "${TMP_DIR}/pkg_resolution.wasm"

setup_runtime_loader_fixture
measure_samples \
  "runtime_link" \
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

write_performance_artifact "${TMP_DIR}/milestone2-performance.json" \
  "${startup_s}" "${startup_rss}" "${startup_cpu}" \
  "${pkg_s}" "${pkg_rss}" "${pkg_cpu}" \
  "${runtime_s}" "${runtime_rss}" "${runtime_cpu}"

echo "milestone_2 performance gate passed"
echo "artifact: ${TMP_DIR}/milestone2-performance.json"
