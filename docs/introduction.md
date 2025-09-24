# ClearLang

ClearLang is a dependently-typed programming language that compiles to WebAssembly.
It aims to be simple like Python and safe like SPARK Ada, with refinement types,
contracts, and an effect system to eliminate entire classes of bugs at compile time.

---

## Core Ideas

- Dependently-typed core + refinement types: types can depend on values; encode logical properties in types (e.g., `type Nat = Int where n >= 0`).
- Contracts (`require`/`ensure`): pre/postconditions proved statically or guarded at runtime.
- Effects (`pure`, `mut`, `io`): clearly separate pure computation, local mutation, and I/O.
- Totality: pure functions must terminate; loops require invariants or bounds.
- Resource/linear discipline: avoid misuse (e.g., double-spend) by construction.

---

## Applications

- Safe app backends: ClearLang logic to Wasm; UI in React/Swift/Kotlin.
- Smart contracts: invariants like non-negative balances and preserved totals.
- High-assurance systems: finance, medical, aerospace where bugs are unacceptable.

---

## Implementation Strategy

- Target: WebAssembly (portable across web, mobile, blockchain).
- Compiler host: Rust (performance, safety, WASM tooling).
- Parser: chumsky (combinator parser).
- Codegen: wasm-encoder.

---

## CLI

- Commands:
  - `emit-hello`: emits a trivial Wasm with `main() -> i32` returning 42.
  - `parse <FILE>`: parses and pretty-prints the AST for a ClearLang source file.
  - `build <FILE> [--out <PATH>] [--validate] [--debug-names]`: compiles to Wasm via IR.
  - `run <FILE> [--invoke <name>]`: runs a Wasm file (default export: `main`).
- Build pipeline: Parse -> Type-check -> Lower to IR -> Codegen (IR->Wasm) -> write output.
- Flags:
  - `-o, --out <PATH>`: output Wasm path; creates parent directories if needed.
  - `--validate`: run `wasm-tools validate` on the produced Wasm (optional).
  - `--debug-names`: include a Wasm name section with function names.
  - `--json-errors`: emit machine-readable JSON on parse/type/build failures (stable codes + spans).
- Examples:
  - `clg emit-hello -o tmp/hello.wasm`
  - `clg parse clearlang-tests/01_hello.clear`
  - `clg build clearlang-tests/02_arith.clear -o out/arith.wasm --validate`
  - `clg build clearlang-tests/17_hello_str.clear -o out/hello_str.wasm --emit-vcs out/contracts.json`
  - `clg run out/arith.wasm`

---

## Docs

- Typing rules: [docs/typing.md](docs/typing.md)
- IR shape and encoding: [docs/ir.md](docs/ir.md)
- Diagnostics JSON: [docs/diagnostics.md](docs/diagnostics.md)
- Style & naming: [docs/style.md](docs/style.md)

---

## Naming Conventions

- Functions/modules: lower_snake_case (AI-friendly, unambiguous).
  - Examples: `std::str::parse_int`, `std::str::len`, `std::map::get`.
- Types/ADTs: PascalCase.
  - Examples: `Int`, `Bool`, `String`, `List<T>`, `Map<K,V>`, `Option<T>`, `Result<T,E>`.
- Type parameters: single uppercase letters (`T`, `K`, `V`, `E`).
- No overloading: one name ? one meaning (simplifies tooling and diagnostics).

See `docs/style.md` for the full style guide.


---

## Safety Levels

1) Compile Time
- Types/effects: ill-typed or effect-unsafe programs are rejected.
- Contracts (planned): `require`/`ensure` with verification; runtime guards early on.

2) Load Time
- Wasm validation: `wasm-tools validate out.wasm` and Wasmtime module validation.
- Policy checks: deny disallowed imports; require metadata/custom sections when applicable.

3) Runtime
- Contract guards trap deterministically on violation (when compiled in).
- Sandboxing: Wasm memory isolation by default.
- Host limits: Wasmtime fuel/epoch deadlines, memory/table caps.

---

### Windows

- Prereqs: Install Rust (MSVC) and VS Build Tools (C++ workload).
- Build release: `cargo build -p clg-cli --release` -> `target\release\clg.exe`
...
