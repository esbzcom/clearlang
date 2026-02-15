# Phase 17.7 - First-Class Functions and Closures

## Status
Design note for 17.7.1 decisions.
Implementation tracking lives in `docs/TODO.md` under Phase 17.7.

## Goals
- Add first-class function values and closures with a small, predictable surface.
- Keep user-facing behavior consistent with existing ClearLang effect and ownership rules.
- Preserve deterministic typing/diagnostics so AI repair loops remain stable.
- Keep closure semantics compatible with proof/VC generation.

## Design Principles Check
- Simple for users: one closure form in v1, no extra capture-list syntax, lexical capture only.
- AI-friendly: parser-deterministic grammar, stable diagnostics, and a small acceptance matrix.
- Provably correct: closure creation/invocation integrates with existing contracts/effects and VC obligations.
- Crypto-focused: no unsafe ownership surprises; linear/resource capture is restricted in v1.

## Non-Goals (17.7.1)
- No runtime optimizations (escape analysis, allocation elision, inlining heuristics).
- No borrow-return closures or lifetime inference system.
- No trait redesign (Phase 17.8).
- No new closure-specific effect model.

## Decision Summary (Locked for 17.7.1)

### D1. No New Keywords
- Decision: introduce no new keyword tokens.
- Rationale: keep syntax familiar and avoid parser keyword growth.
- Consequence: anonymous functions reuse existing language forms/tokens.

### D2. Minimal New Syntax Surface
- Decision: permit the smallest anonymous-function expression form needed for closures.
- Rationale: closures require an expression-level constructor; named `function` alone is insufficient.
- Constraint: keep only one canonical lambda shape in v1.

### D3. JS-Like Lexical Capture, Stricter Semantics
- Decision: closures capture lexical scope like JavaScript conceptually.
- Difference from JS:
  - statically typed parameters/returns,
  - effect-checked execution (`pure`/`mut`/`io`),
  - ownership restrictions for linear/resource values.

### D4. Capture Policy (v1)
- Decision: lexical capture only; no explicit capture-list syntax in v1.
- Default: captures are read-only by default.
- Restriction: capturing linear/resource-owned values is rejected in v1 with deterministic diagnostics.

### D5. Effect Policy
- Decision: closure effect is inferred from closure body and captured operations.
- Enforcement: calling context must satisfy the inferred effect, using existing effect lattice rules.
- No special-case escape hatches.

### D6. Contract/VC Policy
- Decision: closure creation and invocation must map to explicit VC obligations.
- Rule: obligations remain deterministic and ordered, following current VC emission conventions.

### D7. Simplicity-over-Implementation Rule
- Decision: prioritize end-user simplicity and consistency, even if typer/lowering/codegen complexity increases.
- Applied to this phase:
  - avoid user-visible capture modes in v1,
  - preserve current ownership/effect mental model,
  - keep diagnostics predictable rather than permissive.

### D8. Canonical Syntax for v1
- Function types use `function(T1, T2, ...) -> R`.
- Lambda expressions use `(x: T, y: U) => expr`.
- Parameter type annotations are required in v1 to keep parsing deterministic.
- Lambda return type is inferred in v1; explicit lambda return-type annotation is deferred.

### D9. Closure Lowering and Call ABI (for 17.7.4)
- Closure runtime representation is a two-word record: `{ code_id: i32, env_ptr: i32 }`.
- `code_id` identifies a synthetic lambda body function compiled for one concrete `function(...) -> ...` signature.
- Invocation ABI passes `env_ptr` as a hidden first argument, then user arguments.
- Capturing lambdas allocate an environment object and store captured values there; `env_ptr` points to that object.
- Non-capturing lambdas use `env_ptr = 0` and still follow the same invoke ABI for consistency.

### D10. Capture-List Syntax Policy
- No explicit capture-list syntax is added in Phase 17.
- Lexical capture remains the only capture mechanism in v1.
- Future capture-list syntax is deferred until there is a demonstrated need that lexical capture cannot express without ambiguity.

### D11. Lambda Return-Type Annotation Policy
- Lambda return type annotations are not part of Phase 17.
- v1 lambdas rely on return-type inference from the body with existing type unification rules.
- If inference fails, typer diagnostics explain the mismatch; no annotation escape hatch is introduced in this phase.

### D12. Dispatch Mechanism Policy (v1)
- Dynamic closure calls use signature-specific dispatcher wrappers, not Wasm `call_indirect` tables.
- Dispatcher shape (conceptual): `__clg_dispatch_fn_sig(code_id, env_ptr, args...) -> ret`.
- Dispatcher behavior:
  - branch on `code_id`,
  - call the matching synthetic lambda body with hidden `env_ptr` + user args,
  - trap with deterministic runtime diagnostics on unknown `code_id` (`R011`).
- Rationale: avoids introducing table/type-index plumbing in the first closure slice while preserving deterministic behavior.

