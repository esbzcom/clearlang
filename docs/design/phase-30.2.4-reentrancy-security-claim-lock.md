# Phase 30.2.4 - Reentrancy Security Claim Lock

Date: 2026-08-09
Status: Locked
Owner: language-security-owner

## Claim

For a contract-owned `mut` transition accepted by the current ClearLang type checker, the source
program has at most one `external_call`, and no state read/write, event emission, or second
outbound call occurs after that call. The checker rejects violations with `T832`.

The local simulator and executable build path fail closed for every `ExternalCall` because no
target external-call adapter exists. Therefore no supported release can currently contain an
executable outbound-call path.

## What This Does Not Claim

This is not a general reentrancy-prevention claim for EVM or any other target. In particular it
does not cover:

- callbacks, fallback/receive dispatch, delegate calls, proxies, raw calldata, or dynamic
  targets;
- target bytecode that was not produced and checked through this source boundary;
- target storage, transaction ordering, gas behavior, exception/revert encoding, or chain
  reorganization;
- cross-function, cross-contract, or asynchronous reentrancy; or
- an external-call transition's invariant at the call boundary, which remains release-blocking
  until a target adapter and solver model prove it.

## Release Rule

Any outbound-call IR operation, unresolved adapter, callback path, or call-boundary proof
assumption blocks a strict production contract release. Simulation success does not waive this
rule.

## Evidence

The CEI negative fixture covers prohibited post-call state interaction, event emission, and a
second call. The build fixture demonstrates that an external-call contract produces no executable
artifact in the absence of an adapter.

## References

- `docs/design/phase-30.2.0-external-call-reentrancy-lock.md`
- `docs/evidence/milestone_4-contract-platform.md`
- `docs/todo/milestone_4_roadmap.md`
