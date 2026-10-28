# Q037 remote cancellation：持久排队、公平step与可查询receipt

## R1实施契约收敛（2026-10-27，待生产授权）

以下替代本文早期R1草图中的Saved真相/同槽持久化想定；后文保留历史分析。root已选择**最多4个网络CancelStep + 全进程最多1个短metadata lane**，全部用现有可kill/reap/join的Worker；没有新daemon或无限线程。R0真红独立commit `f0104d738f0de3162ea908d8ed672603ce0366d5`，保 `audit/remote-cancel-r0-red-20261027`；原三源SHA与测试/commit/blob一致，生产尚未写、无本轮Cargo。

### 选择1：异步Stop、独立持久真相、显式失败与历史retry

最小签名与Action：

```rust
pub(crate) async fn khor_agent_here(&self, action: Action) -> Result<Report, String>;
// khor_agent_on/with原本已经async；三个生产here调用点直接.await。
enum Action { /* existing */ CancellationStatus { run: String }, RetryCancellation { run: String } }
```

- Stop在入口先按实际run从Running取得控制并`request_stop()`，**早于原有reconcile/config读取**；本地执行器独立退休，不等metadata或远端。然后异步等待去重的`PersistIntent` metadata结果。该lane忙或5s预算/文件错误时返回成功的Report（本地Stop已经请求），但data明确`durability: NotPersisted { code, may_have_landed }`；绝不把该情况返回为DurableQueued/remote ACK。若rename后sync失败等结果不明，may_have_landed=true；重试幂等验证/重写同intent，不自建事务服务。
- metadata写完整且完成现有私有文件写入+sync/rename约定后，Report.data才可为`DurableQueued`。全局metadata单owner，不占四网络槽；同run Stop/Retry合并，忙则明确NotPersisted/Busy，不能无界排一个等待future/线程。此进程仍保唤醒，但**未持久化的Stop不承诺跨崩溃保留**。
- `CancellationStatus{run}`和`RetryCancellation{run}`均走原认证/选定home的KhorAgent路由；只收run，不收root/caller/agent/task。metadata内核safe-token、Saved.id/执行home和同权限。Retry接受合法历史run（不再要求Running存在），使NotPersisted/已结束run仍能创建或唤醒同intent；它不重新Prompt/Send/Answer，不重绑身份。永久identity/authorization错误不会因retry变成成功。
- Status是metadata只读snapshot；lane忙返回`Unavailable/Busy`或带as_of的明确缓存，不能伪装最新。不存在intent和读盘失败分开。Report.data装typed小snapshot转JSON，**不改Report外部DTO**。GUI core只增加`cancel_status`/`cancel_retry`字符串action映射，不增控件。
- **权威文件**：`.khor/agent/cancellations/<hash(run)>/intent.json`及`children/<child-key>.json`。Saved/data_json不再写取消真相；execute只写原本local终态与其它元数据，删除现同步一次性schedule/pending投影。先以Report.data查询投影交付，避免另一个writer改Saved而被execute旧快照覆盖；后续若需要machine_log投影，必须读权威snapshot并带revision/as_of，不能让Saved反向覆盖它。

最小schema（新版本1，无legacy双路径；引用不含token/history）：

```text
Intent {version, intent_id, run, execution_home, requested_at_ms, reason: Stop|RunFailed}
RunProgress {intent_id, revision, child_cursor, scan_cursor, scan_complete, local_terminal_observed}
ChildReceipt {version, intent_id, child_key, binding, task?, phase, attempt_no, attempt_id,
              next_retry_at_ms, last_ack?, last_observed?, safe_error_code?, revision}
Observation {task, target_home, caller, agent, task_state, revision, observed_at_ms}
Phase = Queued | InFlight | OutcomeUnknown | AckPending | ConfirmedCanceled
      | TerminalOther(state) | NeedsAttention(code)
Snapshot {run, execution_home, as_of_ms, revision, durability, counts, active, backlog,
          scan_complete, all_known_children_canceled, unresolved, late_input_fence:false}
```

