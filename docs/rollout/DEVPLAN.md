# ClearLang Development Plan

## Status (2026-03-22)
- `milestone_2` delivery (Phases 20-24) is complete.
- Go-live checklist (`24.2.x`) is complete in `docs/TODO.md`.
- Delivery governance (`24.3.x`) is complete in `docs/rollout/milestone_2-governance.md`.
- Release notes and evidence indexes are published:
  - `release_notes/milestone_2.md`
  - `docs/evidence/milestone_2-readiness.md`
  - `docs/evidence/milestone_2-performance.md`
  - `docs/evidence/milestone_2-supply-chain.md`

## Active Focus
No new implementation phase is currently active in this plan.

Until the next milestone is defined, focus is maintenance-only:
1. Keep CI/replay gates green (`cargo xtask ci`).
2. Keep TODO/evidence/governance docs in sync with delivered state.
3. Treat release-train checks as fail-closed for milestone tags.

## Planning Rules
1. `docs/TODO.md` is the canonical execution checklist.
2. `docs/rollout/milestone_2-governance.md` is the canonical governance lock for milestone_2.
3. `release_notes/milestone_2.md` is the canonical release summary for milestone_2.
4. Historical implementation chronology is archived in `docs/rollout/codex-session-history.md`.

## Key References
- Roadmap checklist: `docs/TODO.md`
- Milestone 2 governance: `docs/rollout/milestone_2-governance.md`
- Milestone 2 release notes: `release_notes/milestone_2.md`
- Runtime rollout runbook: `docs/runtime/runtime-loader-rollout-gate.md`
- Host-profile policy: `docs/runtime/host-profiles-production-policy.md`
