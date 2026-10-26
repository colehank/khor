# Q039 — immutable outbound Doc spool

Owner `doc_outbound_owner`. Author slice implemented, normal checks +14 ordinary tests and independent source/runtime evidence review passed; saved as9c353fb8. Root integration validation remains separate. No main merge/ship; no old Khor compatibility path.

## Scope and current baseline

Reuses `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, branch `fix/quality-doc-transfer`, HEAD `263df47df30879040189f9220f2a3af56607dcc7`, initially tracked clean. Author writes exactly `crates/sync/src/doc_transfer/outbound.rs`, minimal module/re-export in `crates/sync/src/doc_transfer.rs`, and `crates/sync/tests/doc_outbound.rs`. No edits to core/store/apply/lib/Node/proto/locks/dependencies/generated/UI files. Central STATE belongs to root.

Current integration has descriptor and strict Doc apply; initialization recovery is a separate independently owned slice with unchanged public core API. This tree is not rebased/overwritten with that work. Graph-first minimal+file summary/semantic search preceded source reading; index built on9274 does not cover Loaded queries, actual current source used. `.codegraph/` absent, no indexing performed.

## Contract

`PrepareSpec` contains document, authenticated source/recipient bytes, receiver epoch, base VV and Encoding. `prepare<D:Doc>(&mut Loaded<D>, &PrepareSpec, &TransferStore)` returns `Prepared::UpToDate {epoch,version}` or `Prepared::Transfer(Outbound)`.

Preparation rejects wrong epochs and invalid/overbudget descriptor metadata before flush; captures full target, calls actual BlockStore flush, and rejects changed target. If receiver base covers target, no body export/stage is produced (a dirty flush may itself export its block). Otherwise actual Updates or full Snapshot export runs synchronously, target is checked again, actual body length must be <=64MiB, checksum metadata decoding must match complete current Updates/Snapshot mode. Shallow and outdated modes are refused. Partial metadata VVs never replace full captured target; concurrent base/target are valid.

The private reusable `validate_body(descriptor,manifest,bytes)` checks canonical descriptor binding, body budget/length/digest and checksummed Loro mode. It does not prove target coverage/pending absence/Applied. Same implementation is available inside doc_transfer for later receiver work.

`Outbound` owns fixed Descriptor + core Sink (manifest single source) + clone of the original TransferStore root. It retains no Doc/Loaded/pile lease or full export Vec. Preparation resumes the same core manifest at confirmed offset, appends canonical 256KiB/final chunks, requires VerifiedComplete, drops full body Vec. Loro still allocates a full export Vec before the accepted-size check; this slice makes no bounded-peak-Loro-memory claim.

`read_chunk(at)` accepts canonical aligned offsets or EOF; returns a full canonical chunk <=256KiB, looping through short reads, with UnexpectedEof for truncation. Dropping Outbound releases core owner and retains stage/quota for resume. `discard(self)` clones its own store/manifest, drops Sink then invokes same-root core cancel, retaining active-owner/Busy semantics and never rolling back a Doc. Staging errors carry the exact manifest plus core cause for explicit cleanup/retry; they do not automatically delete potentially existing/recoverable stages.

One TransferStore enforces the existing 128MiB/64stages/2owners limits; no separate spool/quota/GC policy is introduced. Node dispatch/authorization, fair pump, receiver checked apply and persisted Applied receipt, Pair bootstrap, real25MB Node convergence remain following slices.

## Prepared verification

Nine ordinary contract tests use real PinDoc/Loro, BlockStore, strict apply and TransferStore:

- Both actual multi-chunk modes survive source Loaded drop; late flushed local op is excluded; body checksum/chunks/metadata and fresh strict apply/reopen agree with fixed target.
- Concurrent base and target plus actual partial metadata VV preserve full target and both branches on strict apply.
- Wrong epoch/negative version/metadata overflow reject before flush; receiver-covering target flushes dirty local data but creates no stage.
- Same-manifest partial stage resumes; drop releases owner, active cancellation stays Busy, explicit discard preserves another published stage.
- Observable real Loro export boundaries demonstrate flush before both modes and no second export for UpToDate.
- Real Loro-based contract fixture detects mutation during flush/export, corrupt bytes, wrong mode and shallow snapshot without staging.
- Staging Busy exposes exact manifest and retries after owner release.
- Real filesystem flush failure creates no stage.
- Actual post-prepare body truncation yields UnexpectedEof rather than a short successful chunk.

These are new API contract tests, not fabricated existing-bug reds. Rustfmt and git diff-check passed. No Cargo/gen invocation, no typecheck or behavior green yet; waiting root's unique slot authorization and independent source review.

Freeze `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/doc-outbound-candidate-01` and internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/doc-outbound-candidate-01` contain exact three source files, complete patch (including untracked sources) and fingerprints. SHA256: doc_transfer `152e4dbeab2e426b74d35639410e6cff371e426a3aaa1289ac6675b8e9244073`; outbound `64480d7a94af53e7f3a45d93270c3f98f947cf4c6825b0b12655308c8dbc444e`; tests `1d7fc933f4b1a71f2f3adc4857f13f2e3ed2fa0f907c2426b8d74424b6e4f493`.

## Initialization dependency replay and candidate02