Intent创建幂等；独立run/child短try-lock和attempt_id fence。每step lease只由其worker持有、最多原5s预算，拿不到立即Deferred，不碰executor claim。先落InFlight再RPC；回包核身份、重读intent/attempt后才commit receipt。强杀或reply前后丢失时，重启发现free lease的InFlight改OutcomeUnknown，下一步先Observe，不凭PID/超时确认停止。单个原子小文件写/现有sync/rename即可；summary可从child truth重建，不造多文件WAL框架。拟定上限：intent4KiB、child receipt16KiB、snapshot128KiB；不把MCP原有tool输出cap改小。

### 选择2：fresh ChildLink绑定实际执行home/caller与原HTTP receipt

- 新typed `Binding { home: DeviceId, caller: DeviceId, agent: String, protocol: Native|Http { endpoint_digest } }`；ChildLink持它与message、task，可留machine仅诊断。caller取原MCP执行home的Node.device，不取入参；HTTP digest取无凭据的canonical(card_url, interface)，不复制credential/card/history进link。child-key以稳定binding+input生成。原parent归属继续按同一稳定input绑定，不允许retry用新parent覆盖。
- 在delegate/known task_answer副作用**之前**用实际选定home解析binding并正式track。native目标用选定DeviceInfo.id而非展示alias；HTTP由该connector的真实owned task receipt/新input endpoint快照解析。增加`delegation::Action::ResolveBinding {agent, task?}` / `Answer::Binding`和`Action::Bound {binding, action: Box<Action>}`（仅Send/Answer/Get/Cancel白名单），复用原proto/IPC Delegation envelope，不额外新增网络服务。
- Bound接收端核`binding.home == self.device`、`binding.caller == authenticated caller`、inner agent一致；已知task仍经原Store/HTTP owned receipt校验，caller不能伪造。发送端用**稳定ID选DeviceInfo并ask_on**，不先把ID转换channel后再次按别名解析；resident IPC也按bound.home路由。reply核home/caller/agent/exact task，原Record.parent可能属于旧run，不错误要求等于新resume run。
- HTTP现有Get/Cancel已用`Receipt.endpoint`，应保留；fingerprint校验放在选定receipt/endpoint的同一执行路径，使用那份已捕获快照发请求，不能先校验再重读可变alias造成TOCTOU。新Send/续答必须在其原input/task gate内校验所选endpoint；新任务配置改绑则发送前明确拒绝，已有task继续原receipt服务；同一服务合法credential轮换沿现逻辑。alias改到B后不会对B取消原A任务，disconnect也不丢原receipt引用。
- R0原测试只走production track同路径，没有完整delegate；R1只做async等价await并增强真实binding/native-owned-receipt夹具，仍保六run/四真实扣包Cancel/第五六实际到达的oracle。可把controlled peer的Get/Cancel改为调用真实`delegation_here(authenticated caller, ...)`，用真实Store持Working Execution lease以免recover将夹具误判中断；仍不冒称ACP宿主执行终态。不得通过拒绝所有identity而让测试假绿。

### 调度、正常生命周期与scope

`remote_cancel`新模块拥有4个有限网络owner和1个metadata owner/已完成结果，统一每root/run去重。内存ready最多256个step单位，溢出保durable backlog；metadata发现/恢复每次最多32项并保存cursor，不以read_dir顺序当公平顺序、不在reactor扫描。run轮转，每run最多一活step；run内child轮转，坏child移队尾/有界backoff（1/2/4…30s），不阻挡兄弟或其它run。单个step只有一次逻辑Get或Cancel，无List循环、无Cancel后同step再Get；bound路由不做应用RPC自动重发，传输建链不算第二个工具动作。初次known task选Cancel；成功Working等非终态reply才AckPending，直接Canceled可确认；AckPending或不明delivery下次Observe，fresh Get Canceled才ConfirmedCanceled，Completed/Failed等归TerminalOther；flag/Working本身从不确认ACK。

bootstrap也不能依赖测试额外ticker：PersistIntent可在持久写成功后返回同一worker内有界的首个32-link发现页；扫描失败不撤销intent，返回durable与discovery pending分开。async Stop等待期间推进同一memory pump，消费metadata结果并admit ready step；finite owner线程独立于测试同步等待推进。RunFailed/execute local-retired由同步execute发memory事件并kick同一metadata owner，不在execute做FS/等待它。没有另外的scheduler后台循环。

