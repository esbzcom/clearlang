# ClearLang Typing Rules (Phase 3)



Scope

- Phase 3.1-"3.2 implemented rules: base types (Int, Bool), variables, binary ops, function calls, returns, and effects stub.

- Diagnostics now include source spans (landed in Phase 3.8) and power JSON error emission.



Types

- Primitive types: `Int`, `Bool`.

- Functions: `function name(params) -> Ret`; params are `(name: Type)` pairs.

- Strings: `String` is a primitive type (Phase 4.2 parse/type).

- Bytes: `Bytes` is an opaque byte buffer type (Phase 14 contract ABI surface).

- Chain packages: chain-scoped types (e.g., `std::eth::Address`) are defined in chain packages, not the core type system.



Namespacing (::)

- Calls may use namespaced paths in callee position: `ident ("::" ident)* "(" args ")"`.

- Example: `std::str::len(s)`, `std::list::push(l, x)`, `std::map::get(m, k)`.

- Variables and function definitions remain simple identifiers (no `::` in names).

- Bare paths without `()` (e.g., `std::str::len`) are not expressions and are rejected.

- Rationale: `::` avoids conflicts with `:` (types) and `.` (future member/method and floats), and is familiar for compile-time paths.



Strings (String)

- Literals: delimited by `"..."`; supports escapes `\n`, `\t`, `\r`, `\"`, `\\`, `\0`.

- Multi-line: allowed -" line breaks within the quotes are part of the value until the closing `"`.

- Types: `String` is first-class in parameters and return positions. String literals type to `String`.

- Examples:

  - `pure function literal() -> String { "hello" }`

  - `pure function poem() -> String { "Roses are red\nViolets are blue" }`

  - `pure function echo(s: String) -> String { s }`

- Runtime: `std::str` built-ins (`len/concat/eq`) lower through the string allocator/runtime added in Phase 5.

Bytes (Bytes)

- Type: `Bytes` is a length-prefixed byte buffer (`[u32 len][u8 len]`) in linear memory.
- Conversions: `std::bytes::from_string(String) -> Bytes` and `std::bytes::to_string(Bytes) -> String`.
- Runtime: `std::bytes` built-ins (`len/concat/eq`) share the same layout as `String` and do not require UTF-8.



Effects

- Lattice: `pure` < `mut` < `io`; the default (omitting the keyword) is `pure`.

- `mut` functions may call other `mut` code and the mutable collection intrinsics; a `pure` caller triggers `T401`.

- `io` functions may call host-facing intrinsics such as `std::wasi::print` and `std::env::{time,random}`; a `pure`/`mut` caller triggers `T401`.

- Mutable collection intrinsics (`std::list/set/map::*_mut`) require a guard `require { std::<collection>::can_mut(var) }` in the same function. Missing guards raise `T402`; non-variable first arguments raise `T403`.

- Guard predicates are pure Bool-valued builtins that document aliasing requirements. They surface in VC generation as `mut_pre` obligations so proofs can reference the same guard expression.

- Totality + loops (Phase 9)
  - `while cond invariant { inv } variant { m } { body }` requires `cond: Bool`, `inv: Bool`, and `m: Int`.
  - Invariants are enforced on loop entry and after each body execution; variants must stay non-negative and strictly decrease at runtime (guards trap otherwise).
  - Pure functions must supply a decreasing measure for recursion/loops; recursive self-calls without a measure raise `T902`, missing loop variants raise `T901`, and constant variants raise `T903`.
  - Guardrail: set `CLG_DISABLE_TOTALITY=1` to skip totality enforcement during migration/testing.

Return (expression form)

- Syntax: `return expr` inside an expression-bodied function.

- Semantics (Phase 4.9 minimal): equivalent to evaluating `expr` as the function body's value.

- Typing: `expr` must type to the function's declared return type (enforced at the body level).

- Notes: This is a stepping stone before multi-statement blocks; early returns inside blocks will arrive with statements/control flow.



Typing Environment

- A function environment is built from top-level declarations: name +' (params, return type).

- Per-function, a local environment maps parameter names to their types; duplicate parameter names are rejected.

- Duplicate function names are rejected.



Expression Rules

- Literals: `Int(n): Int`; `Bool(b): Bool`.

- Variables: `Var(x)` has the type from the current local environment; unknown variables error.

- Arithmetic binary ops: `+ - * /` require both operands to be `Int`; result is `Int`.

- Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=` require `Int` operands; result is `Bool`.

- Logical operators: `&&`, `||` require `Bool` operands; unary `!` flips a `Bool`.

