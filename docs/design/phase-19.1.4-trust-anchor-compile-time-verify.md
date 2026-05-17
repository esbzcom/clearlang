# Phase 19.1.4 - Compile-Time Trust-Anchor Verify Integration

## Status
Design lock for `19.1.4` in `docs/TODO.md`.
This adds external trust-anchor checks for compile-time verification while preserving runtime verification behavior.

## Goal
Require explicit, pinned Lean/Coq checker versions for compile-time trust evaluation without adding theorem-prover kernels to runtime bundles.

## Design Principles Check
- Simple for users: compile-time verify mode has explicit flags and deterministic failures (`C032`, `V004`) when trust-anchor inputs are missing or inconsistent.
- AI-friendly: trust policy schema and signature payload fields are machine-readable, stable, and exact-match compared.
- Provably correct: assurance claims that rely on compile-time checker trust are bound to pinned checker versions and verified against policy.
- Crypto-focused: runtime verification remains signature/hash-focused and kernel-free; stronger trust checks stay in CI/compile-time workflows.

## Scope
1. Extend `clg build --sign` to optionally include pinned checker versions in signature payload:
   - `--lean-checker-version <VERSION>`
   - `--coq-checker-version <VERSION>`
2. Enforce deterministic build validation:
   - both checker-version flags must be provided together,
   - checker-version flags require `--sign`,
   - violations use build diagnostic `C032`.
3. Add explicit verify modes:
   - `--verify-mode runtime` (default, existing behavior),
   - `--verify-mode compile-time` (requires `--trust-policy <FILE>`).
4. Add trust policy verification in compile-time mode with verify diagnostic `V004` for:
   - missing policy for compile-time mode,
   - trust-policy read/parse/schema errors,
   - missing/invalid signature payload trust-anchor data,
   - checker-version mismatch against policy.

## Locked Behavior
1. Runtime verify mode remains unchanged and kernel-free.
2. Compile-time verify mode performs existing signature/hash checks first, then trust-anchor policy checks.
3. Trust policy schema is:
   - `schema_version = 1`,
   - `trust_anchors.lean_checker`,
   - `trust_anchors.coq_checker`.
4. Version matching is exact string equality.

## Non-Goals
1. No theorem-prover execution integration in runtime verify flow.
2. No dynamic policy fetch or network trust resolution in this slice.
3. No expansion of assurance tier semantics beyond trust-anchor pin checks.

## Exit Criteria for 19.1.4
1. Build/sign flow supports pinned checker versions with deterministic validation.
2. Verify flow supports compile-time mode + trust policy checks.
3. Regression tests cover:
   - compile-time mode requiring trust policy,
   - policy mismatch failure,
   - successful compile-time trust match.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/diagnostics.md`
- `docs/proofs/proof-section.md`
- `docs/design/phase-18.0-open-questions.md`