Stop先持久intent；网络推进维持本地worker退休后的登记顺序。execute完成只发内存local-retired wake，metadata扫描/恢复按本地终态观测推进；正常启动reconcile把旧owner已失联的run标interrupted后可恢复known links，但不能据此声称其晚到未知input被fence。子link尚未完整扫描或未知task时不报all-known complete。Read/reconcile只追加内存wake/pump，既有serve startup与wire ticker正常驱动；新FS/scan/RPC都在owned worker。serve给pump已有paused/resting原子状态；paused/resting只reap与metadata查询，不admit网络；单次进程非resident路径由metadata/step内检查持久pause，状态不明就defer。没有resident或进程退出时不承诺继续跑，下一正常入口恢复，不能加常驻daemon。有限handles由下次pump收取/join；OS退出回收不称全部显式join。

R2未做：task=None的link保持OutcomeUnknown/NeedsAttention(UnknownTask)，不List扫页、不重发Send/Answer；没有late-Send tombstone，外部服务无法证明终态时继续Unknown。R1的all_known_children_canceled仅指已有稳定known links，**不提供所有外部代理已停止的总保证，Q037总卡保持open**。

### 实际调用方与最小allowlist

图已先查：当前图built f721、head f010不匹配，找到here符号但不当覆盖；实际rg核全仓调用。生产直接here仅`link.rs`三点：Request::KhorAgent handler3824、on local6092、with local6114，均在async函数内，直接await；GUI `apps/gui/core/src/lib.rs:2397`已经await on。`reconcile_agent`可保持同步原签名，只新增memory wake；其GUI read/machine_log、serve startup原调用不用为新框架全迁。`execute`同步线程只发wake，不await、不做新metadata FS。唯一无法直接await处均测试：`khor_agent.rs`三test、`agent_accounts.rs`一test、`mcp_worker_tests::configured/environment_fixture`及其调用测试、`khor_agent_cancel_tests`隔离worker、`mcp_cancel_tests::exercise/worker`、R0 exercise/worker；迁成async helper/`#[tokio::test]`，outer纯process父与Drop cleanup不迁，不留同步生产wrapper。

生产allowlist：`crates/node/src/{lib.rs(仅mod),khor_agent.rs,mcp_worker.rs,new remote_cancel.rs,delegation.rs,mcp.rs,a2a_http.rs,link.rs}`；`apps/gui/core/src/lib.rs`仅两action映射。Action扩充沿原proto/IPC serde包络，无需改Request/Reply外壳；若实际codec测试揭示必要额外schema源，先报最小delta。测试allowlist为上述六测试文件+new `remote_cancel_tests.rs`，以及`crates/node/tests/khor_agent_wire.rs`、`apps/gui/core/tests/khor_agent.rs`对应新Action/Report透传行为；现Pi/ACP默认process与其它106 fixture不扩语义，旧测试改动只async机械迁移。

最小真实验证：原R0红→绿（六/fourcap/第五六实际IPC）；同run slow child与另一run/兄弟公平不饿死；四网络扣包时第5/6 Stop仍取得真实durable intent；metadata锁/写失败先local退休且NotPersisted、历史Retry成功；重启读取真实intent/InFlight lease释放后Observe恢复；真实Working Cancel reply仅AckPending，丢reply保持Unknown，后续fresh Get Canceled才confirmed，Completed归TerminalOther；真实native caller/home错配拒绝、HTTP alias A→B后只访问原A receipt；迟到旧attempt不能改新receipt；pause/rest不新发Cancel。均保正式worker/schema/fuses与断言前owned cleanup，不以内部队列长度/flag替代实际RPC/持久记录/进程回收证据。

状态：只读提案，未改实现源码、未Cargo、未运行新fixture。作者 mcp_worker_owner，2026-10-26–27。root负责接口决定/授权下一片；本卡不关闭整体Q037，不重开已收口106测试批次。

## 固定现场与证据边界

读取真实集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，HEAD `4493ed3c54f18cd562759a396f3cce90191ea795`，tracked clean。它已按7550 fixture→3e676生产顺序集成MCP。中央STATE仍有9274和旧作者进度，本卡按root最新派发与实际HEAD判断，不覆盖STATE。唯一Cargo/gen槽由root占用，本作者没有启动进程/网络fixture。

