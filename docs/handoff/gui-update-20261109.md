# Updater repair complete — awaiting user's GUI requirements

Updated 2026-11-09. This is the authoritative current checkpoint; previous investigation is preserved in Git history.

## User instruction

“别急着ship. 我们的设置等地方还需要优化GUI, 这做完了等着我”

Updater repair and validation are complete. Keep the changes on the isolated branch. **Do not merge main, tag, ship, install/restart the user's App, or begin additional GUI work. Wait for the user's next requirements.** No agents were spawned.

## Source and validation

- Worktree: `/Volumes/disk0/Khor/worktrees/current/khor-gui-update-20261109`
- Branch: `fix/gui-update-recovery-20261109`
- Exact tested product/workflow commit: `b53048d6d65f2aa7c78a5e0901c27fa4401452ab`
- Trusted Quality run: https://github.com/colehank/khor/actions/runs/42581741576 — **SUCCESS**
- Seven successful jobs: scripts; runtime/update tests on Linux, macOS and Windows; frontend generation/build; GUI unit/types/design/update interaction; Mac native update admission/rollback/component recovery and native WebKit loopback. Five unrelated jobs intentionally skipped in the explicit `updates` scope.
- Mac native log: 15 update tests, 2 companion tests, 1 actual WebKit loopback test passed. This is automated validation, not an actual upgraded Air installation.
- Final documentation-only checkpoint commit may follow the tested commit; it changes no tested product or workflow code.
- No main merge or release was performed. Local main remains `467c37abcd8bb5faea58d02507014c7e6f8fce3a`; its preexisting untracked `lab-preview/` was preserved. Re-fetch and inspect actual main before any future integration.

## Confirmed failure

The affected machine is **MacBook Air**, not MacLab. Over its existing paired Khor connection, read-only diagnostics with the same `khor-update/0.2.0` user agent observed GitHub release-list API HTTP403, `X-RateLimit-Remaining: 0`, and an explicit API-rate-limit error. Release manifests/Tauri metadata returned200; public `releases.atom` returned200 in0.69s. This explains failure before download in the existing API-dependent check path.

The old GUI discarded IPC exceptions and never displayed native `status.failure`, replacing the cause with a generic update-incomplete message. Air's on-disk App is RC11; older adopted/selected records are RC8. `adopted()` already checks the compiled identity, so the old record was not treated as a new version-identification defect.

## Changes

- Discover versions from GitHub's public Atom feed using canonical release links and SemVer/channel rules. Resolve the public stable permalink when stable falls outside finite feed history. API history remains a fallback. Signed manifest, archive, Tauri signature, installation compatibility and health checks remain required.
- Add optional native retry action; show current App version, specific failed stage and expandable error details. Preserve IPC errors and retry the original settings/component operation. Disable retries while native work continues.
- Retain one bounded private atomic `last-failure.json` diagnostic in admitted staging. Fresh checks no longer depend on first adopting an obsolete selection, and failed startup-cache restore no longer silently prevents checking.
- Missing installation cache retries download; missing offer/component plan retries checking; actual installation failure retries installation.
- Restore `updates-ui` to full Quality and add an explicit focused `updates` development scope. Default main/PR and Release still run the complete suite; the Release workflow_call exposes no scope selector. Verified default/release selects21 interaction groups, updater scope1, and workflow YAML has no duplicate keys.

## Evidence and development checks

Local evidence: `/Volumes/disk0/Khor/evidence/gui-update-20261109/`

- `quality-final.json`: exact final source and all job outcomes.
- `native-final.log`: trusted native test results.
- `red-gui.log`: tests-first commit88d1bbe9 fails because the old component has no error-details view.
- `997-gui.log`: intermediate automated assertion ran before asynchronous fold expansion; fixed by waiting for visible content, with final CI passing. Intermediate all run42577804511 and pending4b run42579300118 were canceled, not reported as successful.
- `update-failure.png`: inspected mock-only settings component screenshot; CI now retains updater screenshots through FILE_HANDOFF_OUT.
- Air signed public records and network diagnostics remain local. Do not publish raw diagnostic reports, which include the egress IP and local paths.

Macserver provided development feedback: Rust1.95 check26.6s, native updater unit53.2s; generated TypeScript/types,525-file Rust format check and real updater component browser regression passed. Its artifacts were never executed locally and no signing credentials were used there. The final trusted result is the GitHub run above.

## Cleanup and continuation

All three own Air diagnostic sessions were closed and their six temporary report/header/body/XML files removed. User App and existing session hosts were left running. The isolated macserver review clone was removed; shared Cargo/npm caches and its original checkout were preserved. Small evidence logs remain. No local Node, Rust, Bun, Xcode or Swift builds/execution occurred.

Recovery: `/Users/zgh/Library/Caches/khor-code-quality/recovery/gui-update-20261109/RESUME.md` and `checkpoint.json`. Current updater work has no remaining implementation or verification task. Wait for user steering on settings and GUI; carry the explicit publication hold forward after compaction.
