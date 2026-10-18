param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$JournalDir,
    [string]$FixturePath = 'tests\fixtures\journal\real_explo_01.log',
    [string]$UiDir = 'ui\dist',
    [switch]$ShowWindow,
    [ValidateRange(1, 65535)][int]$Port = 47320,
    [ValidateRange(60, 86400)][int]$Seconds = 120,
    [ValidateScript({ $_ -gt 0 -and -not [double]::IsInfinity($_) })][double]$MaxMB = 150.0,
    [ValidateScript({ $_ -gt 0 -and -not [double]::IsInfinity($_) })][double]$MaxCpu = 1.0
)

$ErrorActionPreference = 'Stop'

function Get-AbsolutePath([string]$Path, [string]$Base) {
    if (-not [IO.Path]::IsPathRooted($Path)) { $Path = Join-Path $Base $Path }
    return [IO.Path]::GetFullPath($Path).TrimEnd('\', '/')
}

function Assert-NoReparsePoints([string]$Path) {
    $current = $Path
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            if ((Get-Item -LiteralPath $current -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Reparse point is unsafe for measurement cleanup: $current"
            }
        }
        $parent = Split-Path -Parent $current
        if ($parent -eq $current) { break }
        $current = $parent
    }
    if (Test-Path -LiteralPath $Path -PathType Container) {
        foreach ($item in Get-ChildItem -LiteralPath $Path -Force -Recurse) {
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
                throw "Reparse point inside cleanup directory: $($item.FullName)"
            }
        }
    }
}

