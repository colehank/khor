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
