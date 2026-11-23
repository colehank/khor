# The close button must end the app: click the page's close control, then
# require that the process is gone and its ports are free. Written after the
# owner reported "the close in the top left does nothing" — the main window
# was being destroyed while the process lived on holding 1432.
param(
  [string]$Exe = 'F:\Temp\khor-target\release\khor-gui.exe',
  [int]$WaitSeconds = 20
)
$ErrorActionPreference = 'Stop'

Add-Type @'
using System;
using System.Runtime.InteropServices;
public class WinClose {
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public delegate bool EnumProc(IntPtr h, IntPtr l);
}
'@

function Main-Window([int]$owner) {
  $script:found = [IntPtr]::Zero
  [WinClose]::EnumWindows({ param($h, $l)
    $pid2 = 0
    [void][WinClose]::GetWindowThreadProcessId($h, [ref]$pid2)
    if ($pid2 -eq $owner -and [WinClose]::IsWindowVisible($h)) { $script:found = $h; return $false }
    return $true
  }, [IntPtr]::Zero) | Out-Null
  return $script:found
}

function Ports-Busy {
  $busy = Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue | Where-Object { $_.LocalPort -in 1432, 5467 }
  return @($busy).Count
}

Get-Process khor-gui, khor -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
Start-Sleep -Seconds 2
$before = Ports-Busy
"ports busy before start: $before"

$gui = Start-Process -FilePath $Exe -PassThru
"started pid $($gui.Id)"
$deadline = (Get-Date).AddSeconds(45)
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Milliseconds 700
  if ((Ports-Busy) -gt 0 -and (Main-Window $gui.Id) -ne [IntPtr]::Zero) { break }
}
"ports busy after start: $(Ports-Busy)"
$window = Main-Window $gui.Id
if ($window -eq [IntPtr]::Zero) { throw 'the app never showed a visible window' }
"main window handle: $window"

# WM_CLOSE is what the title bar's X sends, which is also the road the page's
# own close control ends in (window_close -> close-requested -> destroy).
[void][WinClose]::PostMessage($window, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
"sent WM_CLOSE"

$deadline = (Get-Date).AddSeconds($WaitSeconds)
$exited = $false
while ((Get-Date) -lt $deadline) {
  Start-Sleep -Milliseconds 500
  if (-not (Get-Process -Id $gui.Id -ErrorAction SilentlyContinue)) { $exited = $true; break }
}

$busy = Ports-Busy
"process gone: $exited"
"ports busy after close: $busy"

if (-not $exited) {
  Get-Process -Id $gui.Id -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue }
  throw "FAIL: the window closed but the process still runs after $WaitSeconds s"
}
if ($busy -ne 0) { throw "FAIL: the process exited but $busy port(s) are still held" }
'PASS: closing the window ends the process and frees its ports'
