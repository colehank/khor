# Q039 N1 candidate04：计量与部分 Exchange 独立复审

Reviewer：node_test_review（非 N1 作者），2026-10-27。仅源码/既有日志读取；不编译、不测试、不操作进程、不改生产/测试、不 commit。Q052 源与验证计划继续冻结。本卡只判断 N1，不将 N2 Pair/Vouch 尚未实施当 N1 缺陷。

## 已核来源与结论边界

- 先读中央 STATE/PLAN 与 Q-039-node-transfer 当前契约/03 raw/04 delta。采用 review-changes；CRG get_minimal_context 明示 graph=main@8704a503，不是当前树，query transfer_step=not_found。detect_changes/get_affected_flows 的调用参数被工具拒绝；没有据此得出无风险/0覆盖，直接落实际新源。无 `.codegraph/`，没有运行 CodeGraph。
- 实际树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，HEAD `5f2d5cf77ad05bd7e8ce2e11d0b90c36485c7ef3`，branch fix/quality-node-doc-transfer。candidate04 13 源逐 SHA 与当前全等，完整选择1083 inputs逐 SHA 全等；03→04仅 link_doc_transfer.rs 变动。
- candidate04 external `Q039/node-transfer-n1-candidate-04` 与 internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/node-transfer-n1-candidate-04` 共19文件逐 SHA 相同。manifest `f0edab3f10c6eca88c24b286276f306b8eb501d358d26b46c204314a13b409fd`，fullpatch `62d2949bf936139cfc007cd03f6d31bcea192f599712f2d9fa61bc9f33c569a7`；delta `9ba8749206a700aaacae30625d34e21c35f5901732e07f7b8b68225562220028`；当前 link_doc_transfer.rs `5f12f1e340f6f48e3017fd2301c669414cb57d2f027812b5d03fc04ce6bf7424`。
- 04确实只有三处 `BUSY.into()`→`Error::busy()`及删除常量；已有 From<&str> 是 Invalid，From<String>另按当前文案识 Busy。04修复 typed 分类本身正确，不需要扩大改造错误框架。
- 原 `node-transfer-n1-n0-green-03/n0/raw.log:800–830`：实际4096×64准备成功、raw_gap=25,447,938B、epoch1/1；首次public sync因硬 `document transfer state busy`停在原不接受无关错误的断言。父与ignored worker两段 FAILED 仅同一个普通父；没有首Seen/Pin或最终VV/content读数。result exit101/75.867s（父33.85s），guard=null，PGIDgone、cleanup=true。30s附近与receipt deadline相容，但raw没有分支trace，不能断言它必然来自deadline而非另一Busy分支。04尚未实测，03的11绿不冒04同源绿。

**源码结论：04分类修复可保留；下列B1/B2为可按控制流确定、尚未运行复现的生产问题，应在声称完整正常同步contract满足前修闭。它们不等于已证明是N0-03的失败根因。**

## B1：发生过的 body 在步骤后半段 Err 时不计入 visit 预算

位置：`link_doc_transfer.rs:253–265, 340–414, 421–437, 471–476`。

外层只在 `Ok((bytes,chunks))` 时增加 moved/quantum。已进入真实 exchange、甚至拿到合法完整 reply 后，push 的 Failed(Busy)、本地accept/receipt/receive的Busy，或其它后续错误均可由 `?` 直接返回，丢弃已收发body计量。

可观察的真实场景：A向B的某个大Doc每次发canonical256KiB，B持有该incoming的真实core Sink owner导致其receive返回typed Busy；Exchange仍可携带B→A的独立首块（handle:711–760先构造received，再独立offer）。A在take_push_state返回Busy之前已发送首块并收到反向首块，却返回Err，外层moved加0。若另一个大Doc正常推进，advanced=true令外层下一轮继续，此Busy Doc会再次发真实首块。实际body trace总量因此大于progress.moved，32MiB不是实际body的可靠边界。Write返回Busy/本地pull导入Busy同样漏计。

这不等于“每doc一个quantum可无限发”：Err立即break，该quantum仍停止；正常成功路径的双向Exchange最多2块，`quantum_chunks>2`防下一次Exchange跨过4块，且单Write/Read最多1块，四块公平边界在这些路径本身尚成立。错误是跨quantum/整visit累计不实，不能把所有公平性一起宣称坏了。

同一预算点另有小边界：253只为一个CHUNK_BYTES保留剩余visit空间，而未started的Exchange允许两块。可构造剩余一块空间、两向首块均为完整块的Exchange，使成功路径最多越32MiB一块。必须让调度在发送前考虑这次可能双向的最大body，不能等Ok后才发现越线。

最小修复方向：让步骤结局同时保留已发生/已保守预留的body计量与typed结果，失败也提交预算；网络await前为可能双向Exchange保留足够额度，Write/Read按单块。沿现pump/Work做小结果类型或局部计量，不另建调度框架。明确charge是实际收到/已尝试发送，重传也计入资源预算，不能只按成功apply字节。

建议行为验证：真实已配对A/B、至少两个不同Doc均有可分块body；用实际core owner制造首Doc Busy（或仅test cfg的窄控制点），第二Doc继续推进。记录真实wire request/reply chunk byte长度，断言失败首Doc的sent/received也计入moved；总body≤visit上限，下一Doc仍获轮次；另在visit剩余1块的边界验证不会放行2块Exchange。不要mock transfer_step的返回数来“验证”外层自加计数。

## B2：初始 Exchange 后本地 Busy 可把未接收反向 offer 的文档判为 done

位置：`link_doc_transfer.rs:387–408, 621–624`；外层Pending聚合287–300；`link.rs:10397–10415`。

具体路径：A本身没有待push的变化，B有合法反向offer。首Exchange成功返回；A先设置 `work.started=true`、`work.push=None`，随后 `take_receipt_answers(...).await?`。该函数即使两个列表均空也调用doc_step；此时若本地WAITING满或metadata gate冲突得到Busy，`?`在400–408给work.pull赋值之前直接返回。pull仍None，Work.done()因此true；外层Busy只break，不保留该doc的Pending，且不再调它。同步内容未取，可能还返回nothing-to-move/仅其它Doc的部分文本。

pending_doc_transfers不能补救本次状态：它在transfer_documents前扫描；此A没有incoming Receiving record（receive尚未执行），up-to-date本地offer也未留下outgoing。后续flush_doc_receipts没有该doc的proof可补。若压力在cleanup前解除且其它Doc安静，本次没有任何Pending。下一次新Versions仍能发现差集并通过远端固定Ready计划恢复，所以这里是本visit虚假完成/延迟，不声称永久丢数据或认证失效。

比较其它部分成功路径：take_push_state Busy显式started=false，远端已收的首块与其独立offer可在下一Exchange依固定manifest重试；本地receive Busy发生在work.pull已赋值后，下次status/Missing再Exchange可恢复。前述两条没有被当成不可恢复；漏掉的是receipt-answer这条提前提交started的分支。

最小修复方向：校验并保留两个方向的预期工作之后才能标首Exchange已完成；或者任何该窗口的Busy均重置started并保持Pending以重做幂等Exchange。不能只吞Busy后继续报done；也不应靠人为烧票/放宽预算或无限重试。

建议行为验证：B有大Doc、A为单向接收；让真实Exchange已成功返回后、A开始take_receipt_answers之前持有A私有metadata gate（窄cfg(test) barrier可协调实际冲突，不加产品env故障后门）。要求首visit结果含该doc Pending，固定target未覆盖时不能成功nothing-to-move；释放gate后同Node/重开Node再次public sync按同manifest完成，验证真实内容、epoch和VV覆盖。

## 仍缺的三类最小真实验证

现有11 ordinary的源码范围是4 State、3 wire、metadata journal/gate等有限单元和Pending描述；`doc_transfer_state/tests.rs:63`在receive已返回Complete、receipt已持久后才重开。它们没有真实quiet请求数、双向fair wire trace或postflush/prereceipt窗口证据。现任务卡也明确承认这些缺口；不能把实际编码/文本单元当这些行为已测。

1. **quiet1Versions**：两个私有非hosting Node/自有relay，公开pair并先真实同步至稳定、收据清空；固定业务Doc/readings与实际连接后跑一次public sync。只在实际admitted dispatch/wire边界用test-only observer记录本peer的Request种类（含early opening，不能只数普通exchange_on），断言恰1 Versions、0 DocTransfer/旧Sync/额外continuation/读数刷新，并fresh reopen确认内容/epoch未变。setup发生的pair/sync请求先排除。不要拿Promise或伪peer数请求。
2. **双向fair trace**：A/B对两个以上合法Docs各自生成互不包含的足够长真实历史，固定双向target，公开sync；捕获实际DocTransfer请求和reply内chunk长度/manifest/offset。Exchange两向首块共同计2，Write/Read各按实际payload计1，失败重传也计；按registry顺序确认每一选中文档先获一次轮次，再允许下一轮，每doc每quantum≤4块/1MiB，整visit实际body≤32MiB，否则Pending。保留小Seen/Pin在大Doc未完成时已fresh persisted的独立oracle。结合B1 Busy变体及剩1块边界，不缩小canonical256KiB来过测试。现有N0仍照原4次visit/VV/content/SeenPin oracle跑，不能修改它替代trace。
3. **postflush/prereceipt fresh reopen**：真实Loro body经state.receive→context.apply/apply_staged→Loaded.apply_checked→flush成功，在doc_transfer_state.rs:680之后、Complete receipt的692 save之前，通过仅cfg(test)单次barrier让子进程通知父，父真kill/wait；保原Importing journal、stage。全新进程fresh load Doc与State：target/base已在磁盘，status应persist AlreadyHaveEquivalent再返回Complete（不可凭旧Loaded的on_disk声称Applied）；fresh业务内容/并发本地op/epoch保留。重复status proof稳定、正确角色Confirm/Ack后exact stage/records可清，别的manifest不动、lateWrite不重建。父后验所有PID/PGIDgone与私有scratch。更小的第一步可在该窄点返回私有test错误后drop/reopen，证明同一窗口；但它不替代真进程中断收据。没有现成故障注入，需root另授最小test-only hook/worker；本轮未新增。

fresh恢复源码正向依据：Importing+before先于apply持久（674–680）；apply_staged委托checked flush；恢复completed读取fresh context.version（752–768），只有base+target均覆盖才造AlreadyHaveEquivalent并save后返回（720–750）。这里未确认新的on_disk虚假持久化缺陷，但没有故障窗口runtime证明。不要为补证据复制blob布局、另造事务层或引入产品故障环境变量。

## 本轮交付

仅此新卡；源码13/完整1083仍为candidate04。建议root保留已授权04 unit→原N0用于获得直接结果，同时把B1/B2与上述三个实证缺口列为N1收口条件。任何后续修源要重新freeze，不能把03/04旧binary或旧单元绿冒新源。无新运行结果，不主张N1、N2或全goal完成。
