# ClearLang Typing Rules (Phase 3)



Scope

- Phase 3.1-"3.2 implemented rules: base types (Int, Bool), variables, binary ops, function calls, returns, and effects stub.

- Diagnostics now include source spans (landed in Phase 3.8) and power JSON error emission.



Types

- Primitive types: `Int`, `Bool`.

- Functions: `function name(params) -> Ret`; params are `(name: Type)` pairs.

- Strings: `String` is a primitive type (Phase 4.2 parse/type).



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



Effects

- Lattice: `pure` < `mut` < `io`; the default (omitting the keyword) is `pure`.

- `mut` functions may call other `mut` code and the mutable collection intrinsics; a `pure` caller triggers `T401`.

- `io` remains reserved; using it still raises `T009` until the runtime surface is ready.

- Mutable collection intrinsics (`std::list/set/map::*_mut`) require a guard `require { std::<collection>::can_mut(var) }` in the same function. Missing guards raise `T402`; non-variable first arguments raise `T403`.

- Guard predicates are pure Bool-valued builtins that document aliasing requirements. They surface in VC generation as `mut_pre` obligations so proofs can reference the same guard expression.



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

- `if let` sugar: `if let Some(x) = opt { then } else { else }` (and `Ok`/`Err`) desugars to a two-arm `match`. The typer enforces an `Option<T>`/`Result<T,E>` scrutinee and extends the branch environment with the chosen binder. Missing `else` arms remain a parse error (`P011`).
- `??` (Option coalesce) expands to a `match` on the left operand. The left side must have type `Option<T>`; the `Some` arm unwraps to `T` and the `None` arm evaluates the right operand.
- `?` (propagation) is a postfix operator over `Option<T>` and `Result<T,E>`. The operand must have one of those types and the surrounding function must return the corresponding container. Successful typing yields the inner type (`T` or the `Ok` branch). Diagnostics: `T601`/`T604` (return type must be `Option`/`Result`), `T602` (operand not an ADT), `T603`/`T605` (inner type mismatch), `T606` (missing `$return`).
- Constructors obey the same constraints: `Some(v)` infers `Option<T>`; `None` requires an `Option<_>` return site (`T607`/`T608`); `Ok(v)`/`Err(e)` require a `Result<_, _>` return site (`T609`-`T612`) and validate argument types (`T611`/`T613`).
- Lowering and runtime coverage are live: constructors, destructors, and `Expr::Try` all target the canonical variant layout, emit the shared `R003` trap on invalid tags, and surface deterministic diagnostics. Regression coverage includes `crates/cli/tests/cli_it.rs::run_option_result_success_paths`, `crates/cli/tests/cli_it.rs::run_option_result_propagation_paths`, and `crates/cli/tests/cli_it.rs::run_reports_r003_invalid_variant_json`.

## Option/Result Lowering Status (Phase 6.6/7.1)

- **Runtime encoding**: `Option<T>`/`Result<T,E>` reuse the canonical 16-byte layout from `docs/design/phase-7.1-option-result-runtime.md`. `VariantLoad*` guards now trap with `R003` when tags exceed `1`, keeping runtime diagnostics deterministic and AI-friendly.
- **Constructors**: `None`/`Some`/`Ok`/`Err` write the tag and payload words directly and zero the reserved slot. The SSA structure is locked in by `crates/typer/tests/lowering_variants.rs::option_try_lowering_preserves_payload_and_propagation` and `::result_try_lowering_tracks_ok_flow`.
- **`Expr::Try` lowering**: propagation inspects the tag, emits `ReturnIf` to forward `None`/`Err`, and reuses the payload slot when the tag signals success. The same tests assert the value IDs stay stable across constructor/destructor pairs.
- **VC + SMT**: upcoming work replaces the placeholder comments with `(tag, lo, hi)` tuples so SMT snapshots and docs agree on the runtime contract (see TODO 7.2).
- **Tooling**: CLI docs and the `--emit-vcs` walkthrough will be refreshed alongside the VC update to keep the examples proof-ready and machine-verifiable.

Collections (Type-Only Summary) -" see `docs/collections.md`

- Types: `List<T>`, `Set<T>`, `Map<K,V>`.

- APIs (pure, type-only):

  - List: `len(List<T>)->Int`, `get(List<T>,Int)->Option<T>`, `push(List<T>,T)->List<T>`, `insert(List<T>,T,Int)->List<T>`, `remove(List<T>,Int)->List<T>`, `pop(List<T>)->Option<T>`, `new()` +' T206.

  - Set: `len(Set<T>)->Int`, `contains(Set<T>,T)->Bool`, `insert(Set<T>,T)->Set<T>`, `remove(Set<T>,T)->Set<T>`, `new()` +' T206.

  - Map: `len(Map<K,V>)->Int`, `contains(Map<K,V>,K)->Bool`, `get(Map<K,V>,K)->Option<V>`, `insert(Map<K,V>,K,V)->Map<K,V>`, `remove(Map<K,V>,K)->Map<K,V>`, `new()` +' T206.

- Diagnostics: T206 (cannot infer `new()`), T207 (expected collection kind), T208 (element/key/value mismatch); index must be Int for list ops (T005).







