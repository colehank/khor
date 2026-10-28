# Q037 R0 — independent first-red fixture review

Owner: `doc_apply_review`, not the fixture author. Frozen source and actual first-red evidence reviews completed without a delivery blocker. **R0 business red is established:** after four occupied cancellation owners retire, the fifth/sixth known-task requests still do not reach actual Cancel IPC during normal lifecycle observation. This reviewer ran no Cargo, generator, CLI, native fixture or process cleanup. This closes the first-red fixture evidence slice, not the production R1/R2 fixes or whole Q037.

## Exact reviewed frame

Tree `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`, branch `fix/quality-remote-cancel`, HEAD `f721a2c3ac0e12cb1faf5dd9634015dea1054a4b`. Read central task/contract and actual diff/source. Project review skill graph calls preceded source fallback: minimal context, detect changes, affected flows, tests_for and impact radius. Graph is built on 7550e4f0 rather than f721, does not index this fixture, and cannot find cancel_children. Its zero affected-flow/test results are not safety evidence. No index was generated.

| Source | SHA256 |
| --- | --- |
| `crates/node/src/khor_agent.rs` | `03289b0f471cbe0015b2295763b7945093b39f71a57d0ecefd81d26e01c12fa8` |
| new `crates/node/src/khor_agent_remote_cancel_tests.rs` | `32cf0c86b3e8d1f07623467cbc1189087aeff2cac19bfe6258213761b7a9bac9` |
| new `crates/node/src/fixtures/remote-cancel.py` | `f9e7760583b86bd3809504573ba60827fb1b492a5c65a4f6f18dd6412956309a` |

All three current byte streams match external `Q037/remote-red/candidate-20261026T162252Z/source-freeze.json`/source and internal `recovery/inflight-source/Q037-REMOTE/r0-20261026T162252Z` source copies. Removing exactly the appended blank line plus cfg(all(test,unix))/path/mod wiring leaves khor_agent byte-identical to HEAD. Status is exactly those three paths; original production flow and 106 prior fixtures are not modified. Python parsed with ast only, without execution/pycache. No static compile error identified; only actual compiler output can establish build success.

## Main-path oracle validity

Read all Rust/Python fixture source, then followed actual khor_agent Prompt/Stop/retirement/reconcile, mcp_worker spawn/poll/schedule_cancel, delegation track_child/cancel_children and A2A Store/Execution methods.

- The ordinary test starts a fresh isolated libtest executable selecting only the ignored remote_cancel_worker, so schedule_cancel's static SWEEPS is fresh. The helper is an actual child, not an extra ordinary case. `KHOR_TEST_MCP_EXECUTABLE` is required as an absolute existing CLI path; provenance must be supplied by the run, since the test alone cannot prove it was rebuilt.
- The same private Node executes six actual Prompt/Stop actions. Python consumes the formal start, writes its actual PID/readiness, logs a received abort and exits cooperatively; its 25-second alarm remains independent. Local retirement checks require Saved cancelled, no Running map entry, successful executor claim and actual Pi ESRCH; abort receipt is separately checked.
- The known-task link is not fabricated JSON: actual Store.accept, Execution.claim/update Working and production track_child write the controlled peer record/link, with real caller, parent run and home. Dropping Execution releases its lease without changing its Working record. This accurately exercises durable known-task link consumption but is not a complete delegate transport acceptance, authorization or ACP execution proof.
- Actual cancel_children opens the root and reads those links, then formal delegation_on emits Get followed by Cancel for nonterminal Working. The controlled peer parses the real Handoff framing, validates the private cookie and exact machine/agent/task, and returns the actual cloned Record in the production Reply encoding. It accepts only Get/Cancel; unexpected traffic/errors are surfaced as setup errors.
- Each of the first four runs waits for that exact task's actual Cancel IPC event before proceeding. Replies remain blocked by index under one release flag; no directory/arrival order assumption or hard sleep manufactures four occupancy. Exact CLI direct children are discovered by PPID plus full `<fresh CLI> _mcp_worker` command. At the sixth retirement the fixture requires four current direct children, four accumulated exact identities and less than four seconds since the first held Cancel. Failing this premise is setup failure, not R0. Those actual live unreaped workers establish occupied existing sweep owners; a completed Worker.poll would have reaped them before its sweep handle finishes.
- Only after that premise is checked are the four replies released and all four exact CLI PIDs awaited ESRCH. The next up-to-eight-second window drives only formal reconcile_agent and Action::Read, not a new Prompt/Stop, private scheduler or global Vec/counter. It also exceeds the original five-second owner budget. The sole business failure is missing actual Cancel arrival for tasks 5 or 6. Printed remoteCancellation metadata and process counts diagnose setup; they do not replace that delivery oracle or falsely claim remote cancellation confirmed.

