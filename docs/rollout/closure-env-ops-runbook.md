# Closure-Env No-Free Policy - Host Runbook (Phase 18.0.4)

This runbook operationalizes the product-line decision that closure environments are module-instance-lifetime allocations (no in-place reclamation).

## Status
- Draft execution template for `18.0.4`.
- Do not mark `18.0.4` complete until all `TODO` sub-items and evidence artifacts are filled.

## Design-Principles Gate (README)
Sign-off must explicitly confirm all four principles:
- `simple for users`: operator workflow is short, deterministic, and documented.
- `AI-friendly`: knobs, diagnostics, and alert messages are machine-readable and stable.
- `provably correct`: policy does not introduce hidden safety assumptions; assumptions are explicit.
- `crypto-focused`: deterministic behavior under long-running contract workloads is preserved.

## Required Inputs
- Host profile(s): service names, runtime topology, and module lifecycle model.
- Memory baseline: steady-state RSS and peak RSS for representative workloads.
- Traffic profile: request/tx rates, burst behavior, and concurrency limits.
- SLO policy: restart budget, tolerated recycle latency, and incident paging policy.

## Default Runtime Policy (Fill Per Host)

### Recycle Triggers
- Time trigger: recycle worker after `N` minutes/hours.
- Memory trigger: recycle worker when RSS exceeds `X` MiB.
- Growth trigger: recycle worker when closure-env growth rate exceeds `Y` MiB/min for `Z` minutes.

### Memory Budget Tiers
| Tier | Threshold | Action |
| --- | --- | --- |
| Green | `< fill-me` | Normal operation. |
| Yellow | `>= fill-me` | Warn, increase sampling, prepare drain. |
| Orange | `>= fill-me` | Drain worker, reject new work, recycle. |
| Red | `>= fill-me` | Immediate recycle + incident page. |

## Monitoring and Alerts

### Required Signals
- Worker RSS / heap usage.
- Module-instance count per worker.
- Recycle count and recycle duration.
- Recycle failure count.
- Request/tx latency and error rate around recycle windows.

### Required Alerts
- Memory threshold breach (by tier).
- Recycle failure or timeout.
- Recycle loop/churn (too many recycles in short window).
- Capacity degradation after recycle (latency/error SLO breach).

## Operational Procedure
1. Observe budget tier and trigger source.
2. If Yellow, start controlled drain and raise operator warning.
3. If Orange/Red, stop admitting new work to target worker and recycle.
4. Verify replacement worker readiness before reopening traffic.
5. Record timestamps, trigger values, and outcome.

## Failure Handling
- Open incident when recycle fails, exceeds timeout, or causes sustained SLO breach.
- Escalate to platform/runtime owner if two consecutive recycle attempts fail.
- Freeze high-risk rollout changes until remediation and postmortem are complete.

## Evidence Artifacts (Required for 18.0.4 Closure)
- Host-specific policy doc with filled thresholds and trigger values.
- Alert configuration export (rules, severities, destinations).
- One drill report showing controlled recycle under synthetic pressure.
- One real-workload validation report showing bounded memory growth.
- Design-principles sign-off checklist with reviewer names/date.

## Completion Criteria
- `18.0.4.1` through `18.0.4.5` in `docs/TODO.md` are checked.
- All required evidence artifacts are present and linked from this runbook.
