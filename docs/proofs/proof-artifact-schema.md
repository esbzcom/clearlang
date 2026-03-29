# Proof Artifact Schema (`clg.proof_artifact.v1`)

## Purpose
`--emit-proof` writes deterministic solver-era proof outcomes that can be hashed, signed, replayed, and verified independently from Wasm bytes.

## Top-Level Object (v1)
- `format`: string (`clg.proof_artifact.v1`)
- `schema_version`: number (`1`)
- `generated_by`: string (toolchain id, for example `clg-cli/0.1.0`)
- `compiler_mode`: string (`permissive|standard|strict`)
- `solver_profile`: object
  - `solver_family`: string (for example `z3`)
  - `solver_version`: string
  - `options`: array of `{ key, value }`
  - `timeouts_ms`: object (`per_vc`, optional `total`)
- `solver_profile_hash`: string (`sha256:<hex>`)
- `summary`: object
  - `total_vcs`: number
  - `proved_count`: number
  - `failed_count`: number
  - `unknown_count`: number
  - `timeout_count`: number
  - `generated_count`: number
  - `assumed_vcs`: number
- `vcs`: array ordered by `function`, then `vc_id`

## VC Entry (v1)
- `function`: string
- `vc_id`: string
- `status`: string (`proved|failed|unknown|timeout|generated`)
- `assumption_boundaries`: array of boundary ids
- `vc_hash`: string (`sha256:<hex>` over canonical VC obligation payload)
- `counterexample`: optional object
  - `state`: string (for example `solver_unavailable`, `model_attached`)
  - `format`: string (`clg.counterexample.v1`)
  - `reason`: string
  - `bindings`: array of `{ symbol, value }`

## Canonical Serialization Rules
1. UTF-8 JSON output.
2. Stable key ordering from producer struct fields.
3. Stable array ordering for `vcs` and `assumption_boundaries` (lex asc).
4. No wall-clock timestamps inside proof artifact payload.

## Compatibility Policy
1. Additive fields allowed in `schema_version=1`.
2. Non-additive changes require version bump.
3. Consumers must reject unknown major `schema_version`.

## CLI Contract
1. Build emission:
   - `clg build file.clear --emit-vcs out.vc.json --emit-proof out.proof.json`
2. Verify consistency path:
   - `clg verify --module out.wasm --sig out.sig.json --pubkey pub.json --proof-artifact out.proof.json`
3. When signed payload includes proof-artifact claims, missing `--proof-artifact` is a fail-closed verify error (`V006`).

## Release Policy Notes
1. `generated` is transitional evidence, not release-grade.
2. Release closure requires `total_vcs == proved_count`, with zero `failed|unknown|timeout|generated`.
3. Artifact hash and `solver_profile_hash` are expected to be bound into signed claims for verify-time consistency checks.
