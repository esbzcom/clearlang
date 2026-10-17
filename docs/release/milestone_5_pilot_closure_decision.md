# Milestone 5 Pilot Closure Decision

Date: 2026-10-16
Decision: **Milestone 5 engineering scope is complete as a locally validated, testnet-only
pilot; current pilot acceptance is pending one clean v2 reproduction.**

This decision closes the engineering milestone for the bounded proof-carrying,
caller-authorized configuration-registry workflow. It does not approve a live transaction, certify
an RPC provider or chain, authorize mainnet or production use, establish custody safety, or claim
generic EVM compatibility. A live Sepolia exercise remains an explicit operator action under the
[pilot operator runbook](pilot-operator-runbook.md), with separately reviewed inputs and
authorization.

## Verified Identity Set

| Element | Exact verified value |
| --- | --- |
| Approved clean-clone evidence revisions | `118fd2c3922b0ac71272c3559c23f59c0df6254d` and `928e9ab3822562493abe9291ee00fdad92c0e166` |
| Local host used for the retained rehearsals | `windows-x64` / `x86_64-pc-windows-msvc` |
| Rust / Cargo | `1.89.0` |
| Solver | checked-in Z3 `4.16.0`, external CLI backend; solver profile `25.1.4`, supply-chain policy `25.1.16` |
| Dependency lock SHA-256 | `6a50f62920ff103f68e25f08923e932686ce188c5e8ed63d58e80a00315c9bf2` |
| CLI / independent verifier line | `0.1.0` / `0.1.x` |
| Release bundle | `clg.contract-release-bundle.v1`, schema `1` |
| Verifier result | `clg.contract-release-verifier-result.v1`, schema `1` |
| Current pilot report contract | `clg.pilot-precheck-report.v2`, schema `2`, `scope: "full"` |
| Target / execution profile | `clg.evm-sepolia-pilot.v1` / `clg.evm-stateful-scalar.v1` |
| Historical v1 first and second report SHA-256 | `fed40ee8b5054747c6416815a8324f44c585cba18096aef7635a978312d18d87` |
| Retained versioned walkthrough transcript SHA-256 | `0cc5ee3db89e97a45dc77dc4fc5dd23d5b449febd45c7e368309ae3034827406` |

## Evidence Accepted

1. The first clean-checkout **historical v1** full report passed all five ordered gates: release precheck,
   contract-release fixtures, verifier-distribution fixtures, caller-authorization fixtures, and
   reference-pilot offline replay. Its retained report and digest are in
   `docs/evidence/phase-44.1.2/`.
2. The second clean-checkout **historical v1** report passed the same five gates and is byte-for-byte identical to
   the first. Its retained report and comparison are in `docs/evidence/phase-44.1.3/`.
3. The published offline verifier walkthrough was reproduced in the second clean environment with
   proxy access trapped; it passed without network contact. Its non-secret transcript and witness
   are in `docs/evidence/phase-44.2.2-*`.
4. The local acceptance review found no waived local item. It is retained in
   `docs/evidence/phase-44.2.1-local-acceptance-review.md`.

The retained v1 reports remain historical milestone evidence only. They do not satisfy the v2
acceptance contract. A newly committed clean checkout must reproduce the complete v2 local
precheck before any further testnet action; historical reports must never be edited or
reinterpreted. Any change to the revision, `Cargo.lock`, Rust/Cargo version, supported host,
solver bytes or trust material, target/profile, verifier capability, or report schema likewise
requires a new run under an approved compatibility lock.

## Unresolved Risks and Explicit Exclusions

- No live Sepolia deployment or observation is asserted by this closure record. A future operator
  must retain its own sanitized, bound observations and replay them locally.
- Local verification does not establish RPC honesty, consensus, finality, chain availability, or
  the behavior of any other chain.
- Mainnet use, production-funds use, token/custody/DeFi behavior, key custody, wallet discovery,
  remote signing, and automatic nonce or fee selection remain unsupported.
- Generic EVM behavior, external calls, callbacks, proxy/delegate calls, dynamic ABI values,
  collections, `U256`, arbitrary state, mutable branches, and loops remain outside the product
  profile.
- Remote CI is informational only and has no acceptance authority.

These are boundaries and follow-on risks, not waived acceptance criteria. Any request to cross one
requires a new design lock and its own local evidence before implementation.

This is a locally validated, testnet-only pilot and must not be described as generic EVM support,
remote-CI certification, mainnet readiness, or custody safety.

## Follow-On Candidates

The deferred candidate list in the [Milestone 5 roadmap](../todo/milestone_5_roadmap.md) remains
authoritative: mainnet/key-custody readiness; richer authorization or cryptography; broader ABI,
language, and external-call support; token/custody/DeFi products; and additional chains,
ecosystems, installers, or hosted services. The broader production path remains planning guidance
in `docs/todo/general_production_readiness_path.md`, not scope added to this milestone.
