# Q039 strict Doc apply — independent candidate-03 review

Reviewer `doc_apply_review`, not the implementation author. Review status: **no source blocker found in the frozen candidate-03 scope; behavioral validation and root integration still required**. This card does not close Q039 or authorize main merge/ship. No Cargo, production mutation, service, SSH or user HOME action was performed. The only write is this review card.

## Fixed input and method

Read authoritative STATE/PLAN and Q-039-doc-apply, Q-039-doc-apply-owner, Q-039-pending-root-review. Author tree `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`, branch `fix/quality-sync-apply`, HEAD `1d2640d2de0169184242c503167b55bc2a9ddde5`; actual status has only the allowed modified store.rs and new store_apply.rs/tests/doc_apply.rs. The following SHA256 values were checked twice and match external candidate-03 `source-prepared.json`:

| File | SHA256 |
| --- | --- |
| crates/sync/src/store.rs | edd8828093e7643fb394ba1b77477fcb0e3a701a8244c654057f92eea32ca092 |
| crates/sync/src/store_apply.rs | df7729da78f17fb15546ffdacbc6eb4bae87cec0889e119d00d5be5bf78b5b29 |
| crates/sync/tests/doc_apply.rs | 14e2b593d22418b3d13f34c20ca065c65b5f569c0350c8ccb36e9b1292412a2a |

Used project review-changes skill and code-review-graph minimal/detect_changes/affected_flows/tests_for/impact first. Graph build is `58450bbc`, not actual HEAD; it reported no apply_checked node and only the tracked store diff, omitting the two new files. Zero flows/risk is therefore not safety evidence. No `.codegraph/` exists in the author tree. Fell back to actual diff and full new API/test sources, and relevant existing Lease/load/load_pile/reseed/cache/record_import/absorb/flush/write_atomic/glue and real PinDoc/DeviceDoc delegates. Reviewed implementation before relying on the author's implementation account.

## Source findings

### High risk: no blocking defect identified in this slice

- `apply_checked` lines 86–129 checks loaded epoch, rejects negative spec counters, requires both live-before and store.on_disk to cover base before importing, and constructs only `D::open` in memory. Its seed requires no minted constructor ops, no seed pending and exact before VV; candidate body requires no pending and before/base/target coverage. It does not require target to contain base or local concurrent ops, and does not replace live.
- Lines 130–174 recheck unchanged live version, establish the real pile fence, reject unsupported Spare own-writer advancement, pre-retire when projected or registered pending own-writer counters can advance, merge into original live, register that actual report, retain final checked retirement and require actual coverage. The final actual may legitimately exceed candidate/target when an old live pending dependency is supplied.
- Lines 175–194 require successful ordinary flush, a second fence/epoch check, and on_disk coverage of actual plus target before constructing private-field FlushedImport. Duplicate can flush dirty local ops. Commit errors honestly permit live/partial disk changes; the type is neither network Applied nor directory power-loss durability.
- `ensure_apply_fence` lines 197–260 reuses existing Some lock without upgrading/dropping it. None acquires a real bounded shared lock, or exclusive for stamp initialization; identity is checked before epoch decisions. Stamp initialization rejects appeared `.loro` entries, rechecks epoch and writes only legal new stamp metadata under exclusive. Some+stamp+Missing fails closed. Holding the initialization exclusive until Loaded drop has the accepted concurrency cost; later Node owner must drop promptly.
- `strict_epoch` distinguishes missing, invalid bytes/text/u32 and filesystem IO. Unix identity compares held/current dev+ino. The Windows cfg declares a repr(C) 24-byte volume+128-bit FileIdInfo structure and calls class 0x12; no mtime substitution or dependency change. This is source inspection only, not a Windows execution claim.
- Checked persistent pen retirement writes a fresh name before clearing store.pen/recording retired. Predictable write failure occurs before live import. Registered old pending includes the own writer's maximum end, so the foreign-dependency case does not depend only on candidate projection. Spare rejects after safe controls exist, allowing drop+reload to obtain a new persistent pen without caller mkdir; this does not persist its old RAM queue.
- The only existing behavior change is absorb's real `merge_report` plus `record_import`, before its existing write/ledger/on_disk sequence. It closes the known pending-compaction hole without pretending ordinary old wire RAM pending is durable.

### Medium risk: validation and caller boundaries remain open

