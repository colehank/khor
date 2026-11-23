# Copy the DSH session store to a dated directory and prune old copies.
# Run it once by hand; -Install registers a daily task for the current user.
param(
    [switch]$Install,
    [string]$Destination = 'F:\Backups\dsh',
    [int]$Keep = 7,
    [string]$DshHome = (Join-Path $HOME '.dsh')
)

$ErrorActionPreference = 'Stop'
$python = (Get-Command python -ErrorAction Stop).Source
$script = Join-Path $PSScriptRoot 'khor_lab\backup.py'

& $python $script --dsh-home $DshHome --destination $Destination --keep $Keep
if ($LASTEXITCODE -ne 0) { throw "backup failed with exit code $LASTEXITCODE" }

if ($Install) {
    $task = 'DSH session backup'
    $action = '"{0}" "{1}" --dsh-home "{2}" --destination "{3}" --keep {4}' -f $python, $script, $DshHome, $Destination, $Keep
    schtasks.exe /Create /TN $task /TR $action /SC DAILY /ST 03:00 /F | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "schtasks failed with exit code $LASTEXITCODE" }
    Write-Output "registered daily task: $task"
}
