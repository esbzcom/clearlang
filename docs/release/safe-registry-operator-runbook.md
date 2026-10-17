# Safe Registry Operator Runbook

Date: 2026-10-17

Status: Offline Phase 51 procedure only. It authorizes local review and
validation. It does not authorize key creation or access, Safe setup, RPC
contact, signing, transaction construction, broadcast, deployment, invocation,
or a production claim.

## Purpose

Use this runbook to prepare or abort one proposed production request under the
locked 2-of-3 Safe profile. The only eligible action descriptions are the
direct zero-value registry deployment and the Safe-executed zero-value registry
revision defined in Phase 51.1. This runbook does not make either action
eligible for target submission; the remaining Milestone 6 gates decide that.

## Required Roles

- Release owner: freezes the verified release bundle and verifier result.
- Policy approver: confirms the profile, environment, evidence, and stop
  decision.
- Safe owner custodians: independently review the same request digest. Two
  owners are required. They do not share keys, device access, recovery
  material, or approval channels.
- Deploy operator or executor: prepares only explicit local records. This role
  is separate from Safe ownership and has no authority to alter a record.
- Incident owner: declares a stop, preserves evidence, and coordinates the
  subsequent review.

No role may substitute an endpoint, account, nonce, fee, Safe configuration,
release digest, approval, or environment after the request has been reviewed.

## Offline Preparation

1. Start with a new path-safe request and separate policy-evidence record.
   Confirm the environment is production, the target profile is
   clg.evm-ethereum-mainnet-safe-registry.v1, and the chain ID is 1.
2. Place only non-secret, reviewed release, policy, approval, and—where
   applicable—signature-bundle records below the approved local artifact root.
   Do not place a key, seed phrase, wallet export, recovery material, endpoint
   credential, or absolute machine path there.
3. Run the offline validator:

   cargo run -p xtask -- safe-policy-validate --request REQUEST --evidence EVIDENCE --artifact-root ARTIFACT_ROOT --report REPORT

   It must produce an accepted report. A rejection is a stop, not a request to
   edit an existing approved record in place.
4. Record only the request digest, evidence digest, report digest, public
   identities, reviewer disposition, and expiry in the handoff package. Do
   not copy transaction payloads, raw signatures, or credentials into tickets,
   chat, logs, screenshots, or this repository.

## Two-Owner Handoff

1. The policy approver supplies each selected Safe owner the same request
   digest, release digest, policy digest, expiry, and explicit action summary.
2. Each owner independently compares those values against their own trusted
   display and records a non-secret approval reference. A second owner must
   not approve a modified digest or an unreviewed replacement.
3. The operator assembles only the two or more matching approval records and
   reruns the offline validator. Any different owner set, threshold, Safe
   identity, nonce, target, value, operation, calldata, or expiry stops the
   procedure.
4. The accepted local report is evidence of record conformance only. It is not
   proof of Safe signature validity, target state, endpoint honesty, finality,
   or authority to submit.

## Abort and Stop

Immediately stop local work, signing coordination, executor activity, and any
later submission preparation if a validator report rejects, an approval expires,
a role observes a mismatched digest, a Safe configuration changes, or any
credential/custody concern arises. Preserve the rejected report and the
non-secret input identities. Do not repair, rotate, replace, cancel, or resend
the request automatically.

Create a fresh request only after the policy approver records the reason,
affected environment, and required review. The immutable registry has no
on-chain pause, ownership transfer, recovery, or rollback capability in this
profile.

## Compromise Response

For suspected loss, duplication, compromise, or improper custody of a
production owner, deployer, executor, endpoint credential, or recovery process:

1. Stop all production work; do not use a break-glass key.
2. Preserve non-secret validator reports, digests, timestamps, role
   dispositions, and incident observations.
3. Notify the incident owner and policy approver through the approved
   out-of-band channel.
4. Quarantine the affected role. Do not use an owner key as a convenience
   executor and do not move the material to development or pilot/staging.
5. Require a new governance/security review. Safe owner rotation, threshold
   change, Safe replacement, or recovery is outside this profile and requires
   a new policy profile and fresh evidence.

## Evidence Retention

Retain the canonical request, policy evidence, accepted or rejected validator
report, release and policy digests, public Safe configuration identities, and
incident disposition in the approved evidence store. Retain no secret
material. A failure remains evidence; it is never deleted or relabeled as an
accepted action.

## Offline Drill

The local drill is complete only when the adversarial matrix has been reviewed,
the validator accepts both positive action shapes, each listed record-level
mutation is rejected with its stable class, and the compromise stop path can be
explained without accessing a wallet, endpoint, or target. The target-state
checks explicitly deferred by the matrix remain for later gates.

## References

- docs/design/phase-51.0-safe-operator-and-environment-policy.md
- docs/design/phase-51.1-safe-transaction-request-format.md
- docs/fixtures/phase-51.3/safe-operator-control-adversarial-matrix.json
- docs/todo/milestone_6_roadmap.md

