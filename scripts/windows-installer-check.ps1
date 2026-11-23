# W6 · does the installer actually install, start and uninstall?
#
# Walks one bundle through the gate the roadmap asks for: install silently,
# check the layout it laid down, start the app and wait until its own transport
# answers (a window that draws while every command hangs is not "started"),
# then uninstall and check that what the installer created is gone.
#
# **It only removes what the installer itself created** — the uninstaller is the
# thing doing the removing. Because the target directory can already hold things
# that were not installed (a development run leaves `runtime\` behind), the
# script records what was there BEFORE and asserts the uninstaller left exactly
# that, instead of demanding the whole directory vanish.
#
# Usage:
#   pwsh -File scripts/windows-installer-check.ps1 -Installer <path-to-setup.exe|-msi>
param(
  [Parameter(Mandatory = $true)][string]$Installer,
  # **Not a parameter called `-Home`**: `$HOME` is a read-only automatic
  # variable, and a parameter of that name fails to bind ("Cannot overwrite
  # variable Home because it is read-only or constant"). That is the same shape
  # as the accident this repository's delete guard exists for: a script assigned
  # `$home`, the assignment silently did nothing, and the delete it guarded hit
  # the real profile.
  [string]$Store = 'F:\Temp\khor-install-check\home',
  [int]$ReadySeconds = 60,
  [switch]$KeepInstalled
)

$ErrorActionPreference = 'Stop'
$report = [ordered]@{ installer = $Installer; steps = @() }

function Step([string]$name, [scriptblock]$body) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  try {
    $detail = $body.Invoke()
    $sw.Stop()
    $script:report.steps += [pscustomobject]@{
      step = $name; ok = $true; ms = [math]::Round($sw.Elapsed.TotalMilliseconds); detail = $detail
    }
    Write-Host ("  PASS  {0,-22} {1}ms  {2}" -f $name, [math]::Round($sw.Elapsed.TotalMilliseconds), $detail)
  } catch {
    $sw.Stop()
    $script:report.steps += [pscustomobject]@{
      step = $name; ok = $false; ms = [math]::Round($sw.Elapsed.TotalMilliseconds); detail = $_.Exception.Message
    }
    Write-Host ("  FAIL  {0,-22} {1}" -f $name, $_.Exception.Message)
  }
}

if (-not (Test-Path -LiteralPath $Installer)) { throw "not found: $Installer" }
$isMsi = $Installer -like '*.msi'
$local = [Environment]::GetFolderPath('LocalApplicationData')
$installDir = Join-Path $local 'Khor'
Write-Host "installer : $Installer"
Write-Host "install dir the uninstaller will remove: $installDir"
Write-Host "nothing outside that directory is touched."
Write-Host ''

# What the directory holds before anything is installed. Anything here is not
# the installer's and must survive the uninstall.
$before = @()
if (Test-Path -LiteralPath $installDir) {
  $before = @(Get-ChildItem -LiteralPath $installDir -Force | Select-Object -ExpandProperty Name)
}
Write-Host "already present (must survive): $(if ($before.Count) { $before -join ', ' } else { '(empty)' })"
Write-Host ''

New-Item -ItemType Directory -Path $Store -Force | Out-Null
New-Item -ItemType Directory -Path (Split-Path -Parent $Store) -Force | Out-Null
$env:KHOR_HOME = $Store
$env:KHOR_RUNTIME_DIR = Join-Path (Split-Path -Parent $Store) 'runtime'