- Calls: callee must exist; arity must match; each argument type must equal the parameter type; result is the callee's return type.



Functions

- Body type must equal the declared return type.

- Simple recursion is allowed (no totality checks yet).

- A recursion depth guard prevents runaway type-checking (limit 1024).



Errors (examples)

- Unknown variable: `unknown variable 'x'`.

- Unknown function: `unknown function 'f'`.

- Arity mismatch: `arity mismatch calling 'f': expected N, found M`.

- Arg type mismatch: `arg i type mismatch calling 'f': expected 'Ty', found 'Ty'`.

- Return mismatch: `return type mismatch: declared 'Ty', found 'Ty'`.

- Operand types: `left/right operand must be Int, found 'Ty'`.

- Effects: `effect 'mut'/'io' not supported yet; use 'pure' or omit`.

- Mutable effects: `T401` (missing `mut` effect), `T402` (missing `std::<collection>::can_mut` guard), `T403` (guard argument must be a variable).



Lowering Note (Phase 3.4)

- After successful typing, AST is lowered to IR using SSA-like `Value` ids: parameters are `Value(0..P-1)`, temporaries allocate increasing ids.



Diagnostics Recap (Phase 3.8)

- Source spans are attached to AST nodes and carried into typer errors for precise diagnostics.



Match (Option/Result) -" Minimal Typing (Phase 4.4-"4.5)

- Scope: Only `Option<T>` and `Result<T,E>` scrutinees; no guards; one pattern binder per arm.

- Exhaustiveness:

  - Option: require exactly two arms: `Some(x)` and `None` (order irrelevant). Error if missing/duplicate.

  - Result: require exactly two arms: `Ok(x)` and `Err(e)` (order irrelevant). Error if missing/duplicate.

- Bindings:

  - `Some(x)` binds `x: T` when scrutinee has type `Option<T>`.

  - `Ok(x)` binds `x: T`, `Err(e)` binds `e: E` when scrutinee has type `Result<T,E>`.

  - Pattern binders must not shadow existing parameters/locals; emit a clear error on shadowing.

- Arm typing:

  - Type each arm under its extended environment; both arms must synthesize the same result type `R`.

  - The overall `match` expression has type `R`.

- Invalid scrutinee:

  - Matching on non-`Option`/`Result` types is rejected with a targeted error.



Proposed typer error codes (JSON-stable):

- T201: non-exhaustive `match` (missing arm for Option/Result).

- T202: duplicate or conflicting arm (e.g., two `Some` arms).

- T203: invalid scrutinee type for `match` (expected `Option`/`Result`).

- T204: arm result type mismatch (arms do not agree on a single type).

- T205: pattern binder conflicts with an existing name in scope (shadowing).



Notes

- Parsing exists for `match` in expressions; typing initially emits a clear "not supported yet" error code (T012). The above replaces that once enabled.


ADT Ergonomics (Phase 6.6)

- `if let` sugar: `if let Some(x) = opt { then } else { else }` (and `Ok`/`Err`) desugars to a two-arm `match`. The typer enforces an `Option<T>`/`Result<T,E>` scrutinee and extends the branch environment with the chosen binder. Missing `else` arms remain a parse error (`P010`).
- `??` (Option coalesce) expands to a `match` on the left operand. The left side must have type `Option<T>`; the `Some` arm unwraps to `T` and the `None` arm evaluates the right operand.
- `?` (propagation) is a postfix operator over `Option<T>` and `Result<T,E>`. The operand must have one of those types and the surrounding function must return the corresponding container. Successful typing yields the inner type (`T` or the `Ok` branch). Diagnostics: `T601`/`T604` (return type must be `Option`/`Result`), `T602` (operand not an ADT), `T603`/`T605` (inner type mismatch), `T606` (missing `$return`).
- Constructors obey the same constraints: `Some(v)` infers `Option<T>`; `None` requires an `Option<_>` return site (`T607`/`T608`); `Ok(v)`/`Err(e)` require a `Result<_, _>` return site (`T609`-`T612`) and validate argument types (`T611`/`T613`).
- Lowering and runtime coverage are live: constructors, destructors, and `Expr::Try` all target the canonical variant layout, emit the shared `R003` trap on invalid tags, and surface deterministic diagnostics. Regression coverage includes `crates/cli/tests/cli_it.rs::run_option_result_success_paths`, `crates/cli/tests/cli_it.rs::run_option_result_propagation_paths`, and `crates/cli/tests/cli_it.rs::run_reports_r003_invalid_variant_json`.

## Option/Result Lowering Status (Phase 6.6/7.1)

