# Q039 — Node 单当前 DocTransfer 与可恢复 Pair bootstrap

## root当前集成检查点（2026-10-27）

作者正常同步检查点已提交d7a4192dfde6e589005031e4a91983beee68d5b3，14源码逐SHA等candidate11、作者tree在交付时clean。root将原dd60回归与此实现无冲突cherry-pick为695fa455/c13a5c444ed94b81bc14432679e0486a8f5b3dee，库5f2与已集成686f同变更，未重复引入。

实际source11完整17 distinct各1/0/0，随后原N0正常Node/integration也通过：raw gap25,447,950B、epoch1/1，原最多4次visit不改；首visit Seen123/Pintrue，第四visit固定VV覆盖且业务内容匹配，cap_rejections=0。root逐raw/final独审；A250dual、B49dual、源/真实artifact/组/TMP门通过，worker不额外算业务父。失败前真实I/O收费、partial Pending与无业务进展时停止Busy重发均已回归。

当前c13组合静态核：R1非共同23路径、N1非共同9路径精确保留作者源码，catalog为两方完整键值并集，Q05212路径仍精确source04；共同lib/link/proto需后续集成native/App验证。main/发布版未动。

此检查点只完成普通大文档同步接入，不关闭全部Q039：真实32MiB优化构建网络边界、中途SIGKILL、N2大Pair/Vouch仍待。SIGKILL候选13仅测试hook/模块、1085输入/12dual已交独审，未编验；大卡已由作者交root独占，以下为历史与原始证据。

状态：working（N0测试准备已冻结，等独立源审/首红槽）；root已确认单DocTransfer/1MiB、完整Preparing、receiving-only Write、proof/ack/GC与保持原trust方向。负责人 `node_transfer_owner`；独立审查者由 root 指派。本轮仅设计/实际源码核对及本卡，无生产修改、Cargo、生成器、提交、main 合并或 ship。

## 任务契约与现场

- 完成目标：删除 Node 的旧 `Request::Sync` / `Response::Synced` / `Peer::rounds` base64 通路；正常同步和 Pair devices bootstrap 均只用同一 DocTransfer。真实 25,448,146B 合法历史在独立 Node+relay fixture 中最终收敛，同时 Seen/Pin 公平推进；保认证、epoch、并发、本地 pending、pen、flush 与既有数据。
- 用户决定以中央 STATE/PLAN 为准：未上线，不保旧 Khor fallback；全仓治理、多 agent 已授权；没有 main/ship 授权。下文数值/新 API 是实现建议，未经 root 确认不视为已实现契约。
- 权威卡：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan/docs/code-quality/tasks/Q-039-node-transfer.md`。只写本卡，不写 STATE/其他卡。
- 独占已有树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，`fix/quality-node-doc-transfer`，HEAD **5da91f0332634d9415ba8641aaa5a1e166258000**，接手与读源后 tracked clean。保留旧 init419 / Doc apply6e refs，不覆盖 native-io 库作者候选。
- 已读中央 STATE、PLAN、current-protocol/descriptor/outbound/receive 卡。旧卡阶段用 STATE+实际源校准。recovery 树已有 core/init、strict apply、descriptor、outbound；receive/version 候选从 native-io 实际源只读核对，未复制或改写。
- graph-first：recovery 无 `.codegraph`，未建索引。code-review-graph HEAD不匹配（built92742250，HEAD5da91f），compound symbol search0，link file summary有结果，trust未覆盖，tests_for歧义；已回落实际源。0/歧义不当覆盖或正确性证明。

## 实际源码确认

| 位置（5da91f） | 事实与影响 |
| --- | --- |
| `link.rs:25–64` | `with_doc!` 已是11 global+合法 `wide/<name>` 分派；`ROUND_DOCS` 是首错/访问顺序。复用它，不手写另一张 doc 表。 |
| `link.rs:10336–10900` | 只有当前 Versions admission失败可一次Vouch；`sync_document`→`rounds_from`仍传 `&mut Loaded` 跨 `exchange_on.await`，Peer导出整段base64，未接新库。 |
| `link.rs:3643–3700` | Pair先查票/过期、mutating settle、删票、enroll/flush，再 `table_b64` 放 Paired；大回包失败发生在消费票之后。 |
| `link.rs:9670–9710` | caller整表merge并flush，若本端签了issuer还拿着Loaded跑旧rounds；Pair也须删除旧接线。 |
| `node/trust.rs:658–714` | `settle_with`不是纯检查：Adopt会删旧root key、drop旧rosters、set_anchor/store_roster。仅在它后面写claim不能覆盖崩溃窗口。 |
| `net/trust.rs:216–295` | signed roster自身带u32 body长度、签名/证书校验；无64KiB等业务大小上限，`roster_offer[_in]`也返回完整无界Vec。不得假定roster总能inline。 |
| `proto.rs:1148–1290` | 旧Sync可retry、不可0RTT；Pair不可retry/0RTT。新mutating Transfer/Pair操作须显式更新allowlist，不能继承默认。 |
| `transfer.rs:281–394` | reservations私有、最多64stage/128MiB，receive会在missing创建，cancel精确manifest且Busy保护owner。status/resume不能调用receive猜缺失。 |
| `once.rs` | 现有once锁仅进程内且note会按年龄扫文件，不能直接用作跨进程claim或receipt安全持久层。 |

接手源码 SHA256：link `cbbd59b4b89376d21b92ba5424a90c862864293a292eeac7f5a7e395c975065b`；proto `e0716489b4e9eebb7c88984415be44a4884ba9aa9c1e59b024fc615959225fa3`；node/trust `497b698460c8381cf3d849037ffc309044ea7d46d7098719ef0dc42e5b064a31`；modern fixture实际在 `tests/sync_doc_isolation.rs`，SHA `effbb7aa0a86dd2e2b82b6ceab888c804774bfe1926a89818ce65a5ffa022574`；pairing `3ad82ed506e2691f30f09d07d7f7111eb260500ff635876d91fe82c2a9770c1a`。core `ba7625143caa261f366b5acc88202d6447d5a4effa80dd48d90dc8aae5fa5cd4`、outbound `64480d7a94af53e7f3a45d93270c3f98f947cf4c6825b0b12655308c8dbc444e`、strict apply `df7729da78f17fb15546ffdacbc6eb4bae87cec0889e119d00d5be5bf78b5b29`。

## 建议 wire：同一 body、同一提交路径

保持 msgpack positional schema、binary `ByteBuf`。移除旧 Sync/Synced，而非留兼容分支。`ManifestWire`只搬运core字段，不重做canonical id算法：

```rust
ManifestWire { source:[u8;32], recipient:[u8;32], binding:ByteBuf,
               total_bytes:u64, digest:[u8;32] }
Chunk { manifest:ManifestWire, offset:u64, bytes:ByteBuf }
Access = Admitted | Pair { claim:[u8;32] } | Vouch { operation:[u8;32] }

Request::DocTransfer { access:Access, action:TransferAction }
TransferAction =
  Exchange { document:String, have:ByteBuf, offer:Option<Chunk>,
             proofs:Vec<CompletionProof>, acknowledgements:Vec<ReceiptAck> }
| Write { chunk:Chunk }
| Read { manifest:ManifestWire, offset:u64 }
| Status { manifest:ManifestWire }
| Confirm { proofs:Vec<CompletionProof> } // bounded batch of local receiver proofs
| Acknowledge { receipts:Vec<ReceiptAck> } // same bound; exact peer/manifest

Response::DocTransfer(TransferReply)
TransferReply =
  Exchanged { document:String, have:ByteBuf,
              received:Option<TransferState>, offer:Option<Chunk>,
              proof_acknowledgements:Vec<ReceiptAck>, ack_results:Vec<AckResult> }
| State { manifest:ManifestWire, state:TransferState }
| Chunk { chunk:Chunk }
| Confirmed { acknowledgements:Vec<ReceiptAck> }
| Acknowledged { results:Vec<AckResult> }

TransferState = Missing | Stored { next_offset:u64 }
| VerifiedBlob // Pair/Vouch roster only; never a Doc completion
| Complete(CompletionProof)
| Apart { expected:u32, actual:u32 }
| Failed { kind:TransferFailure, detail:String }
CompletionProof { manifest:ManifestWire, persisted_version:ByteBuf,
                  kind:Applied | AlreadyHaveEquivalent }
ReceiptAck { manifest:ManifestWire, proof_hash:[u8;32] }
AckResult = Released | Absent | Busy | Failed
```

- `have` / `persisted_version`都只用候选 `DocumentVersion::encode/decode`，不继续wire::version_b64。manifest→core.validate/id→Descriptor::from_manifest，所有语义只认这一个canonical binding。Proof hash为域分隔+full manifest id+canonical result version+kind；它是幂等关联值，真实性来自iroh authenticated peer，不伪装数字签名。
- 每次含stage的op带完整有界manifest，不接string id任意文件读取；document、source、recipient、purpose必须一起校验。Write/Status的incoming角色 source=authenticated remote、recipient=self；Read的outgoing角色 source=self、recipient=remote，且必须命中本地持久plan。Confirm是远端receiver对本地outgoing的证明，Acknowledge是远端sender确认本地incoming的receipt。
- Exchange含本端fresh disk have与可选outgoing首块；receiver先处理incoming，再捕获自身fresh have并按调用方have固定反向plan/首块。两方向独立提交，不声称原子双向事务。若incoming失败仍用typed received报告，独立outgoing能继续；whole transport/decode失败才没有reply。
- 小body<=256KiB首块同请求调用真实`apply_staged`、flush、receipt，直接回Complete；无空Begin/每doc独立Commit。反向首块也通过同一个本地接收函数，不写“小docmerge”第二通道。大body最后块也直接进同一个提交函数。
- Confirm只批带本端已持久receipt的proof；对方校验自身outgoing plan并持久Accepted、清body，返回ReceiptAck；本端随后清incoming receipt+stage。最后小pull可整轮一次Confirm，而非每个seen/pin空Commit。ACK/清理路径见下一节，不把Confirm等同Doc apply。
- Proposed Transfer frame独立上限 **1MiB encoded**，每frame至多一个256KiB chunk，manifest沿用core整体64KiB metadata预算、每version<=64KiB；Exchange至多捎1个proof+1个ack，独立Confirm/Acknowledge至多2项且按实际encoded fit切batch。去掉机械192KiB累计control限制，避免合法header永久不能发送。现MAX_FRAME6MiB不提高，旧wire4MiB限制不修改当修复。小Doc优先把proof/ack捎下个Exchange，最后余项才独立Confirm/Acknowledge。
- 编码发送前与解码后都验总长/数组数量/字段大小；收包现通用6MiB outer hard read bound保留，识别Transfer后立刻收紧到1MiB。若要解析前即1MiB，需要另ALPN/length framing，当前不提出这项扩展；不得谎称现outer reader仅分配1MiB。
- `DocumentInventory::Version`改为canonical version bytes；inventory digest也hash同一DocumentVersion编码。**root已决定安静轮仍一次Versions，保docs+reading digests合并首问与首答，不固定多一次reading/docs-only请求。** 仅有大doc/reading内容时按实际encoded预算分页，能放进首答的docs/readings合并首答；后续可docs-only/page读取。每页doc建议最多16，合法单entry必须能放下；cursor仅索引本次有界计划且必须严格推进，零entry不得回原cursor形成无限重试。caller保序完成全部页，bad/truncated cursor明确ProtocolError；readings best-effort不吞doc首错。具体N1 schema按实际Rust encoded边界验证。
- `Exchange/Write/Confirm/Acknowledge/Pair*`均不可0RTT；Read/Status首切片也保守不可0RTT，因为包含权限/claim/状态恢复磁盘工作。只有原Versions保reading early。精确manifest与持久状态令所有Transfer action可重试；错误必须typed，不用Refused文本猜AlreadyApplied/epoch。

### 包预算实算（已核锁定serializer源码，无Rust运行）

锁定本地 `rmp-serde-1.3.1/src/encode.rs:675–760`：unit enum为variant string；带字段enum为单项map+variant string+positional array；struct默认array；Option Some不加wrapper；ByteBuf为bin；默认fixed `[u8;32]`不是bin，最坏array16 header3+32×2=67B。按schema逐项Python整数计算（没有安装依赖/生成制品）：

| 项 | 保守合法长度上界（bytes） |
| --- | ---: |
| ManifestWire（binding<=65536−120） | 65,626 |
| version ByteBuf（raw<=65536） | 65,541 |
| Chunk（full262144B、offset<=64MiB） | 327,781 |
| CompletionProof（最长AlreadyHaveEquivalent） | 131,190 |
| ReceiptAck | 65,694 |
| Exchange request（Pair access、最长合法wide133B、1proof+1ack） | **590,442** |
| Exchanged reply（have+received proof+reverse fullchunk+1ack） | **590,389** |
| 独立Confirm2proof（含Pair access/envelope） | 262,479 |

512KiB=524,288，真实schema的保守边界超它；1MiB余量充分，不能缩短非末canonical chunk去凑512KiB。上表采用各字段独立上界，未声称同一真实Loro文档必能同时取尽所有上限；这是分析编码上界，不冒充actual Rust serde运行。实现阶段新增exact `proto::encode` worst-field边界case与reject-overflow case，确认真实生成schema与此计算一致（不得靠更小字段掩盖溢出）。

### 明确allowlist

| Request | retryable | rides_early | 每请求budget | dispatch授权 |
| --- | --- | --- | --- | --- |
| DocTransfer全部action | true（持久exact状态） | false | 30s | Admitted或exact Pair/Vouch grant；后者仅允许其phase/purpose动作 |
| PairStart/Resume/Commit/Finish | true（同caller/tokenHash/nonce） | false | 30s | 初次token；后续已有同caller claim；不同caller无恢复权 |
| Vouch首帧/进度 | true（同peer+roster manifest） | false | 30s | 当前anchor验证现有cert仅授bounded upload；完整take_vouch才admit |
| Versions | 保当前true | 保当前true | 保当前reading预算 | 只有本op admission失败typed InventoryNotAdmitted触发一次Vouch |

Read只准已有outgoing plan；Write只准已有Receiving；Status不create不apply；Confirm仅outgoing proof；Acknowledge仅incoming receipt。Pair/Vouch grant不能跨这些角色，也不能传给其它Request。这里true是幂等重试，不是0RTT replay许可。

## 本地 sender plan：先固定，再上网

新 `link_doc_transfer.rs` 拥有网络pump；独立 `doc_transfer_state.rs` 拥有Node元数据（不是另一份blob core）。root `.khor/doc-transfer-state/` 与唯一共享blob root分开；Node绝不在core root放plans，也不解析core offsets/body路径。每逻辑key=domain hash(local identity,peer,doc,direction)，每record再包含generation；文件名hash(key,generation)，内容验证full key/generation，短perkey lease管唯一active及terminal关联，避免路径直接拼doc。active/terminal合计计入64record预算，不让每代多一个无限遗留文件。

持久sender状态（每peer/doc唯一active key，含monotonic generation，用cross-process perkey lease同步短临界区）：

```text
Absent -> Preparing{doc,source,recipient,epoch,base,target,encoding}
       -> Ready{manifest} -> Offered{manifest,confirmed_offset_hint}
       -> Accepted{manifest,validated peer proof}
       -> NeedReceiptAck{manifest,proof} -> Done/Absent
```

1. startup先reconcile有界metadata与core retained manifests，未知/坏文件明确错误并保留；不一边扫描一边静默删。同一Loaded同步scope先捕获完整expected Descriptor（doc/source/recipient/epoch/base/target/encoding）、validate，再把Preparing落盘，然后调现prepare；现helper在flush前捕获target并在flush/export后检不变。结果Descriptor须逐项等于Preparing，才写Ready。不得只记base再按最新target猜恢复。`prepare`同步scope内真实flush/export，drop body与Loaded；Ready完整manifest必须落盘后才允许发送任何首块。
2. Active Ready/Offered跨round/restart继续**同一个manifest/body/target**，不因local新op或新inventory换id。confirmed_offset只是hint，每次以远端持久status为准。轮次失败/timeout释放owner保stage。完成旧target后下轮才捕获新target，因此持续本地新op不会饿死旧target。
3. prepare→Ready崩溃可能留无plan spool。持久Preparing +同key排他+core合法manifest枚举将唯一匹配完整Preparing(doc/source/recipient/epoch/base/target/encoding)未被其他record引用的stage收为同Ready；完整body直接resume；未完整且从未进入Ready/Offered的stage可在明确recorded unoffered cleanup步骤后重prepare。0匹配可显式replan尚未发布的Preparing（更新完整目标后再prepare）；多个匹配/active owner/坏/身份/epoch异常严格defer或ReconciliationRequired，保原文件，不猜最新mtime或按摘要覆盖。启动发现孤stage而无Preparing不能静默GC。
4. 收到Complete只在authenticated peer=plan recipient、fullmanifest=plan、result epoch=descriptor epoch、version覆盖base和target时接受。先持久Accepted，再drop owner、core.cancel exact outbound body；清失败保持Accepted retry。Stored/Incomplete/Absent/对端Refused均不清body，不提升成功。
5. body清完持久NeedReceiptAck，向receiver重发ReceiptAck直至Released/Absent；只有此前已持久有效proof的plan可把Absent当cleanup确认。新target可在oldterminal小record存在时开始（独立generation），terminal record不能清/覆盖新active。
6. Apart/StaleBase/ResourceLimit保原plan/body并显式报告；不自动扔old history重建新id。若要废弃过期epoch，需要单独root确认的exact cancellation/reconciliation策略；当前不把失败当有效completion清理。

## receiver receipt、重试与双向清理

每incoming fullmanifest在创建stage前先持久Receiving record，短pertransfer lease串行Write/commit/status/ack；blob owner只在一个同步步骤内存在。Receipt严格绑定fullmanifest、document、source authenticated peer、recipient self、descriptor epoch与canonical resulting VV。

```text
Receiving{manifest} -> Verified(stage)
 -> ImportIntent{manifest,before_disk_vv}
 -> apply_staged + actual FlushedImport
 -> AppliedReceipt{manifest,version,notification_state}
 -> ReceiptAckObserved -> Cleanup{manifest} -> Absent
```

- Write只接受已持久Receiving record的exactmanifest，调用open_existing而非receive；Unknown/Missing Write不创建stage。只有授权Exchange首块或Pair/Vouch首帧可先写Receiving再receive。每append返回core Stored；VerifiedComplete只是字节状态。complete后新load同doc，重验epoch/base，真实apply_staged；成功之后必须写可进程重启恢复AppliedReceipt，才回kind=Applied。无receipt写成功就不能报Applied；失败保stage/record，返回typedCommit/ReceiptIo。Loaded、fullVec、sink全部在网络await前drop。
- fresh disk load已覆盖target且同epoch时可记录并回 **AlreadyHaveEquivalent**，不宣称此digest被导入；可以在receipt写失败/进程在flush后崩溃的恢复中使用。只有fresh disk载入证明，不读尚未flush alias的VV，不拿partial Loro metadata当full target。
- Status先验auth/purpose/doc/epoch，再找exact receipt；有receipt仍核fresh epoch与持久VV，已换epoch回Apart，不重放旧Applied。无receipt但freshVV覆盖base∨target可回Equivalent；否则open_existing返回Missing/Stored/Verified，Missing绝不创建stage。单纯Status不自动导入Verified；重发最后Write可驱动同commit，也可新增明确RetryApply action（待root取舍，不能读状态悄悄变成apply）。
- 最后reply丢失：sender重发相同块/Status，有receipt回同proof，无receipt但持久目标在则Equivalent。旧重复完整首块先做fresh equivalent检查，不重新建stage或执行用户业务。没有满足target时仍需完整校验导入，不把相同长度或digest单独当Applied。
- sender receipt ack只可清**本incoming**精确manifest；先持久ReceiptAckObserved/Cleanup，再dropowner、core.cancel，再删receipt/record。ack丢失可重试；record已无且core也无回Absent，不需要无界tombstone。迟到old Write因Receiving已删返回Missing且绝不建stage；重复Exchange先freshVV判断Equivalent，无stage重建；epoch变了则Apart。cleanup与新id不共享路径，不误删。
- 对反向pull，本端先存AppliedReceipt；在Confirm中把proof交对方。对方同样先Accepted并清outbound，回ReceiptAck；本端再执行同ack cleanup。Confirm reply丢失，对方保NeedReceiptAck可重发/再次响应相同Confirm，最终双方都清；不因outbound body已删就忘掉最后proof。
- stage/receipt故障、postflush prereceipt崩溃、ack清理中断都能以record+freshVV收敛。任何IO失败不能宣称回滚了用户Doc；stage不自动删。也不承诺跨平台整机断电事务，当前store flush只支持已定义process-restart边界。
- 业务effect以真实fresh before/after VV推进为触发依据，receipt记录effect pending/done；重复Applied/status不再次触发。`follow_after_sync`已有setting/receive_for幂等判定，broadcast take已按message/fate/job join。crash在effect后marker前必须靠这些业务id幂等恢复，不能凭marker宣称任意副作用exactly-once。需要真实重复/restart测试验证无重复落地；若existing effect不满足，先补其idempotency，不“先标done后执行”丢effect。

元数据建议常量：最多64 active/terminal sender records+64 incoming receipts/receiving records（每类单record<=192KiB，单record含VV预算后精确encode验长），Pair claims<=16，所有metadata累计<=32MiB；scan读第N+1项立即ResourceLimit，单file超长/非private/symlink/unknown entry明确ReconciliationRequired，不跟随、不删。core仍是**同一个**64stage/128MiB/2owner额度；多个TransferStore::open同root不是额外配额。写record必须private、no-follow、create_new temp、sync、atomic replace、strict bounded decode，跨进程锁与短同步scope；不复制core的stage规则，不直接复用once的宽松文件读取/按龄扫。active/未ack receipt不TTL删除。完成且双方ack的record可删；容量耗尽明确受限，不能无限扫描或默默剥除未确认历史。

## Node 加载/flush/await 与公平调度

- 将现with_doc/ROUND_DOCS移到可共享的registry位置或保持父模块macro并只加同步adapter `capture_version(name)` / `prepare(name,spec)` / `apply(name,sink)`；三个adapter都委托同一with_doc。合法wide用现`chat::wide_dir`/valid_channel，不把任意自称doc名当已授权；inventory按既有known_wide计划选择。
- dispatch先正常admit；Pair Access只走下一节exact claim allowlist。auth失败先于任何stage创建、receipt查询、Doc load。三个wire角色分别验证，不接受“source/recipient是这两人就行”的无方向比较。
- 每次网络step顺序：同步acquire状态lease→fresh doc或spool读取/写入→persistrecord→复制owned boundedwire值→drop Loaded/BlockStore/pile fence/sink/Outbound owner/statelease→await exchange→验证reply→下一同步step。不把泛型Loaded塞进asyncfn参数继续持有，也不把owner寄存在perdoc pump对象里。
- heavy export/import全节点最多1个同步work；不可在Tokio reactor长时间做Loro/FS；用bounded blocking admission，队列至多active doc数/metadata额度，取消等待不会留无界spawn_blocking排队。核心2owner允许短in/out步骤，不能在await时占满导致cleanup Busy。
- pump按ROUND_DOCS后sorted known_wide循环，每doc每quantum最多4个body chunks/**合计1MiB**（双向合计，避免各1MiB实际2MiB），首块计入；先遍历一次所有选中doc，再下一quantum。一次request只出/入至多一chunk，双向Exchange最多2chunk总量计入同quantum。
- visit上限建议32MiB实际body或30s合作边界，先触者返回明确Pending并保存offset；不能中断进行中的checked import后假装回滚。25MB目标一visit若超限就多次真实sync_now至固定target完成；不能要求单轮全历史一定完成。普通控制stream保留原并发；无进度30s结束自有step，保stage。
- 每doc独立Outcome=UpToDate/Applied/Equivalent/Pending/Apart/Failed。聚合首错按固定doc顺序而非完成时间；readings失败不覆盖doc首错。Pending/Apart必须给部分完成文本，不能nothing_to_move。目标未应用不follow devices；broadcast仅实际新增持久op触发。完成旧target仍可能有新op，显示本visit进度而非永远声称完全最新。

## Pair：有界头 + 同Transfer的恢复claim

Pair不是保留旧bootstrap特例。新增 `link_pairing.rs` 使用同ManifestWire/Chunk/core/store/state helpers；TrustRoster是同blob传输的另一purpose，**不伪造Doc VV**。其binding为独立domain+claim id+role+双方identity；body仍有长度/digest，收齐后真实claimed_root→verify_roster验证全部签名/证书，再pure nt::settle。接入Doc时仍用唯一DocTransfer apply_staged。

```rust
PairStart { token:String, nonce:[u8;32], name:String, addrs:Vec<String>,
            devices_have:ByteBuf, roster:Option<Chunk> } // roster首块可inline，非整个任意roster
