# Q039 canonical DocTransfer descriptor — independent source review

Reviewer `doc_apply_review`, not the root author. **No source or reviewed-evidence blocker identified in frozen candidate-01.** Normal sync lib check and the exact five codec tests now pass on the same two hashes, independently verified below. No Cargo/gen/native or production mutation performed by reviewer. Only this card is written. This is a canonical metadata foundation, not Node transport adoption, body/import/flush proof, large-difference convergence or Q039 completion. No main merge/ship. Earlier unrun status below records the original source-review checkpoint.

## Exact scope and method

Read Q-039-doc-descriptor and current project review constraints. Actual tree `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, branch `fix/quality-doc-transfer`, HEAD `927422502316576b75405a7b65b140b4dc535fd9`. Exactly new crates/sync/src/doc_transfer.rs and one pub module export in lib.rs. Actual/source-copy SHA256 values match `Q039/doc-descriptor-candidate-01/freeze.json` at review start and end:

| File | SHA256 |
| --- | --- |
| crates/sync/src/doc_transfer.rs | 687179dd1a3849c32ff8f48f95d998e6105691fcb3a39682cad0f4b3a33c6c18 |
| crates/sync/src/lib.rs | 275a12d0350a1d2dab2196f6678d8af5b99240f1b836a19ab99280dc548dc06a |

Applied project review-changes skill and graph minimal/detect/file_summary/affected_flows/tests_for/impact before source fallback. Actual graph build is ed460746, older than HEAD92742250; no Descriptor/new-file coverage, and tracked diff sees only lib export. Zero flows are not a test/safety result. No `.codegraph/` exists. Reviewed full new module/tests, existing core Manifest::validate/encode/id/decode, and relevant locked Loro VersionVector equality/FromIterator/raw map behavior. `git diff --check` passes; no concrete type error found, but compilation remains unperformed.

## High/medium findings: none in the declared codec contract

- Binding is explicit domain `khor-doc-v1\0`, UTF-8 name with byte length, epoch, encoding discriminant, base and target; integer fields are fixed width big endian. Writers are sorted as u64, counters encode as positive u32 bounded by their i32 source representation. Source/recipient/length/digest stay in the enclosing existing Manifest exactly once; its ID hashes all those fields plus binding, rather than arbitrary JSON or HashMap iteration order.
- Encoder rejects any negative counter before filtering, omits zero entries, bounds collected positive entries before sorting, and requires final Manifest validation. `.take(MAX_METADATA_BYTES/12+1)` cannot silently truncate an oversize VV into a valid descriptor: even that collected prefix exceeds the byte cap and returns ResourceLimit. Work/output arithmetic is bounded by the small metadata cap, with no hostile unchecked count multiplication or name-length conversion overflow.
- Total metadata policy is preserved. The new inner writer checks at 64KiB, while final Manifest::validate includes the 120-byte core header, so the actual accepted binding maximum is 65,416B; no bound is relaxed. Body size remains governed by existing 64MiB validation. A too-long document or either/both large VVs fail before a Manifest can be returned.
- Decoder validates the enclosing Manifest before parsing. `take` uses slice.get before advancing; fixed integer unwraps follow successful exact-width reads. Hostile u32 name length or VV count cannot trigger proportional allocation from the count alone: name take must fit remaining metadata and version count must fit remaining_len/12 first. Decode rejects unknown domain/revision/encoding, invalid UTF-8 or empty name, zero/overflow counter, duplicate/decreasing writer, truncation and trailing bytes. Remaining byte budget bounds both decoded maps.
- Base and target are deliberately independent/concurrent, with no target>=base restriction. This agrees with the already accepted apply contract preserving receiver/local concurrent work; descriptor parsing does not imply either VV is actually present or durable. Updates/Snapshot are declared formats only; receiver must still verify actual body/import semantics and strict apply boundary.

## Test oracle review and limited independent check

Five ordinary unit tests are appropriate contract checks, not an existing behavior bug's red-to-green claim:

1. Normative fixed binary bytes use concurrent base writer 1/target writer 3 and verify roundtrip. Reviewer independently parsed the actual Rust b-literal with a separate Python schema calculation using struct big-endian primitives: exact 57 bytes, matching domain/name/epoch/discriminant/counts/writer/counter fields. This checks the fixture itself, not execution of the Rust codec.
2. Insertion order and explicit zero writer normalize to identical Manifest and ID; locked Loro map type supports those entries, and its version equality treats absent/zero semantically. The encoder's sorting, not map order, is the canonical wire rule.
3. Identity alternatives independently modify document/epoch/encoding/base/target and enclosing source/recipient/length/digest; every alternative must change the core Manifest ID. No field is inferred from body bytes or accidentally excluded by a separate hash.
4. Every proper truncation prefix rejects. Additional cases alter domain, UTF-8, encoding, count=u32::MAX, zero and overflow counters, trailing bytes, and swap/duplicate two writer records. The second duplicate construction is correct: after swap, restoring only the first original record yields two identical writer records rather than accidentally restoring a valid vector.
5. Negative VV, empty/oversize document, >metadata VV and >body budget reject. These assertions use actual public bind/from_manifest calls, not a duplicated test encoder. Eq and FromIterator traits used by Descriptor/tests exist in the locked Loro source; this inspection is not typecheck proof.

## Low-risk coverage limits and caller boundary

No distinct exact-limit/max-i32/huge-name-length decode or normalized non-ASCII name fixture is present. Relevant guards are statically simple and no defect is inferred from absent cases; new tests need not block the first narrow compile/run. There has been no Rust runtime result for this frame.

Document names are exact UTF-8 bytes; the module does not claim Unicode normalization, path safety or document authorization. Node must select/authorize the actual doc before staging and must not use arbitrary descriptor text as an unchecked filesystem path. Likewise authenticated identities, digest/body verification, fixed spool ownership, actual encoding, epoch/VV checks and FlushedImport/remote receipt handling remain separate layers. The module currently has no Node/wire caller, no network fallback or legacy matrix. These limitations are explicit and are not newly invented blockers for this codec slice.

## Handoff

Root may schedule the five codec tests and appropriate sync type/core checks on the exact frozen frame after the current owner releases Cargo. Do not count rustfmt, independent 57-byte fixture calculation, skipped tests or older transfer/Doc greens as this frame's behavioral evidence. Future spool/Node adoption must bind this same canonical descriptor to actual authorized document/bytes and retain the agreed concurrent base/target semantics; this review alone cannot close that later work.

## Independent exact-run evidence audit

Read actual `Q039/doc-descriptor-green-01` run.py, preflight/freshness/results/final/cleanup records, normal-lib.log, descriptor.log and binary receipt. No Cargo or test rerun. The runner performs ordinary default `cargo check --locked --offline -j1 -p khor-sync --lib`, then ordinary `cargo test --locked --offline -j1 -p khor-sync --lib doc_transfer::tests:: -- --nocapture --test-threads=1`; no alternate backend, mock codec, special cfg or feature bypass. KHOR_AGENT_BUNDLE is cleared rather than substituted.

Independent comparisons establish all **1,063** source-before/source-after entries are identical, both frozen source hashes are present unchanged, and all current 1,063 files still match. All **40** selected sync/catalog source/Cargo input mtimes were refreshed with unchanged bytes. Actual normal log checks this tree's khor-sync and compiles catalog; actual test log recompiles both khor-catalog and khor-sync at `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`. This is real current-tree compilation, not just a previously cached binary receipt.

Normal lib check exits 0 in 18.142s. The descriptor log runs exactly the five inspected tests by name: fixed concurrent byte contract, every-field identity, insertion/zero normalization, negative/metadata/body overflow refusal, and malformed/noncanonical/truncation refusal. Rust summary is **5 passed, 0 failed, 0 ignored, 147 filtered out**. Descriptor phase exits 0 in 4.536s; final runner records complete=true, stable source and 23.030s total. There is no bug-red claim for this new interface.

Actual log `Running unittests` names quality-target/debug/deps/khor_sync-6bc0f9688422d392. Receipt SHA256 is `5d3770603eda52c61db8eebe3cc84b47a8d974f2ed816c2520a8800a633c2c3f`; the reviewer independently hashes that live executable and obtains the same value. All steps stopped=null; minimum external 6,903,496,704B/internal 7,131,127,808B remains above 5GiB. Saved cleanup lists PGIDs 66721/67124 absent and empty scratch then rmdir; independent current queries find both groups gone and scratch absent. No process was signalled or artifact removed by reviewer.

**Updated handoff: root may commit exactly the reviewed two source files on fix/quality-doc-transfer.** The matching source and actual narrow type/behavior evidence have no independent-review blocker. No further low-risk test expansion is requested for this slice. Main merge/ship, spool/Node wire adoption and full Q039 completion remain unapproved/unproven here.
