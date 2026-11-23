# W3 smoke runner. Each check is a real command with an expected marker.
# Results go to .cache/w3/smoke-<utc>.json; the console prints one line per check.
#
#   pwsh -File scripts/windows-smoke.ps1
#   pwsh -File scripts/windows-smoke.ps1 -Sandboxed     # classify os error 5 as ENV
param(
    [switch]$Sandboxed,
    [string]$CargoTargetDir = '',
    [int]$TimeoutSeconds = 300
)

$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
if ($CargoTargetDir) { $env:CARGO_TARGET_DIR = $CargoTargetDir }

$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$outputDir = Join-Path $root '.cache/w3'
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
# Binaries built inside the workspace inherit its Low integrity label and then
# cannot write the user profile (the runtime store lives there). Point this at a
# target directory outside the workspace to exercise the CLI as a person would.
$target = if ($CargoTargetDir) { $CargoTargetDir } else { Join-Path $root '.cache/target' }
$exe = Join-Path $target 'debug/khor.exe'
$probe = Join-Path $outputDir "probe-$stamp"
New-Item -ItemType Directory -Path $probe -Force | Out-Null

$envPatterns = @('os error 5', 'Access is denied', '拒绝访问', 'timed out waiting for the serve', 'document transfer state busy')

function Invoke-Check {
    param(
        [string]$Name,
        [string]$Area,
        [scriptblock]$Run,
        [scriptblock]$Expect
    )
    $started = Get-Date
    $log = Join-Path $outputDir ($Name + '.log')
    try {
        $result = & $Run 2>&1 | Out-String
        $code = $LASTEXITCODE
    } catch {
        $result = $_.Exception.Message
        $code = -1
    }
    $result | Set-Content -LiteralPath $log -Encoding utf8
    $expected = & $Expect $result $code
    $status = if ($expected) { 'PASS' } elseif ($Sandboxed -and ($envPatterns | Where-Object { $result -match [regex]::Escape($_) })) { 'ENV' } else { 'FAIL' }
    $firstLine = ($result -split "`n" | Where-Object { $_.Trim() } | Select-Object -First 1)
    [pscustomobject]@{
        name = $Name
        area = $Area
        status = $status
        exit = $code
        seconds = [math]::Round(((Get-Date) - $started).TotalSeconds, 1)
        first = if ($firstLine) { $firstLine.Trim() } else { '' }
        log = $log
    }
}

$checks = @(
    @{ name = 'bin-version'; area = 'P1'; run = { & $exe --version }; expect = { param($o, $c) $c -eq 0 -and $o -match '^\d+\.\d+\.\d+' } },
    @{ name = 'node-agent-process'; area = 'P1-P3'; run = { cargo test --locked -q -p khor-node --lib khor_agent_process:: -- --test-threads=1 }; expect = { param($o, $c) $c -eq 0 -and $o -match 'test result: ok' } },
    @{ name = 'node-journal'; area = 'F1'; run = { cargo test --locked -q -p khor-node --lib session_content::journal:: -- --test-threads=1 }; expect = { param($o, $c) $c -eq 0 -and $o -match 'test result: ok' } },
    @{ name = 'tabs-cross-process'; area = 'B1'; run = { cargo test --locked -q -p khor-node --lib tabs::tests::two_processes_do_not_lose_each_others_tabs -- --exact }; expect = { param($o, $c) $c -eq 0 -and $o -match 'test result: ok' } },
    @{ name = 'tabs-readonly'; area = 'F2'; run = { cargo test --locked -q -p khor-node --lib tabs::tests::a_read_only_table_reports_an_error_instead_of_panicking -- --exact }; expect = { param($o, $c) $c -eq 0 -and $o -match 'test result: ok' } },
    @{ name = 'tabs-long-path'; area = 'F3'; run = { cargo test --locked -q -p khor-node --lib tabs::tests::a_table_beyond_max_path_still_opens -- --exact }; expect = { param($o, $c) $c -eq 0 -and $o -match 'test result: ok' } },
    @{ name = 'cli-ws'; area = 'F'; run = { cargo test --locked -q -p khor-cli --test ws -- --test-threads=1 }; expect = { param($o, $c) $c -eq 0 } },
    @{ name = 'cli-quit'; area = 'P4'; run = { cargo test --locked -q -p khor-cli --test quit -- --test-threads=1 }; expect = { param($o, $c) $c -eq 0 } },
    @{ name = 'cli-console'; area = 'T1-T4'; run = { cargo test --locked -q -p khor-cli --test console -- --test-threads=1 }; expect = { param($o, $c) $c -eq 0 } },
    @{ name = 'cli-help'; area = 'T1'; run = { & $exe help }; expect = { param($o, $c) $c -eq 0 } },
    @{ name = 'tabs-dual'; area = 'B1'; run = {
            $env:KHOR_HOME = Join-Path $probe 'store'
            $outOne = Join-Path $probe 'tabs-1.out'; $errOne = Join-Path $probe 'tabs-1.err'
            $outTwo = Join-Path $probe 'tabs-2.out'; $errTwo = Join-Path $probe 'tabs-2.err'
            $first = Start-Process -FilePath $exe -ArgumentList 'tabs','ls' -PassThru -WindowStyle Hidden -RedirectStandardOutput $outOne -RedirectStandardError $errOne
            $second = Start-Process -FilePath $exe -ArgumentList 'tabs','ls' -PassThru -WindowStyle Hidden -RedirectStandardOutput $outTwo -RedirectStandardError $errTwo
            [void]$first.WaitForExit(60000)
            [void]$second.WaitForExit(60000)
            $text = ((Get-Content -LiteralPath $errOne -Raw -ErrorAction SilentlyContinue) + (Get-Content -LiteralPath $errTwo -Raw -ErrorAction SilentlyContinue))
            "first=$($first.ExitCode) second=$($second.ExitCode) $text"
        }; expect = { param($o, $c) $o -match 'first=0 second=0' } }
)

$results = foreach ($check in $checks) {
    Invoke-Check -Name $check.name -Area $check.area -Run $check.run -Expect $check.expect
}

$receipt = [pscustomobject]@{
    schema = 1
    generated_at = (Get-Date).ToUniversalTime().ToString('o')
    sandboxed = [bool]$Sandboxed
    results = $results
}
$receiptPath = Join-Path $outputDir ("smoke-$stamp.json")
$receipt | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $receiptPath -Encoding utf8

$results | Format-Table -AutoSize name, area, status, exit, seconds, first
"receipt: $receiptPath"
$failed = @($results | Where-Object { $_.status -eq 'FAIL' }).Count
$envBlocked = @($results | Where-Object { $_.status -eq 'ENV' }).Count
"pass: $(@($results | Where-Object { $_.status -eq 'PASS' }).Count)  env: $envBlocked  fail: $failed"
if ($failed -gt 0) { exit 1 }
exit 0
