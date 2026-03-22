# Milestone 2 Release Notes

## Status
- Published on 2026-03-21.
- Go-live checklist (`24.2.x`) is green in `docs/TODO.md`.

## Scope Summary
Milestone 2 delivers the production package/runtime path across Phases 20-24:
- strict package trust and deterministic preflight/link diagnostics (`C101`-`C119`),
- precompiled `std::core` artifact pipeline and import-pruning controls,
- runtime package loader/linker with fail-closed trust gates (`R012`-`R017`),
- production host profile policy (`contract_static`, `shared_app`) and conformance suite,
- cross-phase regression gate ensuring no Phase 19 strict/profile regression.

## Key Delivered Gates
- Runnable namespace baseline in CI:
  - `clg run clearlang-tests/16_namespaced_call.clear`.
- Std-core artifact reproducibility and drift checks in CI.
- Resolver/solver determinism replay gates in CI.
- Runtime loader tamper/replay determinism gates in CI.
- Milestone-2 cross-phase regression job:
  - Phase 19 design principles + profile regression,
  - strict acceptance checks,
  - resolver diagnostics determinism,
  - runtime replay checks,
  - host conformance certification.

## Release Evidence
- `24.2.8` is covered by `docs/evidence/milestone_2-readiness.md`.
- `24.2.9` is covered by `docs/evidence/milestone_2-performance.md`.
- `24.2.10` is covered by `docs/evidence/milestone_2-supply-chain.md`.

## Publish Criteria Status
1. `24.2.x` checklist items are checked in `docs/TODO.md`.
2. Milestone-2 CI regression and release-train gates are configured in `.github/workflows/ci.yml`.
3. Security/performance/supply-chain evidence indexes are linked in this release note.

## Post-Review Hardening (Option 1)
- Supply-chain license checks include workspace crates (no exemption).
- Supply-chain compliance runs after std-core artifact generation to include generated artifacts.
- Runtime host-profile capability validation rejects unsupported/duplicate/empty capabilities with deterministic runtime diagnostics.
- Runtime loader host-capability validation now also enforces runtime-linked provider package imports (not only the root module) under production profiles.
- Host-capability mapping for strict build and runtime loader paths is centralized to one canonical policy module to prevent capability allowlist drift.
- Performance SLO gate includes runtime-link startup latency in addition to startup and package-resolution build metrics.