PairResume { claim:[u8;32], nonce:[u8;32] }
PairCommit { claim:[u8;32] } // roster齐且verify/decision后；可重试
PairFinish { claim:[u8;32], proofs:Vec<CompletionProof> }
PairReply { claim:[u8;32], phase:PairPhase, peer_name:String,
            devices_have:ByteBuf, roster:Option<Chunk>, devices:Option<Chunk> }
```

Pair的name/addrs/nonce/token等scalar头累计<=16KiB（不含各自受64KiB约束的devices_have/manifest）（name<=256B，addrs<=64项、每项<=256B，实际累计再验），每frame仍1MiB。roster与devices body都<=64MiB，满足共享128MiB总额；过限在burn前或明确可恢复阶段报ResourceLimit，不组装大Paired响应。两份roster+devices若超过共享quota，按阶段ack清已完成TrustRoster blob后才准备devices，不能给Pair另开一份quota。

具体时序：

1. caller在本地持久LocalPairIntent（issuer identity、tokenHash、nonce、caller roster固定manifest/base）。不在协作/日志放raw token、roster签名或私钥；磁盘只需tokenHash，不持久raw ticket密码。retry同ticket复用nonce，不能每次随机变op。claim id可domainhash(issuer,caller,tokenHash,nonce)，双方可在首reply丢失前算出。
2. issuer按tokenHash跨进程短lease：若已有claim，只准同authenticated caller+nonce+header roster manifest恢复；其他身份拒绝。没有claim才查现invite freshness，先保存ClaimReceiving+原mint instant，再收caller roster；此时不burn、不settle mutation。坏签名/两网络Clash在burn前拒绝，可在安全释放此claim/stage后继续保有效原票供重试，不把Clash消费成永久票锁。
3. roster收齐后，持devices/trust同步fence重新取本端roster并pure verify/decision，记录SettlementIntent：旧/新public anchor、已验证roster fullmanifest、预期oldroot public指纹、双方identity、决定、phase。必须把现settle_with拆成纯decision与幂等apply；不能在前面仍调用会删key的settle_with。并发root/roster变更必须重验或报Changed；不靠旧roster决定覆盖新网络。
4. 小roster在PairStart首帧已complete时可直接继续步骤4并回同PairReply；大roster最后Write只回VerifiedBlob，由可重试PairCommit执行步骤4，不给roster伪造DocumentProof。SettlementIntent落盘后才burn invite（不存在但同claim Intent已记录视为可恢复burn，其他claim不能复用）。随后按journal step幂等settlement/enroll/flush，最后固定issuer devices outbound plan。`give_way_to`重排为先持久意图/desired anchor和可恢复roster，再完成Doc/flush，最后仅在旧public指纹仍匹配且无新non-lone冲突时退役旧key；不把key bytes写intent、不删除后来新root。Node::open启动需要先reconcile该intent再执行root bootstrap，避免看到半settlement另mint根。具体root bootstrap调用面需实现阶段再graph+source复核。
5. `AdmissionFlushed`后返回有界PairReply+issuer已签roster首块+同DocTransfer devices首块。PairReply至多一个body首块：roster未交完先给roster，下一PairResume再给devices（不能两份256KiB同时无预算塞入）。devices manifest base来自PairStart的fresh disk `devices_have`，反向base来自PairReply的fresh `devices_have`，epoch不一致明确Apart而不自动reset。claim grant只允许这次claim精确roster manifests、devices manifests（限定双方source/recipient）及status/confirm/ack；未admitted caller不能拿grant读其它Doc/FetchPath/任意blob。root持有者不在issuer时，保留原pending/enroll语义，不能靠claim放开一般admit。
6. caller收齐并验证issuer signed roster，先写自身SettlementIntent并同样恢复apply，随后真实checked devices import+receipt；若caller是root签issuer/自身，flush后用同Transfer固定反向devices plan并推进，不再best-effort旧两rounds持Loaded。issuer与caller均按normal proof/ack收口。
7. PairFinish只在所需双向Doc目标已persisted且roster/root工作完成时标BootstrapComplete。最后reply丢失，PairResume同caller返回同claim phase/proof，不新enroll新row/新签seq。完成claim至少保留24h且超过原invite window后才可删，invite原票已burn所以别的caller仍不能复用；local completed intent让caller自身重启不因最后completion reply丢失重新从头配对。未完成claim不自动TTL删，数量满明确受限并可人工exact reconcile。
8. 取消只停止本claim blob worker/未提交stage，已burn/admitted/root settlement不能宣称回滚。partial state明确恢复，不删除设备并发row或他人roster。issuer进程在burn、anchor、enroll、flush、spool、reply各窗口重启均由同claim推进；未知/坏intent保留并拒绝自动mutation。

TrustRoster的VerifiedBlob只证明stage完整，不能触发Doc成功/清sender body。Pair双方roster sender只有收到authenticated同claim `phase>=AdmissionFlushed`（其fullmanifest在persisted claim中且相应roster已进devices flush）才能释放该outgoing roster spool；receiver也只有在settlement intent完成、roster可从user devices持久重载且精确digest匹配后才能释放incoming roster stage。PairResume重复返回该phase+固定roster manifest引用，释放是按claim的明确步骤，不拿DocProof伪造无VV roster；未完成intent pin住stage，即使收到通用ack也回Busy/Deferred。完整Doc devices方向仍严格用DocumentProof+receipt/ack，和roster语义不得互换。

Vouch也必须消除任意inline roster黑洞。实际`take_vouch`在当前anchor下verify整roster，核stale、同时admit双方、seq不倒退、remote未本地gone，再store_roster/upsert/certs/flush；这些规则原样保留。建议将Vouch头改为：

```rust
Request::Vouch { name:String, addrs:Vec<String>, certificate:ByteBuf,
                 roster:Chunk } // first canonical chunk, full roster manifest
Response::VouchProgress { operation:[u8;32], state:Stored|VerifiedBlob|Vouched }
```

- 对陌生caller，头的certificate仅用既有 `nt::verify_cert(current_anchor)` 核真实签名、subject=authenticated remote、live_at，并核现本地gone；cert<=1KiB，name/addrs沿Pair头界。现Cert固定字段预算须actual encode test确认1KiB充分（不是新crypto/新授权证书）。不额外限制Kind或因旧本地roster曾revoked就提前拒绝一个可能更新且已重新admit的roster；原take_vouch接受规则仍由整roster的seq/撤销/双方admit决定。旧撤销cert最多获得这一个有界上传机会，不获得trust或读权限。没有anchor/validcert仍拒绝，不用claimed_root自签授予陌生人权限。
- 这一验证仅给同peer同operation同fullmanifest的**单向roster upload/status** grant，不能Read任意blob、不能读取devices、不能进入一般DocTransfer。operation=domain hash(local,remote,roster manifest id)，metadata与core额度共享，先存Receiving；后续Write只认已存在exactrecord。不得允许Vouch Access随意提供另一manifest。
- 收齐body后真实完整`take_vouch`按上述所有既有条件重新验证，flush后持久Vouched operation receipt才回Vouched；验证失败不admit。小roster首帧可同请求Vouched，大roster最后Write同提交，不加空Begin/Commit。lostreply可查同operation persistedreceipt，或正常Versions已重新admit；sender仅Vouched后清rosteroutgoing，receiver在确认或恢复freshdevices已含exactroster后清incoming。
- 首次InventoryNotAdmitted仍只触发一次同当前inventory的Vouch修复链：该一次可含若干chunk，不是失败后无限vouch；未知op/一般Refused不触发。认证被恢复前，Vouch grant无其它文件/Doc读取权。若巨大roster本体>64MiB，typed受限并保持原admission/票据规则，不能用无界inline兜底。

raw roster在Loro/devices内本来作为证书集合存在；分块只约束网络和accepted blob体积，不保证verify/export峰值RSS。Pair/Vouch roster单体>64MiB明确拒绝；不是任意大网络成员数保证。

## 最小依赖、允许文件与分片

库作者在native-io负责现7源receive/version candidate；不在Node重写该实现。Node需要root协调新增最小库API：

```rust
TransferStore::open_existing(&self, expected:&Manifest) -> Result<Option<Sink>>;
// same gate/private/owner/recover; missing must return None, never mkdir/reserve.
TransferStore::retained_manifests(&self) -> Result<Vec<Manifest>>;
// existing reservations under gate; <=64, no create/delete/read body.
doc_transfer::resume(store:&TransferStore, expected:&Manifest)
    -> Result<Option<Outbound>, OutboundError>;
