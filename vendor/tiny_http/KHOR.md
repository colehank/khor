# tiny_http 0.12.0: bounded unread-body cleanup

Source: crates.io `tiny_http` 0.12.0, MIT OR Apache-2.0. Both license files are retained. The cached `.crate` archive SHA-256 was verified against the original Cargo.lock checksum:

`389915df6413a2e74fb181895f933386023c71110878cd0825588928e64cdc82`

Upstream revision recorded by the published crate: `212b1c45852fef2093dc1374875a9393c55eb4b9`. `UPSTREAM.json` records every copied original file hash. All 43 published source/manifests/tests/examples/benches/documentation/license files needed for this vendored copy are retained. Registry installation markers, CI files, and the upstream development Cargo.lock are omitted. No dependency versions or default features change.

## Patch

`src/util/equal_reader.rs::Drop` previously allocated a Vec as large as the unread Content-Length on each drain iteration. Rejecting a JSON body early therefore allowed the HTTP request destructor to allocate the attacker's declared remaining length. Cleanup now uses a fixed 8KiB stack buffer and takes the smaller of that buffer or the declared remainder. It still drains exactly the remainder and preserves original EOF/error/channel notification semantics. Three local unit tests cover arbitrary declared length, exact partial drain, and early EOF; the original two unit tests remain.

This patch bounds cleanup memory. It does **not** add a deadline, abort a slow connection, limit network bytes drained after a rejection, or change tiny_http's keep-alive behavior. Application JSON budgets live in crates/web/src/json_request.rs; raw upload limits remain with their existing handlers.

Remove the workspace patch and this directory when a published upstream release fixes cleanup allocation, then rerun the request-lifecycle and Web HTTP input tests against that release. Upstream master was also observed to retain the old allocation on 2026-10-25; do not replace this patch with an unverified upgrade.

## Verification

Lightweight source test (no dependency build):

```
rustc --edition=2018 --test vendor/tiny_http/src/util/equal_reader.rs -o /tmp/khor-tiny-http-cleanup-test
/tmp/khor-tiny-http-cleanup-test
```

Workspace validation uses `cargo test --locked -p khor-web --lib -j2 -- --test-threads=1`, with a built frontend available as required by crates/web/build.rs. No real user command is invoked by the loopback tests; their only accepted command is the static console catalog.
