> 最新root检查点：core2211→fdeda、pending ed460→9543、当前inventory1d2640d2已进入集成；下面原阶段过程保留历史。严格Doc apply/持久Applied/Node真正chunk与Pair仍未实施，最新状态读STATE。旧Khor兼容不恢复。

# Q039：统一当前文档传输契约（设计待root接口确认）

状态：core与pending-integrity两批已独立提交/待root审集成。最新pending批通过6定向+sync lib147+transfer12；Node/严格Doc apply/Pair仍待实施，blob VerifiedComplete不是Doc Applied，也不代表25MB Node/Pair已收敛。
作者native_helpers；协议/生产/集成决策root。用户2026-10-26明确尚未上线、无需旧端兼容，机群一起升级可接受；[旧legacy卡](Q-039-legacy.md)保持superseded，旧fixture不恢复。

## 来源与当前事实

- 现场`/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，`fix/quality-sync-chunked`，HEAD **8d87af99fb97da3d3d61435df82dcd7bae3c5eef**，tracked clean。先读中央STATE/PLAN（68aaa新决定）再graph-first；图谱built2a11/HEAD8d87不匹配，symbol查询也有歧义，结论取实际link/proto/wire/Doc/store/glue/trust及锁定vendor Loro源，没有以旧索引代替现场。
- 来源指纹完整保存在`Q039/current-protocol-design/source-before.json`：link **62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e**，proto **b6fd92ece3b754c7904b4c864cb067b7eca17baa7c4e1b711b218cc4a758adca**，wire **bc4adac3222b0c1714bf4693bd10ef8614b635ac9adfef76f35efd5dd2fc6d68**，store **e9a3ccf2b4bfd82f6e5adf7c656c548ed57bd274d2e68789b14e6b2c7253511b**，原modern test仍dbfd5127。
- 已有真实Node合法25,448,146B差集使旧轮Seen0/Pinfalse的红、62eb修复后Seen123/Pintrue但devices仍超限Err的绿，继续有效。本卡不把“独立文档推进”称为“大差集已收敛”。
- `sync_device`10336起目前Versions→Refused→vouch→再次Versions→`sync_everything`，两套轮次和不同epoch结果；`rounds_from`10961起每轮以Peer构造整段base64，4MiB在`changes_since_b64`导出后才检查。`MAX_FRAME`为6MiB。
- `Doc`49–63仅提供完整`Vec<u8>` export/snapshot和`merge()->Result<()>`；`glue::merge`丢弃Loro ImportStatus。锁定vendor确有`ImportStatus.pending`、`decode_import_blob_meta`（partial VV不是完整VV）、UpdatesInRange，但现在第一方API未提供有界export/import reader。不能据这些API名称宣称已经有流式CRDT。
- store `flush`985起写增量block并ledger；`write_atomic`1175起sync文件后rename，未sync父目录。进程重启可重新load与“断电持久ACK”是不同保证；新协议若承诺后者，store必须有显式更强落盘边界。

## 单一方案与结果语义

保留有界文档inventory/Versions作为“哪些doc不同”的计划优化，**删除Versions拒绝后的legacy回退、sync_everything及其专属两轮Bootstrap路径**。当前协议不认识的op、坏响应、未知已声明doc均为明确ProtocolError，继续后续独立doc并保留首错；不再用旧端不支持为由吞optional global doc拒绝。动态board未被双方计划选中属于NotSelected，不伪造成成功同步。

现有Vouch还承担“对端当前roster丢失”的数据修复，不能因旧端分支删除而误删这一功能。应将认证门的`NotAdmitted`与`Unsupported/ProtocolError`分型：只有前者可按现有签名规则尝试一次Vouch并重试**同一当前inventory**；失败仍明确报告，不落入全量旧轮。不存在通用Refused→Vouch→legacy的推测链，Vouch也不绕过认证/签名或跨root冲突。

每个选中文档的两个方向都只用同一个DocTransfer。小增量、25MB历史、首次devices快照都使用相同manifest/验证/落盘/ACK代码；没有旧Sync与新Transfer双路径。可以改变当前msgpack形状并共同升级开发构建，不做能力协商/旧端矩阵。读取型readings仍各自best effort，失败保留旧缓存真实年龄，不覆盖文档首错。

统一内部结果建议：

```
DocOutcome = UpToDate | Applied { pushed, pulled, durable_vv }
           | Apart { local_epoch, remote_epoch }
           | Pending { transfer_id, stored_offset }
           | Failed { kind, reason }
RoundReport { by_doc, moved, first_failure, pending, apart }
```

- 所有doc都继续。首个真正失败按固定doc顺序保留，不被读数或后续失败覆盖；Failed不能假报整轮成功。
- epoch只作数据隔离：请求/manifest/提交时均检查，跨epoch零merge、零follow。保留当前modern的隔离策略，不新增自动删除、降级epoch或reset历史；`Apart`统一记录，不再有legacy Err/modern skip两套。
- Pending/Apart不能格式化成“全部同步完成/没有东西可搬”。内部结构先落地；现有`sync_now -> per-peer Result<String,String>`可暂不改公共签名：hard error映射Err，Pending/Apart映射明确“部分完成/epoch隔离”的文本，禁止借Ok掩盖未收敛。若root要结构化公开该报告，另登记IPC/GUI类型边界。
- devices只有接收到且持久应用成功才follow；失败/仅Stored/Pending/Apart均不follow。broadcast只在本轮持久新增后通知到达；重复Applied/已有VV不重复触发业务。

## 固定descriptor与同一小/大传输

`DocManifest`必须固定：`protocol_revision, doc, epoch, encoding, base_vv, target_vv, total_bytes, digest`；id是该规范编码的域分隔blake3（或独立随机id同时绑定完整manifest），存储key另包含authenticated source id、目标身份与用途。不得只按doc名覆盖stage。VV用规范二进制，chunk用`serde_bytes::ByteBuf`，不base64。

- doc只允许已验证global名/合法wide board；encoding只允许本协议明确的Loro完整Snapshot或Updates，不接受shallow/state-only替代用户历史。
- `base_vv`是本次接收者已持久版本，`target_vv`是本次发送者固定导出版本。并发时target不一定包含base；成功后接收者应包含 **base ∨ target**，不是覆盖成target、更不是要求整个文档等于发送方快照。
- sender先flush本地来源，再从同一Loaded副本导出并计算target/digest，写private不可变spool，drop大Vec和Loaded后传输；网络期间不持pile/全局store锁。之后本地新op留待下轮，不混入当前target。
- receiver当前版本必须包含base且epoch仍相同；否则明确StaleBase/Apart，重新计划，不把旧片接到新epoch。body checksum、Loro checksum/metadata与descriptor一致性必须在改变产品Doc前检查；metadata的partial VV不能充当完整target。

最小wire草图（命名可调整，只有一套DocTransfer）：

```
SyncDocument { doc, epoch, have, offer?: Envelope, applied_receipts[] }
  -> { received: Stored | Applied | Apart | Failure, offer?: Envelope }
