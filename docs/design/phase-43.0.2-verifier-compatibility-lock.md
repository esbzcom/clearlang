# Phase 43.0.2: Independent verifier compatibility and artifact lock

Date: 2026-10-15  
Status: Locked  
Owner: release-assurance-owner

## Scope

This lock defines the public compatibility contract for the dedicated
`clg-contract-verifier` selected in 43.0.1. It applies only to offline verification of the
Milestone 5 `clg.contract-release-bundle.v1` product. It does not broaden target, source,
wallet, RPC, or container support.

## Supported platform artifacts

The first pilot verifier release supports only these native x86_64 targets:

| Platform label | Rust target | Archive | Executable inside archive |
| --- | --- | --- | --- |
| `windows-x86_64` | `x86_64-pc-windows-msvc` | `.zip` | `clg-contract-verifier.exe` |
| `linux-x86_64` | `x86_64-unknown-linux-gnu` | `.tar.gz` | `clg-contract-verifier` |
| `macos-x86_64` | `x86_64-apple-darwin` | `.tar.gz` | `clg-contract-verifier` |

The release is not supported on ARM, musl Linux, 32-bit platforms, other Windows toolchains, or
other operating systems. A user on an unsupported platform must use a supported platform; the
pilot does not authorize source builds, emulation, containers, or a hosted verifier as substitutes.

For a release version `<VERSION>` (without a leading `v`), each archive name is exact:

```text
clg-contract-verifier-v<VERSION>-<PLATFORM>.zip       # Windows
clg-contract-verifier-v<VERSION>-<PLATFORM>.tar.gz    # Linux and macOS
```

The release set also contains these exact cross-platform names:

```text
clg-contract-verifier-v<VERSION>-checksums.txt
clg-contract-verifier-v<VERSION>-manifest.json
clg-contract-verifier-v<VERSION>-manifest.sig.json
```

There is one archive per supported platform, one sorted checksum listing, and one signed manifest.
Archive layout is flat: it contains the named verifier executable and no installer, shell script,
embedded key, source checkout, RPC configuration, or mutable plugin.

## Integrity and trust inputs

`checksums.txt` uses one canonical, lexicographically sorted line per archive:

```text
<64 lowercase hexadecimal SHA-256>  <archive filename>
```

The canonical JSON manifest has format `clg.contract-verifier-distribution-manifest.v1` and schema
version `1`. It binds the exact verifier version, result-schema format, supported platform labels,
archive filename, archive SHA-256, and executable filename. `manifest.sig.json` is a detached
Ed25519 signature over the manifest's canonical bytes, identifies the distribution key, and is
verified using the documented distribution public key obtained through an out-of-band trusted
release channel. The checksum listing is a convenience cross-check, not a replacement for the
signed manifest.

Release-bundle trust is a separate explicit input. The verifier requires:

```text
clg-contract-verifier verify-release --bundle <BUNDLE> --pubkey <RELEASE_PUBLIC_KEY>
```

`<RELEASE_PUBLIC_KEY>` is the existing JSON object with exactly `scheme: "ed25519"` and a
canonical Ed25519 `public_key`. It must be provided by the verifier operator from their approved
trust material. The executable does not discover keys, read a user keyring, fetch a trust policy,
or treat the distribution-signing key as a release-bundle signing key. Any future multi-key or
time-bound trust-policy file needs a separate versioned input and design lock.

## Compatibility contract

The verifier artifact version is SemVer and is published as the ClearLang verifier version. The
pilot compatibility line is `0.1.x`: patch releases in that line preserve support for the exact
input/output formats below. A new minor or major version is never assumed compatible merely from
its version number; the signed distribution manifest's explicit capability list is authoritative.

| Contract element | Accepted by pilot verifier | Failure behavior |
| --- | --- | --- |
| Release bundle | `clg.contract-release-bundle.v1`, schema `1` | Reject unknown format/version |
| Target profile | `clg.evm-sepolia-pilot.v1` only | Reject different/unknown profile |
| Target execution profile | `clg.evm-stateful-scalar.v1` only | Reject different/unknown profile |
| Bundle paths | Bundle-local regular files with safe single-component names | Reject traversal, symlink, absent, or ambient path |
| Release public key | Explicit canonical Ed25519 public-key JSON | Reject missing/malformed/untrusted-by-operator key |
| Machine result | `clg.contract-release-verifier-result.v1`, schema `1` | Reject unsupported requested result schema |

The verifier must expose its own version and capability set before verification. It must reject a
bundle that declares a minimum verifier capability it does not support; it must never select a
nearest schema/profile, silently downgrade a policy, or claim success on an unknown field/version.

## Offline and output contract

Verification is offline by construction. The verifier has no endpoint, download, registry,
telemetry, wallet, signer, plugin, environment-discovery, or retry option. It consumes only the
bundle directory and the explicit public-key file. An attempted network-dependent wrapper is not
the supported verifier operation.

The future shared verification entrypoint has two renderers only:

- Human success: `verified: <bundle_sha256> <target_profile>`.
- Human rejection: `rejected [<stable_code>]: <non-secret reason>`.
- JSON result, one canonical line:

```json
{
  "bundle_sha256": "sha256:<64 lowercase hex>",
  "format": "clg.contract-release-verifier-result.v1",
  "schema_version": 1,
  "status": "verified",
  "target_profile": "clg.evm-sepolia-pilot.v1",
  "verifier_version": "0.1.0"
}
```

Rejected JSON uses the same `format`, `schema_version`, and `verifier_version`, with
`status: "rejected"`, a stable diagnostic `code`, and a non-secret `reason`. Neither renderer may
include a local absolute path, hostname, username, endpoint, key material, timing value, or
environment value. `clg contract verify-release` and `clg-contract-verifier` must render these
same results from the shared core.

## Implementation acceptance

43.1 must locally package every locked platform artifact with its checksum and signed manifest,
then verify the manifest and execute the verifier from a clean directory. The package must reject
version/profile/schema/key/path tampering without contacting a network. These are local tests;
remote CI is not a completion dependency.

## References

- `docs/design/phase-43.0.1-verifier-delivery-form-lock.md`
- `docs/design/phase-30.6.1-contract-release-verification-lock.md`
- `docs/design/phase-29.12.0-multi-platform-ga-support-lock.md`
- `docs/release/milestone_3-binary-operations.md`