This reviewer did not compile or run candidate-03. Root/author's exact-source execution must establish API behavior, not rustfmt parsing or the older absorb red. The intended 23 top-level entries are 22 ordinary tests plus one ignored worker entry actually spawned by two parents, not 23 ordinary tests or two extra independent cases. At review time no candidate-03 green was used as evidence.

`Doc` methods have interior effects despite `&mut Loaded` entry: the no-other-writer-alias and registered-external-import preconditions at API lines 79–80 remain necessary. The pending map can predict only registered imports; callers that use direct merge and omit record_import cannot claim this boundary's own-writer safety. This is documented, not newly solved by this slice. No Node/Pair/transport call site currently uses apply_checked; authentication, source/document/body binding, input budgets, fair chunk transport, durable receipt replay and the 25MB convergence failure remain subsequent work.

The fence assumes cooperating writers/reseed honor `.pile`; identity checks detect a replacement observed at checkpoints, not arbitrary hostile filesystem mutation at every instruction. Existing ordinary Lease fallback and reseed behavior were not redesigned. No claim of transaction rollback or universal non-cooperating filesystem protection is made.

### Low risk: additional targeted checks, not observed defects

Current cases do not separately exercise InvalidVersion, a constructor that mints ops/returns SeedFailure, a seed snapshot that reproduces the wrong VV, or fence loss occurring after live import. The existing branches fail closed statically. When a concrete implementation/platform regression motivates them, use a transparent real-Doc wrapper or a narrow owned filesystem fault and verify the failure phase; do not manufacture successful import reports or copy apply logic into a fake. These omissions are not reported as runtime failures or source blockers.

## Test validity inspection

The new test file uses actual Loro-backed PinDoc histories for missing dependencies, target lies, concurrent base/target, duplicate/dirty-local, preservation/unlocking of old pending, and actual DeviceDoc/ChatDoc cases. The old-own-pending retirement fixture's body contains a foreign writer dependency while the old live queue owns the dangerous writer's edit; retirement failure asserts neither enters live, then restores and retries. Candidate rejection checks value/VV and existing pen/epoch/ledger/block bytes where relevant. Spare recovery has no manual mkdir.

The block-failure proxy delegates real imports/version/snapshot/export; its one-shot hook is only on original live changes_since during flush. It renames the owned directory and adds a path-type conflict, after candidate construction and live import; assertions establish the hook was reached, live may have changed, no block appeared, and retry/reopen succeeds. Ledger conflict occurs after block write and checks real block contents plus retry. Neither fixture fakes a report or treats commit error as rollback.

The shared-stamp and replaced-inode tests check failure without dropping the original lock. Two owned child parents use readiness/release barriers and an 18s fuse; RAII kill/wait and explicit finish reap the owned child. Stale epoch changes the actual disk epoch after unfenced load; timeout checks the real bounded lock wait. These are credible fixtures by inspection, pending exact execution. The absorb test observes compact result, retained pending block, real edit after dependency and VV after reopen; the known pre-fix failure was reported by root/owner and was not independently rerun here.

## Handoff

Source review recommendation: candidate-03 may proceed to its authorized exact-source behavioral run. Integration recommendation remains conditional on those checks, root review and integration-version validation. Freeze changes require new SHA/frame and focused re-review. Windows runtime, final Node adapter ownership/drop behavior, actual network receipt semantics and full Q039 completion remain explicitly unverified.

## Independent evidence verification after candidate-03 execution

Root requested this separate read-only follow-up. Independently read green `Q039/doc-apply-green-01` runner, four actual test logs, phase process/result records, source and input manifests, binary receipts, plus red `Q039/doc-apply-red-01` log/process/result/source manifests. No new test or Cargo command was executed by the reviewer.

### Exact version and execution

- Candidate-03's three full SHA256 values remain exactly as above. All nine `source-prepared.json` entries match the green source manifest, actual current tree, external frozen source copies and internal same-name recovery source copies. Delivery manifest source map equals that nine-entry map; actual complete.patch hash equals its declared `e3a161708ed7422557d5fe8ba955580b10f41db8cee400760453f37bf6695433`.
- Green source-before/source-after are identical for all **44** inputs, and all 44 current files still match those hashes at this review. All **41** selected source input mtimes were refreshed with identical pre/post bytes. The actual logs identify this author tree and show khor-catalog/khor-sync compilation, followed by khor-sync recompilation for each later selected test executable, not merely an unverified reused test result.
- Four executable paths in actual `Running` log lines match binary receipts. The live binaries still match all four receipt hashes: doc_apply `b0cc7e07…`, pending_cache `9146b7f2…`, sync lib `a2b6fac4…`, transfer `3733f9c9…`. This ties the saved execution to concrete artifacts; it does not claim Windows execution.

