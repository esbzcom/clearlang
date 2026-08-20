# Phase 30.6.0 - Contract Release Orchestration Lock

`clg contract release <SOURCE> --plan <FILE> --key <FILE> --pubkey <FILE> --key-id <ID>
--out-dir <DIR>` is the one-command preparation workflow for the supported stateful EVM profile.
It performs these stages in order: strict target build/proof/signing, signature and assurance
verification, deterministic contract-test/simulation campaign, and canonical bundle packaging.

The stateful target artifact itself is the signed module payload. This preserves the existing
Ed25519 proof-package signing format while avoiding a fictional Wasm artifact for a target whose
stateful profile deliberately skips Wasm code generation. The emitted signature binds the target
artifact hash, proof artifact, compiler, and loaded source graph; the package records hashes for
the schema, ABI, wire ABI, target artifact, VCs, proof, signed evidence, and campaign report.

The optional `--prior-state-schema` must pass the existing append-only/migration gate before any
release evidence is produced. Target network submission is intentionally not part of this command:
operators use the explicit `target deploy|call|invoke` commands with the packaged artifacts.

The command verifies evidence that it has just produced. Independent package verification and
trust-policy/keyring selection are deliberately scheduled as 30.6.1.
