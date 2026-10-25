# Native event ownership fixture

`check-native-events.mjs` mounts the production `api.ts::hear` in React and uses the installed, locked `@tauri-apps/api` 2.11.1 event/core modules. It does not replace `listen`, `unregisterListener`, callback storage, or Promise handling. Only native IPC delivery is controlled, so registration eval and invoke acknowledgement can arrive in either order. The same order was observed in the real iOS quality App (Q043).

The small `tauri-2.11.5` source snapshot is third-party Tauri code, Copyright Tauri Programme within The Commons Conservancy, dual licensed MIT OR Apache-2.0. Both original license files are included. It is extracted from the original `tauri` 2.11.5 crate, checksum `667b20e2726d572dea2de7370da16e188eb06008faf9a92fab7cdc46791190b5`, which matches this repository's Cargo.lock. The three source files were compared byte for byte with that crate. Original locations:

- `core.js`: [scripts/core.js](https://github.com/tauri-apps/tauri/blob/tauri-v2.11.5/crates/tauri/scripts/core.js), SHA256 `71056442a732cca1f03572ccae96d68e16104c3a591c64de9618cd304e063070`.
- `event-mod.rs`: [src/event/mod.rs](https://github.com/tauri-apps/tauri/blob/tauri-v2.11.5/crates/tauri/src/event/mod.rs), SHA256 `dfcb7cf11d7674903f71196e64ae0fa95286757ac816bfbc3139ebdedcb25dea`.
- `event-init.js`: [src/event/init.js](https://github.com/tauri-apps/tauri/blob/tauri-v2.11.5/crates/tauri/src/event/init.js), SHA256 `8946488895cc9879f2bf8fee98be454d0f21949b946c264b47d2a2fc84eb1df0`.

`runtime.mjs` verifies those hashes, mechanically extracts the actual Rust format-string bodies, collapses Rust's doubled braces, and supplies test event/id/target/handler values. It supplies the core runtime's `ios`/`http` template values. CI needs no Cargo cache, Rust compilation, or download. The generated cleanup code is the original unguarded implementation; its failure cannot be made favorable by substituting a no-op cleanup engine.

The SDK version/hash check and the real core runtime form the compatibility contract for the adapter's one internal operation, `__TAURI_INTERNALS__.unregisterCallback`. Review this boundary and refresh these tests deliberately when upgrading Tauri. A missing callback-cleanup primitive must fail before allocating a listener rather than falling back to the unsafe SDK cleanup.

Run from the repository root:

```
node apps/gui/scripts/check-native-events.mjs
```

The fixture uses Chrome and WebKit via existing `playwright-core` dependencies. `PLAYWRIGHT_BROWSERS_PATH`, `PLAYWRIGHT_CHROMIUM_CHANNEL`, `NATIVE_EVENTS_PORT` (default 23843), `NATIVE_EVENTS_CHECK_DIR`, and `NATIVE_EVENTS_SCENARIOS` allow isolated verification. It touches no installed Khor App, native services, user files, or HOME. Each case gets a fresh document; successful unlisten is acknowledged only after the controlled native listener has actually been removed. A deliberately rejected unlisten must leave that resource visible and produce a diagnostic while releasing the JS business callback.