// canonical descriptor + existing-only + VerifiedComplete, original target/body.
```

最后一个可省成Outbound::from_existing_verified的等价精确接口，避免Node复制canonical chunk short-read循环。ManifestWire仅DTO→core Manifest+validate，不需公开core私有binarycodec。不为了Node给core加入业务doc/Pair知识。

建议依赖顺序：

| 切片 | 允许修改（待root授予） | 独立完成物 |
| --- | --- | --- |
| N0真实red | `crates/node/tests/sync_doc_isolation.rs`，必要最小独立test helper | 保原4case，新增convergence oracle实际红；Pair另最小真实large bootstrap红，不混实现。 |
| L existing-only | 由库作者改`sync/transfer.rs`、outbound helper及定向tests | missing不创建、existing resume immutable、bounded enumeration、ownerBusy/坏file；先独审再交root接依赖。 |
| N1 state/wire | 新`node/src/doc_transfer_state.rs`、`link_doc_transfer.rs`，最小`lib.rs`模块声明、`proto.rs`类型/allowlist | 有界codec/record/owner与真实process restart测试；尚不替换旧Node arm前不计Node收敛。 |
| N2 adoption | `link.rs`最小registry/dispatch/inventory/pump改接、新module；新Node fixture | 删除Sync/Synced/rounds路径，真实25MB+fairness+proof/ack/restart绿；wire旧helper若仅tests用再另清，不大面积格式重写11k行。 |
| N3 Pair/Vouch | 新`link_pairing.rs`、`node/trust.rs`纯decision/intent apply、必要Node startup恢复接点、Pair fixture | token claim/root multi-file恢复、同Transfer bootstrap与bounded Vouch grant；删除table_b64大Paired与Pair旧rounds/任意inline Vouch roster。 |

proto/catalog用户文本与接口生成影响须先登记；不无意改公开IPC/GUI signature。已有sync_now仍perpeer Result<String,String>时明确Pending/Apart文本可先不扩GUI类型。新生产文件每片源冻后独审，再唯一Cargo槽运行，最后提交exact tested source；不一次性塞巨量实现等待全批结束。

## 可执行真实 fixture 与旧oracle变更

现`sync_doc_isolation`有4普通父（oversize、epoch、inventory-local、inventory-remote），每父实际spawn专用worker，真relay+Node public pair/sync_now，process级deadline/私有HOME/Node nonhosting，source `large_address_history`4096×64地址、恢复真实地址、不造额外机器。setup pair失败/relay失败/compile失败/cleanup失败不算产品red。

N0按root授权先只加测试（未运行）：

1. 新父 `a_large_devices_gap_converges_through_real_node_transfer` 复用同合法fixture，要求raw_gap=原固定25,448,146B（若库编码变化先明确记录实际size且仍>24MiB，不能削小fixture），调用真实client.sync_now有限visit，receiver fresh reopen VV covers固定source target，业务devices设置完整；同round Seen123/Pintrue仍真实推进。当前5da91f应在cap拒绝/target未覆盖处红。用户follow设置在convergence case应允许成功后follow；不能沿用“永不follow”的失败oracle造成伪红。
2. 原oversize父在red阶段原样保留，是现行为基线而非期望永久失败。接N2后把它明确改名/替换为`large_transfer_yields_and_seen_and_pins_sync_before_it_finishes`：阻断devices目标最后提交/将单visit量子限制在test harness已有真实数据路径，使首visit明确Pending、Seen/Pin在devices未全到前落盘，随后解除故障多visit最终覆盖target。原cap-error字符串断言删除理由是生产路径已由chunk取代，不把旧case“转绿”冒充未变oracle。
3. epoch case保原metadata改EPOCH+1、零merge、Seen/Pin推进、devices不follow，新增明确Apart描述而非nothing_to_move；inventory-local/remote两case保原真实目录→普通file错误、具体Err及Seen/Pin，不弱化成anyErr。
4. sender late-local-op+重启：第一次固定plan id/target后加入新op、kill owned sender worker并restart同root；同id target先完成，再下一id包含新op。检查stage offset真推进、无Loaded/owner跨await（网络扣reply期间同root另process可load并flush、cleanup不Busy），有限quantum中Seen/Pin先达。
5. loss matrix：扣最后Write reply→真实receiver process restart→Status exactApplied或Equivalent；postflush/pre-receipt kill只能Equivalent、不伪称Applied；receipt持久后kill可exactApplied；sender Accepted前后kill、receiver ACK cleanup前后kill，最终双方body+record清而Doc不退。late old chunk/ack不删新id，不重复业务落地。
6. auth/body/IO：陌生peer、wrongsource/recipient/doc、wrongmanifest、future/duplicate offset冲突、corrupt body、strictPending、epoch切换、真实receipt path IO失败。测试真实dispatch与库，而不是在fixture里复制状态机；失败不能回Complete、保stage可重试，其它doc推进。
7. Pair真red：issuer邀请后才构造25MB devices历史，caller真实pair；断言成功并fresh reopen双方target/root/rows。当前应issuer已删票/写row但frame失败，记录明确触发点。随后新增burn后丢reply+重启同caller同nonce恢复；其他caller相同token拒绝；bad签名/Clash保持原token；settlement各journal窗口kill恢复（不写key/签名到日志）；超inline roster用真实signed多cert roster（受64MiB）走同chunk，不塞无效随机字节。

N0计划命令（仅root授唯一槽后）：`cargo test --locked --offline -j1 -p khor-node --test sync_doc_isolation -- --nocapture --test-threads=1`；exact新增case先取red，然后normalNode check及原3非oversize/新的公平case+新matrix定向；Pair新isolated process fixture单独exact。最终补现`pairing`、`table_lost`、`broadcast`、current proto/epoch/auth相关真实覆盖；不把ignored worker另计普通父，不盲跑全仓重复计数。

每run显式quality-target、incremental0、短owned TMP、jobs1、locked/offline；阶段240s/总420s等用独立常量AST校验，真实Node父自身processdeadline<=210s可保。外内盘每次>=5GiB、3I/O/ownedPGID guard、actual compile/source-before-after/binary receipts、scratch cleanup记录。大型matrix应拆父/分run，不能让outer420s与子多个210s互相误杀。证据统一`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q039/node-transfer-*`与internal recovery同名；本设计阶段未创建伪运行证据/未占Cargo槽。

## 最新交接

- 接手/主要读源基线5da91f；收尾重新核对root已将同一Node树推进至 **67927fcfacb84fe260553e31614d8c1e2fd1cd5c**，branch `fix/quality-node-doc-transfer` 仍clean。本作者未执行git/source mutation。新HEAD不是本作者新编译/运行证明；旧接线调查SHA仍明确归5da91f。本次唯一写入为中央本卡，无自有Cargo/native/gen/Node/relay PID、端口或scratch。
- 未实现wire/state/Pair，未跑red/green，不声称25MB已收敛。库receive的22绿只属于其冻结源与库契约。
- root最新回覆已原则认可existing-only/list/resume3 API、Pair纯decision+replay方向，并要求完整Preparing、真实最坏包预算、lateWrite不创建、核Vouch；本卡已依此更新。具体字段/实现仍待确认。
- root待定：上述1MiB Transfer cap/actual编码界、inventory分页/quiet成本、最小3个库API、Status只读+最后Write重试提交、metadata/claim数量与Pair完成claim24h窗口、N0→L→N1/N2→N3授写分片。
- 下一动作：root/非作者审N0冻结源并授首红槽；本作者不改生产或Cargo。库existing-only依赖在另一树独占验证；R1作者独占link.rs三个await/startup/ticker，本作者禁碰link.rs。旧core/Doc/outbound refs全部保留，不按历史卡覆盖。


## N0 收敛测试准备 — candidate01（未运行）

root只授`crates/node/tests/sync_doc_isolation.rs`测试源，本作者在67927fcf上恰一源dirty，无生产/库/link/proto/Cargo/lock/生成器修改，未编译、未typecheck、未运行red/green。唯一Cargo49槽属于库existing-only作者，本任务未使用。

- 新普通父`a_large_devices_gap_converges_through_real_node_transfer`调用现run_worker的新增`convergence`case；同一ignored worker只增case白名单与dispatch，ignore文字去掉过时four。独立`exercise_convergence`复用原large_address_history、Serving、hermetic relay与public Node::pair/sync_now，不直接RPC拼接或导入history替代Node。
- pair成功且双方真实2设备之后，原4096×64合法地址历史、恢复实际地址、再写owned scratch下receive_dir；真实flush后捕获固定sourceVV/name/addrs/epoch与actual raw gap。硬断言原receiver不含target、gap>旧4MiB、gap>24MiB且<=64MiB。路径长度可能使raw gap与历史25,448,146B例稍异，实际运行必须记录，不减历史凑数。
- 最多4visit，每visit Node public sync_now强制访问同一真实receiver；Seen123/Pintrue须首visit已持久且每次仍成立。fresh reopen devices核同epoch、2rows、VV covers固定target及receive_dir/name/addrs内容；只有目标与内容都满足且无harderror才绿。当前生产的exact old cap错误可累计诊断，最终target/content absent断言应红；无关dial/transport错误在单独assert失败，不能把它归该产品red。
- 完整actual resident先stop/join，再执行最终业务断言；父实际worker marker/status与210s processdeadline保留，worker180s总限保留。Worker drop kill/wait和owned scratch清理仍现机制。未额外引入用户服务/provider/HOME访问。
- 对candidate作精确inverse edit：移除独立新父/new exercise、还原worker三处文字/route，得到原baseline文件**逐字相同**；因此原4父、exercise/exercise_inventory、旧cap/epoch/首错/Seen/Pin断言及shared fixture原字节未变。不是仅比较测试数。

冻结source SHA **c1f655df9d0da4e275632a4e74caaadd59fc9f535755d417f20022c2640b9987**，baseline SHA effbb7aa0a86dd2e2b82b6ceab888c804774bfe1926a89818ce65a5ffa022574。`rustfmt --edition 2024`与`git diff --check`通过；仅语法/格式，不当编译或行为绿。

双盘逐字相同证据：external `Q039/node-transfer-n0-candidate-01` 与 internal recovery `node-transfer-n0-candidate-01`，含source-before/source、complete.patch、baseline/candidate.json、selected-inputs.json。selected inputs **444**为明示选择的全部tracked non-vendor Rust + Cargo.toml/Cargo.lock SHA（不是完整frontend/generated构建输入保证）；首红前root runner另做精确所有实际build输入冻结/重编/binary receipts。最初freeze前双盘>=5GiB核通过，未创建运行scratch/PID。

计划首红exact父：`cargo test --locked --offline -j1 -p khor-node --test sync_doc_isolation a_large_devices_gap_converges_through_real_node_transfer -- --exact --nocapture --test-threads=1`。只在独立源审与root授槽后执行；本卡此时没有任何新测试结果。Pair N0b未写，其最小scope/oracle另先提给root，不混本父。


## N0 candidate02 与独立 N0b candidate01（仍未运行）

root对candidate01逐diff审提出两项新case oracle修正，已完成：最终只要求固定receive_dir、fresh VV覆盖固定target、同epoch/2rows；name/addrs准备阶段仍断言恢复真实hints，最终只诊断打印，允许合法握手/并发地址更新超出target。旧cap错误分类改为`khor_catalog::read(why)`的`msg.outbound-too-big` key，不绑定首次raw_gap的floor KiB精确文案。没有改原4case或sharedhelper，inverse edit再次逐字恢复baseline effbb7aa。

- 当前N0源SHA **76495a0b1a47c92abbd58996a94f3710836a961b1c4ba4d9b080c3f88ab0d4fa**，candidate01 c1f655…保为历史，未覆盖。
- 新冻结external `Q039/node-transfer-n0-candidate-02` 与internal recovery同名；source-before/source/complete.patch/candidate及445 selected Rust/Cargo输入一致。所有输入含新N0b文件，明确不是frontend/generated完整build输入保证。
- root另仅授新`crates/node/tests/pair_bootstrap_large.rs`。新文件有1普通父`a_large_pair_bootstrap_converges_and_the_same_caller_can_retry`+1真正由父spawn的ignoredworker，不修改pairing.rs、util或旧fixture。文件内large_address_history函数体与原sync fixture逐字相同，实际4096×64地址history+恢复真实hints；分隔测试文件故保局部同样算法，不抽取改已冻结原helper。
- N0b：真实nonhostingissuer+hermeticrelay，fresh invite后才写自己唯一device id的大history和owned issuer receive-dir，flush固定VV/epoch；真实snapshot硬断言>24MiB、>MAX_FRAME且<=64MiB。caller最初不含该target；第一次public pair后检查issuer已入caller row、ticket已消费，避免把bind/dial/setup错误当烧票业务红。raw token仅在私有fixture中定位ownedinvite路径，不日志ticket/token、签名或私钥；错误只输出catalog key/布尔诊断。
- 同一caller同一ticket第二次public pair必须成功完成/幂等完成；可由持久completed Join intent+caller仍在issuer网络返回，不是重复授予已消费票据权限。另一新identity同ticket必须拒绝且自己1row/root不变、issuer仍2rows无第三人。最终caller freshpile核固定VV覆盖、epoch/2rows和issuer receive-dir，重新Node打开核与issuer初始verifiedroot一致、caller不再持自己的lone root且trusted。实际resident先stop，再报最终retry/目标/root业务断言。
- 当前N0b SHA **14a6f2e6c144e4f4e02ea0864b07030db1f430916261081c5a7a9abea34cd931**。external `Q039/pair-bootstrap-n0b-candidate-01` 与internal同名逐字一致，完整untracked新文件patch用`git diff --no-index /dev/null`生成，未把普通git diff忽略的新源漏掉。
- 两组当前共同基线67927fcf，恰原N0测试一源dirty+N0b一新源，生产link/proto保持禁改。rustfmt check和diff-check通过，**未Cargo/未编译/未typecheck/未执行任何red或green**。每组snapshot有445明示选择Rust/Cargo输入、原4case inverse证明、history函数体一致证明、dual-disk字节核对；freeze前内外盘>=5GiB核通过，无自有运行PID/端口/scratch。

N0与N0b必须分别exact执行/分别归因，不以其中一个父失败代表另一个产品red。N0b计划命令（待独审/root授槽）：`cargo test --locked --offline -j1 -p khor-node --test pair_bootstrap_large a_large_pair_bootstrap_converges_and_the_same_caller_can_retry -- --exact --nocapture --test-threads=1`。其scratch env为`KHOR_NODE_PAIR_BOOTSTRAP_SCRATCH`，同180sworker/210s父process护栏。N0b没有扣reply/kill故障，也不覆盖leave/revoked/root变更后旧completedclaim不能复活；这些真实窗口仍明确N3补。当前尚未宣称“Pair烧票真红”。


## N0b candidate02 — 独立审 B1/B2 最小修订（未运行）

已读独立`Q-039-node-n0-review.md`并按root授权实际修订，仅N0b新文件：

- B1：different identity仍有operation timeout，但从`any Err`改为读取真实错误的catalog key并硬要求当前`msg.bad-invite`；打印sanitized key。dial/stream/IO错误不再被当成认证拒绝。保第三方自己1row/root不变、issuer2rows/无第三方row全部原断言。未来明确consumed/claim-owner typed拒绝需要等价映射时由root确认，不回退anyErr。
- B2：first call保`first_error_key`，另记录完整why（含现String中保留的嵌套原因），先精确替换真实ticket与raw token为`[redacted]`，不打印秘密。token仅保本fixture局部用于路径与redaction。runtime必须核具体frame-cap/why；row+burn只是reachability，若实际因果不足只标**large bootstrap+retry失败红**，不能声称framecap真红。
- first call烧票/入row前置当前只定位旧生产red reachability，不是不可改变的未来协议契约。如果N2明确容许preburn Pending→同caller恢复，必须root确认等价oracle迁移，保最终samecaller完成、thirdcaller认证拒绝、数据VV/content/root安全；不得为过测试提前burn。当前没有丢reply/kill注入。

新N0b完整SHA **0f21a85006a32a22fe2a8b4473c2f47828c2072ace1de7a234472a2f82616a84**；N0维持 **76495a0b1a47c92abbd58996a94f3710836a961b1c4ba4d9b080c3f88ab0d4fa**，逐SHA核未变。N0b01 14a6…保历史未覆盖。

external `Q039/pair-bootstrap-n0b-candidate-02` 与internal recovery同名逐文件相同，含新完整source、candidate01 source-before、完整newfile patch、delta-from-candidate01.patch、candidate.json、445 selected Rust/Cargo fingerprints。metadata明确B1/B2/因果限制与未运行边界；freeze前双盘>=5GiB核。两源rustfmt check与git diff-check通过，未Cargo/未typecheck/未runtime，无自有PID/端口/scratch，未动生产或N0。冻结之后保源只读，等reviewer复核该最小delta后root分别授两个exact父首红槽。


## R0真实首跑 — N0业务红；N0b仅准备失败（2026-10-27）

root最终独审后授唯一槽，recovery67927fcf+冻结N0764/N0b0f21原样；未接lookup库新依赖、未改生产/断言/数据。`Q039/node-transfer-first-red-01`使用独立命名STEP=300/TOTAL=720/MIN=5368709120，launcher AST与runner启动AST双验，jobs1/locked/offline/incremental0/既有quality-target，短owned TMP。实际source集合扩大为 **1075**（第一方/lock/vendor，以及真实include的shell/provider/scripts/test等），不是把准备445冒充完整运行输入；**365**第一方Rust/Cargo文件仅mtime touch、字节SHA不变。两个测试都真实重编，不用旧CLI驱动。

| 独立步骤 | 实际结果与边界 |
| --- | --- |
| default normal khor-node --lib check | exit0，15.926s；rawlog显示当前recovery树Node/sync/net/acp等真实check/compile。 |
| N0 exact1普通父 | exit101，100.456s（Cargo编译约1m29s，实际父约9.49s）；**真实产品红**。pair已完成，raw_gap **25,447,954B**（完整同4096×64历史，ownedpath等使其与旧25,448,146B实例稍异，未减数据）；4次public Node访问均真实旧4MiB cap，首visit及后续Seen123/Pintrue已fresh读盘，devices target_covered=false/content_matches=false，最终固定VV+receive_dir收敛断言红。不是bind/setup/timeout错误。worker不另计普通父。 |
| N0b exact1普通父 | exit101，11.811s（新test编译9.83s，实际父1.73s）；**仅fixture准备失败，不是Pair产品红**。在首个Pair调用前，真实snapshot未通过`>24MiB`前置断言；本版actual snapshot长度打印在该assert之后，因此此run没有精确snapshot长度/first_error/burn/retry/third数据，不推断具体cap原因或声称已验证Pair失败。没有静默降门槛/换source/重跑。 |

本次runner显式分类N0 convergence-business-red，N0b setup-or-unexpected-failure并返回1，`all_planned_steps_executed=true`但`execution_complete=false`；不能将两个101都叫业务红。共享target实际二进制receipt：N0 `sync_doc_isolation-862f65292cd2f541` **c22d409e25ad1bb61f52d3f89a9fd1dd0599e030e1f1d08b0c3d3764c4e26b41** /191,836,544B，已证与原before c8186c…不同；N0b `pair_bootstrap_large-70bb126099e049fc` **9fa42435cf508c3eca345acfe75204c678419b2deeab918742c628f8d624e438** /191,761,640B（before不存在）。两binary在全部run结束后再次SHA/size/mtime匹配，未借旧artifact。

总129.283s，无STEP/TOTAL/磁盘/3IO guard触发；最低external **6,257,033,216B**、internal **12,829,810,688B**。Cargo PGID74525/74769/75710全部wait退出并signal0不存在；两真实父等待子退出，each step/root scratch零文件后移除，最终`nt-red-01`不存在。runner74505已退出，唯一槽立即明确归还，无当前Cargo/native/gen/Node/relay owned PID。没有重试、commit或生产改动。

source-before=after=current **1075**逐SHA相同，N0764/N0b0f21保持不变。external run与internal recovery同名完整 **35files**逐字匹配：runner/AST/preflight/freshness/full source指纹、两个source、每组独立rawlog/result/source/binary/cleanup、final/final-audit。库vendor HasIdSpan与tiny_http现有warning记录，不当本任务新增业务失败。当前结果待独立证据审；N0b下一步仅只读分析Snapshot与Updates实际编码/给纯诊断及合法数据形状建议，root另定新fixture，不把当前setup失败计Pair产品红。


## Pair Snapshot只读结论与candidate03纯诊断

当前N0b02及R0首跑证据保持历史，不将Updates体积套给Snapshot：

- `DeviceDoc::snapshot`→`glue::snapshot`→`ExportMode::Snapshot`；锁定vendor默认Snapshot明确包含**完整history+当前state**。`fast_snapshot.rs:286`普通路径取`oplog.encode_change_store()`和`state.store.encode()`；只有已有shallow root store才走独立shallow分支。当前没有调用StateOnly/Shallow，不把体积小解释成GC丢历史。
- `oplog.rs:1213`→`change_store.rs:211 encode_all`→`flush_and_compact`→`external_kv.export_all`。实际flush_and_compact:905仅把完整change blocks与VV/frontiers写KV，不是删除旧map历史的CRDT GC。
- Cargo.lock锁`loro-kv-store 1.16.0`；本机对应registry源码`mem_store.rs:28`默认`CompressionType::LZ4`，export_all:235生成/复用compressed SSTable，compress.rs:41使用lz4 frame。Snapshot多了一层KV/SSTable压缩。
- Updates走`fast_snapshot::encode_updates`→`oplog.export_blocks_from`→`change_store::export_blocks_from:718`重编码block→`encode_blocks_in_store:789`直接block bytes，不经过上述KV/SSTable LZ4导出。故真实25MB Updates不证明Snapshot也25MB；flat/nested镜像和地址JSON重复也可能被压缩。这里是实际代码结构解释，当前run没有实际Snapshot byte数，不能据此捏造压缩率/精确长度。

root已批准唯一candidate03改动：把现有安全`[Q039-PAIR] prepared snapshot_bytes=... epoch=... ticket_present=true` println从三个体积assert之后**移到之前**。没有增加完整Updates导出、没有改4096×64history/阈值/Pair/认证/最终安全断言。精确move reconstruction证明candidate02→03仅该移动（含邻接空行）。N0保持764不变，未占槽（槽当时转R1 normalNode check）。

N0b03完整SHA **f9de265aeac1b3ad558f195666262f14dc5316858ab54f677569804b50913d03**。external `Q039/pair-bootstrap-n0b-candidate-03`+internal recovery同名逐字一致，含03source/02source-before/fullpatch/delta/metadata/445selectedinputs；fmtcheck/diffcheck通过，**03尚未编译/运行**。下一exact是诊断阶段，仍可能在同一准备断言停止，不能称Pair产品红。

合法数据形状方向（待actual byte数及root另定固定参数）：继续同一个现有issuer真实device id的有效IPv6地址历史，增加历史更新次数，最后恢复真实地址，不新增可拨打假设备、不塞无业务含义大label/style/证书、不改编码/压缩/threshold。先量03原4096组实际Snapshot，再确定单个固定、更大的iteration参数并独立freeze/run，目标真实Snapshot>24MiB且<=64MiB；不得盲按Updates比率外推保证，也不写无限“直到够大”生成循环。该后续新形状是新fixture源/新结果，不能套用R0旧source数据。当前仅此方向，未写任何新参数/生产/重跑。


## candidate03 exact体积诊断完成（不是Pair产品红）

root将唯一槽从已结束R1交回，仅授03 exact N0b；本作者只运行该1普通父，未重复normalcheck/N0，未接lookup依赖、改HEAD、history、阈值或生产。runner85656/CargoPGID85686，独立STEP300/TOTAL420/MIN5368709120通过launcher AST+启动AST双验；jobs1/locked/offline/incremental0/shared quality-target、短owned pdiag-03。1075实际inputs前=后=current；365第一方mtime-only字节不变，rawlog真实当前recovery Node重编57.38s，不拿R1覆盖的shared artifact冒当前。

实际日志在前置assert前给出 **snapshot_bytes=9,742,899 / epoch=1 / ticket_present=true**。随后同`>24MiB`前置assert失败，first Pair尚未调用；exact父exit101，阶段64.166s，worker约6.23s。runner exit0/`execution_complete=true`仅表示本次“取得实际体积”的诊断完成并收口，不是测试绿/Pair产品红。没有first/burn/retry/third业务结果，也没有自动加history或继续跑。

实际当前binary `pair_bootstrap_large-70bb126099e049fc` SHA **c856cd06b6e8e965b37f400cd705abe86ef9491def7050a00589f5d32d3b0d93** /191,761,640B，运行前仍为旧9fa424…，后续binary-after再次核SHA/size/mtime完全相同；365freshness与actual Compiling日志一起证明当前来源。最低external **6,181,150,720B**/internal **12,623,212,544B**，未触guard；PGID85686 wait结束并signal0gone，step及root scratch零文件后移除，`pdiag-03`不存在，唯一槽立即归还。

external `Q039/pair-bootstrap-diagnostic-03` 与internal recovery同名 **21files**逐字一致：runner/AST/preflight/freshness/source前后、两source、step raw/result/binary-before/actual/binary-after/cleanup、final-audit等。当前N0764和N0b03f9de未改，未commit或再Cargo。

已向root报告实际9.74MB，并仅建议下一固定参数12288组（4096的3倍，同existingdevice/64合法地址/最终恢复真hints），目标实际Snapshot>24MiB且<=64MiB；约29MB仅待测估计，不保证线性。参数/新fixture写入与重跑等待root另授，当前未加量。原N0 25.45MB Updates真实红保持独立，不套给新Pair形状。


## candidate04固定合法history与真实Pair业务红

root据03 actual9,742,899B明确批准12288×64同existing-device历史。本作者仅改N0b loop4096→12288和准确comment，精确重构证明没有动阈值、Pair/retry/auth/content/VV/root断言。当前04源SHA **e7dbda3c9c74c757f31663ea82d6d4f16dd1ae3399f29efd4fb0c24f1a512e4f**，N0仍76495a0b…d4fa。external `Q039/pair-bootstrap-n0b-candidate-04`+internal同名8files/source/完整patch/delta/source-before/445selected/1075allinputs/source-delta逐字相同；1075对03唯一变化为本N0b，其余1074库/生产/vendor/lock等SHA全等。

准备时曾错误假设445 selectedRustCargo可从runtime1075路径前缀集合导出，freeze脚本在写冻结/Cargo前assert停；按原alltracked选择纠正，metadata保此工具错误，不算产品red。固定source没有因此再改，也未用旧source运行结果套新形状。

root授04唯一槽后，仅单独exact父运行，未重复normalcheck/N0；HEAD67927fcf不变。`Q039/pair-bootstrap-first-red-04` runner89699/CargoPGID89713，STEP300/TOTAL420/MIN5368709120双AST核；jobs1/locked/offline/incremental0/sharedtarget/短pred-04。**0依赖mtime touch**：以03实际同树编译和1074不变来源续接，Cargo实际JSON artifact记录当前recovery manifest/src lib `fresh=true`，本04 test artifact `fresh=false`及真实executable；不是借R1其他树stale binary。仅加Cargo `--message-format=json-render-diagnostics`记录来源，features/backend/cfg/断言不变。

实际结果：**1普通父业务红**（内部worker不重计），exit101、stage25.107s/总25.627s、test编译11.42s、普通父约13.23s：

- snapshot实际 **29,224,082B**，epoch1/ticket_present=true，明确通过原>24MiB/>MAX_FRAME/<=64MiB所有准备断言。
- 首次public Pair `first_ok=false`，完整sanitized链 **`msg.no-answer: 无法读取应答：stream too long`**；issuer fresh row已有caller且原票消费。与实际`exchange.read_to_end(MAX_FRAME)`路径对应，真实oversized Paired回包读限失败，不只是row+burn推测或无关dial错误。
- 同caller第二次public Pair真实 **msg.bad-invite**；third different identity也真实 **msg.bad-invite**（非anyErr假绿），第三方仍自己1row/root不变、issuer仍2rows且无第三方row。
- caller fresh目标VV/content均false、epoch1/1、rows1、same_root=false/holds=true（standing trusted属于其自己的lone网络，不当issuer已同步）；resident stop之后最终samecaller恢复断言实际红。没有伪RPC/import替代Node，没有丢reply/kill注入，不能据此称这些恢复窗口已覆盖。

actual04 binary `pair_bootstrap_large-70bb126099e049fc` **541fdf8539728425bffd3a4e7bb873567496c60db97f4e1c2267258404af6e0c** /191,761,640B，before为03c856…，全部run后再次SHA/size/mtime相等。1075输入before=after=current；sourcee7db/N0764及HEAD679未改。最低external **6,131,322,880B**、internal **14,772,289,536B**，未触guard；PGID89713 wait退出并signal0gone，step/root scratch零文件清后`pred-04`不存在，唯一槽立即归还。

external run与internal同名 **24files**逐字一致，包括compiler-artifacts全JSON/current-node-library-provenance、rawlog、source/freshness、binary before/actual/after、assessment/final-audit、cleanup/AST/资源。当前两真正业务红分别属于：N0764的25,447,954B Updates设备收敛失败；Pair04e7db的29,224,082B Snapshot bootstrap+同caller重试失败。Pair02/03仍是历史setup/diagnostic，不能混称三次Pair产品红。

最新现场：recovery `fix/quality-node-doc-transfer` @67927fcf，恰N0 tracked test dirty+N0b untracked新test，生产未改，无自有Cargo/native/gen/Node/relay PID/端口/scratch。未commit。等待root独立证据审/单N0 audit ref授权，然后再接新lookup库686f依赖与N1小片；R1公共link/proto写权须继续协调，不一次铺完整复杂实现。


## N1 源实现检查点（2026-10-27，尚未编译/运行）

root已独审接收N0与Pair04两项真实业务红，并授权审计提交：`dd60fc5da0d17f96aad9ee771c8762783894b3f4`，仅两fixture，title明确当前失败；ref `audit/node-transfer-n0-red-20261027`。提交tree `6341b27487ea60002dc9f2f8dea2edb237a370f7`，N0764/Pair04e7db blob与真实受测冻结逐字一致。随后仅cherry库lookup原`686f2e42eb634ff5ba0edc34ab822fe8764f13f2`七源为`5f2d5cf77ad05bd7e8ce2e11d0b90c36485c7ef3`（tree79229cf5ab8722852fd21f0ebd8a657799e381d7），七源逐SHA一致；两步收据已双盘放回原run。没有把红fixture并入wave1。此前卡末“未commit/未接lookup”已成为历史，不作为当前阶段。

R1已冻结并移交本树`link.rs/proto.rs/lib.rs`后续写权，root另授catalog en/zh只新增Pending键。当前本作者正在实际实现正常Node N1完整切片，非DTO占位；没有复制R1另一树WIP。实际允许差异共13路径：catalog两源；Node lib/link/proto；新`link_doc_transfer.rs`、`doc_transfer_state.rs`、其io/registry/tests三个私有子源、新`doc_transfer_wire_tests.rs`；定向tests agents_wire及sync_doc_isolation。未动sync/core生产、锁、依赖、GUI、生成器或Pair04源。当前Cargo唯一槽仍root/R1调度，本作者**未Cargo/未typecheck/未运行本候选任何测试**；rustfmt只作解析/格式，不能称N0绿。

- 正常生产`Request::Sync/Response::Synced`、`wire::Peer`及rounds/from调用已移除，只有DocTransfer单路径。canon DocumentVersion用于inventory与have，完整256KiB chunk、encoded1MiB DocTransfer上限；通用MAX_FRAME仍6MiB。Exchange小doc可当请求完整strict apply；大doc双向合计每doc每量子最多4chunk/1MiB，独立doc公平轮转，已知doc失败继续并按固定registry/selected顺序聚合首错。visit有32MiB/30s协作预算，未完成明确Pending；不同epoch明确Apart，不能描述AlreadyLatest。
- heavy步骤进单seat blocking，等待队列最多8；Context只复制root/writer/identity，不重新打开Node。Loaded/pile/metadataGate/blobOwner全在同步闭包退出、network await前释放，chunk只有owned Vec/manifest跨await。对端authenticated source/recipient方向与统一11 global+合法wide registry在metadata/Blob IO之前校验；不提供按任意字符串id读文件。
- Node私有bounded journal存Preparing(full固定doc/source/dst/epoch/base/target/encoding)→Ready完整manifest，Ready之前绝不offer。core retained_manifests/open_existing/resume负责blob扫描/owner/gate/recovery，Node不猜blob FS。唯一准备匹配恢复，多个/坏/unknown保留并报reconcile；合法但未发布的incomplete exact Preparing可取消并重计划，已有Ready目标不会被本地持续新op替换。尚须实测resume每次重开全body校验的时间/RSS；未宣称O(chunk) CRDT/RSS。
- Incoming先persist Receiving后receive；Write仅现存exact record，ACK GC后晚Write Missing不create。VerifiedComplete→Importing落盘→真实apply_staged/apply_checked+flush→可恢复Complete receipt，之后才网络Applied。丢commit结果/重启若fresh VV已覆盖，只可明确AlreadyHaveEquivalent，不伪造特定digest Applied。sender仅valid peer-bound proof后Accepted→删exact outbound blob→NeedAck；receiver收到exact proof hash ACK才Cleanup→删exact incoming stage/receipt。丢ACK结果后晚Exchange产生新Equivalent时Reconfirm新proof再清，不误删新id。重复body不重做普通business通知，通知自身沿现follow/take_arrived幂等机制；进程崩在通知与mark之间仍可能重入幂等通知，不声称跨平台整机断电事务。
- journal最多64 outgoing+64 incoming，每record192KiB、snapshot32MiB，固定3控制文件；checksum/generation+private next image/sync+rename支持process restart。未知/坏complete/links保留并报错，不静默删除。其Node plan/receipt metadata publication独立于core blob管理，不重写core私有stage规则。
- quiet正常轮仍一次合并Versions(doc digests+reading digests)，只有超预算inventory/reading继续页；Deferred必须每页退至少一项且严格names，零进度报错。reading失败best effort，doc首错优先。Pending新catalog真实签名`sync_partly_pending(moved,pending_docs)`与`sync_partly_pending_and_apart(moved,pending_docs,apart_docs)`，en/zh恰4行；无空apart标签。
- 小Pair原signed certificate回推已用同pump，import+flush后的Loaded显式drop后await；为此当前Paired带canonical issuer version。巨大Paired Snapshot/巨大Vouch roster、pure settlement和caller/token claim重试仍**N2未完成**，本片不称Q039全部完成。

定向source oracle：新11普通unit父（尚未执行）包含真实PinDoc checked import+并发本地pin保留、receipt reopen双向GC/晚WriteMissing、large固定target在lateop及owner reopen后不变、丢Released+新Equivalent重确认且旧ACK不删新id、错误authenticated方向、journal所有partial publication prefix恢复/坏complete与unknown保留/Unix symlink hardlink targets不触；实际Rust编码最大合法65416B binding+65532B have/proof+完整256KiB chunk+最坏Reconfirm serde envelope预算、非末片短chunk拒绝、全部六action retryable但不early/30s allowlist、Pending与mixed summary/首错。实际bytes和11父结果留待独审/授槽后，不先写估算为已测值。

root批准旧4MiB oversized case的契约迁移：仅旧oversize父改名为当前body-limit，同existing device原4096×64合法history精确跑3次，先assert actual rawgap>64MiB，随后要求typed当前outbound ResourceLimit的原因（不是anyErr/旧frame cap）。Seen123/Pintrue首visit推进、失败devices不follow/target缺失保持；epoch case加强catalog Apart显式结果。inventory两个核心oracle未弱化。N0新25MiB convergence函数及history helper须与audit逐字核验，Pair04仍e7db未动。原4旧结果属于audit ref；新契约不是把旧cap失败硬保留阻正确协议转绿。

当前剩余：完成源码冻结与独立审；root授唯一Cargo槽后normal Node typecheck、11新unit与原定向normalPair/knownhost/inventory/epoch/当前body-limit/N0真25MiB父按确切来源运行。任何编译/setup/guard/transport失败单独归因，不算业务红/绿；只有真实public Node N0目标、VV/content与Seen/Pin通过才称正常大差集收敛修复。N2 Pair04仍红基线，不在N1中弱化。


N1 candidate01实际冻结：external `Q039/node-transfer-n1-candidate-01` 与internal recovery同名 **25files/约3.36MB**，13授权source（7 tracked+6 new）、**1083**实际选定source inputs，SHA逐字双盘一致。完整source manifest SHA **b7b4bbf980b791ca79b4a4ce2795d3465f664eef8dd9be130ce47f4bd6071ca3**；complete.patch SHA **d45ebfb8fca668a539842abcbe39519d5ac498d7a89f2806b4e47c6c65ed1f0b**。fullpatch含所有untracked新源，临时index从HEAD read-tree后`git apply --cached --check`通过，未改生产index，临时index已移除。source-before/after SHA核不变；oracle-preservation.json明确N0/history/next/inventory/worker和原epoch/inventory父函数逐字相同于audit，Pair04 e7db未动。

图工具在actual recovery先调用minimal-context/file-summary与detect-changes，仍显示build67927、currentHEAD5f2、new link_doc_transfer零节点，故其0flows/漏test不可当覆盖；本片逐source/diff补核。六新源rustfmt check、现lib/link/proto仅emit stdout/skip_children解析、diff-check、两locale TOML解析通过，只有源码/格式证据；没有Cargo、typecheck、testbinary、runtime或N0 green。冻结当下外盘6,014,545,920B/内盘14,684,250,112B，无自有运行scratch/PID/端口。源停止变更交root独审，等待其具体后续槽/修订，不自行运行。


### N1 candidate01 normal类型检查完成（仍无N0/unit结果）

root只授唯一短槽normal khor-node --lib check，本作者复用原R0 runner控制：`Q039/node-transfer-n1-normal-check-01`，命令仅`cargo check --locked --offline -j1 -p khor-node --lib --message-format=json-render-diagnostics`。STEP300/TOTAL420/MIN5368709120由launch AST及runner启动解析、实际Load引用核对；sharedtarget/incremental0/offline、短ownedTMP n1-check01、3IO guard/ownPGID有限drain。没有CLI、测试或外部生成命令。runner44538/CargoPGID44564，exec session89687，源码candidate01全部13固定。

结果exit0，阶段15.307s（Cargo14.72s）、总15.406s；**normal Node类型绿**，不是N0或11新unit行为绿。1083 inputs before=after=current逐SHA一致；373第一方Rust/Cargo仅mtime touch后字节不变。Cargo artifact准确指向当前recovery manifest与src/lib.rs，lib `fresh=false`、`executable=null`，不是R1旧树artifact。实际产物仅rmeta：`libkhor_node-69bf81ac6cf56e4b.rmeta` SHA **cc8acc00700990c33768639f15148510fa3d015404574befc3f9528351f6a946** /16,005,590B；不当真实Node test/CLI binary。json-render-diagnostics把诊断渲染成raw文本，compiler-messages.json为空不是零warning：本候选有normal unused DocProof/DocProofKind imports一警告，保源不自动修；vendor HasIdSpan既有warning另记。

min外盘6,013,374,464B/内盘14,672,281,600B，未触guard。PGID44564已wait并signal0gone，step/root scratch零文件移除、n1-check01不存在；正常类型结果一取得即明确还槽，未续编译。双盘同名**33files**逐字一致，含runner/两AST/preflight/freshness/full inputs/source13份、rawlog/artifacts/currentNode provenance/rmeta SHA/final-audit/cleanup。HEAD/source manifest仍原5f2/b7b4、无source改动或重跑。root独立N1源审和真实N0/unit运行仍待后续；N2 Pair04继续原红基线。


## N1 candidate02：独审B1/C1最小修订与独立首红准备（未编译/未运行）

已完整读`Q-039-node-n1-review.md`。candidate01及其normalcheck通过证据保持历史不改。root仅授源修订，未授本轮Cargo/native/gen；当前HEAD仍5f2d5cf7，所有权仍原13实际路径（agents_wire本已在candidate01的13路径中，不另造第14文件）。本轮相对01恰5源变化：link/proto/link_doc_transfer/doc_transfer_wire_tests/agents_wire；其它8源与1083集合保持。

- B1实际修复：`proto::question(raw_bytes)`在真实decode后检查DocTransfer原bytes.len<=1MiB；真实serve的Watch识别和普通dispatch均调用该入口，超过限额在dispatch/admit/nudge/metadata/blob之前返回专用Refused。保通用read_to_end(MAX_FRAME)=6MiB，不用重编码长度替代实际帧门，也未扩N2。
- C1：最大Exchange request的proof与offer/ack取相反manifest角色，保持同宽ff/fe字节；测试先实际question decode，再调用生产同一私有validate_roles。三个wire测试模块从proto移到documents的test子模块，故不为测试扩大生产roles接口可见性。清理normalcheck指出的unused DocProof/DocProofKind imports；其它生产算法不变。
- 新独立普通父只加在既有agents_wire.rs，整个原agent测试+preamble逐字prefix不变。其仅依赖candidate01已有公开Pair/wire/常量，不调用新增question，因此可对01生产真实跑红。真实issuer住户开机→public Pair让新receiver采用issuer网络→停issuer→receiver实际serve→用issuer原identity.key绑定真实endpoint，硬核sender public id及connection remote_id。秘密不打印。先同连接发送2MiB普通Versions且要求typed空inventory，证明真实admit并保通用6MiB；新的receiver两个transfer目录仍不存在。
- 然后同连接发两份真实原始2MiB payload：完整canonical256KiB合法Write（来自真实flush的Pins大差集）与空Confirm，均以既有public proto::encode后尾部padding组成，不经过producer cap。每次记录sanitized variant及两个transfer目录是否仍缺失。随后同连接发未padding完整canonical256KiB Exchange，必须typed Stored(next_offset=262144)且两目录现在创建，排除认证拒绝/IO或永久关闭整个doc通道冒充B1拒绝。endpoint close、receiver resident abort+await后，先硬断言合法control成功，再断言padded专用raw-cap Refused与无副作用。setup/未admit/合法control错不算B1红。当前未实际执行此fixture，不能声称runtime重现。

独立test-only冻结 `Q039/node-transfer-n1-b1-test-candidate-01` +internal同名**4files**：agents完整源SHA **413d17c920a0d51ac8670347fa08442457a8e01d623b7c1a789b9403041aeba7**、01before/test-only.patch/metadata。新父名 `padded_doc_requests_are_refused_before_receiver_transfer_state_exists`。

红候选 `Q039/node-transfer-n1-b1-red-candidate-01` +internal同名**19files**：冻结时actual树短暂精确恢复01四源，仅agents新父相对01变化，真实1083 fingerprints/source13/fullpatch/独立delta/patch check；随后恢复02，未Cargo。red source manifest **2eb72052ab4aa363093b9fe62685ac77316a6cd8b2f36b9b9eb7edb00fd37c4c**，fullpatch **564b59fe78178ce77a0faab1b8d053ddd1529f4febfcc2944663c23cf2f82b24**。当前不是red源，runner的source preflight必须拒绝当前02，不能冒用02结果叫首红。

02修后 `Q039/node-transfer-n1-candidate-02` +internal同名**19files**，13源/1083 inputs，source manifest **2ba81e31c3438a0529b96be54cde59337d7ca1db23664a6ae7cc4cc3c276073d**；fullpatch **7c8c2a97872992d9caec56882b7ac36c074fe150384bd6704bb0279dd674e87f**；相对01最小delta **bebb4d93f69c65863d4f85c35b3795981902d95d18bb2ab95f453e9f38d61ca9**。red/02新测试完整字节相同。两个fullpatch临时index对HEAD适用check通过，生产index未改、临时index移除；两候选所有文件双盘逐字一致，freeze外盘5,998,395,392B/内盘13,259,161,600B。源静态rustfmt/parse/diff-check通过，N0/Pair04/audit oracle未变；不是typecheck/业务绿。

已准备两个受控runner，仅源码/AST未launch，各2files双盘：`Q039/node-transfer-n1-b1-red-01/run.py` SHA **c80eb7ec78b7f0d4ce4a0e2bcae3659f0cfeb3897ebff3305a05f0fbc1e091b4**；`Q039/node-transfer-n1-b1-green-02/run.py` SHA **bb3c4d4166160895202817205646f7bfefd2d580b5cf0fb5f860c771b17f29d0**。复用normal01 guard控制，分别独立STEP300/TOTAL420/MIN5368709120、launch AST+实际Load引用、lockedoffline/jobs1/incremental0/sharedtarget/短TMP与private child HOME（Cargo/Rustup tooling home保留）、373 firstparty mtime-only、ownPGID有限drain/3IO/source/artifact/binary/cleanup收据。每runner只exact该一个新父，不自动改源；预期red必须实际Missing/Confirmed且目录已创建、同连接合法Stored对照通过，再目标cap断言失败；green须两次专用Refused+目录未创建且合法Stored对照成功。未知/setup/transport/guard/compile另类并停，不冒业务红绿。

后续确切顺序（待root独审/逐槽授权，当前不launch）：先实际安装已冻red四源+同test、只run B1 exact首红；结果收口还槽。root再授安装02四源（同test）并同exact转绿。两命令均`cargo test --locked --offline -j1 -p khor-node --message-format=json-render-diagnostics --test agents_wire padded_doc_requests_are_refused_before_receiver_transfer_state_exists -- --exact --nocapture --test-threads=1`。然后11新unit（wire模块路径现link::documents::wire_tests）、N0真实25MiB public父、其它current64MiB/epoch/inventory/原agent/normalPair/knownhost回归另列真实不同父，不混B1红绿或拿其代替N0。当前N2 Pair04仍原红，未拓范围、未commit、未清缓存，无自有Cargo/native/gen/Node/relay进程。


## B1真实入站帧门：独立exact首红→绿完成（仍非N0/C1 unit）

root先独审candidate02五源delta与完整raw父确认无新增源码阻断，再授唯一槽仅B1 red→green两exact。现场无其它compiler；launch前两个runner SHA/AST300/420/5GiB/Load引用复核；13源/1083 frame一致。按原授权仅四源link/proto/link_doc_transfer/wire_tests从02切到red，并保存双盘source-transition-02-to-red收据，agents源413d完全不变。两阶段皆lockedoffline/jobs1/incremental0/sharedtarget/私有短TMP+child HOME，373第一方mtime-only字节不变；没有额外CLI/unit/N0/native命令或自动修fixture。

| 同一普通父的阶段 | 实际业务判定与来源 |
| --- | --- |
| `Q039/node-transfer-n1-b1-red-01` | runner67142/CargoPGID67160/session30075，test exit101、runner分类 **b1-boundary-business-red**，69.753s（父4.78s）。2MiB普通Versions先typed通过确认实际同连接admit/general cap。两padded原始2MiB帧真实返回`DocTransfer.Missing`、`DocTransfer.Confirmed`，两次transfer_dirs_absent=false；随后合法262419B Exchange/完整262144B chunk真实`Stored(262144)`且目录创建，通过先行control后才在专用raw-cap目标断言失败。不是Pair setup/认证/IO导致的假红。 |
| `Q039/node-transfer-n1-b1-green-02` | 严格达到上列真实红条件后，exact恢复02四源、完整1083 frame匹配及空间>=5GiB，runner68583/CargoPGID68600/session4456，只跑同一父，exit0/**恰1passed0failed0ignored**，56.155s（父4.62s）。两padded2MiB均专用`Refused(document transfer request exceeds 1 MiB)`且transfer_dirs_absent=true；同连接合法262421B Exchange/完整262144B chunk仍`Stored(262144)`且目录创建。正常close endpoint/stop+await resident先于最终业务断言，证明专用门修复且合法chunk可用。 |

两个run均1083 inputs before=after=对应frame逐SHA一致；red后转换当前02属于有收据授权切换，不将green当前源套给历史red。Cargo JSON两段均actual当前recovery normal Node lib src/manifest `fresh=false`，actual agents_wire test `fresh=false`；没有使用旧/R1 CLI代替driver。red binary `agents_wire-9be4b1a61749ce8f` SHA **88ed4056cb3bdf6253085e003ea0e29bf04a70569c4159da0a536517c2c5f804** /197,680,080B；实际在green前再核SHA/mtime/size。green same-path binary SHA **c12f640d26faad24dc46e7fc8d96332dcd43d195b9b75e5e20a90b93f3429d37** /197,879,696B，green binaries-before逐字段正是red实际receipt，green结束再核同SHA/mtime/size。共享cache合法覆盖历史binary不改red记录。

red min外盘5,711,536,128B/内盘13,133,656,064B；green min外盘5,695,352,832B/内盘13,557,776,384B，均无guard/3IO/STEP/TOTAL触发。两PGID67160/68600 wait结束并signal0gone，n1-b1red/n1-b1green两个owned scratch零文件、emptydirs有限清后不存在；两住户/relay仅测试binary scope，test退出，未触用户Node/发布App。每段source/artifact/binary/资源/返回/cleanup分开receipt。external/internal同名red **35files**、green **36files**逐字一致，green另有red-to-green-summary。red保01 unused imports warning，green已移除；vendor HasIdSpan/tiny_http既有warnings留raw。

green结果一取得即还唯一Cargo槽，未续11unit/N0或其它Cargo；当前实际树exact02/manifest2ba81…不改，HEAD5f2不变。仅证明**同一个B1普通父真实红→绿**，不把两次运行当两个去重功能父。C1三个wire unit（最大actual Rust encode字节）、另外8新unit、N0 25MiB真实public收敛及既有normalPair/knownhost/epoch/inventory/current64MiB隔离均尚未本片运行，N2 Pair04仍原红；正常Node编译不是这些行为绿。本片完整目标尚未完成，等待root后续逐槽/证据独审。


### 下一槽11 unit/相关/N0确切计划与控制准备（source-only）

root调整顺序：先R1相关+paired复用其05现有libtest，N1不先覆盖shared khor_node libtest；本作者保持02生产冻结，无Cargo/native/gen/额外测试。已准备双盘同名 `Q039/node-transfer-n1-verification-plan-02/verification-plan.json`（1file）明确11新unit、9既有相关unit、N0、9相关integration精确名字，不用“knownhost”关键词猜不存在文件、不把SSH reach known_hosts当当前Doc同步覆盖。

11新unit按确切Rust模块：4 `doc_transfer_state::tests`（checked receipt+并发pin/reopen/双向GC、固定target+lateop、Equivalent重确认与新id隔离、错误认证方向），3 `doc_transfer_state::io::tests`（全部partialprefix、坏complete/unknown保留、Unixlink目标不触），3 `link::documents::wire_tests`（C1修后最大合法actual envelope encode、短chunk/超大帧拒绝、六action retry/非early/30s），1 `link::current_inventory::pending_and_mixed_rounds_describe_the_actual_remaining_documents`。没有ignored worker重计，也不把B1已过父再列入这11。

已准备 `Q039/node-transfer-n1-unit11-02/run.py` +launcher-ast（2files双盘），SHA **2b43adcdbff87122b02d2ddab0c5c2576431b3887eb94244652cea2a9a38c01f**，未launch。复用已验B1 guarded控制，STEP300/TOTAL720/MIN5368709120 AST+实际引用；仅先`cargo test --locked --offline -j1 -p khor-node --lib --no-run --message-format=json-render-diagnostics`编译本02实际libtest，再由current manifest/src+profile.test+fresh=false唯一JSON executable解析得到实际testbinary；逐个11 exact普通父直跑该刚编译binary，每步独立ownPGID/source/hash/返回/cleanup。不是用未编译Rust名字猜旧/R1二进制路径；build自身不计普通父。任一编译/setup/guard/cleanup/父失败停止，不自动修源/续跑剩余。

另准备 `Q039/node-transfer-n1-n0-green-02/run.py` +launcher-ast（2files双盘），SHA **5db6a481ac691b0ba1a4cf5b844462cc48b858bb120e8112a4983114a06a5679**，未launch。STEP300/TOTAL420/MIN5368709120，仅N0 exact1普通父；显式要求前11阶段12steps（1build+11exact）complete且1083source全等，前置发生在scratch创建前；同树来源可续不再touch373依赖，实际Cargo lib/testfresh flags仍逐审，不借异树staleartifact。public Node max4visits与原210/180/90fuse、固定VV/receive_dir/epoch/2rows及Seen123/Pintrue不动；判绿额外要求actual rawgap>24MiB<=64MiB、firstvisitSeen/Pin与某visit<=4目标/内容true，再恰1passed。任何其它失败分类停，不套旧cap红因果。

9既有unit为current_inventory除新Pending外的6父，加proto inventory错误/DocTransfer帧roundtrip/未知op拒绝3父，待root安排时可复用11同源binary逐exact，未执行。9integration为current64MiB/epoch/local+remoteinventory4父、原agents注册传播1父、normalPair双向chat/concurrentPair/thirdpeer立即admit3父、真实earlyhandshake1父。原agents可以复核B1已验actual c12f binary+1083SHA一致后直接选该另父，无需为重复挑选同源binary重编；其它源码变更/异树artifact覆盖仍按实际来源决定freshness，不自动跑。

尚缺独立行为证据仍明确：quiet实际仅1 Versions、双向每doc量子实际trace、postflush/prereceipt窗口的fresh reopen。已有receipt test只在完成receipt后重开，不把它冒充更早crash窗口；现只计划，未为这些新增test/production hook而打破02冻结。N0/11结果取得后由root决定最小后续fixture授权。当前02 source1083再次核全等，无自有活test/Node/relay/编译进程，没有清cache/commit或自动launch。


### unit11-02首编失败：0个新unit执行，N0未启动（保02冻结）

R1归槽并已内盘保其实际libtest等6binary后，root正式授唯一槽按11全绿才N0。作者重核现场无compiler、13源/1083exact02、runner原SHA及AST300/720/5GiB和实际引用，外盘5,878,296,576B/内盘12,062,699,520B。仅launch `Q039/node-transfer-n1-unit11-02/run.py`（81174/session77297），第一步CargoPGID81244执行`cargo test --locked --offline -j1 -p khor-node --lib --no-run --message-format=json-render-diagnostics`。

实际exit101/30.617s：唯一E0277在`crates/node/src/proto.rs:1510:42`旧inventory错误翻译unit的expected `version: "a-version".into()`，ByteBuf未实现From<&str>。这是作者漏改一处cfg(test)期望值；正常production lib/B1 integration不会编该单测，先前normal与B1绿不证明它。编译尚未产新的current libtest artifact/binary，原cached/R1 binary没有借来执行。**11新普通父执行0、N0未launch、未运行其它相关或重跑B1**，不是业务红。

runner保原宽分类`guard-source-process-cleanup-or-build-failure`，final-audit明确归因纯libtest编译失败，guard=null，source/cleanup均正常；不能误报资源/磁盘异常。373第一方仅mtime touch、1083 before=after=current逐SHA相同，生产/测试源未改。PGID81244 wait结束并signal0gone，n1-unit11 scratch零文件及空目录清后不存在；min外盘5,850,222,592B/内盘12,025,614,336B，无STEP/TOTAL/3IO触发。唯一槽即时归还。

双盘同名完整33files逐字一致（runner/AST/full source13/freshness/各层source输入、原始诊断/当前编译artifact清单/旧binaries-before/result/cleanup/finalaudit）。next N0 run目录仍只有准备run.py+AST，无active.json。未擅自修源、重跑、清cache或commit，当前exact02 source manifest2ba81保持。建议后续仅`proto.rs:1510`改为`version: b"a-version".to_vec().into()`，不改生产/fixture/oracle，另冻03最小delta并等root复核/授槽；此处只是提案尚未实施。


### candidate03最小cfg(test)编译修复已冻（尚未重新编译）

root实读1490–1512确认expected漏转换后，授权仅一行。作者实际把proto.rs:1510 `version: "a-version".into()`改为`version: b"a-version".to_vec().into()`，保持预期字节/全部断言，不改生产逻辑或任何其它源；1083输入唯一proto变化，其它1082全等02，Pair04/N0/core所有oracle不变。02和其编译失败33files证据保留，未修日志/重跑/B1充数。

`Q039/node-transfer-n1-candidate-03`+internal recovery同名**19files**，13源/1083inputs：manifest **ef6ef049f55e8c8c2e1d79b1fb5b30a6c819185b13c9fedec71a183f207fc269**；fullpatch **5a9a9a2c879e20a9f7f1c369f5a512116f73db9f01b0e17d23cd81b82997d3b9**；02→03最小一行delta **b2feef64ad28f4296fdf9682cea808e8ca22480699e31f6edb677fd1c96baed9**；proto source SHA **e8d62923026d22269d4875fac597d1e6688d35ee3422e85e6c17e5a2801ae33c**。fullpatch临时index对HEAD适用check通过且已移除，不改生产index；rustfmt emit-only解析与diff-check通过。只是测试编译修复source，不当业务红→绿或实际typecheck。

新的prepared runner：`Q039/node-transfer-n1-unit11-03/run.py` SHA **8110ff66b92b8c42b8f992f39bbd954a0889e77801498215b3f682d26124e3f5**；`Q039/node-transfer-n1-n0-green-03/run.py` SHA **d2c8ae5d52c4f9a225c0ccf3ecd39fd53a3d93fb15105351e5946a4ca6ecde38**，各run.py+launcherAST双盘2files。只换candidate03/output及N0前置unit03结果路径；exact11名字/命令与300/720或420/5368709120预算完全不变，AST实际Load引用核过。短TMP沿原n1-unit11/n1-n0，unit02已清零、N0从未创建，runner仍不自动改源/修fixture。当前源冻结03，无Cargo/native/gen/newtest启动；root delta审后另授槽才可重新首编→11全绿后N0。


## unit11-03真正11绿；原N0-03首访问hard Busy失败（未收敛，不改源）

root复核03单行cfgtest修复后再次授唯一槽，先unit编译+11全绿/source/binary/cleanup齐才N0。launch重核无其它compiler、13源/1083exact03、两个原runner SHA/AST/Load引用与300/720或420/5368709120；外盘5,843,595,264B/内盘12,023,435,264B，旧TMP不存在。未重跑B1或其它组合。

`Q039/node-transfer-n1-unit11-03` runner83382/session28695：current actual libtest build PGID83414 exit0/59.173s（Cargo58.72），实际src/lib.rs+manifest均recovery树、profile.test=true/fresh=false；新binary `khor_node-73af34573690fd43` SHA **356acc6b3013e1ef3df7442122f11f4175720e1c9a5ca9f09930b3c0e0bce7bb** /234,023,624B，before旧R1 cache24ebfc42… /236,369,160B，未借旧binary跑新源。随后逐11 exact不同普通父全部每个1passed/0failed/0ignored，编译不计父，总82.538s。11执行PGID83988/84008/84101/84109/84112/84150/84158/84166/84179/84187/84194及build83414全部wait并signal0gone，n1-unit11清零移除。

C1最大合法metadata/fullchunk的**actual Rust编码实测**：request **590,328B**、response **721,546B**、chunk **262,144B**，全部producer/decoder与private角色门通过，证明512KiB不足、当前1MiB有余量，未缩短canonicalchunk。其它真实State/receipt/并发pin/fixedtarget/GC/link/journal/allowlist/Pending单位oracle亦仅在这11父范围绿；不能据此称实际quiet请求数/fairtrace/postflush-prereceipt窗口已验。

unit03 373第一方mtime-only字节不变，1083 before=after=current；min外盘5,821,018,112B/内盘12,020,920,320B，无guard。双盘**144files**逐字匹配，含各独立step source/bin/returns/group/cleanup、current libtest JSON、实际encode字节和finalaudit。满足授权继续条件后才启动原N0；该11结果与后述N0失败独立，不将N0失败抹作11未执行。

`Q039/node-transfer-n1-n0-green-03` runner84965/session16669/CargoPGID84979：先验unit03全部12steps complete+同1083源码，0依赖mtime-touch（明确沿刚实际同树编译来源）；只跑原public Node convergence exact1普通父。actual normal Node lib src/manifest当前且fresh=false，test source当前且fresh=false，实际新 `sync_doc_isolation-862f65292cd2f541` SHA **710d1eb87938430973ada87d1f58138dc1e9180420f77addf40f56b402f4de3c** /197,485,824B（before不存在），结束再次SHA/size/mtime核同；忽略worker是本父真实child，不重计另父。

N0 test exit101，75.867s（实际父33.85s）；Pair及真实准备通过，actual raw_gap **25,447,938B** /epoch1/1，原4096×64/24MiB/64MiB/fuse/oracle全部未动。首次public sync却返回硬错误 **`document transfer state busy`**，在fixture原“非old-cap错误不冒large-history红”断言处停止，尚未读取/打印首Seen123/Pintrue或最终VV/content，不能称这些已绿、旧4MiBcap真红或最终收敛失败同因。runner保宽分类compile-setup-or-unexpected-failure，finalaudit明确是setup已过后的真实runtime异常hard Busy，而非compile/大小准备/IO/guard/cleanup；只有该一个普通父失败，未续其它组合。

只读代码具体线索：`link_doc_transfer.rs:555` receipt budget到期 `Err(BUSY.into())`，BUSY为&str而`Error::from<&str>`默认Invalid，`transfer_documents`因此不能按Failure::Busy标Pending而成hard首错。push/pull处理Failure::Busy的:520/:644也同写法；已有Error::busy()构造真正typed Busy。约30s runtime与deadline相符，但当前未添加trace证明是哪一分支；这里只报明确静态类别错配与最小建议“这3处用Error::busy()并清不用的BUSY常量”，**尚未改源/重跑**，也不借此掩盖resume全body校验耗时仍待真实N0测。

N0 1083 before=after=current exact03，min外盘5,519,003,648B/内盘12,017,709,056B，无guard/STEP/TOTAL/3IO；PGID84979 wait并signal0gone，n1-n0 step/root scratch清零移除。双盘**34files**逐字一致，actual source/binary/artifact/returns/minresource/cleanup分开齐。失败结果一取得立即还槽，当前HEAD5f2/source manifest ef6e…冻结03，无后续编译/测试/自动修源/cache清理/commit。N0、current64MiB/epoch/inventory/agent/normalPair等相关和N2仍未全闭合，下一最小源修必须root另授。


### candidate04三处typed Busy最小修复冻结（未编译/未运行）

root实读确认03首visit错误根因：take_push_state/flush_doc_receipts deadline/take_pull_state三处`Err(BUSY.into())`中的BUSY为&str，经From<&str>变Invalid，不能作为Transient Pending。root只授该生产源最小修订，作者已将三处明确改用现有`Error::busy()`，删除未再用的BUSY常量。没有改From<String>文案识Busy或io错误接口（P4待另片）、Wire/库/预算/fixture/N0 SeenPin/VV/max4/30s或任何oracle。只有`crates/node/src/link_doc_transfer.rs`相对03变化，另1082输入逐SHA相同；所有11单测源字节相同，但生产源全映射已变，因此03旧11绿不冒04同源绿，04须重新compile+11再N0。仍未证明性能/四visit实际收敛，不先升预算/减body。

`Q039/node-transfer-n1-candidate-04` +internal recovery同名**19files**，13源/1083inputs：manifest **f0edab3f10c6eca88c24b286276f306b8eb501d358d26b46c204314a13b409fd**；fullpatch **62d2949bf936139cfc007cd03f6d31bcea192f599712f2d9fa61bc9f33c569a7**；03→04确切delta **9ba8749206a700aaacae30625d34e21c35f5901732e07f7b8b68225562220028**；changed source SHA **5f12f1e340f6f48e3017fd2301c669414cb57d2f027812b5d03fc04ce6bf7424**。fullpatch临时index适用check与rustfmt check/diff-check通过；双盘逐文件一致。03 Busy失败/原旧cap真红/02编译失败全部历史保留，未覆盖源证据。

Prepared新runner各2files双盘：`Q039/node-transfer-n1-unit11-04/run.py` SHA **1b263eaf94435e7e4cfb3e60f48e24780022f0367bead16fb7fcd68804bb3d06**；`Q039/node-transfer-n1-n0-green-04/run.py` SHA **ec9df93d331d7bbc8866ae181e67d4cce3b8181b8e0f71660d04bd2e6ae28f10**。只换candidate04/output/前置unit04结果路径，11名字/命令/300/720或420/5368709120预算和Load引用均不变，AST已解析。04尚未Cargo/native/gen/test，无自有运行PID/relay/Node；当前R1唯一槽，作者source-only不launch。

root另通报：03实际libtest356acc6b与N0exe710d1eb8已exact复制保内盘`recovery/runtime-artifacts/N1-candidate03`并核origin/SHA，随后按storage-audit-13精确回收共享两exe约431MB。该合法后续cache清理不回填或否认03历史binary/source/144+34files收据；04后续本就应真实重新编译。未由本作者清任何cache。等待root04 delta/槽授权。


## Followup复审B1/B2：最小真实flow观察/红候选与修复计划（等待root确认hook，未改源）

已完整读`Q-039-node-n1-followup-review.md`及当前STATE/PLAN，图先minimal/file-summary仍旧67927与5f2不符、新模块0节点不代表覆盖，转实际source核两条确认控制流。04 typed Busy正确保留；04 unit/N0 runner尚未launch，不能按旧排程继续明知有阻断的版本。当前13源/1083再次逐SHA全等04，新doc_transfer_flow_tests.rs尚不存在。

B1确认：现只有Ok((bytes,chunks))提交计量，已真实收发后遇本地accept/receipt/receive或远端Busy会丢body，另initialExchange只留1块但能双向2块。B2确认：started=true先于receipt处理、pull未装入，单向pull首Exchange本地receipt Busy可被done()判真；不能用已有pending扫描补这个无record的窗口。它们是source确定问题，尚未实际新复现，不宣称就是03 N0异常的同一根因。

建议新增路径仅`crates/node/src/doc_transfer_flow_tests.rs`，cfg(test)挂link::documents下私有模块；14th源总量不再扩。既有允许路径限定link_doc_transfer.rs（窄观察、后续StepOutcome/预算/B2）、link.rs（实际wire观察）、doc_transfer_state.rs（cfgtest真实私有gate guard；后片postflush checkpoint）。lib/proto/catalog/GUI/锁/依赖/sync-core库不改，不造通用测试/错误框架、不改原N0或限额。

观察点拟root确认后才写：

1. 实际serve给wire dispatch test-only单次tag；shared admit成功后才记request类型及Doc manifest-id/offset/len，实际response write_all成功后记reply chunk元数据。`answer_versions`会内部self.dispatch六reading，必须靠单次wire tag排除内部调用，不能把它们假算网络请求；early Versions也走实际入口。无payload/token/签名/私钥记录。
2. 客户端BeforeStep、首Exchange真实回复后BeforeReceiptAnswers、AfterStepOutcome三个窄barrier。Controller只调整真实metadataGate/coreSink owner的获取/释放时序，hook不返回伪Busy或伪字节数；所有被统计request仍由真正public Node sync发出并由真正Node handler处理。metadata gate在独立owned控制线程持有，Busy结果已出现后、cleanup前释放，避免只靠全visit持Gate得到泛“receipts Pending”掩盖B2 doc false-done。
3. registry只认本case注册私有Root/peer，至多4roots、2048events/root、64 interned manifests，事件只类型/id/offset/len；overflow/timeout算setup失败，RAII release所有barrier，生产cfg无这些符号/无环境故障后门。现有integration util可cfg include复用hermetic relay/关闭mDNS，不新增依赖或第三方服务。

先F1红候选：04生产行为+上述cfg观察和4普通父，独立冻结/源审/逐槽，真实quiet控制应绿，另外3目标分别判红，不拿一个失败代表另几个：

- quiet：两个nonhosting自有Node/relay先公开Pair、同步至真实doc版本/收据稳定，setup trace清零后一次public sync；实际admitted wire trace恰1 Versions、0DocTransfer/额外continuation/reading刷新，fresh Doc epoch/content不变。内部reading dispatch不算wire。
- fair/busy计量：两合法大Doc双向目标（现有两个device各自地址history、真实AgentDoc历史；不堆假设备），实际固定spool可在setup预热。小Seen123/Pintrue先独立持久。第一Doc已有真实Receiving后持其actual Sink owner令后续body Write/Exchange返回真Busy，另一Doc继续。wire两方向payload及重传全部计，和public summary moved/预算对照，逐doc量子≤4chunks/1MiB、visit实际body≤32MiB；不mock transfer_step返回量。
- remaining-one-chunk：用真实wire累计至`CHUNK_BYTES<=32MiB-used<2*CHUNK_BYTES`，通过按doc BeforeStep时真实metadata gate Busy让第三Doc此前未started，并在该边界解除；04只留1块会放行双向2块，新实现必须不发该Exchange且保持Pending，已started的单块工作仍可利用尾量。小Seen/Pin造成的真实非整块字节保留；不seed/缩小quota、不缩canonicalchunk。若硬件/30s令该物理边界未达，只算fixture reachability/setup未达，不能虚称产品红或改预算凑数。
- pure pull receipt Busy：B有真实>1chunk固定offer、A无push；在真正回复后receipt前握A真实gate，AfterStep看到真Busy即释放（cleanup前）。首public visit必须含该doc Pending而非nothing-to-move；fresh reopen/再public sync完成相同manifest的target/epoch/业务值。仍走真实认证session，不能用未到handler的IO/身份拒绝代替。

F1目标红源码固定后才修生产：小StepOutcome保usage与typed Result，调度发送前为Exchange保留最多双向2块、Write/Read1块额度；known reply已带两向body先提交已知用量，再做local后半段，未知reply按reservation保守计，pre-IO Busy可0，失败/重传照计。先commit用量再处理Busy/hard结果；预算不能等Ok才加。初Exchange先保两个方向manifest，任一本地后半段失败保持started=false/Pending，固定spool/幂等重试不丢pure pull。不扩大From<String>/io错误接口，不改visit30s/max4/25MB/VV/SeenPin/256KiB/64MiB。

F2作为独立下一小片仍是N1收口条件：actual context.apply/apply_staged→checked flush返回、drop Sink后、Complete save前cfgtest同步checkpoint；仅owned receiver ignored worker在此暂停，父真kill/wait，全新receiver进程fresh Node/State Status应持久AlreadyHaveEquivalent后回复，重复proof稳定、fresh业务值+原并发local op+epoch保留。用同真实认证source连接作Status/正确ACK，exact旧manifest清理、另一合法partial manifest不动、late Write不重建。source仍使用生产accept/ack helper，不手写blob布局或新事务层；被kill前无Applied回复。worker输入只在cfgtest入口读取，不在生产放env后门。已有Complete后reopen不替代此窗；可后置保持F1清晰，不能遗忘。

当前只是给root确认的具体方案，尚未写hook/新文件/生产修复，没有Cargo/native/gen/commit；唯一槽仍R1 buffered Stop比较。待root确认观察点及14th路径后先实现test-only红候选，所有04/03/旧cap/11绿/B1历史完整保留。


## F1 flow-red candidate05源已冻结（14源/1084，未编译/未运行）

按root批准，04生产行为保留，仅4个既有路径加入窄cfg(test)观察/真实gate helper和1个新私有`crates/node/src/doc_transfer_flow_tests.rs`。lib.rs原已是13源之一，本次仅加cfg(test) self alias让既有integration util引用当前同一unit Node；无自依赖/新依赖/共享util改动。实际总14变化路径、1084完整firstparty/lock/vendor输入，非口算15。04 typed Busy三处保留，Ok-only计量、初Exchange仅留1块、started先于receipt/pull两项缺口均故意未修，作为先红候选。

双盘`Q039/node-transfer-n1-flow-red-candidate-05`与internal recovery同名**28files**逐字一致：source manifest **2a61eca866440a4f88536b296355b6844cf2d5d6bb135b2e9a3bcc7c44f5d429**；fullpatch **dc0ac8d9f5bf23d293d72026051d95dfe433c34e87da6fa364f57cfd7a0d663d**；04→red最小delta **70be24d4ae21434d036e2d8297e65ff3bedafb549320f121da0e3ff618b83fbe**。14source copies、1084frame、4原路径逆还04逐SHA、具体normal分支等价说明、4 exact计划、rustfmt emit-only解析、临时index fullpatch适用check齐；生产index/HEAD未改。这里inverse仅说明确切变更边界与normal表达式绑定等价，不代替编译/类型或真实行为证明。

实际观察边界：一帧携actual peer/root tag跨await，shared admit成功后才记Request、write_all成功才记Response，原dispatch_inner仅调用一次，内部reading dispatch传None。只记录类型/doc/manifest-id/offset/len与Step/quantum元数据；4roots、2048events/root、64manifest/root、8pause槽，overflow/timeout显式fixture失败，RAII释放。真实core Sink/metadata gate只在指定Before→After step短持有，After看到真实typed Busy后即释放，不持整个visit的全局门；无伪Step结果、wire字段、产品env故障开关、payload日志。normal构建只多等价局部response/result/step绑定，handler/预算/typed处理均保持04。

root实读授权简化fixture为A同实例one-shot公共Node、B真实nonhosting resumable resident及既有hermetic relay；publicpair/sync均实际网络认证。A不publish临时roads且无Aserve，不能处理B反向入站；setup断言真实Arow roads空及无endpoint文件，prepared大body前后都保此事实。observer只把A的quantum与B实际入站成对帧计入本次A→B预算，B自己的pump事件不混计。quiet只证明本次正常公共visit的1Versions，不冒跨visit固定QUIC复用效率。清理先真实deafen B endpoint、停止/await resident、释放barrier、等实际peer-round watch完成、移除私有Node根，再做目标业务assert；private relay归test process退出，由后续guarded runner wait证明。

4个普通父完整前缀`link::documents::flow_tests::`：

- `a_stable_public_sync_uses_one_actual_versions_frame`：publicPair后全部11Doc fresh VV/epoch和receipt队列settle，测一次真实publicsync，要求恰1实际Versions/无其它请求、Doc不变；预期green控制。
- `bidirectional_wire_bytes_remain_charged_when_a_real_sink_is_busy`：两个合法现有设备地址历史+AgentDoc历史双向固定body，真实Write在receiver已有Receiving/Sink冲突得到Busy，独立Seen123/Pintrue及Agent full reverse chunk/轮次/每quantum≤4chunk和1MiB先真实成立。然后比较actual wire body（含Busy重传）与public moved，目标断言`real failed/retransmitted body must remain charged after a Busy step`应在04独立红。
- `a_real_last_single_chunk_budget_defers_a_bidirectional_exchange`：真实双向大Device/Agent history加合法wide/remaining各>1chunk；不seed预算/缩cap。真实wire累计至CHUNK≤32MiB-used<2CHUNK，第三Doc此前由每step短gate Busy保unstarted，Seen/Pin已进盘；目标断言`one real remaining chunk must not admit a two-body Exchange`应独立红。实际30s若未到物理窗口，只算setup/reachability失败，不改数据门槛或冒产品红。
- `a_pure_pull_stays_pending_when_the_first_receipt_step_hits_a_real_gate`：B固定真实Agent reverse full chunk到A的receipt前窄barrier，持A真实gate，After真实Busy后cleanup前放开。先取target仍缺，再fresh新A Node publicsync完成同manifest、epoch/VV与固定AgentSpec；目标断言`pure-pull receipt Busy must leave the actual agents document Pending, never false-done`应独立红。

任何编译/setup/auth/transport/overflow/cleanup/resource/其它断言失败不能计目标红；一个父结果不替代其余。N0整个当前源c77849、Pair04 e7dbda、原11测试源和其它1079旧输入均保持04（仅上述4既有源+1新flow有差异）。原N0 max4/25MB/VV/SeenPin/30s、canonical256KiB/64MiB不改。postflush/prereceipt真实kill/reopen仍另片N1收口条件，本片未实现hook、更未声称已测。

当前root未授Cargo/native，候选只source交独审；4父guarded runner尚在后续准备，不launch旧04unit/N0，也不先修生产功能。全部04/03/旧cap/Pair/11绿历史保留，无commit/cache清理。


F1 prepared runner另存`Q039/node-transfer-n1-flow-red-05/run.py`及internal recovery同名，各2files双盘一致，SHA **2fe7cf3e43a6cf77b5022cc4142a8b2fd0490e54d52f8bf58d2f8455da0d39da**。复用已审unit11 guarded控制，只改当前候选/输出/5步清单与独立分类：current-root cfgtest libtest compile一次→quiet exact1→Busy body exact1→真实remaining1chunk exact1→pure-pull receipt Busy exact1，不跑原11/N0/Pair。STEP300/TOTAL720/MIN5368709120，AST+compile builtin只解析不执行，实际Load计1/1/2；1084/full14源前后、第一方mtime-only freshness、currentmanifest/profiletest/fresh=false与actual executable hash/mtime、shortownedTMP+privateHOME、jobs1/incremental0/lockedoffline、双盘5GiB/3IO、独立PGID/TERM-KILL/wait/cleanup均沿旧控制。新增launch ps只读pid+comm核无cargo/rustc/rustdoc/clang/cc，不读第三方命令参数。

编译/quiet与三个目标分别classification；只有实际marker、准确panic业务消息、恰1ordinary失败、真实数据/Busy/独立doc控制/本片cleanup先过、source/process/tmp/resource齐才计该父target red。remaining必须真实actualbody>32MiB且targetExchange=true，未物理到窗口不称红。任何setup/auth/transport/overflow/clean/资源/意外结果立即停，不自动修fixture或继续其它清单。runner本身未launch、无active/PID，无编译或测试结果。source-only再核1084逐SHA仍exact candidate05。


### F1 runner control02（仅控制修订，source05不变，仍未launch）

root与独立F1源审未确认阻断后，按root四项窄修另存`Q039/node-transfer-n1-flow-red-05-control02`与internal recovery同名，**3files**（run.py/AST/delta-from-2fe）双盘一致；原2fe计划完整保留。新runner SHA **89096024a3796b08ea0b396050f7d05d4aff39a7c363fc2db72b0b3758e2bbcb**。只控制变化，不改源/Cases/真实red条件/3007205GiB/32MiB30s：

- 保持原HOME，不创建/覆盖private HOME；Node数据仍explicit私有根+util.hermetic，不读模型/key。CARGO/RUSTUP等原环境自然继承，仍清KHOR_AGENT_BUNDLE。
- 仅对实际Node lib.rs与新flow.rs两源mtime touch；全部1084 SHA前后/current provenance与实际test artifact fresh=false共同证明当前实现，不再touch所有crates依赖。
- 所有child.wait显式具名LEADER_WAIT_SECONDS=5；stop_owned对已知PGID先TERM_GRACE_SECONDS=8、再必要KILL_GRACE_SECONDS=5，leader已退仍检测/回收成员。任何forced drain都把该attempt列控制失败，不计业务红绿。
- 每launch立即持久owned PGIDs，最终重核所有组/leader。若仍活保scratch失败，不清活group的TMP；最终source与actualbinary SHA/mtime同源稳定、所有组gone、无forced drain、exact5steps分类与cleanup共同参与complete和exit，不能只写字段。

AST+compile builtin只解析未执行：STEP/TOTAL/MIN实际Load1/1/2，TERM/KILL/WAIT实际Load3/3/4，两处wait均带timeout。五条实际命令与2fe逐AST相同；current14/1084逐SHA仍exact manifest2a61eca8。当前没有启动Cargo/native/Node测试、无run PID、无源或HEAD变动，等root短审和明确下一槽。


### F1 control02首次编译失败，0/4普通父运行，已归槽（不计业务红）

root短审授唯一槽后现场再核890960 runner/AST及14/1084exact05、无其它compiler；外14,943,219,712B/内12,349,796,352B。runner PID60065/session1261，实际build PGID60123；只touch Node lib与flow两源mtime，count2/内容不变，HOME保持原值。Cargo test --lib --no-run exit101/23.077s（runner总26.321s），22编译错误；compile-only未产新Node testexe，quiet及其它三ordinary均未运行。原宽classification guard-source-process-cleanup-or-build-failure，但guard=null/无drain，finalaudit精确为编译失败，不能说资源红/业务红/quiet失败。

一项确切新cfgtest源码错：flow_tests checkpoint:185把Option<DocTransferFailure>移入Event，:204又用入Pause；类型仅Clone非Copy，E0382。另21项报DocumentVersion/PrepareSpec/Prepared/ReceiveSpec/apply_staged/prepare/resume、TransferStore.open_existing/retained_manifests等不存在；actual冻结source明明含相关exports/定义。实际Cargo khor_sync normal artifact **fresh=true**，manifest/src_path字段指current树，但actual libkhor_sync-357bf3047a8ecc72.rlib/rmeta被复用；复制的小depfile只有旧relative源集合，**不含当前outbound/receive/version三个模块**。因此current路径metadata与fresh声明不能独自证明compiled依赖源同当前完整树。这是实际缓存依赖映射不符的编译证据，不是API在当前树丢失；原libcheck/11受测收据不改。

只读再核同cfgtest after_step:277用borrowed Error的e.kind也应clone（类型仅Clone），本batch依赖API错时尚未给E0507，不能把只修一个E0382冒全部类型齐。建议root单授这两处clone与后续重冻；若重试，只额外touch当前sync/src/lib.rs使确切库重新fresh=false，不需全crates触时/清未知cache。这些均未实施，生产功能红候选/预算/Pending及所有oracle未修。

`Q039/node-transfer-n1-flow-red-05-control02`+internal同名完整**41files**外内每file SHA相同：14源copies、1084before/after/current稳、raw22、actual fresh库artifact路径/SHA/mtime/bytes、actual旧depfile、compiler-artifacts、源export定位、2mtime、PID/guard/cleanup/finalaudit齐。没有新testbinary，final binary_stable=false正确令complete/exit失败；不是拿旧R1exe冒运行。唯一PGID60123已wait/reaped并signal0gone、forceddrains=[]、private n1-flow02 TMP不存在；min外14,931,378,176B/内12,348,645,376B，无5GiB/3IO/STEP/TOTAL触门。当前无compiler，HEAD5f2/source05不变，无修源/重跑/cache清理/commit。编译首错一取得即明确归还Cargo/native槽。


## F1 candidate06：仅两处cfgtest clone；control03精确重建已证实依赖（未运行）

root授权后只改flow Event `failure: failure.clone()`及after_step map `e.kind.clone()`两个cfg(test)行，无Proto Copy/生产预算/Pending/数据/限额/任何oracle变化。14源/1084不变、1082输入逐SHA与05相同；原05 source/41file失败raw完整保留。`Q039/node-transfer-n1-flow-red-candidate-06`与internal同名**23files**双盘一致：manifest **f349f1c4d6f34340bcaa0c0d55b47840004f45226d349b45c18aad5446b7aad4**；fullpatch **5d2e485f2c662e920eeed0f783baf52d45bec62127625d0d6aec8cbbff6abf9a**；05→06恰两行delta **112443b03495dc4775cf174f2bd77573569955991ebb852386c06d5abc0cea19**。含14copies/frame/delta/临时index适用与parse-only/等价说明/R1比较；无Cargo/native/typecheck结果，normal仍04。

R1 S22精确来源为`Q037/remote-r1/gui-entry-stream-candidate01-20261026T223943Z/inputs-frozen.json`（22源/2350inputs），与N1完整frame比较，JSON+小表存06候选：非Node第一方crate union173inputs，除sync与catalog外其它122输入逐SHA全同（a2a/acp/cli/commands/core/detect/net/web），vendor476全同；root Cargo/lock相同、两边frame/config实际均无.cargo/config[.toml]。差异：sync6生产源（doc_transfer.rs/transfer.rs及4新module）+4个integration测试；catalog仅en/zh各新增2Pending键（其build.rs/src/Cargo全同）。GUI core manifest/src/lib/test另3不同，非Node依赖，不编/不刷新。所有非Rust/Cargo的非Nodecrate变化只有那2个catalogTOML，因此未只盯API遗漏编译输入。catalog相对R1 actual file先核SHA再diff确认，不能把不同生成内容当相同依赖。

root确认catalog差异也必须覆盖，prepared新控制`Q039/node-transfer-n1-flow-red-06-control03`+internal同名**3files**，SHA **5e25dbbbb97dd46d207fe728e371f904b284f419ce1d7dfecbd52c13128cf7aa**，保02原件并附delta/AST。只touch这5路径mtime：Node lib/flow、sync lib、catalog en/zh；全部source字节不改，不全crate touch/clean。仍300/720/5368709120+TERM8/KILL5/wait5、原HOME、5条命令/三靶红识别原样，0launch。

control03新增必须验真实依赖：sync normal lib和catalog normal lib都须当前manifest/src_path且fresh=false；保存actual rlib/rmeta bytes/SHA/mtime。actual新sync .d必须含outbound/receive/version三个当前模块，保存.d bytes/SHA与这些source SHA等冻结frame。记录实际catalog build-script-executed/out_dir，复制actual生成catalog.rs并记SHA，确认sync_partly_pending与sync_partly_pending_and_apart函数存在，cat en/zh/build.rs SHA等冻结；same-source build-script executable可fresh true，不强迫无关重编。Node libtest仍须current fresh=false/actualexe。最终同步重核Node exe+这些dependency/d/genRust SHA/mtime稳定才允许complete，不能只在字段展示。

现14/1084再次全SHA exact06，两候选/控制全部双盘一致，任务卡已写。当前无Cargo/native/test/newtouch、无commit/cache改动，等root短审/授槽再执行compile→4exact；后续setup/编译/依赖provenance/边界未达/cleanup/强制drain首错仍停，不能把05编译失败冒业务红。


## F1 control03真实结果：compile与quiet绿，Busy计量真红；remaining边界setup未达，purepull未跑

root复核06+control03后授唯一槽，现场再次核14/1084/source06与runner5e25 SHA，外15,025,250,304B/内12,325,326,848B、无其它compiler。runner72538/session15926；仅已授5个mtime输入，HOME未覆盖。实际build PGID72634 exit0/61.244s（Cargo约60s），current Node cfgtest libtest新编译/manifest/src_path/fresh=false；actual新exe `khor_node-73af34573690fd43` SHA **99b3c42b8404f6a30aa8057489e89e297e5e4ee9d262ea5c3763812ca92b6ab7** /259,959,096B，mtime1793059064930099041。旧before R1exe7a2f36.../236,377,416B未冒当前运行。

本轮修正了05失败的真实依赖来源：normal sync与catalog都实际current fresh=false，actual rlib/rmeta SHA/bytes/mtime另录。sync新.d SHA **adece1dae79a0720df7b537b09e93faf14ff3670bacd06f62deaeff41e45e43d** /3462B，含当前outbound/receive/version，source三SHA与1084冻结一致；实际build-script out_dir下catalog.rs SHA **59f71c4123bb80f4533b90c79ba88f401a5614c454724436a3de80be67a42635** /748,311B，两个Pending函数均在，cat en/zh/build.rs SHA与冻结相同。生成Rust和.d实际字节已复制证据，不只保存Cargo current路径声明。结束又对实际Node exe、两依赖rlib/rmeta、新.d/genRust逐SHA/bytes/mtime实核一致，final dependencies_stable=true。

按原授权自动依次：

- quiet父PGID73272 exit0/2.741s，恰1passed0failed0ignored；实际A→B visit恰1Versions、0DocTransfer，fresh11Doc VV/epoch不变、收据settle。只证明本次正常one-shot公共visit，不冒跨visit固定QUIC效率。
- Busy计量父PGID73299 exit101/19.449s，恰0passed1failed0ignored且准确业务assert；真实6次Sink owner Busy，actual body **6,377,994B**，public charged **4,805,130B**，相差 **1,572,864B=6×262,144**。fresh Seen123/Pintrue在Device target仍缺时已推进、独立Agent full reverse chunk/轮次、量子≤4chunk/1MiB及完整frame配对等控制均在marker前通过。故仅此父确认`busy-body-accounting-business-red`，不是transport/setup红。
- remaining父PGID73514 exit101/50.405s，恰0passed1failed0ignored，停在flow_tests.rs:1059的联合setup门；**不是靶红**。raw实际receiver31成对帧/31body帧，body **7,602,386B**（devices4,194,304+agents3,407,872+seen106+pins104），距32MiB尚 **25,952,046B**，确定没到最后1块窗口；wide/remaining没有真实Exchange入站。actual source各devices/agents4量子、wide4量子，其中3次After显示真实短gate Busy；没有放大30s/seed预算。单独fresh Seen/Pin boolean在联合assert前未打印，不能仅凭seen/pins wire body推其fresh业务成功，虽已能由实际body证明boundary=false。`flow03-setup-analysis.json`完整保此限制。
- remaining setup首错按原控制停止，**purepull父未运行**。没有在失败后擅自调整数据、放大预算、改oracle、重跑或续跑第四父。

本次source1084 before=after=current exact06，只有5mtime而字节不变；总141.837s，min外14,749,609,984B/内11,942,428,672B，无5GiB/3IO/STEP/TOTAL触发。四PGID72634/73272/73299/73514均wait/reaped+signal0gone，无forced drain，n1-flow03 TMP清零不存在。final source/binary/deps/group/cleanup均true，但all5steps未齐且setup分类失败正确令execution_complete=false/exit1。唯一槽在首错结果时即时归还；后续只有只读证据收口，无Cargo/native/源改/cacheclean/commit。

`Q039/node-transfer-n1-flow-red-06-control03`与internal同名完整**83files**逐file SHA相同，含每step/raw/实际binary+依赖artifact/.d/generatedRust/1084frame/5mtime/4PGID/guard/cleanup/最终实核；原06 source23files/05失败41files保留。原N0/Pair04/11均未运行，Step功能仍04未修。下一片由root决定remaining reachability诊断/性能与独立purepull次序，不以这一个已真实红替代其余父或完整N1收口。


### purepull原父 runtime-only补验控制已准备（06源不动，未launch）

按root新授权，仅准备03未执行的原case4，source06 14/1084全部不动。`Q039/node-transfer-n1-purepull-06-runtime01`与internal同名**4files**双盘一致，runner SHA **cb629f397aa75b075dc9e3528f1f3b1f425942e246b089a136647ac258fb7805**；包含run.py/AST/delta-from-control03/prepared-origin-check。实际命令仅原`link::documents::flow_tests::a_pure_pull_stays_pending_when_the_first_receipt_step_hits_a_real_gate --exact --nocapture --test-threads=1`，0Cargo/0mtime/0生成/无其它三个flow/11/N0/Pair重跑。STEP300/TOTAL720/MIN5368709120与TERM8/KILL5/wait5、原HOME、独立PGID/最终源码产物依赖stable/清理门不变；新短TMP n1-pull01。

actual executable直接引用03现存99b3.../259,959,096B及same03实际sync/catalog rlib/rmeta/.d/genRust SHA/mtime，prepared只读逐项核仍全同；compile证据引用03/libtest-build的原绝对path及其内容SHA，在新目录不造build子目录/伪freshfalse。runtime前还将核03 source-before=after=current1084、原provenance true、actual binary/deps receipts；只认可准确purepull falsePending→freshreopen真完成marker+0/1/0与目标panic，setup/transport/认证/清理/forceddrain首错仍停。新parent只有一个，原BY90/数据/30s/32MiB/身份与VV业务oracle完全原样。当前无launch PID、无编译/测试/新touch/改源。


## purepull原父runtime-only真红已收口，native槽归还

root短审授权runtime01后，现场只读再核cb629 runner、1084source06、实际Node99b3与03全部deps/d/genRust bytes/SHA/mtime。0Cargo/0生成/0mtime，原BY90/30s32MiB/原case4数据和oracle全保；runner81044/session73201，唯一实际PGID81083，exit101/3.752s，恰0passed1failed0ignored。准确marker `first_pending=false first_target_absent=true reopened_target=true`，真实After(agents,Exchange)为Busy；首轮目标缺而未Pending。释放真实gate后fresh reopen同manifest、epoch/VV覆盖、固定AgentSpec(/usr/bin/true,[fixed-flow-b])和retry无agents Pending的原控制assert全部在目标marker前实际通过，才落准确业务panic。固定manifest id **82ca4d4495e3a9d027b482db58248f50e3b71349ab114b8443401ab35fe32f11**，epoch1/body545,946B。

本父确认`pure-pull-receipt-pending-business-red`，独立于前Busy计量真红。新runner exit0/execution_complete=true只表示成功采集这一个预期红，不是产品绿。源码14/1084before=after=current=03完全相同；原03的5mtime也仍精确未变，actualNode与依赖最后再次实核稳。PGID81083 wait/reaped+signal0gone，0drain，private n1-pull01 TMP不存在，min外14,918,963,200B/内11,942,338,560B，无guard。没有新build目录，compile证据只引用03原绝对path与SHA。only父完成即明确归还native槽，未重跑另三/11/N0/Pair或改源。

`Q039/node-transfer-n1-purepull-06-runtime01`与internal同名**42files**外内逐file SHA一致，包含14sourcecopies/1084frame/原compile引用/actualexe+deps/原raw/marker与前置控制分析/PGID/guard/cleanup/finalaudit。剩1chunk仍只有原大历史setup未达、N0及完整N1未绿，不能拿此补验当其它gap闭合。

### 只读性能定位与独立预算样本建议（无源改/未运行新fixture）

双盘`Q039/node-transfer-n1-flow-readonly-analysis-01`（2files）记录actual source SHA+03trace链接。图再次尝试State::open无节点，旧67927图不覆盖current5f2；转actualsource。每read_outgoing为epoch读取Context.version，随后core open_existing/recover全完成prefix，doc_transfer::resume又读完整Vec+digest/checksummed Loro encoding，再取一canonical chunk并dropowner；receiver每chunk freshVersion+recover增长prefix。store snapshot cache命中仍fresh Loro Doc导入snapshot，并非O(1)VV缓存；全局WORK seat顺序执行这些步骤。

03trace29个大body帧都对应预热固定Ready，按各actualmanifest长度合计527,924,795B；上述两次完整逻辑文件读下界 **1,055,849,590B**，还未计receiver prefix/hash/checksum/Loro snapshot import/fsync/metadata。此为source派生逻辑读取下界，**不是物理IO实测或阶段耗时占比**；raw没有阶段时钟，不能武断某一个单项占比。已有7.6MB/30s只证明该实际压力未达窗口。

建议root另授独立few-op合法Chat budget父：4个main wide board每侧各1条约4MiB文本（少量Loro操作）加1个deferred board每侧>canonical首块，固定spool/实际registry顺序setup核后用现有真实gate控制累计原32MiB/30s下的最后1块。保持全canonical与SeenPin，没有seed/抬quota/延长visit/伪Step返回；未实际到窗口仍setup，不能预保证性能。原Devices+Agents压力case完整保留为性能问题，新样本绝不冒原case通过；原25MB N0/max4/contentVVSeenPin也必须后续验。先Busy/Pending两真红原小修或先窄Step elapsed+已有load_nanos诊断，次序由root定；涉及core可复用验证读/epoch fence必须另协调库作者，不能Node手写私有FS或unchecked缓存。当前这里只读方案，06源没有改。


## F1 candidate07：独立few-op Chat预算父+5父显式ignore，生产仍04（只source冻结）

root批准后只修改现有`crates/node/src/doc_transfer_flow_tests.rs`：新增独立few-op Chat预算父及私有fixture helper、5个native flow父显式ignore。完整14源/1084仍同数量，另外1083输入逐SHA全等06；原四个父的**完整函数体逐字相同**，仅外部加ignore属性，原Devices+Agents压力case/原N0/Pair04/11和全部限额/生产代码均不动。link.rs含serve_as exact06已冻结给root插acquire_agent_runtime，本片没有碰公共函数。旧06压力case实际setup失败记录继续保留，不冒第三真红。

双盘`Q039/node-transfer-n1-flow-red-candidate-07`与internal同名**22files**逐字一致：source manifest **20183a2d42d5ce4ba94717afba5d3739cd0fe1c84660691b52db666a34682b3e**；fullpatch **cfd25450a5aaefa396e44ce80174fa41c9ac3eada1f9aa3b23500079ee81a134**；06→07 delta **6915a4f318675724de67e50231e7c006750b876a480c301863b51a5cb1940563**；唯一changed flow SHA **a6cca688ccf42b477f40a3f43be9cb0bebfed577b1d53a986dff604388592328**。包含14source copies/fullframe/hash/delta/临时index适用check/rustfmt check、原4body等值及serve_as未动证明/实际exact计划；没有Cargo/native/生成/typecheck或新fixture运行。

新父完整exact：`link::documents::flow_tests::few_op_chat_defers_a_two_body_exchange_with_one_real_chunk_left`，必须`--ignored --exact --nocapture --test-threads=1`实际执行。4个合法wide main board，每侧每board单一约4MiB文本，固定payload **4MiB+256KiB**（固定1/4MiB余量保证main未过早完成，而非改变资源额度）；另`wide/budget-z-deferred`每侧600KiB单消息。全由真实ChatDoc.tell/flush与同public Node/authenticated handler传输，既有两个device/root/自有relay，不伪造RPC/Step结果。实际main导出必须>4MiB且≤64MiB，两侧deferred实际body均>256KiB，registry实际BTreeSet顺序须main四个先于deferred。固定spool预热后clear**仅本scope测量记录**（原Probe API，不清live全局）；真实32MiB/30s/256KiB/每Doc4chunks/1MiB原样。

沿原Before/After真实controller：边界前每个deferred初Exchange短持真实metadata gate，After看到真pre-IO Busy后即释放；真实wire累计到CHUNK≤32MiB-used<2CHUNK才放deferred，并让其它main step同样短Busy守住真实尾量。阶段计数不seed budget、不改时钟；初boundary采实际quantum前缀校验registry cyclic公平，全部actual wire含量子上限/帧对，fresh Seen123/Pintrue必须独立过。若旧04发出deferred，必须唯一真实Exchange完整request/reply、两向fullcanonical首块且ID分别等原prepared immutable manifests；只有这些控制全过后才用目标断言`one real remaining chunk must not admit the few-op Chat two-body Exchange`分类预算红。未来修后应不发送且deferred Pending，原小SeenPin保留。

为到不了窗口时保phase证据，新test只记录one-message-write、fixed-spool/setup elapsed和按doc/Exchange/Write/Read的端到端Step计数/Busy/总耗时/max（至多15项）；每次Before协调后等待实际After并释放，测量包含真实Step网络/receiver/调度，不冒低层IO占比。全部boundary/body/charged/preIOBusy/SeenPin/targetExchange/visit耗时在cleanup后、联合setup门前打印，因此未达门也不再丢具体诊断。仍未到32MiB窗口就只算setup，决不放宽30s/quota/编码或制造已达条件。原Device+Agent压力case保留为独立性能问题，新样本不冒它已过。

5个重native父默认显式ignore，保护MAXROOTS4与真实global gates免默认parallel冲撞；ignore/0tests绝不计绿。按root最新指示不再做evidence专有通用runner/仓库新script；后续集成的versioned独进程接受入口由root统一给R1/N1，本片仅提交ignore+新父源码。未来green接受必须逐actualexe `--ignored --exact`恰1/0/0；本轮第三红收集则须恰0/1/0且目标marker/控制/clean全过，0/0/1不算结果。root若另授临时单父runner只能沿既有control小改，不会运行旧4父。当前Q052持native槽，本作者0Cargo/native，等待source独审。


### 07临时单父control04已准备（不另建统一入口；未launch）

完成root委托R1 close03非作者独审并交卡后回N1，仅沿既有control03窄改准备一次当前Node cfgtest编译+07新Chat父。`Q039/node-transfer-n1-chat-budget-07-control04`与internal同名**3files**，runner SHA **97ae20b01c8064b63b1a4cc38499dd08d4822ad7642585c512c7d432d9343b00**；附delta-from-control03与AST。唯一本次父为`few_op_chat_defers_a_two_body_exchange_with_one_real_chunk_left`，真实`--ignored --exact --nocapture --test-threads=1`；没有旧4flow/11/N0/Pair重跑，也没有另造evidence通用runner或入仓script，root统一R1/N1版本化入口仍由root负责。

300/720/5368709120、TERM8/KILL5/wait5、原HOME、同5个精确mtime输入、sync/catalog实际freshfalse/.d/currentmodule/genRust两Pending内容/最终实际产物SHA稳定门沿control03，1084 exact07来源。本次必须实际1父0/1/0、SeenPin/near真达/realgate/fulltwo-wayframe/registry公平/量子/cleanup等控制在准确targetpanic前通过，且actualbody>32MiB才分类`few-op-chat-last-single-chunk-business-red`；0tests/ignored和未达window不能充数，首错即停，不改数据限额/源或重跑。后续green接受依然需各1/0/0。本控制只准备，尚未授槽，不自动触发；Q052当前独占native。

现14/1084又逐SHA同07，源码没有新变动。R1审卡只写Q-037-close03-independent-review.md，未改R1生产/既有卡；N1没有自审结论、没有Cargo/native/生成/commit。


### 三缺口最小功能修订接口草案（只读设计，07仍冻；待第三真红后另授）

本片拟只改既有私有`link_doc_transfer.rs`及`link.rs`中窄transport receipt接线；不加RPC/协议字段/重试/依赖/通用transport框架，core/库/Proto wire schema不动。Step拟返回`StepOutcome { usage, result: Result<(), Error> }`，内层在同一owned Usage累加，外层**先提交用量再分派typed结果**。`Usage`区分known TX/RX与uncertain预算；visit与quantum都计known+uncertain，public moved只计known，不把保守预留说成已传输/Applied。重发相同manifest+offset的每次应用层RPC都重新计数，不因重传而去重。

为保留write成功但read/decode失败的TX，最小窄helper拟为：link.rs保原`exchange_on`/`exchange`调用方不变，抽同一实现供DocTransfer私有入口传入`DocAttempt { started, request_written }`两bool；编码通过后、open_bi前标started，write_all成功即标request_written（finish/后续read失败仍保）。单一原liveness/RetireUnheard/QUIC work loop共享，不复制另一套read/write/codec。唯一新的计量调用方是link_doc_transfer的doc_exchange；普通ask_on/Pair/Vouch/其它RPC不传receipt、不新增retry。当前DocTransfer的exchange_on只有单attempt，重连重试在其它ask_on路径，不把它误算为本片已自动双发。

失败点与计量合同：

| 发生点 | known与预算 |
| --- | --- |
| pump前line/dial失败 | 0Doc body；当前id/plan保留，不能声称moved |
| 本地packet/descriptor/roles/编码在started前失败 | 0body、0attempt charge，typed错误原样 |
| 已交transport，open/partial-write等未证整request写完 | TX不报known；该次可能发生的合法body按uncertain保守消耗预算，不冒实际发送 |
| write_all完成，finish/read超时或decode/结构校验失败 | request实际body长度计known TX；无法取得typed reply body的方向保留uncertain，不能清0或谎报RX moved |
| proto::answer成功返回Response | 先取实际DocTransfer offer/Chunk长度记known RX，再进行roles/expected variant/doc/manifest/offset/epoch/receipt/localapply处理；wrong-role或wrong-doc的合法canonical body也不丢账 |
| receipt/apply/后半段gate Busy或IO错误 | Usage已保留，Error.kind不经String重包丢型；Busy按Pending、硬错固定doc首错且独立Doc继续，Apart明确不同epoch |

proto::answer当前可能在返回typed Response前拒绝无法decode/非canonical帧；此时本片只能据receipt保TX与保守RX预算，不声称知道其实际body。这和已decode后roles/wrongdoc不同。出格回包若带超出该action合法预留的额外body，不截断观察值来伪称满足限额，应记可知用量、报协议错并停止后续body；32MiB/4chunk是允许发起的当前合法Doc工作预算，不是宣称敌意QUIC字节流绝不会发送额外数据。general6MiB/Doc1MiB入站门不变，本草案不顺带扩raw-malformed transport改造。

调度`Reservation`与实际charge分开：未started Exchange一律先为**2×256KiB、2chunks**同时检查visit32MiB余量及本Doc quantum1MiB/4chunks余量，即使当前只知单向也先保守留双向；Write/Read先留1×canonical/1chunk。容量check通过并不立即消费，local pre-IO Busy可0；真正attempt后unknown才保守计，完成且可知方向转known释放未用预留。metadata-only正常ACK无body，仍走原有batch/time界。32MiB/30s/每Doc4不改，SeenPin与registry轮转/固定首错不改；没有以成功import字节冒transport资源。

初Exchange处理拟延后started最终提交：校验回复身份/doc/epoch后先保留双方准确manifest到Work并保持opening pending，之后receipt→push state→reverse receive逐步处理。任何Busy/局部失败都不能让`started && pushNone && pullNone`假成功；同固定Ready manifest幂等重试，已接受/GC的旧id必须先按原receipt reconcile，不能把未完成旧target覆盖为新target。所有处理成功后才提交started，且独立保留`needs_exchange`（remote/local Missing）信号，最终赋true不能覆盖Missing要求；Apart/硬错走显式状态，不伪称AlreadyLatest。pending反向chunk/manifest只有通过现有角色和doc绑定才保留，不把错误body送apply。

这是基于actual控制流的简要接口草案，不是已实现/已测结果。07源码未动；当前只执行另授control04采第三红，之后由root确定一次最小修和适用red→green清单。


## 07 Chat control04实际首跑：编译绿，新父仍setup未到32MiB（不是第三红）

root完成07独审与control04/prelaunch后授唯一槽，现场再核runner97ae/14源1084exact07，无compiler、旧scratch/active不存在；外14,818,701,312B/内11,612,864,512B。runner23688/session18862；build PGID23725 exit0/67.270s，实际current Node cfgtest+sync/catalog provenance门全过。actual新Node libtest SHA **2c977958a3fc76ce9535728f8b50701156b748abb40d0d1d0e2a4af20698bfd2** /260,068,744B，mtime1793064349433100916，非复用06exe；两normal依赖重新freshfalse、新.d模块与实际genRust/Pending来源都保存。

唯一本次新父确实以`--ignored --exact`运行，PGID24507 exit101/39.185s，**0passed1failed0ignored**；停在flow_tests.rs:1336的setup门，未进入目标预算业务assert。真实diagnostic：`boundary=false actual_body=23856354 charged_body=23856354 pre_io_busy=5 seen_pin=true target_exchange=false visit_ms=30476`。因此fresh Seen123/Pintrue已进盘，deferred是真短metadata gate Busy5次且未发Exchange，但actual距32MiB仍 **9,698,078B**，不是最后1块窗口，不计第三真红。没有调限额/时钟/数据/重跑其它父，也没有擅自修生产。

phase已有具体而非猜测：10条单消息分别23–110ms，累计约0.9s；fixed spool4.961s/setup5.877s。实际4个main Exchange合计2.483s（各609–643ms）；68个Write合计20.171s，每Doc17次约5.0s、max864–918ms；19个Read合计7.105s（多数每Step约0.38s）。deferred5个Exchange preIO Busy合计<1ms。这里是端到端Step计时，含本地/receiver/调度/网络；不是底层fsync/import/hash占比，不能凭这张表就断言单一IO主因。所有phase/raw保留，可供root决定下一最小诊断。

source1084 before=after=current exact07，只有已授5mtime输入内容不变；actualNode exe、sync/catalog rlib/rmeta/new.d/generatedRust最后逐SHA/bytes/mtime仍一致。23725/24507均wait/reaped+signal0gone，无forceddrain，n1-chat07 TMP清零不存在；min外14,672,822,272B/内11,610,734,592B，无5GiB/3IO/STEP/TOTAL触门。final source/binary/dependency/group/cleanup true，但setup分类使execution_complete=false/runner exit1；不是资源红/编译红/skip或预算真红。首错结果即明确归还槽。

`Q039/node-transfer-n1-chat-budget-07-control04`与internal同名完整**58files**逐file SHA一致，含两个actual步骤、真实ignored执行raw、1084frame/14copies、sync.d/genRust、Node/deps所有SHA、phase/marker/pairedframe分析、5mtime/2PGID/cleanup/finalaudit。原06 Busy/Pending两真红与原Devices+Agents 7.6MB/30s setup失败继续分开保留。没有第三红就未获生产修复，07仍冻，旧4flow/11/N0/Pair此次均未跑。

下一仅提案而未执行：可对同源07 actualexe和现deps作**独立内盘owned TMP对照**，0Cargo/0source/0mtime、原32MiB30s/4.25MiB消息/同一exact完全不变，比较FS环境是否限制实际到界；新证据不能覆写本次外盘setup失败或原压力case。或者root另授窄store.load_nanos/阶段测量再定位。不得直接抬30s/预算、seed计数、扩大perchunk全仓优化或把只读设计当已实现。


### 同07 actualexe内盘TMP对照runtime01已准备（未launch）

root仅批准比较fixture存储位置，保07外盘control04的58files原始失败不改。prepared `Q039/node-transfer-n1-chat-budget-07-internal-runtime01`与internal recovery同名**6files**双盘一致，runner SHA **122d765483c2fe77d5727c90558e635b3933d74ca535e4e5974b0a89fc08a51b**（run/AST/从purepull runtime01最小delta/copy manifest/prepared来源与路径检查/otool依赖清单）。只有原07 newChat实际`--ignored --exact`父，0Cargo/0source/0mtime/0生成，原数据/32MiB30s/256KiB/near双向帧/SeenPin/公平/cleanup与target红条件均不变；setup再未达就停，不继续任意试fixture。

为免R1随后覆盖sharedtarget，实际07 Node2c977.../260,068,744B/mtime1793064349433100916已先**copy2**到内盘独立`recovery/runtime-artifacts/N1-chat07/debug/deps/khor_node-73af34573690fd43`，SHA/size/mtime逐项等04 actual origin；copy mode0555、nlink1且inode/dev不等source，没有hardlink或修改/删除共享target。manifest.json保origin→copy和原绝对编译证据。otool只见系统framework/dylib，无sharedtarget相对运行库依赖。后续运行path明确为此copy，不改04历史binary path。sync/catalog实际rlib/rmeta/.d/genRust当前仍逐SHA/bytes/mtime等04；编译来源只引用04/libtest-build原绝对path及证据SHA，不在新目录伪造build。

新独占scratch为`/Users/zgh/Library/Caches/khor-code-quality/tmp/n7`，单case TMP为其`c`子目录，**当前scratch不存在**；前缀逐级无软链、resolve同字面，st_dev与Users盘相同且不同disk0，绝不以短软链藏不同真实Root。Node实际handoff为link.rs:1307的loopback TcpListener，endpoint.json只是普通文件，QUIC/自有relay也无Unix socket文件名限制。保守最大PID例子endpoint文件path101B；util潜在tmux路径按最大UID算96B<macOS sun_path有效103B（实际UID501路径89B），所以不靠超长Unix socket造成拒绝/假红。

runtime复用purepull guarded控制但引用04实际新来源，07 14/1084current又逐SHA exact；03/04的5mtime也未再touch。两盘5GiB、300/720、TERM8/KILL5/wait5、全部known group最终退才清TMP、最终源码/copy/deps稳定及双盘核仍保。这里只准备，没有Popen测试/active/PID或native占槽，待root短审另授；若内盘达到真实target红，仅说明该同源case在内盘到界，不能回填外盘setup失败/原Devices压力性能或N0绿。


## 同07内盘对照仍setup未到界：44双盘收口，停止继续换fixture

root短审runtime01与prelaunch后授唯一native槽，现场再核122d script/1084/currentcopy与6depSHA bytesmtime、内盘scratch真实无软链且不存在、无compiler，外14,798,450,688B/内11,295,170,560B。runner33764/session70129，只actual immutable内盘Node2c977 copy执行原07新Chat`--ignored --exact`，唯一PGID33805 exit101/38.355s，恰0passed1failed0ignored。0Cargo/source/mtime/gen/数据/限额/oracle改动，没有其它父/新fixture重试。

实际marker **boundary=false actual_body=charged_body=24,904,930B pre_io_busy=6 seen_pin=true target_exchange=false visit_ms=30589**。仍距32MiB **8,649,502B**，未到near窗口，停在同setup门；不是第三业务红。相对外盘04只多传 **1,048,576B=4个canonical chunks**，不能凭换内盘把旧外盘失败变绿。10条消息写入/固定spool均真实完成，spool4.590s/setup5.485s；4main成功Exchange合计2.387s，68Write合计19.250s，23Read合计8.293s，deferred6真preIO Busy合计<1ms。读写仍端到端时钟，不能凭这个对照断定物理IO的主导占比或完全排除IO。

最终source14/1084 before=after=current exact07，04原5mtime仍精确未变；immutablecopy260,068,744B/SHA2c977/mtime、实际sync/catalog rlib/rmeta/.d/genRust全部再次实核稳。33805已wait/reaped+signal0gone，无forceddrain，内盘tmp/n7根及c私有caseTMP清零不存在；min外14,795,919,360B/内11,188,195,328B，无5GiB/3IO/STEP/TOTAL触发。源码/copy/依赖/组/cleanup各true，setup分类正确令execution_complete=false/runner exit1。native槽在结果获得后明确归还，后续只有只读核据。

`Q039/node-transfer-n1-chat-budget-07-internal-runtime01`与internal同名完整**44files**逐file SHA齐，含明确prior04编译绝对来源/owned copy manifest、actualOneParent、1084frame/14copies、internaldev/path、phase比较、copy/deps最终证明、PGID/guard/cleanup/finalaudit。另逐SHA复核外盘04原58files依然外内匹配，未回填历史body/phase/路径。原06 Busy/Pending两真红、原Devices+Agents 7.6MB外盘setup失败、07 Chat外盘23.8MB和本次内盘24.9MB setup失败继续分别记账。

按root本次指示，**不再继续任意调整fixture/存储位置试到窗口**；没有第三真实budget红就不自行开始生产修订。下一步由root在production性能诊断/最小改进或明确单独纯budget policy回归之间选择。后者若获授必须诚实标纯policy，不冒实际Node到32MiB；原4/5flow、原N0/max4/25MB/VV/SeenPin、32MiB30s都保留。只读Usage/partial-Exchange方案已在上节，尚未实现，不宣称N1完成。


## candidate08：真实生产预算谓词纯policy red源准备（行为未修，未编译）

root在07外/内盘真实near均未达后明确决定：第三缺口先以真实生产谓词的普通单元红验证，**release整链真实32MiB网络边界仍待验，不把policy红冒实Node到界，不缩最终要求**。08只改既有`link_doc_transfer.rs`，把原break条件语义等价提取为private `fits_step_budget(starting,moved,quantum_chunks)`：`(!starting || quantum_chunks<=2) && moved+CHUNK_BYTES<=VISIT_BYTES`，调用改为`if !fits_step_budget(!item.started,moved,quantum_chunks){break}`。外层bytes/chunks/time/全部4量子条件不变；guard仍错误允许opening余1块，故这不是功能修复。

新增唯一普通纯父`link::documents::budget_tests::an_opening_reserves_two_chunks_before_sending`，不ignore，不操作任何live runtime counter/clock/RPC/Step返回：先控已started单向余1块且q3可走，opening余2块q0/q2可走，opening q3不可走；最后断言opening余1块必须拒绝，预期当前Rust实际运行0passed1failed0ignored并给准确目标消息`opening Exchange requires two canonical chunks of visit headroom`。纯参数边界case不是对live计数seed，也不是actualwire已到32MiB证据。当前尚未编/跑，不能预记红。

双盘`Q039/node-transfer-n1-budget-policy-candidate-08`与internal同名**23files**逐字一致，14源/1084输入：manifest **ebffe36de46e5f0045eaea0e2b3bc16fd62f7b9bf2126818f8955d708f542751**；fullpatch **622ea2e5f990f33f578a5c9f0681e0c897b7a2649f2c3cef71a4c3a1267e6970**；07→08最小delta **c2bf7d1ca9fd298f4e50dbb1fd1fc477de43ad67598130d7ee9884890ffba996**。除这一源外1083全等07，包括5重flow整个源码/ignore属性、N0/Pair04/11、wire/helper/库/catalog/link。没有StepOutcome/Usage/真实reserve修复，不碰serve_as。

production-equivalence记录原逻辑的DeMorgan/同分支短路关系，机械去掉新函数及cfgtest模块、逆还call后**07原文件逐byte/SHA精确相同**（inverse07源已存）；100个正常边界算术sanity仅Python静态辅助，不冒Rusttest/typecheck/业务结果。临时index fullpatch适用、rustfmt check/diffcheck通过，生产index/HEAD5f2未变。原07 source/外盘58/内盘44/06两真红全部历史保留，不回填scope。

当前R1A8排native，本作者只source/prep、无Cargo/native/生成/测试/commit。root审08/另授槽后才可跑这一个真实policy父；采到此policy红后才另外授权一次最小known/uncertain Usage、partial Exchange Pending与双向reserve修复，并按原Busy/purepull/units/N0/release真实网络预算分开验收。旧压力/实际near失败绝不当CI绿，heavy release选择仍由root明确。


### policy-red08 control05已准备（未授槽/未launch）

按root指令只复用control04：candidate改08、唯一普通父改`link::documents::budget_tests::an_opening_reserves_two_chunks_before_sending`，**不--ignored**；只current Node cfgtest compile一次→这一ordinary，旧5heavy/11/N0/Pair不跑。`Q039/node-transfer-n1-policy-red-08-control05`与internal同名**3files**双盘一致，runner SHA **1637cd50c3d3a35cee2b31f1b8c9fb1496ffe40b33de6628168c990bab44d564**；附04最小delta与AST。

继承300/720/5368709120和TERM8/KILL5/wait5、同5精确mtime、current Node/sync/catalog provenance/.d/genRust实际内容、source/binary/dependency最终稳定、全knownPGID退才清TMP与双盘。新短TMP n1-policy08。pure父无Node/IO fixture，故**移除flow cleanup marker要求，不伪造它**；真实group/TMP/源/产物仍由runner独立核。只有实际0/1/0、准确target消息`opening Exchange requires two canonical chunks of visit headroom`且4个控制先通过才记`budget-reservation-policy-red`。0tests/list/skip、编译/setup/guard/清理失败不红，不以此冒物理网络32MiB。

准备后再次核08 source23files双盘每SHA、07 inverse逐byte等原、actual选定14源/完整1084path set与全部SHA仍exact08；control05三files双盘一致。未写Usage/协议/helper功能修复、无Cargo/native/生成/测试进程或commit；原两Chat setup原件与physical32MiB release待验保留。R1A8优先槽，等root短审另授，不自动launch。


### policy红之后功能patch最小3路径收口草案（root认可方向；仍只读）

补正此前两路径草案：实际proto::answer在decode后就cap/validate_reply，未知doc/非canonical等可能在doc_exchange之前丢出Err。root实读后认可以下**3路径**，待实际policy红才另授实现，08目前14/1084全部SHA仍exact。

1. **proto.rs**：原answer体提同一个private inner，普通answer原流程/返回不变；增加crate-private Doc-only observed入口，传`&mut Option<usize>`保存解码后可确定的Doc body总长度（None=未能decode，Some0=已decode无body）。同一次现有Response decode后立刻观察，之后原raw cap/validate_reply/reword一行次序不减；没有通用callback、第二decoder、新wire字段/DTO或RPC/retry。这样unknown-doc/shape错误仍可保known RX，真正decode失败才unknown。
2. **link.rs**：窄private DocAttempt只保存started/request_written/rx_observed_length，非serde/协议类型。原exchange_on与exchange调用方不改行为，共享原唯一encode/open/write/read/liveness/RetireUnheard实现；Doc-only入口传可选receipt，encode成功后、open前started，write_all成功即TXknown（finish/read失败也保），读回后经上项Doc observed入口保存RX。普通请求None走旧answer，无新连接/重试；不动serve_as/其它caller。即使exchange返回Err，receipt在caller栈仍可读。
3. **link_doc_transfer.rs**：private `StepOutcome { usage, result: Result<(),Error> }`配局部Usage，bodyRPC统一`doc_exchange(...,&mutUsage)`，先把receipt converted knownTX/RX与unknown合法方向保守charge写入Usage，再返回typedErr/校验roles/expected doc/manifest/offset/epoch，再receipt/apply；控制ACK调用方也传同样局部Usage，正常为0。transfer_step和receipt-flush都经owned局部Usage封口，outer先提交known/uncertain预算、再处理typedresult（含Busy/hard/Apart），不把?后的已传数据丢掉。public moved只known；budget/quantum=known+uncertain，已知正常Busy帧仍moved==actual。每次相同id/offset重发重新计，别按成功apply/唯一offset去重。

初Exchange保持opening状态到所有本地后半段成功：先验证并保存双方manifest/push/pull pending，再receipt→pushState→reverse receive；预先记录remote/local Missing的needs_exchange信号，末尾started只在全成功且!needs_exchange提交，不能late true覆盖Missing。Busy必须Pending；硬错保持首doc错误而非假完成，Apart明确epoch隔离，原fixedReady/持久receipt/幂等恢复/GC边界不改。不得引入第2存储扫描/unchecked body读取或另外planner。

预算patch以已提取实际fits_step_budget为门：starting预留2canonical/2chunk，started Write/Read1canonical/1chunk，visit余量和quantum bytes/chunks同时满足后才允许attempt；local preIO失败0，已发生与uncertain方向先charge再看结果，所有原32MiB/30s/4chunk/SeenPin/registry公平不变。body大小/未知非法回包的区别沿前表，不把保守attempt冒known moved或Applied。

尚无实现diff/新fixture/新控制框架/Cargo/native；现control05已准备待R1A8与Q052收尾后另授。实际purepolicy红不代网络到界，原外/内两setup原件与release32MiB/N0最终要求均保留。


## source08 policy真实红已收口；槽已归还

root授唯一槽后只运行control05（runner1637cd50…d564），runner91833/build PGID91872，当前Node libtest compile0/67.885s；唯一ordinary exact PGID92617 exit101/0passed1failed0ignored，实际停在link_doc_transfer.rs:1057目标`opening Exchange requires two canonical chunks of visit headroom`，四个前置controls已通过。分类仅**budget-reservation-policy-red**，不是产品绿/网络32MiB到界证明；原两Chat setup原件与实际release整链缺口保留。没有重跑或弱化任何原父。

actual current-tree Node新exe SHA **9dba43f57d00f91c6151ddcb35a6b9f3671842dae506705064ff788dec1268a6** /260068952B/mtime1793071967809238499；Node/sync/catalog compiler-artifact fresh=false，actual sync新.d含outbound/receive/version，actual catalog genRust/Pending函数保存并核源。14源/选定1084 input before=after=current，实际exe与6依赖/生成文件SHA/bytes/mtime最终一致；91872/92617均reaped且再次killpg0 ESRCH，无forceddrain，owned TMP n1-policy08真不存在。min外14622851072B/内10946506752B，300/720/5GiB/3IO均未触门。

control05完整**58files**双盘逐file SHA核齐（含final-audit与policy-red-analysis；以actual目录计数为准），runner execution_complete=true表示已采集预期红，不冒product green。结果获得后明确归还唯一槽，只补审计与本卡，不再Cargo/native。root现另授既定最小3路径功能修复，接下来source-only实施后冻结非作者审；没有先行编译权。


## candidate09：三路径功能修复已源冻结，等待非作者审（无编译权）

实际policy08红后root明确授权最小3路径，现`Q039/node-transfer-n1-usage-pending-candidate-09`及internal同名**23files**双盘逐SHA一致。仍14源/1084选定完整inputs；相对08只有`link_doc_transfer.rs`、`link.rs`、`proto.rs`三源，其余1081字节全等，5个重flow/原budget父/11/N0/Pair04及state/core/catalog/锁与依赖均未改。manifest **5ffdfa9e26d3783a314466dfbadbd3db0be790315f07b6d6020315ded980e126**，08→09 delta **881ee330280b793e4052efaa202ba99a248a3fd60209c9174fb2f5532d126b6f**，fullpatch **abd2dfd0fb9817483178277ea3751e8eacee77f1f2585e3aeb655b318764e8f1**。HEAD仍5f2，实际index不改，未commit。

`DocAttempt`只记录单attempt：encode完成后/open前started，write_all完成即request_written（先于finish），经原唯一QUIC/liveness/RetireUnheard路径；普通入口None共用同inner，无wire/RPC/retry/framework。`proto::answer_doc_observed`用同一次decode先保存Some(bodylen)，包括Some0，然后原raw cap/validate/reword；普通answer仍同流程。decoded非法roles/wrongdoc/epoch/receipt/localapply错误之前已保known RX，decode失败只保unknown。真正dial/本地validate失败未attempt为0；open/write/read未知按合法方向保守budget charge，绝不冒public moved。

`StepOutcome {usage,result}`让Busy/硬错/后半段本地错误保量；outer先提交known+uncertain visit/quantum，再处理typedresult，重复相同manifest/offset请求逐次计量。public moved仅known write_complete TX+decodedRX；Exchange/Read unknown回复最多预留1canonical，Write/metadata正常回复0，非法已decode body保actual不截小。receipt flush也传同一Usage，正常ACK/control0body，不遗漏异常回包。

opening调度必须同时符合visit与quantum bytes/chunks的2canonical预留，started单向1canonical；32MiB/30s/4chunk1MiB/256KiB/64MiB都不改。初Exchange先校验保存双方manifest，started保持false穿过receipt/push/reverse receive；全成功最后提交，显式保存received/local Missing信号，不能late true盖掉需重新Exchange。初receipt Busy即Pending，固定plan/幂等收据/精确GC与Loaded/owner await边界不改。

freeze附review-notes、validation-plan与compatibility-audit。机械移除proto wrapper/observer可逐byte还原08，serve_as到exchange_on前区段逐byte全等08；CRG仍67927旧图而HEAD5f2，0 flows不当覆盖，已落实际调用。3源rustfmt parse与完整patch/index diffcheck过（仅语法/patch，不冒Rust typecheck/测试）；只有link_doc_transfer格式化，无GUI/生成/Cargo/native/新增fixture。

下一交root+node_test_review非作者源审后另授：normal Node check；原11+budget普通父；原quiet/Busybody/purepull独进程真ignored exact；原N0最多4visit/25MB/VV/SeenPin保持。并非现在launch，未将任何旧绿冒09同源。actual网络32MiB release边界、postflush/prereceipt kill/reopen、N2大Pair/Vouch依旧未全验/未实现；原压力与外/内Chat三个setup原件继续保留。


### 09最小回归清单已准备：17 libtest父→原N0，未launch

按root新增ordinary proto2授权，prepared `Q039/node-transfer-n1-green09-validation-plan`/internal同名2files逐SHA齐：plan.json **4688f9b2ac4afe046cbc85a1a317cd651135752bd0beedb643ccab3660d2780c**，plan.md **9b8e47a37616edd510904b221d5e26e4e4aafcf309130b2d814834a9ee074ec9**。全文列每个exact/实际命令：一次current Node --lib --no-run → policy1+原unit11+ordinary proto2+quiet/Busy/purepull3，各actual libtest独立进程精确1/0/0/exit0，只有heavy3用--ignored；3真实flow须原cleanup marker，普通14无假造flow marker。17普通/ignored父真实执行才可记数，不靠list/skip。

之后才原N0 `cargo test --locked --offline -j1 -p khor-node --message-format=json-render-diagnostics --test sync_doc_isolation a_large_devices_gap_converges_through_real_node_transfer -- --exact --nocapture --test-threads=1`，仅一个integration target/普通父，真实child不另计；25MB/max4/VV/SeenPin/30s/所有oracle不改。A 300/720/5GiB，B300/420/5GiB，复用control05有限PGID/actualdep/source/doubleaudit控制及既有11/03真实case gates，不造新fixture/框架。未提出额外normalcheck编译，N0 target须真实当前normal Node来源。

本次未跑debug两个quota setup未到界父、不重跑B1/Pair/旧绿充数；实际32MiB留release网络验证，crash-window仍独立未完。只是计划/命令准备，尚无新runner执行或编译授权。当前再逐SHA核09完整1084仍exact，源码/测试未改。


### 09可执行控制已冻，等待唯一槽（无launch）

root与node_test_review source09独审无确认阻断后，按授权只把既有control05/11/03 gates收成两个runner，无源码/测试/fixture变化、新框架或额外证明suite。`Q039/node-transfer-n1-green09-control06/run.py` SHA **1e2e998e02b7eebe15ad2b2d7c500dbd019827bb427fb8cf3321ae986bca7d45**：一次Node libtest构建→plan所列17父（policy1/原11/proto2/真quietBusyPurepull3），普通14不--ignored、flow3实际--ignored --exact。各exit0且精确1/0/0；heavy3须原真实cleanup marker，Busy marker需busy>0、actual=known moved>0，purepull需first_pending=true/target absent/reopened target=true。18execution步骤不冒18父。

`Q039/node-transfer-n1-n0-green09-control07/run.py` SHA **07f9594bd6955d3c75de0679bc1bfc5838f33a2a109fdeafae6acfeeea9635ab**：仅原N0 exact，一个integration target；先强制control06完整18步骤/17父全绿且before/after1084同当前09，保存前置receipt真实SHA。当前normalNode lib+originalintegration test须fresh=false/本树manifest/src_path，actualnormalNode hashed.d含当前link_doc_transfer/state/proto并核SHA，真实Node rlib/rmeta也纳最终依赖稳定。原N0所有大小/Seen123Pintrue/max4/VV/content断言不动；worker与父各真实1/0/0，不把worker当第二ordinary父。

两controls各**3files**（run/AST/相对旧control05delta）双盘逐SHA齐，AST静态核实际300/720或420/5368709120与TERM8/KILL5/wait5并有真实Name load引用。沿旧source/provenance/guard/全部PGID/TMP/当前actualexe+deps最终稳定收尾；短TMP n1-green09与n1-n0g09，保继承HOME与5个已证差异输入的精确mtime刷新（字节不变），不广touch/清缓存。当前1084再次全SHA exact09，无active/results/编译或测试启动。

明确30s只管**下一Step准入**，不是整个visit硬deadline；合法2/1canonical预算预留不宣称能硬限违规peer的raw字节，非法已decode Doc body仍先known计再拒绝。实际网络32MiB留release验证，两个debug已知quota setup失败父不重跑、不改限额冒绿。现等root控制审/唯一槽另授；R1仍由root协调串行。


## A09实际首跑：15父绿，Busy公平旧断言失败即停（purepull/N0未跑）

root授唯一A槽后，仅冻结control06/09source实际运行。runner14986/session52751；build PGID15069 compile0/67.796s。actual Node新libtest SHA **d117ca81fdfb73f2cb3068b9f468c49818c5fc57b9e1bbd2dee9e4cf215e514d** /260076184B/mtime1793073766946312833；current Node/sync/catalog freshfalse/actualdepfile/genRust内容门全过。policy1+原11+ordinaryproto2共14ordinary各1/0/0；quiet实际--ignored exact1/0/0/3.253s，真实cleanup过。

Busy flow-02 PGID16100 exit101/37.316s，actual0passed1failed0ignored，停doc_transfer_flow_tests.rs:896 `real Busy Doc must not skip its independent sibling's turn`。已按首错停止，未到最终accounting marker/assert，不能预记Busy accounting绿；purepull尚未执行，B原N0未授未跑。runner分类compile-setup-or-unexpected-failure/exit1/completefalse诚实保原。此失败不是原quota near未达setup门，独立SeenPin/reverse fullchunk/真实wire完整性前置已先过。

