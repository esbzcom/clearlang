# Phase 40.0 - Milestone 5 Pilot Product and Threat-Model Lock

Date: 2026-10-13
Status: Locked
Owner: product-and-runtime-owner

## Purpose

Lock the first externally useful ClearLang contract product before Milestone 5 adds caller
authorization, a pilot precheck, a distributed verifier, or a live testnet exercise. This lock
prevents a testnet demonstration from being described as general EVM, mainnet, custody, or
financial-product readiness.

## Product Decision

The Milestone 5 product is a **proof-carrying, caller-authorized configuration registry** on one
EVM-compatible testnet profile selected later by `42.0`.

The registry has one fixed owner, a bounded scalar configuration value, a canonical event for
successful changes, and a versioned state schema. Its purpose is to demonstrate that a developer
can build, prove, campaign-test, simulate, release, deploy/invoke, and independently verify one
complete contract workflow with evidence whose identities remain bound across the workflow.

It is a testnet pilot for configuration integrity. It is not a token, wallet, vault, exchange,
lending protocol, proxy system, arbitrary application runtime, or general-purpose EVM compiler.

## Inherited Milestone 4 Contract

Milestone 5 inherits the following supported Milestone 4 surface unchanged until a later task
extends it under a new lock:

| Area | Inherited contract | Primary reference |
| --- | --- | --- |
| Target profile | `clg.evm-stateful-scalar.v1` through `clg.evm-sepolia-pilot.v1` on chain ID `11155111`; direct initialization, pure reads, and straight-line mutable transitions over `Bool`, `U8`, `U64`, `U128`, and `Int`. | `docs/contract-product-contract.md` |
| Release evidence | Canonical, versioned schema/ABI/wire-ABI/EVM/proof/campaign/signature/assurance/receipt/release-bundle artifacts with cross-artifact identity verification. | `docs/design/phase-30.6.1-contract-release-verification-lock.md` |
| Local workflow | `clg contract test`, `clg simulate`, `clg contract release`, and `clg contract verify-release`. | `docs/contract-product-contract.md` |
| Explicit target actions | `clg target deploy`, `clg target call`, and `clg target invoke`; endpoint, chain ID, signer, nonce, value, gas limit, and gas price are explicit. | `docs/contract-product-contract.md` |
| Diagnostic contract | Bundle/evidence verification failure is `C140`; simulation execution failure is trace-backed `C142`. | `docs/contract-product-contract.md` |
| Local release baseline | Formatting, linting, workspace tests, manifest/architecture/shared-std checks, and the `clg test` schema gate run through `xtask release-precheck`. | `docs/release-process.md` |

Milestone 5 may add the minimum opaque caller identity and equality/guard surface required by the
registry. It must not reinterpret an inherited artifact, profile, diagnostic, or trust boundary
silently.

## Claim Layers

The pilot must use these terms precisely.

| Layer | Allowed statement | Evidence required | Not claimed |
| --- | --- | --- | --- |
| Source/proof | The supported ClearLang source obligations were checked under the named compiler, solver, and proof profile. | Canonical proof artifact, compiler/source identities, and successful release policy evidence. | Correctness of unmodeled language, cryptographic hardness, host, EVM client, chain consensus, or operator behavior. |
| Release evidence | The verifier confirmed that the packaged, bundle-local evidence has matching canonical identities and approved signing/assurance policy. | A successful `clg contract verify-release` result using the approved public key and only packaged evidence. | Safety of a different source tree, edited artifact, substituted target, or an untrusted verifier/key. |
| Local simulation | The supported simulator reproduced the recorded input, caller, limits, transition result, and trace under the named profile. | Canonical simulator trace and replay inputs. | Network submission, target-client correctness, finality, or external-chain behavior. |
| Testnet observation | The selected explicit endpoint returned the recorded transaction/receipt observations for the exact submitted transaction. | Path-safe pilot-evidence record bound to the release bundle, request/response fixtures, and local replay verification. | Mainnet readiness, chain consensus correctness, RPC honesty, permanence/finality beyond the locked observation rule, or custody safety. |

