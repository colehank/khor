# Settings GUI · review ready, publication held

Latest user: settings within the main app, no separate popup; audit against Apple HIG. Earlier instruction explicitly holds merge/ship for settings discussion. Root serial; no new agents, no local Node/Bun/Rust/Xcode builds. Installed user App and live session hosts untouched.

## Source

Worktree /Volumes/disk0/Khor/worktrees/current/khor-settings-gui-20261109, branch integrate/settings-gui-20261109. Product48fc2bca, based on639f3df8, includes previously verified updater repair. Original source-only main checkout remains467c37ab with user-owned lab-preview/ preserved. Read actual git state after this documentation commit.

Implemented: no new native settings window; native menu/editor requests focus main and emit settings:open; UI clicks/shortcut share one main-page handler. Seven settings-only categories, localized Settings title, current-category aria semantics, last-category restoration with safe storage fallback. Existing main frame/material and controls reused. Mobile cold start continues to restore landing only; reopening Settings restores category.

Audit: docs/handoff/settings-hig-audit-20261109.md separates implemented changes from proposals: local/remote scope, setting-item search, saving/errors (including silent About failures), About hierarchy, long-label/larger-text/focus checks. HIG permits an independent macOS settings window; main-container choice follows the owner, not a purported HIG prohibition.

## Preview and evidence

http://127.0.0.1:23950/app/?scene=devices&machines=3 ; More → Settings. Own Python-only preview session42859, serving trusted GH artifact at /Volumes/disk0/Khor/evidence/settings-gui-20261109/final-lab/developer-lab. Snapshot80743a7f, real production App. Its production UI/native sources equal48fc2bca; later changes only fix/scoped test data storage on Node. Do not claim preview includes those later harness edits.

Trusted run42599476841: developer-lab job SUCCESS,56 App scenarios,0 errors; settings desktop/mobile navigation, native-derived setting changes and persistence passed. Screenshots inspected, Chinese layout clean. Overall run FAILURE because the shared component-test fixture touched localStorage in Node.

Fixed in c9629a09: safe browser storage, in-memory Node values; no direct browser API outside guard. Follow-up48fc2bca makes canonical settings metadata opt-in only for full App Lab (bootstrap settings:true). Legacy component harnesses retain their original config=[] contract; complete App Lab regenerates validated native metadata. The tracked generated world data is an old domain cache, NOT untracked: do not assume it already has settings.

Native focused run42598784540 onfbd71678 SUCCESS: native build/tests, actual native WebKit, unit/types, static gates, scripts/format and three-platform runtime checks. native-source-equivalence.json proves all changed native/App-routing sources match48fc2bca. Temporary verify/settings-native-20261109 branch deleted after success.

## Still running — do not claim all GUI green

42601894665 targets c9629a09 (safe Node storage); follow-up final GUI run42602592900 targets48fc2bca and is queued. Check actual status/IDs. No more product changes are planned just to keep CI busy. Publication remains held regardless of success. Broader GUI regression must be green before merge. User can review Settings preview while these run.

The appearance/check-face legacy settings-window assertions and mobile-static obsolete command inventory were updated. The appearance/check-face standalone legacy walks were NOT run this turn; no claim of real macOS CmdComma/window-focus interaction or iPhone keyboard verification.

Source-only Python fixture-cache tests4/4 pass. Macserver SSH repeatedly timed out at banner (one echo succeeded), isolated clone did not start; do not install a local compiler to compensate. Graph MCP Transport closed, targeted reads used; Git hooks updated graph. CUA browser host unavailable, completion summary sent; no user browser/App was changed. Preview HTTP200.

## Next

Review remaining GUI run(s), keep evidence/record failures precisely. If the user steers settings UX meanwhile, preserve this task and hold release. Do not merge main or ship without renewed instruction. Official HIG links are in the audit document.
