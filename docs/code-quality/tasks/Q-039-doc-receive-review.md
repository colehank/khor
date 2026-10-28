# Q039 — independent receive and version metadata review

Reviewer `receive_review`, not the implementation author. **No source, green01 evidence or scoped integration blocker identified for candidate01.** Independently accepted normal Sync check and **22 distinct ordinary tests** against the exact frozen frame, then verified author commit **3e46b6e4** → root **67927fcf** exact blobs and root's same 22-case integration below. This is not main/ship authorization or whole-Q039 completion. This reviewer ran no Cargo, generator, native build, test fixture or process signal and changed no production/test source, STATE or other task card.

## Exact reviewed frame

Read central STATE/PLAN, receive contract and previous outbound review; checked the actual tree `/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`, branch `fix/quality-doc-transfer`, HEAD **9c353fb8c9a27549164835170f13e98c69c86272**. Exactly the seven authorized source paths are dirty. No `.codegraph` exists; no CodeGraph command/index was used.

Used the project `review-changes` skill and graph detect/minimal context/affected flows/tests/impact queries before source fallback. The first detect call preceded reading that skill's extra minimal-entry instruction; the subsequent minimal query does not retroactively make that order compliant. More materially, the graph is built on **e5b0aa46**, not current HEAD, and `tests_for apply_staged` returns not_found. Its zero flows/impact and nominal risk scores supply no coverage evidence. Actual new source and relevant existing boundaries were then read directly.

Independently recomputed all **1070** `all-inputs.json` entries in external `Q039/doc-receive-candidate-01`: zero mismatches with the current tree. All seven current sources, external frozen source copies and internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/doc-receive-candidate-01/source` copies are byte-identical. Both evidence directories have the same eleven relative files and hashes, including source-prepared, full patch and input receipt.

| Source under `crates/sync/` | SHA256 |
| --- | --- |
| `src/doc_transfer.rs` | `83e554888c1de5f8645532b9baed80ef8d1959e3cee693c942092f0d4e0da3f2` |
| `src/doc_transfer/body.rs` | `4d42641c5472f45f36f7a269f31b78c50a9dc185d307a180b86f24972c3c6acd` |
| `src/doc_transfer/outbound.rs` | `adac0f849e76645a93fba5b7055296530a99107e01d842f043128c7638372f5e` |
| `src/doc_transfer/receive.rs` | `b5a36e3da5d35a23110addf5245b382b2049617066950957820d7b8c1d816a5a` |
| `src/doc_transfer/version.rs` | `dfe11a5fd0838b91b5dc2d7b11e1c01957933042809ac87bc1a3baded1b3e7cc` |
| `tests/doc_outbound.rs` | `29c5a7cbea2cb58e54cb736314f23ac842ef048dc04f16584f76aa103be1098c` |
| `tests/doc_receive.rs` | `a112a9a4ae8a5d401ce6470522be493aa135f61943931d46703d761e7f96dd2f` |

`git diff --check` passes. Final source review status/branch/HEAD remains the exact frame above; no source edit occurred during review. Root's earlier integrated normalSync+14 result is only the old outbound baseline.

## Findings by risk

High/medium: none identified in this narrowly scoped library adapter or its green01/scoped integration evidence. The runtime evidence accepted below supports this candidate. Root runner control-record differences are explicitly retained below.

Low / verification limits: no concrete defect identified. The receive fixture does not force successful completion through a nonzero short-read schedule, 64 MiB maximum allocation, real append I/O fault, or receive-level CommitFailed/fence fault. Existing strict apply tests and the direct typed delegation are relevant but are not counted as these new cases passing. The body rejection group checks final empty Doc/no pile and the final stage explicitly; it does not independently assert every rejected stage exists before its explicit test cleanup. The production path contains no cleanup operation, and this is not a source blocker. No speculative tests were added by the reviewer.

## Production conclusions

- `DocumentVersion` is correctly named/documented as metadata, with no persistence proof/flush behavior. Its 16-byte domain, BE epoch and canonical VV share the actual parent encoder/reader. `write_version` rejects negatives, omits zero counters, sorts writers and includes the 20-byte prefix plus count in the 64 KiB budget. Reader bounds the count against remaining bytes, rejects duplicate/decreasing writers, zero/overflow counters and truncation; decode rejects unknown domain/trailing bytes and checks the full input budget before parsing. The tests' 24-byte prefix/count and maximal entry calculation agree with the implementation.
- `apply_staged` lines 79–94 validates canonical descriptor and exact authorized document/source/recipient before progress or body access. These are explicitly Node dispatch inputs; this library does not authenticate a peer or infer the correct Loaded document itself. A different Node dispatch binding would be a caller bug and remains part of the future Node integration review.
- Progress has exactly Stored/VerifiedComplete; Stored returns typed Incomplete, poisoned/digest errors stay typed Transfer. The manifest is immutable through the public Sink API. Before allocating, the receiver enforces <=64 MiB; conversion to usize is safe for the supported 32/64-bit targets at this limit. Every read slice is <=256 KiB. The loop advances by actual count and handles nonzero short reads; zero before expected end becomes typed UnexpectedEof. No full file copy/helper apply implementation was introduced.
- The single shared `validate_body` rebinds the descriptor and manifest, checks accepted size/actual length/BLAKE3 and calls the actual Loro metadata decoder with `check_checksum=true`. Inspected workspace-patched `vendor/loro-internal/src/encoding.rs` 303–368/553–564 and fast_snapshot metadata 541–549: true reaches the embedded checksum check, outdated modes fail, and a shallow snapshot remains distinct. Only complete current Updates/Snapshot matching the descriptor is admitted. Metadata VVs are never substituted for the descriptor's complete target.
- Only after byte/format checks does the receiver call the real `Loaded::apply_checked` with descriptor epoch/base/target. Read the actual existing boundary: it checks epoch and live/on-disk base, validates a separate candidate for pending/coverage, preserves the original live Doc, obtains/checks its fence, imports and flushes, and privately constructs FlushedImport only after coverage checks. Receive preserves every ApplyError variant through `map_err(ReceiveError::Apply)`; it does not recreate/replace the live Doc, swallow CommitFailed, invent a success or promise rollback.
- Every receiver return retains the caller's Sink/staged bytes. No cancel/delete or network receipt exists in this adapter. Successful FlushedImport means the existing local checked import/block flush boundary only. The full body Vec remains during strict apply and is dropped before return; Loro/candidate/snapshot allocations remain additional. The code/docs correctly avoid O(chunk), total RSS, power-loss durability or network Applied claims.
- Body helper extraction retains all old validation conditions in the same order. The intentional unreleased public error change is neutral BodyError under OutboundError::Body; no compatibility wrapper remains. `source()` now exposes actual Transfer/Body/Staging chains; InvalidLoro remains the prior string-based cause rather than a new fabricated typed Loro error. Outbound's original nine case functions/fixtures and assertions are unchanged except import plus the two corresponding precise error patterns. No old case was deleted, weakened to generic is_err, ignored or replaced.

## Test oracle audit

Counted eight ordinary `#[test]` functions and no ignored/child helpers. Helpers only prepare canonical manifests, append real chunks, create scratch paths and deterministic large data. They do not reimplement apply or return a synthetic FlushedImport.

