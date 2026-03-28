# Phase 25.0.15 - Defer `theorem` Keyword

## Status
Design lock + implementation note for `25.0.15` in `docs/TODO.md`.

## Goal
Keep Milestone 3 language surface minimal by deferring any dedicated `theorem` syntax and preserving theorem-grade as certification status from release gates.

## Policy
1. `theorem` is reserved and rejected in parser input for milestone_3.
2. Parser emits deterministic structured parse code `P014` with explicit guidance:
   - theorem-grade is a build/verify certification outcome (`proved_all`), not syntax.
3. User docs must reinforce this distinction.

## Enforcement Wiring
- Parser keyword reservation:
  - `crates/parser/src/tokens.rs`
- Structured parse code mapping:
  - `crates/parser/src/program.rs`
- Parser regression:
  - `crates/parser/tests/parse_structured_errors.rs`
- CLI parse regression:
  - `crates/cli/tests/cli_it/basic.rs`
- User-facing wording:
  - `README.md`
  - `docs/diagnostics.md`

## Exit Criteria for 25.0.15
1. `theorem` input is rejected with `P014`.
2. CLI `--json-errors parse` surfaces `P014` deterministically.
3. Docs explicitly state theorem-grade is certification status, not milestone_3 syntax.

## References
- `docs/TODO.md`
- `README.md`
- `docs/diagnostics.md`
- `docs/design/phase-25.0.2-theorem-grade-certification-policy.md`
