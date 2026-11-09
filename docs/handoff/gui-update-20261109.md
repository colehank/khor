# GUI update recovery — active 2026-11-09

User reports GUI immediately says update incomplete, no download progress. Root serial, no new agents. Base main 467c37abcd8b. Worktree /Volumes/disk0/Khor/worktrees/current/khor-gui-update-20261109, branch fix/gui-update-recovery-20261109. No local Node/Rust/Xcode/builds; trusted GitHub validation only. Preserve running App and session hosts.

## Evidence
- Installed main GUI pid87295 /Applications/Khor.app, RC8 source6485cb83, last transaction install-nDaINN Complete, healthy.json RC8; adopted and selected RC8. No new RC11 installation/rollback. Old transaction live.lock is normal, do not delete.
- Public GitHub API and RC11 manifest/Tauri metadata HTTP200 on local network. Existing installed trusted CLI `upgrade --check --channel beta --json` succeeds RC8→RC11, installed:false. Manifest signature key IDs agree. Does not prove native GUI check succeeds.
- UpdateSettings.tsx discards IPC exception strings and never renders status.failure. Generic error and wrong retry stage confirmed. Main App stdout/stderr /dev/null, no persisted update failure. Root cause still unknown: do not claim network/cache failure.
- CUA works for native app now. Settings opened a blank Appearance window; closed only our settings/about windows, left main on My Khor (previously Khor-net terminal). CUA completion_summary sent.
- User clarifies failure is immediate with no progress.

## Next
1. Capture actual native check failure; add structured retry action/error recording and show useful cause without flooding UI.
2. Correct stage-aware retry, preserve last known version/progress, accessible native progress and current→offered version.
3. Tests first for failure display, fetch failure, install retry and preferences failure not misrouting retries; native gate and actual signed updater preflight.
4. Do not mark complete until actual failure reproduced/fixed or explicitly report limitation. Previous RC11 ship already completed; this is new work.

Graph MCP unavailable (Transport closed), .codegraph absent; targeted rg fallback. debug-issue/khor-design read, design README/states read. Existing untracked lab-preview/ in main is user-owned.