1. Fixed literal bytes assert domain, sorted writers, BE epoch, zero omission and decode equality; this is an external byte oracle, not encode/decode round-trip alone.
2. Every truncated prefix, unknown header, trailing byte, count overflow, duplicate/order/zero/counter overflow, negative encoder input and largest fitting/next-too-large metadata are rejected. Full overbudget decode is a precise ResourceLimit.
3. Both actual prepare encodings exceed a real chunk and use actual outbound reads/core append. Original source Loaded is dropped before receive; a fresh receiver has a dirty local concurrent operation. The first actual 256 KiB stage must return precise Incomplete without changing its VV/pin or creating the pile. Complete receive must return a real block, retain target/local operations, reopen to the exact applied version and retain stage/Busy ownership; after Sink drop, re-opened stage is still VerifiedComplete. This validates persistence and preservation rather than merely observing an in-memory target.
4. Wrong document/source/recipient each beat Incomplete on an actual incomplete stage. Correct binding is still Incomplete, with unchanged Doc/no pile; completing that same stage then succeeds.
5. An actual complete opaque stage with a noncanonical descriptor fails precise InvalidManifest before Doc/disk mutation and retains its directory.
6. Mutating the real already-verified body tests rehash rather than cached core verification; a changed body with matching outer digest still fails Loro checksum. A real snapshot declared Updates and a real shallow snapshot fail the typed mode oracle. Receiver stays empty/no pile; last stage remains.
7. Actual bodies exercise exact WrongEpoch{expected:2,loaded:1}, StaleBase and PendingImport, keeping each stage. The pending body is a real delta missing its dependency; receiver stays without those operations/no pile. This does not duplicate the whole strict apply matrix or claim all commit failure paths are newly run.
8. Truncating the real verified file to one byte requires typed io::UnexpectedEof, unchanged pin/no pile and retained stage. It exercises a short first read followed by EOF, not a fake injected read result.

