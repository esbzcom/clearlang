# Phase 25.0.14 - Docs Assurance Profile Alignment

## Status
Design lock + implementation note for `25.0.14` in `docs/TODO.md`.

## Goal
Ensure README and example release docs do not imply that standard/permissive compile outputs are theorem-grade proved artifacts.

## Policy
1. Documentation must explicitly separate:
   - dev/evidence compiler profiles (`permissive`, `standard`)
   - release-grade profile and verify gates (`strict` + `release-profile production` + `require-assurance proved_all`)
2. Release examples must use production release flags and theorem-grade verify checks.
3. Claims about production safety/proofs must always reference the release policy gate path, not standard-mode defaults.

## Enforcement Wiring
- Updated docs:
  - `README.md`
  - `examples/projects/README.md`
- Gate test coverage:
  - `crates/cli/tests/milestone3_release_gate.rs`

## Exit Criteria for 25.0.14
1. Root README explicitly states standard/permissive modes are not theorem-grade by default.
2. Example quick-release flow includes production profile and `--require-assurance proved_all`.
3. Milestone release gate test asserts these doc claims remain present.

## References
- `docs/TODO.md`
- `docs/release-process.md`
- `README.md`
- `examples/projects/README.md`