沿用explore-codebase图优先：semantic search schedule_cancel为0且图明确main8704，不能覆盖4493；当前树没有.codegraph，回退实际源码。10个关键源SHA及clean/HEAD记录在 `Q037/remote-receipts/read-only-source-4493.json`。以下“现有gap”是源审成立及可执行复现设计，**尚无本片新增运行红/绿**；106旧绿不等于remote completion。

读后复核时root已将集成HEAD推进到`f721a2c3ac0e12cb1faf5dd9634015dea1054a4b`；上述10源SHA仍全部逐字相同、tracked clean，after回执为`read-only-after-4493.json`。未把其它模块的新集成冒充本提案运行证据，当前事实对这10源仍成立。

## 当前实际语义

| 来源 | 当前事实 |
| --- | --- |
| mcp_worker.rs:481–526 | 全进程SWEEPS Vec，登记时join已结束者；最多4个活动handle，第5个直接Err。线程里spawn失败直接return，poll的Ok/Err结果全部丢弃；每worker总5s，内置fuse也是5s。没有待运行队列/完成receipt。 |
| khor_agent.rs:448–457、765–789 | Stop只找到当前Running并请求本地停止；execute退役本地worker后登记一次sweep。Saved.data_json仅写pending/confirmed=false、scheduled或not_admitted及即时登记原因，之后不会因真实Cancel结果改变。父status=cancelled是本地终态。 |
| khor_agent.rs:471–513；link.rs:1343–1345 | reconcile只把未结束run改interrupted或重新发布display receipt；serve启动调用它一次，没有恢复remote cancellation。结束run再次Stop会因无Running被拒绝。 |
| delegation.rs:629–670；mcp.rs:325–346、364–407 | durable link按parent run目录与(machine,agent,message)摘要保存，task可空；delegate发送前track，task_answer已在副作用前track已知task，响应后补actual task。此语义保留，不能用新parent覆盖原task parent。 |
| delegation.rs:674–775 | 顺序遍历read_dir，未知task反复分页List，接着Get/Cancel；一个阻塞请求耗尽5s会使整个worker退休，后续link没有机会。已找到task不回写该link，Cancel结果没有分类；坏link可整体提前Err。 |
| Store::request_cancel，a2a/store.rs:450–474 | Working只记intent；Submitted/InputRequired/AuthRequired为静止态可立即Canceled。exact task取消，不是对整个context发Stop。 |
| gui_host.rs:1176–1184 | 实际host按exact delegated task观察intent并取消同代次operation；后续任务不可因迟到取消被停止。 |
| a2a_http.rs:803–824 | 先在本地ledger写cancellation_requested=true，再发HTTP cancel_task；只有成功reply校验task ID后才保存返回task。**Get看到cancellation_requested=true不能证明远端收到了Cancel**。 |
| apps/gui/core/src/lib.rs:2338–2445 | KhorAgentView.data已有unknown JSON；普通read主要本地reconcile，machine_log读已发布record。现无按历史run查询最新cancel receipt的action。不能把静态pending日志当实时远端状态。 |
| link.rs:1484–1498、1902–1938 | 已有约1s wire ticker与5s sync ticker/启动入口。可在既有生命周期中驱动有界pump；暂停/休眠时不能偷发网络取消。不能把新的同步scan/RPC放在reactor。 |

## 可直接落真实首红的gap

1. **>4run lost admission**：同一scratch home连续6次Prompt→Stop，各有真实durable link，4个实际取消IPC请求进gate后扣reply。第5/6的父本地仍须收口；旧版保存not_admitted，4个owner退休后也没有任何自动补尝试。oracle是第5/6的真实Handoff从未到达，不能只读Vec.len。
2. **poison child阻塞其它child**：同run放3个真实links；不假定read_dir顺序，实际先到的Handoff动态扣gate，另外两个目标可立即答复。旧5s整worker退休后后两者仍未收到取消，且下一轮没有持久cursor。修后保持首gate扣住，后两者必须在有界后续quantum被实际尝试。
3. **receipt永远pending**：真实ACP host接受Cancel后先Working+intent，再实际retire到Canceled；当前父data_json两时刻都pending且没有可查询差异。另让目标先Completed/Failed或明确拒绝Cancel，当前也没有分类结果。Working ACK绝不能提前变成ConfirmedCanceled。
4. **restart lost owner**：在durable pending已保存、实际Cancel已入场但receipt被扣时结束自有执行进程；释放fixture后用新Node/resident启动，不发新Prompt/Stop。旧reconcile只修display，没有补observe/cancel。必须先把残留自有worker/fuse和IPC fixture回收，不丢300s线程。
5. **lost send reply/未知task**：真实delegate已被接收、响应丢失，只有message link。当前每次恢复List从头；native有caller+agent+message→task的真实去重索引却未开放只读lookup。HTTP recover_input只扫4页，每次重置cursor。不得重发Send/Answer来“找回task”。

