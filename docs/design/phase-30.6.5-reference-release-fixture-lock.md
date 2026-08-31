# Phase 30.6.5 - Reference Release Fixture Lock

The reference stateful `Counter` fixture is the complete supported contract workflow: strict
proof with a verified deterministic solver fixture, seeded simulator campaign, release packaging,
independent `clg contract verify-release`, explicit EVM-compatible deployment, and explicit
signed invocation.

The fixture uses only release-emitted target evidence for deploy and invoke: EVM artifact, wire
ABI, state schema, proof artifact, and signed bundle. Its mock JSON-RPC endpoint checks the
locked chain ID and confirms signed transaction submission and receipts, without assuming a
wallet, discovered endpoint, or live external chain.
