# Settings GUI — active, no publication

User requests settings within the main app, no separate window, and an Apple HIG standard audit. Only this requested UI work is authorized; no merge or ship. Root serial; source-only local (no Node/Rust/Bun/Xcode builds). Worktree `/Volumes/disk0/Khor/worktrees/current/khor-settings-gui-20261109`, branch `integrate/settings-gui-20261109`, base639f3df8 includes the verified updater repair (product b53048d6, Quality42581741576 SUCCESS).

## Current implementation (uncommitted)

- Remove dedicated settings-face frontend branches and native window creation. Native menu/editor requests focus the existing main window and emit settings:open with pane/machine; main UI entries share one handler. No window resize.
- Settings-specific category list/header, current category aria-current, per-device last-category restoration. Seven existing categories retained.
- Shared settings-navigation module holds the existing category inventory and last-page storage. No new visual skin, tokens or icons.
- Native validation reuses khor_node::config::PAGES; old auxiliary label remains only in existing close/draft compatibility guards.

## Audit findings to discuss / Lab

Apple official references: https://developer.apple.com/design/human-interface-guidelines/settings ; https://developer.apple.com/design/human-interface-guidelines/search-fields ; https://developer.apple.com/design/human-interface-guidelines/selection-and-input . Read their current official Markdown versions too. HIG allows a separate macOS settings window: main-window integration is the user's product choice, not a claim that HIG prohibits windows.

1. Entry paths split by platform/viewport and method (menu, click, shortcut). Being unified now.
2. Settings mixed with My Khor/storage/diagnostics and labeled More; selected item only data-on. Being clarified now.
3. Always opens Appearance rather than last pane. Being fixed now.
4. Missing search of actual settings. Need a real item index/location flow, not category-name filtering. Proposal only.
5. Local viewing preferences vs remote-machine AgentRecognition in TerminalPage have different scopes with insufficient separation. Proposal only; do not silently remove capability or call all recognition Hook-based without backend verification.
6. Save state/error handling varies: ConfigSections fires mutations without row pending/order handling, WindowAppearance has its own queue, local settings have different paths. Needs delayed/failure interaction tests before claiming a reproduced race. Proposal only.
7. About combines version/update, raw store paths/cache and CLI technical facts. Keep normal info clear; technical details progressive disclosure. Do not reintroduce a General category without discussion (earlier owner removed it).
8. Long labels/value/button rows, larger font, mobile keyboard/focus/back restoration need real layout checks; not yet proven defects. Existing grouped cards/native pickers/tokens are reusable.

## Lab data

Lab native domain generator now emits bilingual ConfigRow metadata from actual config_rows over its temporary empty store, no Node opened. Fixture config/set_config use those rows with Lab-only browser values; no duplicated setting inventory or production credentials. Python fixture cache tests4/4 pass locally.

## Next

Add regression for single main surface, all entries, category restoration and selected accessibility semantics; use the real App Lab, no recreated settings mock page. New source files must be staged for snapshots. Graph MCP Transport closed; fallback targeted rg reads used; .codegraph absent at original root. khor-design and design README/layout read. Need audit final document and lab preview; do not fix all other findings unasked.

Lab docs docs/development/LAB.md; source-only uses frontend mode/remote trusted GitHub. Dedicated public developer-lab.yml exists, configured by scripts/khor_lab/builders.py. Private control repo can have billing failures; don't run untrusted macserver binaries on this Mac. Macserver can give development feedback through isolated source checkouts and shared caches; trusted GitHub validates deliverables. Existing Lab23930 is older baseline and must not be presented as this modification.

## Verification checkpoint

Product fbd71678 pushed on integrate/settings-gui-20261109. First GUI run42597844198 tests5f0a62b0; final GUI run42598733258 targetsfbd71678 and is queued. Temporary verify/settings-native-20261109 ref points to exactlyfbd71678 so focused native/update Quality can run in parallel without canceling GUI validation; delete after verification. Native-source formatting aligned manually; actual compiler/formatter result still required. Macserver SSH repeatedly times out at banner, one simple echo succeeded, isolated clone never started. No local compiler fallback.

Python fixture cache tests4/4 pass. docs/handoff/settings-hig-audit-20261109.md contains implemented vs proposed changes. Need obtain trusted Lab bundle, serve23950, inspect desktop/mobile forms and report native limitations accurately; no merge/ship.

First GUI run unit/types passed, then gates refused the newly added script because the initial revision had no gate documentation. Current docs now name both npm command and script filename (the registry requires the filename). Mobile App deliberately restores only its landing on cold start; regression updated to reopen Settings and check the saved category instead of changing that existing policy. Canceled queued42598733258 before its outdated checks consumed build time. Focused native run42598784540 usesfbd71678 (same Rust sources), may report obsolete GUI docs separately; native compile evidence must be read by job.
