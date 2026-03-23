# Milestone 2 Performance/SLO Gate

This document tracks TODO item `24.2.9`.

## Gate Definition
CI runs `cargo run -p xtask -- milestone2-perf-gate` after release build and enforces
the following default budgets:
- startup overhead (`clg run clearlang-tests/16_namespaced_call.clear`) <= `2.0s`
- package resolution/link build latency (`clg build` on `clearlang-tests/perf/pkg_resolution/main.clear`) <= `2.5s`
- runtime-link startup latency (successful `clg run` with runtime-link/trust/signature inputs) <= `3.0s`
- resident memory budget (RSS) for each measured command <= `400000 KB`
- CPU utilization budget for each measured command <= `400%`
- Each command is measured across `3` runs (`PERF_SAMPLE_RUNS`) and the median
  latency/RSS/CPU sample is compared against thresholds.

## Measurement Artifact
- CI uploads `milestone2-performance` artifact from `tmp/perf/*`.
- Canonical machine-readable output:
  - `tmp/perf/milestone2-performance.json`

## Notes
- Thresholds can be overridden via CI env vars:
  - `STARTUP_LATENCY_MAX_S`
  - `PKG_RESOLUTION_LATENCY_MAX_S`
  - `RUNTIME_LINK_STARTUP_LATENCY_MAX_S`
  - `MAX_RSS_KB`
  - `MAX_CPU_PERCENT`
  - `PERF_SAMPLE_RUNS`
- Release train is blocked if this gate fails.
