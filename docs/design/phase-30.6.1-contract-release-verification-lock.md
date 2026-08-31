# Phase 30.6.1 - Contract Release Verification Lock

`clg contract verify-release --bundle <FILE> --pubkey <FILE>` independently verifies a
`clg.contract-release-bundle.v1` from its bundle directory. It accepts only the fixed artifact
set and safe file-name paths, recomputes every declared SHA-256 digest, validates the locked EVM
target/execution profiles, and checks the target artifact's schema and wire-ABI digests against
the packaged schema and ABI. It also requires a v1 proof artifact and verifies both the Ed25519
target-artifact signature and signed assurance manifest.

The verifier also binds the cryptographically verified signature payload to the package: its
module and proofs hashes must agree with the assurance manifest, its canonical proof-artifact
hash must agree with the packaged proof, and its compiler and loaded-source-graph identities must
agree with the EVM artifact, contract ABI, wire ABI, state schema, and proof. The bundle,
signature, and assurance-manifest key IDs must match; the assurance toolchain and fingerprint must
name the same compiler identity. Any absent, malformed, or mismatched claim fails verification.

Verification is deliberately offline and does not contact an RPC endpoint. It proves the package
is internally consistent and signed by the supplied public key; target receipt or signer-trust
policy selection remain explicit operator/release-policy inputs.