- **Runtime encoding**: `Option<T>`/`Result<T,E>` reuse the canonical 16-byte layout from `docs/design/phase-7.1-option-result-runtime.md`. `VariantLoad*` guards now trap with `R003` when tags exceed `1`, keeping runtime diagnostics deterministic and AI-friendly.
- **Constructors**: `None`/`Some`/`Ok`/`Err` write the tag and payload words directly and zero the reserved slot. The SSA structure is locked in by `crates/typer/tests/lowering_variants.rs::option_try_lowering_preserves_payload_and_propagation` and `::result_try_lowering_tracks_ok_flow`.
- **`Expr::Try` lowering**: propagation inspects the tag, emits `ReturnIf` to forward `None`/`Err`, and reuses the payload slot when the tag signals success. The same tests assert the value IDs stay stable across constructor/destructor pairs.
- **VC + SMT**: the VC generator now emits the canonical `(tag, payload_lo, payload_hi)` encoding. SMT snapshots declare `cl.variant.{tag,payload_lo,payload_hi}` alongside `cl.option.mk`/`cl.result.mk` so Option/Result reasoning stays machine-friendly and solver-ready.
- **Tooling**: CLI docs now include an `--emit-vcs` walkthrough (see `docs/introduction.md`) and the VC schema example is updated with the canonical helpers.

Collections (Type-Only Summary) -" see `docs/collections.md`

- Types: `List<T>`, `Set<T>`, `Map<K,V>`.

- APIs (pure, type-only):

  - List: `len(List<T>)->Int`, `get(List<T>,Int)->Option<T>`, `push(List<T>,T)->List<T>`, `insert(List<T>,T,Int)->List<T>`, `remove(List<T>,Int)->List<T>`, `pop(List<T>)->Option<T>`, `new()` +' T206.

  - Set: `len(Set<T>)->Int`, `contains(Set<T>,T)->Bool`, `insert(Set<T>,T)->Set<T>`, `remove(Set<T>,T)->Set<T>`, `new()` +' T206.

  - Map: `len(Map<K,V>)->Int`, `contains(Map<K,V>,K)->Bool`, `get(Map<K,V>,K)->Option<V>`, `insert(Map<K,V>,K,V)->Map<K,V>`, `remove(Map<K,V>,K)->Map<K,V>`, `new()` +' T206.

- Diagnostics: T206 (cannot infer `new()`), T207 (expected collection kind), T208 (element/key/value mismatch); index must be Int for list ops (T005).



Refinement Types (Phase 10.1 Design & Scope)

- Status: implemented for alias-only refinements; inline refinements on params/returns remain deferred.
- Syntax: refined aliases only (inline refinements on params/returns are deferred). Form: `type Name<T?...> = Base where binder_pred`, where `binder_pred` is a predicate over a single bound value name plus any type parameters.
- Binder rules: the identifier used in the predicate denotes the aliased value (e.g., `n` in `type Nat = Int where n >= 0`). The binder is scoped only inside the `where` predicate and is not visible at use sites. Each use of the alias introduces a fresh logical binder.
- Predicate well-formedness: must type-check to `Bool` using only the binder, type params, pure arithmetic/boolean ops, and pure built-ins. Effects, mutation, and resource consume ops are disallowed in predicates. Recursive/self-referential alias definitions are rejected.
- Interaction surface: refinements compose with existing contracts (`require`/`ensure`) by adding their predicate to the obligation set when an alias is used. Totality, Option/Result sugar, and loops can reference refined types but cannot weaken them; dropping a refinement without proof will be rejected in later phases.
- Early rejection: trivial contradictions (e.g., `false` or `n < 0 && n >= 0`) are rejected at alias definition time (T708).

Refinement Types (Phase 10.2 Syntax & AST)

- Status: parser/AST implemented; typing and VC generation consume alias predicates. The parser accepts `type` aliases with trailing `where` predicates and stores them in `Program::refined_aliases`.
- Grammar additions: extend type alias forms to accept a trailing `where` clause: `type Name<T? ...> = Type where Ident_pred;`. The predicate expression reuses existing expression grammar restricted to pure, terminating Boolean expressions (`Int` arithmetic/comparisons, logical ops, `true/false`, the binder ident, and type params). Inline refinements on parameters/returns remain disallowed and produce a parse error.
- AST representation: refined-alias nodes carry `name`, optional `type_params`, `base_type`, a best-effort `binder_ident` (first variable seen in the predicate), and the `predicate_expr` (with spans preserved for the alias header and predicate). The binder is not added to the surrounding environment; it is scoped only within the predicate expression node.
- Use-site behavior: each reference to a refined alias instantiates a fresh logical binder when generating obligations; the AST/type entry must keep the predicate attached to the alias definition so typer/VC can retrieve it.
- Diagnostics: reject missing/empty predicates, duplicated binders, or use of effects/resources in predicates with dedicated parse/typer codes. Reserve a parse code for inline refinements to make the restriction clear and stable until lifted.

