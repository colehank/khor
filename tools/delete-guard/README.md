# dsh-delete-guard

A DSH bundle that refuses tool calls which name the Windows system drive, a user
profile path, or a user-profile environment variable. It runs on the
`tools/pre-execute` waterfall, so it applies to every executor and remains in
force under `danger-full-access`, where the ACL sandbox is not consulted.

Rules:

- Any command naming `C:`, `$HOME`, `%USERPROFILE%`, `%APPDATA%`, `%LOCALAPPDATA%`
  or `%SystemRoot%` is denied; only a single unchained read-only command may name
  them.
- A destructive command must use literal paths (no variables, globs, or `..`).
- `write`/`edit` may not target the system drive.
- `plugin_manager` may list, and may not change the profile from inside a session.

The guard is a policy layer, not a kernel boundary: a human terminal does not pass
through it, and `subagent_codex` runs a native Codex child whose commands never
reach this pipeline.

```sh
node test/guard.test.mjs && node test/host.test.mjs
```
