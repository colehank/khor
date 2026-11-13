# 本机请求的期限与取消

2026-11-10，设计草案。客户端较短期限回归42729827780已取得5绿/1红，42733228660同完整测试块客户端修正已6/6绿；下述新帧和服务端租约均未实现、未启用。
执行状态以[STATE](STATE.md)为准，验收先记录在[ACCEPTANCE](ACCEPTANCE.md)。

## 已核源码

`ipc::exchange`把`Handoff { cookie, op }`经`proto::encode`编码，write_all后shutdown发送方向，再read_to_end等回复。
`proto::encode`实际是MessagePack（rmp_serde），MAX_FRAME为6MiB；content另有更小的序列化限额。
`Node::handle_handoff`读至EOF、核cookie和payload，再执行；正常请求就有EOF，不能据此取消。
`call_refreshing`目前只沿Op自己的档位包timeout，只有TCP拒绝连接能刷新重试，发送后不重放。
resident在完整解码后重新取Op档位减500ms，已有task-local deadline能取早，但当前IPC没传调用方剩余预算。
`EndpointFile`是owner-only JSON，记录端口和32位十六进制cookie；同一TCP listener有现成的前台恢复/替换发布机制。

## 首个独立修正

客户端入口只建立一次`by = min(调用方当前期限, 入口时刻 + Op原档位)`，所有连接/拒绝刷新/序列化/交换共用它。
连接前和序列化后检查是否已耗尽，回复完成后也检查期限，防止已过期但立即Ready的future被当成功。
原15/25/30/60/280/3600秒工作档继续存在，长操作不统一裁成28秒。
先用40ms父期限和真实mute loopback listener取得红，再修客户端；它不证明legacy resident已经停止工作。

## 单listener的新租约帧提议

保留当前MessagePack Handoff/Op/Reply结构与cookie准入。EndpointFile增加可缺省的`ipc_max_version`；缺失/0/1走现有协议，2及明确兼容后续版本才选择租约帧。
现有公开legacy调用保持可用；Node读取端口/cookie时一并读取版本，拒绝连接后的刷新也必须原子更新这三项。
服务端支持完整旧协议后才发布能力。发送任何新帧字节后不降级重放。

新请求提议：固定magic `FF + KHORIPC2`，接u32 payload长度、u64剩余微秒（整数均big-endian），再接原MessagePack Handoff。
首字节FF是MessagePack负整数，无法成为合法Handoff struct的array/map，故可与合法旧帧区分；最终实现须实际codec正负校准。
长度沿用6MiB及原content限额，先验长度再分配，cookie仍在执行前验证。
剩余时长在最终cookie序列化/校验之后、实际写出之前取值，已耗尽便不发送。

| 阶段 | 旧协议 | 新租约协议 |
| --- | --- | --- |
| 请求边界 | 发送端正常half-close，server读至EOF | 长度明确结束，client继续持有发送方向 |
| server工作时 | 原有期限语义 | server请求读取方向出现EOF或额外数据即结束本次待答任务 |
| 回复 | 原Reply编码，server关闭发送方向 | 同一Reply编码及server发送方向EOF，无额外业务协议字段 |
| 兼容 | 已装旧CLI/旧resident继续有效 | 只在实际能力声明后启用；坏magic/长度/预算/cookie均不执行 |

EOF在新协议表示caller租约结束，因为它已不承担正常请求定界；两种含义按版本明确分开。
预算用剩余duration，不能序列化Rust Instant或假定不同process的Instant表示相同。
接到header即建立本地budget_by，收完整body不能重新领取完整duration；解码后再与原Op安全上限取早。
caller自身绝对截止仍有效。传输/调度滞后单独记录，不能把relative duration称为跨进程共享时钟或实时零误差保证。

服务端取消选择须覆盖执行、回复序列化、等待写出和content permit生命周期；只包住run_handoff不够。
客户端离开后不能让一个阻塞回复继续持有buffer/permit。取消与deadline先于准备执行的工作检查，完成后也检查期限。
原Op减500ms的工作上限继续存在，回复由总caller预算约束；不强制把短caller预算都再减500ms。
任务所创建的持久job/session仍按其本来所有者管理，不能把取消查询变成删除工作。
发送过的写操作可能已经执行，结果未知不能当未执行，重连与版本变化都不触发隐式重放。

