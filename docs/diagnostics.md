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
| P011 | parse | Re-export syntax `export import` is not supported in v1. |
| P012 | parse | Capture-list lambda syntax is not supported in v1. |
| P013 | parse | Reserved legacy code from the Phase 18 deferred-inline period (no longer emitted in Phase 19.2.3+). |
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
| T218 | type | Resource fields require `resource struct`/`resource enum`. |
| T219 | type | Duplicate enum variant. |
| T220 | type | Non-equatable key type for `Map`/`Set`. |
| T230 | type | Duplicate interface. |
| T231 | type | Unknown interface. |
| T232 | type | Duplicate interface method. |
| T233 | type | Interface implementation missing method. |
| T234 | type | Interface implementation has extra method. |
| T235 | type | Interface method signature mismatch. |
| T236 | type | Overlapping interface implementations. |
| T237 | type | Missing interface implementation/bound for a type. |
| T238 | type | Cannot infer type parameters. |
| T239 | type | Duplicate type parameter. |
| T240 | type | Type parameter conflicts with existing type name. |
| T241 | type | Unknown type parameter in interface bound. |
| T242 | type | Type argument count mismatch. |
| T243 | type | Type parameter cannot take type arguments. |
| T244 | type | Reserved legacy code from the Phase 18 deferred-generic-refinement period (no longer emitted in Phase 19.2.3+). |
| T245 | type | Implementation methods cannot declare their own type parameters yet. |
| T246 | type | Interface type parameters not supported yet. |
| T247 | type | Conflicting inferred types for a type parameter. |
| T248 | type | Ambiguous implementation for an interface on a type. |
| T249 | type | Interface default method body effect does not match declared effect. |
| T250 | type | Mangled-name collision under shortening mode (`CLG_MANGLE_MAX_LEN`). |
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
| C014 | build | Strict proof mode detected invalid assumption-boundary metadata for emitted VCs. |
| C020 | build | Unknown module in import. |
| C021 | build | Imported item is missing or not exported. |
| C022 | build | Import name conflicts with an existing name. |
| C023 | build | Module header does not match file path. |
| C024 | build | Duplicate module path across source files. |
| C025 | build | Import cycle detected. |
| C026 | build | Reserved std module path used in user code. |
| C027 | build | Invalid compiled-package metadata/ABI contract for standard or permissive build/run indexing (legacy `clg-packages.json` is rejected; canonical `clg.package-metadata.json` and `clg.package-abi.json` must be valid and aligned). |
| C028 | build | Module path conflict between source files and compiled-package metadata. |
| C029 | build | Strict compiler mode requires `--emit-vcs <FILE>`. |
| C030 | build | Strict compiler mode cannot be combined with `--proof-strict=false`. |
| C031 | build | Strict compiler mode L3 claim is blocked because unlabeled assumptions remain. |
| C032 | build | Trust-anchor checker version flags are invalid for this build command combination. |
| C033 | build | Strict language profile rejected a deferred/unchecked assumed boundary surface. |
| C034 | build | Assurance manifest output flag is invalid for this build command combination. |
| C035 | build | Invalid std-core activation contract for the selected build mode/profile (for Phase 21, `--std-core-link-mode precompiled` requires `--compiler-mode strict`). |
| C101 | build | Strict source-of-truth gate failed (e.g., missing/unreadable `clg.lock.json` or disallowed package source such as legacy `clg-packages.json` in strict preflight context). Remediation: provide strict preflight inputs only and resolve packages via lockfile + trusted local store. |
| C102 | build | Strict artifact identity gate failed (invalid lockfile digest format or `(name, version, digest)` mismatch between `clg.lock.json` and strict package metadata). |
| C103 | build | Strict trust gate failed (missing/unreadable/malformed trust-policy or package-signature envelope, untrusted/revoked signer, invalid signer window at signature timestamp, or signature verification failure). Remediation: provide valid trust policy/signature preflight inputs and trusted signer keys. |
| C104 | build | Strict preflight schema input is malformed or unsupported (strict lockfile v0, strict package metadata v0/v1, strict package ABI v0). Remediation: provide valid accepted schema files with required keys only. |
| C105 | build | Strict ABI/link gate failed (package metadata/ABI contract mismatch, unresolved ABI import symbol, or resolved symbol signature/effect mismatch against strict ABI expectations). Remediation: align strict ABI contracts with resolved external import surface exactly. |
| C106 | build | Strict host-profile validation failed (missing/unreadable/malformed profile or required capability absent). Remediation: provide valid `clg.host-profile.json` schema v0 with required capability ids. |
| C107 | build | Strict determinism replay failed (identical strict inputs did not produce identical canonical direct-dependency import map and diagnostics ordering). Remediation: normalize gate evaluation ordering and import-map serialization to be deterministic. |
| C108 | build | Strict import-map artifact emission failed (canonical strict preflight artifact could not be written). Remediation: fix output path/permissions and retry the strict build. |
| C109 | build | Package metadata model compatibility failure (conflicting or unsupported metadata model, including migration/coexistence violations). |
| C110 | build | Metadata trust-anchor/signature linkage failure against trust policy. |
| C111 | build | Lockfile generate/update input contract failure (invalid roots or policy preconditions). |
| C112 | build | Deterministic transitive dependency cycle detected in package resolution graph. |
| C113 | build | Deterministic semver solver found no satisfiable version set for constraints. |
| C114 | build | Deterministic semver tie-break policy could not select a unique candidate. |
| C115 | build | Advisory deny policy rejected selected package version(s). |
| C116 | build | Advisory forced-upgrade policy could not find a compliant resolvable version. |
| C117 | build | Advisory policy input is missing/invalid (including strict-mode `--advisory-as-of` precondition, malformed advisory schema/timestamps, or strict trust/signature verification failure). |
| C118 | build | Build/run resolution policy mismatch for equivalent package inputs. |
| C119 | build | Resolver/solver replay determinism mismatch on identical inputs (graph/lockfile/diagnostics drift). |
| C120 | build | `--release-profile production` requires `--compiler-mode strict`. |
| C121 | build | Production release profile requires theorem-grade assurance (`proof_status=proved_all`). |
| C122 | build | Production release profile only permits proved std release surfaces from the canonical proof matrix allowlist. |
| C123 | build | Production release profile blocks unresolved crypto proof boundaries (`crypto.uninterpreted`) with fail-closed diagnostics. |
| V001 | verify | Signature failure (invalid key/signature or malformed signature file). |
| V002 | verify | `clearlang.proof` section missing from module. |
| V003 | verify | Module/proofs hash mismatch. |
| V004 | verify | Trust-anchor verification failed (missing policy, policy parse/schema failure, payload trust-anchor mismatch). |
| V005 | verify | Release policy/assurance gate failed (invalid policy/manifest, manifest tier below required minimum, or `proved_all` requested while assumption boundaries/disallowed release surfaces remain). |
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
| R012 | runtime | Runtime package artifact is missing/unavailable for a locked runtime dependency. |
| R013 | runtime | Runtime package artifact digest mismatch against locked/runtime-link evidence. |
| R014 | runtime | Runtime package signature/trust verification failed (untrusted/revoked/invalid signer or signature mismatch). |
| R015 | runtime | Runtime package ABI/link binding mismatch (import contract does not match runtime provider symbol contract). |
| R016 | runtime | Runtime loader capability/profile mismatch (invalid host-profile input or required host capability absent under active profile). |
| R017 | runtime | Runtime loader determinism replay mismatch on identical inputs (loaded package set/binding map/diagnostics drift). |
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