No user-facing document may use `proved`, `verified`, `simulated`, `observed`, `production`, or
`mainnet` outside the layer defined above.

## Pilot Actors and Trusted Inputs

| Actor | Responsibility | Trusted inputs | Must not be assumed |
| --- | --- | --- | --- |
| Release owner | Approves source, release policy, signing key identity, and release bundle for the pilot. | Reviewed source; approved public key and policy; generated local evidence. | That a signature proves target deployment or that a bundle is safe after manual edits. |
| Deploy operator | Supplies explicit target endpoint, chain ID, signer, sender, nonce, fee, gas limit, and constructor/invocation data. Retains non-secret submission evidence. | Their own selected target inputs and signing authority. | That the CLI discovers the right network, wallet, fee, nonce, or account; that an RPC response is truthful. |
| Independent verifier | Validates the distributed verifier package and independently runs release-bundle verification. | Approved verifier checksum/signature, public key, trust policy, and bundle-local evidence. | The release owner's checkout, an ambient source tree, a network connection, or a private key. |
| Observer | Reviews retained non-secret evidence and reports incompatibility or verification mismatch. | Published evidence and verification instructions. | Authority to submit, replace, or approve a transaction. |

The release owner and deploy operator may be the same person for a testnet pilot. Independent
verification must be reproducible from a separate local directory or a second clean environment.

## Threat Model

### Assets protected by this milestone

1. The integrity of the source-to-release verification decision.
2. The identity binding among source graph, proof, schema, ABI, EVM artifact, campaign evidence,
   signature/assurance evidence, target action, and receipt.
3. The registry's supported configuration invariant and caller-authorization result.
4. The operator's ability to detect wrong-chain, wrong-artifact, malformed-evidence, and
   unauthorized-transition failures before accepting a pilot result.
5. Testnet signing keys and funds only as operator-managed test assets; they must never appear in
   source, release bundles, fixtures, reports, documentation examples, or retained evidence.

### Attacks the milestone must reject or expose

1. Source, proof, schema, ABI, wire-ABI, artifact, campaign, signature, assurance, receipt, or
   bundle-path substitution.
2. Release-bundle traversal paths, omitted evidence, altered evidence ordering, and replay under
   a mismatched identity.
3. Wrong caller, malformed caller, absent caller, or caller substitution between proof,
   simulation, campaign, target invocation, and receipt.
4. Wrong chain ID, endpoint/profile mismatch, malformed RPC response, reverted transaction, and
   receipt/artifact identity drift.
5. An incompatible or tampered verifier package, public key, trust policy, schema, or profile.
6. Accidental secret inclusion in published artifacts, reports, or runbooks.

### Explicitly outside the claim

1. Safety of an operator's machine, operating system, dependency supply chain, private key,
   wallet, hardware signer, or endpoint credential.
2. Honesty, availability, consensus/finality, censorship resistance, reorganization behavior, or
   implementation correctness of any RPC provider, EVM client, testnet, or blockchain.
3. Cryptographic hardness, side-channel resistance, or correctness of external cryptographic
   libraries beyond existing versioned assurance boundaries.
4. Any value custody, economic correctness, regulatory suitability, user interface, web/mobile
   integration, or recovery of a lost key/fund.
5. Generic EVM behavior or unsupported ClearLang source/target features.

## Required Failure Response

Every failure below is fail closed: it must not be represented as a successful release, deployment,
invocation, receipt, or pilot observation.

