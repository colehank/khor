# khor shell integration for PowerShell, loaded after native Profiles.
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
  return $s.Replace([string][char]0x1b, '\x1b').Replace([string][char]0x07, '\x07')
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
    if ([System.IO.Path]::DirectorySeparatorChar -eq [char]92) {
      $out += __Khor-Osc ("9;9;" + $loc.ProviderPath)
    } else {
      $out += __Khor-Osc ("7;file://" + [System.Net.Dns]::GetHostName() + "/" + $loc.ProviderPath.TrimStart('/'))
    }
  }
  $out += __Khor-Osc "133;A"
  $out += & $Global:__KhorUserPrompt
  $out += __Khor-Osc "133;B"
  $global:LASTEXITCODE = $code
  return $out
}

function Global:__Khor-EditorCheckpoint {
  $line = $null; $cursor = 0
  [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
  if ($line.Length -gt 1024) {
    $limit = if ([char]::IsHighSurrogate($line[1023])) { 1023 } else { 1024 }
    $line = $line.Substring(0, $limit)
  }
  [Console]::Write((__Khor-Osc ("633;P;KhorEditorCheckpoint=$cursor;" + (__Khor-Escape $line))))
}

if (Get-Module PSReadLine) {
  # Ask the editor after built-in actions that can change text without
  # emitting it as keystrokes. Keep custom and vi-mode bindings in place.
  if ((Get-PSReadLineOption).EditMode -ne 'Vi') {
    $editorActions = @(
      'PreviousHistory', 'NextHistory', 'HistorySearchBackward', 'HistorySearchForward',
      'Complete', 'MenuComplete', 'TabCompleteNext', 'TabCompletePrevious',
      'Undo', 'Redo', 'RevertLine', 'Paste', 'Cut',
      'BackwardDeleteWord', 'DeleteWord', 'BackwardKillWord', 'KillWord',
      'AcceptSuggestion', 'AcceptNextSuggestionWord',
      'AddLine', 'ForwardChar', 'EndOfLine'
    )
    $reader = [Microsoft.PowerShell.PSConsoleReadLine]
    foreach ($binding in (Get-PSReadLineKeyHandler -Bound)) {
      if ($binding.Function -notin $editorActions) { continue }
      $action = $reader.GetMethod($binding.Function, [type[]]@([Nullable[ConsoleKeyInfo]], [object]))
      if ($null -eq $action) { continue }
      $handler = {
        param($key, $argument)
        $null = $action.Invoke($null, [object[]]@($key, $argument))
        __Khor-EditorCheckpoint
      }.GetNewClosure()
      Set-PSReadLineKeyHandler -Chord $binding.Key -BriefDescription KhorEditor -ScriptBlock $handler
    }
  }
  Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {
    $line = $null; $cursor = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$cursor)
    [Console]::Write((__Khor-Osc ("633;E;" + (__Khor-Escape $line))))
    [Console]::Write((__Khor-Osc "133;C"))
    $Global:__KhorRan = $true
    [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
  }
}
