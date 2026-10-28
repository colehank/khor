# Q037 普通 ACP Stop：只读设计与真实复现准备

状态：**阶段1已独立提交50f73f7f并冻结供非作者复审；目标host红留待阶段2，完整Q037未完成**。agent_redaction 独占本卡。研究树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，branch `research/quality-sync-catchup`，HEAD `fcd0ad2959205bc0e914df479f8fb8fa38497587`，起止 clean，保留 Q039 研究提交。图先行后因索引不覆盖 ACP 符号回落实际源；索引仍基于29fcad93，不冒充当前覆盖。源码/锁 SHA 在 `Q037/acp-design/source.json`。已读中央 Q037-design-review、Q037-review ACP 段。

## 真实生产边界

- `gui_host.rs:1184` 普通 Stop 只调用 `handle.cancel()`、撤权限/elicitation，不设置 cancellation_sent。现 :1068 的10秒检查与取消探测只在 `delegated.is_some()` 的 tick 分支；没有普通 turn 期限。
- :946 `biased;` 首先收 events，再 joined、delegation tick、ops、turn/replay/set/control 完成。持续 notes 不只可能拖住 deadline，还可拖住 Stop 入场、完成响应及新客户端；单把 deadline 检查放在 tick 不能解决。Tokio 官方直接要求在高流量 biased select 中为 shutdown 安排公平顺序。[Tokio1.53.1 select](https://docs.rs/tokio/1.53.1/tokio/macro.select.html#fairness)
- 事件本身为 unbounded channel（khor-acp两级转发）；note handler 还可同步保存 delegated note 和 fanout。因此高频场景必须限制 fixture 输出且单列内存界限，不能用无限洪水测试把进程打爆后计作“超时红”。
- **额外必要等待边界**：Fanout::send/send_to (:674) 持所有 clients Mutex 同步 write_frame；accept 后 socket 未设写超时。一个不读的 GUI 可堵 current_thread runtime，Stop 即使已进入 ops_rx 也不能入场。仅调整 select 排序不能承诺普通 Stop 完整收口。需限时且总预算的 fanout，或每 client 有界 writer+队列满退役慢客户端；不能持全局 clients 锁等 OS 写，也不能只设单 client 超时而让无界数量的 clients 串行累加。
- `Handle._close` 是非共享 oneshot Sender，只有最后 Handle drop 才释放。gui_host 以 Arc 包装，prompt/replay/control/set 任务均持 clone；in_turn 的 JoinHandle 仅 drop 不取消任务，Replay::start 甚至没保存 JoinHandle。离开主 loop 不保证 _close 被 drop。
- khor-acp transport task select close_rx 时会 drop connect_with future，再发送 Event::Closed(None)。这是“驱动被关闭”的本地通知，**不是 process wait/reap 的收据**。

## 锁定第三方可复用与不可假定的能力

依赖 agent-client-protocol **3.5.0**、async-process **2.5.0**，不升级/patch锁。

SDK AcpAgent 提供解析配置 `config/into_config`、公开 `spawn_process()` 和 `ByteStreams::new`/`Lines::new`，可用调用者自有 transport 跑完整同一 Client builder 与标准 handshake。`ConnectTo<Client>` 是现 Client.connect_with 的准确类型约束。普通 cancel 是 session/cancel notification，enqueue 成功不能说已被 agent 接收；原 prompt 的 cancelled 响应才是协议合作确认，最后 notes 应在其前交付。[官方 ACP prompt cancellation](https://agentclientprotocol.com/protocol/v1/prompt-turn#cancellation)

内置 AcpAgent spawn 在 Unix建 process_group(0)，Windows只有 CREATE_NO_WINDOW。ChildGuard drop killgroup/Child.kill，但忽略 signal 错误，不是同步 wait；connect_to 在协议结束后等待子进程1秒，超时 drop child_wait 后也可能返回 Ok。stderr64KiB尾缓存与1秒收尾可参考，但不能当10秒 Stop/Windows Job/确认回收能力。

`async_process::Child.status/try_status` 会 wait/try_wait；macOS底层 signal reaper 对仍持有且未轮询的 Child 不会无条件自行 reap（进程 drop 后才入 zombie list），但内置 ACP driver 从连接开始就并行 poll status，**调用方拿不到确保 PGID anchor 的生命周期**。不得另加裸 killpg“补救”一个可能已被其 driver reap 的 PID。[async-process2.5 Child](https://docs.rs/async-process/2.5.0/async_process/struct.Child.html)

## 三条路线比较与推荐

A. 继续内置 AcpAgent，只给 Handle 加 close/completion：最少改代码，但只能证明连接驱动退出，拿不到 Child owner/Windows Job/同步reap，不能满足完整目标，排除为最终方案。

B. khor-acp 自行用 spawn_process+ByteStreams 并监督其 Child：公开 API 足够，但需自己实现 deadline、Unix未reap anchor、Windows Job、stderr、I/O中断和收据；Windows已有约600行后端会重复，直接扩散治理负担。可做隔离实验，不推荐生产路线。

C. **推荐 root 提出的 Node 内部共用 process supervisor 路线**：把已验证 Pi Session/Control/Pipes 与 Unix/Windows module 从 khor_agent 私有子模块机械重定位到 Node 公共内部叶模块（不新 crate），Pi继续调用同一实现。Node ACP driver解析原 AcpAgentConfig→构造 std::Command（原 command/args/env，不shell、不更改cwd规则）→同一 Session::spawn→自有 I/O适配→SDK ByteStreams。khor-acp仅加 caller-provided transport/lifecycle 窄入口，保持原 start/start_resume/start_configured 默认行为兼容。不给 ACP 复制 Win backend，也不让 async-process 管同一 Child。具体入口与共享 Control 小扩展须 root 先确认，本卡不自动开始重定位。

## 建议窄接口（设计草图，非已编译签名）

`khor_acp::start_configured_on(transport: impl ConnectTo<Client>, lifecycle: TransportLifecycle, cwd, resume, environment)` 复用当前 builder、open_one、events与初始化取消路径。

TransportLifecycle 可为无第三方依赖的具体类型：cloneable `request_close: Arc<dyn Fn() + Send + Sync>`（一次性幂等）与 tokio watch/oneshot共享 completion（每个 waiter 可观察同一不可逆结果）。完成类型要区分 `TransportClosed` 与 `ProcessReaped{status}`；默认内置 transport 无法提供后者，不能填假 receipt。不要把 Node私有 Process类型泄漏给 khor-acp。

Handle 新增 `request_shutdown()` 非阻塞与 `shutdown().await -> confirmed result`，所有 Arc clones 共用 closing latch/completion。首次请求先关闭 admission，再要求 Node provider强制退役本次 process；后续不重复 kill，不因某个 waiter 取消/drop而取消 owner。prompt/replay/set/control wrapper 在 dispatch 前检查 closing，并在等待期间 select shared closing，返回明确“连接退役/在途结果不确定”，避免仍持 clone 的任务永久等待；不盲重发请求、不谎称有副作用的 control 从未执行。旧 `_close` drop 仍触发 best effort，但任何“已确认”终态须 await provider receipt。

owner 由 Node单独线程/固定任务持 Session 和3个I/O worker，并持 operation identity；Handle/connection clones 只有请求能力与只读 completion，不能拥有/重新生成 PID。Session::finish 同步 join 必须在独立 owner线程执行，不能直接在 current_thread async runtime 阻塞调用。退出收据只有 process确认回收、I/O workers退出并 join 后发布；强制kill失败保留error，不先发布Cancelled/Gone再后台回收。构造失败与 opening timeout 也要通过同一owner收口，不靠最后一个Arc drop。

## ACP 是长进程：取消期限需要可解除、按代次归属

现 Pi Control::stop 是不可撤销的单一“本进程最初deadline”；Pi每个run一进程，这很合适。ACP一个process跨多turn，直接复用 stop 会在合作Cancelled后的第10秒误杀下一turn。

推荐共享supervisor区分永久退役请求与 `arm_cancel(operation_generation, first_deadline)` / `ack_cancel(same_generation)`。永久close/自然exit I/O收尾deadline不可被 ACK清掉；重复Stop只保留同代次最初时间；旧turn/旧replay完成不能解除新代次deadline。若 root 不希望扩 Control，可另建 Node ACP watchdog+operation gate，但仍只调用同一process owner的 force-close，不另造PID监督。无论哪条，期限推进必须在 events/fanout 外，不随note/tick被饿死。

有合作取消时：撤 pending permission/elicitation，继续接收有界已入场notes；原 prompt cancelled response证明本turn取消，解除同代次grace并保持process可用，不应要求整个process退出。deadline届满时：原process/transport不可逆退役，拒绝新prompt，确认reap+I/O后才报告本地终止/连接需重开，不能伪造agent回应Cancelled。replay/control/set的session/cancel未必提供业务取消语义；最多按超时退役并保留不确定结果，不能把“设置可能已应用”当回滚。

## 有界 I/O 桥与次序

Node已有 futures直接依赖，可在Node实现 futures::io AsyncRead/AsyncWrite 薄桥供SDK ByteStreams，无需给 khor-acp引入新的process库。每物理方向一个worker，单stdin writer保证JSON帧不交错；stdout/stderr独立读。使用固定8KiB chunk、最多8项队列（每方向约64KiB）和小额ack，stderr尾保留64KiB。通道入队与实际写成功分开，poll_flush必须等此前bytes的OS写/flush确认；不能把“进入队列”当写线成功。

所有 channel 等待与OS管道等待都可被 owner关闭；满stdout channel须在关闭receiver时唤醒，空stdin queue worker须显式close/唤醒，不能重现Input自持Arc空等问题。关闭驱动时不再续写半帧或复用transport；已写半帧直接退役process，不补在新连接上。读完的原末尾notes在成功/合作取消时保留顺序；强制退役不为无限补发notes延长期限。

这只约束桥新增缓冲，**不宣称整个ACP内存有界**：SDK JSON/单帧字符串和现events/outbound unbounded queue仍可能持大数据。不要偷偷设小帧cap破坏图片/长prompt；若要输入/notes总预算应单独说明协议兼容与失败语义。GUI需要公平调度：到期/关闭先处理，控制有限轮转，每批最多N条notes，完成响应之前只排空其协议顺序前的有限积压，不能因无穷notes永不处理turn/Stop。

## 已准备的真实 fixture 与下一次红绿路径

证据目录 `Q037/acp-design/`（根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`）：

- `fake-acp.py` 真实stdio JSON-RPC v1 synthetic agent，支持 initialize/new/load/prompt/cancel；记录独立scratch PID/PGID、prompt与cancel收讫。模式 ignore-cancel收到cancel仍不回应；cooperative发末尾note→原prompt cancelled且下一prompt end_turn；notes-flood有节流1024B消息；stdin-stall在首prompt后不再读。55秒自退出保险，不接网络、模型、用户凭据。
- `fixture-selfcheck.py` 仅标准库parent实际启动进程，经stdio完成handshake/prompt/cancel；ignore与cooperative **2/2自检**，后者实际下一prompt成功。外层仅杀自己的start_new_session进程组并communicate回收。`fixture-selfcheck.json`明确 khor_production_executed=false。最初TextIO/select预读导致自检超时，已改字节读避免误判；不是产品红。
- **还没有执行Khor库/GUI host**，没有目标红，不把fixture自检算修复或生产复现。

授权后先在专用worker测试真实 gui_host_main→Hello/Welcome→Say→fixture prompt.marker→GuiOp::Stop。以 fixture cancel.marker证明请求确已收到；基线15秒仍无收口才记普通ACP业务红，不能因握手/命令解析出错计红。修复后初次终止可见点立即验证原PID已被回收/I/O退出；后续同process合作取消需能新prompt，强制关闭后走显式重开（agent支持load/resume时同session，未支持不可伪装无感继续）。

至少覆盖：安静不合作；notes洪流与Stop/过期timer公平；stdin塞满仍能Stop入场；克隆Handle/replay/control未返回仍可shutdown；多次Stop不续期；首prompt前Stop不杀后续新turn；旧ack不能解除新deadline；合作Cancel保留末尾notes和下一turn；自然leader退出+同组后代/逃逸FD；慢GUI socket不读；close/handshake timeout同owner收口。Windows Job/overlapped分别按平台实跑，不拿macOS替代。

不要直接复用 `crates/node/tests/gui_session.rs::stub_path()` 运行本任务，它会内部启动Cargo build。新增夹具应使用明确python命令/既有编译stub，不在测试内嵌套Cargo；唯一Cargo槽由root串行授权。worker父硬期限、fixture自退出与精确已归属进程清理独立存在，失败不得触碰用户PID/服务。全链测试通过之前本卡保持未实施。

## root 待确认的具体落点

1. 是否按C路线重定位Node内部进程叶模块，并增加 transport+shared completion窄入口。
2. ACP合作取消是否通过带generation的可解除grace扩现Control，还是独立Node watchdog；二者都不得撤销永久shutdown。
3. fanout慢client收口选最小总预算写，还是bounded per-client writer；这会影响能承诺的host期限。

上述选项有可实施路径，不向用户空问偏好；root确认后再授权生产范围/完整Cargo红绿。

## root 确认与实施分期（最新权威）

root 已确认 C：Node内部共用 owned process、不新crate/不复制Windows叶；同一Control扩按operation generation grace，permanent/natural-exit期限独立且ACK不可撤；期限到点在同一短锁内提交不可逆kill状态。Fanout选bounded per-client writer，帧数与字节预算，共享已编码packet，发布/clients锁内无OS写，满/写超时退役该慢client；不采用过短25ms全局扔好client。ACP保留原env/cwd，Windows显式完整环境，不能套Pi allowlist。

实施树已复用并切 `fix/quality-acp-lifecycle`，base `f186d3403ad80e1423ed83fd2cef7e24c17d2b98`。原Q039 `research/quality-sync-catchup`/fcd0ad29完整保留；没有新worktree。仅新增test-only路径：gui_host.rs4行cfg入口、gui_host_cancel_tests.rs、fixtures/acp-cancel.py。Cargo/lock未改。

### 首次真实目标红（已执行，不是fixture自检）

root授唯一槽后执行：

```sh
CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target cargo test --locked -j 2 -p khor-node --lib gui_host::cancel_tests::an_uncooperative_acp_has_a_bounded_stop_and_can_be_reopened -- --exact --nocapture --test-threads=1
```

编译45.08s成功，外父真实1测试失败，专用ignored子worker实际被执行1失败（不是空跑/编译失败）；目标错误为普通Stop在15秒后仍未退役，不是setup。worker运行15.11s、父15.13s。先通过继承ambient env/standing cwd断言，再经真正gui_host_main→Hello/Welcome→Say→fixture prompt标记→GuiOp::Stop→fixture cancel标记，两个标记是同一prompt id。PID=PGID69326。目标红发生前未人为杀agent，不以断开pipe模拟取消。

证据 `Q037/acp-stop-red/`：cargo.log、source-before/after.json、result.json、markers/ignore-cancel/{pid.json,prompt.jsonl,cancel.jsonl,received.jsonl}、cleanup.json。source_stable=true，launcher PGID68832，exit101，总60.38s；最低可用7,370,141,696B，未触5GiB线，无磁盘错误，构建槽已明确释放。失败后worker进程退出关闭自有stdio，fixture正常EOF退出；额外只读signal0已确认原PID69326 ESRCH，没有遗留本轮agent。

红例现在冻结；第二段“显式同session reopen后下一turn成功”和首个Gone时立即ESRCH断言尚未到达，不冒称已经执行/通过。未做生产变更。

### 下一阶段1最小接口补丁清单（先供root源审）

1. **零复制移动边界**：优先保留现 `khor_agent_process*.rs` 文件名，只把 common 模块声明从 khor_agent 子模块移到 lib.rs的内部 `agent_process`（#[path]），Pi改为use同一模块。这样Windows约600行、harness引用路径不复制、不漂移。调整common对Node siblings可见的pub(crate)/pub(super)，平台叶仍私有；已有process_tests仍是同一文件，选择入口从khor_agent::process::tests改为agent_process::tests，验证必须同时跑它与khor_agent::*（不能只看原33filter漏掉共享测试）。
2. **Control扩展**：Requests保留独立不可逆terminal/natural deadline，加 operation=(generation,first_deadline) 与 terminating latch。arm同代次不续期；ack与supervisor expire都在同一锁内先判now>=有效deadline并提交terminating，然后才允许清同代次operation；过期ACK不能抢在下一轮supervisor poll之前撤销已到点期限。Pi原stop映射terminal期限，ACP只用operation期限，close/自然exit可覆盖为更早terminal。
3. **Node `acp_process.rs`**：从SDK公开AcpAgent::config/into_config读取原command/args/env；用完整继承env显式铺到Command再overlay声明env，cwd保持原站立行为。经共用Session::spawn取得同步可取消Pipes。Node已直接依赖futures与Tokio，可实现小型futures IO桥，不新增Cargo依赖。
4. **bounded桥**：每方向8KiB×8，stdio各一个worker，stderr64KiB尾；flush等此前实际写/flush ack。queue满、空等待与pipe读写均响应shared close，owner关通道并join；SDK transport future drop不能成为worker唯一停止条件，也不能持资源Arc自等待。Node owner独占Session.finish/join，完成watch只在真实进程回收+workers退出后给ProcessReaped。
5. **khor-acp `start_configured_on`**：泛型现SDK `ConnectTo<Client>` transport + caller lifecycle，提取同一builder/open_one/events逻辑。保留原start*构造使用内置AcpAgent，不重复一份handshake。Handle新增幂等request_shutdown与多waiter共享shutdown completion；明确DriverClosed与ProcessReaped不同证据。最后Handle Drop仍request best effort；显式close不能依赖最后Arc。注意不要让transport task强持有唯一close sender形成drop环：若close gate需共享，Handle实现Drop显式请求，初始opening guard取消也请求owner清理。
6. 阶段1专用回归：共用process全部老生命周期/Windows编译入口、operation grace到期/ACK锁竞态/旧代次ACK、桥backpressure/写完成/close唤醒/末尾缓冲、ACP injected transport握手/权限/close共享收据。此阶段不悄悄实现整套host/Fanout；普通host红仍可保留待阶段2，选取可独立集成的生产提交与新测试由root协调。

阶段2再接真实gui_host启动到新driver，普通与delegated Stop公平入场/同代次deadline/合作取消解除，tracking各任务句柄并显式shutdown，bounded per-client共享packet writer及慢client退役，然后让冻结目标红及扩展矩阵转绿。每阶段独立commit、源审、明确Cargo授权；不碰main/ship。

### 阶段1接口首版已写（尚未 Cargo 编译）

- Node lib.rs通过#[path]声明agent_process，Pi引用同一原文件；Unix/Windows叶及独立Windows harness均未改。公共模块仍只std+既有平台依赖。
- Control票据为opaque owner Arc identity + generation（不仅u64），跨owner旧票据不匹配；begin/cancel/complete、terminal/natural、同锁terminating边界有6项std-only实际抽取测试，6/6通过。构建源码抽取仅覆盖Control，不冒充完整Node编译。
- Node acp_process.rs/acp_process_io.rs提供完整显式继承环境+声明override、原cwd、共用Session owner、8KiB×8桥、物理write/flush ack、stdin/stdout满空队列close、stderr64KiB尾。owner在Session.wait与I/O workers join后发布ProcessExit。构造期Building guard对部分worker创建失败也强制清理。
- ACP lifecycle.rs公开TransportLifecycle、ProcessExit、ShutdownOutcome；start_configured_on准确接受SDK ConnectTo<Client>。DriverClosed与ProcessReaped明确区分。Handle request_shutdown/shutdown共享completion，pending RPC对closing响应；司机task有Drop guard，runtime销毁而Handle仍存活也请求provider退役。已完成response优先于同时关闭，避免自然EOF抹掉最终response；关闭之后的新操作拒绝。
- 新测试源码：Control6、bridge6、Node managed transport5、ACP lifecycle4；除Control6 std probe外尚未执行。涵盖OS flush真实确认/错误、full/empty close、合作cancel同进程下一轮、权限dismiss实际跨线、显式多Handle确认、refusal未生成Handle清理、runtime drop旧Handle清理、错误owner确认拒绝、已交付response与close同时发生。
- 仍无新依赖/lock/原生command，无host新行为/Fanout修改。冻结普通host红三路径保持未提交，阶段1生产commit会明确排除它们。
- 内盘第一次恢复快照：`/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-ACP/snapshot-20261025T155813Z`。它保存当时15项真实源与diff/receipt，后续源码以现场/下一快照为准，禁止按摘要覆盖。

阶段1完整验证计划：先 `cargo test --locked -j2 -p khor-node --lib acp_process:: -- --test-threads=1` 真正编译接口并执行新桥/driver；同源成功后复用实际Node binary运行 `agent_process::` + `khor_agent::` 两组覆盖原Pi与移出测试；再 `cargo test -p khor-acp --lib lifecycle::` 与现有conversation集成保证原入口/权限兼容，Windows既有harness交叉check单独登记。只在root授权唯一槽后运行；当前仍未取得阶段1完整类型/行为绿，不提交为可集成完成。

root首轮阶段1源审指出driver runtime/abort退役时result_tx会随future丢失，provider已独立完成但旧Handle.shutdown未复用其receipt。源码已加provider存在时result watch关闭fallback到confirmed_exit，provider失败仍原Err；没有provider则保持未确认错误，不造DriverClosed。Node真实managed transport测试改为保留Handle→drop原runtime→新runtime先调用Handle.shutdown并要求ProcessReaped；另加leaf正/负provider与unowned watcher关闭对照。当前新增测试源码23项（Control6+bridge6+Node5+lifecycle6），仍只有此前std Control6实际运行，完整Cargo未授权。

root第二轮源审要求new API returned Refusal也在返回前确认清理：首版全部验证后，start_configured_on已加独立lifecycle clone，Err分支request_shutdown→await confirmed_exit，成功则保留原Refusal类别/正文；cleanup失败追加明确未确认信息。legacy start_configured未承诺reap；外部timeout/drop无法await，阶段2caller必须保留cap再request/confirm。Node refusal测试改为Err返回即now_or_never取得receipt并立即signal0=ESRCH（先于任何额外await）；ACP增加受控provider withheld-completion测试，确认Err不能先返，provider失败仍明确。此轮源码在重新编译/相关组回归中，不拿首版绿代替它。

## 阶段1正式交付：50f73f7f（2026-10-26）

commit `50f73f7f2c36422331938f43cb95a1fee012b64d`，base f186d340，branch fix/quality-acp-lifecycle，12个明确生产/专用测试文件。未改依赖/lock/GUI协议/原生命令，Unix/Windows叶与Windows harness内容未复制或修改。保留同目录另外3个未提交的host目标红路径：gui_host.rs仅4行cfg入口、gui_host_cancel_tests.rs、fixtures/acp-cancel.py；全部排除本提交。Q039研究分支fcd0ad29保留。

最终固定源码验证（每组 source_stable=true，结果源SHA全部与提交/现场逐项核对）：

| 真实执行组 | 结果 | 证据目录Q037下 |
| --- | --- | --- |
| Node acp_process::（真实SDK/owned process/OS bridge） | 11通过0失败 | acp-stage1-refusal-bridge |
| 新agent_process::（原4行为+新6Control） | 10通过0失败，1专用worker标ignore由父实际启动 | acp-stage1-final-shared |
| 原khor_agent::其余Pi组 | 29通过0失败，5专用worker标ignore由父实际启动 | acp-stage1-final-pi |
| ACP lifecycle:: | 7通过0失败 | acp-stage1-refusal-lifecycle |
| 原ACP conversation | 19通过0失败，2需要真实agent的可选检查未跑 | acp-stage1-refusal-conversation |
| 原Windows独立harness，x86_64-pc-windows-msvc --all-targets | offline/locked check exit0 | acp-stage1-final-windows |

合计76项通过0失败（新增24+原Pi33+原ACP19），不把8个ignore混为业务全覆盖：6是父测试实际使用的隔离worker，另2真实agent可选检查未运行。Windows仅公共层/既有平台叶+harness类型检查，未做Windows链接或runtime，不冒充跨平台实跑。原GUI host Stop红仍未转绿，本阶段未接新host/Fanout。

两轮root源审疑点均有实际新证明：driver runtime已退役而Handle仍存活，新runtime调用Handle.shutdown拿到ProcessReaped；new API Refusal返回即now_or_never取得receipt且signal0立即ESRCH，受控provider不放completion时Err不会先返，cleanup失败附明且不伪造成功。stdoutEOF尾缓存、OS flush完成/错误、空满队列close、同process合作取消下一turn、权限取消真实跨线均实际通过。

完整机器交付 `Q037/acp-stage1-delivery.json`：12项提交文件SHA、所有最终测试source hashes/command/PGID/双盘minimum、Node测试binary SHA、remaining diff与边界。最终各组外盘最低7,180,980,224B，内盘最低7,550,279,680B（逐组精确数值见JSON，全部>5GiB）；无I/O异常/停组。唯一Cargo槽已向root明确释放，没有遗留本agent cargo/rustc。

编译保留一条阶段性stderr字段未被host消费的dead_code提示；本层实际保留64KiB诊断尾，下一阶段host接入后使用，没有以allow掩盖。当前所有新API和old入口兼容证据仅限上表，不宣称新host取消或慢GUI退役已修。

下一步只做root非作者复审与集成，**不自动开始阶段2**。阶段2需在root确认后再接host公平入场/代次grace、显式shutdown以及bounded per-client shared packet writer，让已冻结真实host红转绿；外部timeout/drop仍由Nodecaller持provider请求并确认收口后才写refused/允许重试。

## 阶段1 test-only followup（待槽、尚未编译）

50f73f7f保持不改，仅改acp_process_tests.rs与khor_agent_process_control_tests.rs。root第三轮要求的两个动态边界已写：合作cancel ACK后真实sleep1.2秒越过原1秒grace，先确认原PID活着，再用同Handle下一prompt完成；真实in-flight permission使agent保持活跃，仅arm200ms operation deadline并repeat30s（repeat若因调度已过期可返回false），不调用cancel notification/complete_operation/shutdown/finish，直接await owner.confirmed_exit并立即ESRCH，IO workers确认包含在owner receipt中。只有5秒失败时单独request_shutdown清理后仍panic，不会把失败清理计作通过。Ask与Handle保持到证明后才drop。

纯Control重复Stop例改为ZERO已到期→延长请求被拒绝→ACK拒绝/新operation拒绝的公开行为，不再在10ms内假定线程能及时repeat，也不以私有deadline字段相等代替真实owner过期（后者由新进程例覆盖）。当前无生产接口/fixture变动，普通host红三路径仍独立。等待root唯一Cargo授权；没有把编写完成称为通过。


## root测试followup实际运行（2026-10-26）

50f生产代码未变，只有acp_process_tests.rs与khor_agent_process_control_tests.rs补强：合作取消超过原1s grace仍存活、同Handle下一轮成功；保留Ask/pending Handle，仅operation deadline200ms（重复30s不续期）而无shutdown/complete/cancel通知也确实reap并完成I/O。acp-stage1-review-followup实际Cargo12/12 source_stable=true；同二进制Control6/6直接执行通过。两个test文件待独立提交；冻结gui_host入口/host红/fixture不纳入phase1。


root test followup已提交013efc40，源和实际Node二进制SHA见acp-stage1-review-followup/control-result.json。阶段1 50f→bf559f4f、test013→7493e8e9已进入独立集成分支；仍红host三路径未带入。集成头验证待run11 Safari释放唯一native槽，不能称普通host取消已修。
