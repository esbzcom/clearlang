# Phase 19.4.3 - AI proof context bundle

## Status
Design lock for `19.4.3` in `docs/TODO.md`.

## Goal
Emit a deterministic machine-readable proof context bundle per VC containing the exact VC payload, assumptions, model snippet envelope, and span map for AI repair workflows.

## Design Principles Check
- Simple for users: one bundled object provides all proof-failure context needed for debugging and repair.
- AI-friendly: bundle fields are stable and explicit (`vc`, `assumptions`, `model_snippet`, `span_map`).
- Provably correct: bundle is trace metadata only; it does not change VC semantics or assurance-tier claims.
- Crypto-focused: proof/debug tooling receives explicit assumption and model-boundary context for cryptographic surfaces.

## Scope
1. Add `diagnostics.proof_context` in VC JSON output.
2. Bundle includes:
   - `vc`: function, vc_id, status, pre/post AST+SMT2, VC SMT2, clause kind.
   - `assumptions`: same machine-readable assumption payload shape.
   - `model_snippet`: deterministic solver-unavailable envelope with symbol bindings.
   - `span_map`: focus role and source byte-offset maps (`pre`/`post` when available).
3. Add integration/snapshot coverage for deterministic output shape.

## Locked Behavior
1. `diagnostics.proof_context.format` is `clg.proof_context.v1`.
2. Bundle is emitted for each VC in `--emit-vcs` output.
3. Model values remain `null` until external solver/model attachment is implemented.

## Non-Goals
1. No external solver execution or concrete model extraction.
2. No proof-section schema/CBOR changes in this slice.
3. No automatic source rewriting.

## Exit Criteria for 19.4.3
1. VC JSON includes `diagnostics.proof_context` with required bundle fields.
2. CLI integration + snapshot tests pass with deterministic outputs.
3. `docs/TODO.md`/`docs/TODO.md` advance focus to `19.5.1`.

## References
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `crates/cli/src/commands/build.rs`
- `crates/cli/tests/cli_it/vc_outputs.rs`
- `crates/cli/tests/vc_snapshots.rs`
