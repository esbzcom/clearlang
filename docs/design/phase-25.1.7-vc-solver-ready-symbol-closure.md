# Phase 25.1.7 - VC Solver-Ready Symbol Closure

## Status
Implementation lock for `25.1.7` in `docs/TODO.md`.

## Goal
Ensure emitted VC obligations are solver-ready by removing unconstrained local symbol leakage from SMT payloads.

## Delivered
1. Closed local-binding SMT leakage in expression lowering:
   - `Expr::Block` now lowers `let` statements into nested SMT `let` bindings.
2. Closed refinement-flow symbol leakage:
   - let-flow refinement substitutions now use RHS expressions.
   - precondition refinement obligations are filtered to closed/allowed symbols (function-parameter scope).
3. Closed return-refinement SMT leakage:
   - VC SMT prelude instantiates return refinement obligations against the concrete function body expression.
4. Added deterministic parameter declarations to emitted VC SMT:
   - all function parameters are declared with deterministic SMT sorts before obligation clauses.
5. Added regression test coverage for symbol closure and sorted declarations.

## CI/Gate Coverage
Added proof-regression command:
- `cargo test -p clg-typer --test vc vc_smt_declares_parameters_with_sorts_and_closes_let_locals`

## References
- `crates/typer/src/vc/smt.rs`
- `crates/typer/src/vc/refinements/collect.rs`
- `crates/typer/src/vc/generate.rs`
- `crates/typer/src/vc/generate/helpers.rs`
- `crates/typer/tests/vc/refinements_and_assumptions.rs`
- `.github/workflows/ci.yml`
- `crates/cli/tests/ci_workflow.rs`
- `docs/evidence/milestone_3-proof-gate.lock.json`
