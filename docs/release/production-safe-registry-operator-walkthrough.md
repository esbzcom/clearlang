# Production Safe Registry Operator Walkthrough

Status: Offline preparation and stop procedure. It does not authorize a
transaction, key use, Safe setup/change, endpoint contact, signing, broadcast,
deployment, invocation, or production claim.

## Inputs You May Supply

Supply only reviewed, non-secret, path-safe records under the approved artifact
root:

- candidate release bundle digest and independent verifier result;
- exact profile `clg.evm-ethereum-mainnet-safe-registry.v1`, chain ID `1`, and
  scalar registry action description;
- Safe public identity, version `1.4.1`, three public owner identities, and
  threshold `2-of-3`;
- request, policy-evidence, and approval-record digests, with expiry; and
- production support policy, independent-observation evidence, and soak
  decision records.

Never supply a private key, seed phrase, hardware-device recovery material,
raw signature, endpoint credential, wallet export, absolute machine path, or
account secret. Development, Sepolia/staging, and production records, Safe
identities, credentials, approvals, and nonces are never reused across
environments.

## Offline Checks and Handoff

1. Prepare the request and matching policy evidence according to
   [`safe-registry-operator-runbook.md`](safe-registry-operator-runbook.md).
2. Run the local policy check:

   ```powershell
   cargo run --offline --locked -p xtask -- safe-policy-validate --request <REQUEST> --evidence <EVIDENCE> --artifact-root <ARTIFACT_ROOT> --report <REPORT>
   ```

   A rejection is a stop. An accepted report is record-conformance evidence,
   not a Safe-signature, target-state, or submission authorization.
3. Give each selected Safe owner the same non-secret request digest, release
   digest, policy digest, expiry, and explicit action summary through separate
   approved channels. Each owner compares it independently. Two matching owner
   approvals are required; no owner shares a device, key, recovery material,
   or approval channel.
4. Capture and replay Safe evidence only with the offline tools and schemas in
   Phase 52. A mismatch, missing anchor, duplicate observer, stale evidence,
   or changed identity is a stop.
5. Run the candidate-evidence gate into a new output path:

   ```powershell
   cargo run --offline --locked -p xtask -- production-precheck --report <REPORT>
   ```

   At present it must write a `blocked` report with `M6D005` because the public
   testnet soak has not begun. Preserve that report; do not recast it as a
   passing or launch-ready result.

## Optional Network Boundary

The only possible network action described by Milestone 6 is a separately
authorized Phase 52.5 public-testnet soak, using staging-only credentials and
the distinct testnet Safe identity. It is not initiated by these commands or
this walkthrough. It requires its own explicit authorization, public staging
inputs, and stop conditions. No current document authorizes a mainnet action.

## Abort and Retention

Immediately stop if any digest, profile, chain ID, Safe owner/threshold,
nonce, action, calldata, value, operation, approval, expiry, verifier result,
observation, support-policy input, or soak decision differs from the reviewed
record. Stop also for any device/custody concern, endpoint discrepancy, or
unsupported tool/version. Do not edit an approved record, retry automatically,
rotate a Safe, use a break-glass key, or transfer material between environments.

Retain the canonical request, policy evidence, approval references, verifier
result, precheck report, observations, decision records, and non-secret SHA-256
digests for at least seven years. Retain rejected results as evidence. Store no
secret material in the repository, evidence package, ticket, chat, log, or
screenshot.