Refinement Types (Phase 10.3 Typing & Propagation Plan)

- Status: implemented for alias resolution, predicate checking, obligation propagation, and refinement preservation checks, including loop invariant/variant usage.
- Alias env: collect aliases at program load; reject name conflicts with resources/functions and ensure predicates type-check to `Bool` under a synthetic env binding the alias binder and type params. Base types must be well-formed; recursive/self-referential aliases are rejected. Refined aliases cannot wrap resource types (T706).
- Type recognition: when a type name matches a refined alias, treat it as a nominal refined type and carry its predicate alongside the base type. Base-type checks for ops/conditions accept refined Int/Bool aliases. Inline refinements remain invalid.
- Constraint propagation (typing): using a refined alias in a param/let binding/call/return attaches its predicate as an obligation scoped to that value. Substitution of the binder to the concrete value expression is deferred to VC generation. Typing enforces base-type compatibility and rejects refinement loss at bindings (T705); branch joins must agree on the same refined type or they fail with existing branch/match mismatch diagnostics (T301/T204).
- Preservation rules (interaction matrix):
  - Bindings (param/let/call/return): refined values may only flow into the same refined alias; refined to base is rejected (T705).
  - Branch joins: both sides must preserve the same refined alias or the join fails (T301/T204).
  - Containers/sugar: constructors/`??`/`?` preserve refinements; dropping predicates via rewrap/coalesce/try is rejected (T204/T603/T605/T705).
  - Loops: invariants may mention refined binders; loop bodies must not rebind refined values to weaker types (T705).
  - Contracts: `require`/`ensure` add refinement predicates as obligations; predicates remain pure.
- Interactions:
  - `require`/`ensure`: refined params/returns add obligations equivalent to extra requires/ensures; they must remain `pure`.
  - Effects/resources: predicates must stay pure and cannot consume/borrow resources; refined aliases cannot wrap resources (T706) and impure predicates raise T707.
  - Option/Result/sugar: refinements commute with the outer container; pattern matches preserve refinements when projecting. Dropping refinements via constructors/coalesce/try is rejected by type mismatches (T204/T603/T605) and by refinement-loss checks at bindings (T705).
  - Loops/invariants/variants: invariant expressions may reference refined binders; loop bodies cannot rebind refined values to weaker types (T705), preserving refinements across iterations and variant checks.
- Diagnostics: T705 refinement loss on bindings/calls/returns, T706 refined aliases over resources, T707 impure predicates, T708 unsatisfiable predicates. Existing branch/match mismatch codes surface when refinement drops across joins (T301/T204).
- Shallow unsat check: detects contradictory Int bounds over conjunctions of comparisons on `n`, `n + k`, or `n - k`; it does not solve general linear arithmetic.
- VC ordering: preconditions are conjoined in source `require` order, then implicit alias-param predicates, then in-body refinement obligations. Ensure VCs follow source `ensure` order with an implicit refined-return predicate appended.
- VC tie-in (10.4): each refined alias use yields a VC premise that substitutes the binder with the concrete expression; violations surface as standard VC failures rather than runtime traps. See the Phase 10.4 VC/SMT section below.
- Migration/escape hatch: none. Refinement loss remains a hard error to keep typing and VC soundness aligned; migration requires explicit refactors instead of disabling checks.

Examples

Passing refinement flow:

```
type Nat = Int where n >= 0;

pure function inc(n: Nat) -> Nat { n + 1 }
```

Refinement loss at a binding (T705):

```
type Nat = Int where n >= 0;

pure function bad_bind(n: Nat) -> Int { n }
```

Branch join drops refinement (T301):

```
type Nat = Int where n >= 0;

pure function bad_join(n: Nat) -> Int {
    if n > 0 { n } else { 0 }
}
```

Impure predicate (T707):

```
mut function bump(x: Int) -> Int { x + 1 }
type Bad = Int where bump(1) > 0;
```

Refinement VC/SMT (Phase 10.4)

- Refinement obligations are surfaced explicitly in `--emit-vcs` outputs and documented alongside the VC schema in `docs/proofs/vc-schema.md`. Worked fixtures live in `docs/proofs/fixtures` (see `refinement-basic.vc.json`, `refinement-contracts-loops.vc.json`, and `refinement-call-site.vc.json`).




