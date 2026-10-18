# Phase 54.2 - Independent Clean-Environment Reproduction Contract

Date: 2026-10-18

Status: Prepared but not started. No approved production verifier package,
eligible soak evidence, or commissioned/retested independent review exists, so
this phase cannot produce a passing reproduction result.

## Required Independent Directory

The eventual reproducer starts from a new local directory containing only the
approved distributed production-verifier package and its trust inputs, the
candidate release/evidence bundle, public release key, frozen review handoff,
and secret-free replay/precheck inputs. It must not receive a source checkout,
Cargo build, private key, wallet, endpoint, production request, Safe signature,
or ambient project artifacts.

The verifier package must be validated using the distribution manifest,
checksum, detached signature, explicitly trusted public key, and exact
production-profile support record before any candidate package is evaluated. A
source build, Sepolia verifier, profile substitution, unknown version, or
network fetch is a blocker—not a fallback.

## Required Offline Reproduction

After the production verifier exists and all Gate D/E prerequisites are
available, the independent environment must:

1. verify the candidate release/evidence bundle for
   `clg.evm-ethereum-mainnet-safe-registry.v1` / chain ID `1`;
2. run `safe-policy-validate`, `safe-evidence-replay`, and
   `production-precheck` with only copied, secret-free inputs and offline,
   locked tooling;
3. run the Phase 53.4 documentation fixture in its own temporary directory;
   and
4. compare canonical report bytes and digest projections against the frozen
   candidate evidence. Any difference, missing file, blocker, unsupported
   format, or non-passing status stops the phase.

Reports cannot be normalized, merged, or hand-edited to erase a difference. A
successful local command does not bypass an unstarted soak, absent production
verifier, or uncommissioned review.

## Current Expected Result

The current decision record is
`docs/fixtures/phase-54.2/independent-clean-reproduction-decision.json`. It
records `not_started` and `not_approved`, with the exact blockers. This is a
precondition record, not a reproduction report. It does not authorize a
network action, Safe action, deployment, invocation, or launch decision.

## Completion Evidence

A future completed Phase 54.2 record must bind the verifier distribution
identity, public trust inputs, candidate/bundle identity, reviewer handoff and
findings disposition, offline command reports, canonical byte/digest
comparisons, supported tool identities, and the secret-free independent
directory inventory. It must state every unresolved exclusion and preserve all
rejecting reports.