These are new API contract tests; no fabricated preexisting red is required. At the source-review checkpoint no execution was credited. The requested normal Sync, eight receive, original nine outbound and existing five descriptor cases have now been executed and independently audited below. Commit exact-blob checks and integration evidence remain subsequent steps.

Node adoption/authenticated dispatch, real 25 MB convergence, fair pumping, persisted/network receipt, Pair bootstrap, forced short-read scheduling, peak memory and Windows runtime remain open or outside this slice. No main merge/ship authorization is inferred.

## Candidate01 green01 evidence accepted

Independently read the actual `Q039/doc-receive-green-01/run.py`, launch/preflight/freshness/results/final receipts, all four raw logs, three binary receipts plus binary-after and cleanup. Recomputed all **1070** hashes: candidate freeze = run before = run after = current author bytes. HEAD remains **9c353fb8** with exactly the original seven dirty sources, including both test files; the seven SHA values above have no delta. There is no source-level temporary test cfg, alternate apply/helper or assertion change between the reviewed source and execution.

The normal default-library check uses locked/offline jobs1 and actually compiles current native-io catalog/checks current Sync. All three test logs actually compile current Sync. The runner uses quality-target, incremental0, clears KHOR_AGENT_BUNDLE and creates its own short TMPDIR/scratch. Forty-seven catalog/Sync Rust/manifest entries are recorded as mtime-refreshed, with source bytes verified unchanged. Actual corresponding Cargo fingerprint JSON for each of the three current executed binaries has `features: []` and `rustflags: []`; the run commands supply no alternative features/backend/cfg. The preexisting vendor HasIdSpan warning is present but is not a candidate failure.

The raw test names exactly match the eight source-reviewed receive, original nine outbound and existing five codec cases. Receive/outbound logs each show zero filtered/ignored/failed; codec shows five passing and 157 unrelated filtered. Thus **22 different ordinary tests**, with no child/helper or old strict-apply/core test count added. The real source-drop/fresh-receiver multi-chunk test executes both encodings, and its actual assertions establish partial-stage rejection, local-concurrent preservation, real flushed block/reopen and stage retention. Typed negative cases, unchanged live/no-pile observations and the limited per-stage retention assertions establish exactly the source oracles described above. No trace of unasserted behavior is inferred from a passing name.

| Stage | Actual runner seconds | Actual Rust test result |
| --- | --- | --- |
| normal Sync lib check | 3.548776375 | exit0 |
| receive | 6.102632958 | 8 passed, 0 failed/ignored; 1.74s test runtime |
| outbound | 2.526638916 | 9 passed, 0 failed/ignored; 1.78s test runtime |
| descriptor | 2.543524584 | 5 passed, 0 failed/ignored; 0.00s test runtime |

Full runner total is **14.977662708s**, complete=true; no stage stop/guard trigger. Independently parsed the actual runner AST: independent literal STEP_TIMEOUT_SECONDS=240, TOTAL_TIMEOUT_SECONDS=420, MIN_FREE_BYTES=5368709120 are used in the corresponding step/total/resource checks. Its self-check occurs before subprocess creation. The separate launch receipt records the same external check and the current runner's matching SHA **23a0d8fc61f0baa94090b71d6a15e0a6dd58d516a4a145966d86d225e01ab7b3**. The previous outbound runner's global-substitution deviation is absent from this actual script.

Independently hashed the current actual Cargo Running paths and checked size/mtime: all equal the per-step receipts and binary-after exactly.

| Artifact | SHA256 | Bytes |
| --- | --- | --- |
| `doc_receive-1747035ff8a0277f` | `8365a820eb6c88c609197b040fc8973677413f748106070059b18529df034977` | 20,317,888 |
| `doc_outbound-761610bf76283ee7` | `cb43dfcd139dbb657344d4e108145c3b565eaed44be5b24830b74553c721d1d1` | 20,385,088 |
| `khor_sync-6bc0f9688422d392` | `d65c0277dcce2a82134146a1563ed52288d6a79aca9c7b7a1fa4f28dd5ff64cb` | 24,681,832 |

