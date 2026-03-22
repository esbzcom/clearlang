# ClearLang Real-Case Starter Projects

This folder contains two runnable multi-file projects:

1. `generic/` - a non-crypto service-style project.
2. `crypto/` - a crypto-oriented project that uses `std::crypto`.

Both projects are designed for current CLI behavior:
- `clg run <entry.clear>` compiles and executes from source.
- entrypoint must be `main() -> Int`.

## Run the generic project

```powershell
clg run examples/projects/generic/main.clear
```

Expected result:
- prints `generic: checkout pipeline`
- returns `900`

## Run the crypto project

```powershell
clg run examples/projects/crypto/main.clear
```

Expected result:
- prints `crypto: auth pipeline`
- returns `1`

## Build CLI, create Wasm, and run

From repository root (`c:\clearlang\clearlang`):

```powershell
# 1) Compile CLI executable
cargo build -p clg-cli --release

# 2) Create Wasm from the generic example
.\target\release\clg.exe build examples\projects\generic\main.clear -o tmp\generic.wasm

# 3) Run using CLI
.\target\release\clg.exe run examples\projects\generic\main.clear
```

Use `examples\projects\crypto\main.clear` in steps 2 and 3 to build/run the crypto example.

## What these validate

- Multi-file module graphs with namespaced imports.
- `pure` + `io` boundary separation.
- Contract guards (`require`/`ensure`) in domain logic.
- `Option`/`Result` control flow in business decisions.
- Crypto runtime integration (`hash`/`hmac`) with constant-time compare (`eq_ct`).

## Production Path Review (milestone_2)

Current roadmap is split into Phases 20-24 in `docs/TODO.md`:

1. Phase 20: runnable namespace baseline + lock gates.
2. Phase 21: precompiled `std::core` pipeline.
3. Phase 22: package trust + dependency resolution (transitive + semver + lockfile).
4. Phase 23: runtime package loader/linker with fail-closed trust checks.
5. Phase 24: host profiles + go-live/release gates.

Immediate execution order:

1. Make `clearlang-tests/16_namespaced_call.clear` runnable.
2. Keep these two example projects green in CI as baseline app fixtures.
3. Add package-trust and runtime-loader integration tests on top of these fixtures.
