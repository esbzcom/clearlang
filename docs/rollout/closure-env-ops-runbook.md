# Closure-Env No-Free Policy - Host Runbook (Phase 18.0.4)

This runbook operationalizes the product-line decision that closure environments are module-instance-lifetime allocations (no in-place reclamation).

## Status
- Baseline runbook published for `18.0.4`.
- Host teams may tune thresholds, but must keep deterministic trigger ordering and documented evidence outputs.

## Design-Principles Gate (README)
Sign-off must explicitly confirm all four principles:
- `simple for users`: operator workflow is short, deterministic, and documented.
- `AI-friendly`: knobs, diagnostics, and alert messages are machine-readable and stable.
- `provably correct`: policy does not introduce hidden safety assumptions; assumptions are explicit.
- `crypto-focused`: deterministic behavior under long-running contract workloads is preserved.
- Verification evidence links: `docs/evidence/closure-env/README.md`.

## Required Inputs
- Host profile(s): service names, runtime topology, and module lifecycle model.
- Memory baseline: steady-state RSS and peak RSS for representative workloads.
- Traffic profile: request/tx rates, burst behavior, and concurrency limits.
- SLO policy: restart budget, tolerated recycle latency, and incident paging policy.

## Default Runtime Policy

Host-owned config keys (recommended names):
- `CLG_HOST_RECYCLE_MAX_AGE_MIN=60`
- `CLG_HOST_RECYCLE_WARN_AGE_MIN=45`
- `CLG_HOST_RECYCLE_RSS_YELLOW_MIB=512`
- `CLG_HOST_RECYCLE_RSS_ORANGE_MIB=768`
- `CLG_HOST_RECYCLE_RSS_RED_MIB=896`
- `CLG_HOST_RECYCLE_GROWTH_MIB_PER_MIN=8`
- `CLG_HOST_RECYCLE_GROWTH_WINDOW_MIN=10`
- `CLG_HOST_RECYCLE_TIMEOUT_SEC=120`

### Recycle Triggers
- Time trigger:
  - Start drain at 45 minutes.
  - Force recycle at 60 minutes.
- Memory trigger:
  - Use tiered thresholds from the budget table below.
- Growth trigger:
  - Recycle when closure-env growth rate is `> 8 MiB/min` for `10` consecutive minutes.

### Memory Budget Tiers
| Tier | Threshold | Action |
| --- | --- | --- |
| Green | `< 512 MiB RSS` | Normal operation. |
| Yellow | `>= 512 MiB RSS for 5 min` | Warn (`SEV3`), increase sampling, prepare controlled drain. |
| Orange | `>= 768 MiB RSS for 2 min` | Drain worker, stop new admissions, recycle within 2 min (`SEV2`). |
| Red | `>= 896 MiB RSS for 1 min` | Immediate recycle, page on-call (`SEV1`), open incident. |

## Monitoring and Alerts

### Required Signals
- `clg_worker_rss_bytes` (gauge)
- `clg_worker_heap_bytes` (gauge)
- `clg_worker_module_instances` (gauge)
- `clg_worker_recycle_total` (counter)
- `clg_worker_recycle_seconds` (histogram)
- `clg_worker_recycle_failures_total` (counter)
- `clg_closure_dispatch_trap_total{code="R011"}` (counter)
- `clg_request_latency_ms` (histogram, p95/p99)
- `clg_request_error_rate` (gauge)

### Required Alerts
- `closure_env_memory_yellow`:
  - trigger: RSS `>= 512 MiB` for 5 minutes
  - severity: `SEV3`
- `closure_env_memory_orange`:
  - trigger: RSS `>= 768 MiB` for 2 minutes
  - severity: `SEV2`
- `closure_env_memory_red`:
  - trigger: RSS `>= 896 MiB` for 1 minute
  - severity: `SEV1`
- `closure_env_growth_hot`:
  - trigger: growth `> 8 MiB/min` for 10 minutes
  - severity: `SEV2`
- `closure_env_recycle_timeout`:
  - trigger: recycle duration `> 120 sec`
  - severity: `SEV1`
- `closure_env_recycle_churn`:
  - trigger: more than 3 recycles in 30 minutes per worker pool
  - severity: `SEV2`
- `closure_env_post_recycle_capacity_drop`:
  - trigger: p95 latency `> 2x` baseline for 10 minutes or error rate `> 2%`
  - severity: `SEV2`

## Operational Procedure
1. Observe budget tier and trigger source.
2. If Yellow, start controlled drain, pin trace sampling to high, and raise operator warning.
3. If Orange, stop admitting new work, recycle target worker within 2 minutes.
4. If Red, recycle immediately and page on-call (`SEV1`) before traffic reopen.
5. Verify replacement worker readiness (health checks + warm path pass) before reopening traffic.
6. Record timestamps, trigger values, recycle duration, and outcome.

## Failure Handling
- Open incident when recycle fails, exceeds timeout, or causes sustained SLO breach.
- Escalate to platform/runtime owner if two consecutive recycle attempts fail.
- Freeze high-risk rollout changes until remediation and postmortem are complete.
- If `R011` trap rate spikes during recycle windows, block rollout and perform dispatcher/code-id integrity check before resuming.

## Evidence Artifacts (Required for 18.0.4 Closure)
- `docs/evidence/closure-env/host-policy.md`
- `docs/evidence/closure-env/alerts.yaml`
- `docs/evidence/closure-env/drill-YYYY-MM-DD.md`
- `docs/evidence/closure-env/workload-validation-YYYY-MM-DD.md`
- `docs/evidence/closure-env/design-principles-signoff-YYYY-MM-DD.md`

## Completion Criteria
- `18.0.4.1` through `18.0.4.5` in `docs/TODO.md` are checked.
- Required evidence artifact paths exist and are linked from `docs/evidence/closure-env/README.md`.
