# Phase 30.6.5 - Reference Release Fixture Lock

The reference stateful `Counter` fixture is the complete supported contract workflow: strict
proof with a verified deterministic solver fixture, seeded simulator campaign, release packaging,
independent `clg contract verify-release`, explicit EVM-compatible deployment, and explicit
signed invocation.

The fixture uses only release-emitted target evidence for deploy and invoke: EVM artifact, wire
ABI, state schema, proof artifact, and signed bundle. Its mock JSON-RPC endpoint checks the
locked chain ID and confirms signed transaction submission and receipts, without assuming a
wallet, discovered endpoint, or live external chain.

## Controlled reproducibility

The fixture runs `clg contract release` twice with the same source, strict preflight inputs,
campaign plan, signing key, solver fixture, and output directory. It requires byte-for-byte
equality for the EVM artifact, schema, ABI, wire ABI, VCS, proof artifact, campaign report, and
campaign-evidence index; it independently verifies the resulting bundle after both runs.

The canonical bundle comparison removes only `artifacts.signature` and
`artifacts.signed_assurance_manifest`. Those records hash signed payloads with a wall-clock
release timestamp and are intentionally variable until a future explicit release-time input is
introduced. Every other bundle field, including the campaign-evidence artifact hash, must match.
