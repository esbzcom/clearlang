# Phase 18.0.5 - Language Ergonomics Execution Lock

## Status
Design lock for `18.0.5.1` in `docs/TODO.md`.
This locks the execution model for ergonomics follow-through after the `18.0.0` decision lock.

## Scope
- Convert `18.0.0` language-surface decisions into deterministic implementation tasks.
- Keep parser/typer behavior explicit for deferred or unsupported forms.
- Define the acceptance boundary for `18.0.5.2` through `18.0.5.4`.

## Design Principles Check
- Simple for users: unsupported forms fail with explicit, stable diagnostics and clear alternatives.
- AI-friendly: parser/typer outputs remain deterministic and machine-readable.
- Provably correct: no surface expansion that weakens current proof guarantees.
- Crypto-focused: deterministic language/runtime behavior remains primary over feature breadth.

## Non-Goals (18.0.5)
- No coherence relaxation.
- No interface/implementation generic surface in this phase.
- No generic refinement aliases in this phase.
- No inline refinements on function params/returns in this phase.
- No expansion of deferred resource/type-surface limits in this phase.

## Decision Mapping from 18.0.0

### D1 - Interface/implementation generics
- Locked behavior: remain unsupported in this product line.
- Required diagnostics: `T246` (interface type params unsupported), `T245` (implementation method type params unsupported).

### D2 - Generic refinement aliases
- Locked behavior: remain deferred.
- Required diagnostics: `T244`.

### D7 - Inline refinements on params/returns
- Locked behavior: alias-first only.
- Required parser behavior: deterministic parse rejection with dedicated code `P013`.
- Required docs behavior: explicitly state alias-only surface and rationale.

### D8 - Deferred resource/type-surface limits
- Locked behavior: keep deferred/disallowed forms.
- Required diagnostics: `T806` for unsupported resource-container forms/operations (including `Set<Resource>` and resource array/slice forms).

## Accepted Ergonomics Execution Changes (This Slice)
1. Add/verify explicit diagnostics + docs rationale for all deferred/rejected forms from D1/D2/D7/D8 (`18.0.5.2`).
2. Keep implementation deterministic: no fallback inference or implicit behavior for unsupported forms (`18.0.5.3`).
3. Add SDK usability gates so diagnostics/import ergonomics regression is measurable (`18.0.5.4`).

## Syntax/Typing/Diagnostics Lock
- Syntax:
  - No new grammar accepted in `18.0.5`.
  - Existing alias-first refinement syntax remains canonical.
- Typing:
  - Existing restriction diagnostics are the normative behavior for this slice (`T244`, `T245`, `T246`, `T806`).
- Diagnostics:
  - All deferred forms must fail with explicit, stable codes/messages.
  - No silent fallback to broader generic parse/type errors when a dedicated restriction exists.

## Acceptance Matrix (18.0.5 Boundary)

| Case | Parser | Typer | Expected result |
| --- | --- | --- | --- |
| Interface type parameters | accept | reject | `T246` |
| Implementation method type parameters | accept | reject | `T245` |
| Generic refinement alias declaration | accept | reject | `T244` |
| Inline param/return refinement syntax | reject | n/a | deterministic parse rejection (`P013`) |
| `Set<Resource>` | accept | reject | `T806` |
| Resource arrays/slices | accept | reject | `T806` |

## Execution Criteria
- `18.0.5.1`: this design lock exists and is referenced from roadmap docs.
- `18.0.5.2`: diagnostics + docs rationale are explicit for each deferred/rejected item.
- `18.0.5.3`: implementation/tests enforce deterministic rejection rules with no silent fallback.
- `18.0.5.4`: SDK usability checks are defined and wired into release gating.

## SDK Usability Gate (18.0.5.4)
- Gate implementation:
  - CLI integration metrics checks in `crates/cli/tests/cli_it/sdk_usability.rs`.
  - CI enforcement step `SDK usability gates` in `.github/workflows/ci.yml`.
- Required gate dimensions:
  - import ergonomics,
  - error quality (stable code/stage/span/actionable wording),
  - migration friction (single, deterministic primary diagnostic for migration fixtures).

## References
- `docs/design/phase-18.0-open-questions.md`
- `docs/TODO.md`
- `docs/typing.md`
- `docs/diagnostics.md`
