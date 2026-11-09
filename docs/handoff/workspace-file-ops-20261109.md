# Workspace file operations · 2026-11-09

## Scope and authorization

Independent branch `fix/workspace-file-ops-20261109`, base `137fc856`.
Fix case-only rename and add internal file/directory moves to the production FileTree shared by standalone and session workspaces. No release, main merge, installation, app restart or host restart.

## Evidence

- This worktree contains no AGENTS.md or .codegraph. Parent provided repository instructions apply.
- Read debug-issue and khor-design skills and design-system README/layout/states/behavior.
- code-review-graph minimal context and semantic search returned an empty graph (0 nodes/0 files); targeted rg fallback used.
- GUI renamePath invokes local/peer files::rename_path or SSH satellite::rename_path. Both reject any lstat-visible destination before rename, including case aliases of the same entry.
- Standalone WorkspacePane and session SideWorkspace both render FileTree. Existing onMoved callbacks update open descendant file paths and Git state.
- path-drag/window-drop own shared DOM/native drag source and deduplication. Wry can intercept internal drops, so native target routing is required; external OS imports must remain unchanged.

## Status and next steps

Implementation in progress. Confirm same directory entry using physical parent and directory enumeration, not inode equality alone. Reuse atomic no-replace native rename and stage case changes with rollback. Add FileTree internal target handling and focused verification.

## Validation restrictions

Local machine is source-only: no Node/npm/Bun/Rust/Swift/Xcode/browser CLI/Playwright. Git/Python/read-only OS commands allowed. One macserver connection probe currently pending; use bounded retry or focused trusted GitHub CI. Browser checks and native app checks must be reported separately; user app must not be installed/restarted.