Envelope { manifest, first_bytes }              // 0..256KiB
TransferWrite { id, offset, bytes } -> Ack      // 最后一个完整块触发同一commit
TransferRead { id, offset } -> { offset, bytes }
TransferStatus { id } -> Missing | Stored | Applied
TransferCancel { id } -> Cancelled | Applied
SyncFinish { applied_receipts[] } -> Finished
```

`SyncDocument`同时接收本端offer并按`have`准备对向offer；descriptor首帧携首块。**小差集<=256KiB可在一次request中验证、merge、持久化并回复Applied**，没有每doc空Begin/Commit往返。大body的首帧只回Stored，后续同一个sink的最后一块调用相同commit；不得另写“小doc快捷merge”绕过验证/落盘。对向小pull由本地同sink落盘，Applied receipt捎到下一请求，最后一批由每轮一次SyncFinish确认，不给每个seen/pins加独立ACK往返。上行下行独立提交，整个doc双向交换不宣称原子事务。

## 落盘ACK、重复、取消与重启

Stored和Applied必须是不同类型；收到全部网络字节不是“同步成功”。

1. 接收stage为private `.khor/sync-transfer/<peer>/<id>/`，create_new/非symlink、明确owner；manifest与durable contiguous offset有界持久记录。
2. 只允许下一连续offset。写chunk→sync data→原子更新offset metadata并sync目录后，才能回`Stored(next_offset)`；掉电发生在data与metadata之间，重开只信已确认offset并丢弃未确认尾部。
3. offset早于已确认值时核对应已存字节/块hash完全一致，才幂等返回当前offset；不同内容拒绝。未来offset拒绝并返回expected。不同id/peer/doc/epoch不可混片，旧owner cleanup不删除新owner目录。
4. 收齐后从spool核总长/digest与Loro格式/元数据；提交时重新加载当前pile、重新核epoch/base，使用候选Doc导入验证及`merge_checked`保证无pending/覆盖base∨target，再保留本地并发op地导入并持久写block。doc dispatch在Node边界，传输core不猜业务表。
5. 必须保留`took_in`的pen退役语义；`flush_durable`写完block/必要epoch和ledger并sync父目录，后写Applied receipt并sync。只有这之后返回`Applied { target, durable_vv, digest }`。错误发生在中途不得回Applied；候选验证前不让坏body污染活Doc。
6. ACK丢失后重发最后chunk或Status：若有匹配receipt返回Applied；若receipt尚未落盘但block已持久，fresh load的VV可证明target已具备，返回区别明确的AlreadyHave/AppliedEquivalent，而不宣称某未核digest已导入。幂等重导入也安全，不能重复业务通知。
7. 断连仅结束当前worker，不删已确认接收stage或已落盘doc；同manifest可续传，sender重启spool丢失则按receiver fresh VV重导出**新id**。允许重传，不允许把内存offset当持久进度。
8. Cancel与commit在每transfer的串行owner锁下确定先后：commit未开始则停止owner并清只属于此id的stage；commit已完成返回Applied，不能声称取消回滚已合并op。旧轮晚到chunk只可触及原id，不覆盖/清新轮。连接关闭和取消不删除用户pile。

store实现须明确“文件sync+目录sync”的平台保证及失败返回；不能将当前flush直接改名就称断电安全。若首切片只做到进程kill/restart，测试与文档只宣称该边界，Applied durability定义未达时不能完成整任务。Windows目录sync/原子替换需对应实现与运行证据，不能借macOS绿。

## Pair必须同批接入，否则新机器仍有黑洞

现场Pair arm3643起：先settle、burn token、enroll/flush，再把`wire::table_b64(snapshot)`直接塞Paired.devices；该export没有4MiB保护，base64后可超过6MiB MAX_FRAME。客户端pair_with9670起完整read_to_end后才合并，因此可能出现**issuer已接纳/token已消费，joiner收不到表**。这由源审确定路径，但本次没有新运行，不能先写“Pair25MB真红”。

建议新增独立pairing operation模块而非link里再加状态：

- Pair响应只携有界admission operation引用、对端身份/root信息及bootstrap DocManifest+首块；devices snapshot由**同一个DocTransfer**分块到私有候选stage，epoch/签名/settle通过后才导入现有本地store，绝不先删本地表。
- Pair请求/响应的roster本身也可能大，不继续把任意`Vec<u8>`塞握手。以同一个通用blob sink承载`TrustRoster`用途（有签名digest、无伪造Doc VV）；DocTransfer是在blob envelope之上的Doc语义，复用chunk/owner/ACK，不复制第二套传输协议。root clash判定必须在token不可逆消费前取齐并验证必要trust blob。
- issuer对尚未一般admit的joiner只发**绑定token claim+remote_id+operation id的bootstrap grant**，只可读该devices/trust blob；绝不放开未配对身份的任意Doc/文件访问。所有mutating Begin/Write/Cancel/Pair op不允许0-RTT重放。
- 在消耗token/改admission前，持久记录pair op的authenticated remote、client request nonce、claim和阶段；token只能由这一对身份的同op恢复，其他id仍拒绝。使用独立per-token跨进程锁+原子记录，不能仅靠进程Mutex或重试时重新消费token。先写可恢复claim，再做可重复admission/enroll与descriptor记录。
- joiner先持久保存issuer身份/operation引用，收齐验证signed roster、root settlement及snapshot；应用落盘后ACK，issuer才报告bootstrap complete。ACK/响应丢失可同op查询/恢复；新机器重启不陷入“token已烧、表永远没取到”。
- cancel只停止bootstrap stage，不自动撤销已持久enroll/root settlement；如已部分admitted，操作状态明确可恢复，不用删除他人并发设备行来假装事务回滚。完整跨文件trust变更崩溃恢复归pair op状态机验证，不能由单个Doc ACK替代。

## 资源预算与诚实限制

首实现建议常量（待root对齐，不靠抬4MiB掩盖）：

| 项目 | 建议与理由 |
| --- | --- |
| raw chunk | **256KiB**，首块与后续块相同；每transfer最多1个未确认chunk，无无界channel |
| manifest/control | descriptor总64KiB，VV各至多24KiB，inventory/readings分page64KiB；SYNC_FRAME_MAX384KiB，独立于其他业务6MiB MAX_FRAME；编码前与解码后都验长度/计数 |
| 活跃数 | 每peer每doc每方向唯一owner；节点最多2个transfer、最多1个Loro export/import worker，普通控制stream有保留seat；CPU/磁盘工作不持wire task/全局store锁 |
| 暂存 | 首实现每blob **64MiB**、节点所有incoming/outgoing总**128MiB**、每peer不超过64MiB；按manifest预留再接收，超额明确ResourceLimit，已落盘用户store不计入可清stage quota |
| 公平性 | 每doc最多4块/1MiB一quantum，然后让其他doc与控制；小doc优先、仍有大doc最低份额，避免devices独占whole visit；visit30秒/32MiB后显式Pending并保持进度，backoff不把Pending当拨号失败 |
| 生命周期 | 无进度30秒关闭worker；空闲可恢复stage最多24小时，GC只清确认没有活owner的传输私有目录，启动时先reconcile receipt/offset；从不GC用户pile |

**64MiB是明确首切片资源边界，不是任意大文档已支持的宣称。** 25.4MB研究输入在其内；超过上限不得无限自动重试/假成功，必须显示受限原因并保持其他doc推进。若关闭Q039要求覆盖更大历史，需在同一contract后补基于锁定Loro UpdatesInRange的多export-page cursor，或真实有界export writer；不能把64MiB当“问题根治”。该后续仍不需要旧端矩阵。

256KiB只约束线上chunk与发送队列，**不约束Loro内存**：当前export完整Vec、candidate snapshot/Doc与final import、store.flush再次export、全局KEPT snapshot cache都可能同时存在。first cut应spool后立即drop导出Vec、export/import单worker，并对25MB真实Node记录peak RSS/阻塞时长/取消延迟；不能预称O(chunk)内存或给未测“2×64MiB安全”承诺。压缩blob长度也不保证解码后的对象内存，64MiB post-export检查不能防止导出前OOM；要严格内存预算必须进一步改Doc/Loro边界或隔离受限worker，这不是多加一个Vec长度判断能完成的。

## 文件/API边界与阶段顺序

建议先与root对齐下面接口，获写权后再实现；本卡未创建任何模块：

| 位置 | 职责 |
| --- | --- |
| `crates/sync/src/transfer.rs`（新） | manifest规范编码/验证、typed ACK/错误、spool连续offset/重复/校验状态；不依赖Node/iroh/业务名列表 |
| `crates/sync/src/store.rs`、`glue.rs`、Doc各实现 | `merge_checked -> MergeReport { applied, pending }`与候选验证接口；`flush_durable`/fresh durable VV，保留pen/epoch/不可变block语义。Loro类型留sync内部 |
| `crates/node/src/proto.rs` | 一套SyncDocument/Transfer消息及有界metadata/raw bytes，不保留旧Sync base64契约；mutating op禁止early；encode/read budget独立受控 |
| `crates/node/src/sync_transfer.rs`（新） | authenticated peer/purpose grant、owner/quotas/磁盘路径、wire source/sink适配和ACK边界 |
| `crates/node/src/sync_round.rs`（新） | 统一Doc dispatch、planner、公平pump、RoundReport；把10336后的同步策略移出link，不给12k文件继续叠状态 |
| `crates/node/src/pairing.rs`（新）、trust小接口 | pair op恢复、token claim/admission grant、bounded signed roster+devices bootstrap；验证root/cert仍走已有trust核心 |
| `link.rs` | 只保留连接/认证/分发接线及现有Node对外入口；删除fallback/两轮base64发送专属实现，不复制with_doc列表到多处 |

实施顺序建议：

1. **阶段1最小切片**：先固定DocManifest/Ack/Envelope与结果语义，纯transfer/store tests先红；实现inline和multi-chunk同sink、disk offset/commit与一条真实Node doc双向路径。先用新的独立test验证25MB在受控manifest/cap内实际收敛、重读接收store具备target，保持同轮Seen/Pins；此时Pair缺口仍开放，不能宣称全Q039完成。
2. 提取round模块并替换当前全部Doc Sync；删除Versions拒绝fallback/legacy-only实现；按公平quantum继续独立docs/readings。既有现代oversize用例从“devices仍Err”变成单列故障注入隔离用例，另新增25MB收敛验收，不能悄悄删掉失败隔离覆盖。
3. Pair operation+同blob core的signed roster/devices bootstrap，先红测超过MAX_FRAME及token消费后断连/ACK丢失，再跑真正全新Node加入。完成前禁止将“大差集可同步”推论为“新机器也可加入”。
4. owned取消/断连/重启与磁盘故障矩阵、实际RSS/控制公平性门，最后在精确集成头重跑所触及pairing/sync/store与真正Node流程。shared target只由root排槽。

必要先红场景：25.4MB单轮/有限预算多轮最终收敛；单块<=256KiB小Seen/Pins无额外空往返；错doc/epoch/base/target/digest/总长与超额metadata无merge；重复/乱序/最后ACK丢失；每个chunk后kill+restart从持久offset继续；block已落盘但receipt未落盘恢复；flush/fsync/磁盘满不得Applied；接收中epoch变化/源本地并发编辑不混代、不丢并发op；取消与commit竞态结果诚实、旧owner不清新owner；大devices期间Seen/Pins/普通控制请求继续；**Pair snapshot>6MiB与大roster**、断连重入同claim、其他身份不可偷grant、root冲突不burn token/不删除旧store。

当前仅设计，所有新协议/预算/Pair红测均未运行。未动生产、测试、Cargo/锁或legacy冻结快照；STATE由root维护。

## 阶段1：blob核心实现准备（尚未编译/运行）

上方“仅设计”保留为接审历史。本节记录root后续批准的第一切片：作者独占新增`crates/sync/src/transfer.rs`、`crates/sync/tests/transfer.rs`，`lib.rs`仅加`pub mod transfer`，Cargo.toml仅复用现有workspace `blake3`。root独占修改锁中khor-sync dependencies一行，包版本/checksum不动；因此**锁确实改变**，不能声称锁未变。没有改Node proto/link、Doc/glue/store现有API或恢复legacy测试。

准备源SHA：

| 文件 | SHA256 |
| --- | --- |
| transfer核心 | 20f077942f9c1a3030e8c34dd2d179e72118c3ff4b49cd180761a241c3fb54e4 |
| transfer集成测试 | bdb918d88939629b8691759fd1a60e44d385250519dc9fec36bfb5ca03a99016 |
| sync/Cargo.toml | 5c4a07f141722d3faeee0c6d4d0f5b224545320cb6dcd96b1cc4df8dd395f62f |
| sync/src/lib.rs | 6c1648d6acd6c04a47f9f00a6ef6665628322a6fc3d987d305006a3252d64c27 |
| Cargo.lock（root写） | 4436a386ad74fdceec35dbfcb67ecbb32d593bfff56badfc92e9d5f5437bf9e6 |

`Q039/transfer-core-prepared/{source-prepared.json,tracked.diff,core-source.rs,tests-source.rs}`保存实际来源；link仍62eb，原现代test仍dbfd。父树仍HEAD8d87，未提交本批。仅新文件rustfmt/--check及git diff --check通过，这只证明解析/格式，不证明类型或行为。

### 已写core API与边界

```
Manifest { source:[u8;32], recipient:[u8;32], binding:Vec<u8>, total_bytes:u64, digest:[u8;32] }
Manifest::validate() / id() -> TransferId
TransferStore::open(private_root) / receive(manifest) -> Sink / reserved_bytes()
Sink::append(offset, bytes) -> Stored { next_offset } | VerifiedComplete { id, bytes }
Sink::progress() / next_offset() / manifest() / read_verified(offset, <=256KiB buffer)
```

- manifest id由固定magic/字段次序/明确大端长度/内容blake3决定，source/recipient/purpose binding/长度/digest均参与；core不使用JSON字段顺序。上层后续仍须把doc/epoch/baseVV/targetVV/encoding规范编码进binding并验证authenticated peer；本切片不伪装已实现这些Doc语义。
- 首块与后续块唯一`append`入口；非末块固定256KiB，末块至多256KiB，空body在校验空摘要后确定VerifiedComplete。不能用微小分片放大metadata。
- `Stored`使用真实文件写入：data sync→绑定manifest-id与prefix digest的checkpoint record写入+sync。journal每条72B，最多初始记录+256块=257条；重开流式校验已记录prefix，只截断同id未确认body/撕裂journal尾部。重复必须读真实已存字节相同，冲突/未来offset/重叠/超长明确拒绝。
- 任何不确定写入使Sink poisoned，后续不能回Stored/VerifiedComplete，必须drop/reopen；真实只读journal fd故障测试针对“body已写但checkpoint未确认”窗口。digest不符即使收齐也不是VerifiedComplete。
- 一个私有专用root，leaf0700/file0600（Unix），拒绝已见的symlink/非regular文件；std gate OS锁串行quota/接管，两个稳定owner文件锁维护跨进程lease，进程内registry补同进程锁差异。Drop先close OS owner再放registry，释放owner**不删除stage或reservation**；不查PID、不自动GC、不提供Applied。
- 不完整初始化或不明root entry明确InvalidStage，需要未来显式reconcile，不静默删除未知目录；本次调用初始化失败只清自己刚建stage，cleanup失败带原cause和明确cleanup诊断。root必须是调用者专管的private tree，尚不宣称抵御同一用户在其内并发恶意替换路径的竞态。

root已批准补充最多**64 retained stages**，否则空blob可绕body quota无限占manifest/inode。body reservation上限仍128MiB；metadata另至多64×64KiB=4MiB，checkpoint另至多64×18,504B=1,184,256B，固定两个32B owner绑定/gate及目录项另计；**128MiB不是总磁盘上界**。chunk/reopen hash buffer至多256KiB；这没有改变后续Loro完整Vec/export/import的内存事实。

### 准备的真实scratch回归与排队命令

普通集成9个（Unix包含私有模式/静态symlink1个）+1个由普通父真正启动的ignored kernel-lease worker；另1个core unit故障测试。范围：空/inline/多块同契约、每字段绑定id、manifest/body预算、reopen/精确重复/冲突/乱序、未确认body与撕裂checkpoint尾部、已确认字节损坏、完整body错摘要、2 live owner与128MiB retained reservation分离、64空stage预算、private mode/symlink不改目标、真实子进程持OS lease被kill+wait后重开保留offset。子worker只处理自己的scratch，无provider/Node/store/用户HOME；父RAII只kill/wait确切child，等ready10秒护栏，worker最长20秒自行收尾。

没有为新模块造placeholder panic/编译错冒充业务红，原25MB真实Node红仍是业务来源。新模块将以真实scratch行为通过证明契约，任何编译/setup失败单列。

root排唯一槽后建议两个明确入口，统一quality-target/jobs2/offline，`KHOR_TRANSFER_SCRATCH`与TMPDIR均置于本次Khor证据scratch；编译运行前后保存本批全部源/Cargo/锁SHA、自有PGID、双盘>=5GiB与墙钟护栏：

```
cargo test --locked --offline -j 2 -p khor-sync --test transfer -- --nocapture --test-threads=1
cargo test --locked --offline -j 2 -p khor-sync --lib transfer::tests:: -- --nocapture --test-threads=1
```

顶层不加ignored；父应实际启动1个专用worker，不把0测试或worker未执行计绿。目前**未Cargo、未类型检查、未运行、未计绿**。Node统一接线、checked pending/base∨target、Doc Applied receipt/durable目录、取消/全stage显式reconcile/GC、Pair大快照与trust/admission恢复均在下一切片，不能借blob VerifiedComplete声称它们已完成。STATE与构建槽仍由root维护。

### root源审后补充：显式释放retained reservation（仍未编译/运行）

root审20f/bdb固定帧指出两项真实源缺口：没有取消/释放入口使坏digest、损坏body或新manifest后的旧64MiB reservation无法释放；以及旧`initialize`若mkdir失败，外层仍无条件remove_dir_all，未证明目录属于本次创建。此为源审缺口，没有伪造不存在API的编译红或把新模块编译失败算业务红。

已在同一allowlist修正并重新冻结：core **f010cac088a3648e0e33b859560a68aa4258d7155122623a60523e6be56d7567**，集成测试 **b301ee73777d5a8133f6aa83b3aa11face3da14b1acf9617786b70c8c998eb2b**。Cargo/lib/锁仍5c4a/6c16/4436，Node link62eb、原testdbfd未动。旧准备帧保留，新源/diff/7项指纹在`Q039/transfer-core-cancel-prepared`。

- 新公开`TransferStore::cancel(&Manifest) -> Cancelled::{Removed,Absent}`。验证完整manifest/id，在全局gate下取得普通File owner lease，先拒绝相同id的任一活owner（包括别的进程），再只在同id目录完整manifest仍逐字段相等时删除stage。既有body/journal损坏不妨碍按完整manifest释放；其他source/recipient/binding对应其他id，不能删除原stage。重复取消Absent，不修改其他retained stage。
- lease是普通File，不是会在Drop里再次锁LOCAL的Owner类型；持gate期间没有Owner Drop重入。app将来须先终止并drop自己的Sink，再调用cancel；晚到旧cancel无法穿过新owner的活锁。blob取消不回滚Doc，亦无Applied/断电持久保证。
- 新`create_stage`先成功mkdir，才进入文件初始化与失败清理区；mkdir失败直接返回，无权删除未知既有目录。实际创建后的cleanup失败仍保留原cause与额外cleanup诊断。
- 新增3普通集成回归：坏digest/损坏body与checkpoint可按完整manifest取消并释放配额；活owner/新owner/不同或被改manifest保护；128MiB满额取消一个后可再申请64MiB且别的stage字节不变。原kernel lease父追加跨进程cancel Busy判据。新增unit用真实既有目录+sentinel验证mkdir失败不清未知目录。

现在准备组为**12普通integration（Unix）+1真实父启动的专用worker、2 unit**。依然只执行rustfmt/--check与diff-check（exit0），未Cargo/类型检查/行为运行，不计绿。Q045持唯一槽，本任务无PID/槽占用。

明确后续`stage-reconcile`边界：若进程在stage初始化一半时死掉，目录可能没有完整manifest，目前返回InvalidStage并可能阻塞后续reservation扫描；本补丁不擅自猜owner或自动GC，无法按完整manifest取消的半初始化目录仍须专门显式reconcile策略/故障测试。核心未接Node前不能把它称为所有崩溃阶段已恢复；继续保留该后续任务。Doc取消/commit线性化、节点转发/授予身份、Applied落盘和Pair等上层工作仍未实施。

### 固定源实际运行通过（2026-10-26）

root复核f010 cancel/create_stage后授唯一槽，源码f010/b301/4436全程未改。运行前仅对本树sync/catalog39项.rs/.toml输入touch，前后逐项SHA相同；完整mtime manifest保留。实际Cargo日志显示catalog/sync从本树重编，没有编Node/新建target。

证据`Q039/transfer-core-green-01`：jobs2/shared quality-target/locked/offline，TMPDIR和KHOR_TRANSFER_SCRATCH均为专用证据目录，source-before/after43项（含输入、Cargo/锁及未动Node基线）一致。

| 阶段 | 实际执行 |
| --- | --- |
| integration | runner89482/Cargo自有PGID89502；**12 passed/0failed/1ignored**，5.15秒测试、11.076秒含编译。ignored仅为普通kernel lease父真正spawn并等ready、核跨进程Busy、kill+wait的专用worker；不能另算第13普通父或0测试。 |
| 新unit | Cargo自有PGID89625；**2/2**，3.526秒含编译。既有目录mkdir失败保留sentinel、真实read-only journal写失败poison与重开恢复均通过。 |
| 完整sync lib基线 | 不再Cargo，用上一阶段Cargo日志实际指向的`khor_sync-754f7fafad29b8a1`直接运行，PGID89658；**146 passed/0failed/0ignored**，38.10秒测试、38.295秒阶段。包含新2unit；不将146+2称为148个不同用例。 |

总52.945秒，三阶段均exit0/source_stable=true；baseline binary前后SHA **d71eda3539140a6944f57bccf6842c6541930103be6264691b158f3b098d888d**，size24362632，path/mtime在lib-binary-before/after.json。最低external**6,805,020,672B**/internal**11,059,986,432B**，未触5GiB或阶段240秒/整体360秒护栏。各Cargo/测试独立PGID89502/89625/89658结束后killpg(signal0)确认不存在，专用scratch为空；已立即交还唯一槽，没有后台本任务进程。

保存完整run.py、各阶段test.log/process/result、整体result/source-before/after、touch-inputs与binary记录。新增模块无旧产品入口可重取业务红，故不造placeholder panic/编译错；原25MB真实业务红仍作为Q039来源。当前绿仅证明本blob/core契约与现有sync库基线，Windows runtime、断电durability、半初始化stage reconcile、Doc pending/base∨target/Applied与Node公平传输、Pair大快照/准入恢复仍不在本次实测范围。

root接收固定源结果后授权独立保存，本批已提交 **2211daa9be1dc3a7f9ab2ba87d50b9787d29a0c6**（`feat(sync): stage bounded resumable blob transfers`），仅5文件：新core/tests、lib模块声明、sync Cargo依赖及root的锁依赖行；未含中央卡/其他任务。提交前后再次核f010/b301/4436及7项来源不变，`fix/quality-sync-chunked` tracked clean，收据`transfer-core-green-01/commit-receipt.json`。commit hook图谱更新不是新的Cargo或运行证明。原8d87为父提交，未改Node生产/旧现代用例；后续Doc层接口分派等root，不自行扩写。

## 下一切片Doc apply接口草图（待root精确确认，生产未改）

锁定Loro `ImportStatus { success: VersionRange, pending: Option<VersionRange> }`必须真实映射；每writer范围为`[start,end)`，VersionRange可能是单区间包络，不能加总当精确新增op数。`success`也可能包含本次依赖到达后解锁的先前pending。`pending`报告当前导入仍阻塞的范围，不是完整版本向量或整个Doc全部历史pending列表。

建议最小数据/API：

```
ImportSpan { writer:u64, start:i32, end:i32 }
MergeReport { applied:Vec<ImportSpan>, pending:Vec<ImportSpan> }
Doc::merge_report(&self, bytes:&[u8]) -> Result<MergeReport,String>  // 必实现
Doc::merge(&self, bytes:&[u8]) -> Result<(),String>                 // 调真实report再丢弃
```

`glue::merge_report`直接调用锁定Loro.import并排序映射success/pending；空bytes才是已知空report。禁止默认“旧merge Ok所以empty pending”。普通`merge`保留Ok-with-pending，store.read_once按文件顺序恢复时仍可先收edit后收dependency；不能把partial batch标成broken/跳过或从held里丢掉。

13个Doc实现必须接真实report：AccountDoc、AgentDoc、DeviceDoc、DirPinDoc、PinDoc、RecentDoc、RosterDoc、SeenDoc、TitleDoc、WebPinDoc、WorkspaceDoc、ChatDoc、store::AnyDoc。ChatDoc固有`merge`也委托实际report再丢弃，维持普通恢复语义。各wrapper构造当前只设置peer/Undo等，不写业务op；候选构造仍应验初始VV为空，不能依赖将来构造默认值不变的猜测。

严格传输应用建议集中在新`Loaded<D>::apply_checked(spec, bytes) -> FlushedImport`（名称待确认），spec含expected_epoch/base_vv/target_vv，返回真实report、version、took_in退休peer及实际block路径，不命名Applied/durable receipt：

1. 验epoch和live版本包含base；从同一live已应用snapshot创建**只导入、不mint本地op**的候选D，seed report无pending且seed VV与live capture一致。候选无需占一个新store/pen，更不替换live Doc。
2. 候选导入body的report必须无pending，候选VV必须包含`before ∨ base ∨ target`。不是把partial_start/end VV当完整target，也不要求target包含base（本地与远端可各有独立并发分支）。metadata额外op边界/encoding校验在Doc manifest adapter完成，blob digest仍由verified transfer层保证。
3. 候选不回写KEPT或清理live pending。验证到提交期间若live VV变动，重验/返回Retry，不能把旧候选赋回live。持有适当pile共享fence并在提交重核epoch；已有Lease取锁失败/新空目录无pile锁时须显式取得commit fence，不能只信内存epoch。
4. 对live真实merge_report，保留本地并发op与可被此次依赖解锁的旧pending；复核目标覆盖，调用原took_in（保留pen退休语义）再flush。flush失败不发成功ACK，live内存可能已merge，须诚实错误并由同payload幂等重试/重开恢复，不承诺回滚。
5. 当前flush仅file.sync+rename；FlushedImport不是断电Applied证明。真正Applied须在block/必要epoch与ledger、目录持久边界后，再持久写匹配transfer的receipt，最后ACK。进程restart测试与目录fsync/平台断电保证分开，不能简单改名flush_durable。

候选snapshot只覆盖已应用op；live里等待依赖的旧pending不得因候选而丢失。真实先红/控制矩阵应包含：缺dependency的edit_only导入虽然Ok但pending/target未达；payload完整但谎报更大target（pending为空仍不能通过）；base未具备；base与target并发不互含时合并保留双方；重复导入；candidate验证失败live VV/业务/磁盘不变；took_in同pen冲突仍退休；flush错误不能Applied；普通冷恢复合法分批pending最终完整。上述方法尚未实施/运行。

### 优先核验：KEPT可能丢pending（仅测试已准备）

源审发现store::load_cached把已读block文件名与doc.snapshot一起缓存；若snapshot不携尚未应用的pending，下次只导入新增dependency时原edit block已被当成缓存覆盖而跳过。这是当前数据完整性**待运行假设**，不是已经证实的产品红。root授权先新增测试，暂不改缓存生产/Doc/glue/store。

唯一新增`crates/sync/tests/pending_cache.rs`，SHA **336ee2f3de9e69a779ac85e0a305a568e2c60212bebbeec17c24c27c7c2bed02**，HEAD2211当前仅该untracked。store仍e9a3、glue743d、pins2f47、锁4436。8项完整来源与测试在`Q039/pending-cache-prepared`。

fixture由真实PinDoc两次commit构造：creation_only在第二次edit存在前立即导出，之后仅导出edit_only。direct普通merge按edit→creation应从未应用状态恢复两项；冷store把edit命名在creation前，同次load/reload必须完整，保护合法分批pending语义。目标case先只落edit→load→再load暖缓存，才添加**不含edit**的creation_only→reload，要求edit被重放及targetVV覆盖。每目录reads_of/cache_hit打印，仅用于诊断，正确修复允许绕过不安全缓存，不把“必须cache hit”写成产品契约。

两个普通case，无Node/服务/provider或真实HOME；唯一scratch RAII回收。计划root授槽后`cargo test --locked --offline -j2 -p khor-sync --test pending_cache -- --nocapture --test-threads=1`，外部owned PGID/墙钟/双盘5GiB/source-before-after。目前仅rustfmt解析/格式与diff-check通过，未Cargo/类型检查/行为运行，不计红绿。真实目标红后才获准实现merge_report与最小缓存修复，2211核心保持独立ref，不混入这批。

### 实际pending cache与compact红（2026-10-26）

root源审后授槽。`Q039/pending-cache-run-01`按原336ee test、store e9a3/glue743d/锁4436实际执行；sync/catalog40项.rs/.toml仅touch且SHA相同，Cargo日志实际catalog/sync重编（5.18秒）。runner15723/Cargo自有PGID15735，总6.074秒/exit101，**两个普通测试实际1绿1红**，非编译/setup失败：

- cold control按edit→creation同次读与再次load均create=true/edit=true/target_covered=true、broken0；own reads(1,2)不增长，证明普通pending恢复与完整cache控制有效。
- 目标首次只edit：reads(1,1)，warm仍(1,1)，**cache_hit=true**。只添加真正creation_only后reads(2,2)，实际create=true、**edit=false/target_covered=false**、broken0。最终“previously pending edit must survive cached reads”内容断言真红，证明旧KEPT将pending文件当snapshot已覆盖而漏重放。
- 44源前后一致，最低external6,798,307,328B/internal11,038,973,952B，无5GiB/180秒guard触发。PGID15735结束且signal0确认不存在，scratch为空，槽已明确归还。binary SHA dda5e723c4951da227e238ad4fe648f6a9afc1a109ab4fdaf62672473d23fd6a。

root随后授权第三compact用例先取红。只新增该用例后test **2551f571f6cc1b744f55db7617d56e709facc75e5525dfaee9ddaeaa3ff49307**，原两例未弱化；生产仍e9a3。`Q039/pending-compact-run-01`runner19591/Cargo PGID19604，exact普通1实际执行，编译2.62秒/总3.046秒/exit101/source44稳定：

- 实际`BlockStore::compact`返回**Ok(snapshot)**，原edit block已被删除（retained_edit_block=false）。随后只添creation_only重载，create=true但**edit=false/target_covered=false**、broken0，内容断言真红。这是不可恢复原块删除的独立现象，不只静态held风险或“没返回Err”的源码断言。
- 最低external6,779,404,288B/internal11,030,691,840B，无guard触发；PGID19604退出并确认不存在，scratch回收。原2例cache红绿及第三compact红均原样保留，未改运行源。

root据两笔实际红授权13Doc/glue/store真实MergeReport与最小完整缓存保护；当前正在准备候选，尚未运行绿。策略为读批次累计真实pending spans，用最终Doc VV逐writer检查end是否已覆盖；未解依赖不建立新KEPT，旧安全base可保留。compact在未解时明确Err、不写snapshot/不删原块，不将pending标为broken；依赖后已解仍可正常cache/compact。`sweep`现有Err继续表示优化未完成，不会把它计入完成结果，无需让resident sync退出。

另root指出live调用面：`wire::answer`与`Peer::absorb`当前merge_b64后took_in/flush，若只在read_once登记则同进程live pending仍漏compact保护。作者已提出`BlockStore::record_import(&MergeReport)`及wire两处实际报告登记的必要最小适配，等待额外wire.rs写权，未擅改Node/proto。严格候选apply/FlushedImport仍待下一批，不混入本次修复。

### 完整缓存修复候选冻结（尚未运行绿）

root随后批准wire.rs最小接线与live门。当前HEAD仍2211，本批独占16文件：glue/store、13 Doc实现（含store::AnyDoc与ChatDoc）及wire、新pending_cache测试；没有改Cargo/锁、core、Node/proto/link或现有现代2例。

完整diff（包括untracked新测试）、全部候选源及24项来源在`Q039/pending-candidate-01/{complete.patch,source-prepared.json,source/}`。关键SHA：store **8506b0a4304bbb07334a354d1bf52e6af49d2cda9f1416b9822321b1c8afe41e**，glue **5f7fb8df853531b900b33f4cae4466d0c896e55ddc5c8fa0ea6877686fdae428**，wire **110ae0c702c1d7070432bebabd1d156432fbd5db3f1a93b31ea0ca2aeea19a7d**，新test **097bb693dd5e7afa07a9aacb57a0c5388e17f9715c428aa1d0844b4616cdc585**。

- 13Doc `merge_report`为必实现并真正委托Loro report；ordinary merge默认丢弃真实report，ChatDoc固有merge也通过glue真实report。只有明确空payload才构造空报告，不从旧Ok推导pending为空。
- read_once保留合法pending block为held/ledger，不标broken；BlockStore记住真实pending ranges，cache与compact均按当前Doc VV是否覆盖全部end判断。依赖在同次或后续导入解开时允许cache/compact；重列表仍走完整read_once重新取得真实报告。旧安全KEPT base可留，含未解pending的新fingerprint不入KEPT；已有snapshot seed如导入pending则拒复用并完整重读。
- compact最早检查未解依赖并返回Err，尚未make_dir/写snapshot/删除held；compact_dir传播错误，sweep现有Err路径继续而不计完成。不将此解释为所有文件系统错误已回滚。
- wire私有`merge_b64_report`取真实报告；公开merge_b64继续普通pending语义。answer/absorb在原took_in/flush前调用record_import，原计算远端差集/epoch验证/pen退休/flush/peer version顺序不变。**本批没有把这些旧wire响应升级为严格Applied**；新当前协议checked apply仍下一批。
- 新test6个普通case：原cold/cache/compact3场景不弱化，新增真实pending/applied半开range检查1个及实际wire answer/Peer absorb live pending→compact Err→dependency解开→compact/reload完整2个。没有复制Node策略或造placeholder红。

新test/glue已局部格式化，既有Doc/store其余原格式保持；全部变更Rust经rustfmt stdout解析、git diff-check通过，但**未Cargo/未类型检查/未运行绿**。新固定源已交root独立审，当前槽先给store_owner短Bun验证，本任务无PID/槽占用。原cache/compact真红两笔证据均保留，core2211独立ref与12+146基线不变，后续按同冻结源排队验证。

### 最终pending归并、真实绿与独立提交

root源审补充：长寿命live store若每次record_import都向Vec追加相同pending，元数据可随消息次数增长。这是源风险修正，未伪造新业务红。最终将**store内部**pending改为有序`BTreeMap<writer,max_end>`，read_once与record_import共用登记helper，按当前VV覆盖end判断已解；公开MergeReport的真实applied/pending Vec范围不变、不加总span。新增1个unit用真实Loro pending report重复登记4096次，store仍只有1个writer项，依赖后compact恢复。

最终候选`Q039/pending-candidate-02`含完整patch/16源与24项指纹；store最终 **6c906fe18b80cb24ea45586d7c07b3d81f6eeb8ba063f112274331faece8b943**，wire110ae0c7/glue5f7fb8df/test097bb693不变。上节candidate-01的Vec帧留作历史，不借它冒充最终map实现。

store_owner释放短Bun槽后root授串行槽。`Q039/pending-green-01`按此最终源运行：先40项sync/catalog输入touch，全部字节SHA相同，实际Cargo日志catalog/sync重编；locked/offline/jobs2/同quality-target，私有TMPDIR/KHOR_PENDING_SCRATCH/KHOR_TRANSFER_SCRATCH，46项来源前后完全一致。

| 实际组 | 结果 |
| --- | --- |
| pending_cache普通6例 | **6/6**，编译6.57秒/测试0.12秒/阶段7.057秒。cold resolved仍cache命中reads(1,2)不增长；原cache红现在warm读(2,2)/cache_hit=false，依赖后两项及target全true；原compact红现在明确Err、原edit block保留，依赖后target完整；report精确范围与两条实际live wire门均通过。 |
| 完整sync lib | **147/147**，测试34.54秒/阶段37.265秒；含core原2unit与本批新增4096重复report归并1unit，不另加计为150。原cache/store/reseed/pen/compaction/wire等既有控制都实际执行。 |
| 原transfer core集成 | **12/12**，顶层专用worker入口1 ignored但普通kernel lease父实际spawn/ready/OS锁检查/kill+wait；测试3.23秒/阶段7.065秒。core源f010/b301不变。 |

runner38631，三阶段自有PGID38680/38778/39278全部结束并signal0确认不存在；scratch为空，已立即归还唯一Cargo/native槽。总**51.419秒**，三阶段exit0/source_stable=true；最低external**6,257,414,144B**/internal**10,952,970,240B**，未触5GiB/阶段240秒/整体420秒guard。完整run.py/log/process/result/source/mtime与三个已编binary SHA保存；lib binary SHA **17a05d96879dafd89a9622f2cdc0049f1fa18198fb8c7dcd3f5d985b2a7f8174**。

本批保存独立提交 **ed46074626a94cfe272a50decfca5e601a1ad6a5**（`fix(sync): retain pending imports across cached reads and compaction`），父 **2211daa9be1dc3a7f9ab2ba87d50b9787d29a0c6**，恰16授权文件。提交前后24项SHA再次匹配，branch fix/quality-sync-chunked **tracked clean**，`pending-green-01/commit-receipt.json`记录refs/文件/来源。锁4436、Node link62eb/原modern testdbfd均未改；core ref保持独立，不混旧端fixture。

保证边界：这次修复已证的cache/compaction丢pending完整性，普通恢复仍允许合法分批pending，live wire已登记防止错误compact。**现有wire flush只持久化已经应用的op，收到的未解pending若只在内存中，重启保存/严格目标确认仍未闭环**；不能声称旧wire Ok代表持久Applied。这是后续单当前Transfer/checked apply与receipt的工作，不因本批6绿消失。当前也没有实现Loaded::apply_checked、目录fsync断电保证、Node/Pair新协议或25MB本体收敛；停止扩写等待root下一切片接口与独立审查/集成。
