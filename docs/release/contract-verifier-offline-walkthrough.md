# Offline contract-verifier walkthrough

This is the external-user path for independently checking one ClearLang contract-release
bundle. It is deliberately separate from the release-owner and pilot-operator paths: it does
not use a source checkout, Cargo, an account, a private key, a package registry, or a network
endpoint.

The current pilot supports `windows-x86_64`, `linux-x86_64`, and `macos-x86_64` only. The
copy/paste procedure below is for the Windows artifact. A user on another supported platform
must use the exact archive and executable named for that platform in the signed manifest; they
must not substitute a source build, container, emulator, or a similarly named binary.

## Inputs and local prerequisites

Before starting, obtain these files through the release channel, then work from a local directory:

- the complete verifier package for one version, including its three archives, checksum listing,
  manifest, and detached manifest signature;
- a complete contract-release bundle directory, with the bundle JSON and every file it references;
- the release-bundle public-key JSON supplied by the approved release-key channel; and
- the distribution public-key hexadecimal value and distribution key ID from a separately trusted
  release channel. Do not take this key from the downloaded package.

The procedure needs PowerShell 7 or later and a locally installed `openssl` command capable of
Ed25519 `pkeyutl` verification. Both are local tools: do not install or update them while running
this verification path. Copy the supplied files before disconnecting from a network if that is
useful for your operating procedure.

The distribution public key is not the release-bundle public key. Neither input is a private key.

## Windows copy/paste path

Create a new directory containing the received files with this layout. The package directory must
contain only the release-package files; the bundle directory must retain all bundle-local evidence
beside its `*.contract-release-bundle.json` file.

```text
received/
  verifier-package/
  contract-release/
  release-public-key.json
```

In PowerShell, change the five values at the top of this block. The public key and key ID must be
transcribed from the separately trusted release channel, not copied from package metadata. The
script rejects before extraction when the package, checksum list, manifest, or signature is not
the locked form. It never invokes the verifier until those checks pass.

