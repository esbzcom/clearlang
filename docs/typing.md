# ClearLang Typing Rules (Phase 3)

Scope
- Phase 3.1–3.2 implemented rules: base types (Int, Bool), variables, binary ops, function calls, returns, and effects stub.
- Diagnostics with source spans are planned in Phase 3.8.

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
- Multi-line: allowed — line breaks within the quotes are part of the value until the closing `"`.
- Types: `String` is first-class in parameters and return positions. String literals type to `String`.
- Examples:
  - `pure function literal() -> String { "hello" }`
  - `pure function poem() -> String { "Roses are red\nViolets are blue" }`
  - `pure function echo(s: String) -> String { s }`
- Not yet: `std::str` built-ins (`len/concat/eq`) are planned as type stubs next; runtime/codegen for strings arrives in Phase 5.

Effects (stub)
- Accepted: `None` (omitted effect) and `pure`.
- Rejected: `mut`, `io` (these error with a clear message in this phase).

Return (expression form)
- Syntax: `return expr` inside an expression-bodied function.
- Semantics (Phase 4.9 minimal): equivalent to evaluating `expr` as the function body’s value.
- Typing: `expr` must type to the function’s declared return type (enforced at the body level).
- Notes: This is a stepping stone before multi-statement blocks; early returns inside blocks will arrive with statements/control flow.

Typing Environment
- A function environment is built from top-level declarations: name → (params, return type).
- Per-function, a local environment maps parameter names to their types; duplicate parameter names are rejected.
- Duplicate function names are rejected.

Expression Rules
- Literals: `Int(n): Int`; `Bool(b): Bool`.
- Variables: `Var(x)` has the type from the current local environment; unknown variables error.
- Binary ops: `+ - * /` require both operands to be `Int`; result is `Int`.
- Calls: callee must exist; arity must match; each argument type must equal the parameter type; result is the callee’s return type.

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

Lowering Note (Phase 3.4)
- After successful typing, AST is lowered to IR using SSA-like `Value` ids: parameters are `Value(0..P-1)`, temporaries allocate increasing ids.

Planned (Phase 3.8)
- Attach source spans to AST nodes and carry them into typer errors for precise diagnostics.

Match (Option/Result) — Minimal Typing (Phase 4.4–4.5)
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
- Parsing exists for `match` in expressions; typing initially emits a clear “not supported yet” error code (T012). The above replaces that once enabled.
Collections (Type-Only Summary) — see `docs/collections.md`
- Types: `List<T>`, `Set<T>`, `Map<K,V>`.
- APIs (pure, type-only):
  - List: `len(List<T>)->Int`, `get(List<T>,Int)->Option<T>`, `push(List<T>,T)->List<T>`, `insert(List<T>,T,Int)->List<T>`, `remove(List<T>,Int)->List<T>`, `pop(List<T>)->Option<T>`, `new()` → T206.
  - Set: `len(Set<T>)->Int`, `contains(Set<T>,T)->Bool`, `insert(Set<T>,T)->Set<T>`, `remove(Set<T>,T)->Set<T>`, `new()` → T206.
  - Map: `len(Map<K,V>)->Int`, `contains(Map<K,V>,K)->Bool`, `get(Map<K,V>,K)->Option<V>`, `insert(Map<K,V>,K,V)->Map<K,V>`, `remove(Map<K,V>,K)->Map<K,V>`, `new()` → T206.
- Diagnostics: T206 (cannot infer `new()`), T207 (expected collection kind), T208 (element/key/value mismatch); index must be Int for list ops (T005).
