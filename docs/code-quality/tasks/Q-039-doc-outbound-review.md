# Q039 — independent outbound Doc spool review

Owner: `doc_apply_review`, not the implementation author. Source review and candidate02 green-01 evidence review completed without blocker. Normal Sync check plus nine new outbound and five existing codec tests are confirmed for the exact frame. No Cargo, generator, fixture execution or production changes by this reviewer. Root may authorize commit of the three exact outbound source files; Node adoption, integration validation and main/ship are separate.

## Reviewed frame and method

Read central STATE/PLAN and the outbound contract. Applied project `review-changes` skill: minimal graph context, detect changes, affected flows, tests query and impact radius before actual source fallback. The graph was built on 92742250, omitted the new outbound/tests, and the `prepare` test query incorrectly resolved an unrelated GUI symbol. Its zero flows/gaps/risk provide no safety evidence. No `.codegraph` directory was present and no index was generated.

Candidate01 author tree `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, branch `fix/quality-doc-transfer`, initially HEAD `263df47df30879040189f9220f2a3af56607dcc7`; exactly three source changes:

| Source | SHA256 |
| --- | --- |
| `crates/sync/src/doc_transfer.rs` (only three module/re-export lines) | `152e4dbeab2e426b74d35639410e6cff371e426a3aaa1289ac6675b8e9244073` |
| `crates/sync/src/doc_transfer/outbound.rs` | `64480d7a94af53e7f3a45d93270c3f98f947cf4c6825b0b12655308c8dbc444e` |
| `crates/sync/tests/doc_outbound.rs` | `1d7fc933f4b1a71f2f3adc4857f13f2e3ed2fa0f907c2426b8d74424b6e4f493` |

Independently read all new production/test source and descriptor diff, then followed actual `Doc`, `Loaded`, BlockStore flush, TransferStore receive/cancel, Sink append/recover/progress/read_verified and private-path checks. Checked locked Loro 1.16.2 metadata decoder/modes in local primary crate source: `true` reaches checksum parsing; obsolete encodings error and shallow mode remains distinguishable.

External `Q039/doc-outbound-candidate-01/source-prepared.json` records these three hashes and `cargo_run:false`. Three sources match current author files, external source copies and `/Users/zgh/Library/Caches/khor-code-quality/recovery/doc-outbound-candidate-01/source` byte for byte (11229/8850/19830 bytes). Receipt and complete patch also match between disks.

During this read-only review root authorized the author to take the separately reviewed initialization dependency. Current HEAD subsequently became `e5b0aa46bbc34e020a8cab0d82f482030bf90af8` (parent 263df47d, commit subject recover interrupted transfer initialization); the three outbound SHA values remained identical at the end of this review. This does not silently turn candidate01 into a new frozen combined frame. Candidate02 dependency freeze/delta and actual checks remain required; no review source was edited.

## Source conclusions

- `prepare` lines 150–185 reject epoch mismatch and invalid/overbudget canonical descriptor before any flush. Target is the full actual Doc VV, captured before actual BlockStore flush and compared after flush and after actual export. UpToDate is tested only after flush and returns captured target, including when receiver base strictly covers it. Concurrent base/target is permitted; partial Loro metadata VV never substitutes for target. Callers must honor the explicit no concurrent alias mutation contract.
- One Loro export Vec remains live for body validation/staging; the earlier flush delta is released by BlockStore before this export. The adapter hashes/decodes borrowed body, passes canonical slices to core append, and explicitly drops the full Vec before returning. Outbound owns only Descriptor, Sink and original TransferStore clone. Core recovery uses a bounded 256 KiB buffer. This does not bound Loro's initial full export allocation or claim a measured peak-memory result.
- `validate_body` lines 209–249 checks the complete descriptor/identity/length/digest binding, 64 MiB accepted limit, actual byte length/BLAKE3 and Loro checksum/mode; only complete current Updates or Snapshot is accepted. The private module helper is `pub(crate)` and can be reused by a future receiver inside doc_transfer. It is format/byte validation, with no assertion of dependency completeness, pending absence, target coverage, authenticated dispatch or Applied receipt.
- `stage` lines 252–265 uses the same canonical manifest and core `receive`, starts at recovered confirmed offset, writes complete 256 KiB/final slices and requires `VerifiedComplete`. Existing confirmed prefixes are hashed by core recovery, and complete-stage digest is checked; it does not restart from zero or silently overwrite a different manifest. On any staging error the returned `Staging {manifest,cause}` retains the exact resumable/cancellable identity. There is no automatic deletion of an existing stage, quota reservation or other transfer.
- Outbound drop releases its core owner but retains published stage/quota. `discard(self)` lines 133–137 clones its own store and manifest, drops its Sink/owner, then calls that same root's cancel under existing Busy semantics. A race in which another caller acquires ownership therefore fails rather than deleting an active stage. It cannot cancel an unrelated manifest or roll back a Doc.
- `read_chunk` lines 105–128 rejects offsets beyond total or unaligned non-EOF offsets, caps allocation to 256 KiB, fills short reads by advancing absolute offsets, and converts a zero read before the promised length into `UnexpectedEof`. EOF returns an empty Vec. The API relies on the core's exclusively managed private directory/owner contract; its cached verification is not a claim to defeat another actor mutating an already-open body. Truncation is explicitly checked by the new fixture.
- Directory, quota, owner, checkpoint and cancellation behavior is reused from core rather than introducing another spool. Candidate01 still had the previously reviewed initialization failure, so actual validation must use the approved initialization dependency as root has scheduled. No Node/proto/authentication/receiver apply path is changed in these three files.

## Test validity and remaining checks

Counted nine ordinary `#[test]` functions, zero ignored helpers. Read the assertions rather than counting names. They use real PinDoc/Loro, actual BlockStore files, real core stages and public strict apply/reopen:

1. Both encodings must actually exceed one chunk, checksum after reading, survive source Loaded drop/reload, preserve fixed target despite a later flushed local write, and strict apply/reopen to the exact old target. The shared helper asserts every canonical read size and whole digest; bad offsets and explicit discard are observed.
2. Real two-branch concurrent base/target with common ancestor asserts actual partial metadata differs from full target; strict receiver apply must preserve common, sender and receiver operations.
3. Wrong epoch, negative VV and oversized metadata assert no pile flush. A dirty receiver-covering case must flush and reopen with the operation while creating no stage.
4. A real core partial first chunk is resumed under the same manifest; active cancel is Busy, drop frees owner, later receive sees VerifiedComplete, and discard preserves a second recipient's complete stage and bytes.
5. `ObservedDoc` delegates real Loro exports/import reports/version and records whether actual `.loro` files exist at export boundaries. The flush-order test requires `[false,true]` for both modes and no additional export once UpToDate. It does not fake BlockStore or copy prepare logic.
6. The same transparent real Loro fixture changes actual VV during flush/export, corrupts actual bytes, or exports actual wrong/shallow mode; precise errors and zero stages are asserted. This is intentional boundary fault injection, not a fabricated production report.
7. Actual active same-id prepare yields Staging Busy with the exact manifest, preserves its stage, then retries after owner drop and explicitly discards.
8. A real file at the pile path prevents filesystem flush; result must be Flush and stage count stays zero.
9. Real post-prepare body truncation to one byte must produce UnexpectedEof, then explicit cancellation succeeds.

These are new API contract fixtures, not preexisting bug reds. No compiler or runtime result is inferred from them. No source-level compile error was identified by this review. Still unmeasured/unrun here: the nine assertions themselves, actual short-read scheduling, oversized real export allocation, post-receive append I/O failure/retained quota, Windows runtime, and integration after the initialization dependency. Existing core checks cover their own contract and are not automatically counted as this frame's runtime green. None of these unrun branches is a concrete source blocker for the narrowly scoped adapter.

Next: compare candidate02 dependency/frame without changing these three outbound bytes, run authorized normal checks and the actual nine tests, and audit those logs/artifacts against the new freeze. Node chunk adoption, receiver checked apply/receipt and real 25 MB convergence remain open Q039 work.

## Candidate02 dependency delta verified

Independently checked `Q039/doc-outbound-candidate-02/source-prepared.json` and all 1066 `all-inputs.json` entries against current author bytes. HEAD is `e5b0aa46bbc34e020a8cab0d82f482030bf90af8`, parent 263df47d; its commit changes exactly `crates/sync/src/transfer.rs` and new `crates/sync/src/transfer/platform.rs`. Both current files equal their HEAD blobs and original independently reviewed dependency commit `419f73c52bebae6be5e66dfd39d160326a914143` blobs byte for byte: SHA `ba7625143caa261f366b5acc88202d6447d5a4effa80dd48d90dc8aae5fa5cd4` and `05c2b2964e9bda636956d497a45882ececb6d10f5b32dc26b845dcc55af82c37` respectively. All three outbound SHA values above remain unchanged and match candidate02 source copies. Tracked status retains only doc_transfer plus the two intended untracked outbound/test files. No dependency or outbound delta blocker; authorized candidate02 runtime checks may proceed. This review did not run Cargo or signal any process.

## Candidate02 actual green-01 evidence accepted

