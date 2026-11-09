# Workspace file operations · 2026-11-09

Status: implemented and verified. Branch `fix/workspace-file-ops-20261109`, base `137fc856`. Product and Lab source at `c6df625f` passed [focused CI 42610650989](https://github.com/colehank/khor/actions/runs/42610650989). The temporary branch-specific workflow was removed. A later integration-review patch is recorded below; the earlier green run does not claim compilation of its unsupported-target branch.

## Scope and boundaries

Fix case-only rename and support internal file/directory movement in the production FileTree shared by standalone WorkspacePane and session SideWorkspace. No main merge, release/tag, installation, user App/session-host restart, local Node/Rust/browser execution, or macserver artifact execution occurred. A single read-only macserver connection probe timed out during banner exchange; verification used trusted GitHub-hosted runners.

This worktree contains no AGENTS.md or .codegraph. Parent-provided repository instructions apply. Read debug-issue/khor-design and the design-system README/layout/states/behavior. code-review-graph returned an empty graph (including impact query), so targeted source inspection supplied the call-chain evidence; zero graph results are not evidence of zero impact.

## Implementation and evidence

- GUI `renamePath` → GUI core `files::rename_path` → `Link::rename_path/rename_with` (`link.rs:6751`): local and peer Request::RenamePath call `files::rename_path`; direct/sponsored SSH and Request::SatRename call `satellite::rename_path`. Both roads move on one resolved machine. No copy/import API is substituted.
- The original production SSH script returned `taken` for Methods.md → methods.md on local insensitive APFS, with the original spelling intact (red). Both native and SSH implementations previously rejected every existing target, including a case alias of the same entry.
- Native case detection uses physical parent, exact entry spelling and final-entry identity. A distinct hardlink/symlink is not treated as permission to replace it. General moves reuse `tree::rename_new`; Linux/Apple use existing exclusive primitives, Windows now uses the existing windows-sys dependency's MoveFileExW with no replacement/copy flags. [Windows API reference](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).
- Case changes stage under an exclusive sibling directory. Landing failure rolls back with no-clobber semantics. Failed rollback retains the original bytes and reports their recovery path; staging cleanup never recursively removes user data.
- SSH uses physical-parent/entry enumeration, same-device preflight, `mv -n` and exact resulting inode verification. It refuses cross-device copying. A raced BSD destination directory reports the actual preserved recovery location instead of claiming success.
- FileTree scoped drag metadata distinguishes internal workspace moves from OS/plain/cross-machine imports. Both DOM and shared Wry event paths use the existing move endpoint. Same-parent, self/descendant, known occupied, cross-machine and cross-workspace targets are refused. Existing neutral tokens supply target feedback; no new icons/tokens or alternate tree UI were added.
- Existing onMoved callbacks follow open files/drafts and refresh Git state. Move completion explicitly refreshes both parents, invalidates superseded reads, remaps expanded descendant paths and preserves expansion memory. Error dismissal does not shift targets during the next drag.
- `dev/lab/navigation.ts` now handles rename_path using mutable copies of the finite generated snapshot, including directory subtrees, text/write cache relocation and independent machine snapshots. Old paths stop resolving. `fixture.ts`, `world.ts`, generated JSON, App and Settings were not edited. This is simulation only; it does not claim real filesystem, Git or OS behavior.

## Validation

All jobs in CI 42610650989 passed on the tested source revision:

- Isolated verbatim production Rust rename functions/tests on Ubuntu 24.04, macOS 26 and Windows 2025, Rust 1.95.0. Covers file/directory case spelling, ordinary and hardlink conflicts, exclusive no-clobber primitive, directory moves/descendant refusal; Unix also covers dangling links and physical-parent aliases.
- Production SSH script on Linux and macOS. Local APFS also passed controlled landing/rollback failure and raced BSD-directory fault injection. Linux cross-device check verifies source retained with no copied destination.
- Changed Rust formatting; production bindings generation (compiles the full Linux khor-node dependency), GUI TypeScript and App Lab TypeScript after canonical fixture-data preparation.
- Workspace target model; shared native/DOM source, ordering, cancellation, external imports, dedup and handler isolation; finite Lab rename model.
- Chromium against the real production FileTree with a disk transport fixture: file → directory → root blank, directory subtree move/expansion, source/target refresh, onMoved, invalid/occupied targets and backend refusal. This mounts the production component; it is not a native App end-to-end result.

Standalone retained scripts: `scripts/check-workspace-rename.py`, and GUI `scripts/check-workspace-move.mjs`, `check-workspace-move-lab.mjs`, `check-terminal-drop.mjs`, `check-workspace-move-ui.mjs`. Rust/Node/browser commands remain restricted to trusted CI/macserver; prepare production bindings and canonical Lab data before the Lab checks. The temporary dedicated workflow was removed as requested and must not be integrated into product CI.

## Remaining limits

Actual Wry/AppKit drag gestures/cursors, a real SSH host and user App behavior were not tested or installed. Non-Unix symlink case aliases are conservatively refused. Unix case aliases with another matching inode entry in that parent may also be refused rather than guessed. A process crash between staging and landing can leave original data at `.khor-rename-*/entry`; automatic crash recovery is not added. Arbitrary concurrent parent/mount replacement is not treated as a transactional filesystem API.

## Integration

Apply the full branch range from `137fc856`, not only the final cleanup commit. To keep temporary CI history out of the product integration commit, use `git cherry-pick --no-commit 137fc856..<final-delivery-sha>`, review the accumulated diff, then create one integration commit. The final accumulated diff contains no workflow file. Main/release authorization remains with the parent/user.


## Integration-review follow-up

- Android is an explicit application/file-handoff target (`apps/gui/src-tauri/Cargo.toml`, `file_handoff_supported/open` and `native-files`), while `hosts_here=false` only describes process hosting. `Link::land_tree` and local `tree::copy_into` both publish via `Landing::publish` without a mobile/hosts cfg gate. Other targets also compile this generic helper. The old generic `std::fs::rename` fallback therefore could not safely satisfy the no-replacement contract.
- `tree::rename_new` now fails with `io::ErrorKind::Unsupported` for every target outside macOS/iOS/Linux/Windows. It neither renames nor copies in that branch. No Android syscall/linker assumptions, toolchains or dependencies were added. The four existing exclusive implementations remain unchanged; Android/other uncovered targets must not be described as supporting this exclusive operation.
- Removed all three newly added `cursor-not-allowed` classes from FileTree to obey the existing control-cursor rule. Existing neutral target marking/data attributes and `dropEffect="none"` remain.
- Validation of this follow-up is source-only: Python assertions verify that the fallback contains Unsupported and no filesystem rename call, that FileTree contains none of the prohibited cursor class, and that blocked DOM drops retain `dropEffect=none`; `git diff --check` passed. The prior hosted Linux/macOS/Windows compile/tests cover unchanged native branches, not a new Android cross-compile. A bounded macserver probe again failed during SSH banner exchange, so no fresh Rust/Node compilation is claimed. The parent should run its normal integration checks on the resulting combined source.
- SSH `mv -n` validation remains the scoped evidence above. No general transactional/atomic guarantee is made for arbitrary SSH utilities, concurrent parent replacement or mounts. No temporary workflow was restored.
