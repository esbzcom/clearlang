# Milestone 6 - Mainnet Readiness for One Registry Profile (50-54)

## Goal

Decide, with reproducible evidence, whether the existing proof-carrying,
caller-authorized configuration-registry profile may be operated as a limited,
low-value production profile on one explicitly named EVM mainnet. The milestone
does not broaden ClearLang's language, EVM, custody, or financial-product claim.

The only completion claim is:

> Production-ready for the named configuration-registry profile on the named
> chain and under the published operator policy only.

That claim is unavailable until every local, operational, deployment, and
independent-assurance gate below succeeds. A passing gate never authorizes a
different contract, chain, signer policy, or value-bearing use case.

## Inherited Product Boundary

Milestone 6 inherits the Milestone 5 source and target surface without widening
it:

- one fixed-owner, scalar configuration registry with caller authorization;
- the existing fail-closed source, proof, release-bundle, and independent
  verifier contracts;
- direct deployment and explicit target inputs only; and
- local deterministic evidence as the primary acceptance authority.

The selected mainnet, signer/governance model, finality rule, and launch limit
are deliberately **not** selected by this roadmap. They require the Phase 50
design lock and explicit accountable owners. No implementation task may infer
them from a testnet setting or an operator machine.

Milestone 6 is a profile-promotion milestone, not a feature-development
milestone. It may add the policy, evidence, and operational tooling needed to
operate this one profile, but it may not add ClearLang syntax, types, effects,
proof assumptions, IR operations, target semantics, or executable extension
hooks. The future Profile Pack architecture is explicitly out of scope; it can
package this profile only after the Milestone 6 closure record exists.

## Design-Principle Guardrails

| Principle | Milestone 6 decision rule |
| --- | --- |
| Simple for users | One published production-precheck and one small operator runbook replace informal deployment knowledge; every target and signer input remains explicit. |
| AI-friendly | Versioned schemas, canonical reports, stable diagnostics, and table-driven stop conditions make every decision inspectable and reproducible. |
| Provably correct | The existing proof boundary remains fail-closed; production promotion additionally verifies the exact deployed bytecode, constructor inputs, release bundle, and receipt identities. |
| Crypto-focused | Spend effort on key authority, chain/finality behavior, artifact substitution, incident response, and independent review before adding convenience features. |

## Explicit Non-Goals

- Generic EVM, multi-chain, wallet-discovery, remote-signing, automatic
  nonce/fee selection, or hosted-service support.
- Tokens, custody, DeFi, value transfer, proxies, delegate calls, external
  calls/callbacks, dynamic ABI values, collections, `U256`, mutable branches,
  loops, or broader cryptography.
- An assertion that local evidence proves RPC honesty, chain consensus,
  finality, client correctness, key custody, or security of any unmodeled
  component.
- A mainnet transaction before the explicit launch-decision gate approves it.

## Entry Condition

Milestone 6 may plan and implement local-only controls, but it may not perform
any new target action until the unresolved Milestone 5 closure condition is
satisfied: a clean checkout at the committed revision produces a full
`clg.pilot-precheck-report.v2` report, and the retained report is verified
under the current compatibility lock. If the reproduction differs from the
locked contract, stop and repair or supersede the Milestone 5 evidence through
an explicit compatibility decision before continuing.

## Acceptance Rule

Every completed implementation task adds deterministic local tests before it is
checked off. The milestone can close only when a clean environment demonstrates:

1. the entry condition and all production-precheck gates pass in canonical
   order;
2. a named-chain deployment candidate is bound to its source, proof, bundle,
   signer policy, constructor inputs, bytecode, address, and receipt evidence;
3. the operator and incident controls work in positive and adversarial
   simulations, with no secret material retained in reports;
4. the evidence is reproduced independently in a second clean environment;
5. the required independent review reports no unaccepted blocker; and
6. a launch decision records the exact approved profile, value/exposure cap,
   stop conditions, and non-claims.

