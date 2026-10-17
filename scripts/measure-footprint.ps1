param(
    [Parameter(Mandatory = $true)]
    [string]$Exe,

    [Parameter(Mandatory = $true)]
    [string]$JournalDir,

    [int]$Seconds = 120,

    [double]$MaxMB = 150.0,

    [double]$MaxCpu = 1.0
)

$ErrorActionPreference = "Stop"
$env:PATH = "C:\Users\danik\.cargo\bin;$env:PATH"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exePath = if ([System.IO.Path]::IsPathRooted($Exe)) { $Exe } else { Join-Path $repoRoot $Exe }
if (-not (Test-Path $exePath)) {
    Write-Error "No se encontró el ejecutable en: $exePath"
    exit 1
}

$replayExe = Join-Path $repoRoot "target\release\tmc-replay.exe"
if (-not (Test-Path $replayExe)) {
    Push-Location $repoRoot
    try {
        cargo build --release -p tmc-replay --offline | Out-Null
    } finally {
        Pop-Location
    }
}

if (Test-Path $JournalDir) {
    Remove-Item -Recurse -Force $JournalDir
}
New-Item -ItemType Directory -Force -Path $JournalDir | Out-Null

$tempDataDir = Join-Path $env:TEMP ("tmc-footprint-data-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tempDataDir | Out-Null
$tempConfigPath = Join-Path $tempDataDir "config.toml"
$escapedJournal = $JournalDir.Replace('\', '\\')
$escapedData = $tempDataDir.Replace('\', '\\')
@"
journal_dir = "$escapedJournal"
data_dir = "$escapedData"
language = "es"
auto_copy_next_system = true
alerts_enabled = true
theme_mode = "hud"

[server]
port = 47320
lan_enabled = false
"@ | Set-Content -Path $tempConfigPath -Encoding UTF8

$fixturePath = Join-Path $repoRoot "tests\fixtures\journal\real_01.log"
$cpuCores = [Environment]::ProcessorCount

function Get-TreeProcesses([uint32]$rootPid) {
    $allCim = Get-CimInstance Win32_Process
    $byParent = @{}
    foreach ($p in $allCim) {
        if (-not $byParent.ContainsKey($p.ParentProcessId)) {
            $byParent[$p.ParentProcessId] = @()
        }
        $byParent[$p.ParentProcessId] += $p
    }

    $pids = [System.Collections.Generic.List[uint32]]::new()
    $queue = [System.Collections.Queue]::new()
    $queue.Enqueue($rootPid)

    while ($queue.Count -gt 0) {
        $current = [uint32]$queue.Dequeue()
        if (-not $pids.Contains($current)) {
            $pids.Add($current)
        }
        if ($byParent.ContainsKey($current)) {
            foreach ($child in $byParent[$current]) {
                if ($child.Name -eq "msedgewebview2.exe" -or $child.ProcessId -eq $rootPid) {
                    $queue.Enqueue([uint32]$child.ProcessId)
                }
            }
        }
    }

    return $pids
}

$tmcProc = $null
$replayProc = $null

try {
    $tmcProc = Start-Process -FilePath $exePath -ArgumentList "--config", $tempConfigPath, "--journal-dir", $JournalDir -PassThru
    $replayProc = Start-Process -FilePath $replayExe -ArgumentList "play", "--from", $fixturePath, "--to", $JournalDir, "--speed", "5" -PassThru -WindowStyle Hidden

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $prevCpuByPid = @{}
    $prevTimeSec = 0.0

    $memorySamples = [System.Collections.Generic.List[double]]::new()
    $idleCpuSamples = [System.Collections.Generic.List[double]]::new()

    while ($sw.Elapsed.TotalSeconds -lt $Seconds) {
        Start-Sleep -Seconds 2
        $nowSec = $sw.Elapsed.TotalSeconds

        if ($nowSec -ge ($Seconds - 32) -and $null -ne $replayProc -and (-not $replayProc.HasExited)) {
            Stop-Process -Id $replayProc.Id -Force -ErrorAction SilentlyContinue
        }

        $treePids = Get-TreeProcesses -rootPid ([uint32]$tmcProc.Id)
        $totalPrivateBytes = 0L
        $currCpuByPid = @{}
        $deltaCpuSec = 0.0

        foreach ($procId in $treePids) {
            $gp = Get-Process -Id $procId -ErrorAction SilentlyContinue
            if ($null -ne $gp) {
                $totalPrivateBytes += $gp.PrivateMemorySize64
                $cpuSec = $gp.TotalProcessorTime.TotalSeconds
                $currCpuByPid[$procId] = $cpuSec
                if ($prevCpuByPid.ContainsKey($procId)) {
                    $diff = $cpuSec - $prevCpuByPid[$procId]
                    if ($diff -gt 0.0) {
                        $deltaCpuSec += $diff
                    }
                }
            }
        }

        $dt = $nowSec - $prevTimeSec
        $cpuPct = if ($dt -gt 0.0 -and $prevTimeSec -gt 0.0) {
            ($deltaCpuSec / ($dt * $cpuCores)) * 100.0
        } else {
            0.0
        }

        $prevCpuByPid = $currCpuByPid
        $prevTimeSec = $nowSec

        $mb = $totalPrivateBytes / 1MB

        if ($nowSec -ge 20.0) {
            $memorySamples.Add($mb)
        }
        if ($nowSec -ge ($Seconds - 30)) {
            $idleCpuSamples.Add($cpuPct)
        }
    }

    $peakMb = if ($memorySamples.Count -gt 0) {
        ($memorySamples | Measure-Object -Maximum).Maximum
    } else {
        0.0
    }

    $avgCpuIdle = if ($idleCpuSamples.Count -gt 0) {
        ($idleCpuSamples | Measure-Object -Average).Average
    } else {
        0.0
    }

    $maxMbRounded = [math]::Round($peakMb, 2)
    $avgCpuRounded = [math]::Round($avgCpuIdle, 2)
    $passed = ($peakMb -le $MaxMB) -and ($avgCpuIdle -le $MaxCpu)
    $status = if ($passed) { "PASS" } else { "FAIL" }

    Write-Output ("max_mb={0} (limit={1}) avg_cpu_idle={2}% (limit={3}%) {4}" -f $maxMbRounded, $MaxMB, $avgCpuRounded, $MaxCpu, $status)

    if (-not $passed) {
        exit 1
    }
    exit 0
} finally {
    if ($null -ne $replayProc -and (-not $replayProc.HasExited)) {
        Stop-Process -Id $replayProc.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $tmcProc -and (-not $tmcProc.HasExited)) {
        Stop-Process -Id $tmcProc.Id -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path $tempDataDir) {
        Remove-Item -Recurse -Force $tempDataDir -ErrorAction SilentlyContinue
    }
}
