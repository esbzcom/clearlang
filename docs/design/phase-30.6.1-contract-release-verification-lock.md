# Phase 30.6.1 - Contract Release Verification Lock

`clg contract verify-release --bundle <FILE> --pubkey <FILE>` independently verifies a
`clg.contract-release-bundle.v1` from its bundle directory. It accepts only the fixed artifact
set and safe file-name paths, recomputes every declared SHA-256 digest, validates the locked EVM
target/execution profiles, and checks the target artifact's schema and wire-ABI digests against
the packaged schema and ABI. It also requires a v1 proof artifact and verifies both the Ed25519
target-artifact signature and signed assurance manifest.

Verification is deliberately offline and does not contact an RPC endpoint. It proves the package
is internally consistent and signed by the supplied public key; target receipt or signer-trust
policy selection remain explicit operator/release-policy inputs.
