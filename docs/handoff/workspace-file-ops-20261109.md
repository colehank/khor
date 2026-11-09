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

## Implementation checkpoint

- `f889c59d`: initial implementation and focused harness.
- `429259cd`: dedicated branch-only CI trigger (new workflow cannot dispatch before it exists on default branch).
- `444fbe89`: production FileTree Chromium regression; Rust formatting corrections.
- Native rename reuses `tree::rename_new` (Linux renameat2 RENAME_NOREPLACE, Apple renamex_np RENAME_EXCL; Windows std rename already refuses replacement). Case aliases require equal physical parent, absent exact destination entry, unique lstat identity in that directory, and equal source/destination identity. Another hardlink is an occupied entry. Case changes park under an exclusive temporary directory; failed rollback never recursively deletes user data.
- SSH uses directory enumeration and inode identity without following the final symlink; same case staging, `mv -n`, safe cleanup and explicit recovery path. GNU -T/BSD -h retain terminal symlink handling. This is the existing SSH move endpoint, never a cross-machine copy API.
- FileTree drag scopes contain machine, root and directory kind. Only the same workspace/machine can move; OS paths and plain/cross-machine drags retain original owners. Target directories/root blank use existing neutral tokens; native hover uses shared listener; native/DOM source consumption stays deduplicated. Existing onMoved updates open content. Both parent listings are explicitly refreshed; expanded descendant paths follow moved directories.

## Evidence through 444fbe89

- Original production SSH script on local insensitive APFS returned `taken` for Methods.md → methods.md (red).
- `python3 scripts/check-workspace-rename.py --shell-only`: green on local APFS, including symlink, physical parent alias, existing names, descendant refusal, quoted names and staging cleanup. Added fault injection also confirms rollback and retained recovery data when rollback fails.
- GitHub run https://github.com/colehank/khor/actions/runs/42605089493: Linux and macOS isolated production Rust rename tests, production SSH checks and changed Rust formatting passed. Windows harness stopped on UTF-8/default-codepage decoding before compiling; fixed in current uncommitted follow-up. GUI generating production bindings, not yet validated.
- Earlier run 42604711106 first exposed formatting changes; its matrix siblings were cancelled by default fail-fast. The current matrix disables fail-fast. No app installed/restarted, no release/tag/main merge, no local prohibited runtime executed. Git hooks were disabled per command because their Rust/Node checks violate source-only restriction; equivalent relevant checks are delegated to trusted CI.

## Remaining before integration

Push current follow-up (UTF-8 harness, stale-listing invalidation, native hover consumer isolation), obtain Windows and GUI results, fix any real failure, update this record and provide cherry-pick range. Browser regression mounts the actual FileTree with transport fixture; it does not establish native AppKit/Wry end-to-end behavior. Real App verification remains prohibited by current task authorization.
