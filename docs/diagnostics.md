# Diagnostics and JSON Errors

ClearLang provides machine-readable diagnostics to keep tooling simple, provable, and AI-friendly.

## CLI Flag
- `--json-errors`: when set on `clg parse` or `clg build`, failures are printed as JSON to stdout.

## JSON Shape
```
{
  "ok": false,
  "errors": [
    {
      "code": "T003",
      "stage": "type",
      "message": "at 12..18: arg 1 type mismatch calling `add`: expected `Int`, found `String`",
      "file": "path/to/file.clear",
      "start": 12,
      "end": 18,
      "function": "main" // optional
    }
  ]
}
```

- `ok`: always `false` for error output.
- `errors`: one or more error objects.
- `code`: stable error code.
- `stage`: one of `parse` | `type` | `build`.
- `message`: concise, human-readable text.
- `file`: input filename as provided to the CLI.
- `start` / `end`: byte offsets in the source.
- `function` (optional): when available, the current function context.

## Codes
- Parse (`Pxxx`):
  - `P001`: generic parse error (may appear multiple times per file in the future).
- Type (`Txxx`):
  - `T001`: unknown function
  - `T002`: arity mismatch
  - `T003`: argument type mismatch
  - `T004`: return type mismatch
  - `T005`: Int operand required
  - `T006`: unknown variable
  - `T008`: duplicate function
  - `T009`: effect not supported (use `pure` or omit)
  - `T010`: duplicate parameter
  - `T011`: type-check recursion limit exceeded
  - `T012`: Bool operand required (`!`, `&&`, `||` contract predicates)
  - `T013`: operands of comparison/equality must share a type
  - `T014`: contract predicate must be `Bool`
  - `T101`: collections unavailable (std::{list,set,map} planned in Phase 4.3)
  - `T201`-"`T205`: match typing diagnostics (Option/Result)
  - `T206`: cannot infer element type for `std::{list,set,map}::new()`
  - `T207`: expected collection kind (wrong argument type to a collection API)
  - `T208`: element/key/value type mismatch for collection operations
  - `T301`: branch type mismatch in expression-form conditionals (Phase 4.10)
  - `T701`: duplicate type alias
  - `T702`: type alias conflicts with a resource name
  - `T703`: cyclic refinement alias detected
  - `T704`: refinement predicate must be Bool
  - `T705`: refinement loss on binding/call/return
  - `T706`: refined alias cannot wrap resource types
  - `T707`: refinement predicate must be pure
  - `T708`: refinement predicate is unsatisfiable (shallow check)
  
- Build (`Cxxx`):
  - `C001`: invalid main signature (only `main() -> Int` supported in this phase)
  - `C002`: missing `main` function

- Runtime (`Rxxx`):
  - `R000`: contract guard failed at runtime; detail differentiates `require` vs `ensure`.
  - `R001`: string allocator ran out of memory while materializing a new `String`.
  - `R002`: string runtime rejected a pointer/length pair that would violate UTF-8 invariants.
  - `R999`: reserve for unexpected traps (should not appear in released builds).
  
- Parse (`Pxxx`) additions:
  - `P010`: missing `else` in expression-form `if` (Phase 4.10)

## Examples
- Parse:
```
clg parse bad.clear --json-errors
```
- Build:
```
clg build bad.clear --json-errors -o out.wasm
```

## Notes
- Human messages follow `at <start>..<end>: <message>` for consistency.
- Codes and JSON shape are stable; text remains concise and actionable.



