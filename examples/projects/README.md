# ClearLang Real-Case Starter Projects

This folder contains three multi-file projects:

1. `generic/` - a non-crypto service-style project.
2. `crypto/` - a crypto-oriented project that uses `std::crypto`.
3. `testing/` - a Gate D planning example for unit tests + deterministic mock sets.

`generic/` and `crypto/` are designed for current CLI behavior:
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

## Verify the testing example layout

`testing/` is intentionally a Phase `25.3` contract example. It documents the planned
unit-test and mock-set structure before `clg test` ships.

Current validation:

```powershell
clg run examples/projects/testing/main.clear
```

Planned invocation once Gate D lands:

```powershell
clg test examples/projects/testing --report json
```

See `examples/projects/testing/README.md` for the full layout and `tests/test-plan.json`
mock-binding example.

## Build CLI, create Wasm, and run

From repository root (`c:\clearlang\clearlang`):

```powershell
# 1) Compile CLI executable
cargo build -p clg-cli --release

# 2) Create Wasm from the generic example
.\target\release\clg.exe build examples\projects\generic\main.clear -o tmp\generic.wasm

# 3) Run the generated Wasm using CLI
.\target\release\clg.exe run tmp\generic.wasm
```

For debug iteration, you can also run source directly:

```powershell
.\target\release\clg.exe run examples\projects\generic\main.clear
```

Use `examples\projects\crypto\main.clear` in step 2 (build) and run `tmp\crypto.wasm` in step 3 for the crypto example.

## What these validate

- Multi-file module graphs with namespaced imports.
- `pure` + `io` boundary separation.
- Contract guards (`require`/`ensure`) in domain logic.
- `Option`/`Result` control flow in business decisions.
- Crypto runtime integration (`hash`/`hmac`) with constant-time compare (`eq_ct`).

## Quick Release

For a strict production-style release of an example project:

```powershell
# 1) Generate deterministic lock inputs
clg pkg lock --generate --compiler-mode strict --advisory-as-of 2026-03-26T00:00:00Z --root examples/projects/generic

# 2) Build in strict mode and emit VCs
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --compiler-mode strict --release-profile production --validate

# 3) Sign and verify before publish
clg build examples/projects/generic/main.clear -o out/generic.wasm --emit-vcs out/generic.vc.json --compiler-mode strict --release-profile production --validate --sign --key keys/signing.json --key-id release-2026q1 --scope both --sig-out out/generic.sig.json --assurance-manifest-out out/generic.assurance.json
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/trust-policy.json --assurance-manifest out/generic.assurance.json --explain
clg verify --module out/generic.wasm --sig out/generic.sig.json --pubkey keys/public.json --verify-mode compile-time --trust-policy examples/projects/generic/trust-policy.json --assurance-manifest out/generic.assurance.json --require-assurance proved_all
```

Full release flow details: `docs/release-process.md`.
