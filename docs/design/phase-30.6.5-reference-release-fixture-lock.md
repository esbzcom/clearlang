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

## Tamper and replay matrix

The reference fixture copies a completed release and separately tampers its source-graph/schema
identity, target artifact/profile, wire ABI, proof, signature payload, assurance payload, campaign
trace, campaign input, campaign replay metadata, and a bundle artifact path. Where appropriate it
also updates the bundle's declared file hash, proving that the verifier's cross-evidence bindings
rather than only raw file hashing reject the alteration. Every case must fail with the stable
`C140` release-verification diagnostic.

## Upgrade, migration, and reentrancy boundaries

The fixture first releases and independently verifies `Counter` version 1. It then releases and
verifies version 2 against that bundle's emitted state schema. Version 2 adds a scalar field and
declares a migration bound to the exact prior schema digest, so the release workflow exercises the
supported schema-compatible upgrade and explicit migration path all the way through packaging.

The same fixture submits a candidate version 3 with a state write after an outbound call. The
release must fail with `T832` and must not emit a release bundle. This establishes the precise CEI
boundary at the packaging entry point; it does not claim protection for targets or interaction
patterns outside the supported model.
