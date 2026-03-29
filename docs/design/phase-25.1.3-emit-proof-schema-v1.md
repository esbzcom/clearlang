# Phase 25.1.3 - `--emit-proof` Schema and Canonical Serialization (v1)

## Status
Design lock for `25.1.3` in `docs/TODO.md`.

## Goal
Define the first stable proof artifact contract so solver results can be produced, hashed, signed, and verified deterministically.

## Locked Decisions
1. Proof artifact format id: `clg.proof_artifact.v1`.
2. Top-level schema version: `schema_version = 1`.
3. Canonical serialization:
   - UTF-8 JSON,
   - deterministic key order from struct field order,
   - deterministic array ordering (`function`, then `vc_id`),
   - no nondeterministic timestamps inside proof evidence payload.
4. Required status vocabulary for per-VC entries:
   - `proved|failed|unknown|timeout|generated`.
5. `generated` is transitional and release-invalid; release closure requires `proved` for all release-enabled VCs.

## Compatibility Policy
1. Additive fields are allowed in v1 and must preserve existing keys/semantics.
2. Non-additive changes require `schema_version` bump.
3. Consumers must:
   - reject unknown major schema version,
   - ignore unknown additive fields for known version.

## Validation Contract
1. Artifact must include:
   - format id + schema version,
   - solver profile snapshot + hash,
   - per-VC outcomes and summary counters.
2. Artifact hash is computed from canonical JSON bytes.
3. Signed payload/manifest binding must reference this hash for verify-time consistency checks.

## References
- `docs/proofs/proof-artifact-schema.md`
- `docs/design/phase-25.1.5-vc-proof-schema-compatibility-gates.md`
- `docs/TODO.md` (`25.1.10` implementation task)