Root verified integrated initialization dependency with normalSync and28 different ordinary parents, then explicitly authorized its nonoverlapping replay while outbound source remained dirty/frozen. Commit419f73c52bebae6be5e66dfd39d160326a914143 is replayed as **e5b0aa46bbc34e020a8cab0d82f482030bf90af8**, direct parent263df47d. Exactly `crates/sync/src/transfer.rs` and `crates/sync/src/transfer/platform.rs`, each byte-identical to419. Three outbound SHA from candidate01 remain unchanged; still only those three pending files. No stash/rebase/overwrite/Cargo.

Initial precheck guessed the platform path incorrectly as `crates/platform/src/atomic.rs`, raised AssertionError, and shell proceeded with the already authorized cherry-pick. The subsequent postcheck verifies actual two paths, no overlap, direct parent, dependency byte identity and all three frozen outbound SHA. The failed check is a tooling mistake, not a product red; future dependent shell operations use one checked script or fail-fast gating. This is recorded in candidate02 metadata.

`Q039/doc-outbound-candidate-02` and internal recovery same name contain three exact sources, original complete outbound patch, refs/dependency receipt and **1066** first-party/build input fingerprints. Source review continues against unchanged candidate01/02 three SHA. Unique Cargo slot remains unclaimed; prepared runner exists outside the freeze, syntax checked only, planned normalSync +9 outbound +5 codec. No redundant Doc/core suite without a new affected concern.

## Exact frozen candidate validation

Independent source reviewer found no blocker in candidate01; root verified replay e5b0aa46 + exact three dirty source hashes and authorized the unique slot. `Q039/doc-outbound-green-01` runs the unchanged candidate02 with normal default sync libcheck, `--test doc_outbound` exactly9 ordinary integration cases, and `--lib doc_transfer::tests::` exactly5 codec cases. All exit0: **3.024s / 6.075s / 2.523s**, total **11.783s**, 14 different ordinary tests. No alternate backend/features/cfg; no repeated Doc/core suite added to counts. Existing vendor HasIdSpan unused-import warning remains unrelated.

Current native-io catalog/sync actually compile/check in raw logs. 43 current sync/catalog `.rs`/Cargo inputs are touched mtime-only with byte SHA equality (exact freshness receipt is authoritative), **1066** complete first-party/build inputs before=after=current; all three outbound and both419 dependency hashes unchanged. Actual outbound binary `doc_outbound-761610bf76283ee7` SHA **f8d147e106b7f15f7773a8bbafa2ddc24fdb04943ef9a79ec8080cc70bf892cd**; codec unit binary `khor_sync-6bc0f9688422d392` SHA **22bd496850a4b1bee856f1f54461380f8d4849363c1e9bb11b4f22a0b924a669**.

PGID97850/97936/98083 exited and signal0 confirms absent; short owned scratch `/Volumes/disk0/Khor/caches/tmp/doc-outbound-01` contains no files, empty subdir/root removed. Minimum external **6,732,103,680B** / internal **12,893,388,800B**, above5GiB. Slot returned immediately after tests, no current Cargo/native/gen processes owned by this task. Logs/runner/preflight/freshness/input/binary/final/cleanup complete in external run and internal recovery same-name copy.

Execution deviation, not hidden or described as requested protection: runner text substitution matched `step_start` suffix when setting total420, so actual single-stage threshold was420 instead of requested240. Longest stage was6.075s, total11.783s, no guard triggered; disk/3IO/ownedPGID guards stayed active. A later independent240s monitor attached after run completion (empty PGID list), and is explicitly **not** counted as effective runtime protection. Original executed runner retained unchanged; `runner-deviation.json` records this. Root notified immediately. No test/source edits followed the run; behavior results are real greens, not bug-red claims.

Final runtime evidence review/commit authorization pending; no commit of the outbound slice yet. This does not close Node/25MB current-transfer convergence, receiver Applied receipts, fairness or Pair bootstrap.


## Final author submission

Root confirmed final independent source/evidence review found no blocker and authorized only the tested three-source submission. Commit **9c353fb8c9a27549164835170f13e98c69c86272** (`feat(sync): spool immutable outbound document transfers`), direct parent **e5b0aa46bbc34e020a8cab0d82f482030bf90af8**, contains exactly the minimal doc_transfer module/re-export, new outbound module and nine-case integration file. Core initialization dependency is **not** mixed into this commit again.

`doc-outbound-green-01/commit-receipt.json` records testedfreeze=gitblob=worktree for each full three SHA, all1066 inputs still unchanged (including lock and two dependency sources), branch `fix/quality-doc-transfer` tracked clean, no own PGIDs or scratch left, no additional Cargo invocation. Receipt and updated card copied to both evidence/recovery. Commit hook updated the local graph only; its generic test-gap heuristic is not an additional test/validation result.

The runner's420/240 threshold deviation remains preserved with original raw evidence and has been independently reviewed without requiring a repeated test run. For the next runner, step/total thresholds must be explicit separate constants checked after parsing; no broad text replacement. This author's outbound slice is delivered. Root will integrate exact three-source commit and run its new normalSync +9outbound +5codec combination. Current Node wire adoption/real25MB fairness/Applied/Pair work remains open.
