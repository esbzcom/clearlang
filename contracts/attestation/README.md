# Attestation Registry Artifacts

This folder contains the attestation registry contract and payload examples.

## Files

- `AttestationRegistry.sol`: registry contract with signer authz, key rotation controls, revocation, and schema versioning baseline.
- `sample-payload.json`: example attestation payload (wraps a `clg build --sign` payload).

## Workflow (Hardened Baseline)

1) Build proof bundle and sign:
   - `clg build my.clear -o out.wasm --emit-vcs out.vc.json --sign --key key.json --key-id dev --scope both --sig-out out.sig.json`
2) Wrap the signature file with chain metadata to produce an attestation payload (see `sample-payload.json`).
3) Canonicalize JSON (sorted keys) and compute SHA-256 as `payload_hash`.
4) Upload the payload to IPFS and obtain a `uri`.
5) Ensure signer and schema version are authorized by the registry owner:
   - `setSignerAuthorization(signer, true)`
   - `setSchemaVersion(schema_version, true)` (schema `1` is enabled by default)
6) Compute the attestation id:
   - `attestation_id = keccak256(abi.encode(registry, payload_hash, signer, schema_version))`
7) Register on-chain:
   - `AttestationRegistry.register(attestation_id, payload_hash, uri, schema_version)`
8) Revoke if needed:
   - `AttestationRegistry.revoke(attestation_id)` (allowed for signer or owner)

Notes:
- `sample-payload.json` uses placeholder hashes/signature values and is not a valid, verifiable payload.
- `sample-payload.json` now includes `schema_version` to match hardened registry ID binding.
- `attestation_id` is validated on-chain against `(registry, payload_hash, signer, schema_version)`.
- `register` rejects zero payload hashes, empty URIs, and URIs above `MAX_URI_BYTES` (512).
- Runtime and CI policy should treat revocation as a hard failure for release attestations.
- Additional production work (audits/fuzzing/ops controls) is tracked in Phase 18.

## Test

If you have Foundry installed, you can run contract tests:

```bash
cd contracts/attestation
forge test
```
