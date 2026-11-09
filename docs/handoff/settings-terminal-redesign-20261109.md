# Settings / Terminal / Notifications redesign · 2026-11-09

## Authority / resume
User authorized this work, serial root only. Active worktree /Volumes/disk0/Khor/worktrees/current/khor-settings-gui-20261109, branch integrate/settings-gui-20261109. Starting HEAD98ff9336 (product UI df468c97, test follow-up fe3aa665). Publication held: no main merge/ship/install/user App or live-host restart. Local remains SOURCE ONLY: no local Node/Bun/npm/Rust/Xcode builds or browser CLI/AppleScript. Git/Python/metadata/archive reading allowed; trusted public GH builds/tests and native CUA allowed. Graph MCP repeatedly Transport closed, targeted source fallback documented. khor-design/README already read. Do not spawn agents.

## User requirements (latest supersede older hierarchy)
- Remove unnecessary “设置” sidebar caption and large blank space above both sidebar groups and right contents. Previous right-title deletion left empty Region header; remove useless reservation without breaking folded nav/native Windows controls.
- Sidebar grouped by blank space, EXACT order: 会话 / 工作区 / 外观 ; 连接 / 安全 / 存储 / 诊断 ; 关于. User corrected “机器” was a typo: “打错了, 无需机器.”
- Tighten icon-label gap (currently wide avatar slot + row gap); meaningful content section headings such as 头像 should be larger/stronger. Use existing tokens, not invented pixel claims about HIG.
- Session settings fully expanded, NO Terminal child page/menu entry: groups 通用 and 终端. Keep old terminal deep-link compatible by routing to Session/Terminal section.
- Terminal appearance first block in precise order: 配色、字体、字号 (editable number + bar; no separate small preview)、可读性增强 (no explanatory adjustment row)、光标样式、光标闪烁、光标颜色 (NEW)、完成提示 (rename bell, no note)、命令块分隔 (no note). Bottom of this block ONE current-config terminal preview, theme + font + size + cursor all applied live.
- Theme selector includes 自定义. Selecting custom reveals one action row 导入 / 设置. Import existing JSON capability retained; 设置 opens editor. Fix editor preview collapse/overlap shown screenshot. Retain existing presets/import/export/removal capabilities where appropriate without four permanent action buttons.
- Terminal functions second block: 快捷键行 (rename Terminal key row)、缓存行数、预测回显 (no note)、Option键作为Meta键. Preserve applicable pointer/platform options without clutter or silently losing user settings.
- Shortcut editor: add chord through complete VISUAL keyboard template/library, not base-key dropdown + modifier list. Mac/Windows labels/layout differ appropriately. Remove test/try UI. Clear selection/modifier feedback, combination/sequence editing, reorder/delete retained, accessibility. Must preserve codec/protocol meaning; don’t pretend Cmd/Win can be serialized to PTY when unsupported. Research upstream supported keys/platform before choosing.
- Agent recognition bottom panel: trace runtime status/type detection vs legacy hook/notify cleanup; user recalls hooks not required and asks if UI is necessary. Remove redundant primary setup UI if runtime doesn’t depend on it; retain safe migration/cleanup logic and required runtime contract.
- Notifications correction (explicit): NOT an inbox or saved notification history. APP-INNER transient notices like system banners: Desktop top-right, Mobile top. This round must implement. No Notifications page in More, no saved notice-content archive. Auto-dismiss + close; actionable session events should open actual session without stealing focus, queue/dedup/replay must avoid storms. Session unread markers remain separate from transient banner lifetime. No APNs/server introduced.

## Serial implementation plan
1. Map current shared Region/MoreFrame layout, TerminalSection/theme preview/cursor appearance, key model/codec/editor, notification controller and backend recognition. Capture decisions grounded in source.
2. More sidebar exact grouping, tighter glyph slot, heading typography, remove blank desktop header where no native/folded chrome. Session expanded composition and terminal deep-link compatibility.
3. Refactor Terminal appearance controls into coherent reusable rows, shared actual-xterm preview that tracks theme/font/size/cursor; fix dialog flex sizing; add cursor override through shared appearance path + persistence/migration.
4. Theme custom flow and desired Terminal functions order/labels; no duplicate font preview/hints specified by user.
5. Keyboard library/template evaluation -> adapter for existing key model/codec; accessible visual layout, Mac/Windows distinctions; remove test pad; preserve combination/sequence edit semantics.
6. Ephemeral in-app banner controller/host, top-right/top position, proper dedup and open/close, no inbox UI/history persistence. Trace recognition and remove obsolete setup surface only.
7. Focused meaningful tests for defaults/migration/appearance propagation, actual renderer preview geometry, key editor interaction/encoding, notification lifecycle/no persistence, exact More/Session hierarchy and foreground mobile navigation. Build trusted GH then serve same Lab23950, show explicit pending/native boundaries; no release.

## Current reference/evidence
- Live23950/session84233 currently serves prior df468c97 hierarchy (Terminal child; to be superseded).23951/session68652 older2a;23952/session34486 df inspection, preserve user Safari state.
- Native Safari verified independent My Khor map/back/menu; Terminal child clicks blocked by user-change guard. CUA summaries completed; don’t force current user window.
- Previous UI runs had test selectors/strict menu-bound comparisons, not proven navigation product failures. Latest trusted42627749202 status must be retrieved, no all-green claim. Older42615350693 includes unresolved WebKit reader resize/upward-wheel assertion before these requests; must resolve before eventual release.
- Evidence under /Volumes/disk0/Khor/evidence/more-network-gui-20261109, latest compiled under standalone-session/compiled-lab.
- Keyboard primary candidate researched: hodgef/react-simple-keyboard MIT, upstream docs support layout/display, theme/buttonTheme/buttonAttributes, useButtonTag, disableButtonHold, physical keyboard options. Do not rely on scraped secondary descriptions. Verify registry/source and key model before adding dependency.

## Decisions / progress
- No machine settings page (user correction).
- No notice inbox: user explicit clarification overrides earlier persisted Inbox design.
- Product edits are implemented in this worktree, not yet remotely compiled: exact More groups/headerless form Region, expanded Session/Terminal, reactive shared real-xterm preview/flex fix, custom theme flow, per-viewer cursor colour, visual keyboard via pinned MIT react-simple-keyboard3.8.296 (metadata/integrity/license read without executing locally), safe existing key model/codec preserved, touch settings nested in row editor, Codex exact-thread native-title association repair moved to lazy Diagnostics fold (no hooks), transient banner controller/view with metadata-only persistence and migration deleting old inbox content. No Notifications page/nav badge/history. Session unread remains independent.
- Updated meaningful unit/UI checks for expanded hierarchy, preview, visual keyboard/sequence and transient lifecycle. Local Tree-sitter/TOML/JSON parsing passes so far; no local JS/Rust build or tests run. Need trusted remote compiler/tests, inspect actual artifact and update Lab23950 only after success.
- Previous42627749202 completed: frontend and GUI passed, Developer Lab failed. Its old hierarchy checks are superseded by this new expanded design; retrieve bounded failure details if still relevant.
- Pending: validate types/build, new UI behavior/geometry, exact new Lab acceptance. Native touch/keyboard/notifications integration still explicit review boundary; publication HOLD unchanged.