只读actual trace定位：前10个devices/agents quantum严格交替至order140 Agent；reverse Agent最后reply order149 offset2097152+43164恰固定manifest2140316末片，sender AfterRead order150 failure=None。随后43个Quantum只Devices（151起），触无条件整visit相邻不得同Doc的旧oracle。Work.done原语义允许已完成独立Doc退出；09已把Busy真实已传量算advanced，故兄弟完成后剩余Busy Doc可继续合法尝试。当前只交root判断是否将公平oracle限定真实双方pending窗口，不自改测试/生产。receiver原trace可算17388042B/70个body events，但最终accounting断言未执行，不冒其已通过。

actual17PGIDs（compile+16执行父）全部wait/reaped且再次killpg0gone，无forceddrain；ownedTMP n1-green09真清零不存在，1084 before=after=current09/实际Node+6deps SHAbytesmtime稳，min外14214262784B/内10617839616B，guard未触。结果获得即明确归槽，无修源/重跑。control06完整**238files**双盘逐SHA齐，含完整raw/源frame/actual产物/provenance/cleanup/final-audit与busy-failure-analysis。原red08/06及两Chat setup历史不改，原N0/30s32MiB等合同不放宽。


## candidate10 fixture-only冻结：真实双方Agents完成证明限定公平窗口

