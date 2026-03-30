# Phase 25.1.17: U64 Bitvector Bridge and Unsigned Boundary Retirement

## Goal
Retire `unsigned.int_model` for covered `U64` paths while introducing deterministic bitvector encoding artifacts in emitted VC SMT.

## Implementation
1. VC generation now emits a `U64` bitvector bridge prelude when a function has `U64` parameters:
   - `(declare-fun clg.u64.to_int ((_ BitVec 64)) Int)`
   - `(declare-const clg.u64.bv.<param> (_ BitVec 64))`
   - `(assert (= <param> (clg.u64.to_int clg.u64.bv.<param>)))`
2. Assumption boundary emission now treats `U64` as covered for this phase:
   - `unsigned.int_model` is emitted only for uncovered unsigned symbols (currently non-`U64` paths).

## Validation Coverage
- typer VC assumptions tests confirm no unsigned boundary for the `U64` covered fixture.
- CLI VC output/proof-section tests confirm no `unsigned.int_model` item for `U64` covered fixture.
- VC snapshot fixture `proof-model-assumptions.vc.json` updated to include bitvector bridge prelude and retired unsigned boundary entries.

## Non-goals
- This phase does not close bitwise/crypto assumption boundaries.
- Full bitvector semantics for all unsigned widths/intrinsics continue in follow-up tasks.
