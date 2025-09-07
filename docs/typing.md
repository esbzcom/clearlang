# Lumi Typing Rules (Phase 3)

Scope
- Phase 3.1–3.2 implemented rules: base types (Int, Bool), variables, binary ops, function calls, returns, and effects stub.
- Diagnostics with source spans are planned in Phase 3.8.

Types
- Primitive types: `Int`, `Bool`.
- Functions: `fn name(params) -> Ret` where params are `(name: Type)` pairs.
 - Strings: `Str` is a primitive type (Phase 4.2 parse/type).

Namespacing (::)
- Calls may use namespaced paths in callee position: `ident ("::" ident)* "(" args ")"`.
- Example: `std::str::len(s)`, `std::list::push(l, x)`, `std::map::get(m, k)`.
- Variables and function definitions remain simple identifiers (no `::` in names).
- Bare paths without `()` (e.g., `std::str::len`) are not expressions and are rejected.
- Rationale: `::` avoids conflicts with `:` (types) and `.` (future member/method and floats), and is familiar for compile-time paths.

Strings (Str)
- Literals: delimited by `"..."`; supports escapes `\n`, `\t`, `\r`, `\"`, `\\`, `\0`.
- Multi-line: allowed — line breaks within the quotes are part of the value until the closing `"`.
- Types: `Str` is first-class in parameters and return positions. String literals type to `Str`.
- Examples:
  - `pure fn literal() -> Str { "hello" }`
  - `pure fn poem() -> Str { "Roses are red\nViolets are blue" }`
  - `pure fn echo(s: Str) -> Str { s }`
- Not yet: `std::str` built-ins (`len/concat/eq`) are planned as type stubs next; runtime/codegen for strings arrives in Phase 5.

Effects (stub)
- Accepted: `None` (omitted effect) and `pure`.
- Rejected: `mut`, `io` (these error with a clear message in this phase).

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