## Cleanup and failure classification

Within the isolated helper, exercise Result and caught panic both enter explicit cleanup before any business assertion. Cleanup releases barriers, asks an actually Running executor to stop, waits map/claim drain, stops/joins the fixture listener and its normal-path handlers, and waits all logged Pi plus current exact CLI children ESRCH. Its result is asserted before the primary business outcome, so a cleanup failure cannot be presented as the expected R0 red. Rust panic is converted to a setup error while retaining cleanup, not converted to success.

The outer parent waits/reaps its owned libtest child and applies a 55-second process-group fuse. It observes exact direct CLI children every 250ms for fallback accounting. Fallback signals require the full owned Python/CLI command and `getpgid(pid)==pid`; it does not signal a bare logged PID or the user's general process group. On the ordinary path inner cleanup has already proved process disappearance, and fallback should do nothing. The Pi alarm, worker five-second fuse, request three-second read fuse, held reply fifteen-second fuse and listener forty-second bound are distinct safeguards; none is evidence of a successful clean run before execution.

Nonblocking static limits, to keep runtime claims precise:

- Peer listener returns early on a rare accept/non-loopback error, and `handler.join()?` returns early on a handler panic. Those exceptional branches can drop remaining JoinHandles rather than explicitly joining every handler. The eventual isolated-process wait/OS teardown is a separate fallback, not a universal all-background-joined guarantee. The intended R0 path must show cleanup joined/drain/reaped all Ok; any exception is setup/cleanup failure and needs its own correction before classifying a business red.
- The outer PS observer uses unwrap_or_default, so its fallback inventory is best effort. Normal-path inner worker_pids and Pi log checks are fallible, explicit cleanup checks. A wall fuse or observation error is not an adequate R0 result. Existing 5-second worker fuse still bounds a missed CLI independently; no claim is made that outer polling captures every possible future child under arbitrary panics.
- The four-second observation is a tested setup premise rather than a scheduling guarantee on a slow machine. A lost window must remain setup failure; do not relax it or infer saturation from six Stops alone. Actual CLI/schema freshness and short TMPDIR provenance are execution preconditions.

## Required execution evidence

Build the current formal CLI and libtest from a frozen input manifest under the sole authorized slot; do not reuse old release or a fixture implementing worker behavior. Run only `six_stopped_runs_eventually_deliver_remote_cancel_after_four_held_owners_retire` as the ordinary parent, with the actual ignored helper spawned internally. Retain full logs, exact binaries/hashes, source stability, resource guards and child/PGID cleanup receipts.

A valid first red needs all six Pi/local retirements, four task-specific Get→Cancel barriers and exact live CLI premise, release/reap of the four, normal lifecycle observation, then explicit all-Ok cleanup **before** the missing-task business assertion. Compiler/setup/fuse/cleanup failures are not a business red. No changes to production cancellation or assertions were made by this review. R1 durable intent/fair pump/receipt and R2 unknown-task/late acceptance fencing remain open design/implementation work; this fixture addresses the narrower known-task delivery admission gap.

## Actual first-red run independently accepted

Evidence `Q037/remote-red/run-20261026T163311Z` was read in full for the executed pipeline, raw frontend/CLI/test-compile/exact-r0 logs, all phase results, first-red receipt, before/after binary receipts, source manifests and cleanup/delivery receipts. Three fixture source hashes remain unchanged at the end of this audit. Root/author have not changed production cancellation for this run.

Normal same-tree offline frontend build succeeded (87.8976317919878 seconds), followed by actual formal CLI build (11.139169957998092 seconds) and Node libtest no-run compilation (53.579483541994705 seconds). Frontend log shows standard prebuild/gen_ts/gen_bindings, tsc and Vite; producer manifest has 155 generated source files plus 277 dist files, with no borrowed old dist. Current producer-output hashes match the complete input frame. Freshness records 419 first-party mtime updates. Raw compile logs name current agent-protocol first-party paths, and test compilation names the exact Node libtest used next. These are build/setup successes, not additional behavior tests.

The 1914 original tracked input hashes are stable through frontend. After ordinary generated/dist output and locked node_modules package metadata are added, all **2347** frozen input hashes match CLI before/after, test-compile before/after, exact-R0 before/after and independently current author bytes. No source bypass or post-failure oracle weakening occurred.

