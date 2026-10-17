# Run from the approved ClearLang checkout root.
$Repo = (Get-Location).Path
$Clean = Join-Path ([System.IO.Path]::GetTempPath()) ("clg-pilot-precheck-" + [guid]::NewGuid())
$Report = Join-Path $Clean 'pilot-precheck-full.json'
$ProjectionPath = Join-Path $Repo 'docs/fixtures/phase-40.2.3/pilot-precheck-digest-projection.v2.json'
New-Item -ItemType Directory -Path $Clean -ErrorAction Stop | Out-Null

$env:CARGO_NET_OFFLINE = 'true'
try {
    cargo run --offline --locked -p xtask -- pilot-precheck --report $Report
    if ($LASTEXITCODE -ne 0) { throw 'STOP: pilot-precheck failed; retain its report and do not validate it as acceptance evidence' }
}
finally {
    Remove-Item Env:CARGO_NET_OFFLINE -ErrorAction SilentlyContinue
}

$ReportJson = Get-Content $Report -Raw | ConvertFrom-Json
$Projection = Get-Content $ProjectionPath -Raw | ConvertFrom-Json
if ($ReportJson.schema_version -ne $Projection.report_schema_version -or
    $ReportJson.kind -cne $Projection.report_kind -or
    $ReportJson.scope -cne $Projection.scope -or
    $ReportJson.overall_status -cne 'passed') {
    throw 'STOP: pilot-precheck report schema, scope, or status is not accepted'
}
if ((@($ReportJson.gate_order) -join "`n") -cne (@($Projection.gate_order) -join "`n")) {
    throw 'STOP: pilot-precheck gate order differs from the locked projection'
}
foreach ($ExpectedGate in $Projection.gates) {
    $ActualGate = @($ReportJson.gates | Where-Object { $_.id -ceq $ExpectedGate.id })
    if ($ActualGate.Count -ne 1 -or $ActualGate[0].status -cne 'passed') {
        throw "STOP: gate $($ExpectedGate.id) is absent, duplicated, or non-passing"
    }
    foreach ($Kind in 'input_digests', 'evidence_digests') {
        $Actual = @($ActualGate[0].$Kind | ForEach-Object { "$($_.id):$($_.sha256)" })
        $Expected = @($ExpectedGate.$Kind | ForEach-Object { "$($_.id):$($_.sha256)" })
        if ((@($Actual) -join "`n") -cne (@($Expected) -join "`n")) {
            throw "STOP: gate $($ExpectedGate.id) has a substituted $Kind projection"
        }
    }
}
if ((Get-ChildItem -LiteralPath $Clean -Force | Measure-Object).Count -ne 1) {
    throw 'STOP: clean report directory contains unexpected entries'
}
Write-Output "pilot-precheck reproduction validated: $Report"
