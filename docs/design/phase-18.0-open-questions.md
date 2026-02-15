# Phase 18.0 - Open Questions Decision Lock

## Status
Design lock for `18.0.0` in `docs/TODO.md`.
This document resolves Phase 18 kickoff decisions so execution gates can proceed.

## Scope
- Resolve `18.0.0.1` through `18.0.0.10`.
- Keep decisions aligned with README principles:
  - simple for users,
  - AI-friendly,
  - provably correct,
  - crypto-focused.

## Non-Goals
- No claim of universal proof-power superiority over Coq/Agda/Lean/F*.
- No immediate expansion of language surface where proof/diagnostic determinism would regress.

## Decision Summary (Locked)

### D1. Interface/implementation generics (`T246`/`T245`)
Decision: keep disallowed in this product line until a bounded, deterministic coherence design is ready.

Rationale:
- Preserves predictable resolution and diagnostics.
- Avoids opening high-complexity overlap/ambiguity states in the type system.

Follow-up:
- Track in Phase 19 proof/ergonomics upgrades before revisiting.

### D2. Generic refinement aliases (`T244`)
Decision: keep deferred for now.

Rationale:
- Current alias model is deterministic; generic substitution adds VC complexity and proof-surface risk.

Follow-up:
- Revisit only with bounded substitution rules and explicit VC growth limits.

### D3. `Map`/`Set` backend strategy
Decision: keep deterministic linear-search backend as the default semantics for current releases.

Rationale:
- Maximum determinism and easiest audit/proof story.
- No hash function soundness/perf-tuning ambiguity in the trusted path.

Follow-up:
- Hashing may be explored later as an opt-in profile with deterministic hashing rules and regression proofs.

### D4. `U128`/`U256` arithmetic modeling scope
Decision: prioritize production-needed arithmetic operators with explicit unsupported/assumed boundaries.

Rationale:
- Improves practical assurance without over-claiming full coverage prematurely.

Follow-up:
- Increase solver coverage incrementally; each new operator must declare proof status in emitted artifacts.

### D5. Bitwise/shift proof model
Decision: pursue stronger SMT modeling where feasible; require explicit assumptions for uncovered cases.

Rationale:
- Keeps assurance honest and machine-auditable.
- Avoids silent over-approximation.

Follow-up:
- Maintain feature-level `proved` vs `assumed` mapping as part of release criteria.

### D6. Crypto proof model
Decision: hybrid model is the baseline.

Rules:
- Primitive semantics proved where practical.
- Remaining gaps must be labeled assumptions.
- Outputs must clearly separate proved guarantees from assumed claims.

Rationale:
- Best trade-off for production timelines while preserving trust transparency.

### D7. Inline refinements on parameters/returns
Decision: keep alias-first surface for now (no inline refinement syntax in this phase).

Rationale:
- Simpler parser/typechecker surface.
- Keeps diagnostics and AI generation patterns stable.

Follow-up:
- Revisit only if proof ergonomics data shows alias-only is a bottleneck.

### D8. Deferred resource/type-surface limits
Decision: keep deferred/disallowed in this phase:
- `Set<Resource>`,
- resource arrays/slices,
- additional array-key equality expansions beyond current approved scope.

Rationale:
- Resource linearity soundness stays priority over feature breadth.

Follow-up:
- Revisit with explicit ownership/aliasing proofs and deterministic diagnostics.

### D9. External trust anchor for `verify` mode
Decision: Lean-first integration for compile-time trust anchoring, with pinned version policy.

Rules:
- Verification toolchain is compile-time/CI only.
- Runtime bundles remain kernel-free.
- Exact toolchain versions must be pinned and reported in assurance artifacts.

Rationale:
- Strong trust anchor with practical integration path and modern tooling ergonomics.

### D10. Assurance tier acceptance criteria (`L0`-`L3`)
Decision: adopt the following baseline release criteria.

| Tier | Meaning | Minimum Criteria |
| --- | --- | --- |
| `L0` | Assumed | Compiles/tests pass; proof claims may be assumption-heavy; assumptions listed when present. |
| `L1` | Checked core | Core typing/VC checks pass for covered features; unsupported proof areas are explicitly labeled assumed. |
| `L2` | Verified module | Module-level obligations discharged for covered features; no unlabeled assumptions; diagnostics/artifacts enumerate remaining assumptions. |
| `L3` | Verified package profile | Package-level strict verify mode passes, trust-anchor check passes, dependency trust labels complete, and release policy gate accepts manifest. |

Strict-mode rule:
- `L3` is invalid if any unlabeled assumption remains.

## Execution Mapping
- Language/proof execution tasks in `18.0.5` and `18.0.6` consume this lock.
- Assurance and trust-boundary implementation in Phase 19 must preserve this decision baseline unless superseded by a new explicit lock.

## References
- `README.md` (Design Principles)
- `docs/TODO.md` (Phase 18 and 19)
- `docs/diagnostics.md` (`T244`, `T245`, `T246`)
