# Q048 — independent f721 integration evidence review

Owner: `doc_apply_review`, independent of root's integration and execution. Limited evidence review completed without blocker. This does not close all Q039 or authorize main merge/ship. Reviewer ran no Cargo, generator, fixtures, process signals or cleanup.

## Frame and actual checks

Evidence `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/integrated-f721-final-01` corresponds to root wave1 HEAD `f721a2c3ac0e12cb1faf5dd9634015dea1054a4b`. Independently read runner, preflight, freshness, all raw Cargo logs, final/results, binary receipt and integration receipt. Current root HEAD remains f721 and tracked clean. All 1068 before/after input hashes are identical and match current files individually. This integration contains the reviewed initialization fix, descriptor and prior Doc work; it does not contain the new dirty outbound adapter.

The runner sets the shared quality target, jobs 1, offline/locked, `CARGO_INCREMENTAL=0`, clears KHOR_AGENT_BUNDLE and uses its own `/Volumes/disk0/Khor/caches/tmp/if721` scratch. It refreshed 41 Sync/catalog Rust/manifest mtimes without changing bytes. The normal log actually compiles catalog and checks Sync from this exact wave1 tree; the unit log actually compiles Sync test code from this tree. No profile/feature bypass or copied prebuilt artifact is used. The existing vendor Loro unused-import warning is preserved.

Actual four phases all exit 0 with no guard stop:

| Phase | Actual ordinary tests | Count treatment |
| --- | --- | --- |
| Normal Sync library check | — | Build/check only |
| `--lib transfer::tests::` | 16 (11 core + 5 descriptor) | Rust substring filter also matches `doc_transfer::tests::` |
| `--test transfer` | 12 | Core integration parents |
| `--lib doc_transfer::tests::` | 5 | Exact repeats of the five above; add zero |

Independently compared the named tests and Rust summaries: **28 distinct ordinary parents**, not 33. The ignored initialization and kernel-owner workers are real children of their respective parents and are not additional ordinary tests. No ignored helper was promoted to an unexecuted green based on its name.

The original initialization recovery parent logs child 91272 killed by SIGKILL, actual `.initializing`, public `reserved=Ok(19)` and fresh `VerifiedComplete {bytes:44}`. The second-cleanup parent logs 91283 offsets_synced then 91284 cleanup_after_unlink. The phase parent logs 91285/91286/91287/91289/91295/91296/91297/91298 for before_mkdir, manifest_partial, manifest_synced, body_created, offsets_partial, offsets_synced, before_publish and after_publish respectively; the final case's actual namespace is the published 64-hex id. These are the unchanged production-stage hooks/public API assertions reviewed in Q039 init, rather than hand removal presented as recovery. Child traces do not inflate the parent count.

## Artifact, resources and cleanup

Both unit invocations name `/Volumes/disk0/Khor/caches/rust/khor-quality-target/debug/deps/khor_sync-6bc0f9688422d392`. The post-descriptor contemporaneous binary receipt records SHA `f53aa961a2a91113f8e2733423b4023ebeabc2264e6104cf6ed22e0ba77adab9`. At this independent read the authorized next outbound build has already reused that shared path (current SHA `22bd496850a4b1bee856f1f54461380f8d4849363c1e9bb11b4f22a0b924a669`). Therefore this audit confirms the recorded f721 run/artifact linkage from its fresh compile logs and receipt, not a present-file hash match or an archived f721 binary. That ordinary serial shared-target replacement does not alter the matching source/log evidence or make the next binary a f721 artifact.

Final duration is 17.522849249988212 seconds. Observed minimum free bytes are external 6,760,087,552 and internal 12,829,691,904, both above 5 GiB; no resource guard fired. Independently confirmed all four recorded PGIDs 91119/91180/91307/91346 and all eleven traced initialization children above are now ESRCH. No reviewer signals were used.

Integration receipt explicitly preserves the initial post-run cleanup assertion mistake: the root expected no entries but found only its known empty `transfer` container; that assertion stopped before deleting files. Root then checked the empty container, removed it and its run-owned scratch root and wrote the final receipt. The scratch path is independently absent now. This audit has the final receipt/current absence, not an independent pre-removal directory observation, and does not count that cleanup setup assertion as a business red or hide it as a successful first attempt.

The internal mirror `/Users/zgh/Library/Caches/khor-code-quality/recovery/integrated-f721-final-01` has the exact same relative file set and all 14 SHA values as external evidence at this check.

Conclusion: the exact f721 integration has adequate evidence for normal Sync check and the 28 distinct relevant parents. No need to repeat those tests solely to correct the duplicate count. This is not Node-wide regression, Windows runtime, power-loss durability, new outbound behavior or real large-document convergence evidence; those boundaries remain with their own tasks.
