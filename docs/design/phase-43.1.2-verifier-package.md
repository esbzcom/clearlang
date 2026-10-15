# Phase 43.1.2: Deterministic verifier package

Date: 2026-10-15  
Status: Implemented  
Owner: release-assurance-owner

## Purpose

This phase supplies the local, deterministic package producer and verifier for the dedicated
`clg-contract-verifier` executable. It implements the Phase 43.0 artifact names and detached
manifest-signature contract without a network call, installer, registry lookup, key discovery, or
source checkout at verification time.

## Package command

The release owner supplies the three explicitly built native binaries, an explicit Ed25519
distribution signing-key JSON file, an explicit key identifier, exact version, and a new output
directory:

```powershell
cargo run -p xtask -- contract-verifier-package `
  --version 0.1.0 `
  --artifact windows-x86_64=target/x86_64-pc-windows-msvc/release/clg-contract-verifier.exe `
  --artifact linux-x86_64=target/x86_64-unknown-linux-gnu/release/clg-contract-verifier `
  --artifact macos-x86_64=target/x86_64-apple-darwin/release/clg-contract-verifier `
  --signing-key keys/verifier-distribution-signing-key.json `
  --key-id verifier-distribution-2026q4 `
  --out-dir out/verifier-package
```

The signing-key input is a local JSON object with exactly `scheme: "ed25519"` and a lowercase,
32-byte hexadecimal `private_key`. It is packaging-only and distinct from both the release-bundle
signing key and the public key given to an independent verifier. The command refuses to overwrite
an existing output directory.

It requires exactly one artifact for each locked platform; a partial package cannot be promoted.
It writes a flat distribution directory containing:

```text
clg-contract-verifier-v<VERSION>-windows-x86_64.zip
clg-contract-verifier-v<VERSION>-linux-x86_64.tar.gz
clg-contract-verifier-v<VERSION>-macos-x86_64.tar.gz
clg-contract-verifier-v<VERSION>-checksums.txt
clg-contract-verifier-v<VERSION>-manifest.json
clg-contract-verifier-v<VERSION>-manifest.sig.json
```

Windows uses a deterministic stored ZIP with its timestamp fixed at the ZIP epoch. Linux and
macOS use a deterministic USTAR archive inside a stored-block gzip stream, with zero timestamps,
uid, and gid. Each archive contains exactly its flat, named verifier executable.

## Manifest and signature

The package manifest is canonical JSON with format
`clg.contract-verifier-distribution-manifest.v1`, schema `1`, verifier version, the locked result
and profile capabilities, and one sorted record for every platform archive. Each record binds the
platform, archive name and SHA-256, executable name, and executable SHA-256.

The checksum listing has one lexicographically sorted canonical line per archive:

```text
<64 lowercase hexadecimal SHA-256>  <archive filename>
```

`manifest.sig.json` has format `clg.contract-verifier-distribution-signature.v1`, schema `1`, key
ID, SHA-256 of the exact canonical manifest bytes, and a detached Ed25519 signature over those
bytes. The public distribution key is deliberately not embedded in the package.

## Offline verification and self-check

After generating all files, the packaging command verifies its own output before it reports
success. It validates canonical manifest bytes, the detached signature, every checksum line and
archive digest, archive format, flat executable name, and executable digest.

An independent local check requires the operator's explicit trusted distribution public-key JSON:

```powershell
cargo run -p xtask -- contract-verifier-package-verify `
  --package-dir out/verifier-package `
  --pubkey keys/verifier-distribution-pubkey.json
```

The public-key input has exactly `scheme: "ed25519"` and a lowercase, 32-byte hexadecimal
`public_key`. Verification rejects unknown capability/profile/result formats, partial platform
sets, malformed names, noncanonical manifests, mismatched checksums, signature failures, unsafe
archive contents, or substituted executables. It makes no network request.

## Local evidence

`contract_verifier_package_is_deterministic_signed_and_self_verified` produces two packages from
the same three local fixture binaries and key, compares every emitted byte stream, verifies the
package using only its explicit public key, and confirms that a modified archive is rejected.

## References

- `xtask/src/main/core/dispatch_and_drift.rs`
- `xtask/src/main/tests.rs`
- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
- `docs/design/phase-43.0.3-verifier-lifecycle-and-rollback-lock.md`
