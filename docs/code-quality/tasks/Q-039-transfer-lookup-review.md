# Q039 — independent retained lookup and outbound resume review

Owner `doc_apply_review`, independent of author. Candidate02 source and green-01 evidence reviews completed without a blocking finding. Normal Sync check and 49 different relevant ordinary tests are confirmed for the frozen frame. No Cargo, generator, fixture execution, source edits or process actions by reviewer. Root may authorize commit of the seven exact files, followed by exact delivery/integration checks. This library slice does not complete Node/25 MB/Applied/fairness/Pair or authorize main/ship.

## Exact frame and method

Read the authoritative lookup card and actual seven-file diff/source in native-io, `fix/quality-doc-transfer` at `3e46b6e4a92c146952a3794d85b8dddbf12d4653`. Graph-first minimal/detect/affected-flows/tests_for/impact calls preceded source fallback. Graph is built on 9c353fb8 rather than current HEAD, lacks open_existing/new tests and reports zero flows; its metrics are not safety evidence. No graph index was generated.

Candidate02 `Q039/transfer-lookup-candidate-02/source-prepared.json` is the actual freeze filename. Independently verified all **1072** `all-inputs.json` hashes against current files. Each of the seven frozen source copies also equals current source and internal `recovery/transfer-lookup-candidate-02/source` bytes:

| File | SHA256 |
| --- | --- |
| `crates/sync/src/transfer.rs` | `50e71755aded30c9f0992218b4af0fe69a91a40e64aa598d56c2f9d60407dd4d` |
| `crates/sync/src/doc_transfer.rs` | `d9fde4f64ccd61248384b45d4460f1f86c28d51372a309699a17bb91b3be211f` |
| `crates/sync/src/doc_transfer/body.rs` | `663f359c55e1f3b7ca4f24c86059e23d8f61d01010ac6b3995aa438d80dc182b` |
| `crates/sync/src/doc_transfer/outbound.rs` | `f3e0e89b6f0c76517b85fdcb292989831b6aa1fcb23522eb5ffbfae09e980cc0` |
| `crates/sync/src/doc_transfer/receive.rs` | `f3db83ca30a37d947704429aac58c11732f6f4f39865df4d4f853d2176006dda` |
| new `crates/sync/tests/transfer_lookup.rs` | `65f039b5200d22267a8200f90eafaa5e600e26cd3a94d6e97de8a701097435e3` |
| new `crates/sync/tests/doc_resume.rs` | `6074aede3e7de6d4bacfa0e0f9bde217ef46d647e2413a4a731832472818ff43` |

Status has exactly five production files modified and these two new tests. Candidate01→02 manifest comparison changes only transfer.rs; actual source delta is the additional `manifest.validate()?` before borrowed manifest clone. This prevents a second allocation of an overbudget binding before admission and is a static resource correction, not a fabricated runtime bug red. The original core integration, receiver and outbound test files are independently byte-identical to HEAD.

## Core opening and ownership

`receive(Manifest)` now calls common `open_stage(manifest,true)` and converts only an impossible None to InvalidStage. The old id→gate→reservations→quota/owner→initialize-if-absent→manifest comparison→file open/recover→owner bind sequence is preserved. Existing stages still pass full manifest equality and checkpoint/prefix recovery. No new private filesystem traversal or public-stage deletion path is introduced.

`open_existing(&Manifest)` validates budget before cloning, then uses that same helper with false. It returns None immediately after the gated reservation scan confirms the id absent, before quota expansion, available_owner or any stage creation. Consequently an absent id needs no owner and remains None while both actual owners are busy. It does not promise filesystem purity: the common gate may create its control file or reconcile a legitimate abandoned `.initializing` using the already reviewed policy. Other malformed retained state still fails closed rather than reporting an unsafe absence.

Existing paths acquire exactly the original LOCAL/OS gate and owner lease, and preserve manifest/id/128 MiB/64-stage/two-owner checks. Recovery or binding failure drops ordinary File values while the gate is held; Owner is constructed only after local insertion, with no fallible work afterward. The Option wrapper adds no recursive Owner drop under the gate. Sink drop on later resume error releases only owner, keeping stage/quota; it does not automatically cancel or clean published bytes.

`retained_manifests` is precisely gate plus existing reservations, then `BTreeMap::into_values().collect()`. It moves existing manifest values into a Vec rather than cloning a second full metadata map; the 64-stage and 64 KiB-per-manifest bounds remain the same. It includes active, idle and zero-body retained entries, treats opaque binding as opaque, and neither parses it as Doc nor guesses a cancellation/GC policy. Node must use this interface rather than independently scanning private core files; authorization/recipient policy belongs to its future dispatch.

## Resume and shared reader