Minimum measured free bytes external **6,594,011,136**, internal **12,868,448,256**, both above 5 GiB. Independently checked PGIDs 28921/28947/29001/29020 all ESRCH and own scratch absent. Cleanup receipt records zero scratch files and removal of only empty receive/outbound subdirectories plus root; this reviewer did not claim a separate pre-deletion observation or perform cleanup. All **19** external/internal green evidence relative files and SHA values match at this check.

Final recommendation: **root may authorize committing the seven exact reviewed source files.** No further run is required solely for this evidence audit. Private FlushedImport still originates only in actual Loaded::apply_checked and still means local checked import/flush. The source review's per-stage assertion caveat, successful short-read schedule, receive-specific CommitFailed/fence failures, non-Darwin runtime and Loro peak allocation remain unmeasured/unrun here. Future Node/auth dispatch, network receipt and 25 MB/fairness/Pair work must not be represented as covered by these 22 tests.

## Exact commit delivery and root 67927fcf integration accepted

Independently read git objects for author **3e46b6e4a92c146952a3794d85b8dddbf12d4653** and root cherry-pick **67927fcfacb84fe260553e31614d8c1e2fd1cd5c**. Each commit changes exactly the seven authorized paths. For every path, author blob SHA = root blob SHA = candidate01 freeze SHA in the table above = current root working file SHA. No conflict resolution or source delta was introduced. The author tree now has seven dirty paths belonging to the later lookup/resume candidate; those bytes are deliberately excluded from this receive delivery conclusion. Root wave1 is at 67927fcf and tracked clean at this audit.

Read actual `integrated-6792-final-01` runner, preflight/freshness, four raw logs, results/final, unit binary and integration receipts. Recomputed all **1074** root inputs: run before = run after = current root. This is the root combined frame, not a claim that the entire author and root worktrees are identical. All **14** relative integration evidence files and hashes match the internal recovery copy. Forty-seven Sync/catalog source/manifest mtimes were refreshed without changing bytes. Raw normal log compiles current wave1 catalog and checks current wave1 Sync; every test log compiles current wave1 Sync. The commands are ordinary locked/offline jobs1/default-library/test commands with incremental0 and the quality shared target, not alternate source/feature invocation.

Actual normal check exits0 in **3.039026041s**; receive **8/8** in **6.083888875s**, outbound **9/9** in **2.527015291s**, descriptor **5/5** in **3.044068208s** including compilation. Total **14.889825416s**, all exit0/source_stable, complete=true. Raw exact case names agree with the already-reviewed oracles; no ignored/failed receive/outbound/codec cases, codec retains 157 unrelated filtered. This is the **same 22 distinct ordinary cases** exercised on the integration frame, not 22 additional behaviors or a combined 44-case claim. No broader strict apply/core/Node/full-Q039 green is inferred.

Current actual unit binary matches root's receipt and its raw Running path: `khor_sync-6bc0f9688422d392`, SHA **1111ac763ba9796e157e3417bb8040c2ead064961530506ec33b316b82a7ba30**, **24,681,864B**. The root runner archives only this unit hash; it does not preserve independent per-step integration-test SHA/size/mtime receipts like author green01. Thus this audit credits the actual current-tree compile/Running/raw results and source stability, without inventing equivalent root binary receipts or reusing author's prior hashes after integration rebuild.

The actual root script uses step **180s** and total **300s** literal guards plus the 5 GiB/three-consecutive-I/O guards. Unlike author green01 it does not use separately named step/total constants or retain an external AST limit-check receipt; it must not be described as satisfying that previously requested runner-control workflow. This is a control-record difference, not an observed business failure or a new source delta. The script preserves the actual distinct limits, every stage ended normally within 6.084s, total was 14.890s, no guard fired, and no log/assertion filtering produced success. Matching blobs, 1074 stable inputs and complete raw results make the scoped 22 behavior results credible; no repeat solely to change never-reached guard formatting is required by this audit.

Minimum measured free bytes external **6,593,761,280**, internal **12,855,664,640**, both above 5 GiB. Independently checked four root PGIDs **36871/36895/36954/36982** all ESRCH and short own scratch `/Volumes/disk0/Khor/caches/tmp/i6792` absent. The integration receipt records zero scratch files and removal of empty containers; reviewer did not perform or independently observe the prior deletion. No source/Cargo/process action occurred during this read-only delivery audit.

Receive/version library slice delivery and this limited root integration are accepted. Later lookup/resume, Node authenticated routing, network Applied/receipt, 25 MB convergence/fairness/Pair, non-Darwin runtime and peak Loro memory remain separate. Main/ship and entire-governance completion remain unauthorized/unestablished here.