root读A09实际raw后只授fixture最小修订，已冻`Q039/node-transfer-n1-flow-fixture-candidate-10`/internal同名**23files**逐SHA齐，14源/1084。仅`doc_transfer_flow_tests.rs`变化（SHAfa4d9b6391d4c1ffda321ece55f064ae4fe90b73b84c9906f71990a899a7b66d），生产三源与其余1083逐byte等09；manifest **7f7d8fa8be85ffa179f8668f66800eaa61d9895fce8b6eea97cf8b6f1f899634**，09→10delta **1f12553a7a47bb939befdf0e0d7f57120eb3619bd90c311ae6cfff4466913c50**，fullpatch **1dd63bcb1f443228cfe803ec10339cad5db4583daa1706632b6865a856ff3601**。

复用现有After(agents,Read)暂停点，不添生产hook。Probe::note只返回所记录Entry的同一个actual CLOCK顺序，Pause带该order；bounded registry/events/manifest及overflow失败保留。Busy controller在该真实成功AfterRead暂停时，用既有target_present读取A/B两端的fixed Agents target（含epoch与VV覆盖），只在两者真覆盖时记录完成order。不是按最后一个AgentQuantum/字节数猜Work.done。

fairness仍先要求两个Doc各得真实首轮、共同pending prefix严格D/A交替。无完成proof则仍整visit严格；有proof还须该order匹配实际成功Agents AfterRead Entry，只有之后才允许Devices-only尾段。Seen123Pintrue独立、真实逆向fullchunk、wire完整/4chunk量子、busy>0、最终charged==actual与quota全部原断言保；其他4个heavy父body逐byte等09，原数据/限额/ignore入口不变。