Both contemporaneous before/after binary receipts agree, and reviewer rehashed both current binaries:

| Artifact | SHA256 | Bytes |
| --- | --- | --- |
| actual CLI `debug/khor` | `efcce3e14eb59cc0e05230a95a6bcce03cd20bfb377863fdc8c25470cf268af6` | 257,288,312 |
| actual `khor_node-73af34573690fd43` | `b2ec51a0ab013f6342805480104161dd29dc520000a67259ae787b0f1113a811` | 229,287,064 |

### Actual failure and causality

The exact ordinary parent invocation took 11.726716667006258 seconds; Rust's ordinary parent summary is 0 passed/1 failed, 0 ignored (11.50 seconds). Its one ignored helper was actually selected/spawned and failed its inner business assertion (11.47 seconds), not a second distinct parent. Inner/outer libtest exit is 101. The pipeline wrapper then returns 1 due to check=True; its Python CalledProcessError is downstream of the retained business assertion, not an independent fixture setup exception.

Raw log shows all six actual Prompt/readiness/durable track_child/Stop cycles. Pi PIDs 19996/20037/20044/20049/20056/20057 each reach ESRCH, Saved cancelled, map false and claim free. Four initial exact task ids each log real cookie Get and Cancel. First-four reply holds and the source's less-than-four-second/live-four premise both pass, as execution reaches the release barrier. Four actual CLI child PIDs 19999/20038/20045/20052 are then released and awaited ESRCH before normal lifecycle observation.

After the unchanged formal Read/reconcile eight-second loop, actual Cancel count remains four. Exact missing later tasks are `01a17913-566a-7649-9b61-d6e3dd764ae4` and `01a17913-570d-72c3-928b-06cd074e75c6`. Their printed sweep not_admitted metadata diagnoses the current implementation but is not an assertion that would forbid a future fair implementation; only missing actual IPC delivery triggers the failure.

The raw log next proves `IPC handlers/listener joined=Ok(())`, `map/claim drain=Ok(())`, and `all owned Pi/CLI ESRCH=Ok(())` **before** the inner line-498 BUSINESS RED R0 assertion. Outer wait records its child reaped, Pi/native ESRCH true and cleanup empty before its own failure assertion. Thus none of the exceptional cleanup limitations from static review was exercised, no fuse killed the test into a synthetic business red, and no setup/source/resource/cleanup error preceded the intended failure.

This is a real known-task cancellation admission/delivery gap through current production Stop and worker scheduling. It is not proof that a complete delegate/ACP launch was canceled, a mesh authorization path was exercised, or a remote ACP terminal receipt was persisted/queried. The controlled peer response remains deliberately narrower.

### Cleanup, resource and delivery checks

All recorded phase `remaining_owned_groups` maps are empty. Reviewer independently confirmed all 13 first-red receipt PIDs (runner, wrapper/libtest, six Pi and four CLI) ESRCH, plus frontend/CLI/test-build PGIDs 17785/19152/19302 ESRCH. The executed runner uses explicit asserted 180-second stage/360-second total constants, source guards and 5 GiB resource checks; no guard fired, phase I/O-error counters are zero, and total pipeline time is 165.4668311669957 seconds. The minimum across phases is external 6,372,880,384 and internal 12,868,730,880 bytes, above 5 GiB.

Only exclusively created `/Volumes/disk0/Khor/caches/tmp/r0` was cleaned. Saved pre-cleanup manifest has 250 regular single-link files totaling 3,362,456 bytes, all with SHA/metadata and contained in that root's compile-cache tree; lsof stdout/stderr empty with no open handle result. Cleanup receipt marks exclusive ownership and deletion, and the path is independently absent now. Reviewer did not perform deletion or claim an independent pre-deletion filesystem/lsof observation.

Rehashed every delivery-snapshot entry against both internal `recovery/inflight-source/Q037-REMOTE/r0-delivery-20261026T1640Z` and external candidate/run source: **35 files / 2,839,910 bytes** match all recorded SHA values. No broad cleanup, user HOME change or service action is part of the reviewed run.

Conclusion: fixture delivery is acceptable as the first-red regression for the precise known-task gap, with one distinct failing ordinary parent and clean owned resources. R1 production changes require their own authorized fixed frame and independent review; they are not approved by accepting this red. Preserve this exact business oracle through implementation and keep setup/cleanup failures separately classified.
