# Milestone 3 Binary Operations

## Scope
Operational guidance for install/upgrade/uninstall/verify workflows on GA baseline targets:
- Windows
- Linux
- macOS

## Install
Supported install channels:

- GitHub release bundle archives for Windows, Linux, and macOS
- Homebrew for Linux and macOS
- Winget for Windows

### Direct GitHub Install

1. Download the platform bundle archive and checksum-bearing metadata from release artifacts.
2. Verify the published checksum/signature metadata before placing `clg(.exe)` in PATH.
3. Verify release evidence bundle with:

```powershell
clg verify-bundle --bundle out/generic.release-bundle.json --keyring keys/release-keyring.json --require-provenance
```

Release artifact production command (CI/operator workflow):

```powershell
$env:CLG_BINARY_RELEASE_SIGNING_KEY_HEX = "<32-byte-ed25519-private-key-hex>"
cargo run -p xtask -- milestone3-binary-bundle --platform <windows|linux|macos> --out-dir tmp/milestone3-binary/<platform>
```

Installer/publication-channel metadata generation command:

```powershell
cargo run -p xtask -- milestone3-installer-channels `
  --version <MAJOR.MINOR.PATCH> `
  --release-tag <TAG> `
  --windows-bundle-dir tmp/milestone3-binary/windows `
  --linux-bundle-dir tmp/milestone3-binary/linux `
  --macos-bundle-dir tmp/milestone3-binary/macos `
  --out-dir tmp/milestone3-installer-channels/<TAG>
```

### Homebrew Install

- Use the generated `homebrew/Formula/clg.rb` output from `milestone3-installer-channels`.
- The formula pins the canonical GitHub milestone_3 bundle archive URL and SHA-256 for:
  - `macos`
  - `linux`

```powershell
brew install --formula ./homebrew/Formula/clg.rb
```

### Winget Install

- Use the generated winget manifests under:
  - `winget/manifests/c/ClearLang/ClearLang/<VERSION>/`
- The installer manifest pins the canonical Windows GitHub bundle archive URL and SHA-256.
- The portable command alias remains `clg`.

```powershell
winget install --manifest .\winget\manifests\c\ClearLang\ClearLang\<VERSION>
```

## Upgrade
1. Download or publish the new channel metadata and bundle archives for the release tag.
2. Verify checksums and signed bundle metadata before promotion.
3. Run `clg --help` and `clg verify-bundle --help` smoke checks.
4. Verify release bundle with keyring/provenance requirement.
5. Replace existing binary atomically or promote the generated Homebrew/winget metadata.

## Uninstall
1. Remove `clg` executable from installed location/PATH or uninstall through the package manager.
2. Remove associated cache/work directories if policy allows.
3. Retain signed release evidence artifacts for audit history.

## CI Smoke Coverage
`milestone3-binary-smoke` job validates per-GA-target binary boot and primary help surfaces:
- `clg --help`
- `clg release --help`
- `clg verify-bundle --help`

`milestone3-proof-parity` and `milestone3-proof-parity-compare` also validate that the bundled
release-grade solver path emits deterministic parity artifacts across Windows, Linux, and macOS.

## References
- `.github/workflows/ci.yml`
- `docs/release-process.md`