rustfmt parse/check、完整patch diffcheck、delta反向apply-check通过，只是source检查不冒typecheck/runtime绿；已给root+node_test_review非作者delta审，无Cargo/native/commit/重跑。A09原15绿+Busy断言失败/238dual历史保留，不回填计量绿。下一待独审另授仅重建cfgtest+原Busy/purepull2；已过15不重跑，N0尚待后续。


### candidate10最小resume控制已冻（不执行）

root源审通过并待另一独审收口时，按明确授权只准备`Q039/node-transfer-n1-green10-resume-control08/run.py`，SHA **76329f5dc527adfb1182ab1163270445e272dc06d94b243f0e61168f526b4873**；run/AST/相对control06delta共**3files**双盘逐SHA一致。现source10 14/1084逐SHA仍exact。只1次必要当前libtest构建→原Busy/purepull2实际--ignored --exact，各原1/0/0/exit0、accounting/quota/cleanup markers全保，当前Node/sync/catalog/.d/genRust来源与最终source/artifacts/deps/全部有限group回收门沿用control06。300/720/5GiB与TERM8/KILL5/wait5不变，没有新fixture/通用控制框架。

前置只继承原A15真实绿：验证原17execution结果前16=build+15绿、末条原Busy确为旧fair断言失败，原completefalse不回写；原source/artifact/group/TMP收据真全，当前比原source09只flow1个input不同/其他1083精确。不重跑15/原debug quota setup父/N0/Pair。原N0 B07历史冻结不改（其A全17前置现在仍不成立），resume两父绿后由root另授最小承接当前10/继承15+新2的N0控制。

