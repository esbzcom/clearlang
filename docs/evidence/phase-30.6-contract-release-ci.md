# Phase 30.6 Contract Release CI Gate

The `Milestone 4 contract release workflow gates` CI step is required on every push and pull
request. Its complete release-fixture command is:

```sh
CLG_CONTRACT_RELEASE_EVIDENCE_DIR=tmp/contract-release-fixtures \
  cargo test -p clg-cli --test cli_it basic::contract_release_
```

This runs the actual `contract release` then independent `verify-release` happy path,
deterministic replay comparison, the complete tamper/replay matrix, and supported
upgrade/migration plus reentrancy-negative scenarios. The surrounding step also retains the
stateful artifact, simulator campaign, schema, target configuration, receipt, and type-boundary
regression gates needed by the supported product profile.

The gate is intentionally offline. Its deploy/invoke fixtures use the strict in-process
EVM-compatible endpoint; no public RPC endpoint, wallet, or secret is used in CI. A live-network
deployment is not evidence for this release gate.

The command list is kept in `.github/workflows/ci.yml` so changing any covered release surface
requires changing the reviewed CI contract. Successful runs retain bundle-local evidence at
`tmp/contract-release-fixtures/{happy-path,reproducible,tamper-baseline,migration-v1,migration-v2}`
and publish it as the `milestone4-contract-release-fixtures` GitHub Actions artifact. Matrix
diagnostics and the command result remain in the required job log; no live-network evidence or
credentials are retained.