function Assert-JournalPath([string]$Path, [string]$Workspace) {
    $root = Get-AbsolutePath $Workspace $Workspace
    $absolute = Get-AbsolutePath $Path $root
    if (-not $absolute.StartsWith($root + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'JournalDir must be strictly inside the workspace, never its root'
    }
    Assert-NoReparsePoints $absolute
    if (Test-Path -LiteralPath $absolute) {
        if (-not (Test-Path -LiteralPath $absolute -PathType Container)) { throw 'JournalDir must be a directory' }
        $marker = Join-Path $absolute '.tmc-footprint-journal'
        if (@(Get-ChildItem -LiteralPath $absolute -Force).Count -gt 0 -and -not (Test-Path -LiteralPath $marker -PathType Leaf)) {
            throw 'JournalDir is nonempty and was not created by this script; refusing to erase it'
        }
    }
    return $absolute
}

function Assert-TemporaryDataPath([string]$Path, [string]$CreatedPath, [string]$TempRoot) {
    $root = Get-AbsolutePath $TempRoot $TempRoot
    $absolute = Get-AbsolutePath $Path $root
    $created = Get-AbsolutePath $CreatedPath $root
    if (-not $absolute.Equals($created, [StringComparison]::OrdinalIgnoreCase) -or
        -not (Split-Path -Parent $absolute).Equals($root, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path -Leaf $absolute) -notmatch '^tmc-footprint-data-[0-9a-f]{32}$') {
        throw 'DataDir cleanup is restricted to the exact temporary GUID directory created by this run'
    }
    Assert-NoReparsePoints $absolute
}

function New-MeasurementConfig([string]$JournalPath, [string]$DataPath, [string]$UiPath, [int]$ServerPort) {
    # TOML literal strings preserve Windows backslashes, including spaces.
    foreach ($path in @($JournalPath, $DataPath, $UiPath)) {
        if ($path.Contains("'") -or $path.Contains("`n") -or $path.Contains("`r")) { throw 'Unsupported character in config path' }
    }
    return @"
journal_dir = '$JournalPath'
data_dir = '$DataPath'
ui_dir = '$UiPath'
language = "es"
online_services = false
auto_copy_next_system = false
alerts_enabled = true
theme_mode = "hud"

[server]
port = $ServerPort
lan = false
"@
}

function Assert-NoExistingTmc([object[]]$Processes, [string]$ExecutableName) {
    foreach ($process in $Processes) {
        if ($process.ProcessName -ieq $ExecutableName -or $process.ProcessName -match '^(TMC|tmc-desktop)$') {
            throw "TMC is already running (PID $($process.Id)); close it yourself before measuring"
        }
    }
}

function Get-TreeProcesses([uint32]$RootPid) {
    $all = @(Get-CimInstance Win32_Process)
    $queue = [Collections.Queue]::new()
    $queue.Enqueue($RootPid)
    $visited = [Collections.Generic.HashSet[uint32]]::new()
    while ($queue.Count -gt 0) {
        $current = [uint32]$queue.Dequeue()
        if (-not $visited.Add($current)) { continue }
        foreach ($process in $all) {
            if ($process.ProcessId -eq $current) { $process }
            if ($process.ParentProcessId -eq $current) { $queue.Enqueue([uint32]$process.ProcessId) }
        }
    }
}

function Assert-MeasurementTree([object[]]$Tree, [uint32]$RootPid) {
    if (-not @($Tree | Where-Object { $_.ProcessId -eq $RootPid }).Count) { throw 'Launched TMC process disappeared' }
    if (-not @($Tree | Where-Object { $_.Name -ieq 'msedgewebview2.exe' -and $_.CommandLine -match '--type=renderer(?:\s|$)' }).Count) {
        throw 'TMC has no WebView renderer; refusing a false low-memory result'
    }
}

function Assert-VisibleMeasurementWindow([long]$Handle, [bool]$Visible, [bool]$Minimized) {
    if ($Handle -eq 0 -or -not $Visible -or $Minimized) {
        throw 'TMC must have a visible, non-minimized window throughout measured samples'
    }
}

$repoRoot = Get-AbsolutePath (Split-Path -Parent $PSScriptRoot) $PSScriptRoot
$exePath = Get-AbsolutePath $Exe $repoRoot
$fixture = Get-AbsolutePath $FixturePath $repoRoot
$uiPath = Get-AbsolutePath $UiDir $repoRoot
$replayExe = Join-Path $repoRoot 'target\release\tmc-replay.exe'
# Validate before mutation, launching processes or stopping anything.
$journalPath = Assert-JournalPath $JournalDir $repoRoot
foreach ($file in @($exePath, $replayExe, $fixture, (Join-Path $uiPath 'index.html'))) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { throw "Required file not found; build it explicitly if needed: $file" }
}
Assert-NoExistingTmc -Processes @(Get-Process) -ExecutableName ([IO.Path]::GetFileNameWithoutExtension($exePath))
if (@([Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners() | Where-Object Port -eq $Port).Count) {
    throw "Port $Port is already occupied"
}

$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\', '/')
$tempDataDir = Join-Path $tempRoot ('tmc-footprint-data-' + [guid]::NewGuid().ToString('N'))
$tmcProc = $null
$replayProc = $null
$createdData = $false
$replayStopped = $false
try {
    if (Test-Path -LiteralPath $journalPath) {
        # Revalidate the resolved target immediately before every recursive removal.
        $journalPath = Assert-JournalPath $journalPath $repoRoot
        Remove-Item -LiteralPath $journalPath -Recurse -Force
    }
    New-Item -ItemType Directory -Path $journalPath -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $journalPath '.tmc-footprint-journal') -Value 'Synthetic replay only' -Encoding UTF8
    Assert-TemporaryDataPath $tempDataDir $tempDataDir $tempRoot
    New-Item -ItemType Directory -Path $tempDataDir | Out-Null
    $createdData = $true
    $tempConfigPath = Join-Path $tempDataDir 'config.toml'
    $config = New-MeasurementConfig $journalPath $tempDataDir $uiPath $Port
    [IO.File]::WriteAllText($tempConfigPath, $config, [Text.UTF8Encoding]::new($false))
    # Check again after setup to avoid accidentally connecting to another instance.
    Assert-NoExistingTmc -Processes @(Get-Process) -ExecutableName ([IO.Path]::GetFileNameWithoutExtension($exePath))
    if (-not ('TmcFootprint.NativeWindow' -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
namespace TmcFootprint {
    public static class NativeWindow {
        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool IsWindowVisible(IntPtr handle);
        [DllImport("user32.dll")]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool IsIconic(IntPtr handle);
    }
}
'@
    }
    $tmcArgs = '--config "{0}" --journal-dir "{1}" --ui-dir "{2}"' -f $tempConfigPath, $journalPath, $uiPath
    $windowStyle = if ($ShowWindow) { 'Normal' } else { 'Hidden' }
    $tmcProc = Start-Process -FilePath $exePath -ArgumentList $tmcArgs -PassThru -WindowStyle $windowStyle
    $replayArgs = 'play --from "{0}" --to "{1}" --speed 10000' -f $fixture, $journalPath
    $replayProc = Start-Process -FilePath $replayExe -ArgumentList $replayArgs -PassThru -WindowStyle Hidden
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $previousCpu = @{}
    $previousTime = 0.0
    $memorySamples = [Collections.Generic.List[double]]::new()
    $idleCpuSamples = [Collections.Generic.List[double]]::new()
    $cores = [Environment]::ProcessorCount
    while ($watch.Elapsed.TotalSeconds -lt $Seconds) {
        Start-Sleep -Seconds 2
        $now = $watch.Elapsed.TotalSeconds
        $tmcProc.Refresh()
        if ($tmcProc.HasExited) { throw "Launched TMC exited with code $($tmcProc.ExitCode)" }
        $replayProc.Refresh()
        if ($replayProc.HasExited -and -not $replayStopped -and $replayProc.ExitCode -ne 0) { throw "Replay failed with exit code $($replayProc.ExitCode)" }
        if ($now -ge ($Seconds - 32) -and -not $replayProc.HasExited) {
            Stop-Process -Id $replayProc.Id -Force
            $replayStopped = $true
        }
        $tree = @(Get-TreeProcesses ([uint32]$tmcProc.Id))
        # Allow the renderer to initialize during the first 20 seconds only.
        if ($now -ge 20) {
            Assert-MeasurementTree $tree ([uint32]$tmcProc.Id)
            $windowHandle = $tmcProc.MainWindowHandle
            Assert-VisibleMeasurementWindow -Handle $windowHandle.ToInt64() `
                -Visible ([TmcFootprint.NativeWindow]::IsWindowVisible($windowHandle)) `
                -Minimized ([TmcFootprint.NativeWindow]::IsIconic($windowHandle))
        }
        $privateBytes = 0L
        $currentCpu = @{}
        $deltaCpu = 0.0
        foreach ($entry in $tree) {
            $process = Get-Process -Id $entry.ProcessId -ErrorAction SilentlyContinue
            if ($null -eq $process) { continue }
            $privateBytes += $process.PrivateMemorySize64
            $cpu = $process.TotalProcessorTime.TotalSeconds
            $currentCpu[$process.Id] = $cpu
            if ($previousCpu.ContainsKey($process.Id)) { $deltaCpu += [math]::Max(0, $cpu - $previousCpu[$process.Id]) }
        }
        if ($now -ge 20 -and $privateBytes -le 0) { throw 'No private bytes measured; invalid process sample' }
        $cpuPct = if ($previousTime -gt 0) { $deltaCpu / (($now - $previousTime) * $cores) * 100 } else { 0.0 }
        $previousCpu = $currentCpu
        $previousTime = $now
        if ($now -ge 20) { $memorySamples.Add($privateBytes / 1MB) }
        if ($now -ge ($Seconds - 30)) { $idleCpuSamples.Add($cpuPct) }
    }
    if (-not $memorySamples.Count -or -not $idleCpuSamples.Count) { throw 'Insufficient valid measurement samples' }
    $peakMb = ($memorySamples | Measure-Object -Maximum).Maximum
    $idleCpu = ($idleCpuSamples | Measure-Object -Average).Average
    $passed = $peakMb -le $MaxMB -and $idleCpu -le $MaxCpu
    $result = if ($passed) { 'PASS' } else { 'FAIL' }
    Write-Output ('max_mb={0:F2} (limit={1}) avg_cpu_idle={2:F2}% (limit={3}%) {4}' -f $peakMb, $MaxMB, $idleCpu, $MaxCpu, $result)
    if (-not $passed) { throw 'Footprint exceeds configured limits' }
} finally {
    foreach ($owned in @($replayProc, $tmcProc)) {
        if ($null -ne $owned) {
            $owned.Refresh()
            if (-not $owned.HasExited) {
                Stop-Process -Id $owned.Id -Force -ErrorAction SilentlyContinue
                $owned.WaitForExit(5000) | Out-Null
            }
        }
    }
    if ($createdData -and (Test-Path -LiteralPath $tempDataDir)) {
        Assert-TemporaryDataPath $tempDataDir $tempDataDir $tempRoot
        Remove-Item -LiteralPath $tempDataDir -Recurse -Force
    }
}
