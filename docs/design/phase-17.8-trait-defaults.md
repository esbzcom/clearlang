# Phase 17.8 - Trait Default Method Bodies

## Status
Design lock for 17.8.1.0.
Implementation tracking lives in `docs/TODO.md` under Phase 17.8.

## Goals
- Add default method bodies to traits without changing ClearLang's static-dispatch model.
- Keep trait syntax familiar and parser-deterministic.
- Preserve deterministic typing, diagnostics, and monomorphization behavior.
- Keep effect safety explicit: default bodies must honor declared method effects.

## Design Principles Check
- Simple for users: trait methods use one of two familiar forms (declaration or default body) with no extra syntax.
- AI-friendly: parser/typer acceptance boundaries and diagnostics are explicit and deterministic.
- Provably correct: default methods remain under the same type/effect/proof rules as other function bodies.
- Crypto-focused: no dynamic dispatch, no coherence relaxation, and no effect weakening.

## Non-Goals (17.8.1.0)
- No trait objects or dynamic dispatch.
- No explicit impl-selection syntax.
- No trait state/fields.
- No specialization, negative impls, or coherence relaxation.
- No change to existing impl override signature/effect matching behavior from Phase 17.2.

## Decision Summary (Locked for 17.8.1.0)

### D1. Trait Method Surface
Trait methods support exactly two forms:
1. Declaration form: `effect function name(params...) -> Ret;`
2. Default-body form: `effect function name(params...) -> Ret { ... }`

Mixed forms are invalid (for example, both `;` and `{ ... }` on one method).

### D2. Impl Completeness Rule
- If a trait method has no default body, impls must provide it.
- If a trait method has a default body, impls may omit it and inherit that default.
- Existing checks remain:
  - missing required method: `T233`,
  - extra method in impl: `T234`,
  - override signature/effect mismatch: `T235`.

### D3. Effect Rule for Default Bodies
- A trait method declaration remains the source of truth for the method effect.
- The default body must type-check under that exact declared effect.
- Effect mismatch between declared method effect and default body is a dedicated type error (`T249` planned).

### D4. Coherence/Dispatch Policy
- Trait defaults do not change coherence.
- Trait calls remain statically resolved at compile time.
- No runtime dispatch table/vtable semantics are introduced.

### D5. Unsupported Forms Policy (This Slice)
The following stay unsupported and must fail deterministically:
- explicit impl selection syntax (deferred; keep strict coherence),
- trait state/fields,
- dynamic dispatch through trait objects.

Parser errors use stable parse diagnostics (existing parse code path), and unsupported semantic forms use `T017` until dedicated codes are introduced.

## Locked Syntax (v1 for 17.8.1.x)

Allowed:
```clear
trait Eq {
    pure function eq(a: Self, b: Self) -> Bool;
}

trait Ord {
    pure function le(a: Self, b: Self) -> Bool { a == b }
}
```

Disallowed:
```clear
trait Bad {
    pure function f(x: Int) -> Int; { x }
}
```

## Parser/Typer Acceptance Matrix (17.8.1 boundary)

| Case | Parser | Typer | Diagnostic/Notes |
| --- | --- | --- | --- |
| Trait method declaration form (`...;`) | accept | accept | Baseline existing behavior. |
| Trait method default-body form (`... { ... }`) | accept | accept | New 17.8.1 behavior. |
| Mixed method terminators (`...; { ... }`) | reject | n/a | Stable parse diagnostic (parser error path). |
| Impl omits trait method without default | accept | reject | `T233` |
| Impl omits trait method with default | accept | accept | Inherits trait default. |
| Impl defines method not in trait | accept | reject | `T234` |
| Impl override signature/effect mismatch | accept | reject | `T235` (existing Phase 17.2 rule). |
| Default body violates declared effect | accept | reject | `T249` (planned dedicated code). |
| Explicit impl-selection syntax attempt | reject/accept | reject | Parse error or `T017` (unsupported in strict coherence model). |

## Diagnostics Plan (Locked for 17.8.1)
- Reuse existing trait diagnostics: `T233`, `T234`, `T235`.
- Add dedicated `T249`: trait default body effect mismatch.
- Keep parse failures deterministic via stable parser diagnostics for invalid trait method forms.

## Acceptance Criteria for 17.8.1.0 Completion
- Design note exists and is referenced by TODO.
- Design-principles check is explicit and matches README wording.
- V1 scope is frozen to declaration/default-body trait methods only.
- Deterministic diagnostics policy is defined for unsupported forms and effect mismatch.
- Parser/typer acceptance boundaries are documented in a matrix.

## References
- `README.md` (Design Principles)
- `docs/TODO.md`
- `docs/design/phase-17.2-generics-traits.md`
- `docs/diagnostics.md`
