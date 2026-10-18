# Phase 53.4 - Independent-User Documentation Lock

Date: 2026-10-18

Status: Offline documentation contract. It does not authorize a release,
Safe operation, endpoint contact, signing, deployment, invocation, or a
production claim.

## Paths

Phase 53.4 has two deliberately separate paths:

1. [`production-verifier-offline-walkthrough.md`](../release/production-verifier-offline-walkthrough.md)
   is for an independent verifier. It begins with a distributed verifier,
   public distribution-key/trust policy, release/evidence package, and public
   release key. It needs no source checkout, Cargo, private key, wallet, or
   network.
2. [`production-safe-registry-operator-walkthrough.md`](../release/production-safe-registry-operator-walkthrough.md)
   is for an operator preparing local evidence. It identifies each supplied
   value, required approval handoff, optional network boundary, abort rule, and
   evidence-retention action. It grants no authority to take a target action.

The walkthroughs are not interchangeable. An operator does not become an
independent verifier by running a local command, and verifier success does not
authorize a Safe transaction or a production launch.

## Current Support Boundary

The existing distributed `clg-contract-verifier` package is locked to the
Milestone 5 Sepolia pilot profile. It is **not** a production-profile verifier.
An attempt to use it for
`clg.evm-ethereum-mainnet-safe-registry.v1` on chain ID `1` must stop as an
unsupported profile/version rather than reinterpret the older result.

This phase documents the required inputs and rejection behavior so an
independent user can detect that boundary. A future production verifier needs a
separately distributed package, exact public trust input, compatibility record,
and clean-directory validation before it can replace this stop condition.

## Offline Documentation Fixture

`docs/fixtures/phase-53.4/independent-user-documentation-fixtures.json` locks
the artifact names, commands, schemas, expected statuses, prohibited inputs,
and current blocker. It is an assertion fixture, not live target evidence. The
documentation test must read it from a temporary independent directory and
must not require a source checkout, secret, wallet, endpoint, or network.

## Non-Claims

These documents do not establish a release, independent review, testnet soak,
Safe signature validity, endpoint honesty, finality, deployment, invocation,
mainnet readiness, launch approval, or custody safety.
