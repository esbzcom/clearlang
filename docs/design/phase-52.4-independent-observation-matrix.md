# Phase 52.4 - Independent Observation Matrix

Date: 2026-10-17

Status: Locked offline fixture policy. It authorizes no endpoint selection or
contact, signer, transaction construction, deployment, invocation, or mainnet
claim.

## Minimum Observation Set

Each target observation uses exactly two roles: a primary submission observer
and a separately operated independent verification observer. They must have
distinct observer identifiers and distinct disclosed execution-client families.
Client version remains recorded compatibility metadata; it is not a correctness
or consensus claim. The matrix deliberately does not name a live provider,
endpoint, or version. Those are explicit Phase 53 support-policy and operator
inputs.

Both observers must report the same chain ID, transaction/receipt, receipt
block and confirmations, runtime bytecode, schema-defined storage,
ValueChanged event, Safe configuration, finalized head, and reorganization
window identities. Equality is exact; no majority vote or compatibility
heuristic is permitted.

## Disagreement Rule

Different observer identifiers or client families alone do not establish an
Ethereum truth claim. They provide an independent observation surface. Any
disagreement for bytecode, receipt/log, storage, confirmation/finality, or a
later reorganization yields `unaccepted` / `M6C006`, preserves the secret-free
evidence, and stops. Automatic retry, resubmission, replacement, and a
production-acceptance claim are prohibited.

## Fixture

`docs/fixtures/phase-52.4/safe-registry-independent-observation-matrix.json`
is the canonical offline matrix. The local replay validator rejects either a
shared observer identifier or a shared client family with `M6C006`.
