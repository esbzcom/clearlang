# Phase 30.1.1.4.2.2 - Target Receipt Identity Binding Lock

Date: 2026-08-20
Status: Locked
Owner: language-and-proof-owner / target-runtime-owner

## Policy

`clg target deploy`, `clg target call`, and `clg target invoke` require explicit contract-state
schema, proof artifact, and signed-bundle files in addition to the target artifact and wire ABI. Before any
chain-ID request or transaction submission, ClearLang rejects any mismatch in compiler identity,
loaded-source graph, contract identity, or target/wire-ABI digest. For stateful target artifacts,
the embedded state-schema digest must equal the supplied schema digest.

The signed bundle must carry the canonical proof-artifact hash and the same compiler and
loaded-source identities. A target receipt records those identities plus schema, proof-artifact,
and signed-bundle digests. Invalid evidence, cross-artifact drift, or a failed transaction writes
no successful receipt.

## Boundary

This gate validates identity binding, not signer trust policy: independent signature verification
and trust-anchor decisions remain release-verification responsibilities.
