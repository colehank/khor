# Q039 — canonical version metadata and verified Doc receive

Owner `doc_outbound_owner`. Source and final runtime evidence independently reviewed without blocker; normal check and22 ordinary tests passed. Author commit3e46b6e4 saved, root integration validation remains separate. Root owns Node/link/proto in a separate existing recovery tree; this slice supplies only library interfaces. No main merge/ship or old Khor compatibility.

## Baseline, ownership and interface

Reuses `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, `fix/quality-doc-transfer` at **9c353fb8c9a27549164835170f13e98c69c86272**, initially clean. Root integrated previous outbound as5da91f0332634d9415ba8641aaa5a1e166258000 and verified normalSync+14 ordinary tests/1070 inputs; that does not substitute for new receive validation.

Exactly seven authorized files: doc_transfer.rs module exports; new doc_transfer/version.rs, receive.rs and body.rs; minimum outbound.rs shared-helper/error adaptation; original tests/doc_outbound.rs typed-oracle adaptation; new tests/doc_receive.rs. No store/apply/transfer/lib/Node/proto/lock/dependency/generated/UI edits. Graph minimal/file-summary consulted first; actual implementation read afterward. No CodeGraph index created.

`DocumentVersion {pub epoch:u32,pub version:VersionVector}` is pure metadata, not a persistence proof. `encode(&self)` and `decode(&[u8])` use `khor-version-v1\0` + BE epoch + exactly the parent's shared write_version/Reader canonical VV implementation. Full value <=64KiB; zero counters omitted in encode, negative rejected; alternate order/duplicates/zero encoded counters/overflow/truncation/trailing/unknown header rejected. No flush is imposed by this value. Node captures a fresh disk-loaded VV; outbound preparation remains responsible for sender flush.

`ReceiveSpec {document:String,source:[u8;32],recipient:[u8;32]}` is supplied by authenticated Node dispatch. `apply_staged<D:Doc>(&mut Loaded<D>, &ReceiveSpec, &mut Sink)->Result<FlushedImport,ReceiveError>` parses canonical descriptor and matches document/source/recipient before progress or body import. It requires VerifiedComplete, reads a local full Vec <=64MiB with each read <=256KiB and loops through short reads/UnexpectedEof, then rechecks canonical binding/actual length/digest/checksummed complete current Loro mode through the single shared body helper. Partial metadata VVs are never treated as full target.

Only then does it construct actual ApplySpec from descriptor epoch/base/target and call real Loaded.apply_checked. Typed Transfer/Body/Apply causes survive, including exact WrongEpoch/StaleBase/Pending/CommitFailed. It does not replace live Doc or swallow apply errors. The only success is the existing private-construction FlushedImport from strict apply. No network Applied/receipt claim, automatic cancel, stage deletion or rollback promise. Body Vec is local and dropped before return; Sink/staged bytes stay with caller on success/error for future Node receipt and explicit cleanup. Receiver peak memory remains full-Vec/Loro, not O(chunk) RSS.

Shared private body.rs::validate_body is extracted verbatim in behavior from outbound: canonical binding,64MiB/length/digest, `decode_import_blob_meta(check_checksum=true)`, exactly current Updates/Snapshot mode. Public neutral BodyError wraps typed transfer/InvalidLoro/EncodingMismatch causes. OutboundError now wraps Body(BodyError), removing the old outbound-specific InvalidBody/EncodingMismatch variants. The original9 case names, fixtures and behavioral assertions remain; only two error pattern matches/import are updated equivalently. No compatibility variants are retained for the unreleased interface. Error source chains are preserved.

## Prepared verification

Eight new ordinary integration cases cover:

1. Normative fixed version bytes, BE epoch, insertion/zero normalization.
2. Every truncation prefix, unknown header/trailing/noncanonical duplicate/order/zero/overflow, negative counters and exact metadata budget boundary.
3. Actual prepare → multi-chunk stage → dropped original source Loaded → fresh strict receiver for both encodings, preserving dirty local concurrency and exact reopened version. A real first256KiB partial stage rejects apply without mutation/disk creation; success retains stage and active cancel stays Busy.
4. Wrong authorized document/source/recipient rejected before incomplete-stage handling; incomplete stage does not merge, then same stage completes and applies.
5. A complete opaque byte stage with noncanonical descriptor rejected before touching Doc/disk.
6. Real body changed after verification fails rehashed digest; corrupted Loro bytes with matching transfer digest fail internal checksum; mismatched and real shallow snapshot modes reject; Doc/disk unaffected and stage remains.
7. Real strict apply WrongEpoch/StaleBase/PendingImport remain exact typed errors and preserve stage/Doc.
8. Actual verified file truncation yields typed UnexpectedEof before import.

New API contract tests, not fabricated existing-bug reds; not a duplicate of the previous22-case strict apply matrix. Rustfmt all seven files and git diff-check pass. No compile/type/runtime green yet, no Cargo/native/gen PID, waiting root review and unique slot.

## Frozen source and recovery

`Q039/doc-receive-candidate-01` in external evidence and same-name internal recovery contains exact seven source files, complete patch including untracked additions, refs/status, and1070 full first-party/build input fingerprints. Source SHA256:

| File | SHA256 |
| --- | --- |
| doc_transfer.rs | 83e554888c1de5f8645532b9baed80ef8d1959e3cee693c942092f0d4e0da3f2 |
| body.rs | 4d42641c5472f45f36f7a269f31b78c50a9dc185d307a180b86f24972c3c6acd |
| outbound.rs | adac0f849e76645a93fba5b7055296530a99107e01d842f043128c7638372f5e |
| receive.rs | b5a36e3da5d35a23110addf5245b382b2049617066950957820d7b8c1d816a5a |
| version.rs | dfe11a5fd0838b91b5dc2d7b11e1c01957933042809ac87bc1a3baded1b3e7cc |
| doc_outbound.rs | 29c5a7cbea2cb58e54cb736314f23ac842ef048dc04f16584f76aa103be1098c |
| doc_receive.rs | a112a9a4ae8a5d401ce6470522be493aa135f61943931d46703d761e7f96dd2f |

Future runner step/total thresholds must be separately named explicit constants and AST-checked before execution; no global text substitutions. Node adoption/real25MB/fair pump/persisted receipt/Pair remain separate open work.


## Frozen candidate validation — green01

Root confirmed formal independent source review,1070/7-source/dual-disk consistency and granted the sole slot. `Q039/doc-receive-green-01` ran default normalSync libcheck → exact8 `doc_receive` integration → original9 `doc_outbound` integration → exact5 `doc_transfer::tests::` codec. All exit0, **22 distinct ordinary tests**, no helper/Doc22/core23 duplication. Stage timings **3.549s / 6.103s / 2.527s / 2.544s**, total **14.978s**. Full actual case names are recorded in logs/results.

Independent literal constants STEP_TIMEOUT_SECONDS=240, TOTAL_TIMEOUT_SECONDS=420, MIN_FREE_BYTES=5368709120 were AST-parsed and checked both before the runner launch and at the runner's start before subprocesses. No global substitutions, no execution deviation or guard trigger. jobs1/locked/offline/quality-target/incremental0, short owned TMPDIR and scratch environment. Minimum external **6,594,011,136B** / internal **12,868,448,256B**.

All **1070** input SHA before=after=current=candidate01; seven exact frozen source hashes unchanged, including the minimal old9 typed-oracle adaptation. **47** sync/catalog Rust/Cargo inputs touched mtime-only with SHA unchanged; raw logs show actual current native-io catalog/sync check/compile, no alternative feature/cfg/backend. Existing vendor HasIdSpan warning remains unrelated.

Actual binary receipts (path/size/SHA/mtime before and after):

| Group | Binary SHA256 |
| --- | --- |
| receive8 | 8365a820eb6c88c609197b040fc8973677413f748106070059b18529df034977 |
| outbound9 | cb43dfcd139dbb657344d4e108145c3b565eaed44be5b24830b74553c721d1d1 |
| codec5 | d65c0277dcce2a82134146a1563ed52288d6a79aca9c7b7a1fa4f28dd5ff64cb |

PGID28921/28947/29001/29020 exited and signal0 confirmed absent; sole slot returned immediately. `/Volumes/disk0/Khor/caches/tmp/doc-receive-01` had zero files, empty receive/outbound subdirectories and root removed. Source/runner/preflight/AST/freshness/actual case names/three binary receipts+after/cleanup/final logs copied to internal recovery same-name run. No further Cargo or source edits; no commit until final evidence review/authorization. Node wire/persisted Applied/25MB/fairness/Pair remain open.


## Final author commit and handoff

Root confirmed final independent review verified1070 input SHA, seven frozen sources, three actual binaries (features[]/rustflags[]),22 exact ordinary case names, AST240/420 thresholds, four absent owned PGIDs, empty/removed scratch and19 matching dual-disk run files. No blocker. Root explicitly authorized exactly the current seven-source submission.

Saved **3e46b6e4a92c146952a3794d85b8dddbf12d4653** (`feat(sync): validate and apply staged document transfers`), direct parent **9c353fb8c9a27549164835170f13e98c69c86272**, branch `fix/quality-doc-transfer` tracked clean. Commit has exactly doc_transfer.rs, new body/receive/version modules, minimal outbound adaptation and the two tests. No underlying core/store/Node/lock/dependency changes.

`doc-receive-green-01/commit-receipt.json` records testedfreeze=gitblob=worktree for all seven full hashes,1070 inputs still unchanged, all four former own PGIDs signal0 absent and scratch absent. Receipt and updated card copied to both evidence and recovery. No new Cargo/native/gen call; commit hook graph update is not runtime validation. Source write ownership returned to root. Any following open_existing/retained_manifests/Outbound.resume work requires its separately defined slice and is not mixed into these seven files/this commit. No main merge/ship.
