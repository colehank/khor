# Q051：686f 有限集成证据独立审查

日期 2026-10-27；审查者 `node_test_review`，非实现/runner 作者。仅写本卡，未 Cargo、未生成、未运行测试、未改源码/进程/缓存或提交。范围为 Q051 集成与实际证据核对；不重称一次完整生产逻辑审查。实现源码前审见 Q-039-transfer-lookup-review，graph 的旧覆盖/零指标不作为本次能力或测试覆盖证据。

结论：**该有限集成 frame 无证据阻断**。wave1 当前 HEAD `686f2e42eb634ff5ba0edc34ab822fe8764f13f2`、`integrate/code-quality-wave1` tracked/untracked clean；实际 normal khor-sync library check 与 **49 个不同普通测试**通过。证据为 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/integrated-686f-final-01`，internal recovery 同名镜像。不能据此宣称 Node/Pair 25MB 收敛、持久网络 Applied 闭环、其他平台 runtime、P5 或 main/ship。

## 精确集成与源稳定

- 独立对 `686f` 的七个变更路径逐 blob 比较 author `8ffe6aa2`、集成 commit 与现场文件，全部相同：transfer.rs，doc_transfer.rs，doc_transfer/{body,outbound,receive}.rs，tests/{transfer_lookup,doc_resume}.rs。SHA 与前审冻结七源表全部一致；没有依赖/oracle 漂移夹入 cherry-pick。
- `all-inputs.json = source-before.json = source-after.json` 的 **1076** 个 hash 全同，并逐项重算与当前 wave1 相同。该 manifest 覆盖 runner 明示的 crates/vendor/agent/gui-core/Tauri/scripts 与 Cargo/workflow 路径；不是所有应用/机器/生成制品输入的穷尽保证。
- freshness 记录 **49** 个 Sync/catalog source/manifest mtime 更新且字节不变。normal-lib 原日志明确从当前 wave1 compiling catalog/checking khor-sync；lookup 与 core-00 原日志实际编译 Sync test artifacts。保留 vendor `HasIdSpan` unused import 警告，没有用其他 features 或隐藏失败换绿。

## 原日志与计数

独立从全部 **17** phase 原日志重新解析 ordinary `test … ok` 名单与 Rust summary，逐 phase 与 results/final 对应；17 phase 均 exit 0、无 stopped reason/source 变更/live PGID。normal check 不计测试；11 个 exact core phase 各一普通父；ignored process worker 不单独加数。

| 实际组 | 不同普通通过数 |
| --- | ---: |
| lookup | 4 |
| resume | 5 |
| receive | 8 |
| outbound | 9 |
| exact core unit | 11 |
| core integration | 12 |
| 合计（raw names 去重） | **49** |

49 个原日志名字全部唯一且等于 final 的 actual_names，未将 author49 + root49 算成98。core-integration summary 12 passed/0 failed/1 ignored，ignored worker 由父调用的覆盖边界保持原审说明。core-00 通过 Cargo 全限定名 `--exact`，后十项直接使用同一实际 unit binary 全限定名 `--exact`，没有 substring 扩入 codec/helper。

## 实际二进制与执行控制

六个 actual binary 来自原 Cargo Running 行或后续 exact unit command；独审分别核其 receipt SHA/bytes/mtime 在 run 前后相同，审查时当前文件也三项全相同（后续合法 Cargo 可覆盖共享 artifact，不能据新 binary 回填本 run）。

| Binary basename | SHA256 |
| --- | --- |
| transfer_lookup-bde48dae5e6a93f2 | `6258433da2e972036e0963209cbf329bc207e3f39fbc5d507ca55dec94569102` |
| doc_resume-f5ff697ebfe14ef1 | `612e37e37d26a999118d0aaa235f1293c198d969db9e36cbbac859c8f28069bc` |
| doc_receive-1747035ff8a0277f | `5731312c51dbadb58057292a3b0bd419cfc682c4d2503bd9d57b0bb41a420e2a` |
| doc_outbound-761610bf76283ee7 | `04851151926330448d2480f381f7733b0e6f20b959e7e61851aa0a84664b42d0` |
| khor_sync-6bc0f9688422d392 | `a1653b27342c101d6c8c3ebe0e1a05f9281d48ab2a5b57f8746b30f43057f039` |
| transfer-ebd3cc272a00feb9 | `1a1785160985ec53f86a07fbaf71966d053830845c32264ba888c86d96a2863a` |

已完整读 runner 并独立解析 AST：`STEP_TIMEOUT_SECONDS=240`、`TOTAL_TIMEOUT_SECONDS=600`、`MIN_FREE_BYTES=5368709120` 为独立具名常量，实际 loop 分别引用，launch receipt/自检/当前 script SHA `435fc4789fa1cdcdddc1a968924f48a00d5df4b059d9473b0a1c46030c72f700` 全吻合。源 precheck assert 在 launch 前且失败会停止；标准 locked/offline/jobs1、共享 quality target、incremental0、短 owned TMP。每 phase 新 session，resource/deadline/连续3次 IO 错误仅停自己的 PGID。

总 **29.402282542s**；记录采样 minimum free 外盘 **6,581,297,152B**、内盘 **12,840,390,656B** 均超过5GiB，无 guard fired。此数是 runner 采样记录，未伪称审查者独立连续监测。

17 phase PGID 当前独立探测均 ESRCH。原日志另见真实11个初始化 child SIGKILL（63163；63170/63172/63173/63179/63180/63182/63184/63185；63188/63189），当前逐 PID ESRCH；phase 从 before_mkdir 至 after_publish、第二次 cleanup_after_unlink 均有真实 marker，不加成普通测试数量。

`integration-receipt.json` 记录 scratch 无文件、只清已知空容器；独审当前 `/Volumes/disk0/Khor/caches/tmp/i686f` 不存在。本审查未亲见清理前目录列表，因此明确是 receipt + 当前 absence，未自称独立执行或重新清理。

外盘与 internal recovery 证据的 relative file sets 及全部 **46** files 内容 SHA 独立比较一致，含完整原日志、runner、AST/launch receipt、source freeze/before/after、per-binary records 与结果/cleanup receipt。现有有限证据足以确认该集成49通过与七源交付；未来 Node adoption/receipt/Pair/平台/最终 App 验收仍分别留 open。