There are no automatic retries and no exception path from a failed gate to a
production claim. A testnet rehearsal, CI result, audit, or successful live
transaction is additional evidence; none substitutes for the local gate.

## Gate Contracts

The gate label determines both its required deliverable and its maximum
authority. A task cannot obtain greater authority because a later gate exists.

| Gate | Required exit artifact | Maximum target authority | Permits the next gate when |
| --- | --- | --- | --- |
| A: 50 | Published product/profile lock, v2 prerequisite record, named-mainnet profile, and claim terminology fixtures | No target action | The inherited profile and the one eligible mainnet are unambiguous, and v2 evidence is accepted without a waiver. |
| B: 51 | Approved operator-policy lock, canonical transaction-request/policy records, validator, fixture matrix, and operator runbook | Local simulation only | Every request, approval, secret-handling, and abort condition is validated fail closed. |
| C: 52 | Correspondence/evidence lock, schemas, capture/replay tooling, tamper matrix, and public-testnet soak decision | Explicit public-testnet actions only | All required observations replay offline and any disagreement has a documented stop result. |
| D: 53 | Versioned production-precheck, support policy, canonical candidate report, and independently executable documentation | No additional target action | A clean local environment passes the complete candidate evidence chain without network-dependent verification. |
| E: 54 | Independent-review disposition, second-environment reproduction, pre-launch decision, final launch evidence, and closure record | One approved limited mainnet action, and only in 54.4 | The decision is `approved`; an unapproved or incomplete decision authorizes no mainnet submission. |

The `production-precheck` is a candidate-evidence gate. Independent review and
the launch decision are deliberately separate Gate E inputs; Gate D must not
pretend they already exist.

## Dependency-Ordered Execution Plan

Execute unfinished work only in this order. A gate may produce local fixtures
and documentation before its successor begins; it may not authorize a target
action until every prerequisite explicitly says so.

1. [x] 50.0 product, claim, actor, and pilot-integrity lock.
2. [x] 50.1 clean v2 pilot-evidence reproduction and compatibility decision.
3. [x] 50.2 Safe-owner mainnet profile amendment and terminology-drift lock.
4. [ ] 51.0-51.1 operator authority, transaction-request, and secret-retention
   locks.
5. [ ] 51.2-51.3 fail-closed policy implementation and adversarial fixture
   matrix.
6. [ ] 52.0 deployment-correspondence and observation/reorganization lock.
7. [ ] 52.1-52.3 canonical evidence schema, capture, and offline replay.
8. [ ] 52.4 differential-observation matrix and 52.5 public-testnet soak.
9. [ ] 53.0-53.3 production-precheck/report/support-policy lock and
   implementation.
10. [ ] 53.4 independent-directory walkthrough and documentation fixtures.
11. [ ] 54.0 independent-review commissioning, then 54.1 findings
    disposition and 54.2 independent clean-environment reproduction.
12. [ ] 54.3 launch decision, then 54.4 limited launch and 54.5 final
    production-profile record. A negative launch decision stops at 54.3 and is
    still a complete, valid decision record, but cannot close the milestone as
    production-ready.

### 50. Production Profile and Pilot-Integrity Gate [Planning Gate A]

**Gate exit:** the profile, actors, evidence mapping, current pilot evidence,
and one eligible mainnet are locked. Gate A creates no signer authority and
submits no transaction.

