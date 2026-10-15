# Phase 43.1.3: Verifier clean-directory matrix

Date: 2026-10-15  
Status: Implemented  
Owner: release-assurance-owner

## Scope

This phase proves the distributable verifier's behavioral boundary with a black-box test. The
test copies the `clg-contract-verifier` executable, a completed release-bundle evidence tree, and
the explicit release public key into a new temporary directory. It deletes the source file outside
that directory before invoking the copied binary. No source checkout, private key, target endpoint,
wallet, signing environment, or ambient artifact is supplied to the verifier.

The test requires the copied binary to accept the valid copied bundle and emit the canonical
`clg.contract-release-verifier-result.v1` success result without exposing the original workspace
path.

## Rejection matrix

The same clean-directory binary must return stable `C140` JSON diagnostics for every case below:

| Class | Cases exercised |
| --- | --- |
| Inherited evidence tampering | source graph, target profile, wire ABI, proof artifact, signature payload, assurance manifest, campaign trace, campaign input, campaign replay, unsafe bundle path |
| Required evidence and trust | missing proof evidence, wrong explicit Ed25519 release public key |
| Version/profile compatibility | unknown bundle schema, unknown target profile, bundle minimum verifier version `0.2.0` when this verifier is `0.1.0` |

The release bundle now records `minimum_verifier_version`. The shared core accepts an absent value
for existing v1 evidence, rejects an invalid value, and rejects a bundle requiring a later exact
SemVer verifier. It never treats an unknown schema/profile or future minimum version as a reason
to downgrade checks or select a nearby format.

## Offline assertion

For the valid black-box invocation, the test configures HTTP, HTTPS, and generic proxy variables
to an ephemeral local listener and then checks that the listener received no connection. The
verifier succeeds despite that trap because it has no endpoint, download, registry, telemetry,
wallet, signer, retry, or environment-discovery operation. This is a local test; remote CI is not
a completion dependency.

## References

- `crates/cli/tests/cli_it/basic/contract_test_campaign.rs`
- `crates/cli/src/commands/contract_release.rs`
- `docs/design/phase-43.1.1-shared-verifier-entrypoint.md`
- `docs/design/phase-43.1.2-verifier-package.md`