暂无native执行授权、未Cargo/启动/mtime/测试。30s仍准入门非全visit硬deadline，合法2/1reserve不冒违规peer raw硬cap；原A15绿+Busy旧断言失败/238dual原件、10源码23dual都保留。


## resume08真实结果：Busy全绿；purepull合并setup/control失败即停

10两位独审/控制root接受后授唯一槽。runner26090/session31584，build PGID26123 compile0/67.834s；新actualNode **bf59cd7141e4f548cdbcc7bc197db30b532469dabcca9e88253dd4464962d4a7** /260101832B/mtime1793074879786842791，本树Node/sync/catalog freshfalse及actual.d/genRust/源门通过。只重建+Busy/purepull，未重跑继承15/N0/Pair/debugquota。

Busy PGID26839 exit0/36.742s，**真实1/0/0**；42次真实owner Busy，**actual_body=known charged_body=15815178B**，SeenPin独立真，Agents实际双方固定target完成proof Some(150)。原反向全块、wire帧与四chunk量子、共同pending公平/完成尾段、最终accounting equality与quota、cleanup全通过。不能将此15.8MB冒32MiB真实边界已测。原A09的Busy旧公平断言失败/15绿历史不回写。

purepull PGID27198 exit101/4.248s，实际0/1/0，停1215合并`setup/control: release and fresh reopen must genuinely complete the fixed reverse target`，没有到最终Pending断言，不能计其绿/原B2红。runner首错停，B原N0未跑。真实raw在fresh reopen之前已有：首Exchange reply14→本地receipt Busy After16→同visit第二Exchange reply21→Read reply27(offset262144/262144B)及reply32(offset524288/21658B最后片)，After33成功→Confirm35/36→Versions38/39，同固定manifest c0d130…、总545946B。