Independently read the executed runner, raw normal/outbound/descriptor logs, preflight/freshness/results/final, both binary receipts, cleanup and guard-deviation receipts in `Q039/doc-outbound-green-01`. Recomputed all 1066 hashes: candidate02 freeze = run before = run after = current author bytes. HEAD remains e5b0aa46 with only the original three dirty outbound paths. All five source SHA values (including the dependency pair) remain as reviewed above. Also checked the actual workspace-patched `vendor/loro-internal/src/encoding.rs` metadata entry point used by the build; checksum parsing/mode semantics agree with the earlier primary crate source check.

The ordinary locked/offline jobs-1 runner uses the quality shared target, `CARGO_INCREMENTAL=0`, cleared KHOR_AGENT_BUNDLE and its own short scratch. Forty-three first-party Sync/catalog source/manifest mtimes were refreshed without byte changes. Raw normal log actually compiles catalog and checks Sync from this author tree; both test logs actually compile Sync. No old binary or alternate implementation substitutes for the candidate.

Actual normal check takes 3.0236022080061957 seconds. Outbound invocation takes 6.075441666995175 seconds including compilation; Rust reports all nine named ordinary contract tests passed, zero ignored/filtered/failed, runtime 1.84 seconds. The existing codec invocation takes 2.523180083007901 seconds including compilation; Rust reports five named tests passed, zero ignored/failed (157 unrelated filtered). Thus **14 different ordinary tests**, with no helper/child additions and no prior Doc22/core12 or f72128 added to this count. Those actual nine names exactly match the previously source-reviewed oracles, including both multi-chunk encodings, strict apply/reopen after Loaded drop, fixed target after late writes, real concurrent base, real flush fault and actual truncated read. This establishes their assertions, not a trace of unasserted memory usage or future Node behavior.

Current independently hashed executed artifacts match both receipts and their raw Cargo Running paths:

| Artifact | SHA256 | Bytes |
| --- | --- | --- |
| `doc_outbound-761610bf76283ee7` | `f8d147e106b7f15f7773a8bbafa2ddc24fdb04943ef9a79ec8080cc70bf892cd` | 20,383,136 |
| `khor_sync-6bc0f9688422d392` | `22bd496850a4b1bee856f1f54461380f8d4849363c1e9bb11b4f22a0b924a669` | 24,681,832 |

### Runner threshold deviation

The preserved executed `run.py` plainly has `step_start > 420` **and** total `start > 420`, whereas the requested single-stage guard was 240 seconds. `runner-deviation.json` records the overly broad string replacement, rather than rewriting the executed script. The separate claimed 240-second guard attached only after completion: its receipt sees no PGIDs and terminates nothing. It did not enforce the requested bound and this audit does not credit it as a rescue or effective guard.

This is an execution-control deviation, not a passing test assertion or a production business red. All three actual stages ended normally within 6.076 seconds, the total was 11.78307095899072 seconds, no threshold/resource guard fired, and no source/assertion/output filtering was used to obtain success. Given independently matching sources/binaries and complete raw results, the fourteen behavior results remain credible. Repeating the same fourteen solely to change a never-reached deadline would add no behavior evidence; no automatic rerun is required by this review. Future runners must use separate explicit step/total constants rather than broad substring edits.

Minimum observed free bytes: external 6,732,103,680; internal 12,893,388,800, both above 5 GiB. Independently confirmed all three PGIDs 97850/97936/98083 ESRCH, and the own scratch `/Volumes/disk0/Khor/caches/tmp/doc-outbound-01` is absent. Cleanup records no scratch files, only the empty outbound container before rmdir; this reviewer did not perform cleanup or claim an independent pre-deletion listing. The entire external/internal green evidence relative file set and all **17** SHA values match at this check.

Final recommendation: no source or evidence blocker for root to authorize the three exact outbound files' commit on this dependency. No need to enlarge the verified set merely for the documented runner deviation. Post-commit exact blob check and root's eventual integration checks still belong to delivery. Full-export peak memory, forced nonzero short reads, a post-receive append I/O fault, Windows runtime, receiver/Node adoption, Applied receipt and real 25 MB convergence remain outside this completed slice.

## Committed source delivery verified

Author commit `9c353fb8c9a27549164835170f13e98c69c86272` has parent e5b0aa46 and changes exactly the three reviewed outbound paths (832 insertions; doc_transfer only the three wiring lines). Independently hashed each commit blob: all three equal candidate02/green-01 SHA values above. Root integration `5da91f0332634d9415ba8641aaa5a1e166258000` has parent f721, contains exactly the same three blob byte streams, and matches the author commit/freeze. The author working tree now contains subsequent receiver work; this audit did not attribute those newer dirty bytes to the outbound commit or review them as part of this delivery. Root's separate exact integration run is audited in its own Q049 scope; author green-01 is not automatically that integrated runtime result.
