# Phase 25.0.7 - Production Release Profile Gate (`release == proved`)

## Status
Design lock + implementation note for `25.0.7` in `docs/TODO.md`.

## Goal
Add compile-time fail-closed behavior so production compile/publish attempts are rejected unless theorem-grade policy passes.

## CLI Contract
- New build flag:
  - `--release-profile <dev|production>`
- Default:
  - `dev`
- Production contract:
  - `--release-profile production` requires `--compiler-mode strict` (`C120`).
  - Build fails unless `proof_status=proved_all` (`C121`).

## Enforcement Rules
1. Preflight gate:
   - if `release_profile=production` and `compiler_mode!=strict`, fail with `C120`.
2. Theorem-grade gate:
   - after VC generation and strict checks, compute deterministic `proof_status`.
   - if status is not `proved_all`, fail with `C121`.
3. Applies to both unsigned and signed build paths (publish/sign is blocked by the same compile gate).

## Determinism
- Gate depends only on explicit CLI inputs and deterministic VC/proof-status evaluation.
- No ambient environment/time source is consulted.

## Exit Criteria for 25.0.7
1. Build command exposes `--release-profile`.
2. Production profile is fail-closed (`C120`/`C121`) for non-theorem-grade outputs.
3. Integration tests and diagnostics docs cover both failure modes.
4. Release process docs show `--release-profile production` in production commands.

## References
- `docs/TODO.md`
- `docs/design/phase-25.0.3-release-equals-proved-policy.md`
- `crates/cli/src/main.rs`
- `crates/cli/src/commands/build/prelude.rs`
- `crates/cli/src/commands/build/run.rs`
- `crates/cli/tests/cli_it/diagnostics/type_and_mode_basics.rs`
