# Q039 Node N1 candidate01 独立源码审

2026-10-27，reviewer `node_test_review`，未参与本片生产实现；只读源码/证据，独占本卡。实现树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer@5f2d5cf77ad05bd7e8ce2e11d0b90c36485c7ef3` +13授权源。未编译/测试/改源/清缓存/提交；作者仅获normal libcheck，不以其结果替代本审或N0运行。自身Gen candidate02三源继续冻结。

已读权威STATE/PLAN及Q-039-node-transfer当前N1契约。图first minimal+detect：本树无.codegraph；CRG仍build67927、HEAD5f2不匹配，7 changed漏6新模块、0flow不代表覆盖。随后完整读7 tracked diff及6新源（约2931行），并追实际shared handshake/admit、sync core prepare/resume/apply、load_pile/cache/fresh-copy与锁定rmp-serde源码。Pair/Vouch巨大N2明确不在N1本片，不因其未做判本片假完成，也不能称Q039整体完成。

## 已确认阻断 B1：1 MiB仅约束重编码，未约束真实入站DocTransfer帧

位置：`crates/node/src/link.rs:3525,3555–3567`，`link_doc_transfer.rs:85–104,766–827`；对照`proto.rs:1424`的response原始bytes长度门。

真实入站先 `recv.read_to_end(MAX_FRAME)`，通用6MiB；随后直接 `proto::decode::<Request>(&bytes)`。DocTransfer的validate_action只重新encode已解析action并检查其<=1MiB。锁定rmp-serde1.3.1 `src/decode.rs:1171–1187` 的from_slice→from_read_ref只调用Deserialize::deserialize，未检查reader耗尽。因此对一个合法当前DocTransfer（最小空Confirm或完整canonical Write）编码后追加零填充至2MiB、总量仍小于6MiB，decode仍返回原请求；roles/action语义检查通过并进入doc_step/State::open，真实入站1MiB契约被绕过。它不会得到无限内存（通用6MiB仍在），但当前声称的专用帧门不成立；不是N2/旧端兼容问题。

最小建议：在真实decode后、dispatch/任何metadata/stage之前对DocTransfer检查**原bytes.len**，或增加与proto::answer同级的小request解码入口并令实际handle使用。保shared admit/early-data规则，不用全局新框架或放大帧。定向行为验证：合法canonical Write/Confirm加尾部填充到>1MiB<=6MiB必须拒绝，且新receiver没有doc-transfer-state/blob副作用；原合法最坏envelope和完整256KiB chunk仍可用。不能只再测encode_request或直接validate_action，因为它们正是当前漏点。此结论来自完整实际链与锁定decoder源，尚未runtime复现。

## 已确认的测试准确性问题 C1（不是生产尺寸算法错误）

`doc_transfer_wire_tests.rs:55–64` 的最大Exchange request用同一个manifest方向ff→fe构造offer、proof、ack。实际 `validate_roles:934–936` 要求Exchange.proofs方向为fe→ff，与offer相反；因此该request虽满足encode_request的无上下文语义/大小检查，却不是整包role-legal。response那组方向正确。最小改法仅反转request proof的manifest角色；ff/fe都为同宽高字节编码，尺寸不会因此变小。适用验证应让这份最坏request也经过对应local/remote角色门；不要仅encode后便声称它可被真实receiver接收。该test尚未运行，因此不是已出现的假运行绿，也不据此断言1MiB容量不足。无需新协议/框架。

## 其它生产链核查（源码确认与限制）

- 正常Sync/Synced/rounds与wire Peer运输已删，当前DocTransfer单路径；Paired新增canonical issuer version，小Pair证书回推已drop Loaded再同pump。通用6MiB供其它请求，专用1MiB外壳及256KiB canonical chunk、合法VV/binding/批次等在producer/response路径已有门，B1只指出真实incomingraw漏门。
- `dispatch_inner:3600` shared admit及0rtt等待在DocTransfer之前；`answer_doc_transfer:93–103` role/name/action在doc_step创建State前。11 global+合法wide registry唯一，source/recipient/manifest绑定有各action方向门，不接任意路径。Context只带root/writer/local；单WORK+8等待令heavy closure内拿State/Loaded/pile/blob owner，退出后才跨network await。没有在该路径发现Loaded/metadataGate/blobOwner跨await。
- 公平pump `:239–275` 把initial Exchange最多双向2chunk也计入量子；3chunk剩余时不再开双向Exchange，每doc合计最多4chunk/1MiB，再yield并访问独立doc。总visit32MiB/30s为协作预算，单次prepare/resume CPU/IO不被该Instant硬中断；尚需实际N0时间/RSS，不声称O(chunk)工作集或已测公平。重复chunk也计bytes，不凭已存在数据跳过传输预算。
- 固定plan `State::offer:440–549` Ready沿完整manifest精确resume，不因late op重选target；Preparing先绑定完整doc/source/dst/epoch/base/target，完整core body验证后保存Ready才offer。唯一未发布incomplete Preparing可精确取消重规划，歧义/未知stage保留报reconcile。旧published epoch计划保留需显式reconcile，是候选已列限制；不能据此声称epoch reset后自动迁移/清理，但也不是旧Khor兼容要求。
- `receive:612–693` 先role/epoch，Receiving持久后才append，VerifiedComplete后持久Importing再调用真实apply_staged/checked flush，再持久Complete，之后才能产生Applied。`completed:695–750` 从fresh Context.version得到版本已涵盖时只发AlreadyHaveEquivalent，不把它当特定body Applied。Context.version经load_pile/load_cached/kept_copy新建D::open，缓存是由磁盘fingerprint建立的不可变snapshot，不是其它Loaded的共享可变doc；本审未发现因该cache本身把未flush内存当on_disk。真正postflush/prereceipt中断及fresh reopen仍须针对性验证。
- 完成proof核base/target/epoch及绑定角色；receiver接exact proof hash ACK后才Cleanup精确stage/receipt；senderproof匹配exact outgoing后Accepted→删exact spool→NeedAck，重复Confirm只回该manifest ACK，不删别的generation。晚Write在无record时Missing、不重建；晚Exchange可重建Equivalent并以Reconfirm换proof hash，旧ACK不删除新id。该行为源码与四个State测试oracle一致，尚未运行。
- Journal固定3controls、32MiB image/64各向/192KiB record、checksum/generation、private next+sync+rename，完整next语义校验后恢复，partial prefix保旧；未知/坏complete/links保留。它是process restart publication，不是整机断电/Windows runtime证明。没有扩展为所有本地FS/同uid竞态审计。
- 初始quiet为一次合并Versions；大的doc版本用Deferred再小页，continuation要求names完整、不重复且每页至少一项退役；空页/零进度报错。每doc失败在work先标记，独立doc继续，按registry/selected顺序聚合首错；Pending与Apart明确，reading个别失败best effort。特殊invalid continuation是整轮协议错误，不当普通doc unchanged。metadata损坏需reconcile，并不宣称它能自动隔离一切磁盘损坏。

## 新旧测试oracle与证据固定

外盘candidate实际25文件/3,362,602B，内盘真实路径为 `/Users/zgh/Library/Caches/khor-code-quality/recovery/node-transfer-n1-candidate-01`（此候选未带Q039层），逐文件SHA相同。13current源=冻结全副本；1083 selected source SHA逐项核当前相同，不把选择库存当coverage。source_manifest按sorted compact JSON重算为 `b7b4bbf980b791ca79b4a4ce2795d3465f664eef8dd9be130ce47f4bd6071ca3`；complete.patch `d45ebfb8fca668a539842abcbe39519d5ac498d7a89f2806b4e47c6c65ed1f0b`吻合。

独立从audit `dd60fc5d` 实际提取并比较9完整函数：N0普通父/exercise_convergence/history/next/inventory/worker/两inventory父/epoch父逐字不变；Pair04整源仍SHA `e7dbda3c9c74c757f31663ea82d6d4f16dd1ae3399f29efd4fb0c24f1a512e4f`。旧oversize exercise仅按已授权3次合法history迁到真实>64MiB前置，并要求typed当前resource-limit特征；Seen123/Pintrue、no follow/目标缺失/epoch/inventory核心断言保留，epoch summary加强exact Apart。它不是继续要求旧4MiB错误以阻止正确新协议。

11个新普通父=4 State真实PinDoc import/并发本地pin/reopen/GC/fixed target/late op/reconfirm/role（4父）、3 journal prefix/坏complete/Unix link（3父）、3 actual encode/chunk/重试非early/budget（3父）、1Pending/mixed/first-error summary（1父）。没有新增ignored计数；这些源码都未运行。C1对应wire最大request角色前置需修，不把actualencode测试等同真实Node/对端admit全链。

## 建议下一步与结论

**candidate01先修B1，并校正C1后另冻delta；本片尚不能给无阻断结论。** 不需要新框架或拓宽N2。normal --lib check只证明正常库编译；随后定向11+新增rawrequest边界、既有normalPair/agents/knownhost/inventory/epoch/64MiB失败隔离及原N0真25MiB父，按root唯一槽分别判断。原N0固定VV/receive_dir与Seen/Pin真正收敛才证明本片产品目标；还需核quiet实际1 Versions、bidirectional quantum与一份postflush/prereceipt reopen行为，不能用unit summary或source字面形状替代。巨大Pair04/Vouch/claim N2仍open，不能把未做N2当这片修复阻断，也不能提前称全goal完成。

本次审没有启动任何Cargo/native/gen/Node/relay、没有更改生产/测试源或进程、没有cleanup/commit；没有为其自身Gen实现出独审结论。
