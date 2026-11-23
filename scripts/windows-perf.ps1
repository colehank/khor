#!/usr/bin/env pwsh
# W7 · the Windows performance matrix (docs/windows-migration/APPROVAL-PACKAGE.md §5).
#
# Measures the load profile that document names, on this machine only: CLI cold
# start, session creation, the session list, a file scan, a fixed file copy,
# doctor, and GUI cold start to the moment its own transport answers.
# Reports median / P95 / peak per metric with the warmup runs dropped, the way
# §5.2 asks.
#
# **What this does NOT do by itself**: compare against Linux/macOS. §5.2 wants
# those as the baseline; run this same script's metric list on the other host
# (`scripts/windows-perf.sh` does the same seven there) and put the two columns
# side by side. Saying "within 30%" without the other column would be the exact
# thing §6 forbids.
#
# Usage:
#   pwsh -File scripts/windows-perf.ps1 -Khor <khor.exe> -Gui <khor-gui.exe> `
#        -Store <scratch store> -Runs 5 -Warmup 2 -Out <json>
param(
  [Parameter(Mandatory = $true)][string]$Khor,
  [string]$Gui,
  # **Not `-Home`.** `$HOME` is a read-only automatic variable; a parameter of
  # that name fails to bind ("Cannot overwrite variable Home because it is
  # read-only or constant"). Same shape as the accident this repository's
  # delete guard exists for, so the name is avoided everywhere.
  [string]$Store = 'F:\Temp\khor-perf\home',
  [int]$Runs = 5,
  [int]$Warmup = 2,
  [int]$GuiReadySeconds = 60,
  [string]$Out = 'F:\Temp\khor-perf\perf.json'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

foreach ($needed in @($Khor)) {
  if (-not (Test-Path -LiteralPath $needed)) { throw "not found: $needed" }
}
$root = Split-Path -Parent $Store
New-Item -ItemType Directory -Path $Store -Force | Out-Null
New-Item -ItemType Directory -Path $root -Force | Out-Null
New-Item -ItemType Directory -Path (Split-Path -Parent $Out) -Force | Out-Null
$env:KHOR_HOME = $Store
$env:KHOR_RUNTIME_DIR = Join-Path $root 'runtime'

# A fixed file set for scan/copy, made once so every run reads the same bytes.
$fixture = Join-Path $root 'perf-files'
if (-not (Test-Path -LiteralPath $fixture)) {
  New-Item -ItemType Directory -Path $fixture -Force | Out-Null
  for ($i = 0; $i -lt 40; $i++) {
    $text = ("khor perf line {0}`n" -f $i) * 400
    Set-Content -LiteralPath (Join-Path $fixture ("f{0:D2}.txt" -f $i)) -Value $text -NoNewline
  }
}

function Assert-LastExit([string]$what) {
  if ($LASTEXITCODE -ne 0) { throw "$what exited $LASTEXITCODE" }
}

function Measure-One([scriptblock]$body) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $body.Invoke() | Out-Null
  $sw.Stop()
  [pscustomobject]@{ ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1) }
}