## 验收与分批接入

1. 保留旧half-close正常答复、拒绝TCP才刷新、答复丢失不重放及所有长档；先闭合客户端短期限红绿。
2. 实际codec覆盖新旧帧、缺失能力、截断/超长/坏cookie/未知版本、已过期入口及传输前后预算消耗；错误无副作用。
3. 真loopback验证新租约正常答复、caller drop/超时、body未完断开，以及重复通知不会重复取消；legacy EOF仍正常执行。
4. 真实Node验证取消解除阻塞响应和32份content预算，短取消保留健康共享wire、其他请求继续；保留F7真正不应答退役。
5. 分process/独立runtime核剩余时长及EOF所有权，准确旧/新binary交叉兼容；不能用同process task-local值替代这一项。
6. QUIC端远程执行另审计SendStream::stopped及已接受写/持久job的语义。IPC caller结束不自动等于远端执行停止，0-RTT准入不扩大。

只在前一层合同绿后接下一层。默认端口恢复、个人信任模型、已有写入语义及跨平台TCP结构保持，新增协议的数值界限与格式以最终冻结合同为准。


## 编解码分批实现（01:20UTC）

新增ipc::lease独立helpers，尚未接入EndpointFile/真实client/server，不发布v2能力。magic为9B，长度4B、剩余微秒8B，共21B头；头到达立即锚by，body消耗它。完整frame admission另取已有DEADLINE_LOCAL=5秒上限（与caller取早），包括legacy/新帧，避免未认证半个header永久占用任务；有效旧请求仍以正常EOF结束。预分配前核长度，额外1B检测legacy超长。未知magic版本、零预算/零长度、截断均拒绝；cookie仍由实际handler在执行前核，不由codec假称已认证。

within helper仅对有by的新帧监测读取方向EOF/额外字节/错误，并与到期优先竞速；legacy不把EOF当取消。future持有工作/编码/写出/permit直至完成，取消整体drop；测试涵盖未poll前取消和阻塞回复owner释放。全部是helpers层，尚未取得远端结果，更未取得32席位/分process/真实Node/远端执行取消证据。只有本层远端合同绿后才接入真路径。


## 首次真实接入候选（2026-11-10 01:44UTC）

42737166916准确f466d990/treeb6d853dc/control4bf7e6eb已取得helpers+legacy客户端14绿与其他便携全绿；真实32回复在预条件后等待5秒不能归还permits，行为红。整个wire_tests.rs SHA945c22aa在修改真实路径前冻结。此前42736367611 attempt1源下载截断/attempt2格式失败分开保留，不计行为红。

开始接入同listener双协议：新EndpointFile公开ipc_max_version=2，缺省0仍legacy，字段语义为向下兼容最大版本，双方选择已知v2（max>=2）；旧公开ipc::call(port,cookie)继续legacy，新Node端口/cookie/版本作为Target原子刷新，只有拒绝TCP才可刷新。request长度结束后v2 client不half-close，原caller绝对期限仍覆盖全部交换。server用既有helper读取、先核cookie，再在within内持有执行/编码/阻塞写/所有response permits；legacy正常EOF继续执行。按header接收时刻锚remaining与Op上限，run_handoff_within原500ms工作余量继续取早，不对短caller预算另减500ms。原wire fixture逐字不改；候选未远端验证，不把它称已取消QUIC远端执行或跨process预算已验收。


首次真实候选结果：42738040751仅格式失败，精确remote patch随后494e5241已应用；42738965522（snapshot1b22a3b0/tree8afa1dc2/control4bf7e6eb）fmt、IPC18及便携全绿，但原32回复用例在tokio worker栈溢出SIGABRT，不能记资源绿。候选将大型run_handoff_within分派future在进入lease组合器前Box::pin，所有权/取消仍归同future；这是待原整fixture验证的最小修正假设，不增加线程栈或改变32/5秒/barrier。错误与whole SHA945c22aa保持。