建议先取1或2窄红；每轮只有确定Handoff barrier、guard/cleanup成功后才能算业务红。cap/setup/编译错误另记。第3–5按同一schema逐个加真实case，不先扩大mock集合。

## 推荐最小实施片R1：已知task的闭环，未知task明确Unknown

R1同时覆盖持久排队、>4run、公平、restart、结果分类和查询；不把未知input伪装成已确认。后续R2补高效、可恢复的未知task解析，不重做本地worker/Pi/ACP监督器。

### 持久状态复用

- `Saved.record.data_json.remoteCancellation`作为**run意图和有界摘要**：requested、revision、requested_at、counts、next_retry_at、last_progress_at。保留所有其它data_json。先把意图和父本地终态以现有save原子发布，再wake scheduler；容量满只是Queued，不是丢请求。save失败必须返回/记录未持久化，不声称已排队。
- 复用每个`ChildLink`文件存小型`cancel`状态：phase、attempt token/number、last operation/result code、已知task identity、远端state/revision、时间与retry_at、未知task的page cursor。**不存整段task history/artifacts**，不把所有children塞进父CommandRecord。坏文件是NeedsAttention并计入未完成，不能continue后汇总Complete。
- 复用link摘要作为稳定child key；run/child cursor推进和attempt reservation必须在发RPC**之前**持久化。超时/worker死亡留下InFlight/OutcomeUnknown，下一轮先观察，不能重新从第一个child/第一页开始。
- `.khor/agent/cancellation`只需派生queue/cursor和短持有的try-lock，intent/child receipt仍在原Saved/links。派生index损坏/缺失可从持久intent重建，不能导致请求被认为不存在。所有新增文件操作/锁/恢复scan仍在现有可回收worker内；不向reactor加阻塞File.lock。
- 不复用executor.lock作为remote lease；下一Prompt的claim必须可立即重取。多进程同home用专门run/child try-lock和attempt token fencing，避免两个resident/CLI同时取消同一child或旧完成覆盖新retry。

### 有限scheduler，沿现有生命周期推进

- 把现max4有限owner登记改成带(root,run,attempt)的槽和FIFO ready队列，重复wake合并，不再“满4就遗失”。持久intent是恢复真相；内存队列有明确预算，溢出只保持durable Queued并报告容量，不能删除intent。槽完成必须收取结果、join后释放。
- worker改为`CancelStep { run }`：一次只选一个child和一个阶段RPC（已知task可先exact Cancel，未知delivery先Get；无需一次扫完所有link）。保留现5s硬预算，实际量子开始/结束可观测；每轮run轮转，每个run内child轮转，offline/slow child进入backoff到队尾。一次坏child不能中止整个run。所有state写和RPC均由这个实际worker执行。
- 复用serve启动+现有wire ticker驱动pump；pump本身仅内存登记/reap/有界admission，scan/RPC不能在ticker上同步做。复用原最多4个有限owner线程，不新增长期daemon、额外无限channel或持续新线程。也可在cancel_status查询/明确retry时wake。
- 每次owner退休后只产完成事件并归还槽；持续推进由既有ticker负责，不在完成回调无限自生下一条线程。paused/resting时仍reap、可读状态，**不发新网络RPC**；恢复后按原cursor继续。无resident或机器关机时不承诺继续执行，下一次现有Node启动恢复。
- restart读取durable Queued/InFlight/AckPending：先检专属lease；旧worker仍活就Deferred，不根据裸PID/超时猜已死。lease释放后旧InFlight变OutcomeUnknown，先Get同一task。绝不恢复模型Prompt、Send或Answer。
- 父本地cancelled/failed与remote run aggregate分开，不等待所有peer完成才释放claim。已有5s是单step资源界限，**不是“5s后远端都停止”**；正常但超过预算的peer仍Unknown/backoff，不能转成成功。

