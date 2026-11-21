# Does this binary carry the Khor agent runtime?
#
# The runtime is embedded at build time from `KHOR_AGENT_BUNDLE`
# (`crates/node/build.rs` -> `include_bytes!`). A release built without that
# variable has no agent at all, and the only symptom is that creating a Khor
# conversation fails — which reads exactly like "the local agent was removed"
# (the owner's report, 2026-11-18). This is the gate that would have caught it.
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows-agent-check.ps1 -Exe <path to khor.exe>
#
# Exit 0 = the runtime is embedded; anything else = it is not.
param(
  [Parameter(Mandatory = $true)][string]$Exe
)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path $Exe)) {
  Write-Output "FAIL no such binary: $Exe"
  exit 2
}
$raw = & $Exe _runtime_info 2>$null | Select-Object -First 1
if (-not $raw) {
  Write-Output "FAIL $Exe printed no runtime information"
  exit 3
}
try {
  $info = $raw | ConvertFrom-Json
} catch {
  Write-Output "FAIL runtime information is not JSON: $raw"
  exit 4
}
$release = $info.release
$portable = [bool]$release.portable
$pi = $null
if ($release.embedded -and $release.embedded.PSObject.Properties['pi']) {
  $pi = $release.embedded.pi
}
if (-not $portable -or -not $pi) {
  Write-Output "FAIL the Khor agent runtime is NOT embedded in $Exe"
  Write-Output "     portable=$portable embedded=$($release.embedded | ConvertTo-Json -Compress)"
  Write-Output "     build it: npm ci --prefix apps/agent; bun apps/agent/build.mjs <target> apps/agent/dist/khor-agent"
  Write-Output "     then rebuild with KHOR_AGENT_BUNDLE=<that file>"
  exit 1
}
Write-Output "PASS agent runtime embedded"
Write-Output "     version=$($release.version) role=$($release.role) platform=$($release.platform)"
Write-Output "     pi=$pi portable=$portable"
exit 0
