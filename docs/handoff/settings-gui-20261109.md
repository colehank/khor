# Settings GUI · review ready, publication held

Latest user: settings within the main app, no separate popup; audit against Apple HIG. Earlier instruction explicitly holds merge/ship for settings discussion. Root handles Settings GUI; one workspace-file-ops agent explicitly authorized by the latest user request. No local Node/Bun/Rust/Xcode builds. Installed user App and live session hosts untouched.

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

## New user scope: workspace file operations

User adds this round: Methods.md → methods.md wrongly reports existing file; workspace file/directory dragging into another directory should move it. Explicitly authorizes one agent while root stays with GUI discussion. Agent /root/workspace_file_ops owns /Volumes/disk0/Khor/worktrees/current/khor-workspace-file-ops-20261109, branch fix/workspace-file-ops-20261109, base137fc856. Agent must not spawn others, merge/ship/install/restart App. It will maintain docs/handoff/workspace-file-ops-20261109.md and submit commits for root integration. Root will not edit its target file-operation paths. Scope should cover shared standalone/session workspace components and real safe backend move/rename behavior. Root asked optional settings-GUI focus question; no new GUI design choice inferred yet. Publication stays on hold.

## Latest GUI direction: one More destination

User requires Storage/Settings/Diagnostics all within a unified More entry and explicitly adds My Khor. Implement More button as direct page entry, not dropdown. Fixed navigation for all More pages: My Khor, overflow landings, a clearly labeled group of seven settings categories, Storage/Notifications/Diagnostics. Same list while any of these is open; My Khor no longer hides desktop list or draws a separate desktop title bar. Mobile leaf back returns this unified More list. Reuse current page bodies/material/tokens; native settings deep links retain last category and main-window behavior. Root owns App/FloatNav/LandingStrip/MyKhor/MoreNavigation; agent owns file operations. No merge/ship.

More implementation now extracts MoreNavigation (fixed grouped list), pure more-navigation state (valid inventory + last leaf), simplifies FloatNav/LandingStrip to direct onMore, removes My Khor desktop dial special shell and portals its segment into shared PageAction. Mobile More leaf back always returns the same list. Settings deep links/native focus unchanged. Shared component tests still use config=[]; full App Lab uses canonical settings rows. Synthetic read-only About/Relay/Doctor/RoadShare readings added for More preview (explicit unmeasured diagnostic, no real machine check). Added raw canonical world-cache report artifact so later source-only UI revisions can consume validated generated metadata without repeated Rust generation; no policy bypass. Not yet compiled/previewed after this More change.

## Apple Settings reference / network identity

User provided macOS Settings screenshot and clarified My Khor is the account-equivalent but represents a network cluster, not a human account. Implement generic NetworkMark shared in sidebar and profile (do not reuse current machine name/avatar as network identity). NetworkOverview uses existing MoreFrame/cards, actual member rows with MachineFace, add-machine wizard, connection settings, relation-map detail and existing guarded recovery actions. Current-machine-only leave/reset explicitly grouped/labeled; no new shared network-name/avatar backend fields invented. MyKhorView controlled by parent supports overview/map/matrix, defaults overview; map/matrix retained as internal detail with return-to-overview, shared header/sidebar unchanged. Existing device popovers and destructive confirmation controllers retained. Need compile + exact App Lab, add to design/user-review checklist.

## Verification: More + network overview

Productfb072f46 pushed. Previous complete GUI run42602592900 on48fc2bca SUCCESS (the shared Node fixture issues are resolved). New More/profile GUI validation is now running; obtain run ID from current workflow state. More direct-entry, network identity overview, branch-local public src changes only; preview23950 still80743a7f until new trusted artifact passes. Do NOT call it the new More preview yet. Locale TOML/package JSON/YAML syntax and Python native-fixture cache tests pass locally; JS/Rust not run locally. Agent /root/workspace_file_ops continues independent safe-rename/drag work, not integrated yet.

Navigation polish after initial More commit: overview/map/matrix are presentation modes of ONE My Khor page, owned locally by MyKhorPage. The visualizer's existing segmented control now includes Overview, with overview default. This removes parent-only view state and avoids conflicting back depths (button/swipe both leave the same More leaf to the More list on mobile). Relation row selects the visualization; explicit Overview mode returns to the profile. Sidebar/title remain My Khor throughout. This is not a second account or a new account backend. Hero title uses Network cluster to avoid echoing page title. Need exact-revision Lab after these changes.

Initial More run42608739937 frontend passed. Developer Lab failed its type check before UI tests: About.store_bytes is generated as bigint, while JSON transport fixtures use numbers. Keep native-compatible JSON numeric data via explicit Omit<About,store_bytes> & {store_bytes:number}, validating all other About fields, rather than 0n (the Node fixture HTTP bridge JSON.stringify cannot serialize bigint). Existing formatBytes accepts both. No production Rust/schema changes for this fixture fix. Exact GUI run on combined corrected source still needed.

