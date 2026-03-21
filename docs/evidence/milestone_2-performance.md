# Milestone 2 Performance/SLO Gate

This document tracks TODO item `24.2.9`.

## Gate Definition
CI runs `scripts/ci/milestone2_perf_gate.sh` after release build and enforces
the following default budgets:
- startup overhead (`clg run clearlang-tests/16_namespaced_call.clear`) <= `2.0s`
- package resolution/link build latency (`clg build` on `clearlang-tests/perf/pkg_resolution/main.clear`) <= `2.5s`
- resident memory budget (RSS) for each measured command <= `400000 KB`
- CPU utilization budget for each measured command <= `400%`

## Measurement Artifact
- CI uploads `milestone2-performance` artifact from `tmp/perf/*`.
- Canonical machine-readable output:
  - `tmp/perf/milestone2-performance.json`

## Notes
- Thresholds can be overridden via CI env vars:
  - `STARTUP_LATENCY_MAX_S`
  - `PKG_RESOLUTION_LATENCY_MAX_S`
  - `MAX_RSS_KB`
  - `MAX_CPU_PERCENT`
- Release train is blocked if this gate fails.
