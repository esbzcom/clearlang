# Phase 19.2.3 - Refinement Ergonomics for Production Proofs

## Status
Design lock for `19.2.3` in `docs/TODO.md`.
This slice closes the remaining refinement ergonomics gaps needed for production proof authoring.

## Goal
Enable practical refinement authoring without alias-only friction by supporting:
1. inline param/return refinements, and
2. generic refinement aliases with deterministic instantiation.

## Design Principles Check
- Simple for users: function signatures can express local refinement intent directly (`x: T where ...`, `-> T where result ...`) while preserving existing alias workflows.
- AI-friendly: parser normalization rewrites inline refinements into deterministic synthetic alias entries, keeping downstream typing/VC artifacts stable.
- Provably correct: refinement predicates remain pure and typed; refinement loss and unsound flows continue to fail closed (`T705` family).
- Crypto-focused: stronger refinement ergonomics reduce contract verbosity for crypto-adjacent invariants while preserving explicit assumption boundaries in proof artifacts.

## Scope
1. Parser support:
   - inline parameter refinements,
   - inline return refinements (binder `result`),
   - deterministic normalization into refined-alias entries.
2. Typer support:
   - generic refined-alias declarations and instantiation via type arguments,
   - alias base-type resolution/substitution with cycle protection preserved.
3. VC/refinement support:
   - instantiated generic alias predicates participate in refinement premises and VC pre/post obligations.
4. Migration fixture updates:
   - prior deferred samples (`01_inline_refinement_param`, `04_generic_refinement_alias`) become passing examples.

## Locked Behavior
1. Inline refinements are normalized to deterministic synthetic alias names (`__clg$inline_ref$...`) before type checking.
2. Return inline refinements require `result` binder syntax.
3. Generic alias uses require matching type-argument counts (`T242` remains authoritative for mismatch).
4. Existing refinement safety checks remain unchanged (no refinement-loss relaxations).

## Non-Goals
1. No new refinement predicate language features beyond current expression grammar.
2. No trait/interface generic-surface expansion (`T245`/`T246` remain in effect).
3. No changes to assurance-tier policy semantics.

## Exit Criteria for 19.2.3
1. Inline param/return refinements parse and type-check in production flows.
2. Generic refined aliases instantiate soundly and appear in VC refinement premises.
3. Migration/CLI integration tests reflect lifted restrictions.
4. TODO/docs are updated to the next execution focus.

## References
- `docs/TODO.md`
- `docs/TODO.md`
- `docs/typing.md`
- `docs/diagnostics.md`
- `clearlang-tests/migration/README.md`
