# Milestone 3 Binary Operations

## Scope
Operational guidance for install/upgrade/uninstall/verify workflows on GA baseline targets:
- Windows
- Linux

macOS is preview in this phase and non-blocking for GA closure.

## Install
1. Download the platform binary and checksum sidecar from release artifacts.
2. Verify checksum before placing in PATH.
3. Verify release evidence bundle with:

```powershell
clg verify-bundle --bundle out/generic.release-bundle.json --keyring keys/release-keyring.json --require-provenance
```

Release artifact production command (CI/operator workflow):

```powershell
$env:CLG_BINARY_RELEASE_SIGNING_KEY_HEX = "<32-byte-ed25519-private-key-hex>"
cargo run -p xtask -- milestone3-binary-bundle --platform <windows|linux> --out-dir tmp/milestone3-binary/<platform>
```

## Upgrade
1. Download new binary + checksums + updated release bundle.
2. Verify checksums.
3. Run `clg --help` and `clg verify-bundle --help` smoke checks.
4. Verify release bundle with keyring/provenance requirement.
5. Replace existing binary atomically.

## Uninstall
1. Remove `clg` executable from installed location/PATH.
2. Remove associated cache/work directories if policy allows.
3. Retain signed release evidence artifacts for audit history.

## CI Smoke Coverage
`milestone3-binary-smoke` job validates per-GA-target binary boot and primary help surfaces:
- `clg --help`
- `clg release --help`
- `clg verify-bundle --help`

## References
- `.github/workflows/ci.yml`
- `docs/release-process.md`