| Condition | Required response |
| --- | --- |
| Proof, bundle, verifier, or identity mismatch | Stop; preserve the failing non-secret evidence; rerun independent verification only after identifying the changed input; do not submit a target transaction. |
| Wrong chain/profile or malformed target response | Stop before accepting/submitting where possible; preserve sanitized target evidence; do not create a success receipt. |
| Revert or authorization failure | Record it as the expected negative result; assert no state commit/success receipt; do not relabel simulator evidence as a target success. |
| Suspected signer/key compromise | Stop all pilot transactions, revoke the pilot key from operator use, preserve evidence, and use a newly generated test-only key only after review. |
| Unexpected target behavior | Stop the pilot, preserve source/release/request/response/receipt identities, publish no compatibility claim, and open a compatibility investigation. |

## Measurable Milestone 5 Acceptance Statements

Milestone 5 can close only if all statements are demonstrably true on a clean local checkout:

1. `xtask pilot-precheck` runs all required local gates in canonical order and emits a
   secret-free, path-safe, versioned report whose evidence is independently checkable.
2. A reference registry source file can be proven, campaign-tested, simulated, released, and
   independently verified using only the emitted release bundle and approved public key.
3. An authorized caller update succeeds in the locked simulator and target fixture; the recorded
   source/proof/schema/ABI/artifact/campaign/receipt identities agree with the release bundle.
4. An unauthorized or malformed caller fails deterministically, commits no state, and cannot
   produce a successful release or receipt.
5. The distributed verifier gives the same canonical acceptance/rejection result as the primary
   bundle verifier without a source checkout or network connection.
6. An explicitly initiated testnet exercise records a deployment and authorized/unauthorized
   invocation observation whose identities verify locally and can be replayed from recorded
   fixture responses.
7. A second clean local directory or independent operator reproduces the offline verification and
   local closure evidence.

## Non-Claims and Promotion Rule

Milestone 5 must not claim mainnet readiness, production financial suitability, custody safety,
generic EVM compatibility, remote-CI certification, or arbitrary-contract correctness.

Promotion beyond a testnet pilot requires a new design lock. The next eligible promotion is the
same registry profile on one named mainnet under the Stage 2 requirements in
`docs/todo/general_production_readiness_path.md`; it is not authorization to add general language
features or money-moving use cases.

## Terminology-Drift Review Fixture

`docs/fixtures/phase-40.0/pilot-claim-terminology-checklist.json` is the versioned checklist for
documents that state a Milestone 5 product claim. Before adding or changing such a document, the
author must review the listed required terms, prohibited claims, and source documents. The final
closure review (`44.0.3`) upgrades this checklist into an enforced documentation-consistency gate.

## Design Principles Check

- **Simple for users:** one narrow registry, explicit target inputs, and separate language for
  proved, verified, simulated, and observed results avoid misleading magic.
- **AI-friendly:** stable terms, named actors, canonical evidence, and table-driven failure rules
  make an automated workflow inspectable and repairable.
- **Provably correct:** the source proof boundary remains explicit, and mismatched evidence or
  unsupported behavior fails closed rather than inheriting trust from a happy-path result.
- **Crypto-focused:** the lock prioritizes authorization, signing/evidence identity, target
  correspondence, and adversarial substitution over unrelated language convenience.

## Exit Criteria

1. The roadmap marks `40.0`, `40.0.1`, `40.0.2`, and `40.0.3` complete and references this
   lock.
2. The inherited contract inventory names authoritative documents and has no conflicting claim.
3. Every pilot claim maps to a distinct evidence layer and measurable local acceptance statement.
4. Actors, trusted inputs, attacker capabilities, exclusions, and failure responses are explicit.
5. The terminology-drift checklist fixture exists and is referenced by this lock.

## References

- `README.md`
- `docs/todo/milestone_5_roadmap.md`
- `docs/todo/general_production_readiness_path.md`
- `docs/contract-product-contract.md`
- `docs/contract-operations.md`
- `docs/release-process.md`
- `docs/design/phase-30.0.0-milestone-4-production-contract-platform-lock.md`
- `docs/design/phase-30.6.1-contract-release-verification-lock.md`
