$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot '..\measure-footprint.ps1'
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count) { throw ($parseErrors | Out-String) }
# Load function definitions only: never launch TMC, replay, builds or measurement.
foreach ($function in $ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $false)) {
    . ([scriptblock]::Create($function.Extent.Text))
}
function Assert-Rejected([scriptblock]$Action) {
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw 'Expected unsafe input to be rejected' }
}
$root = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
$safe = Assert-JournalPath -Path '.tmp\footprint-test' -Workspace $root
if ($safe -ne (Join-Path $root '.tmp\footprint-test')) { throw 'Incorrect absolute path' }
Assert-Rejected { Assert-JournalPath -Path $root -Workspace $root }
Assert-Rejected { Assert-JournalPath -Path '..\outside' -Workspace $root }
Assert-Rejected { Assert-JournalPath -Path ($root + '-other\journal') -Workspace $root }
Assert-Rejected { Assert-JournalPath -Path 'tests\fixtures\journal' -Workspace $root }
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$created = Join-Path $tempRoot ('tmc-footprint-data-' + [guid]::NewGuid().ToString('N'))
Assert-TemporaryDataPath -Path $created -CreatedPath $created -TempRoot $tempRoot
Assert-Rejected { Assert-TemporaryDataPath -Path $tempRoot -CreatedPath $created -TempRoot $tempRoot }
Assert-Rejected { Assert-TemporaryDataPath -Path $root -CreatedPath $created -TempRoot $tempRoot }
Assert-Rejected { Assert-TemporaryDataPath -Path $created -CreatedPath ($created + '-other') -TempRoot $tempRoot }
$config = New-MeasurementConfig -JournalPath $safe -DataPath $created -UiPath (Join-Path $root 'ui\dist') -ServerPort 47321
foreach ($expected in @('online_services = false', 'auto_copy_next_system = false', 'lan = false', 'port = 47321', 'ui_dir = ')) {
    if (-not $config.Contains($expected)) { throw "Missing config setting: $expected" }
}
if ($config.Contains('lan_enabled')) { throw 'Incorrect LAN configuration key' }
Assert-Rejected { Assert-NoExistingTmc -Processes @([pscustomobject]@{ProcessName='TMC'; Id=42}) -ExecutableName 'custom-tmc' }
Assert-Rejected { Assert-NoExistingTmc -Processes @([pscustomobject]@{ProcessName='custom-tmc'; Id=42}) -ExecutableName 'custom-tmc' }
Assert-NoExistingTmc -Processes @([pscustomobject]@{ProcessName='unrelated'; Id=43}) -ExecutableName 'TMC'
$main = [pscustomobject]@{ProcessId=42; Name='TMC.exe'; CommandLine='TMC.exe'}
$renderer = [pscustomobject]@{ProcessId=43; Name='msedgewebview2.exe'; CommandLine='webview --type=renderer --other'}
Assert-MeasurementTree -Tree @($main, $renderer) -RootPid 42
Assert-Rejected { Assert-MeasurementTree -Tree @($main) -RootPid 42 }
Assert-Rejected { Assert-MeasurementTree -Tree @($renderer) -RootPid 42 }
Assert-Rejected { Assert-MeasurementTree -Tree @($main, [pscustomobject]@{ProcessId=43; Name='msedgewebview2.exe'; CommandLine='webview --type=gpu-process'}) -RootPid 42 }
Assert-VisibleMeasurementWindow -Handle 42 -Visible $true -Minimized $false
Assert-Rejected { Assert-VisibleMeasurementWindow -Handle 0 -Visible $true -Minimized $false }
Assert-Rejected { Assert-VisibleMeasurementWindow -Handle 42 -Visible $false -Minimized $false }
Assert-Rejected { Assert-VisibleMeasurementWindow -Handle 42 -Visible $true -Minimized $true }
# Execute only parameter binding; the actual script body is never invoked.
$bindOnly = [scriptblock]::Create($ast.ParamBlock.Extent.Text + "`n'parameters-valid'")
Assert-Rejected { & $bindOnly -Exe 'unused' -JournalDir 'unused' -Port 0 }
Assert-Rejected { & $bindOnly -Exe 'unused' -JournalDir 'unused' -Port 65536 }
Assert-Rejected { & $bindOnly -Exe 'unused' -JournalDir 'unused' -Seconds 59 }
Assert-Rejected { & $bindOnly -Exe 'unused' -JournalDir 'unused' -MaxMB ([double]::NaN) }
Assert-Rejected { & $bindOnly -Exe 'unused' -JournalDir 'unused' -MaxCpu ([double]::PositiveInfinity) }
if ((& $bindOnly -Exe 'unused' -JournalDir 'unused') -ne 'parameters-valid') { throw 'Defaults rejected' }
if ((& $bindOnly -Exe 'unused' -JournalDir 'unused' -ShowWindow) -ne 'parameters-valid') { throw 'Visible layout measurement rejected' }
Write-Output 'PASS: syntax, defaults, parameter bounds, safe paths, isolated config and live renderer requirements'
