# Lumi Style Guide

This guide keeps Lumi simple, provable, and AI‑friendly by standardizing naming and diagnostics.

## Naming Conventions
- Modules/namespaces: lower_snake_case.
  - Examples: `std::str`, `std::map`, `std::list`.
- Functions/variables: lower_snake_case.
  - Examples: `std::str::parse_int`, `std::str::len`, `std::map::get`.
- Types/ADTs: PascalCase.
  - Examples: `Int`, `Bool`, `String`, `List<T>`, `Map<K,V>`, `Option<T>`, `Result<T,E>`.
- Type parameters: single uppercase letters.
  - Examples: `T`, `K`, `V`, `E`.
- Constants (future): UPPER_SNAKE_CASE.
- No overloading: one name → one meaning. Prefer distinct names over arity/type-based overloading.

Rationale: consistent casing makes code readable, errors predictable, and tooling (linters/AI) reliable.

## Diagnostics & Error Codes
- Human text pattern: `at <start>..<end>: <message>` (spans are byte offsets).
- JSON output (`--json-errors`): stable fields for automation.
  - Shape: `{ ok: false, errors: [ { code, stage, message, file, start, end, function? } ] }`.
  - Code families: `Pxxx` (parse), `Txxx` (type), `Cxxx` (build/compile).
- Keep messages short and consistent; prefer actionable wording.

## Project Organization (DX)
- Split by responsibility; keep `lib.rs` as a thin façade.
  - Example: `lumi-typer` has `errors`, `builtins`, `check`, `lower`.
- Avoid premature features; prefer staged, testable slices in the roadmap.

## Examples
- Good: `std::str::len(name: String) -> Int`
- Good: `std::map::get(m: Map<K,V>, k: K) -> Option<V>`
- Good: `Result<Int, ParseError>` (future ADT)
- Avoid: mixedCase, kebab-case, or overloading the same name with different arities.

