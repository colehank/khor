# Q039 — canonical document transfer descriptor

Owner root. Working, frozen candidate for source review, no Cargo/typecheck/runtime evidence yet. This is the next interface slice for the single current Node DocTransfer; no legacy compatibility/dual path, no main merge/ship.

## Worktree and scope

Reused `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, preserved old fix/quality-sync-chunked@78e978a8 ref; new branch `fix/quality-doc-transfer` from integrated927422502316576b75405a7b65b140b4dc535fd9. Exactly new `crates/sync/src/doc_transfer.rs` and one `pub mod doc_transfer` in sync/lib.rs. transfer core, Doc apply, Node, proto, locks/dependencies unchanged. Graph-first attempted, indexed1d2 missing current transfer/round symbols, so reviewed actual source; no coverage inferred from graph miss.

## Contract

Existing byte Manifest binds authenticated source/recipient, total length and body digest. New Descriptor binds document UTF-8 name, epoch, encoding (complete Updates or Snapshot), fixed base and target VV into its opaque binding. Domain is `khor-doc-v1\0`; lengths/counts/epoch/writer/counter big-endian, writers strictly increasing and counters positive <=i32::MAX. Encoding omits zero-count writers, rejects negative counters. Decode rejects alternate order/duplicates/zero/overflow/truncation/trailing bytes/unknown revision or encoding, under existing total64KiB metadata and64MiB body budgets. No serde serialization/hash of arbitrary JSON or Loro HashMap order.

Base and target may be concurrent. Doc authorization remains Node dispatch before staging; descriptor parsing alone grants no authority and proves neither body format nor import/flush/Applied. Body content/Loro encoding checks, immutable outbound spool, stage ownership, receiver checked apply and final receipt are following slices; this candidate does not implement them. One field source participates once in canonical binding; body identities/length/digest are already in the core manifest/id, not redundantly added to binding.

## Prepared verification

Five ordinary unit tests cover normative fixed bytes+concurrent versions, insertion-order/zero normalization, identity changes for every descriptor+manifest field, all truncation prefixes and malformed/noncanonical encodings, negative versions/metadata/body limits. These are new codec contract tests, not an existing bug red. Rustfmt and diff-check passed; independent Python schema calculation matches the fixed normative fixture only, not library runtime proof. No Cargo/gen while MCP/init own slot.

Freeze `Q039/doc-descriptor-candidate-01`: doc_transfer SHA687179dd1a3849c32ff8f48f95d998e6105691fcb3a39682cad0f4b3a33c6c18; lib SHA275a12d0350a1d2dab2196f6678d8af5b99240f1b836a19ab99280dc548dc06a; complete patch+source+refs in evidence and internal recovery/doc-descriptor-candidate-01. Pending independent source review, exact type/runtime checks, and Node wire adoption before this foundation can close current-transfer goal.

## Exact candidate validation and independent evidence review

`Q039/doc-descriptor-green-01`: default normal sync libcheck0 (18.142s) and five exact codec unit cases5/5 (4.536s), total23.030s. No alternate features/backend/cfg. Current-tree catalog and sync actually recompiled,40mtime-only touches SHA unchanged;1063 inputs before=after=current. Test binary5d3770603eda52c61db8eebe3cc84b47a8d974f2ed816c2520a8800a633c2c3f. Minimum external6,903,496,704B/internal7,131,127,808B; no guard. PGID66721/67124 gone, scratch empty then removed. Complete logs/producer/fingerprints/cleanup in external evidence and internal recovery/doc-descriptor-green-01.

Independent reviewer source+real logs/oracles/input/binary/PGID checked without new blockers, seeQ-039-doc-descriptor-review.md. Fixed2source SHA unchanged. This is a new codec contract, not a claimed bug red. Next save exact2source commit; do not count this foundation as Node/25MB convergence.

Saved author263df47df30879040189f9220f2a3af56607dcc7, replayd16d83df3e5288fac99384c6b75663ab8cee1681; both tracked clean and tested2SHA identical. Current4493 combined run includes normalNode/CLI/front producer and these same5codec cases (part of62 different ordinary parents), all source2230stable. WholeNode adoption still open.
