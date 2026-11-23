# vendor/portable-pty

Patched copy of `portable-pty 0.9.0` (crates.io), wired in through
`[patch.crates-io]` in the workspace `Cargo.toml`.

## Why

`src/win/psuedocon.rs` hardcoded `PSUEDOCONSOLE_INHERIT_CURSOR` when creating
the ConPTY. On Windows, when the parent process has no console of its own — the
GUI, a service, a test harness, anything started with redirected stdio — that
flag makes conhost deliver **no output at all**:

```text
CreatePseudoConsole(..., flags = 0)                  → "khor-conpty-ok" arrives
CreatePseudoConsole(..., flags = 2 or 4)             → arrives
CreatePseudoConsole(..., flags = 1 (INHERIT_CURSOR))  → nothing
CreatePseudoConsole(..., flags = 7)                   → nothing
```

Measured 2026-11-16 with a raw Win32 probe (`CreatePseudoConsole` +
`PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` + `CreateProcessW` + `ReadFile`) and
reproduced through the host: `cmd.exe /d /c echo hello-host` never reached
`last.txt`.

## What changed

- `src/win/psuedocon.rs` no longer passes `PSUEDOCONSOLE_INHERIT_CURSOR`.
- `src/cmdbuilder.rs` uses native separators when building the executable
  path in argv. `CreateProcessW` can open a path like
  `C:\Windows\System32/cmd.exe`, but the child reads `/c` as a switch and
  exits. This reproduced on both Windows Server 2022 and 2025.

## Removing this patch

Remove the vendored crate and the `[patch.crates-io]` entry once upstream
works without `INHERIT_CURSOR` and passes the native CMD launch test.
