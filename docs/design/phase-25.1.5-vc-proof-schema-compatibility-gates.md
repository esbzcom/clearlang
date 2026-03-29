# Phase 25.1.5 - VC/Proof Schema Versioning and Compatibility Gates

## Status
Design lock for `25.1.5` in `docs/TODO.md`.

## Goal
Lock compatibility behavior for VC and proof artifacts before solver-era status expansion lands.

## Locked Rules
1. VC schema:
   - current `version` remains required per VC object,
   - solver-era statuses (`unknown|timeout`) are additive values for `status`,
   - unknown non-additive major version is reject-by-default for strict tooling.
2. Proof artifact schema:
   - requires explicit `format` + `schema_version`,
   - unknown major schema version is fail-closed in verify/release gates.
3. Counterexample payload:
   - `diagnostics.counterexample.format` stays versioned (`clg.counterexample.v1`),
   - unknown counterexample format may be ignored for explain-only flows but must fail in strict release verification when claimed as required evidence.

## Compatibility Gate Matrix
1. Producer gates:
   - reject invalid/unknown status vocabulary in strict release mode,
   - reject malformed counterexample envelopes if status requires model details.
2. Consumer gates:
   - verify must reject unknown required schema versions,
   - verify must reject hash-bound artifacts that parse but violate version contract.
3. Replay gates:
   - status vector and schema metadata must be byte/hash stable for identical inputs.

## Non-Goals
1. This slice does not define proof compression or binary transport.
2. This slice does not define external theorem checker proof format support.

## References
- `docs/proofs/vc-schema.md`
- `docs/proofs/proof-artifact-schema.md`
- `docs/design/phase-25.1.3-emit-proof-schema-v1.md`
