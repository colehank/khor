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

## Confirmed root cause (Air, not MacLab)
User clarified MacBook Air. Connected over existing paired Khor, used three isolated shell sessions solely for read-only diagnostics with private /tmp reports. Session IDs stored under /Volumes/disk0/Khor/evidence/gui-update-20261109/air-probe*-session.txt; close only these after completion. Do not stop user sessions or App.
- Air anonymous GitHub API: HTTP403, X-RateLimit-Remaining:0, explicit API rate limit exceeded. API request uses same khor-update/0.2.0 UA. Public signed manifest and Tauri metadata HTTP200. Public releases.atom HTTP200 in 0.69s.
- Air on-disk App is already RC11 source467c37ab; adopted/selected record RC8. adopted() already guards the compiled build identity, so old adopted record is not itself a current-version bug. Do not change that guard.
- Evidence: air diagnostic report and signed public records in /Volumes/disk0/Khor/evidence/gui-update-20261109. HTTP text contains public egress IP; keep local, do not publish raw diagnostics or user paths.

## Implementation in progress, uncommitted
- crates/update feed: public Atom discovery, canonical release links, SemVer/channel selection; public latest stable permalink fallback when stable falls outside finite feed; API only as fallback. Authenticating signed manifests/packages unchanged. Reuse existing locked xml-rs 0.8.29, add dependency + lock edge.
- UpdateAction retry optional field, native failure stage + bounded private atomic last-failure.json. Initial cache restore failure no longer silently stops checking; manual fresh check no longer requires adopting previous cache first. Download stage displayed before SDK network setup.
- GUI retains actual IPC/native errors behind shared MoreFold, correct operation-specific retry, current App version, retries failed preference/companion persistence rather than misrouting it to downloads. New tests cover these and initial status read failure.
- Existing updates-ui test was omitted from Quality GUI group; added it. Tests-first commit88d1bbe9 pushed. Red run42575257612 is still executing; need read real failure before claiming red evidence. No native builds on this Mac.
- Remote rustfmt available on macserver /Users/z13/.cargo/bin/rustfmt. Used STDIN, no builds or uploaded credentials. Some parallel calls returned empty stdout; NEVER overwrite source from empty formatter output. app_updates formatted output reviewed and whitespace/literals verified; feed formatted output reviewed structural-only but will need reformat after stable fallback changes.

## Required remaining
1. Finish feed transport tests, format remotely (only text) and inspect diff. Run fresh Quality all including native Mac updater and khor-update tests; updates-ui regression must fail old code and pass new.
2. Read-only Air test of actual fixed feed logic from trusted build, then GUI update route/retry verification. Existing RC11 binary cannot receive code changes in place; a first repaired installer may be needed because its updater is the failing code.
3. Close own three diagnostic sessions and remove only their private /tmp files. Preserve user App/session hosts.
4. Review all changes, report measured validation limits. Previous ship already complete; current fix not yet merged/released.

## Red evidence recorded
Quality42575257612 finished FAILURE only GUI. Job129022956465 log shows updates-ui failed at check-updates-ui.mjs:108 waiting for [data-update-failure] summary: old component discards native error. Full log retained locally at evidence/gui-update-20261109/red-gui.log. Existing other interaction groups passed. New transport tests cover API-independent beta discovery, public stable permalink history, prerelease-only stable404 and canonical XML links. Native record boundedness/privacy test added.
