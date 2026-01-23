# Diagnostics and JSON Errors

ClearLang provides machine-readable diagnostics to keep tooling simple, provable, and AI-friendly.

## CLI Flag
- `--json-errors`: when set on `clg parse`, `clg build`, `clg run`, or `clg verify`, failures are printed as JSON to stdout.

## JSON Shape
```json
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
      "function": "main"
    }
  ]
}
```

- `ok`: always `false` for error output.
- `errors`: one or more error objects.
- `code`: stable error code.
- `stage`: one of `parse` | `type` | `build` | `verify` | `runtime`.
- `message`: concise, human-readable text.
- `file`: input filename as provided to the CLI.
- `start` / `end`: byte offsets in the source.
- `function` (optional): when available, the current function context.

## Error Code Table
| Code | Stage | Meaning |
| --- | --- | --- |
| P001 | parse | Generic parse error. |
| P010 | parse | Missing `else` in expression-form `if`. |
| T000 | type | Fallback for internal type-check failures. |
| T001 | type | Unknown function. |
| T002 | type | Arity mismatch. |
| T003 | type | Argument type mismatch. |
| T004 | type | Return type mismatch. |
| T005 | type | Int operand required. |
| T006 | type | Unknown variable. |
| T008 | type | Duplicate function. |
| T009 | type | Effect not supported (use `pure` or omit). |
| T010 | type | Duplicate parameter. |
| T011 | type | Type-check recursion limit exceeded. |
| T012 | type | Bool operand required. |
| T013 | type | Binary operands must share a type. |
| T014 | type | Contract predicate must be Bool. |
| T016 | type | Block expression requires a tail expression. |
| T101 | type | Collections unavailable (generics/ADTs not implemented yet). |
| T110 | type | Unsigned integer operations for U128/U256 are not supported yet. |
| T111 | type | Unsigned cast expects an unsigned literal or value. |
| T201 | type | Non-exhaustive match (missing arm). |
| T202 | type | Duplicate match arm. |
| T203 | type | Invalid match scrutinee (expected Option/Result). |
| T204 | type | Match arm type mismatch. |
| T205 | type | Binder conflicts with an existing name. |
| T206 | type | Cannot infer element type for `std::{list,set,map}::new()`. |
| T207 | type | Expected collection kind (wrong argument type to a collection API). |
| T208 | type | Element/key/value type mismatch for collection operations. |
| T301 | type | Branch type mismatch in expression-form `if`/`else`. |
| T401 | type | Call requires a stronger effect. |
| T402 | type | Missing required mut guard for a mut call. |
| T403 | type | Mut guard requires a variable as the first argument. |
| T601 | type | `?` requires function return type `Option<_>`. |
| T602 | type | `?` operand must be `Option<_>` or `Result<_, _>`. |
| T603 | type | `?` expects matching `Option` inner type. |
| T604 | type | `?` requires function return type `Result<_, _>`. |
| T605 | type | `?` expects matching `Result` ok/err types. |
| T606 | type | Internal error: missing `$return` binding for `?`. |
| T607 | type | Internal error: missing binding for Option constructor. |
| T608 | type | `None` requires function return type `Option<_>`. |
| T609 | type | Internal error: missing binding for Result constructor. |
| T610 | type | `Ok` requires function return type `Result<_, _>`. |
| T611 | type | `Ok` argument type mismatch. |
| T612 | type | `Err` requires function return type `Result<_, _>`. |
| T613 | type | `Err` argument type mismatch. |
| T701 | type | Duplicate type alias. |
| T702 | type | Type alias conflicts with a resource name. |
| T703 | type | Cyclic refinement alias detected. |
| T704 | type | Refinement predicate must be Bool. |
| T705 | type | Refinement loss on binding/call/return. |
| T706 | type | Refined alias cannot wrap resource types. |
| T707 | type | Refinement predicate must be pure. |
| T708 | type | Refinement predicate is unsatisfiable (shallow check). |
| T801 | type | Resource used after consume. |
| T802 | type | Resource consumed twice. |
| T803 | type | Cannot consume a borrowed resource. |
| T804 | type | Resource ownership mismatch across branches. |
| T805 | type | Resource must be consumed before returning. |
| T806 | type | Resources cannot be stored in collections or tuples yet. |
| T901 | type | While loops in pure functions require a variant for totality. |
| T902 | type | Recursive call requires a decreasing measure. |
| T903 | type | Loop variant is not decreasing. |
| C001 | build | Invalid main signature (only `main() -> Int` is supported in this phase). |
| C002 | build | Missing `main` function. |
| C010 | build | Missing `apply` function for contract build. |
| C011 | build | Missing `query` function for contract build. |
| C012 | build | Contract entrypoint has an invalid signature. |
| C013 | build | Contract build reserves `init`/`handle`; use `apply` instead. |
| V001 | verify | Signature failure (invalid key/signature or malformed signature file). |
| V002 | verify | `clearlang.proof` section missing from module. |
| V003 | verify | Module/proofs hash mismatch. |
| R000 | runtime | Contract guard failed at runtime (detail indicates require/ensure). |
| R001 | runtime | String allocator ran out of memory. |
| R002 | runtime | String/Bytes runtime rejected invalid input or malformed buffer. |
| R003 | runtime | Option/Result variant tag was invalid. |
| R004 | runtime | Runtime limits exceeded (meter/fuel/epoch). |
| R005 | runtime | Unsigned integer overflow. |
| R999 | runtime | Unknown runtime trap (should not appear in released builds). |

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
- Messages include `at <start>..<end>:` when a span is available.
- Codes and JSON shape are stable; text remains concise and actionable.
