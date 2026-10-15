# Phase 43.1.1: Shared verifier entrypoint

Date: 2026-10-15  
Status: Implemented  
Owner: release-assurance-owner

## Delivered form

`clg-contract-verifier` is a dedicated native binary in the `clg-cli` package. Its only command
is the offline explicit-input form:

```text
clg-contract-verifier verify-release --bundle <BUNDLE> --pubkey <RELEASE_PUBLIC_KEY>
```

The binary is a thin Clap front end over `commands::contract_release::verify`, the same public
entrypoint called by:

```text
clg contract verify-release --bundle <BUNDLE> --pubkey <RELEASE_PUBLIC_KEY>
```

Consequently neither command copies release-bundle parsing, bundle-local path checks, hashing,
signature validation, campaign evidence validation, or cross-artifact policy. Both commands also
share the existing `--json-errors`, `--json-events`, and verbosity behavior.

## Canonical successful result

Both entrypoints render the same path-free JSON result after a successful verification:

```json
{
  "bundle_sha256": "sha256:<64 lowercase hexadecimal characters>",
  "format": "clg.contract-release-verifier-result.v1",
  "schema_version": 1,
  "status": "verified",
  "target_profile": "clg.evm-sepolia-pilot.v1",
  "verifier_version": "0.1.0"
}
```

The bundle digest binds the exact supplied manifest bytes. The output contains neither the bundle
path nor any ambient-machine value. Rejections continue to use the existing stable `C140` release
verification diagnostic when `--json-errors` is requested; Phase 43.1.3 will exercise that
diagnostic contract from an independent clean directory.

## Local evidence

`independent_verifier_matches_primary_verifier_canonical_result` creates a complete supported
release, invokes both executables with the same explicit bundle and public key, and requires
byte-for-byte equal stdout. It also validates the result format, schema, version, bundle digest,
target profile, and absence of the temporary local directory path.

## Deferred packaging

This phase adds the executable but does not claim a distributed archive. Phase 43.1.2 adds the
deterministic archive, checksums, manifest, and signature/provenance bindings; Phase 43.1.3 adds
the clean-directory package/tamper matrix.

## References

- `crates/cli/src/bin/contract_verifier.rs`
- `crates/cli/src/commands/contract_release.rs`
- `crates/cli/tests/cli_it/basic/contract_test_campaign.rs`
- `docs/design/phase-43.0.1-verifier-delivery-form-lock.md`
- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
