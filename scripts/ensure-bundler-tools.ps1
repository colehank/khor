# Pre-seed the tools `tauri-bundler` would otherwise download, because its own
# download times out on this network ("timeout: global") while the same URLs
# answer fine here.
#
# What it puts where (from tauri-bundler-2.10.4, windows/{msi,nsis}/mod.rs):
#   <cache>/WixTools314/*                       <- wix314-binaries.zip
#   <cache>/NSIS/*                              <- nsis-3.13.zip (nsis-3.13/ renamed)
#   <cache>/NSIS/Plugins/x86-unicode/additional/nsis_tauri_utils.dll
# where <cache> is dirs::cache_dir()/tauri, i.e. %LOCALAPPDATA%\tauri.
#
# It only creates files. Nothing that already exists is deleted.
param(
  [int]$Retries = 4,
  [int]$TimeoutSeconds = 600
)

$ErrorActionPreference = 'Stop'
$cache = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'tauri'
$work = 'F:\Temp\tauri-tools'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
New-Item -ItemType Directory -Path $work -Force | Out-Null
Write-Host "cache: $cache"

function Get-WithRetry([string]$url, [string]$to) {
  if ((Test-Path -LiteralPath $to) -and (Get-Item -LiteralPath $to).Length -gt 0) {
    Write-Host "  have $(Split-Path -Leaf $to)"
    return
  }
  for ($i = 1; $i -le $Retries; $i++) {
    try {
      Write-Host "  GET $(Split-Path -Leaf $to) (try $i/$Retries)"
      $sw = [System.Diagnostics.Stopwatch]::StartNew()
      Invoke-WebRequest -Uri $url -OutFile $to -UseBasicParsing -TimeoutSec $TimeoutSeconds
      $sw.Stop()
      $mb = [math]::Round((Get-Item -LiteralPath $to).Length / 1MB, 2)
      Write-Host "    ok ${mb} MB in $([math]::Round($sw.Elapsed.TotalSeconds,1))s"
      return
    } catch {
      Write-Host "    failed: $($_.Exception.Message)"
      if (Test-Path -LiteralPath $to) { Remove-Item -LiteralPath $to -Force -ErrorAction SilentlyContinue }
      Start-Sleep -Seconds (3 * $i)
    }
  }
  throw "could not download $url"
}

$wix = Join-Path $work 'wix314-binaries.zip'
$nsis = Join-Path $work 'nsis-3.13.zip'
$dll = Join-Path $work 'nsis_tauri_utils.dll'

Get-WithRetry 'https://github.com/wixtoolset/wix3/releases/download/wix3141rtm/wix314-binaries.zip' $wix
Get-WithRetry 'https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.13/nsis-3.13.zip' $nsis
Get-WithRetry 'https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll' $dll

# WiX: the zip's entries are flat, so it extracts straight into WixTools314.
$wixDir = Join-Path $cache 'WixTools314'
if (-not (Test-Path -LiteralPath (Join-Path $wixDir 'light.exe'))) {
  New-Item -ItemType Directory -Path $wixDir -Force | Out-Null
  Expand-Archive -LiteralPath $wix -DestinationPath $wixDir -Force
  Write-Host "  wix -> $wixDir"
} else { Write-Host "  wix already in place" }

# NSIS: the zip holds a single `nsis-3.13/` directory that has to become `NSIS`.
$nsisDir = Join-Path $cache 'NSIS'
if (-not (Test-Path -LiteralPath (Join-Path $nsisDir 'makensis.exe'))) {
  $stage = Join-Path $work 'nsis-stage'
  New-Item -ItemType Directory -Path $stage -Force | Out-Null
  Expand-Archive -LiteralPath $nsis -DestinationPath $stage -Force
  $inner = Get-ChildItem -LiteralPath $stage -Directory | Select-Object -First 1
  if (-not $inner) { throw "the nsis zip did not hold a directory: $stage" }
  if (Test-Path -LiteralPath $nsisDir) { throw "refusing to overwrite $nsisDir" }
  Move-Item -LiteralPath $inner.FullName -Destination $nsisDir
  Write-Host "  nsis -> $nsisDir"
} else { Write-Host "  nsis already in place" }

# The one plugin the bundler fetches separately.
$plugins = Join-Path $nsisDir 'Plugins\x86-unicode\additional'
New-Item -ItemType Directory -Path $plugins -Force | Out-Null
Copy-Item -LiteralPath $dll -Destination (Join-Path $plugins 'nsis_tauri_utils.dll') -Force
Write-Host "  plugin -> $plugins"

Write-Host ''
Write-Host 'required files present:'
foreach ($f in 'light.exe', 'candle.exe') { "  wix  $f : $(Test-Path -LiteralPath (Join-Path $wixDir $f))" }
foreach ($f in 'makensis.exe', 'Bin/makensis.exe', 'Include/MUI2.nsh', 'Plugins/x86-unicode/additional/nsis_tauri_utils.dll') {
  "  nsis $f : $(Test-Path -LiteralPath (Join-Path $nsisDir $f))"
}