CI preview acceleration: frontend.yml now generates native Lab metadata after npm prebuild has already compiled the same core; includes it as lab-world.generated.json in the existing frontend artifact BEFORE the immutable receipt is created. restore-frontend verifies the receipt then optionally restores the sidecar (legacy artifacts retain fallback generation). Developer Lab's existing fixture_data.ensure still validates source inputs/payload before reuse. This avoids a second full Rust build in a fresh Lab job without changing signing/release or relaxing trust. Sidecar lives outside dist and is never application UI/runtime data. Added receipt mutation test for this cache; no untrusted builder executable consumed locally.

## File operations integrated

Parent applied137fc856..d7914804 with git cherry-pick --no-commit, reviewed accumulated native and drag source, no conflicts. No temporary workflow remains. Agent green42610650989 coversc6df625f; finald7914804 adds conservative Unsupported on unverified OS targets (including Android) and removes forbidden cursor-not-allowed CSS, preserving tested native primitives. No fresh Android compile claimed. Parent registered three workspace-move scripts in unit/interaction/Quality GUI and gate docs to avoid orphaned tests. Same-workspace/machine moves only, real-target and rollback preservation boundaries retained. Still no main merge/ship or userApp restart.

## Combined candidate / current checkpoint

Current productd63a38d5 pushed: network identity + direct More navigation + atomic supported-platform rename + scoped drag moves + proper gate registration + frontend domain-cache reuse. Initial More-onlyfb072f46 frontend TS build passed; its Lab-only About numeric/bigint mismatch is fixed inbd5ee1be. Native scoped file-op evidence42610650989 remains green on its tested functions; d7914804 guard/style follow-up reviewed and integrated. Parent reran production SSH script --shell-only on actual case-insensitive APFS: case rename/link/hardlink/conflict checks, rollback injection and raced BSD directory recovery all PASS. No local Rust/Node/browser tests.

New combined GUI run dispatched afterd63 push; record live run ID. It should validate the exact More/profile UI, 22 interaction groups including real FileTree, More settings restoration and Lab state simulation. Frontend now also generates/receipts raw native Lab world data, so Developer Lab no longer recompiles core if receipt/source/payload match. Preview23950 is still old80743a7f; replace only after the new Lab passes. Root promised about10–20minutes to a new Lab if checks are smooth; do not claim release time.

Root owns current source; the one explicitly requested file-op agent has finished (d7914804), no new agents authorized beyond that task. Do not merge main or ship; user is reviewing More + network identity design. Network graph is a presentation mode, not a nested navigation page; Overview is now included in the mode picker. Recovery remains native/capability-gated and actions labeled current-machine-only.

## 2026-11-09 12:29 UTC: exact combined Lab green

Candidate d63a38d5 / trusted Quality42615350693: frontend SUCCESS, developer-lab SUCCESS (completed12:25UTC), GUI unit/types/design/mobile-static passed,22 interaction groups still running at last observation. Downloading developer-lab-review to /Volumes/disk0/Khor/evidence/more-network-gui-20261109/final-lab via exec session37988. Do not launch a duplicate download. Old Python preview23950/session42859 remains old807 until replacement. CUA browser transport unavailable but native Chrome works; inspected current native AX and saw user actively composing in another tab, preserved their state and ended CUA task with completion_summary. Do not steal focus to inspect preview. Product UI automated real-App Lab has passed; user can review new More/profile once trusted artifact is served. No main merge/ship/install/restart.

## Review ready · 2026-11-09 12:33 UTC

Trusted exact d63a38d5 Lab artifact downloaded and manifest checked (production-app, clean source). results.json:56 scenarios,0 failures. Includes actual-App More/profile navigation on desktop and mobile. Screenshot inspection: shared sidebar retained on desktop, native page/back treatment on mobile; profile rendering additionally exercised by automated App checks, no separate profile screenshot was captured. User actively composing in Chrome; native CUA preserved their page and ended without navigation.

Preview23950 was replaced only after checking old listener command belongs to our own pinned Python server. New session16624 serves /Volumes/disk0/Khor/evidence/more-network-gui-20261109/final-lab/developer-lab. URL http://127.0.0.1:23950/app/?scene=devices&machines=3 ; click More then My Khor. HTTP App and referenced entry assets200. No user App/live host touched. Exact42615350693 GUI22-group job still running at last check; frontend, developer-lab, unit/types/gates/mobile-static passed. Existing native file-op scoped evidence and APFS SSH harness are separate; real Wry drag/physical mobile remain manual acceptance boundaries. No main merge or ship authorized.

## User Lab correction: independent Browser + original My Khor map

User rejects Browser inside More, redundant desktop page title/close, the new network profile and oversized My Khor glyph. Implemented shared NavigationOverflow chooser (hidden landings + single More destination) in both FloatNav/LandingStrip; MoreNavigation no longer receives landings, all glyphs including My Khor are bare same-size icons/rows. MyKhorPage map/matrix restored directly, profile components and eight unused catalog labels deleted. Main More Region retains shell/native controls/real PageActions but no duplicate title or page-close; mobile preserves existing title/back and map deck. Same23950 still previousd63 until fresh exact validated Lab replaces it. Tests updated to verify Browser independent route, menu choices, map default, equal glyph geometry, no desktop title/X and mobile return. Added screenshots chooser/map for review. Source-only local, graph MCP Transport closed (fallback targeted diff); TOML/diffcheck pass, remote compile/Lab required. No merge/ship/install or userApp/host mutation.
