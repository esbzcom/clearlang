# [LOCAL / NO NETWORK]
# Run from the approved documentation checkout. The copied fixture directory is
# intentionally retained for independent inspection after the assertions pass.
$ErrorActionPreference = 'Stop'

$Repo = (Get-Location).Path
$FixtureRoot = Join-Path $Repo 'docs\fixtures\phase-53.4'
$Independent = Join-Path ([System.IO.Path]::GetTempPath()) ('clg-independent-user-docs-' + [guid]::NewGuid())
$CopiedFixture = Join-Path $Independent 'independent-user-documentation-fixtures.json'
$CopiedVerifier = Join-Path $Independent 'production-verifier-offline-walkthrough.md'
$CopiedOperator = Join-Path $Independent 'production-safe-registry-operator-walkthrough.md'

function Stop-Fixture([string] $Message) {
    throw "STOP: $Message"
}

foreach ($Path in @(
    (Join-Path $FixtureRoot 'independent-user-documentation-fixtures.json'),
    (Join-Path $Repo 'docs\release\production-verifier-offline-walkthrough.md'),
    (Join-Path $Repo 'docs\release\production-safe-registry-operator-walkthrough.md')
)) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-Fixture "missing fixture input $Path"
    }
}

New-Item -ItemType Directory -Path $Independent -ErrorAction Stop | Out-Null
Copy-Item -LiteralPath (Join-Path $FixtureRoot 'independent-user-documentation-fixtures.json') -Destination $CopiedFixture
Copy-Item -LiteralPath (Join-Path $Repo 'docs\release\production-verifier-offline-walkthrough.md') -Destination $CopiedVerifier
Copy-Item -LiteralPath (Join-Path $Repo 'docs\release\production-safe-registry-operator-walkthrough.md') -Destination $CopiedOperator

$Fixture = Get-Content -LiteralPath $CopiedFixture -Raw | ConvertFrom-Json
if ($Fixture.format -cne 'clg.independent-user-documentation-fixture.v1' -or
    $Fixture.schema_version -ne 1 -or $Fixture.scope -cne 'offline_only') {
    Stop-Fixture 'fixture schema is not the locked offline form'
}
if ($Fixture.verifier.required_future_result.target_profile -cne 'clg.evm-ethereum-mainnet-safe-registry.v1' -or
    $Fixture.verifier.required_future_result.chain_id -cne '1' -or
    $Fixture.verifier.current_status -cne 'blocked_unsupported_profile_version') {
    Stop-Fixture 'verifier profile, chain identity, or current stop condition drifted'
}
if ($Fixture.operator.current_precheck_status -cne 'blocked' -or
    $Fixture.operator.current_precheck_diagnostic -cne 'M6D005') {
    Stop-Fixture 'operator precheck outcome drifted'
}
if ($Fixture.retention.minimum_years -ne 7 -or -not $Fixture.retention.secret_free_only) {
    Stop-Fixture 'retention policy drifted'
}

$Verifier = Get-Content -LiteralPath $CopiedVerifier -Raw
$Operator = Get-Content -LiteralPath $CopiedOperator -Raw
foreach ($Required in @('source checkout', 'private key', 'unsupported profile/version', 'does not authorize')) {
    if ($Verifier.IndexOf($Required, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
        Stop-Fixture "verifier walkthrough is missing $Required"
    }
}
foreach ($Required in @('M6D005', 'Phase 52.5 public-testnet soak', 'at least seven years', 'No current document authorizes a mainnet action')) {
    if ($Operator.IndexOf($Required, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
        Stop-Fixture "operator walkthrough is missing $Required"
    }
}

$Files = @(Get-ChildItem -LiteralPath $Independent -File | ForEach-Object Name | Sort-Object)
$ExpectedFiles = @(
    'independent-user-documentation-fixtures.json',
    'production-safe-registry-operator-walkthrough.md',
    'production-verifier-offline-walkthrough.md'
) | Sort-Object
if (($Files -join "`n") -cne ($ExpectedFiles -join "`n")) {
    Stop-Fixture 'independent directory contains an unexpected documentation artifact set'
}

Write-Output "independent-user documentation fixture validated offline: $Independent"
