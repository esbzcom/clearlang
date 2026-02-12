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
| T017 | type | Feature not supported yet. |
| T101 | type | Collections unavailable (generics/ADTs not implemented yet). |
| T110 | type | Unsigned integer operations for U128/U256 are not supported yet. |
| T111 | type | Unsigned cast expects an unsigned literal or value. |
| T112 | type | Unsigned literal is out of range for the target type. |
| T113 | type | Unsigned constant expression overflows (compile-time). |
| T114 | type | Array index is out of bounds. |
| T115 | type | Tuple index must be a constant integer. |
| T116 | type | Array length mismatch. |
| T201 | type | Non-exhaustive match (missing arm). |
| T202 | type | Duplicate match arm. |
| T203 | type | Invalid match scrutinee (expected Option/Result/enum). |
| T204 | type | Match arm type mismatch. |
| T205 | type | Binder conflicts with an existing name. |
| T206 | type | Cannot infer element type for `std::{list,set,map}::new()`. |
| T207 | type | Expected collection kind (wrong argument type to a collection API). |
| T208 | type | Element/key/value type mismatch for collection operations. |
| T209 | type | Unreachable match arm. |
| T210 | type | Unknown type name. |
| T211 | type | Unknown struct field. |
| T212 | type | Missing struct field in literal. |
| T213 | type | Duplicate struct field. |
| T214 | type | Struct field type mismatch. |
| T215 | type | Unknown enum variant. |
| T216 | type | Enum variant arity mismatch. |
| T217 | type | Expected struct type (field access or literal). |
| T218 | type | Resource fields are not allowed in structs/enums yet. |
| T219 | type | Duplicate enum variant. |
| T220 | type | Non-equatable key type for `Map`/`Set`. |
| T230 | type | Duplicate trait. |
| T231 | type | Unknown trait. |
| T232 | type | Duplicate trait method. |
| T233 | type | Trait impl missing method. |
| T234 | type | Trait impl has extra method. |
| T235 | type | Trait method signature mismatch. |
| T236 | type | Overlapping trait impls. |
| T237 | type | Missing trait implementation/bound for a type. |
| T238 | type | Cannot infer type parameters. |
| T239 | type | Duplicate type parameter. |
| T240 | type | Type parameter conflicts with existing type name. |
| T241 | type | Unknown type parameter in trait bound. |
| T242 | type | Type argument count mismatch. |
| T243 | type | Type parameter cannot take type arguments. |
| T244 | type | Generic refinement aliases not supported yet. |
| T245 | type | Impl methods cannot declare their own type parameters yet. |
| T246 | type | Trait type parameters not supported yet. |
| T247 | type | Conflicting inferred types for a type parameter. |
| T248 | type | Ambiguous impl for a trait on a type. |
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
| T701 | type | Duplicate type name. |
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
| T806 | type | Unsupported resource-collection form or operation (e.g., `Set<Resource>`, array/slice forms containing resources, or non-`*_take` move-out path). |
| T901 | type | While loops in pure functions require a variant for totality. |
| T902 | type | Recursive call requires a decreasing measure. |
| T903 | type | Loop variant is not decreasing. |
| C001 | build | Invalid main signature (only `main() -> Int` is supported in this phase). |
| C002 | build | Missing `main` function. |
| C010 | build | Missing `apply` function for contract build. |
| C011 | build | Missing `query` function for contract build. |
| C012 | build | Contract entrypoint has an invalid signature. |
| C013 | build | Contract build reserves `init`/`handle`; use `apply` instead. |
| C020 | build | Unknown module in import. |
| C021 | build | Imported item is missing or not exported. |
| C022 | build | Import name conflicts with an existing name. |
| C023 | build | Module header does not match file path. |
| C024 | build | Duplicate module path across source files. |
| C025 | build | Import cycle detected. |
| C026 | build | Reserved std module path used in user code. |
| V001 | verify | Signature failure (invalid key/signature or malformed signature file). |
| V002 | verify | `clearlang.proof` section missing from module. |
| V003 | verify | Module/proofs hash mismatch. |
| R000 | runtime | Contract guard failed at runtime (detail indicates require/ensure). |
| R001 | runtime | String allocator ran out of memory. |
| R002 | runtime | Runtime rejected invalid input or malformed buffer. |
| R003 | runtime | Option/Result/Enum variant tag was invalid. |
| R004 | runtime | Runtime limits exceeded (meter/fuel/epoch). |
| R005 | runtime | Unsigned integer overflow. |
| R006 | runtime | Crypto algorithm unsupported or unknown. |
| R007 | runtime | Crypto input length is invalid for the selected algorithm. |
| R008 | runtime | Crypto input is malformed (e.g., invalid signature encoding). |
| R009 | runtime | Collection bounds error (e.g., list insert/remove index out of bounds). |
| R010 | runtime | Invalid collection handle (null, misaligned, or out-of-bounds header). |
| R011 | runtime | Closure dispatch received an unknown `code_id`. |
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
- Runtime (crypto):
```
clg run bad-crypto.clear --json-errors
```
```json
{
  "ok": false,
  "errors": [
    {
      "code": "R006",
      "stage": "runtime",
      "message": "unsupported crypto algorithm: sha999",
      "file": "bad-crypto.clear",
      "start": 0,
      "end": 0
    }
  ]
}
```

## Notes
- Messages include `at <start>..<end>:` when a span is available.
- Codes and JSON shape are stable; text remains concise and actionable.

