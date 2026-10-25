# Windows owned-process acceptance

This standalone crate imports the production Windows process adapter directly. It does not build the Khor workspace or use user sessions.

Run on Windows 10 / Server 2016 or newer:

```sh
cargo run --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml
```

All seven runtime groups must pass. They exercise actual process and descendant handles, Job termination, Drop cleanup, command arguments and UTF-16, environment and cwd, stdio, the explicit child PATH, excluded inheritable handles, and independent cancellation of blocked reads/writes while the child remains alive. A 30-second process-exit fuse closes every owned kill-on-close Job if a check hangs.

Compilation alone on another platform:

```sh
cargo check --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml \
  --target x86_64-pc-windows-msvc --all-targets
```

This cross-check does not execute Windows, link the final App, or prove the runtime groups. Running this executable on a non-Windows host exits with status 2.

The common supervisor is compiled from production source too. Its eighth runtime
check fills stdin, repeats Stop after 3/6/9 seconds, and requires termination
from the original ten-second deadline. Drop checks require signalled retained
process handles immediately when Drop returns, without an exit grace period.