| Actual suite | Rust summary | Meaning |
| --- | --- | --- |
| doc_apply | 22 passed, 0 failed, 1 ignored; 11.54s | 22 ordinary parent tests |
| pending_cache | 6 passed, 0 failed; 0.12s | Six different integration tests |
| sync lib | 147 passed, 0 failed; 34.50s | Includes the existing pending-map unit; do not add that unit again |
| transfer | 12 passed, 0 failed, 1 ignored; 3.02s | Optional ignored entry is not a pass |

The total is **187 different ordinary tests**, not a complete Q039 run. Actual doc_apply log contains two nested `running 1 test / pile_lock_worker ... ok` summaries, 10.02s and 0.01s, immediately within the timeout and stale-epoch parents. The top-level ignored worker is therefore actually executed twice through its parents; those child summaries are **not** added to 187. The fixed source's parent `.finish()` asserts successful child wait; its RAII fallback kills/waits the same owned child. Both parent tests pass.

All four phases exit 0, have `stopped=null` and source_stable=true. Saved runner elapsed is 60.337054916s. Recorded minimum available space across phases is external 8,096,743,424B and internal 6,294,171,648B. Saved owned-group checks are true, and reviewer `killpg(pgid, 0)` queries independently find all four phase groups 11802/12082/12103/12433 gone. Dedicated scratch is empty both in the receipt and current directory. The evidence `tmp` retains one ordinary sync-lib fixture (`khor-sync-test-wire-vv-12128/u-0000000000000091-00000000.loro`); empty scratch must not be expanded into a claim that every evidence temp file was removed. No cleanup was performed.

### Red-to-green and observable assertions

Original absorb red ran the exact selected business test after actual compilation (11.63s), exited 101 with 0 passed/1 failed and the content assertion `absorb must register pending before compaction deletes its only block`. Its trace is compact=Ok(snapshot), retained=false, edit=false, coverage=false. Red source-before/source-after contain 44 identical inputs, and the old store source matches candidate-01 with the old `doc.merge(bytes)?` absorb implementation. The unrelated initial setup-failure record did not execute Cargo and is not counted as the red.

The complete absorb test function from its signature through file end is byte-identical between the red candidate-01 source and green candidate-03 source (SHA256 `7374378f829dece9ea7b3f26c12a7a54c6f958288533e8897f4d984f976be043`); its source hash matches the red run manifest. Green actual trace is `compact=Err("cannot compact unresolved CRDT imports") retained=true edit=true coverage=true`, and the same content/VV/retained/Err assertions pass. This is a same-fixture behavioral red-to-green, not a renamed assertion or setup failure.

For apply failure/success phases, actual named tests all pass against the fixed source assertions independently inspected above: pending/invalid body/unmet target reject before live/VV/filesystem-image change; missing live or durable base rejects even when body could supply it; existing candidate rejection preserves pen/epoch/ledger/block bytes; shared-stamp Missing and replaced inode retain the held lock; actual stale disk epoch and real timeout reject before importing. Real report/unlocked old pending, concurrent/dirty/duplicate, persistent retirement failure and both Spare cases, block/ledger commit errors and retry/reopen are likewise actual passing cases. These logs mostly expose test names and assertion success, rather than serializing every before/after byte image or VV; this verification relies on the exact assertion source plus observed execution, and does not invent extra traces.

The pending log additionally prints true edit/target coverage with broken=0, rejects unresolved compaction while retained_edit_block=true, and shows unresolved warm reads have cache_hit=false. Both wire registration tests pass. No new success claim is made for old RAM-only pending persistence.

### Updated recommendation and retained limits

**The frozen candidate-03 source and its limited real behavioral evidence have no review blocker. Root may authorize saving/committing these exact three source files and the review record on the implementation/plan branches.** This is not permission to merge main or ship. Integration-version checks remain root-owned; source changes require a new frame and focused review.

Earlier low-risk unexercised branches remain unexercised: negative spec VV, constructor/seed fault variants and post-live-import fence loss. Windows runtime/FFI is still untested. The caller must register external imports, prevent other aliases from writing during apply, and retain/retry commit failures with dirty live state. Node adapter lifetime/drop behavior, authenticated transfer binding, network Applied/receipt replay and power-loss directory durability, Pair bootstrap, and actual 25MB convergence remain outside this evidence. The 187 passes do not close full Q039.
