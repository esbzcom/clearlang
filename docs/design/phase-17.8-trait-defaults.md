# Phase 17.8 - Interface Default Method Bodies

## Status
Design lock for 17.8.1.0.
Implementation tracking lives in `docs/TODO.md` under Phase 17.8.

## Goals
- Add default method bodies to interfaces without changing ClearLang's static-dispatch model.
- Keep interface syntax familiar and parser-deterministic.
- Preserve deterministic typing, diagnostics, and monomorphization behavior.
- Keep effect safety explicit: default bodies must honor declared method effects.

## Design Principles Check
- Simple for users: interface methods use one of two familiar forms (declaration or default body) with no extra syntax.
- AI-friendly: parser/typer acceptance boundaries and diagnostics are explicit and deterministic.
- Provably correct: default methods remain under the same type/effect/proof rules as other function bodies.
- Crypto-focused: no dynamic dispatch, no coherence relaxation, and no effect weakening.

## Non-Goals (17.8.1.0)
- No interface objects or dynamic dispatch.
- No explicit implementation-selection syntax.
- No interface state/fields.
- No specialization, negative implementations, or coherence relaxation.
- No change to existing implementation override signature/effect matching behavior from Phase 17.2.

## Decision Summary (Locked for 17.8.1.0)

### D1. Interface Method Surface
Interface methods support exactly two forms:
1. Declaration form: `effect function name(params...) -> Ret;`
2. Default-body form: `effect function name(params...) -> Ret { ... }`

Mixed forms are invalid (for example, both `;` and `{ ... }` on one method).

### D2. Implementation Completeness Rule
- If an interface method has no default body, implementations must provide it.
- If an interface method has a default body, implementations may omit it and inherit that default.
- Existing checks remain:
  - missing required method: `T233`,
  - extra method in implementation: `T234`,
  - override signature/effect mismatch: `T235`.

### D3. Effect Rule for Default Bodies
- An interface method declaration remains the source of truth for the method effect.
- The default body must type-check under that exact declared effect.
- Effect mismatch between declared method effect and default body is a dedicated type error (`T249` planned).

### D4. Coherence/Dispatch Policy
- Interface defaults do not change coherence.
- Strict coherence is locked in this phase: overlapping implementations are conflicts and must fail at compile time (`T236`), and any unresolved ambiguity at call resolution must fail (`T248`).
- Interface calls remain statically resolved at compile time.
- No runtime dispatch table/vtable semantics are introduced.

### D5. Unsupported Forms Policy (This Slice)
The following stay unsupported and must fail deterministically:
- explicit implementation selection syntax (deferred; keep strict coherence),
- interface state/fields,
- dynamic dispatch through interface objects.

Parser errors use stable parse diagnostics (existing parse code path), and unsupported semantic forms use `T017` until dedicated codes are introduced.

## Locked Syntax (v1 for 17.8.1.x)

Allowed:
```clear
interface Eq {
    pure function eq(a: Self, b: Self) -> Bool;
}

interface Ord {
    pure function le(a: Self, b: Self) -> Bool { a == b }
}
```

Disallowed:
```clear
interface Bad {
    pure function f(x: Int) -> Int; { x }
}
```

## Parser/Typer Acceptance Matrix (17.8.1 boundary)

| Case | Parser | Typer | Diagnostic/Notes |
| --- | --- | --- | --- |
| Interface method declaration form (`...;`) | accept | accept | Baseline existing behavior. |
| Interface method default-body form (`... { ... }`) | accept | accept | New 17.8.1 behavior. |
| Mixed method terminators (`...; { ... }`) | reject | n/a | Stable parse diagnostic (parser error path). |
| Implementation omits interface method without default | accept | reject | `T233` |
| Implementation omits interface method with default | accept | accept | Inherits interface default. |
| Implementation defines method not in interface | accept | reject | `T234` |
| Implementation override signature/effect mismatch | accept | reject | `T235` (existing Phase 17.2 rule). |
| Overlapping implementations for the same interface/type | accept | reject | `T236` (strict coherence). |
| Ambiguous implementation resolution at a call site | accept | reject | `T248` (no implicit selection). |
| Default body violates declared effect | accept | reject | `T249` (planned dedicated code). |
| Explicit implementation-selection syntax attempt | reject/accept | reject | Parse error or `T017` (unsupported in strict coherence model). |

## Diagnostics Plan (Locked for 17.8.1)
- Reuse existing interface diagnostics: `T233`, `T234`, `T235`.
- Keep strict coherence diagnostics explicit: `T236` for overlap conflicts and `T248` for ambiguity.
- Add dedicated `T249`: interface default body effect mismatch.
- Keep parse failures deterministic via stable parser diagnostics for invalid interface method forms.

## Acceptance Criteria for 17.8.1.0 Completion
- Design note exists and is referenced by TODO.
- Design-principles check is explicit and matches README wording.
- V1 scope is frozen to declaration/default-body interface methods only.
- Deterministic diagnostics policy is defined for unsupported forms and effect mismatch.
- Parser/typer acceptance boundaries are documented in a matrix.

## References
- `README.md` (Design Principles)
- `docs/TODO.md`
- `docs/design/phase-17.2-generics-traits.md`
- `docs/diagnostics.md`
