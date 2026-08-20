# Phase 30.6 Contract Release CI Gate

The `Milestone 4 contract release workflow gates` CI step is required on every push and pull
request. It runs deterministic fixtures for the stateful EVM artifact, seeded campaign replay,
release/verification CLI contract, schema upgrade compatibility, explicit deploy/invoke
configuration, signed transaction receipts, cross-artifact tamper rejection, and the
checks-effects-interactions reentrancy-negative path.

The gate is intentionally offline. Its deploy/invoke fixtures use the strict in-process
EVM-compatible endpoint; no public RPC endpoint, wallet, or secret is used in CI. A live-network
deployment is not evidence for this release gate.

The command list is kept in `.github/workflows/ci.yml` so changing any covered release surface
requires changing the reviewed CI contract.
