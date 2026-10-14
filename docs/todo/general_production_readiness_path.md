# General Production Readiness Path

## Purpose

This is a practical planning reference for growing ClearLang from the Milestone 5 testnet pilot
into a broadly useful production contract platform. It is **not** a promise that all listed work
is required before any production deployment, nor an authorization to start every track at once.

ClearLang becomes production-ready profile by profile. A profile is a small, explicit combination
of source features, proof assumptions, target behavior, operations, and supported use cases. A
profile can be production-ready while broader language or EVM features remain unsupported and
fail closed.

## Product Goal

Enable users to deploy and independently verify narrowly defined, production contracts with
credible security and operational boundaries, then expand those boundaries only when real users
need a specific additional capability.

The first intended production product is not a general DeFi platform. It is a single-chain,
caller-authorized configuration-registry profile: a low-value, non-custodial contract whose
authorized state changes are proved, evidence-bound, independently verified, and operated with
explicit keys and target inputs.

## Working Principle

Do not grow the language because a feature is familiar or convenient. Grow it when a concrete
pilot is blocked, and only after the feature has a bounded security claim, formal model, target
mapping, deterministic local evidence, and operations story.

The primary acceptance authority remains the local deterministic gate. Hosted/remote CI may be
informational, but it is not a release or readiness requirement in this path.

## Incremental Path

### Stage 1 — Milestone 5: Testnet Pilot

Deliver the caller-authorized configuration registry on one locked EVM-compatible testnet.

Success means:

- the source, proof, ABI, EVM artifact, campaign evidence, release bundle, and target receipts
  have verified cross-artifact identities;
- an independent user can verify the bundle from a distributed verifier without a source checkout;
- local `xtask pilot-precheck` reproduces all required offline evidence;
- a live testnet exercise is explicit, replayable from recorded fixtures, and clearly labelled as
  observed target behavior rather than a proof of the chain or RPC provider.

This stage produces a credible pilot, not a mainnet, financial, or generic EVM claim.

### Stage 2 — Milestone 6: Mainnet Readiness for One Profile

Promote only the Stage 1 registry profile to one selected mainnet. Keep the source profile and
business use case unchanged while hardening the real deployment boundary.

Required work:

1. Lock one mainnet profile: chain ID, transaction envelope, required RPC methods, compatible
   client observations, confirmation/finality rule, reorganization response, and a documented
   fee/nonce/replacement policy.
2. Add production operator controls: approved signer model, multisig or equivalent governance
   policy, key rotation/revocation process, least-privilege roles, emergency pause decision, and
   a secret-free evidence-retention process.
3. Verify the actual deployment: independently compare deployed runtime bytecode, constructor
   inputs, address, storage-layout identity, transaction/receipt evidence, and release-bundle
   evidence before the deployment is accepted as successful.
4. Add operations: monitoring, alerting, incident triage, wrong-chain and wrong-configuration
   prevention, failed-transaction handling, vulnerability disclosure, and an immutable-contract
   remediation plan.
5. Require independent assurance proportionate to risk: external review of the compiler/backend
   boundary, reference contract, verifier, and operator runbook; an audit is additional evidence,
   not a replacement for local proofs and tests.
6. Run a public testnet soak and a limited, low-value mainnet launch with explicit stop conditions.

Completion claim:

> Production-ready for the named configuration-registry profile on the named chain only.

### Stage 3 — Production Platform Foundation

Make the workflow repeatable for more than one carefully bounded application without expanding
contract semantics yet.

Required work:

- support/version matrix for compiler, verifier, solver, operating systems, and target clients;
- deterministic local `production-precheck`, with canonical evidence report and clean-environment
  reproduction requirement;
- verifier and release distribution lifecycle: signing, checksums, compatibility, rollback, CVE
  response, and deprecation policy;
- differential execution and receipt-validation fixtures against independent EVM clients for every
  supported target profile;
- property/fuzz testing, fault injection, and tamper/replay regression matrices;
- a public product contract, security-contact process, and release/incident ownership.

Completion claim:

> The operational and evidence pipeline is reusable for published supported profiles.

### Stage 4 — Safe Stateful Application Profiles

Add source features only as needed for non-custodial applications such as governed registries,
allowlists, and bounded parameter management.

Likely sequence:

1. Roles, multisig administration, key rotation, and deliberately specified emergency controls.
2. Bounded collections/mappings and deterministic storage proofs.
3. Checked `U256` arithmetic and overflow/underflow proof semantics.
4. Guarded mutable branches and loops with explicit invariants, termination/resource bounds, and
   state-transition proof obligations.
5. Standardized, audited reference templates for the supported profiles.

Each item is its own profile extension. It is not enabled until the capability-promotion rule
below is satisfied.

### Stage 5 — Value and Composability Profiles

Treat money-moving and cross-contract functionality as a separate high-risk program, not a small
extension of a registry.

Candidate work:

- resource/token semantics and conservation proofs;
- source-level cryptographic signatures, domains, nonces, and replay protection;
- value transfer and accounting safety;
- outbound calls, callbacks, failure handling, and formally specified reentrancy boundaries;
- token, vault, and settlement reference profiles;
- stronger compiler-to-EVM correspondence evidence and specialized external audits.

No contract handling customer funds, external counterparties, or composable value should be called
production-ready merely because Stages 1–4 are complete.

### Stage 6 — Mature Ecosystem

Only after multiple profiles have real users and stable evidence should ClearLang broaden its
delivery and target story:

- additional chains and target profiles;
- package governance and long-term language compatibility;
- IDE/installer ecosystem maturity;
- public security research, bug-bounty process, and third-party verifier adoption;
- wider support commitments and published service/security objectives.

## Capability-Promotion Rule

Every newly supported capability follows this sequence. Skipping a step means it remains
experimental or unsupported, not production-ready.

1. Write a design, threat-model, proof-claim, and explicit non-goal lock.
2. Define source typing/effects, canonical artifacts, diagnostics, and compatibility behavior.
3. Implement a fail-closed parser/type/IR/VC model and a bounded target mapping.
4. Add deterministic unit, integration, property/fuzz, tamper, replay, and differential fixtures
   appropriate to the capability.
5. Bind the capability's identities into proof, schema, ABI, artifact, receipt, and release-bundle
   verification.
6. Add it to the local deterministic precheck and independently reproduce the evidence in a clean
   environment.
7. Obtain independent review proportionate to the potential value and attack surface.
8. Pilot at low risk, publish the supported profile and exclusions, then promote it deliberately.

## What Is Not Required Up Front

The following work is useful only when it unblocks the currently selected profile or a real user:

- generic EVM compatibility;
- every language feature in the README vision;
- multiple chains, wallets, installers, or hosted services;
- DeFi, tokens, custody, proxy upgrades, or external calls;
- remote CI as a release gate.

This keeps the work practical: Stage 2 can deliver a real production registry before the later
stages exist.

## Readiness Labels

Use exact labels in documentation and release evidence:

| Label | Meaning |
| --- | --- |
| Experimental | No production claim; behavior may change or be incomplete. |
| Testnet pilot | Locally validated on one testnet; no mainnet/funds claim. |
| Production profile | Approved for the named use case, target profile, and operational policy only. |
| General platform | A collection of separately approved production profiles; never an implication that arbitrary contracts are safe. |

## Next Action

Complete Milestone 5 task `40.0` first. After its testnet pilot closes, create the Milestone 6
roadmap from Stage 2 for the exact registry profile and selected mainnet; do not begin later stages
preemptively.

