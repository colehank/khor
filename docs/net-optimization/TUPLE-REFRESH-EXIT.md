# 补丁38退出的实际验收边界

问题是当前 iroh1.4.1/Noq1.4.0 是否已能正确跟随实际 NAT 远端 tuple，允许删除本地定期 reconcile helper。版本号和库测试通过都不能单独回答。

两臂是 R（tree cf9d172b）和仅移除该 helper、保留 select_path_on(true) 的 M38（tree 2d702718）。两臂不含 W 唤醒消融。Mac 完整 iroh 库对照已经结束：各198通过、0失败、原ignored1、0filtered，199项名称与结果一致；证据为 E/public-mac-tuple-exit-{baseline,disabled}-v3 中的准确源码、产物及核验记录（E=/Volumes/disk0/Khor/evidence/net）。这只满足库检查，尚不批准退出。

真实 Linux 验收沿用 `/Volumes/disk0/Khor/evidence/net/tuple-refresh-exit-plan-v1.json` 的原 r0/r1 和强 oracle。两臂必须先有相同 net-diagnostics feature 的真实 netlab 与实际 CLI producer；原 native_provenance、源/上下文/ELF/dep-info 核准全部保持。编译不与性能计时争用 Turing。

原物理夹具 `crates/cli/tests/netlab/paths/rebind.rs` 在R/M38两臂的SHA256已核，必须保持为 `ac97726906429a5dd63dd91b5c9ed99624c273d9a253f7facb916ffcf47e8a9b`。先核 r0 的真实外部端口变化，再执行 r1；不能用设置过规则、缓存状态或最终一次成功替代注入与业务。

固定 R M M R R M，每臂3个新场景，不挑绿、不替换。原124次读及30秒caller、120秒setup、45秒Direct保持和240秒场景上限不变。`scripts/netlab-nat-selection.py` 的 `warm_remote_requests_select_the_rebound_tuple_v1` 还要求：接口IPv4/IPv6集合不变，conntrack删除和实际50001→50002映射成立；全部124逻辑读成功；30次暖远端 stream-open 全部选中新 tuple、cached/actual 不匹配为零，并有新 tuple 的 journal 见证。r1 原Rust门没有直接断言这个选路属性，必须另核oracle。

实际固定批次已结束，runtime control `35f36b80d5d7fd21b20a7cac66164d8426618a61`。6块/12个原native场景、744次逻辑读全部成功，180次暖远端stream-open全量保留，替代0；实际source/control/注入/owner与producer的selected文件还原均已核验。producer R为102852Z-ad942c9e，M38为103337Z-208ba0bf，准确tree分别cf9d172b与2d702718。root串行器53952已实际退出。

| 顺序 | 源 | Turing run | 暖远端选中新tuple | cached/actual mismatch | 强属性 |
| --- | --- | --- | --- | --- | --- |
| 1 | R | 104315Z-1e59e08a | 30/30 | 0 | 通过 |
| 2 | M38 | 105128Z-1fdcc171 | 0/30 | 83 | 失败 |
| 3 | M38 | 105258Z-4bbd8c01 | 30/30 | 61 | 失败 |
| 4 | R | 105528Z-ef353d3e | 30/30 | 0 | 通过 |
| 5 | R | 105704Z-a3c42047 | 30/30 | 0 | 通过 |
| 6 | M38 | 105819Z-07615539 | 0/30 | 71 | 失败 |

R强属性3/3；M38强属性0/3，累计215个mismatch。M38三轮原Rust业务门都通过，但强选路合同都未满足；其中一轮暖远端正确不消除其他路径缓存不匹配。原日志保留cached remote50001、actual remote50002、同Noq连接/path ID的具体见证。完整结果E/tuple-exit-nat-comparison-verification-v1.json，各run下tuple-exit-nat-verification-v1.json由本地数据分析重新执行原oracle并核对远端结果。

**结论：在当前R/M38源、iroh1.4.1/Noq1.4.0与所测NAT合同下保留patch38，不批准移除。** 库绿不足以替代该合同；这是该准确源的保留依据，不是所有移动场景都必须使用此实现的证明。ledger v5据此将此项从待核改为保留，另外56项退出仍待审查。最终集成源码仍须核相关行为。

它不覆盖独立IPv6隐私地址/前缀、双端与middle同时移动、真实iPhone或能耗；这些仍属于完整N4/N5。