# Launch the window and wait until its own transport answers: that is the
# moment the page can actually ask for anything, which is what "interactive"
# has to mean here. A window that draws while every command hangs is not ready.
function Measure-GuiColdStart([string]$exe, [int]$port) {
  $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = ''
  $out = Join-Path $root 'gui.out'
  $err = Join-Path $root 'gui.err'
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $exe -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
  try {
    $ready = $false
    while ($sw.Elapsed.TotalSeconds -lt $GuiReadySeconds) {
      try {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:$port/lock_state" -Method Post -Body '{}' `
          -ContentType 'application/json' -UseBasicParsing -TimeoutSec 3
        if ($r.StatusCode -eq 200) { $ready = $true; break }
      } catch { Start-Sleep -Milliseconds 120 }
    }
    $sw.Stop()
    if (-not $ready) { return [pscustomobject]@{ ms = $null; note = "not ready in ${GuiReadySeconds}s" } }
    $peak = 0
    try { $peak = [math]::Round((Get-Process -Id $proc.Id).PeakWorkingSet64 / 1MB, 1) } catch { }
    return [pscustomobject]@{ ms = [math]::Round($sw.Elapsed.TotalMilliseconds, 1); peakMb = $peak }
  } finally {
    Get-Process | Where-Object { $_.ProcessName -match '^khor' } | ForEach-Object {
      Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 400
  }
}

$metrics = [ordered]@{}
function Add-Metric([string]$name, [scriptblock]$body) {
  $samples = New-Object System.Collections.ArrayList
  $total = $Warmup + $Runs
  for ($i = 0; $i -lt $total; $i++) {
    try { $r = $body.Invoke() } catch {
      $metrics[$name] = [pscustomobject]@{ status = 'FAILED'; error = $_.Exception.Message }
      Write-Host ("  {0,-22} FAILED: {1}" -f $name, $_.Exception.Message)
      return
    }
    if ($i -ge $Warmup -and $null -ne $r.ms) { [void]$samples.Add($r.ms) }
  }
  if ($samples.Count -eq 0) {
    $metrics[$name] = [pscustomobject]@{ status = 'NO_SAMPLES' }
    Write-Host ("  {0,-22} no samples" -f $name)
    return
  }
  $sorted = $samples | Sort-Object
  $median = $sorted[[int][math]::Floor($sorted.Count / 2)]
  $p95 = $sorted[[int][math]::Min($sorted.Count - 1, [math]::Ceiling($sorted.Count * 0.95) - 1)]
  $peak = ($sorted | Select-Object -Last 1)
  $metrics[$name] = [pscustomobject]@{
    status = 'OK'; unit = 'ms'; runs = $samples.Count
    median = $median; p95 = $p95; peak = $peak; samples = @($samples)
  }
  Write-Host ("  {0,-22} median {1,8} ms   p95 {2,8}   peak {3,8}" -f $name, $median, $p95, $peak)
}

Write-Host "khor perf · $Khor"
Write-Host "store $Store · runs $Runs (+$Warmup warmup)"

Add-Metric 'cli_version'  { Measure-One { & $Khor version 2>&1 | Out-Null; Assert-LastExit 'khor version' } }
Add-Metric 'cli_doctor'   { Measure-One { & $Khor doctor --json 2>&1 | Out-Null; Assert-LastExit 'khor doctor' } }
# **A command that exits by itself.** `khor open -d -- cmd.exe` leaves a shell
# waiting for input with no tty behind it, so the call can sit there and the run
# measures nothing; `/c exit` gives the same session-creation path and returns.
Add-Metric 'session_create' { Measure-One { & $Khor open -d -- cmd.exe /c exit 2>&1 | Out-Null; Assert-LastExit 'khor open' } }
Add-Metric 'sessions_list' { Measure-One { & $Khor sessions 2>&1 | Out-Null; Assert-LastExit 'khor sessions' } }
Add-Metric 'file_scan'    { Measure-One { & $Khor ls $env:COMPUTERNAME $fixture 2>&1 | Out-Null; Assert-LastExit 'khor ls' } }
Add-Metric 'file_copy' {
  Measure-One {
    $to = Join-Path $root 'perf-copy'
    if (Test-Path -LiteralPath $to) { Remove-Item -LiteralPath $to -Recurse -Force }
    Copy-Item -LiteralPath $fixture -Destination $to -Recurse -Force
  }
}
if ($Gui) {
  Add-Metric 'gui_cold_start' { Measure-GuiColdStart $Gui 1432 }
}

$metrics | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $Out -Encoding utf8
Write-Host ''
Write-Host "written: $Out"
Write-Host ''
Write-Host 'NOTE: the Linux/macOS column is not measured here. Run scripts/windows-perf.sh'
Write-Host '      on that host and put the two side by side; §5.2 s 30%/100% judgement'
Write-Host '      needs both, and these numbers alone are not a comparison.'
