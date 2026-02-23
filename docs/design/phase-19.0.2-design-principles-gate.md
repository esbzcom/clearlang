# Phase 19.0.2 - Design Principles Gate Lock

## Status
Design lock for `19.0.2` in `docs/TODO.md`.
This preserves README design principles as a required gate for every Phase 19 design/implementation slice.

## Goal
Make the README design principles an explicit, enforceable release gate for all `19.x` changes:
- simple for users,
- AI-friendly,
- provably correct,
- crypto-focused.

## Scope
1. Process gate:
   - Every `docs/design/phase-19*.md` document must include a `## Design Principles Check` section.
   - That section must address all four principles explicitly.
2. Regression gate:
   - CI/workspace tests must fail if a new Phase 19 design doc omits the section or any required principle label.
3. Roadmap gate:
   - TODO/DEVPLAN references must point to this lock for `19.0.2`.

## Design Principles Check
- Simple for users: each Phase 19 slice must state how UX complexity is reduced or bounded.
- AI-friendly: each slice must preserve deterministic diagnostics/artifacts for tooling loops.
- Provably correct: each slice must avoid hidden assumptions and keep proof boundaries explicit.
- Crypto-focused: each slice must preserve deterministic, audit-grade behavior in contract/runtime contexts.

## Enforcement Rules
1. File selection:
   - Scope includes all files matching `docs/design/phase-19*.md`.
2. Required section:
   - Must contain heading `## Design Principles Check`.
3. Required labels:
   - Section/file content must contain each exact label:
     - `Simple for users`
     - `AI-friendly`
     - `Provably correct`
     - `Crypto-focused`

## Implementation
- Added regression test:
  - `crates/cli/tests/phase19_design_principles.rs`
- The test scans `docs/design/phase-19*.md` files and enforces the rules above.
- CI already executes workspace tests, so this gate is active without extra workflow steps.

## Non-Goals
- No policy about prose style beyond required gate labels.
- No requirement to backfill pre-19 docs.
- No language/runtime behavior changes in this slice.

## Exit Criteria for 19.0.2
1. This lock doc is published and referenced from roadmap docs.
2. Automated regression test is present and passing.
3. `19.0.2` is marked complete in TODO with evidence links.

## References
- `README.md`
- `docs/TODO.md`
- `docs/rollout/DEVPLAN.md`
- `docs/design/phase-19.0.1-success-target.md`
