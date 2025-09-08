# Codex Session Summary

- Implemented AST→IR lowering (Phase 3.4) in `lumi-typer`; `check` now returns an IR `Module`.
- Added lowering tests validating param SSA ids, const, binops, calls, and final `Ret`.
- Cleaned up parser tests by silencing unused variable warnings in ignored typer-bound cases.
- Updated `docs/TODO.md` to mark 3.4 complete and set next focus to 3.5 Integration.
- Next: wire CLI build to the IR path (parse → type → lower → IR→Wasm), adapt codegen to accept IR, and add `--validate`.

## Session 2025-09-02 — Follow-up

- Created `docs/rollout/DEVPLAN.md` detailing Phase 3.5 integration steps (CLI wiring, IR→Wasm mapping, tests).
- Updated `docs/TODO.md` Next focus to Phase 3.5 and referenced DEVPLAN for details.
- Kept `docs/rollout` artifacts ignored in git except `codex-session-history.md` and the new `DEVPLAN.md`.
- Ready to implement CLI Build path switch to IR→Wasm and add `--validate` flag.

## Session 2025-09-03 — Phase 3.5 Done

- Wired CLI build to IR path: parse → type → lower → IR→Wasm; added `--validate`.
- Implemented IR→Wasm for `IConst`, `IBin(Add|Sub|Mul|Div)`, `Call`, and `Ret` (Int/Bool→i32).
- Added `run` subcommand (embedded Wasmtime) to execute Wasm: `lumi run out.wasm`.
- Added initial IR pipeline test; README updated with CLI and Windows instructions.
- TODO updated: Phase 3.5 complete; Next focus on 3.6 Tests and 3.8 Diagnostics & Spans.

## Session 2025-09-03 — Phase 3.6 Done

- Added IR pipeline e2e tests for samples 01–05; confirmed 06 fails to parse; 07 parses but no main export; 08 returns 42.
- Strengthened typer negative tests: arity (too many/zero‑arg), binop operand types, duplicate functions, arg type mismatches.
- Added simple stage logs to `lumi build` (parsed/type‑checked/IR/Wasm bytes/validated).
- Updated TODO: mark all 3.6 items complete; set next focus to 3.8 Diagnostics & Spans and Phase 4 prep.

## Session 2025-09-03 — Phase 3.8 Done; 3.9 Planned

- Added source spans to AST via chumsky and propagated into typer errors; messages now include `at start..end`.
- Adjusted lowering and const-eval to new Expr shapes; added span-aware typer tests.
- Extended tests and samples; added CLI parse/build/run smoke tests and failure cases.
- Introduced `--debug-names` flag and CodegenOpts to optionally emit Wasm name section.
- TODO updated: 3.8 marked complete; added Phase 3.9 (polish/optimizations) and set Next focus to 3.9 then Phase 4.

## Session 2025-09-03 — Plan Reshuffle to Phase 4/5

- Simplified roadmap: split oversized 3.9 into a dedicated Phase 4 and bumped Codegen to Phase 5.
- Phase 4 now covers: namespacing (std::path calls), Strings (parse/type, incl. multi-line), std collections stubs (List/Set/Map) and small DX.
- Phase 5 now covers: IR→Wasm codegen polish (type dedup), callee indices, memory/runtime for Strings + List first, then Set/Map.
- Cleaned TODO duplication; kept a single numbered Phase 4–6 plan; README roadmap/status updated accordingly.

## Session 2025-09-07 — Phase 4.1 Start

- Implemented parser support for namespaced path calls (e.g., `std::str::len(x)`), keeping variables as simple identifiers.
- Added parser tests for namespaced calls and a negative case for bare paths without args.
- Updated DEVPLAN with Phase 4.1 progress notes; typer/built-in wiring to follow in 4.3.

## Session 2025-09-07 — Phase 4.1 Complete

- Added typer negative test to confirm unknown namespaced callees yield clear spanful errors.
- Updated TODO to mark 4.1 fully done (parser + typer acceptance of namespaced callees).

## Session 2025-09-07 — Phase 4.2 (Strings) Parse/Type

- Introduced `Type::Str` and `Expr::Str` with escapes and multi-line support in parser.
- Typer recognizes `Str` as first-class; lowering uses a placeholder until Phase 5 runtime.
- Added parser tests (escapes, multi-line, invalid escape) and typer tests (Str echo, spanful mismatches).
- Added lumi-tests `15_str_literal.lumi` (parse/type only) and `16_namespaced_call.lumi` (parse-only) and updated `full_pipeline` to include them.

## Session 2025-09-07 — Function-only Syntax, Hints, and CLI polish

- Syntax: removed `fn`; `function` is now the exclusive keyword for definitions. Updated parser, tests, samples, and docs.
- Parser UX: added a friendly hint when `:` is used for return types (suggests using `->`).
- Parser DX: completed module split (`tokens`, `types`, `literals`, `path`, `expr`, `func`, `program`).
- Typer: added `std::str` built-in type stubs (`len/concat/eq`).
- CLI bin name: tests reference `lumi` (not `lumi-cli`); docs updated accordingly.
- CLI build: enforces presence of `main() -> Int` and fails on missing/wrong signature.
- Samples: added `17_hello_str.lumi` (parse/type only) and migrated all samples to `function`.

## Session 2025-09-08 — Phase 4.2 Rename Str → String

- Renamed the string type from `Str` to `String` across AST (`Type::String`, `Expr::String`), parser, typer, tests, and samples.
- Updated error messages and tests to expect `String` in diagnostics.
- Left built-in namespace as `std::str::{len, concat, eq}` (no runtime change yet).
- Updated docs: `docs/typing.md`, README status, and `docs/lumi_*` summaries to use `String`.
- Marked TODO item “Rename Str→String” as done; adjusted DEVPLAN terminology accordingly.
- Note: transitional hint for `Str` usage is not added yet (optional). Parser will now expect `String` in type positions.