Resume parses canonical Descriptor before touching the store, so malformed/overbudget descriptor errors cannot create a gate or stage. It calls open_existing and returns None for an absent id. Otherwise common full-reader first requires VerifiedComplete, enforcing cached core total digest/progress, then caps total at 64 MiB before allocating. Core open recovery already rereads/checks its confirmed prefix; malformed journals/body cannot become a nominal resumed Outbound.

The shared reader limits every read slice to at most 256 KiB, advances by actual returned count, and errors UnexpectedEof if zero arrives before promised total. Short positive reads cannot skip bytes or return a short successful full body. The full Vec is intentionally local; this is not a bounded-Loro/RSS claim. Resume then runs the unchanged body validator for descriptor/manifest canonical binding, actual length, whole digest, checksum and exact current complete encoding. It drops the Vec before returning fixed Descriptor+Sink+same-store clone. No Loaded, Doc export, new target/version or mutable plan is consulted.

Core open/progress/read/incomplete errors map to `Staging {manifest,cause}`; format/checksum/mode errors map to `StagingBody {manifest,cause:BodyError}`. Both preserve exact typed cause and input identity; error source wiring covers the new variant. Failed resume drops the local Sink/Vec and retains published stage/quota for explicit policy. Descriptor failure before open remains Transfer, with caller already owning that manifest. Existing prepare's pre-stage Body errors are unchanged. None of these validations proves pending absence, target coverage, receiver authorization or Applied.

The receiver extraction preserves ordering and typed surface: its original document/source/recipient checks and `Progress::Stored -> ReceiveError::Incomplete {next_offset}` happen before calling the shared reader. Existing reader I/O remains Transfer/UnexpectedEof, and body/apply errors retain their old conversions. The extra internal progress check cannot turn an exclusively borrowed stored Sink into a successful body. Diff does not change apply_checked or stage-release behavior.

## New test oracles and pending runtime

Counted four lookup plus five resume ordinary tests, zero helper-count inflation. Read all assertions; they use real TransferStore journals/files/locks and real PinDoc/Loro, with no fake Doc or copy of the production opening/resume algorithm:

- Missing lookup with a declared 64 MiB manifest creates no id, reserves zero and creates neither owner file; both actual owner slots can subsequently be occupied, and Missing still succeeds as None.
- Real partial checkpoint reopens at the confirmed canonical offset; both lookup and receive reject a live same-id owner; actual completion rereads final bytes after owner drop.
- Retained enumeration contains active opaque, idle empty-body and canonical Doc manifests with exact metadata/quota, and leaves all stages present. Different recipient/invalid size lookup cannot alter another id's manifest/body/journal bytes.
- Both real multi-chunk encodings prepare, release Loaded/Outbound/store, reload source and flush a later operation, then resume from only the old manifest. Descriptor/target and whole body bytes must equal the original despite newer source VV. Explicit discard protects another complete opaque stage and reservation.
- Noncanonical descriptor rejects before gate creation; valid Missing returns None without retained bytes. Partial resume returns exact typed manifest error, preserves confirmed body, and an actual core completion then resumes successfully.
- Actual snapshot bytes declared as Updates and separately checksum-corrupted Loro bytes with matching outer byte manifest produce the distinct StagingBody causes while keeping staged bytes. An active owner yields Staging Busy; real on-disk confirmed body corruption yields typed core InvalidStage without deletion.

New API tests need no fabricated existing-bug red. No source-level compile error was found, but compilation and all assertions remain unrun. Required planned distinct set is nine new + eight original receiver + nine original outbound + eleven exact core units + twelve core integrations = **49**, plus ordinary Sync check. Avoid the substring transfer::tests:: accidentally counting codec tests as core; leave unrelated codec/Doc matrices unexpanded when unchanged.

Unrun boundaries remain explicit: forced short-positive-read scheduling, non-empty gate cleanup side effects through these new public methods, injected read I/O during a concurrently corrupted private body, platform runtime beyond the eventual test host, process-restart Node status/resume adoption, fair pump and persisted Applied receipt. Existing core contracts plus this static review do not turn those into current candidate runtime green. No concrete blocking counterexample was identified in the narrow factoring/resume implementation.

## Final green-01 evidence independently verified

Read `Q039/transfer-lookup-green-01` executed runner, preflight/launch-preflight, freshness, all 17 raw phase logs, results/final, all six per-binary records and saved Cargo fingerprints, binary-after, cleanup and internal mirror. Candidate remains uncommitted on 3e46b6e4 with the same seven paths. Rehashed every input: all **1072** candidate02 freeze = before = after = current author files. All seven source SHA values above remain unchanged. Cargo.lock and original core integration/receiver/outbound test files also independently equal HEAD bytes; no oracle or dependency change accompanied the pass.

