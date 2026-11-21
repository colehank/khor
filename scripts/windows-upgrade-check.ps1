# W6 · the upgrade and the rollback, at the level a person actually experiences:
# installing a newer build over an older one, and putting the older one back.
#
# The app's own signed updater is a different road and is NOT tested here: it
# verifies signed release artifacts against a trust root (crates/update), and
# signing is explicitly out of scope for this milestone (`ROADMAP.md` W6:
# "签名：只做验证设计，不签名"). What this covers is the installer road, which is
# what a person double-clicking a newer `setup.exe` goes through.
#
# Each step proves the installed build by reading the binary's own
# `ProductVersion`, and proves it runs by waiting for the window to answer on its
# transport. Nothing outside the install directory is touched, and the
# uninstaller does the removing.
#
# Usage:
#   pwsh -File scripts/windows-upgrade-check.ps1 `
#       -Base <0.2.0 setup.exe> -Newer <0.2.1 setup.exe>
param(
  [Parameter(Mandatory = $true)][string]$Base,
  [Parameter(Mandatory = $true)][string]$Newer,
  [string]$Store = 'F:\Temp\khor-upgrade-check\home',
  [int]$ReadySeconds = 60
)

$ErrorActionPreference = 'Stop'
$steps = @()
function Step([string]$name, [scriptblock]$body) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  try {
    $detail = $body.Invoke()
    $sw.Stop()
    $script:steps += [pscustomobject]@{ step = $name; ok = $true; ms = [math]::Round($sw.Elapsed.TotalMilliseconds); detail = $detail }
    Write-Host ("  PASS  {0,-26} {1,6}ms  {2}" -f $name, [math]::Round($sw.Elapsed.TotalMilliseconds), $detail)
  } catch {
    $sw.Stop()
    $script:steps += [pscustomobject]@{ step = $name; ok = $false; ms = [math]::Round($sw.Elapsed.TotalMilliseconds); detail = $_.Exception.Message }
    Write-Host ("  FAIL  {0,-26} {1}" -f $name, $_.Exception.Message)
  }
}

$local = [Environment]::GetFolderPath('LocalApplicationData')
$installDir = Join-Path $local 'Khor'
New-Item -ItemType Directory -Path $Store -Force | Out-Null
New-Item -ItemType Directory -Path (Split-Path -Parent $Store) -Force | Out-Null
$env:KHOR_HOME = $Store
$env:KHOR_RUNTIME_DIR = Join-Path (Split-Path -Parent $Store) 'runtime'

Write-Host "install dir: $installDir (only what the installer creates is removed)"
$before = @()
if (Test-Path -LiteralPath $installDir) {
  $before = @(Get-ChildItem -LiteralPath $installDir -Force | Select-Object -ExpandProperty Name)
}
Write-Host "already present (must survive): $(if ($before.Count) { $before -join ', ' } else { '(empty)' })"
Write-Host ''

function InstalledVersion {
  foreach ($name in 'Khor.exe', 'khor-gui.exe') {
    $p = Join-Path $installDir $name
    if (Test-Path -LiteralPath $p) {
      return [System.Diagnostics.FileVersionInfo]::GetVersionInfo($p).ProductVersion
    }
  }
  return $null
}

function InstallBundle([string]$setup, [string]$expect) {
  $p = Start-Process -FilePath $setup -ArgumentList '/S' -PassThru -Wait
  if ($p.ExitCode -ne 0) { throw "setup exit $($p.ExitCode)" }
  $got = InstalledVersion
  if ($got -ne $expect) { throw "installed version is $got, expected $expect" }
  "installed $got"
}

function ProveItRuns {
  $exe = @((Join-Path $installDir 'Khor.exe'), (Join-Path $installDir 'khor-gui.exe')) |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
  if (-not $exe) { throw 'no binary to run' }
  $out = Join-Path (Split-Path -Parent $Store) 'upgrade-gui.out'
  $err = Join-Path (Split-Path -Parent $Store) 'upgrade-gui.err'
  $proc = Start-Process -FilePath $exe -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $ready = $false
  while ($sw.Elapsed.TotalSeconds -lt $ReadySeconds) {
    if ($proc.HasExited) { throw "exited at $($proc.ExitCode) before answering" }
    try {
      $r = Invoke-WebRequest -Uri 'http://127.0.0.1:1432/lock_state' -Method Post -Body '{}' `
        -ContentType 'application/json' -UseBasicParsing -TimeoutSec 3
      if ($r.StatusCode -eq 200) { $ready = $true; break }
    } catch { Start-Sleep -Milliseconds 150 }
  }
  $sw.Stop()
  Get-Process | Where-Object { $_.ProcessName -match '^khor' } | ForEach-Object {
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
  }
  Start-Sleep -Milliseconds 500
  if (-not $ready) { throw "no answer within ${ReadySeconds}s" }
  "answered in $([math]::Round($sw.Elapsed.TotalMilliseconds))ms"
}

Write-Host 'upgrade'
Step 'install base'      { InstallBundle $Base  '0.2.0' }
Step 'base runs'         { ProveItRuns }
Step 'upgrade to newer'  { InstallBundle $Newer '0.2.1' }
Step 'newer runs'        { ProveItRuns }

Write-Host 'rollback'
Step 'roll back to base' { InstallBundle $Base  '0.2.0' }
Step 'rolled-back runs'  { ProveItRuns }
Step 'uninstall' {
  $u = Get-ChildItem -LiteralPath $installDir -Filter 'uninstall*.exe' | Select-Object -First 1
  if (-not $u) { throw 'no uninstaller' }
  $p = Start-Process -FilePath $u.FullName -ArgumentList '/S' -PassThru -Wait
  if ($p.ExitCode -ne 0) { throw "uninstaller exit $($p.ExitCode)" }
  'uninstaller exit 0'
}
Step 'clean' {
  $deadline = (Get-Date).AddSeconds(40)
  while ((Get-Date) -lt $deadline) {
    if (-not (Test-Path -LiteralPath $installDir)) { return "$installDir is gone" }
    $now = @(Get-ChildItem -LiteralPath $installDir -Force | Select-Object -ExpandProperty Name)
    $extra = @($now | Where-Object { $before -notcontains $_ })
    if ($extra.Count -eq 0) { return "installer's files gone; kept: $(if ($now.Count) { $now -join ', ' } else { '(nothing)' })" }
    Start-Sleep -Milliseconds 500
  }
  throw "still installed: $(@(Get-ChildItem -LiteralPath $installDir -Force | Select-Object -ExpandProperty Name) -join ', ')"
}

$out = 'F:\Temp\khor-upgrade-check\report.json'
$steps | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $out -Encoding utf8
Write-Host ''
$failed = @($steps | Where-Object { -not $_.ok })
Write-Host ("{0}/{1} steps passed · report: {2}" -f ($steps.Count - $failed.Count), $steps.Count, $out)
Write-Host ''
Write-Host 'NOT_TESTED: the app'"'"'s own signed updater (crates/update). It verifies signed'
Write-Host 'artifacts against a trust root; signing is out of scope for W6.'
if ($failed.Count) { exit 1 }
