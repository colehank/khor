# Q039 — retained transfer lookup and fixed outbound resume

Owner `doc_outbound_owner`. Candidate02 source and final runtime evidence independently reviewed without blocker; normalSync and49 distinct ordinary tests passed. Author commit8ffe6aa2 saved; root integration validation remains separate. New library interfaces support a later Node pinned plan/status/resume; they do not complete Node/25MB/fairness/Applied/Pair. No main merge/ship.

## Baseline and ownership

Reuses native-io `fix/quality-doc-transfer` at **3e46b6e4a92c146952a3794d85b8dddbf12d4653**, initially clean. Root integrated receive as67927fcf; its current combined check/test evidence belongs to root. Graph minimal/file-summary used before actual source. No index creation or platform changes.

Seven files only: transfer.rs minimal common open helper and two methods; doc_transfer.rs resume re-export; outbound.rs resume/error variant; body.rs shared verified reader; receive.rs minimal reader extraction; new tests/transfer_lookup.rs and tests/doc_resume.rs. Underlying platform/init policy/quotas/store/apply/Node/proto/lock/dependency/UI unchanged. Original core integration12, receive8 and outbound9 test bytes unchanged; all previous1070 outside-allowlist inputs verified unchanged.

## Public and shared contracts

- `TransferStore::open_existing(&Manifest)->transfer::Result<Option<Sink>>` and `receive(Manifest)` share private `open_stage(manifest,create_missing)`. Manifest validation, gate+existing initialization reconciliation, reservations, owner acquisition and checkpoint recovery stay on the original path. Missing returns None immediately after the gated existing reservation scan, before any owner acquisition; no published stage/quota/owner created for Missing. Gate control files or legal abandoned `.initializing` reconciliation can still have side effects, documented explicitly.
- `TransferStore::retained_manifests()->Result<Vec<Manifest>>` uses exactly existing gate/reservations, including active/idle/empty and opaque purposes. Moves map values into one Vec; no Doc parse, second filesystem implementation, publication deletion or GC policy.
- `doc_transfer::resume(&TransferStore,&Manifest)->Result<Option<Outbound>,OutboundError>` parses canonical descriptor, calls open_existing, returns None if absent, otherwise requires VerifiedComplete. Shared full verified reader then common body helper recheck body <=64MiB, canonical binding, length/digest/checksummed current complete mode. A local full Vec is dropped before returning fixed Descriptor+Sink+same-store clone; no Doc/Loaded/export/new target.
- Existing core/read/incomplete errors after attempted opening are `Staging {manifest,cause:core Error}`. Root approved new `StagingBody {manifest,cause:BodyError}` for retained body validation errors, preserving exact manifest/typed error/staged bytes. Descriptor parse failure before open can remain Transfer error, with caller still holding the input manifest. No invalid/incomplete stage is deleted automatically. Fresh prepare's pre-stage Body error remains unchanged.
- Private body.rs `read_verified_body(&mut Sink)` is extracted from receiver's existing full-Vec reader, retaining <=256KiB read/short-loop/UnexpectedEof behavior. Receiver retains its earlier exact Incomplete check before this helper, same source/document/recipient gate and apply semantics. No old test or typed oracle is changed.

## Prepared verification and required next checks

Four new real core contract cases: Missing creates/reserves no stage and occupies no owner (including when both real owners busy); existing partial and complete reopen/checkpoint path plus active Busy; retained mixed opaque/Doc/empty purposes without deletion; different/invalid manifest preserves all files of another id.

Five new real Doc resume cases: both prepared multi-chunk encodings survive sourceLoaded/spool/store drops, source reload+late flushed local op, then same fixed target/bytes resume; same-root discard protects another opaque published id; Missing/noncanonical no stage; partial typed manifest error retains confirmed bytes then actual completion resumes; wrong mode/real Loro checksum errors carry StagingBody+manifest and retain bytes; busy/core corruption remain typed and retained. These are new API contract tests, not fabricated bug reds.

Rustfmt seven files and diff-check passed. No compile/run green yet. After independent source review, the shared core opening change requires original11 core ordinary units +12 integrations in addition to directed lookup/resume and relevant receiver/outbound checks. Do not use substring `transfer::tests::` as if exactly11: it also matches doc_transfer codec names; use exact per-case binary filters or explicitly account distinct names. No unnecessary epoch/fence matrix duplication.

## Frozen source and recovery

External `Q039/transfer-lookup-candidate-01` and internal recovery same name contain complete patch including new tests, seven exact sources, refs/status and **1072** first-party/build input fingerprints. Original12/8/9 unchanged. SHA256:

| Source | SHA256 |
| --- | --- |
| transfer.rs | 6063234269a393428dbb634f209dc71df16a834b6bb0073c246776bd24a5eb8a |
| doc_transfer.rs | d9fde4f64ccd61248384b45d4460f1f86c28d51372a309699a17bb91b3be211f |
| body.rs | 663f359c55e1f3b7ca4f24c86059e23d8f61d01010ac6b3995aa438d80dc182b |
| outbound.rs | f3e0e89b6f0c76517b85fdcb292989831b6aa1fcb23522eb5ffbfae09e980cc0 |
| receive.rs | f3db83ca30a37d947704429aac58c11732f6f4f39865df4d4f853d2176006dda |
| transfer_lookup.rs | 65f039b5200d22267a8200f90eafaa5e600e26cd3a94d6e97de8a701097435e3 |
| doc_resume.rs | 6074aede3e7de6d4bacfa0e0f9bde217ef46d647e2413a4a731832472818ff43 |

