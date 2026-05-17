# Phase 19.0.3 - Compiler Modes Lock

## Status
Design lock for `19.0.3` in `docs/TODO.md`.
This introduces explicit compiler modes so users can choose strictness without ambiguity.

## Goal
Expose a single explicit strictness selector for build workflows:
- `--compiler-mode permissive`
- `--compiler-mode standard`
- `--compiler-mode strict`

## Design Principles Check
- Simple for users: one mode flag communicates strictness intent directly.
- AI-friendly: deterministic mode-to-policy mapping with stable diagnostics (`C029`, `C030`).
- Provably correct: strict mode requires proof artifact emission and enforces strict proof checks.
- Crypto-focused: production users can opt into strict, auditable settings via a single mode.

## Scope
1. CLI surface:
   - Add `--compiler-mode` to `clg build`.
2. Policy mapping:
   - Define strictness defaults/constraints per mode.
3. Diagnostics:
   - Add explicit build diagnostics for invalid mode combinations.

## Mode Semantics (Locked)
1. `permissive`
   - Default `proof_strict = false` when not explicitly set.
2. `standard`
   - Default `proof_strict = true` when not explicitly set.
   - Preserves existing behavior.
3. `strict`
   - Requires `--emit-vcs <FILE>`.
   - Forces `proof_strict = true`.
   - Rejects explicit `--proof-strict=false`.

## Diagnostics
- `C029`: strict compiler mode requires `--emit-vcs`.
- `C030`: strict compiler mode cannot be combined with `--proof-strict=false`.

## Tests
- Unit tests for mode-resolution semantics in `crates/cli/src/commands/build.rs`.
- CLI integration tests for `C029` and `C030` in `crates/cli/tests/cli_it/diagnostics.rs`.

## Non-Goals
- No assurance-tier semantics changes (`19.1` owns those).
- No trust-anchor integration changes.
- No parse/run/verify mode flags in this slice.

## Exit Criteria for 19.0.3
1. `clg build` has explicit compiler modes with deterministic semantics.
2. Invalid strict-mode combinations fail with stable diagnostics.
3. TODO docs record completion and evidence.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/design/phase-19.0.2-design-principles-gate.md`
- `docs/design/phase-18.0-open-questions.md`
