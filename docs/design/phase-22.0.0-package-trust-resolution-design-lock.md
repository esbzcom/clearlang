# Phase 22.0.0 - Package Trust and Dependency Resolution Design Lock

## Status
Design lock for `22.0.0` in `docs/TODO.md`.

## Goal
Lock the Phase 22 production design before implementation:
- canonical package metadata v1,
- canonical lockfile v1 workflow,
- deterministic transitive resolver + semver solver policy,
- signed advisory/vulnerability response policy,
- reserved diagnostic code plan for Phase 22 execution.

## Design Principles Check
- Simple for users: one canonical package model and lockfile flow.
- AI-friendly: deterministic ordering, stable artifacts, stable diagnostics.
- Provably correct: explicit trust boundaries and fail-closed policies.
- Crypto-focused: no ambient trust, signed artifacts and signed advisories.

## Locked Design Document Set
1. Metadata migration:
   - `docs/design/phase-22.0.1-canonical-package-metadata-migration.md`
2. Metadata v1:
   - `docs/design/phase-22.0.2-package-metadata-v1.md`
3. Lockfile v1:
   - `docs/design/phase-22.0.3-lockfile-v1.md`
4. Resolver + semver determinism:
   - `docs/design/phase-22.1.0-resolver-semver-determinism.md`
5. Vulnerability/advisory policy:
   - `docs/design/phase-22.1.3-vulnerability-response-policy.md`
6. Diagnostics reservation:
   - `docs/design/phase-22.0.6-phase22-diagnostics-reservation.md`

## Scope Boundary
- Phase 22 covers compile-time trust and deterministic dependency resolution.
- Phase 23 covers runtime package loader/linker behavior.
- Phase 24 covers host-profile finalization, go-live, and governance gates.

## Acceptance Criteria
1. All six design docs above are published and cross-referenced from TODO.
2. TODO marks `22.0.0` complete and execution focus moves to `22.0.1`.
3. No Phase 22 implementation starts without these locked policies.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/design/phase-20.0-std-packaging-runtime-linking.md`
- `docs/design/phase-21.0-std-core-package-surface.md`