### D13. Closure Environment Lifetime Policy (v1)
- Capturing closure environments are heap-allocated using the existing shared bump allocator.
- No environment deallocation is introduced in Phase 17 (same policy as current collection/runtime allocations).
- Environment lifetime is module-instance lifetime under `clg run`; dropping closure values does not reclaim memory in v1.
- Product-line decision: keep the no-free policy (no bounded reclamation rollout planned in the near term).
- Operational guidance: long-running hosts should recycle module instances/workers to bound closure-environment memory growth.
- Allocation failure reuses existing allocator OOM trap behavior/codes.

### D14. Recursive/Self-Referential Closures Policy (v1)
- Self-referential closure values are rejected in v1.
- Mutual recursion between closure values is rejected in v1.
- Recursion remains supported through named functions under existing totality rules; closures may call named functions.
- Rationale: keep typing/ownership/effect reasoning deterministic without introducing closure fixpoint semantics in Phase 17.

### D15. Function-Value Call Effect Policy (v1)
- Calls through function values are effect-checked conservatively.
- If the callee effect is unknown at type-check time (for example function-typed parameters), invocation requires `io` capability.
- For local `let`-bound lambdas and direct aliases (`let g = f`), typer tracks known closure-body effect and uses that effect when checking later calls.
- Rationale: preserve soundness without adding effect annotations to `function(...) -> ...` types in Phase 17.

### D16. Closure Recursion Diagnostics Policy (v1)
- Typer performs explicit closure dependency-cycle checks for `let name = (..) => ...` bindings in a block.
- Self-cycles and mutual cycles are rejected with deterministic closure-policy diagnostics (not fallback unknown-function errors).
- Rationale: keep recursion policy user-visible and stable while closure lowering is still pending.

### D17. `code_id` Scope Policy (v1)
- `code_id` values are unique across the whole compiled module, not per enclosing function.
- Allocation is deterministic in lowering order so diagnostics and runtime behavior are reproducible.
- Rationale: keeps closure identity stable and avoids namespace coupling between signature dispatchers and function-local state.

### D18. Dispatcher Construction Boundary (v1)
- Signature-specific closure dispatchers are generated at lowering/IR construction time.
- Wasm codegen remains a mechanical encoder of IR instructions and does not own closure semantic decisions.
- Rationale: preserves proof/debug determinism and keeps closure behavior testable at IR-level before backend encoding.

### D19. Completion Criteria Policy for 17.7.4.1/17.7.4.2
- 17.7.4.1/17.7.4.2 are not marked complete until end-to-end invoke path is wired (dispatcher + hidden `env_ptr` ABI + runtime tests).
- Partial milestones (record layout, capture env allocation, `env_ptr = 0` for non-capturing lambdas) are tracked as in-progress notes in TODO.
- Rationale: prevents premature "done" status and keeps rollout state aligned with observable runtime behavior.

## Locked Surface (v1)

Function type:
```
function(Int) -> Int
```

Closure expression:
```
(x: Int) => x + base
```

Notes:
- `function(...) -> ...` is treated as type-form syntax, reusing the existing `function` keyword in type context.

## Ownership and Resource Rules (v1)
- Capturing non-linear immutable values is allowed.
- Capturing linear/resource-owned values is rejected.
- Passing linear/resource values through closure boundaries requires a future design slice with explicit ownership transfer semantics.
- Self-referential closure captures are rejected (see D14).

## Diagnostics Plan (v1)
- Reuse existing diagnostic families where possible (effects, linear ownership, type mismatch).
- Add closure-specific codes only when existing families cannot express the error unambiguously.
- Diagnostics must include stable codes and deterministic spans.
- Parser boundary: malformed `function(...) -> ...`, malformed lambda syntax, or missing lambda parameter types are parser errors.
- Typer boundary: effect incompatibility, capture restrictions (including linear/resource capture), and closure-call type mismatch are typer errors.
- Runtime boundary: unknown `code_id` in closure dispatch traps with deterministic runtime code `R011`.

## Parser/Typer Acceptance Matrix (v1)

| Case | Parser | Typer | Notes |
| --- | --- | --- | --- |
| Typed lambda with non-linear lexical capture | accept | accept | Baseline happy path. |
| Lambda body needs `mut` in `pure` context | accept | reject | Existing effect-gate behavior. |
| Lambda captures resource value | accept | reject | Linear/resource capture restriction in v1. |
| Lambda with missing parameter type | reject | n/a | v1 requires typed lambda params for deterministic parse/type flow. |
| Returning closure from function | accept | accept/reject by effect/ownership rules | No special closure escape rule. |
| Self-referential closure value | accept | reject | Rejected by D14 in v1. |
| `pure` function calls function-typed parameter | accept | reject | Conservatively requires `io` unless callee effect is known (D15). |
| Dynamic closure call with unknown `code_id` | n/a | n/a | Runtime trap via dispatcher (D12). |

## Decision Status
- All originally tracked post-17.7.1 questions are now resolved as D9 through D19.
- New questions should be added only if they introduce user-visible behavior changes not covered above.

## References
- `README.md` (Design Principles)
- `docs/TODO.md`
- `docs/design/phase-8.2-linear-typing.md`
- `docs/design/phase-17.2-generics-traits.md`
- `docs/design/phase-17.6-linear-aware-collections.md`