### 结果状态必须分清

| child状态 | 唯一可接受证据/语义 |
| --- | --- |
| Queued / InFlight | 本地intent已保存/有限owner持有attempt；不是远端收到 |
| AckPending | 成功的exact Cancel RPC返回匹配Task identity、非terminal；仅确认该次请求被答复，仍需观察 |
| ConfirmedCanceled | 匹配身份的实际Cancel/Get返回TaskState::Canceled；保存观察home/task/revision/time |
| TerminalOther(state) | 实际Completed/Failed/Rejected等终态；远端工作结束但不是“已取消”。Task Failed与Cancel请求失败不混用 |
| OutcomeUnknown | timeout、IO断开、worker被回收、丢receipt、暂时NotFound/未知input；保留link和最后attempt，不能当gone |
| Failed / NeedsAttention(code) | 明确权限/协议identity错误、损坏link、取消拒绝等；不伪确认，不删除证据，允许同权限显式retry |

`cancellation_requested`只能作为intent观测字段；尤其native HTTP先写flag，不能据此从Unknown升成AckPending。task_answer的新parent link指向既有task时，原Record.parent可以是旧run，不能要求它等于当前run而误拒合法resume；以保存的link/调用home/caller/exact task授权。

同一task的Cancel重试只能来自同一durable cancellation intent，先校验新Get的目标身份，绝不重新Send/Answer或对context发无范围Stop。未知Cancel delivery先观察；对外部服务无法确认的情况继续Unknown，不能从本地flag猜ACK。永久错误与可重试传输错误用code区分，不把任意Err字符串自动分类为权限失败。

### 可查询状态与权限

- 增`khor_agent::Action::CancellationStatus {run, after}`；沿现`khor_agent_on(machine, action)`认证/选择home路径，返回现`Report.data`（GUI已有unknown data，不需新界面）。App core映射`cancel_status`，普通`read`不偷偷变成远端全量RPC。可增同权限显式`RetryCancellation {run}`，只wake已有intent。
- 只接受合法run ID、校验Saved.record.id及owner/execution home；payload不收任意root、caller、agent、task参数。child身份取持久link和已认证reply，不能让查询变成跨home取消能力。old run不能改由“当前新config.home”执行。
- 返回小型typed snapshot：run/revision、as_of、phase、counts、next_retry、worker/queue情况、分页child状态与安全reason code。查询页有明确条数/字节预算；不回传凭据、endpoint token、私有文件路径、完整history。
- run摘要投影回原CommandRecord.data_json供既有machine_log查看；专属receipt是查询真相，必须带revision/as_of，不能把旧投影当最新远端证据。snapshot读不得等远端RPC/长锁；槽满可给已知Queued/last-known snapshot及age，或明确NotLoaded/Unavailable，不能回假Complete。
- 新查询的同步文件工作也应走有界worker并在async handler中等待已受控owner；不能通过spawn_blocking+abort宣称取消。该handler不能把5s阻塞塞进serve reactor。

## R2：未知task解析最小补齐，保留不可确认边界

- native Store的`messages`已按(caller,agent,message_id)索引Dedup.task（store.rs:295–305）。加只读`find_owned_message`，再加当前Delegation `LookupOwnedInput`/小型resolution reply；复用caller_entry/target校验，不枚举其它caller、不返回大history，不发模型工作。
- HTTP复用原Files.inputs/Endpoint snapshot。已保存response直接提取task；未知response把recover_input拆成**一个可保存cursor的page step**，每次只扫有限页/字节，在公平队列中推进。当前4页后重置的问题不延续；只用原input.message的wire ID匹配，绝不重发原Send。
- 扫不到不等于从未接受：旧RPC可稍后才入场，外部服务也可能不提供足够history。此时保持Unknown/ScanExhausted并可查询，不能把“没有task ID”算成ConfirmedCanceled。native如要保证cancel-before-late-Send，需要另一个明确的message-key cancel fence/tombstone协议（Store.accept原子拒绝该input）；**不自动塞入R1**，须root单独定契约/预算/数据保留。HTTP无同等能力时如实Unknown。
- link当前保存machine字符串，重试不得因别名重指而取消另一home：创建link时尽量保存同一解析路径选定的稳定home/caller/target，后续用该身份核reply与dispatch；不能仅凭新config或字符串别名重绑。该身份扩充先由root确认，不造旧Khor网络fallback矩阵；缺证明的历史link保Unknown，不静默丢弃。