Step 'install' {
  if ($isMsi) {
    $p = Start-Process -FilePath 'msiexec.exe' -ArgumentList @('/i', "`"$Installer`"", '/qn', '/norestart') -PassThru -Wait
    if ($p.ExitCode -ne 0) { throw "msiexec exit $($p.ExitCode)" }
    'msiexec exit 0'
  } else {
    $p = Start-Process -FilePath $Installer -ArgumentList '/S' -PassThru -Wait
    if ($p.ExitCode -ne 0) { throw "setup exit $($p.ExitCode)" }
    'setup exit 0'
  }
}

Step 'layout' {
  # **The installed binary is `khor-gui.exe`, not `Khor.exe`.** The cargo bin is
  # named after the crate, and Tauri only renames it when `mainBinaryName` is set
  # — which expects the cargo bin to carry that name too, so changing it would
  # ripple through every build command and the existing W3/W4 evidence. The
  # display name a person sees (Start Menu, uninstall entry, window title) is
  # already "Khor"; the file name is the release-polish item recorded in
  # W6-PACKAGING.md. Accept either here so the check tests the gate, not the name.
  $candidates = @(
    (Join-Path $installDir 'Khor.exe'),
    (Join-Path $installDir 'khor-gui.exe')
  )
  $exe = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
  if (-not $exe) {
    $found = Get-ChildItem -LiteralPath $installDir -Filter '*.exe' -ErrorAction SilentlyContinue |
      Select-Object -ExpandProperty Name
    throw "no app binary under $installDir (exes: $($found -join ', '))"
  }
  $uninst = Get-ChildItem -LiteralPath $installDir -Filter 'uninstall*.exe' -ErrorAction SilentlyContinue |
    Select-Object -First 1
  if (-not $uninst) { throw "no uninstaller under $installDir" }
  $files = Get-ChildItem -LiteralPath $installDir -Recurse -File | Measure-Object
  "$(Split-Path -Leaf $exe) + $($uninst.Name), $($files.Count) files, $([math]::Round($files.Sum / 1MB,1)) MB"
  $script:appExe = $exe
}

Step 'start' {
  $exe = $script:appExe
  if (-not $exe) { throw 'the layout step did not name a binary' }
  $out = Join-Path (Split-Path -Parent $Store) 'installed-gui.out'
  $err = Join-Path (Split-Path -Parent $Store) 'installed-gui.err'
  $proc = Start-Process -FilePath $exe -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $ready = $false
  while ($sw.Elapsed.TotalSeconds -lt $ReadySeconds) {
    if ($proc.HasExited) { throw "the app exited at $($proc.ExitCode) before answering" }
    try {
      $r = Invoke-WebRequest -Uri 'http://127.0.0.1:1432/lock_state' -Method Post -Body '{}' `
        -ContentType 'application/json' -UseBasicParsing -TimeoutSec 3
      if ($r.StatusCode -eq 200) { $ready = $true; break }
    } catch { Start-Sleep -Milliseconds 150 }
  }
  if (-not $ready) { throw "not answering within ${ReadySeconds}s" }
  $sw.Stop()
  $banner = (Get-Content -LiteralPath $err -ErrorAction SilentlyContinue |
    Select-String -Pattern 'window transport').Line
  "answered in $([math]::Round($sw.Elapsed.TotalMilliseconds))ms · $banner"
}

Step 'stop' {
  Get-Process | Where-Object { $_.ProcessName -match '^khor' } | ForEach-Object {
    Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
  }
  Start-Sleep -Milliseconds 600
  'stopped'
}

if (-not $KeepInstalled) {
  Step 'uninstall' {
    $uninst = Get-ChildItem -LiteralPath $installDir -Filter 'uninstall*.exe' -ErrorAction SilentlyContinue |
      Select-Object -First 1
    if (-not $uninst) { throw 'no uninstaller to run' }
    $p = Start-Process -FilePath $uninst.FullName -ArgumentList '/S' -PassThru -Wait
    if ($p.ExitCode -ne 0) { throw "uninstaller exit $($p.ExitCode)" }
    'uninstaller exit 0'
  }
  Step 'removed' {
    $deadline = (Get-Date).AddSeconds(40)
    while ((Get-Date) -lt $deadline) {
      if (-not (Test-Path -LiteralPath $installDir)) { return "$installDir is gone" }
      $now = @(Get-ChildItem -LiteralPath $installDir -Force | Select-Object -ExpandProperty Name)
      $extra = @($now | Where-Object { $before -notcontains $_ })
      if ($extra.Count -eq 0) { return "installer's files gone; kept: $($now -join ', ')" }
      Start-Sleep -Milliseconds 500
    }
    $now = @(Get-ChildItem -LiteralPath $installDir -Force -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name)
    $extra = @($now | Where-Object { $before -notcontains $_ })
    throw "still installed: $($extra -join ', ')"
  }
}

$out = 'F:\Temp\khor-install-check\report.json'
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $out -Encoding utf8
Write-Host ''
$failed = @($report.steps | Where-Object { -not $_.ok })
Write-Host ("{0}/{1} steps passed · report: {2}" -f ($report.steps.Count - $failed.Count), $report.steps.Count, $out)
if ($failed.Count) { exit 1 }
