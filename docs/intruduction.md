# lumi-lang

Lumi is a dependently-typed programming language that compiles to WebAssembly.
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

- Safe app backends: Lumi logic to Wasm; UI in React/Swift/Kotlin.
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
  - `parse <FILE>`: parses and pretty-prints the AST for a Lumi source file.
  - `build <FILE> [--out <PATH>] [--validate] [--debug-names]`: compiles to Wasm via IR.
  - `run <FILE> [--invoke <name>]`: runs a Wasm file (default export: `main`).
- Build pipeline: Parse -> Type-check -> Lower to IR -> Codegen (IR->Wasm) -> write output.
- Flags:
  - `-o, --out <PATH>`: output Wasm path; creates parent directories if needed.
  - `--validate`: run `wasm-tools validate` on the produced Wasm (optional).
  - `--debug-names`: include a Wasm name section with function names.
  - `--json-errors`: emit machine-readable JSON on parse/type/build failures (stable codes + spans).
- Examples:
  - `lumi emit-hello -o tmp/hello.wasm`
  - `lumi parse lumi-tests/01_hello.lumi`
  - `lumi build lumi-tests/02_arith.lumi -o out/arith.wasm --validate`
  - `lumi run out/arith.wasm`

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
- No overloading: one name → one meaning (simplifies tooling and diagnostics).

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
- Build release: `cargo build -p lumi-cli --release` -> `target\release\lumi.exe`
- Install to PATH (optional): `cargo install --path crates/cli --bin lumi`

---

## Git Hooks (Pre-Commit)

- Enable hooks: `git config core.hooksPath .githooks`
- Hook runs: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`
- Skip toggles: `SKIP_PRECOMMIT=1`, `SKIP_FMT=1`, `SKIP_CLIPPY=1`, `SKIP_TESTS=1`

---

## lumi-tests Samples

- Location: `lumi-tests/`
- Quick parse: `cargo run -p lumi-cli -- parse lumi-tests/01_hello.lumi`
- Includes a negative case: `lumi-tests/06_trailing_call_comma.lumi`

---

## Vision

- Simple - approachable syntax.
- Safe - formal guarantees.
- Portable - runs anywhere via WebAssembly.
- Proof-oriented - prevent bugs before they run.

---

## License

Currently private, all rights reserved.
An open-source license (e.g., MIT or Apache-2.0) may be applied when Lumi is released publicly.
