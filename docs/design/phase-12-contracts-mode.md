# Phase 12 Contracts Mode Flag

## Goal

Define `--contracts=runtime|hybrid|static` so projects can control how
`require`/`ensure` are enforced while the proof pipeline matures. The default
should preserve current behavior (runtime guards) and allow a gradual migration
toward proof-first workflows.

## Modes

### runtime (default)

- Behavior: emit runtime guards for `require`/`ensure` and trap on violations.
- Proofs: `--emit-vcs` is optional and does not affect codegen.
- Usage: best for local development and early adoption.

### hybrid (runtime + proofs)

- Behavior: keep runtime guards and also require proof artifacts.
- Proofs: `--emit-vcs` is required (or implied) so the build produces a VC JSON
  and the `clearlang.proof` custom section.
- Signing: `--sign` should be allowed and encouraged in this mode; `clg verify`
  continues to validate signatures and hashes.
- Usage: migration mode for teams rolling out proofs while preserving runtime
  safety.

### static (proofs only)

- Behavior: do not emit runtime guards; the contract obligations are enforced by
  external proof verification.
- Proofs: `--emit-vcs` is required and the build should fail if proofs are not
  produced.
- Runtime execution: `clg run` should warn or refuse to execute without an
  explicit override (for example `--allow-unsafe-runtime`) because runtime
  enforcement is intentionally disabled.
- Usage: production mode once proof verification is integrated in CI/CD.

## CLI Semantics

- `clg build`:
  - `--contracts=runtime`: current behavior; no extra requirements.
  - `--contracts=hybrid`: require VC emission and proof section generation.
  - `--contracts=static`: require VC emission and suppress runtime guards.
- `clg run`:
  - `runtime|hybrid`: run as today, using runtime guards.
  - `static`: refuse by default unless explicitly overridden.
- `--emit-vcs`:
  - Optional in `runtime`.
  - Required or implied in `hybrid` and `static`.

## Migration Path

1. Keep default `runtime` for existing users to avoid regressions.
2. Encourage `hybrid` in CI to produce proofs while maintaining runtime safety.
3. When proof verification is enforced in the pipeline, switch to `static` for
   production builds and disable runtime guards to reduce overhead.

## Open Questions

- Should `--contracts=hybrid` auto-select a default VC output path when none is
  provided, or should it fail with a clear error?
- Should `static` automatically require `--sign` (or a proof bundle) once
  signature verification is mandatory in CI?