- [x] 50.0 Publish the Milestone 6 product and threat-model lock before
  selecting a chain, signing policy, or target behavior.
  (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
  `Completed: 2026-10-17`.
  - [x] 50.0.1 Inventory the exact inherited source, proof, verifier, target,
    diagnostics, schemas, release bundle, and known exclusions. Record stable
    version/digest inputs and reject an ambiguous or locally edited baseline.
    (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
    `Completed: 2026-10-17`.
  - [x] 50.0.2 Define the release owner, policy approver, deploy operator,
    signer custodian, independent verifier, independent reviewer, and incident
    owner. For each, list trusted inputs, authority limits, and required
    handoffs.
    (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
    `Completed: 2026-10-17`.
  - [x] 50.0.3 Define protected assets, attacker capabilities, failure
    responses, evidence-retention requirements, and explicit non-claims for
    source proof, verifier acceptance, RPC observation, finality, and key
    custody.
    (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
    `Completed: 2026-10-17`.
  - [x] 50.0.4 Define the only eligible production claim, permitted low-value
    registry use, maximum exposure, stop conditions, and exclusions. State that
    neither generic EVM support nor custody safety is claimed.
    (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
    `Completed: 2026-10-17`.
  - [x] 50.0.5 Add an acceptance-to-evidence matrix mapping every planned
    completion statement to a canonical report, fixture, review record, or
    decision record.
    (`docs/design/phase-50.0-milestone-6-production-profile-lock.md`)
    `Completed: 2026-10-17`.

- [x] 50.1 Resolve the Milestone 5 v2 evidence prerequisite before any new
  target action. (`docs/evidence/phase-50.1-clean-v2-reproduction.md`)
  `Completed: 2026-10-17`.
  - [x] 50.1.1 Run the full `xtask pilot-precheck` from a fresh clean checkout
    of the committed revision and retain its path-safe v2 report plus digest
    projection.
    (`docs/evidence/phase-50.1/`) `Completed: 2026-10-17`.
  - [x] 50.1.2 Verify the report using the current compatibility and report
    contracts; record the exact toolchain, solver, verifier, lockfile, and
    profile identities used for the run.
    (`docs/evidence/phase-50.1-clean-v2-reproduction.md`)
    `Completed: 2026-10-17`.
  - [x] 50.1.3 Compare the new v2 result with the historical v1 evidence only
    as a compatibility audit. It must not relabel, modify, or reinterpret the
    historical evidence as v2 acceptance.
    (`docs/evidence/phase-50.1-clean-v2-reproduction.md`)
    `Completed: 2026-10-17`.
  - [x] 50.1.4 If the v2 run fails or differs from its locked contract, publish
    a blocker or an explicit compatibility/rollback decision. Do not continue
    to mainnet-profile work by treating the difference as waived.
    The qualifying v2 run passed its locked projection, so the failure branch
    was not taken. (`docs/evidence/phase-50.1-clean-v2-reproduction.md`)
    `Completed: 2026-10-17`.

- [x] 50.2 Publish the named-mainnet profile and claim-terminology lock after
  50.1 passes. Ethereum Mainnet remains the one-chain target, but the former
  direct-owner profile is superseded before target action by the 2-of-3 Safe
  owner amendment:
  [`docs/design/phase-50.2-safe-owner-amendment.md`](../design/phase-50.2-safe-owner-amendment.md).
  Gate A remains local-only and authorizes no target action.
  `Completed: 2026-10-17`.
  - [x] 50.2.1 Select exactly one chain: **Ethereum Mainnet**, chain ID `1`.
    The target-policy lock fixes the transaction type, required RPC methods,
    endpoint authentication boundary, and client/version observation policy.
    An unspecified or multiple-chain profile is invalid.
    `Completed: 2026-10-17`.
  - [x] 50.2.2 Revalidate confirmation depth, finality condition, observation
    interval, reorganization detection window, and the required response to a
    conflicting, missing, replaced, reverted, or Safe-execution transaction.
    (`docs/design/phase-50.2-safe-owner-amendment.md`)
    `Completed: 2026-10-17`.
  - [x] 50.2.3 Revalidate the allowed deployment and invocation envelope:
    exact
    registry source/profile, constructor inputs, caller roles, gas/fee/nonce
    policy, value prohibition, Safe owner/threshold identity, and exposure cap.
    Every value and Safe configuration input must be supplied or approved
    explicitly. (`docs/design/phase-50.2-safe-owner-amendment.md`)
    `Completed: 2026-10-17`.
  - [x] 50.2.4 Revise terminology and documentation-consistency fixtures to
    reject unqualified `production`, `mainnet`, `verified`, and `proved`
    claims, distinguish Safe threshold evidence from source/release evidence,
    and ensure all approved claims point to their evidence layer.
    (`docs/fixtures/phase-50.2/ethereum-mainnet-claim-terminology-fixtures.json`,
    `xtask/src/main/tests.rs`) `Completed: 2026-10-17`.
  - [x] 50.2.5 Record the 2-of-3 Safe owner decision and explicitly supersede
    the direct-owner profile before target action.
    (`docs/design/phase-50.2-safe-owner-amendment.md`)
    `Completed: 2026-10-17`.

### 51. Operator Authority and Deployment Controls [Security Gate B]

**Gate exit:** transaction-policy validation and its adversarial matrix run
locally against canonical records. Gate B may simulate actions but must not
contact a target.

- [x] 51.0 Publish the Safe-aware operator-authority, signer-policy, and
  environment-isolation lock.
  - [x] 51.0.1 Specify the approved signer/governance model, required
    approval threshold, least-privilege roles, separation of duties, and the
    non-secret proof that an action had the required approval.
  - [x] 51.0.2 Specify provisioning, rotation, revocation, compromise response,
    and break-glass policy. No private key format, seed, endpoint credential,
    or recovery secret may become a ClearLang artifact.
  - [x] 51.0.3 Specify the operational stop, pause, and immutable-contract
    remediation policy. Where the profile has no on-chain pause capability,
    document the actual off-chain stop action rather than implying one exists.
  - [x] 51.0.4 Define separate development, pilot/staging, and production
    security domains. Lock non-reuse of keys, recovery material, Safe accounts,
    endpoint credentials, request identifiers, nonce records, and approvals;
    require explicit environment/profile binding.
    (`docs/design/phase-51.0-safe-operator-and-environment-policy.md`)
    `Completed: 2026-10-17`.

- [x] 51.1 Lock the canonical transaction-request and policy-evidence format.
  - [x] 51.1.1 Define versioned, path-safe request records for the single direct
    deployment and the distinct Safe-executed registry revision. Bind every
    request to one named environment, chain/profile, release bundle, signer
    policy, approval reference, nonce, gas limit, fee fields, expiry, and
    zero-value requirement; no record may encode a generic arbitrary call.
  - [x] 51.1.2 Define exact rejection diagnostics for missing, repeated,
    substituted, stale, cross-chain, over-cap, nonzero-value, or
    policy-incompatible inputs, including wrong environment, Safe identity,
    owner set, threshold, Safe nonce, inner target, `CALL` operation, or ABI
    calldata.
  - [x] 51.1.3 Define redaction and retention rules for request, approval, log,
    and evidence artifacts. Include secret-scanning fixtures for private-key
    markers, endpoint credentials, absolute paths, environment leakage, and
    cross-environment approval reuse.
    (`docs/design/phase-51.1-safe-transaction-request-format.md`,
    `docs/fixtures/phase-51.1/safe-transaction-request-fixtures.json`)
    `Completed: 2026-10-17`.

- [x] 51.2 Implement fail-closed offline transaction-policy validation using the
  locked records.
  - [x] 51.2.1 Add strict CLI argument and request-schema validation; reject
    implicit environment, network, account, Safe configuration, nonce, fee,
    value, and signer discovery.
  - [x] 51.2.2 Bind accepted requests and approval evidence to the release
    bundle, registry profile, and environment before an adapter may submit a
    transaction. Enforce the Safe-specific deployment and invocation shapes.
  - [x] 51.2.3 Emit deterministic, secret-free acceptance/rejection reports
    whose digest can be included in later correspondence evidence.
    (`xtask safe-policy-validate`; `xtask/src/main/core/safe_transaction_policy.rs`;
    `xtask/src/main/tests.rs`)
    `Completed: 2026-10-17`.

- [x] 51.3 Add the operator-control adversarial matrix and offline runbook.
  - [x] 51.3.1 Add positive fixtures for the separately approved direct
    deployment and Safe-executed zero-value registry revision only.
  - [x] 51.3.2 Add negative fixtures for wrong chain, wrong signer, inadequate
    approval, expired request, internally inconsistent nonce, closed fee
    envelope, nonzero value, and explicitly classify live nonce-collision or
    dynamic-market-fee questions as later target-observation work;
    reject wrong Safe/owner set/threshold, non-`CALL` operation,
    arbitrary calldata, policy drift, cross-environment reuse, key rotation,
    unsafe paths, and secret leakage.
  - [x] 51.3.3 Publish a short operator runbook covering preparation, two-party
    handoff where required, abort, evidence preservation, and compromise
    response. Documentation tests must exercise its local/offline steps.
    (`docs/fixtures/phase-51.3/safe-operator-control-adversarial-matrix.json`,
    `docs/release/safe-registry-operator-runbook.md`, `xtask/src/main/tests.rs`)
    `Completed: 2026-10-17`.

### 52. Mainnet Correspondence and Operational Evidence [Target Gate C]

**Gate exit:** target observations are captureable, replayable, and
tamper-resistant; the public-testnet soak has an explicit eligibility decision.
Only the explicit testnet exercises in 52.5 may contact a target.

- [x] 52.0 Publish the deployment-correspondence and target-observation lock.
  - [x] 52.0.1 Define the canonical identity chain from source/profile, proof,
    state schema, ABI, EVM creation/runtime bytecode, constructor inputs,
    release bundle, approved request, transaction hash, contract address,
    receipt, and storage observations.
  - [x] 52.0.2 Define address-derivation and constructor-input checks, runtime
    bytecode comparison, init/state-schema checks, and the exact storage/event
    observations required before a deployment may be accepted.
  - [x] 52.0.3 Define an independent-client/differential observation policy and
    response to disagreement. One RPC response is evidence of observation,
    never proof of chain or client correctness.
  - [x] 52.0.4 Define finality/reorganization evidence and the deterministic
    status for missing, pending, reverted, replaced, conflicting, or late
    receipts.
    (`docs/design/phase-52.0-safe-registry-correspondence-lock.md`,
    `docs/fixtures/phase-52.0/safe-target-observation-lock-fixtures.json`,
    `xtask/src/main/tests.rs`) `Completed: 2026-10-17`.

- [x] 52.1 Define the versioned production evidence and offline replay schemas.
  - [x] 52.1.1 Specify canonical, secret-free evidence records for deployment,
    authorized invocation, expected unauthorized invocation, confirmation, and
    reorganization observation.
  - [x] 52.1.2 Bind records to all identities from 52.0, including observer
    client identity and sanitized request/response digests, without trusting an
    ambient checkout, endpoint, or wallet.
  - [x] 52.1.3 Define stable replay result statuses and diagnostics; unknown
    record/schema/profile/client-policy versions must fail closed.
    (`docs/design/phase-52.1-safe-registry-evidence-replay-schemas.md`,
    `docs/fixtures/phase-52.1/safe-registry-evidence-schema-fixtures.json`,
    `xtask/src/main/tests.rs`) `Completed: 2026-10-17`.

- [x] 52.2 Implement deterministic local evidence capture and offline replay.
  - [x] 52.2.1 Capture only the locked response fields, normalize ordering and
    paths, redact secrets, and write evidence atomically.
  - [x] 52.2.2 Replay recorded responses without network access and compare
    deploy/invoke/authorization/finality observations to the locked identity
    chain.
  - [x] 52.2.3 Emit a canonical replay report suitable for the later
    production-precheck; distinguish malformed evidence from an observed
    target mismatch.
    (`xtask safe-evidence-capture`, `xtask safe-evidence-replay`,
    `xtask/src/main/core/safe_registry_evidence.rs`, `xtask/src/main/tests.rs`)
    `Completed: 2026-10-17`.

- [x] 52.3 Add correspondence and replay tamper fixtures.
  - [x] 52.3.1 Add the separate offline, secret-free trusted identity anchor
    needed to reject a different-but-valid source, bundle, approval,
    constructor, creation/runtime bytecode, schema, or Safe identity digest;
    an unsigned self-contained evidence file cannot establish that comparison.
  - [x] 52.3.2 Cover omitted, duplicated, reordered, path-traversal,
    cross-chain, stale, secret-bearing, unknown-version, and malformed
    evidence records with the local capture/replay corpus.
  - [x] 52.3.3 Cover pending/reverted/replaced/conflicting receipt and
    reorganization scenarios, asserting no scenario is emitted as accepted
    final deployment evidence.
    (`docs/fixtures/phase-52.3/safe-registry-evidence-replay-tamper-matrix.json`,
    `xtask/src/main/core/safe_registry_evidence.rs`, `xtask/src/main/tests.rs`)
    `Completed: 2026-10-17`.

- [x] 52.4 Build the independent-observation fixture matrix.
  - [x] 52.4.1 Select the minimum independent clients or recorded compatible
    client observations required by the 50.2 lock; document why each is in
    scope and what it cannot establish.
  - [x] 52.4.2 Add deterministic agreement and disagreement fixtures for
    bytecode, receipt, log, storage, confirmation, and finality observations.
  - [x] 52.4.3 Require disagreement to produce a preservation-and-stop result,
    not a majority vote or automatic retry.
    (`docs/design/phase-52.4-independent-observation-matrix.md`,
    `docs/fixtures/phase-52.4/safe-registry-independent-observation-matrix.json`,
    `xtask/src/main/core/safe_registry_evidence.rs`, `xtask/src/main/tests.rs`)
    `Completed: 2026-10-17`.

- [ ] 52.5 Run the public-testnet soak using the unchanged registry profile
  and production-shaped controls.
  - [x] 52.5.0 Publish the no-network soak decision package, required external
    inputs, exercises, evidence-retention contract, and non-claim boundary.
    (`docs/design/phase-52.5-public-testnet-soak-decision-package.md`,
    `docs/fixtures/phase-52.5/safe-registry-public-testnet-soak-decision-fixture.json`)
    `Completed: 2026-10-17`.
  - [ ] 52.5.1 Execute approved deploy, authorized update, expected
    unauthorized update, wrong-chain prevention, and interrupted-operation
    exercises with explicit inputs.
  - [ ] 52.5.2 Retain replayable, secret-free evidence and independently replay
    every exercise locally.
  - [ ] 52.5.3 Publish a soak decision recording duration, actions, anomalies,
    unresolved risks, and whether the profile is eligible for Gate D. This is
    not a production claim.

### 53. Reproducible Production Assurance [Release Gate D]

**Gate exit:** `production-precheck` emits and self-validates a canonical
candidate-evidence report from a clean environment. Gate D validates a launch
candidate; it does not approve or submit a mainnet transaction.

- [x] 53.0 Lock `xtask production-precheck` and its report contract.
  - [x] 53.0.1 Define the CLI, fixed gate order, fail-fast behavior, report
    location, exit codes, redaction rules, and rule that no gate implicitly
    contacts a network.
  - [x] 53.0.2 Define the versioned canonical report schema and bind it to the
    exact 50.2 profile lock, 51 policy, release bundle, correspondence evidence,
    replay result, and support matrix. The independent-review disposition and
    launch decision are Gate E inputs and must be represented as `not_run` or
    absent from the candidate report rather than inferred.
  - [x] 53.0.3 Define the required status vocabulary (`passed`, `failed`,
    `blocked`, `not_run`) and diagnostics for invalid reports, unknown
    sub-gates, reordered evidence, and unsupported versions.
    (`docs/design/phase-53.0-production-precheck-report-lock.md`,
    `docs/fixtures/phase-53.0/production-precheck-report-fixture.json`,
    `xtask/src/main/tests.rs`) `Completed: 2026-10-17`.

- [x] 53.1 Publish the production support, compatibility, and response policy.
  - [x] 53.1.1 Define the supported compiler, solver, verifier, operating
    system/architecture, chain profile, transaction format, and client
    observation matrix.
  - [x] 53.1.2 Define upgrade, deprecation, vulnerability/CVE, emergency
    rollback, and evidence-compatibility behavior. Unknown or expired versions
    must reject rather than inherit an earlier approval.
  - [x] 53.1.3 Define the minimal public artifacts and long-term retention
    period for a production-profile verification record.
    (`docs/design/phase-53.1-production-support-compatibility-policy.md`,
    `docs/fixtures/phase-53.1/production-support-compatibility-fixture.json`,
    `xtask/src/main/tests.rs`) `Completed: 2026-10-17`.

- [ ] 53.2 Implement `production-precheck` as a strict composition of locked
  local gates.
  - [ ] 53.2.1 Invoke the existing release/pilot checks plus operator-policy,
    correspondence/replay, compatibility, and documentation checks in the
    locked order. It must not require a Gate E review disposition in order to
    establish candidate evidence.
  - [ ] 53.2.2 Write the report atomically with canonical ordering, input
    digests, tool identities, redacted paths, and later-gate `not_run` results
    after a failure.
  - [ ] 53.2.3 Verify its own report before returning success; a report that
    cannot be parsed, hashed, or independently validated is a failed gate.

- [ ] 53.3 Add the production-precheck validation and tamper matrix.
  - [ ] 53.3.1 Test CLI parsing, fixed ordering, fail-fast behavior, repeat-run
    canonicality, report self-validation, and no-network operation.
  - [ ] 53.3.2 Test missing, unknown, substituted, reordered, duplicated,
    path-unsafe, secret-bearing, stale, cross-profile, and unsupported-version
    inputs and reports.
  - [ ] 53.3.3 Test every inherited and new Gate B/C failure maps to a stable
    report result and blocks any production acceptance statement.

- [x] 53.4 Publish and test independent-user documentation.
  (`docs/design/phase-53.4-independent-user-documentation-lock.md`,
  `docs/release/production-verifier-offline-walkthrough.md`,
  `docs/release/production-safe-registry-operator-walkthrough.md`,
  `docs/fixtures/phase-53.4/`) `Completed: 2026-10-18`.
  - [x] 53.4.1 Write a verifier walkthrough that starts from the distributed
    verifier, public key/trust policy, and release/evidence package, with no
    source checkout, private key, or network requirement for verification.
  - [x] 53.4.2 Write an operator walkthrough that identifies every supplied
    value, approval handoff, optional network action, abort condition, and
    evidence-retention step.
  - [x] 53.4.3 Execute the offline documentation paths from a temporary
    independent directory and retain fixture assertions that artifact names,
    commands, schemas, and output statuses have not drifted.

### 54. Independent Assurance and Controlled Launch Decision [Release Gate E]

**Gate exit:** either a documented `not approved` decision with no mainnet
action, or one limited approved launch whose final evidence independently
reproduces. Only 54.4 may submit a mainnet transaction.

- [ ] 54.0 Publish and commission the independent-review scope and acceptance
  policy before relying on a review.
  - [ ] 54.0.1 Set the review boundary: compiler-to-target correspondence,
    reference registry, release verifier, signer/governance controls,
    transaction-policy validator, evidence/replay implementation, and incident
    runbook.
  - [ ] 54.0.2 Define reviewer independence criteria, permitted evidence,
    disclosure handling, report redaction, and a method for verifying that
    reviewed source/artifact identities match the launch candidate.
  - [ ] 54.0.3 Define severity, owner, remediation evidence, expiry, and
    dispute rules. Unresolved critical/high findings block launch unless the
    published risk policy explicitly rejects the profile.
  - [ ] 54.0.4 Publish the reviewed artifact identity set and engagement
    handoff. A changed compiler, source, `std` dependency, policy, verifier, or
    evidence identity invalidates the handoff and requires a new review-scope
    decision.

- [ ] 54.1 Conduct the independent review and record the findings disposition.
  - [ ] 54.1.1 Supply the locked artifact set and evidence matrix; record
    identities and scope exclusions before review begins.
  - [ ] 54.1.2 Preserve a non-secret findings report and bind each finding to
    its owner, remediation evidence, accepted risk decision, or launch block.
  - [ ] 54.1.3 Re-run affected local gates after every accepted remediation and
    invalidate prior evidence where an identity changes. An audit is supporting
    assurance; it cannot override a failed proof, correspondence, or
    production-precheck gate.

- [ ] 54.2 Reproduce the complete evidence chain in an independent clean
  environment.
  - [ ] 54.2.1 Build or obtain the approved verifier through documented means,
    validate its package, and verify the candidate release/evidence bundle
    without a source checkout or network access.
  - [ ] 54.2.2 Re-run production-precheck, policy validation, offline replay,
    and the documentation walkthrough from the independent environment.
  - [ ] 54.2.3 Compare canonical reports and digest projections. Any difference
    is investigated as a blocker rather than normalized or merged.

- [ ] 54.3 Publish the pre-launch decision before any mainnet submission.
  - [ ] 54.3.1 Record the exact profile/chain, compiler and verifier versions,
    signer policy, exposure cap, monitoring/incident owners, evidence digests,
    review disposition, stop/rollback conditions, unresolved risks, and
    exclusions.
  - [ ] 54.3.2 State an unambiguous `approved` or `not approved` outcome. A
    conditional or incomplete record is `not approved`.
  - [ ] 54.3.3 If not approved, preserve the evidence and stop. Do not create a
    mainnet action or describe the profile as production-ready.

- [ ] 54.4 Execute one limited mainnet launch only after a 54.3 approval.
  - [ ] 54.4.1 Revalidate the bundle, request, approval evidence, signer policy,
    and chain/profile immediately before submission; any identity drift aborts
    the action.
  - [ ] 54.4.2 Submit only the approved zero-value, low-exposure deployment and
    bounded registry operation using the explicit request. Record no secret
    material.
  - [ ] 54.4.3 Capture confirmation/finality and independent observations,
    replay them offline, and stop/escalate on any mismatch, reorganization, or
    unexpected behavior.

- [ ] 54.5 Publish the final production-profile record and closure decision.
  - [ ] 54.5.1 Bind the final deployed address, runtime bytecode, constructor
    inputs, release/evidence bundle, approval, receipts, observations, and
    canonical production-precheck report into a public, secret-free record.
  - [ ] 54.5.2 State the sole supported production claim and every remaining
    exclusion; do not imply general-platform, custody, financial, or
    multi-chain readiness.
  - [ ] 54.5.3 Publish monitoring, vulnerability disclosure, incident response,
    operator handoff, and profile retirement procedures. Mark the milestone
    complete only when every acceptance item and evidence mapping is satisfied.

## Completion Boundary and Follow-On Candidates

Milestone 6 does not automatically authorize Stage 3 or later work from
`general_production_readiness_path.md`. The next milestone must be selected
from observed operator or user blockers after this bounded profile has a stable
evidence record. Candidate directions include a reusable production-assurance
foundation, a declarative Verified Profile/Profile Pack architecture, a
separately locked non-custodial state feature, or expanded target support.
Profile Packs remain post-Milestone-6 work and must not introduce language
semantics or executable plugin hooks; see
`docs/design/verified-profile-packs.md`. Product Profile Packs and Target
Profile Packs will evolve independently, while every deployment stays bound to
one exact compatible pair; no future pack may silently broaden this milestone
to another chain. Token, custody, DeFi, and composability work remain a
distinct high-risk program.

## First Task

**53.2 - implement the offline production-precheck.** The support policy now
pins the candidate compatibility surface and blocks unknown or changed inputs.
The next task composes the fixed local gates into the atomic canonical report;
the unstarted soak must remain a `blocked` result.