Forty-nine Sync/catalog source/manifest mtimes were refreshed without byte changes. Normal library log actually compiles catalog/checks Sync from current native-io; directed and initial core unit logs actually compile Sync test artifacts from this tree. The runner uses standard locked/offline jobs-1 commands, quality target and `CARGO_INCREMENTAL=0`; its actual six Cargo fingerprints and preserved copies independently match hashes, `features:"[]"` and `rustflags:[]`. The retained vendor HasIdSpan warning is visible, with no hidden command failure or feature bypass.

Independently parsed successful ordinary test names from every raw log and matched them to the result manifest. There are exactly **49 names, all unique**:

| Actual group | Ordinary passes | Runtime evidence |
| --- | --- | --- |
| lookup | 4 | real Missing/owner/quota, partial/full recovery, opaque list and other-id preservation contracts |
| resume | 5 | fixed target/bytes after new source ops, typed incomplete/busy/core/body failures with retained stage |
| existing receiver | 8 | includes unchanged typed Incomplete, truncation, checksum/mode and strict apply contract |
| existing outbound | 9 | unchanged export/flush/target/drop/partial/IO contracts |
| exact core units | 11 | first compiled through Cargo exact full name; remaining ten invoke that exact unit binary with --exact |
| existing core integration | 12 | one ignored worker is actually spawned by its parent and not counted separately |

The core units do not use ambiguous substring filtering, so no unrelated codec pass is counted. Actual normal check plus six test groups amount to **17 phases** (eleven core cases are separate phases), all exit 0 with source stable/PGID gone and no stop reason. No prior historical author/integration results are added to 49. The new nine assertions establish the previously reviewed real filesystem/PinDoc oracles, not memory measurements or future Node adoption.

The eleven real initializer child kills still execute: original child 52815 logs SIGKILL, `.initializing`, public reserved Ok19/fresh VerifiedComplete44; eight phase children 52822/52828/52829/52830/52831/52832/52834/52835 cover before_mkdir through after_publish, and second-cleanup children 52837/52838 cover offsets_synced/cleanup_after_unlink. All are independently ESRCH now. Parent public recovery and owner-release assertions pass; no test-side manual stage removal substitutes for recovery. These children are not extra ordinary passes.

### Exact artifacts and execution controls

Six actual executed binary paths from raw Cargo Running output/per-case exact unit commands have identical before/after receipts, and independently current SHA, size **and mtime** agree with those receipts:

| Binary | SHA256 | Bytes |
| --- | --- | --- |
| `transfer_lookup-bde48dae5e6a93f2` | `74c6a3ff273355366f56ceec53812be10aefb1804630ad8692809fd758c15f6f` | 1,892,752 |
| `doc_resume-f5ff697ebfe14ef1` | `42d30c52f8348b002966d134c538ba2448f53435c4b9c96056e8817909d440e1` | 20,227,192 |
| `doc_receive-1747035ff8a0277f` | `8df6a4cd13dcc4778c8ec8b23e547e4ed0ffd65a0042af0d03cf32c66a10be04` | 20,319,184 |
| `doc_outbound-761610bf76283ee7` | `95f677e022ab02fd88d9b56f23f3869834a09a6c88e253afcf50418fe87d003d` | 20,386,016 |
| `khor_sync-6bc0f9688422d392` | `2831e4570d23b0be3cd92dee75af27b77b4e7087864f14ffc9c037381e8a47b4` | 24,682,584 |
| `transfer-ebd3cc272a00feb9` | `0dbddbe55f77b490dee7fb4226f5d0c0049a43a7f038a62d80b5fd05bbab6028` | 1,875,128 |

Reviewer parsed the retained script AST: separate STEP_TIMEOUT_SECONDS=240, TOTAL_TIMEOUT_SECONDS=600 and MIN_FREE_BYTES=5368709120 agree with launch-preflight and the script's own asserted map. Script SHA independently matches its launch receipt. Actual loop refers separately to those constants; there is no broad replacement deviation or post-completion guard credit. Total run is **28.725414000000455 seconds**, no deadline/resource guard fired. Minimum free bytes are external **6,567,673,856**, internal **12,846,739,456**, above 5 GiB.

All 17 recorded PGIDs independently return ESRCH (52618/52654/52694/52721/52749/52767/52821/52836/52839/52878/52882/52883/52885/52896/52898/52899/52900). Own scratch `/Volumes/disk0/Khor/caches/tmp/transfer-lookup-01` is absent. Cleanup records zero files and only four empty resume/transfer/receive/outbound containers before removing them/root. This is a receipt plus current absence, not an independent pre-removal listing; reviewer did not delete anything. External/internal `recovery/transfer-lookup-green-01` relative sets and all **53** file SHA values match.

Conclusion: no source or evidence blocker to committing the seven exact candidate02 files. Preserve the reviewed API/error behavior and unchanged test bytes in the commit. Post-commit blob equality and eventual root integration checks still apply; all earlier stated unrun platform/Node/receipt/large-convergence boundaries remain open. No automatic expansion or repeated test run is needed for this completed narrow frame.