```powershell
$ErrorActionPreference = 'Stop'

# Values supplied out of band by the trusted distribution-key channel.
$Version = '<VERSION>'
$DistributionPublicKeyHex = '<64-lowercase-hex-ed25519-distribution-public-key>'
$ExpectedDistributionKeyId = '<distribution-key-id>'

# Received local files only. Change the bundle filename if the release owner named it differently.
$PackageDirectory = (Resolve-Path '.\received\verifier-package').Path
$BundlePath = (Resolve-Path '.\received\contract-release\counter.contract-release-bundle.json').Path
$ReleasePublicKeyPath = (Resolve-Path '.\received\release-public-key.json').Path
$Platform = 'windows-x86_64'

function Stop-Verification([string] $Message) {
    throw "STOP: $Message"
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') { Stop-Verification 'version is not exact SemVer' }
if ($DistributionPublicKeyHex -notmatch '^[0-9a-f]{64}$') {
    Stop-Verification 'distribution public key must be 32 lowercase hexadecimal bytes'
}
if ([string]::IsNullOrWhiteSpace($ExpectedDistributionKeyId)) {
    Stop-Verification 'expected distribution key ID is required'
}
if (-not (Get-Command openssl -ErrorAction SilentlyContinue)) {
    Stop-Verification 'openssl with Ed25519 support is required locally'
}

$ManifestPath = Join-Path $PackageDirectory "clg-contract-verifier-v$Version-manifest.json"
$SignaturePath = Join-Path $PackageDirectory "clg-contract-verifier-v$Version-manifest.sig.json"
$ChecksumsPath = Join-Path $PackageDirectory "clg-contract-verifier-v$Version-checksums.txt"
foreach ($Path in @($ManifestPath, $SignaturePath, $ChecksumsPath, $BundlePath, $ReleasePublicKeyPath)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { Stop-Verification "missing file $Path" }
}

# Keep the exact manifest bytes: the detached signature authenticates these bytes, not a reprinted JSON value.
$ManifestBytes = [System.IO.File]::ReadAllBytes($ManifestPath)
$ManifestSha256 = (Get-FileHash -LiteralPath $ManifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
$Manifest = [System.Text.Encoding]::UTF8.GetString($ManifestBytes) | ConvertFrom-Json
$Signature = Get-Content -LiteralPath $SignaturePath -Raw | ConvertFrom-Json

if ($Manifest.format -cne 'clg.contract-verifier-distribution-manifest.v1' -or
    $Manifest.schema_version -ne 1 -or $Manifest.verifier_version -cne $Version) {
    Stop-Verification 'unsupported verifier manifest format, schema, or version'
}
if ($Signature.format -cne 'clg.contract-verifier-distribution-signature.v1' -or
    $Signature.schema_version -ne 1 -or $Signature.signature_format -cne 'ed25519' -or
    $Signature.key_id -cne $ExpectedDistributionKeyId -or
    $Signature.manifest_sha256 -cne "sha256:$ManifestSha256" -or
    $Signature.signature -notmatch '^[0-9a-f]{128}$') {
    Stop-Verification 'manifest signature metadata does not match the trusted release information'
}

# Build a DER SubjectPublicKeyInfo wrapper for the explicitly trusted raw Ed25519 public key.
$SpkiPrefix = [Convert]::FromHexString('302a300506032b6570032100')
$DistributionPublicKey = [Convert]::FromHexString($DistributionPublicKeyHex)
$SpkiBytes = New-Object byte[] ($SpkiPrefix.Length + $DistributionPublicKey.Length)
[Array]::Copy($SpkiPrefix, 0, $SpkiBytes, 0, $SpkiPrefix.Length)
[Array]::Copy($DistributionPublicKey, 0, $SpkiBytes, $SpkiPrefix.Length, $DistributionPublicKey.Length)
$SpkiPath = Join-Path $PackageDirectory '.distribution-public-key.der'
$SignatureBytesPath = Join-Path $PackageDirectory '.manifest.sig'
[System.IO.File]::WriteAllBytes($SpkiPath, $SpkiBytes)
[System.IO.File]::WriteAllBytes($SignatureBytesPath, [Convert]::FromHexString($Signature.signature))
try {
    & openssl pkeyutl -verify -pubin -inkey $SpkiPath -keyform DER -rawin -in $ManifestPath -sigfile $SignatureBytesPath
    if ($LASTEXITCODE -ne 0) { Stop-Verification 'manifest signature did not verify with the trusted distribution key' }
}
finally {
    Remove-Item -LiteralPath $SpkiPath, $SignatureBytesPath -Force -ErrorAction SilentlyContinue
}

$ExpectedArtifacts = @{
    'windows-x86_64' = @{ archive = "clg-contract-verifier-v$Version-windows-x86_64.zip"; executable = 'clg-contract-verifier.exe' }
    'linux-x86_64' = @{ archive = "clg-contract-verifier-v$Version-linux-x86_64.tar.gz"; executable = 'clg-contract-verifier' }
    'macos-x86_64' = @{ archive = "clg-contract-verifier-v$Version-macos-x86_64.tar.gz"; executable = 'clg-contract-verifier' }
}
$Artifacts = @($Manifest.artifacts)
if ($Artifacts.Count -ne 3) { Stop-Verification 'manifest does not bind exactly three platform artifacts' }
$ManifestPlatforms = @($Artifacts | ForEach-Object { $_.platform } | Sort-Object)
$LockedPlatforms = @($ExpectedArtifacts.Keys | Sort-Object)
if (($ManifestPlatforms -join "`n") -cne ($LockedPlatforms -join "`n")) {
    Stop-Verification 'manifest platform set is not the locked platform set'
}
foreach ($Artifact in $Artifacts) {
    if (-not $ExpectedArtifacts.ContainsKey($Artifact.platform)) { Stop-Verification "unsupported platform $($Artifact.platform)" }
    $Expected = $ExpectedArtifacts[$Artifact.platform]
    if ($Artifact.archive -cne $Expected.archive -or $Artifact.executable -cne $Expected.executable -or
        $Artifact.archive_sha256 -notmatch '^sha256:[0-9a-f]{64}$' -or
        $Artifact.executable_sha256 -notmatch '^sha256:[0-9a-f]{64}$') {
        Stop-Verification "manifest artifact for $($Artifact.platform) is not the locked form"
    }
    $ArchivePath = Join-Path $PackageDirectory $Artifact.archive
    if (-not (Test-Path -LiteralPath $ArchivePath -PathType Leaf)) { Stop-Verification "missing archive $($Artifact.archive)" }
    $ActualArchiveSha256 = (Get-FileHash -LiteralPath $ArchivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($Artifact.archive_sha256 -cne "sha256:$ActualArchiveSha256") { Stop-Verification "archive digest mismatch for $($Artifact.archive)" }
}

$ExpectedChecksumLines = @($Artifacts | Sort-Object archive | ForEach-Object {
    "$(($_.archive_sha256).Substring(7))  $($_.archive)"
})
$ActualChecksumText = [System.IO.File]::ReadAllText($ChecksumsPath)
$ExpectedChecksumText = ($ExpectedChecksumLines -join "`n") + "`n"
if ($ActualChecksumText -cne $ExpectedChecksumText) { Stop-Verification 'checksum listing does not match the signed manifest' }
$ExpectedPackageNames = @($Artifacts | ForEach-Object { $_.archive }) + @(
    (Split-Path -Leaf $ManifestPath),
    (Split-Path -Leaf $SignaturePath),
    (Split-Path -Leaf $ChecksumsPath)
) | Sort-Object
$ActualPackageNames = @(Get-ChildItem -LiteralPath $PackageDirectory -File | ForEach-Object Name | Sort-Object)
if (($ExpectedPackageNames -join "`n") -cne ($ActualPackageNames -join "`n")) {
    Stop-Verification 'package contains missing or unbound files'
}

$SelectedArtifact = $Artifacts | Where-Object { $_.platform -ceq $Platform }
if (@($SelectedArtifact).Count -ne 1) { Stop-Verification "manifest does not contain exactly one $Platform artifact" }
$ArchivePath = Join-Path $PackageDirectory $SelectedArtifact.archive
$ExtractDirectory = Join-Path $PackageDirectory "verified-$Platform"
if (Test-Path -LiteralPath $ExtractDirectory) { Stop-Verification "refusing to reuse extraction directory $ExtractDirectory" }
Expand-Archive -LiteralPath $ArchivePath -DestinationPath $ExtractDirectory
$VerifierPath = Join-Path $ExtractDirectory $SelectedArtifact.executable
if (-not (Test-Path -LiteralPath $VerifierPath -PathType Leaf)) { Stop-Verification 'archive has no locked verifier executable' }
$VerifierSha256 = (Get-FileHash -LiteralPath $VerifierPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($SelectedArtifact.executable_sha256 -cne "sha256:$VerifierSha256") { Stop-Verification 'extracted verifier digest mismatch' }

$VerificationOutput = & $VerifierPath --json-errors verify-release --bundle $BundlePath --pubkey $ReleasePublicKeyPath
$VerifierExitCode = $LASTEXITCODE
$VerificationJson = $VerificationOutput -join "`n"
$ResultPath = Join-Path $PackageDirectory 'contract-verification-result.json'
[System.IO.File]::WriteAllText($ResultPath, $VerificationJson, [System.Text.UTF8Encoding]::new($false))
if ($VerifierExitCode -ne 0) {
    $Failure = $VerificationJson | ConvertFrom-Json
    if ($Failure.errors.Count -ne 1 -or $Failure.errors[0].code -cne 'C140') {
        Stop-Verification 'verifier failed without the stable C140 rejection diagnostic'
    }
    Write-Host "REJECTED: C140 - $($Failure.errors[0].message)"
    exit 1
}
$Result = $VerificationJson | ConvertFrom-Json
if ($Result.format -cne 'clg.contract-release-verifier-result.v1' -or
    $Result.schema_version -ne 1 -or $Result.status -cne 'verified' -or
    $Result.target_profile -cne 'clg.evm-sepolia-pilot.v1' -or
    $Result.verifier_version -cne $Version) {
    Stop-Verification 'verifier success output is not the locked result schema'
}
Write-Host "VERIFIED: $($Result.bundle_sha256) $($Result.target_profile)"
Write-Host "Retained result: $ResultPath"
```

## Interpret the result

`VERIFIED` means that the exact signed verifier artifact checked the supplied bundle and its
bundle-local evidence using the explicit release public key. Retain the full package, bundle,
public key, trusted distribution key ID, and `contract-verification-result.json` together.
It does not claim that a testnet transaction occurred, that an RPC endpoint is honest, or that
the contract is suitable for mainnet or custody.

`REJECTED: C140` is the verifier's stable failure code for a release-bundle verification rejection.
The accompanying non-secret message identifies the failed check. Do not alter files or retry with
a different key merely to obtain success; preserve the inputs and report the mismatch to the
release owner. A `STOP:` message occurs before the verifier runs and means the distribution
package or its trust input was not acceptable. In either failure case, do not execute the
extracted binary for any other purpose.

## Product boundary

This verifier checks only `clg.contract-release-bundle.v1` evidence for the
`clg.evm-sepolia-pilot.v1` / `clg.evm-stateful-scalar.v1` pilot profile. It has no wallet, signer,
RPC, deployment, telemetry, plugin, or network-discovery behavior. It is part of a **locally
validated, testnet-only pilot** and must not be described as generic EVM support, remote-CI
certification, mainnet readiness, or custody safety. It does not validate a live chain result.

## References

- `docs/release/milestone_5_pilot_product_contract.md`
- `docs/release/milestone_5_pilot_incident_and_rollback.md`
- `docs/design/phase-43.0.1-verifier-delivery-form-lock.md`
- `docs/design/phase-43.0.2-verifier-compatibility-lock.md`
- `docs/design/phase-43.0.3-verifier-lifecycle-and-rollback-lock.md`
