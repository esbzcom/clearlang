# Phase 30.4.0 - Contract Crypto Surface Lock

Date: 2026-08-09
Status: Locked
Owner: crypto-and-proof-owner

## Selected Surface

The first contract profile selects no source-level cryptographic primitive for deployment,
authorization, or target interaction. Existing compiler/release signing primitives remain tooling
integrity mechanisms only and do not become contract semantics by this lock.

## Claim Boundary

Until a contract primitive has a separately locked semantic model or independently verified
attestation boundary, it cannot support a theorem-grade contract proof or a production contract
release claim. Any crypto-dependent contract path must retain a labeled, release-blocking
assumption boundary.

## Exclusions

This lock does not claim cryptographic hardness, key management, signature validity in contract
source, domain separation, replay protection, random-oracle behavior, timing behavior, or
side-channel resistance. It does not expose host crypto APIs to contract transitions.

## Consequence

30.4.1 has no selected contract primitive to model today. It must instead preserve the existing
fail-closed policy and cannot promote tooling crypto to contract assurance without a new lock.

## References

- `docs/design/phase-25.1.19-crypto-release-closure.md`
- `docs/todo/milestone_4_roadmap.md`
