> 用户验收修正：新建标签首次打开内容原位转换，不保留额外新建页；以 [R3-review-corrections.md](R3-review-corrections.md) 和最新回归为准。

# R3-03 Shared workspace navigation

State: implementation and author self-review complete; committed with this task card. Root only; no main merge/ship.

- Both main WorkspacePane and the original session ContentPicker share WorkspaceGitNavigation, using the latest accepted Lab token geometry. Session history aligns with original X; standalone switch aligns with New/Search. Branch glyph uses nav-glyph and text-body. FileTray is distinct from workspace identity; no second session strip/deck.
- Session history uses the existing bounded, paginated WorkspaceHistoryMemory/WorkspaceHistory. Separate file/history queries and retained source deck; new-file actions return to files. Non-Git has no Git row; detached checkout retains file/history controls.
- Existing durable content tabs accept renderer `commit`, addressing an immutable full id and repository coordinate as a JSON tuple, alongside existing device/session fields. APIs already resolve subdirectories to repository roots. No backend schema/new metadata fields; actual commit metadata loaded by CommitPage after restart, never fabricated.
- Workspace document selection has one transient preview slot per session inside useSessionContent, reusing its optimistic identities and serialized writes. Existing kept destinations remain kept. Double-click a document tab or editing keeps it; this is independent of the persistent nonclosable pin. Persisted previews reopen as kept documents. Address-field/URL opening retains its earlier same-tab route.
- Commit collapsed-file/reading state is retained by handle+destination, removed with its destination. Mobile commit uses the original session content deck and group navigation.

Evidence: `/Volumes/disk0/Khor/evidence/round3/R3-03`. Actual model red (missing workspace preview policy), model green; Rust commit renderer rejected before allowlist, then all 30 tabs tests passed. GUI unit/type and bounded history memory checks passed. Actual Chrome session integration verifies source retention, preview replacement, tab double-click keep, paging/collapse; history/X center both1370 at1400 wide. Actual standalone WorkspacePane also passed in ui-retry7. Earlier fixture setup failures are retained (label, shell seats, code_here shape), not product regressions. Final rerun required after mobile deck and comment/format cleanup.

An accidental Rust command omitted --lib and started unrelated integration-target builds; stopped own Cargo with SIGINT (exit130), then reran bounded --lib. No user process killed or live app modified; external free91GiB afterward. No release/native build performed.

Final evidence: ui-extended3/report.json has all four actual Chrome groups passed with source hashes unchanged. This includes real CodeMirror editing and retained unsaved draft, standalone WorkspacePane via original shell seats, and390px touch-emulated commit deck/return. Both switch buttons measured36×30; history/X centers1370. types-final and model-final exit0; unit/history passed. New icon viewed in screenshots. Native Mac/iPhone human acceptance remains separate; R3-04 shared preview typography follows after this commit.
