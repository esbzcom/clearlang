# Phase 18.3 - Attestation Security Review and Fuzzing

## Status
Baseline security review completed for the attestation registry and payload envelope validation path.

Implemented artifacts:
- Contract hardening coverage:
  - `contracts/attestation/AttestationRegistry.sol`
  - `contracts/attestation/test/AttestationRegistry.t.sol`
  - `contracts/attestation/test/AttestationRegistryFuzz.t.sol`
- Payload validation coverage:
  - `crates/cli/src/signing.rs` (`validate_attestation_payload_value`)
  - unit tests in `crates/cli/src/signing.rs`

## Threat Model (Phase 18.3 scope)
- Unauthorized publication of attestations.
- ID spoofing (caller supplies arbitrary `attestation_id`).
- Replay/abuse across schema versions.
- Ambiguous or malformed off-chain payload envelopes.
- Operational failure modes from malformed inputs (empty/oversized URIs, zero hashes).

Out of scope for this slice:
- On-chain signature verification of attestation payloads.
- Chain availability guarantees and pinning SLAs (Phase 18.2).
- External dependency compromise and infra key custody procedures.

## Review Findings and Actions

### Resolved in this slice
1. Enforce strict ingress validation on contract registration:
   - reject unauthorized signers,
   - reject unsupported schema versions,
   - reject mismatched canonical IDs,
   - reject zero payload hashes,
   - reject empty/oversized URIs.

2. Strengthen ownership transition:
   - newly transferred owner is auto-authorized as signer.

3. Add revocation safety checks:
   - signer-or-owner only,
   - one-way revocation.

4. Add payload envelope validator (CLI-side):
   - strict shape checks for top-level fields,
   - schema version required and bounded,
   - registry address format check,
   - signature format/hex length checks,
   - payload hash hex-length checks,
   - signature scope must match payload scope.

### Residual risks (tracked)
1. No on-chain cryptographic verification of payload signatures.
2. No chain fetch/verification integration in `clg verify` yet (off-chain workflow responsibility).
3. DA/retention policy and disaster-recovery drills still pending (`18.2`).

## Fuzzing Coverage

### Contract fuzz/property tests
- Deterministic attestation ID derivation.
- Unauthorized signer rejection.
- Mismatched ID rejection.
- Schema toggle behavior.
- Revoke authorization checks.

File: `contracts/attestation/test/AttestationRegistryFuzz.t.sol`

### Payload validation negative coverage
- Missing required fields (`schema_version`).
- Invalid registry address shape.
- Signature/payload scope mismatch.
- Invalid hex-length for signature and hash fields.

File: `crates/cli/src/signing.rs` unit tests.

## Recommended Execution Commands
- Contract tests (Foundry):
  - `cd contracts/attestation && forge test`
- CLI unit tests:
  - `cargo test -p clg-cli signing::tests`

## References
- `docs/TODO.md` (`18.3`)
- `docs/design/phase-18.1-attestation-hardening.md`
- `docs/rollout/attestation-production.md`
