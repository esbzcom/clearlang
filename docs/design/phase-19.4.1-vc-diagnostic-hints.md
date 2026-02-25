# Phase 19.4.1 - VC diagnostic hints for minimal repair

## Status
Design lock for `19.4.1` in `docs/TODO.md`.

## Goal
Emit deterministic repair hints that suggest the minimal contract or loop annotation to try when a VC fails.

## Design Principles Check
- Simple for users: each VC includes one direct, copyable clause suggestion (`ensure`, `require`, `invariant`, or `variant`).
- AI-friendly: hints are machine-readable and deterministic under `diagnostics.repair_hints`.
- Provably correct: hints do not claim proof success; they are candidate obligations tied to the exact VC target.
- Crypto-focused: strict profile proof workflows gain actionable repair guidance without weakening assurance boundaries.

## Scope
1. Add per-VC `diagnostics.repair_hints` in `--emit-vcs` output.
2. Cover current VC families with minimal-clause suggestions:
   - `vc:*` -> `ensure { ... }`
   - `mut_pre:*` -> `require { ... }`
   - `loop:*:invariant` -> `invariant { ... }`
   - `loop:*:variant_nonneg` -> `variant { ... }`
   - `loop:*:variant_decrease` -> `variant { ... }`
3. Mark hint applicability as `on_status = "failed"` so consumers can bind suggestions to failed VCs.

## Locked Behavior
1. Hints are deterministic functions of emitted VC metadata (`vc_id`, `post.ast`).
2. Hints are advisory and do not change VC generation, assumptions, or assurance tier semantics.
3. Hints are emitted in VC JSON only for this slice.

## Non-Goals
1. No SMT solver or counterexample integration.
2. No automatic source rewriting.
3. No proof-section schema changes for hint transport in this slice.

## Exit Criteria for 19.4.1
1. VC JSON includes machine-readable repair hints for covered VC families.
2. CLI integration tests verify deterministic hint shape/content.
3. `docs/TODO.md` advances focus to `19.4.2`.

## References
- `docs/TODO.md`
- `docs/proofs/vc-schema.md`
- `crates/cli/src/commands/build.rs`
- `crates/cli/tests/cli_it/vc_outputs.rs`