Sole Cargo/native/gen slot unclaimed; no owned process. Wait root's source review/slot authorization, then real exact validation with independent explicit AST-checked runner step/total constants and double-disk5GiB guard. No commit yet.

## Candidate02 — pre-clone metadata admission

Author source audit identified that open_existing initially cloned a borrowed Manifest before the common owned helper validated its metadata budget. Root approved a minimal **`manifest.validate()?` before clone** to avoid an extra copy of oversized borrowed binding before rejection. This is a static resource-risk correction, not a claimed runtime bug red; no implementation-mirroring test added. Existing malformed lookup contract remains unchanged.

Candidate01 preserved. `Q039/transfer-lookup-candidate-02` and internal recovery same-name freeze contain updated complete patch/seven sources/1072 inputs. Exactly one production line changed relative to candidate01, transfer.rs now SHA **50e71755aded30c9f0992218b4af0fe69a91a40e64aa598d56c2f9d60407dd4d** (62,284B); other six source hashes and all other1071 inputs identical. Rustfmt/diff-check passed; no Cargo/native/gen or runtime result. Root approved eventual normalSync +4lookup +5resume +8receive +9outbound +11exact core units +12core integrations =49 distinct ordinary parents, no repeated codec5. Source review/unique slot pending.


## Frozen candidate02 validation — green01

Root confirmed independent source review without blocker and granted the sole slot for exactly the agreed scope. `Q039/transfer-lookup-green-01`: normal defaultSync libcheck0; lookup**4/4**, resume**5/5**, receive**8/8**, outbound**9/9**, original core**11/11** by full exact names, original transfer integration**12/12**. **49 different ordinary names**, programmatically de-duplicated, no codec repetition and no helper-worker count inflation. Core first exact case was built/run through current-tree Cargo, remaining10 used that exact same unit binary with `--exact`; integration's1 ignored dedicated worker is actually spawned by its ordinary parent and not added to49.

Total **28.725s**, all17 stages exit0. Normal/lookup/resume/receive/outbound timings3.549/4.564/2.024/2.540/2.532s; core first3.041s, remaining10 exact cases each0.503–1.019s; final integration4.066s. Logs/results contain every actual name, command and nonzero count. Existing vendor HasIdSpan unused-import warning is unrelated.

AST-checked explicit independent STEP_TIMEOUT_SECONDS=240/TOTAL_TIMEOUT_SECONDS=600/MIN_FREE_BYTES=5368709120 before outer launch and runner child launch; no textual substitution, guard deviation or guard trigger. Jobs1/locked/offline/quality-target/incremental0 and short owned TMPDIR. All**1072** source/input hashes before=after=current=candidate02 including exact7source, original12/8/9 tests and lock. **49** sync/catalog Rust/Cargo input mtimes touched only, SHA unchanged; raw normal/test logs prove current native-io catalog/sync compile/check.

Six unique actual binaries retain unchanged path/SHA/size/mtime before/after. Fingerprint JSON copied and verified **features[]/rustflags[]** for all6:

| Binary group | SHA256 |
| --- | --- |
| lookup4 | 74c6a3ff273355366f56ceec53812be10aefb1804630ad8692809fd758c15f6f |
| resume5 | 42d30c52f8348b002966d134c538ba2448f53435c4b9c96056e8817909d440e1 |
| receive8 | 8df6a4cd13dcc4778c8ec8b23e547e4ed0ffd65a0042af0d03cf32c66a10be04 |
| outbound9 | 95f677e022ab02fd88d9b56f23f3869834a09a6c88e253afcf50418fe87d003d |
| core11 unit | 2831e4570d23b0be3cd92dee75af27b77b4e7087864f14ffc9c037381e8a47b4 |
| core12 integration | 0dbddbe55f77b490dee7fb4226f5d0c0049a43a7f038a62d80b5fd05bbab6028 |

Minimum external **6,567,673,856B** / internal **12,846,739,456B**. All17 own PGIDs are recorded in final/cleanup and signal0 confirmed absent. Short `/Volumes/disk0/Khor/caches/tmp/transfer-lookup-01` had zero files, four empty subdirs/root removed. Slot returned immediately after completion. Entire run/metadata/fingerprints/cleanup and card copied to internal recovery same-name directory; original candidate01/02 evidence retained. No source edits/new scope/new Cargo or commit after run; await final independent evidence review/commit authorization. This remains a library foundation, not Node/25MB/Applied/fairness/Pair completion.


## Final author commit and handoff

Root confirmed final independent evidence review found no blocker and authorized exactly candidate02 seven sources. Saved **8ffe6aa2a5eedfd3026ef1244578120f9834dcf6** (`feat(sync): reopen retained transfers and resume fixed document spools`), direct parent **3e46b6e4a92c146952a3794d85b8dddbf12d4653**, on `fix/quality-doc-transfer`, tracked clean. Exactly5 production files +2 new tests; original tests, lock, platform/store/apply/Node/proto/dependencies unchanged.

`transfer-lookup-green-01/commit-receipt.json` records testedfreeze=gitblob=worktree for each seven full SHA, all1072 inputs unchanged and all6 actual tested binaries still matching SHA/size/mtime at commit. All17 former owned PGIDs signal0 absent; scratch remains absent, no new Cargo/native/gen call. Receipt and final card are in both external evidence and internal recovery. Commit hook refreshed graph only (its broader10-file incremental analysis is not the git commit scope and not new runtime evidence).

Source write ownership returned to root. Public open_existing/retained_manifests/resume are delivered together for root's independent integration and Node owner adoption. No further library work, main merge or ship performed; full Node transfer/receipt/25MB/fairness/Pair targets remain open.
