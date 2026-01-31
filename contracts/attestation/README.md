# Attestation Reference Artifacts

This folder contains the Phase 16.7 minimal reference implementation artifacts.

## Files

- `AttestationRegistry.sol`: minimal on-chain registry contract.
- `sample-payload.json`: example attestation payload (wraps a `clg build --sign` payload).

## Workflow (Reference)

1) Build proof bundle and sign:
   - `clg build my.clear -o out.wasm --emit-vcs out.vc.json --sign --key key.json --key-id dev --scope both --sig-out out.sig.json`
2) Wrap the signature file with chain metadata to produce an attestation payload (see `sample-payload.json`).
3) Canonicalize JSON (sorted keys) and compute SHA-256 as `payload_hash`.
4) Upload the payload to IPFS and obtain a `uri`.
5) Register on-chain using `AttestationRegistry.register(attestation_id, payload_hash, uri)`.

This reference flow is intentionally minimal. Production hardening (authz, key rotation, revocation, and audits) is tracked in Phase 18.
