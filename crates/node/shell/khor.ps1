# khor shell integration for PowerShell 7 (written, not wired yet: khor does
# not host on Windows; see docs/SHELL-INTEGRATION.md for how it would load).
#
# The marks are khor.zsh's: OSC 133 A/B/C/D, OSC 633;E and OSC 7, following
# VS Code's shellIntegration.ps1: the prompt function is wrapped so D and A
# come before the person's prompt and B after it, and PSReadLine's Enter
# handler sends E and C before the line runs.
if ($Global:__KhorLoaded) { return }
$Global:__KhorLoaded = $true
$Global:__KhorRan = $false
$Global:__KhorUserPrompt = $function:prompt

function Global:__Khor-Osc([string]$s) { "$([char]0x1b)]$s$([char]0x07)" }

function Global:__Khor-Escape([string]$s) {
  $s = $s.Replace('\', '\\').Replace(';', '\x3b').Replace("`n", '\x0a').Replace("`r", '\x0d').Replace("`t", '\x09')
  return $s
}

function Global:prompt {
  $ok = $?
  $code = $LASTEXITCODE
  $out = ''
  if ($Global:__KhorRan) {
    $n = if ($ok) { 0 } elseif ($code) { $code } else { 1 }
    $out += __Khor-Osc "133;D;$n"
    $Global:__KhorRan = $false
  }
  $loc = $executionContext.SessionState.Path.CurrentLocation
  if ($loc.Provider.Name -eq 'FileSystem') {
    $out += __Khor-Osc ("7;file://" + [System.Net.Dns]::GetHostName() + "/" + ($loc.ProviderPath -replace '\\', '/').TrimStart('/'))
  }
  $out += __Khor-Osc "133;A"
  $out += & $Global:__KhorUserPrompt
  $out += __Khor-Osc "133;B"
  $global:LASTEXITCODE = $code
  return $out
}

if (Get-Module PSReadLine) {
  Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    [Console]::Write((__Khor-Osc ("633;E;" + (__Khor-Escape $line))))
    [Console]::Write((__Khor-Osc "133;C"))
    $Global:__KhorRan = $true
    [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
  }
}
