# Phase 16.7 - On-Chain Attestation (Reference Design)

This note defines the minimal on-chain attestation flow that ties ClearLang proof bundles to a chain registry. It is intentionally small and is *not* production-hardened.

Status note: this document records the Phase 16.7 reference model. Production-hardening behavior was added in Phase 18.1 (see `docs/design/phase-18.1-attestation-hardening.md`).

## Goals

- Anchor proof bundles on-chain with a stable identifier.
- Make verification deterministic: payload hash + registry entry + signature.
- Keep the reference implementation small and auditable.

## Non-Goals

- On-chain signature verification or authorization.
- Key rotation, revocation, or multi-sig governance.
- Data availability guarantees (pinning, backups, retention).

## Data Model

### Registry (on-chain)

The registry stores a single record per `attestation_id`:

- `attestation_id`: `bytes32`
- `signer`: `address` (transaction sender)
- `payload_hash`: `bytes32` (SHA-256 of canonical JSON payload)
- `uri`: `string` (e.g., `ipfs://...`)
- `timestamp`: `uint256` (block timestamp)

Reference contract: `contracts/attestation/AttestationRegistry.sol` (current code includes Phase 18.1 hardening controls beyond this reference note).

### Payload (off-chain)

The payload wraps the existing `clg build --sign` output in a chain-aware envelope:

```
{
  "version": "1",
  "chain_id": 1,
  "registry": "0x...",
  "signature": {
    "key_id": "...",
    "scope": "module|proofs|both",
    "signature_format": "ed25519",
    "signature": "<hex>",
    "payload": {
      "module_hash": "<hex>",
      "proofs_hash": "<hex>",
      "toolchain": "clg/<version>",
      "timestamp": "<rfc3339>",
      "scope": "module|proofs|both"
    }
  }
}
```

Canonical JSON (sorted keys) is hashed with SHA-256 to form `payload_hash`. The same canonical JSON is what the signature covers (via `clg build --sign`).

Sample payload: `contracts/attestation/sample-payload.json`.

## Attestation ID

Phase 16.7 reference formula:

```
keccak256(abi.encode(registry, payload_hash, signer))
```

This keeps the identifier chain- and signer-specific, while leaving the payload hash as the global content address.

Current hardened baseline (Phase 18.1):

```
keccak256(abi.encode(registry, payload_hash, signer, schema_version))
```

The current registry validates this binding on-chain.

## Verification Flow

1) Build and sign proof bundle:
   - `clg build --emit-vcs --sign ...`
2) Wrap the signature file in the attestation payload (above).
3) Canonicalize JSON and compute `payload_hash` (SHA-256).
4) Upload payload to IPFS and obtain a `uri`.
5) Register on-chain:
   - Phase 16.7 reference shape: `register(attestation_id, payload_hash, uri)`.
   - Current hardened shape: `register(attestation_id, payload_hash, uri, schema_version)`.
6) Verifier:
   - Fetch payload via `uri`.
   - Recompute `payload_hash` (canonical JSON + SHA-256).
   - Compare to on-chain registry entry.
   - Verify the signature in `signature` over the embedded payload.

## Notes

- This reference design is intended for Phase 16.7 only; production hardening is tracked in Phase 18.
- See `docs/design/phase-18.1-attestation-hardening.md` for the migration checklist.
