# Q037 R1 candidate01 独立源码审查

2026-10-27，reviewer `node_test_review`，非作者。R1 **有阻断，保持open**；建议先修下列确定项并重冻，再安排真实定向测试。本审查未 Cargo/测试/gen、未改源码/依赖/公共文件、未提交。

基线树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，`fix/quality-remote-cancel@f0104d738f0de3162ea908d8ed672603ce0366d5`，16 tracked modified + 2 new files。先读中央STATE/PLAN与Q037两卡顶端实施契约，先 graph minimal/detect 后读实际diff与新源，再核作者说明。graph build f721、47 files、不匹配当前HEAD，detect称17 paths/0 affected flows不能覆盖实际18源，更不是无风险证明；无 `.codegraph`，未建图。

当前完整18源逐一与 external `Q037/remote-r1/candidate01-20261026T181911Z/source-freeze.json`、source副本一致，1,916 tracked/new输入逐项hash与现场相同；外盘与 internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-REMOTE-R1/candidate01-20261026T181911Z` 全22 files相同。新remote_cancel.rs SHA `964e2e4532e2837eac2499599bf85c61dfe9c69fff4c6dbcdc0ecf76841fa084`，new tests `01deae711a1ccd5d9c0db19560c02666c75c253f9f9eb3ef72c1011ba31ef7ff`。后续修订须新candidate，不覆盖01历史。作者已另获normal Node --lib check且卡记exit0；本次没有独审其运行证据，也未把normal库check等同测试源码编译/运行绿。

## 阻断一：已知task的delegate续输入在副作用前仍记成未知

**高，生产逻辑。** `crates/node/src/mcp.rs:337–352` 已把optional task放进 `message.task_id`，ResolveBinding同样按该task取得真实owned binding，却在before-send `track_child` 仍传 `None`。实际调用链是 MCP delegate → ResolveBinding{task:Some(T)} → track_child(task=None) → Bound Send{message.task_id=Some(T)}。如果远端已恢复T的工作、原Send reply尚未回来，Stop杀掉本地MCP worker，后面的补写Some(T)不会发生；R1 discover_children把这个link记 `NeedsAttention(UnknownTask)`，known T得不到Cancel。

这不是R2“首次Send不知task”问题：调用者已给T、owned resolution已经验证T。最小修复是before-send传 `message.task_id.as_deref()`，fresh Send保持None；不重发Send、不改task original parent。建议真实回归：原Task处于InputRequired，真实delegate{task:T,new request}继续它，在实际Send响应前hold，Stop/退休本地worker，正常lifecycle必须发exact T Cancel；断言原parent不被新run覆盖。现task_answer的before-side-effect Some路径不能代替delegate continuation覆盖。

## 阻断二：一个坏child阻塞整个run的所有合法兄弟

**高，生产公平/错误隔离。** `remote_cancel.rs:285–311` 对单个link解析/identity错误或已有receipt错误直接返回整页Err；只有整页成功才在 `:335` 推进scan_cursor。`step:555–556` 在挑选任何RPC前用 `?`调用discover/collect；collect `:360–366` 也因任一坏receipt中止整run。

可复现场景：同一已结束run有三个正式links，其中一个JSON损坏，其余两个是可验证binding+known task。无论坏entry的排序位置，discover最终Err，step始终未执行RPC；重复普通Read/tick也只重撞同页。即使前面的合法receipt已写出，也不能被选中。坏文件持续存在时两个合法兄弟永久饿死，与“坏child移队尾、不阻挡兄弟”契约相反。

最小修复：把单child失败变成可见且持久的小型NeedsAttention/安全错误记录，保坏源原字节，推进该child/page cursor；collect应能保留安全失败计数并继续其它可验证receipt。无法取得binding时不得伪造可执行目标；需要的只是明确unresolved、不可报all-known complete。整体目录无法读/资源超限仍可fail closed。建议真实worker回归分别注入坏source link、坏receipt，正常lifecycle下两个有效兄弟实际Cancel IPC到达，坏项保持NeedsAttention/unresolved且原字节保留。不要skip后假Complete。

## 阻断三：R0增强fixture的task Store与真实Binding查询不在同一目录

**高，fixture setup blocker，不能算新业务红或通过。** `khor_agent_remote_cancel_tests.rs:297` 用 `Store::open(scratch/controlled-peer-store)` 创建接受记录与持有Execution；新增 `:353–359` 的真正 `delegation_here(ResolveBinding{task})` 则调用 `Node::delegation_store`（delegation.rs:194–196），读取 `node.root/.khor/a2a`。当前root是scratch/store，所以两ledger不同。Store::get按caller/target/task精确查，当前task只在前一ledger，ResolveBinding必NotFound，第一run就退出，六Stop/四实hold/第五六到达永远不成立。peer.records只提供gate诊断，不会把task写进真实ledger。

最小改fixture通过 `node.delegation_store()`建立同一原task/Execution，仍持真实Working lease，并让gate继续调用真实delegation_here；不恢复伪造成功identity或伪terminal响应。原六Prompt/Stop、Pi ESRCH/mapfalse/claimfree、原四实际Cancel gate+四freshCLI hold窗口、release/reap、正常Read/reconcile与第五六实际Cancel到达断言在diff中保留；Saved投影改真实Stop Report durability属于已批准契约。修setup后必须跑原普通父，不能以其它十测试替代。

## 阻断四：持久RPC backoff没有约束CLI/Node重启成本

**中，生产资源/响应风险；不是RPC重复取消。** `step:559–578` 所有active children的next_retry_at都在未来时回空串；scheduler `:835–842` 只把字串retired视作退出，其他结果统一100ms重排。正常ticker约1s推进，async查询等待中的pump更频繁；所以处于30s backoff的一组run仍反复真实spawn CLI、Node::open、读取/扫描同一批receipt。四network owner的并发上限存在，但预算被无事可做的process churn持续消耗，未实现量子not-before的资源退避。

最小修复：step给小型明确结果（Retired/ReadyAt或等价现有string编码），带最早eligible时间；scheduler按该时间保存ready admission deadline，正常发现页不得把已排队同run提前。Busy/临时IO也给有限重试延迟，不扩新daemon/队列框架。建议真实owner测试：让四run都得到Working/timeout并进入较长future retry，保持正常tick，观察真实worker PID启动数在deadline前稳定且无RPC，另一fresh ready run能及时实际Cancel；到期仅按公平队列恢复。只断言远端RPC次数不足以检测这项抖动。

## 中风险五：Persist已启动但返回坏metadata payload时丢失unknown语义

`remote_cancel.rs:1051–1053` 在外层Worker成功返回raw、raw不能解析为Reply或没有snapshot时，一律返回 `Unavailable/InvalidReply`，包含persist=true的Stop/Retry。此时metadata worker可能已成功rename intent，坏/不完整的逻辑回执不能证明未落盘；契约应是 `PersistenceUnknown`。Status坏回执才是Unavailable。当前正常same-source成功路径不会自然造坏Reply，此项属于错误边界的确定分类缺口，不把它说成已观察数据丢失。

最小按persist区分decode/缺snapshot失败；同样校验snapshot.run等于requested run，避免任意JSON object被当作本请求回执。建议在实际metadata reply decode边界注入已提交后坏payload/无snapshot，Stop先local请求且结果Unknown，随后真实Status/Retry可查同intent；Status坏payload保持Unavailable，不能回Absent/NotPersisted/假ACK。无需更改Report外壳。

## 已核正向路径与十项新测试的准确边界

完整阅读18源diff及新remote_cancel1055行/new tests910行，跟到原Worker预算/kill/reap、Store ownership、HTTP Input/Receipt路径。正向设计可保留：Stop先request_stop再等待；4net+1meta且handles可join；run/child独立lease与exact attempt fence；成功Cancel Working才AckPending；Unknown/InFlight先Get、fresh Canceled才ConfirmedCanceled、其它terminal分开；binding的home/caller/agent/task核对；native稳定ID选DeviceInfo→ask_on；HTTP同一captured endpoint校验与请求，已有task用原receipt，凭据不写binding；小snapshot不塞Task历史。新增BoundAction为flat whitelist，Send保持原40s预算是已批准delta；GUI只两action映射，旧测试主要机械async。

十个新test的断言没有把Working/本地intent当ConfirmedCanceled，实际事件和当前Store观测是oracle；8个需要fresh CLI且ignored、native错身份/真实official HTTP A/B两项普通，尚不能计任何运行绿。HTTP测试确实比较原A增加2次、B无task RPC；fixture不代替实际ACP宿主终态。应准确标注两处测试强度：

- restart分支是写入InFlight fixture image，再用不同真实CLI读取并Observe，并没有在真实持锁owner中SIGKILL；只能证明ownerless InFlight状态恢复，不能声称真实kill窗口已验。
- late-attempt测试持真实reply时直接改private receipt绕过production lease，证明compare-and-commit fence，不是证明另一个生产owner能同时越过该lease。此测试有价值，描述保持边界。

当前新测试未覆盖坏child隔离、known-task delegate lostreply、真实worker backoff启动成本和坏metadata回执分类，故不能靠现十项通过关闭以上问题。

## 留open的边界与交接

R2未知task lookup/late-Send fence按契约明确未做，本卡不以其未做阻断R1，也不声称全部代理已停。未持久化Stop不承诺跨崩溃；no-resident/no-exit持续进展也非本片承诺。没有扩大到全本地FS攻击面、旧Khor兼容或完整历史迁移审计。

另外当前 `Metadata::Discover:519–524` 对坏pending引用直接skip：已存在权威intent的run可在restart后长期不被发现，显式Status仍可显示DurablyQueued。建议最小失败可见/按同hash已知intent恢复指针；不要静默当不存在，更不必全历史扫描。这项与坏child隔离可同一错误处理小片审定，尚未声称运行复现。

root可据上述五项先授最小修订，保持candidate01原快照；改后重冻独审delta，再以实际同源CLI+原R0/定向新case验证，按normal库check/测试编译/setup/业务红/绿分别记账。R1尚不建议提交/集成，main/ship无授权。
