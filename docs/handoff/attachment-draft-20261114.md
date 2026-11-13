# Attachment draft Lab — 2026-11-14

User approved source → files → target → preview/edit → send, with khor-drop as the default. Implemented in production TerminalPane / ChatView and shared AttachmentPicker; this is not a second mock UI. No main merge/release/install authorized.

- Terminal selection, paste and native/browser drops enter a retained draft. Explicit Send prepares all files, lands them through the existing target receive path (default khor-drop), then the existing backend quotes/pastes the paths without a return. Alternate destinations resolve against the target machine.
- Source browsing, selected cards, preview/remove and target are together. The source remains visible on each card. Preview has no per-file send bypass. Browser File drafts can be renamed and bounded text can be edited into a new File; source files are never overwritten.
- ACP picks/drops retain their File/path source until explicit Send. Upload failures retain the original draft for retry; moving the viewing session cannot move a prompt to another owner. Existing files on the session machine are explicitly marked references and retain their source path.
- Ready send uses the existing brand green; blocked/empty send is neutral. Terminal transfer cancellation remains available; only the in-flight item owns its temporary staging files.
- Attachment tasks use shared responsive TaskOverlay placement. Desktop has a toolbar attachment entry; mobile opens from the attachment key. The old inline keyboard-band attachment branch has been removed.

Checks and exact evidence are recorded under /Volumes/disk0/Khor/evidence/attachment-draft-20261114/STATE.md. Focused Chromium/WebKit 1280/390/320 flows passed at3dc773c1; final full-App regression is still running. WebKit synthetic file-list delivery exposed the old unconditional filesystem-entry enumeration; plain files now use FileList, while actual directories retain recursive enumeration. This is browser-fixture evidence, not physical Finder/iPhone verification. Native OS drops, native PDF/share behavior and the system keyboard remain device acceptance items.

Scope: this Lab iteration changes terminal and ACP attachment flows. MachineChat/Khor's separate broadcast/agent attachment adapter retains its existing backend behavior; do not describe that path as migrated by these commits. Image crop/markup and arbitrary binary editing are not implemented.