## 建议精确allowlist与实施顺序（尚未授权写生产）

R1生产：新增`crates/node/src/agent_cancellation.rs`（必要时拆store/helper文件）、`lib.rs` mod、`khor_agent.rs` intent/summary/query/recovery、`mcp_worker.rs`有限step owner/队列、`delegation.rs`ChildLink receipt/单step、`link.rs`现有ticker/startup与async查询路由、`apps/gui/core/src/lib.rs`两个action映射。若稳定destination需新解析helper/track签名，额外`mcp.rs`仅接现pre/post-track，保parent和批准语义。没有Pi/ACP监督器/GUI布局/token/新服务/安装器改动。

R2额外：`crates/a2a/src/store.rs`只读索引API、`delegation.rs`当前Action/Answer、`a2a_http.rs`一页恢复、`proto.rs`当前协议roundtrip测试。不增Khor旧端协商/双路径，不提高MAX_FRAME或把过大receipt截断成成功。native message cancel fence若选做，另卡另allowlist。

测试：新`agent_cancellation_tests.rs`及私有fixture，复用本片实际CLI hidden worker/真实Handoff/ACP stub/Store；必要扩`mcp_worker_tests.rs`，不改106既有oracle，不放宽13s/gate/late结果要求。

建议先R1已知task 6-run首红→持久intent/step/receipt→恢复/查询；再R2索引lookup与page cursor。不能先造完整后台框架或先把未知input标成“无工作”后收口。

## 必须形成的实际行为oracle

| 场景 | 最小oracle |
| --- | --- |
| 6run停止/4槽满 | 每run本地先退役、下一Prompt可用；第5/6durable Queued；不重发Stop/Prompt，既有tick释放槽后所有6个目标实际收到各自RPC；活动slot≤4、相同run/attempt不重复 |
| run与child公平 | gate实际先入场的慢target；其它ready run及同run其它child在有限quantum/tick轮次内实际入场，首gate仍扣住；不依赖read_dir顺序或sleep猜入场 |
| crash/restart | durable intent后、RPC前/已接受但扣receipt两个barrier分别退出自有进程；新Node启动恢复，旧lease释放后先observe，保存cursor不回第一页；Send/Answer计数不增加 |
| ACK与确认 | 实际ACP或自有A2A endpoint先回Working再Canceled；查询先AckPending后ConfirmedCanceled；仅flag=true、local reaped或timeout不得升确认 |
| completed/failed/reject | Completed/TaskFailed分别TerminalOther；明确Cancel拒绝为Failed/NeedsAttention；NotFound/IO/HTTP本地flag为Unknown，原link保留 |
| 丢task reply | 实际接受delegate后扣reply，native lookup找原task并exact取消；HTTP多页cursor跨重启继续；服务不提供可识别历史时Unknown，不重发Send |
| cold query/权限 | run已结束且无Running也能查询；错误home/caller/任意task payload被拒；非法run/path被拒；分页/字节预算明确，原其它data_json保留 |
| 旧attempt迟到 | 新retry/新run已开始后释放旧reply，旧token不能覆盖新revision，也不能Stop后续context里的任务 |
| pause/退出/资源 | pause/resting期间不发网络RPC但可读pending；resume按cursor继续；worker/IO明确reap/join，root关闭无新daemon残留；Windows runtime仍单列未知 |

restart保证从**已持久化的取消意图**起算。Stop立即返回与最终Saved持久化之间现仍有窗口；若要把Action::Stop返回改成“durable cancellation已接受”，需root另定响应/失败语义。保存前崩溃的interrupted parent不能未经决定就推断为用户取消；不得借恢复自动重启模型工作。

交接：本片仅本卡及10源只读指纹，无源码dirty、无Cargo/fixture/PID。下一步请root确定R1入口/状态schema、是否同时R2，以及Stop durable-ack和native input-fence边界，再授唯一写树与首红准备；main/ship边界不变。