只读因果：fixture在首次Busy AfterStep即drop(gate)+release再等首visit返回；09已把收到Busy body计advanced，故释放后同visit可合法重试/完成，原合并setup却先要求absent_first=true。实际first/present_after/Spec子条件没打印，不能捏造其值；原完整末片/成功AfterRead/Confirm强支持“假设一次Busy必须终止visit”的fixture过约束方向。已交root判断是否只把真实gate持到首visit返回后才drop/fresh reopen，未自改fixture或生产/重跑。

3PGIDs26123/26839/27198均wait/reaped且再次signal0gone，无forceddrain；ownedTMP n1-g10r真absent，1084 before=after=current10、实际Node与6deps SHAbytesmtime稳，min外14120796160B/内10638184448B，guard未触。结果获得即归唯一槽，completefalse/runnerexit1保原。新完整**71files**双盘逐SHA核齐（final-audit/resume-result-analysis含actualsource/binary/deps/group/TMP与原A238原件未改证明），没有追加测试/清缓存/改源。现在继承15+新Busy1共16功能父绿，purepull待修fixture或诊断、N0/release32MiB/crash/N2仍未全闭。


## candidate11：收费与逻辑进展分开，只删advanced OR（无runtime）

root复核10实际42Busy及purepull真实trace后决定不移动purepull gate/不改fixture：09把失败流量charged当advanced会使仅剩Busy Doc继续整visit空转，真正修订应恢复原仅Ok推动外轮语义。前节fixture方向只是当时只读推测，不能盖过root本次决策或据以改断言。已按授权**只删除**`advanced |= step.usage.charged() > 0;`一行，usage/quantum仍在任何typedresult处理之前收费，Busy/硬错不丢已传量；advanced仅Ok=true。

source-only `Q039/node-transfer-n1-busy-yield-candidate-11`/internal同名**23files**双盘逐SHA齐，14源/1084。只有link_doc_transfer.rs变化（SHA1d0c026ccef66b0c0aa2e645c148341ab37c5922c94043b468c13956e907a2b7），其余1083精确10，含purepull原gate/body/10真实VV完成因果公平检查、所有原数据限额及schema/元数据/收据协议。manifest **e30e91fe4fe967ddc907d84f2410f69c51d4037d219d3ae0a267e11f6463d5f3**；10→11delta **ae15907895706db4f504cd43b6474734c92beab52654aefc345f25c243c596fb**；fullpatch **fa23e4c9bfafc42020b4db7990a558e7eabcbb47844a57566841429412b9e48d**。

反例核：只Busy时当前量子break，未Ok的外轮结束，work仍未done而Pending；健康独立Doc仍以Ok推进并给Busy Doc合法后续轮次；最后sibling完成的成功轮后最多再一个Busy-only轮即退出；本地preIO Busy为0但仍Pending；硬错先收费再记固定首错而独立Docs继续。原metadata flush/immutable plans/2-1 reserve/32MiB/30s准入/4chunk等不动。未确认正确性阻断，真正结果待运行。

机械补回一行可逐byte还原10源，其他1083全SHA精确；rustfmt parse/check/patch/reverse-check过，只是源码检查，未Cargo/native/commit/重跑。已交root+node_test_review独审；10 Busy绿+purepull控制失败/71dual、A09历史238dual保持。共享生产变动后不预记旧15/Busy绿冒11全同源结果，下一套验证范围由root独审后明确授予。


### 11重新全17验证控制已冻（无launch）

11生产一行变更已双审无确认阻断，root选择重新全17、不做旧15跨生产继承证明。`Q039/node-transfer-n1-green11-control09/run.py` SHA **dbba85a038183ac670056c84749af40dbd7f15d4ff822cfac4e5918ee7d8cd17**，run/AST/control06最小delta共**3files**双盘一致。runner相对已执行control06只candidate09→11、输出/internal目录及短TMP n1-g11替换，1必要build+原17 distinct exact/14ordinary3ignored、所有绿色cause/accounting/quota/cleanup分类原表达式AST完全相同；300/720/5GiB/TERM8KILL5wait5/source与actualdeps/产物/PGID/TMP稳定门原样。没有08部分继承/旧Busy失败特殊门、新框架或新增fixture。

源码11当前14/1084再逐SHA exact，无Cargo/native/mtime/测试启动。此回不借旧15/10Busy绿称11同源绿；全17实际绿收口后才交root再授原N0。原A09/10失败与旧运行证据全部保留。


## candidate11当前同源全17实际绿，已正式归槽

root明确授green11-control09唯一槽后执行，runner32407/session10742，build PGID32542 compile0/82.035s。一次当前Node libtest构建后，原policy1+unit11+ordinaryproto2+quiet/Busy/purepull3 **17 distinct父全部实际1/0/0 exit0**，普通14/真ignored exact3，18execution steps/complete=true/runner0；没有继承旧source绿或重跑已知quota debug setup、N0/Pair。

actual新Node **9d29ed3752ce0f639402fecd1f91b61a348fee6afa8553e24cc4cc9ea1a96699** /260101896B/mtime1793075580790104749；Node/sync/catalog本树freshfalse、actualdepfile/currentoutbound/receive/version与catalogPending生成内容门过。Busy6次真实短owner冲突，**actual=known charged6377994B**，SeenPin独立、Agents实际双target完成proof Some(150)、原公平/逆fullchunk/wire四chunk/计量/quota均过，21.525s。purepull原gate时序不变，实际first_pending=true/first_target_absent=true/reopened_target=true，原同manifest/epoch/VV/精确Spec/第二轮非Pending全部通过，4.230s；quiet实际一次Versions/3.265s。

删除错误advanced OR后Busy仍收费，而无Ok外轮终止Pending，原purepull真正恢复收敛合同已得实证；不是靠改purepull门制造绿。旧06两真红/08policy红/09公平旧断言与10空转control失败完整保留。仍未把本次6.4MB视为actual网络32MiB/release边界，也未声称crash窗口、N0或N2完成。

1084 before=after=current frozen11、actualexe与6actualdeps SHAbytesmtime最后一致；18PGIDs全wait/reaped并再次signal0gone，ownedTMP n1-g11真不存在，无forceddrain/guard。min外14096736256B/内10577059840B，300/720/5GiB全保。complete=true包含源/产物/依赖/组/TMP真实闭合，结果获得即正式归槽，N0未自启。完整**250files**双盘逐file SHA核齐，含final-audit/regression-result-analysis与全原始actualsteps。后续root审实际17与收尾后才接原N0当前正常Node验证。


### 11 B原N0最小承接已冻（未launch）

root明确只授准备后，复用旧control07精确runner，仅candidate→11/前置A→green11-control09/输出internal及短TMP目录切换。新`Q039/node-transfer-n1-n0-green11-control10/run.py` SHA **fd0470ed64b7aab1080fd92ea1b3a47c7fdb3cea4f9bcec7a80168b43a0a78f1**，run/AST/control07最小delta共3files双盘核齐。原唯一sync_doc_isolation exact普通父、当前normalNode/sync/catalog/actualintegration来源门和所有N0 body/max4/VV/内容/SeenPin/大小/资源/收尾断言保持，不加新门/框架/Pair/releasequota/重复17。AST300/420/5GiB及TERM8KILL5wait5真实引用，源11 1084仍exact，前置A17 complete真且source同源，尚无Cargo/native执行。


## B11原N0真实25MiB差集第4visit收敛绿，正式归槽

root实核A11全17原raw后授冻结control10唯一槽。runner36267/session2725，唯一Cargo/ownedgroup36304；normalNode+原sync_doc_isolation当前真实重新编译46.24s，原ordinary父36753与真实ignored worker36754均1/0/0/exit0（同一个ordinary验收父，worker不另充数），worker111.61s，runner158.955s/complete=true/exit0。实际正常Node freshfalse/本树manifest/srcpath、normalhashed.d当前linkdoc/state/proto与rlib/rmeta、sync/catalog actual模块/genRust来源全部门通过，不借CLI或旧/R1binary。

真实prepared **raw_gap=25447950B**/epoch1/1/max4，data/body/全部原断言未改。visit1真实已传6816274B，**首Seen123/Pintrue已fresh在盘**且固定target/content尚未覆盖；visit2 8912896B、visit3 8126464B仍Pending devices/receipts，SeenPin保；**visit4 2379278B后固定source VV覆盖=true/receive_dir内容匹配=true**，原固定2rows/sourceepoch oracle通过，cap_rejections四轮全0。name/addrs原仅诊断，不额外钉死合法后续地址状态；原N0 max4/25MB/VV/SeenPin/所有operation fuses保。

actual当前integration **21061609c6e13c7c3604f78e0ea9ebe33aa5371dbd4927702893e3bd6b45a2a8** /197491744B/mtime1793075933809343083，1084 before=after=current11、actualtestbinary和**9个actual依赖/normalNode hashed.d/生成文件**最终SHAbytesmtime一致。36304组wholegone/reaped，刚实际观察36753父/36754worker亦再次kill0 ESRCH；TMP n1-n0g11真清不存在，无forceddrain/guard。min外13751148544B/内10561810432B，300/420/5GiB未触门，结果到即正式归槽，无Pair/releasequota/17重跑。

本B完整**49files**双盘逐SHA实核齐，含全部原raw/currentnormalNode和sync/catalog/integration来源、前置A17历史SHA、sourceframes、后验group/PID/TMP、final-audit/convergence-result-analysis。原旧cap N0真红/03Busy失败、09/10/policy/Pairsetup与29MBPair烧票红历史全部保。当前11达到A17真绿+原N0真绿，**仍不代表真实32MiB网络release边界、postflush/prereceipt真实kill重开与N2大Pair/Vouch完成**；不声称全N1/governance goal完成，无commit/集成/发布行为。


## N1当前实现提交/任务卡写权交回root（2026-10-27）

root独核B11真实收敛后明确授权提交，已提交 **d7a4192dfde6e589005031e4a91983beee68d5b3** `feat(node): sync documents in chunks with reliable accounting and pending state`。只candidate11 exact14实现源码路径；逐git blob SHA与冻结14及当前14全同，选定1084也全SHA未变，branch fix/quality-node-doc-transfer **clean**，parent仍5f2d5cf7。不带docs/证据/生成/库cherry变更，未main/ship/集成。原dd60回归与lookup父由root按已集成686f关系处理，不重做库集成。

`Q039/node-transfer-n1-commit-checkpoint-11`/internal同名2files（commit receipt/git show stat）逐SHA一致，记录exact14/tree/clean/sourceframe。当前checkpoint验证为A11同源17distinct真实绿+B11原25,447,950B/max4/固定VV与内容收敛真绿，全部原raw/失败/资源/来源/cleanup收据保持。actual32MiB release网络边界、postflush/prereceipt SIGKILL fresh恢复、N2 Pair/Vouch大bootstrap仍未完成；标题/提交不声称整个Q039或N2完成。

本卡写权现在**交回root暂接checkpoint**；作者后续不再写本卡，除非root重新明确移交。下一仅只读准备真实postflush/prereceipt SIGKILL最小方案，无源码变更或Cargo/native。

## 当前N2执行恢复（2026-10-27，root）

N1 source11/crash14、trust15与root Q054都已集成；组合run04的正常CLI/Node＋61 distinct实际通过（333.684s/81dual，root核），包含原flow quiet/Busy/purepull、crash sole业务父及trust18。paired另target、真实release32MiB边界和整App不在该61中。

node_transfer_owner已返唯一native槽，现回作者树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery@27093f4a4ff94afcdffa36aa4cd6143e11ecaaad` 实作本卡已批准N2完整契约：大Pair04与烧票恢复、durable caller/issuer claim-intent/root fence、bounded devices与signedRoster/Vouch，旧Khor wire无fallback。不得用另一轮纯types/plan提取代替垂直修复。允许Node link/proto/trust/lib/doc_transfer_state(+io/registry)/link_doc_transfer及必要private pairing模块/tests；不改a2a/R2/GUI/依赖，Q054 typed gate行为保留。先coherent源+原红oracle/恢复fixtures，root独审后统一授Cargo/native，不先运行或扩大热/磁盘负荷。R2由另一owner在R1树独立实施。


## N2 candidate16已交源码独审（root）

作者树27093f4a上9tracked+3new共12路径，双盘29文件，patch SHA256 `9c36f7fa102a8e160fa0b712d8185dc54666ff8c51a32fca507aa863365ac9dc`，manifest `73a27f9e4b4e3c996b07c65bc12b533aa89a0a2145354d34a6de216dee2f36e7`。外盘 `Q039/node-pair-vouch-candidate16`。完整Pair/Vouch已启动垂直路径及恢复fixtures交审，12源Rust解析/diffcheck过；Cargo/native/commit为0，不标N2修复。shutdown_owner非作者独审；作者只准备既定验证入口，不launch。2普通+3ignored新业务父、1helper不计父，原29,224,082B Pair04 90s门、原pairing语义保留，只invite文件定位由token转tokenHash。

沿现有State/TransferStore/worker与N1 pump，没有whole-table/inline roster fallback。Pair设备bootstrap用现有canonical Snapshot以保原Pair04数据/64MiB门，normal N1仍Updates。863 git-listed frame不是实际native closure，后续用Q055实际selected/ignored来源。独立新问题如已有normal devices plan冲突仅登记下一轮，不安全删除plan、不扩本轮范围；用户明确已开始做完，后续只记录。


### candidate16非作者源审完成（shutdown_owner＋root）

实际29dual/12current payload与manifest核齐；root读Pair/claims、doc route、prepare/apply与recover-before-bootstrap、Vouch，shutdown_owner独审同一契约，未确认明确阻断。已核认证caller+nonce+immutable header、raw ticket首Start/tokenHash不当凭证、Ready与target-before-flush、exact旧key、epoch和当前gone/revoked限制、purpose/manifest pins、Vouch upload-only与重复Write不重建、1MiB/256KiB/64MiB原预算；原两fixture只文件定位改hash。图仍旧快照，不当当前覆盖证明。

允许准备实际3target/47 distinct＝新5+原Pair04 1+原pairing6+trust18+N1原17；先实际list/ignore核对，再定向新与原Pair04，后同artifact既有回归。原N0另target放最终集成组合，避免作者/整合重复重型构建，不丢验收。当前唯一native槽空闲，shutdown先准备，N2需root授槽后才launch。source/type/native/App未知不以源审替代。


### 首次编译红保留，最小candidate17 / run02执行中

run01 terminal exit1/37.138s，失败在Node编译，0业务父：trust.rs四root::load返回anyhow不能传Error::from_std（重复诊断共8条），一处tuple的&String/&str分支不一致。13dual/raw SHA `be126f294f40b134e7c404244c138117ae7e442b9aac4c6d7b96ebdb90ce3e17`；1932输入before/after/current一致，owned groups与runner全退、/tmp/n2v16清、无forced/cleanup错误。root实核raw、13dual与实际PGID，不记作产品运行红。

root授权最小编译修，candidate17仅4路径：trust四处现有String→Error映射与as_str/as_slice类型，另去本片unused wire/Doc与无必要parens；TESTS/oracle/范围不变。delta SHA `c8946e9298573fac21a739df223014eada01d37329371dffffe14d27b27f9fc8`，root已读，30dual。plan02只换候选/输出/短TMP，runner SHA `cdefd8196c56413cedd8b2030f405d2c26ee4452b8861ec95e7164333f28f195`，data `fdf60889df674fb9519100e3551e838b1e3ed48c06fddbda5299cd1e251c5f45`，7dual，仍1932完整Git输入/实际selected来源与3target47父。当前session53766/runner43484，metadata1.05s/0后build-node43578，尚无业务结果。共享native槽仍N2，其他owner不得启动。


### run02现成小Pair业务红与source18最小修（当前run03）

run02三必要build全部0：Node75.78s、原Pair04 target48.25s、pairing10.16s，实际三个产物及all/ignored inventory来源过。新2ordinary已绿；第3父原`pairing_joins_both_tables_and_chat_flows_both_ways`在123行`b.post("@alpha from beta")`报alpha不通。此前真实Pair和两端2rows断言已过，随后发消息失败是产品红；其余44父未跑，不能说47绿。run02 144.726970s、31dual，1932输入与3产物稳定，13PGID/runner43484全退、TMP /tmp/n2v17无、guard/cleanup空，root读raw并实核SHA/PGID。原raw SHA `ded06f402675a6ed2e2ecfdbcd3e8dc961bb3a60e8a074a2cdc6e29783fbefb7`。

定位当前本地settlement给issuer remote_addrs传空Vec，新caller Snapshot可传播空hint。root核实际Ticket.direct是已有Vec<String>、且dial成功用了同一direct，授权最小沿票据保已成功拨号地址。source18仅link_pairing两行：clone ticket.direct并传入settlement；delta SHA `3525f20fc67adf8bb8fc9428cfe4c6c605ad6c2665df8f4b6b03c988026cbc1d`，root已读，31dual，无新wire/地址框架/父。原小Pair就是现成红绿oracle不改。plan03仍1932/3target47，7dual，runner `86e8c24bc26d5a32e8f98798cfd884a3777005776d38e682b7f02e7010b24046`，data `04e0809f3e19a0378ae90d6b0014ba70c22c9f5b5932784d1977e51867bdcdcf`。

当前唯一槽继续N2，run03 session40694/runner49774，metadata0/.94s后build-node49812；源不变、未宣称新业务绿。shutdown plan02已准备（runner2bc5ee7c…94c42），补内盘ROOT场景显式外盘target持续5GiB门，原37/真实Pi/normalTauri不变，等本轮归槽再执行。


### run03四父绿，sweep同义夹具定位补齐，run04执行中

source18 run03三build/proof/list全部0，当前新2ordinary＋原pairing完整双向聊天/认证/Seen链＋真实expiry共4父绿。case05原sweep旧票/空票删除断言均过，最后fresh minted仍按raw token文件名查找失败；其余42未跑，不能充47绿。167.260s、33dual、1932源和三actual产物稳、15PGID/runner49774全退、/tmp/n2v18清、guard/cleanup空。raw SHA `08c59139f95791a7fa6fa31537c83ed77df70301082c2af933db83b9d9b824cf` 保留。

source19只tests/pairing.rs补同义hash定位（3hash行+原assert），所有原过期/旧票/新票/时限断言保持；root已检查全部invites相关测试定位，无另见raw路径。delta SHA `4d4a40c5d29985b3e14fe7242076e5cf7a920cd7ba1543deac704315212c4b4f`，32dual。用户收口要求下不新建跨版本承接/不改proof源域/不扩矩阵，仍当前3target47父，未跑恢复/Vouch/Pair04等须当前实际artifact。plan04只有候选/输出/短TMP切换，runner SHA `bb85351745bf4204dd888b7d52db6682c81c0dd04cd4b8c7ad9c0c82df308800`，data `83c9c345c05fba6f0d6e08fd0f6b55901ed942add742f8b05f6feebafdedce7c`，7dual。当前唯一槽N2，session71681/runner56774，metadata0后build-node56827，尚未新业务结果。
