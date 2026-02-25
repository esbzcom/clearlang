# ClearLang Release Notes - milestone_1

Date: 2026-02-25
Tag: `milestone_1`
Status: Release-ready

## Summary

This milestone closes the roadmap through Phase 19 (`docs/TODO.md`).
ClearLang now ships a usable high-assurance language/toolchain for its v1 scope:
- parser + typer + lowering + Wasm codegen/runtime,
- contracts/effects/refinements/resources/totality flows,
- proof artifacts and verification workflows with explicit trust boundaries.

## Included in milestone_1

1. Language and compiler core (Phases 0-17)
- Core syntax, typing, IR, and Wasm pipeline.
- Effects (`pure`/`mut`/`io`), contracts (`require`/`ensure`), loop invariants/variants.
- Option/Result, generics/interfaces/implementations, closures, resources, arrays/slices, imports/packages.
- Deterministic diagnostics with stable machine-readable JSON errors.

2. Production hardening and attestation (Phase 18)
- Attestation registry hardening and security review/fuzzing.
- Data-availability policy and operational drill runbooks.
- Proof-model assumption boundaries and strict proof defaults with CI gates.
- Published proof-coverage matrix (`proved` vs `assumed`).

3. High-assurance ergonomics (Phase 19)
- Assurance tiers (`L0`-`L3`) in artifacts and diagnostics.
- Strict fail-closed profile and profile regression CI guardrails.
- Verified std/core subset publication and fixture gating.
- VC repair hints, failure slicing/counterexample envelopes, AI proof-context bundle.
- Signed assurance manifest output on signed builds.
- `clg verify --explain` human-readable assurance summary.
- Release policy gate (`--assurance-manifest` + `--release-policy`) with deterministic `V005`.

## Verification and release gates

Current workspace validation before release:
- `cargo test -p clg-cli --tests` passed.
- `git status` clean.

Recommended pre-tag check:
```powershell
cargo test -p clg-cli --tests
```

## Known v1 boundaries (intentional)

- Some surfaces remain intentionally deferred/disallowed in v1 and are enforced with diagnostics:
  - `P011`: `export import` unsupported.
  - `P012`: lambda capture-list syntax unsupported.
  - `T245`: implementation method type parameters unsupported.
  - `T246`: interface type parameters unsupported.
  - `T806`: unsupported resource-collection forms.
  - `T110`: `U128`/`U256` operation coverage remains bounded.
- Strict mode (`--compiler-mode strict`) fails closed on deferred/assumed boundaries (`C033`).
- Project positioning remains bounded-assurance focused (not a universal theorem-prover replacement).

## Release recommendation

`milestone_1` is an appropriate tag for this state.
If you also want versioned semver discoverability, add a second tag (for example `v0.1.0`) to the same commit.
