# Q037：同步 MCP 与远端取消的只读深审

状态：audit / 待真实首红，未修复。作者：fanout_owner，2026-10-26。

代码树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，`fix/quality-ssh-lifecycle@a1625c7fc475b28e14d23600ccd9e0d95b2add0d` clean。Q045两提交与既有卡保持冻结，本任务只新写本卡；没有 Cargo/native、没有执行 fixture/SDK/SSH/模型请求，没有生产改动。当前唯一构建槽归 native_helpers pending-cache。无 main 合并或发布授权；按当前 Khor 接口设计，不增加旧 Khor fallback。

已读中央 STATE/PLAN 和 Q037 ACP卡，按 explore-codebase 技能先 get_minimal_context，再 semantic/query MCP/调用关系。CRG built 8a22cfdb、HEAD a162，不覆盖 apps/agent/mcp、mcp.rs、delegation.rs；因此对这些位置查实际源码。图谱不能证明当前测试覆盖或代替下列调用链。

## 当前确切调用链

### Pi 中的 MCP 客户端

1. `apps/agent/src/host.ts:35–48`：run 创建模型 runtime 后 `connectTools(transport)`。唯一 transport 是本进程的 `HostTransport`，不是外部 MCP 服务器发现/子进程管理器。
2. `apps/agent/src/mcp.ts:25–29`：new `McpClient` → `client.connect(transport)` → `client.listTools()`；Rust Khor catalogue 是当前唯一 tools/list 来源。现在 catalogue 直接返回一页，不能把 SDK 理论1000页当本产品实际调用1000页。
3. `mcp.ts:36–42`：每个工具 executionMode=sequential，execute 收到 Pi 的 signal，将它传给 `client.callTool`；delegate timeout60s，其余30s，带 onProgress。客户端取消/超时会拒绝对应 Promise，并向 transport 发送 notifications/cancelled，**不是没有取消能力**。
4. `HostTransport.send:14–17` 只在写前检查 signal，再调用 host.ts:28 的 process.stdout.write；此处成功意味着本地写入已调用，不等于 Rust 已处理或业务停止。stdout.write 的 false/backpressure 当前没有等待。
5. `host.ts:118–130`：Rust回来的 MCP帧交给 transport.receive；abort 将 stopped=true 并 `void session?.abort()`。session 未创建时只记录 stopped，connect/list 本身没有接这个 signal。正常 finally 在91–95关闭 session/client/input，但 try 从66开始，早于它的 connect/list/resources.reload 失败不走该 finally。这是启动失败资源归属的静态边界，不冒充已复现长期泄漏。

### Rust 执行与 MCP 分派

1. `khor_agent.rs:315–383`：Prompt取得 executor.lock 的 try_lock claim；std线程持 `_claim` 跑 execute，execute返回后才从 running map 删除 root。
2. `khor_agent.rs:589–623`：Pi进程归共享 Session/Control；stdin writer、stderr drain、stdout reader各有owner。reader用容量64的channel交事件。
3. `khor_agent.rs:628–649`：唯一 execute事件线程 recv_timeout；MCP帧同步进入 `McpSession.message`。等待批准时仅在 recv_timeout(100ms) 的 Timeout 分支调用 pending.poll。
4. `McpSession.message:783–820`：notifications/cancelled只清理仍在 pending 的批准请求；read/已放行请求进入 `mcp_request`。`poll:857–862` 先清 pending 再同步执行已批准调用。
5. `mcp_request:870–928`：initialize/ping/tools/list本地快速返回；tools/call校验、检查active.stopped和配置，随后同步 `mcp::call_with_parent`。调用期间不会继续消费该Pi事件channel。返回后才再次检查 stopped 并调用 cancel_children。
6. `mcp.rs:296–396`：delegate/task_status/task_wait/task_cancel等 → 新 current_thread runtime → `block_on(n.delegation_on(...))`；ls/read/search/open/close/tell也各自同步 block_on其业务调用。不是只 delegation 一种工具可能占住事件线程。
7. `link.rs:6005–6060`：delegation_on 先尝试 resident IPC，再 local spawn_blocking(delegation_here)，否则通过当前认证mesh；不能把drop外层future等同于取消其已启动的spawn_blocking。

### 停止与远端子任务

1. `khor_agent.rs:90–100`：request_stop原子标记停止，独立共享process.stop保留首次10s deadline，并向Pi输入abort。这个监督器可以在MCP事件线程阻塞时杀/回收Pi进程；它不能终止仍在Rust线程里执行的MCP业务函数。
2. Action::Stop（419–432）每次都新spawn一个不保存JoinHandle的cancel_children线程，无“这个run已有取消sweep”的归属/去重。
3. execute只有在同步MCP返回、事件循环退出后，才在719调用Execution.finish并join3路I/O；接着734又同步cancel_children。终态save（736–758）、running.remove与executor claim释放都在这些步骤之后。因此 **Pi已被监督器reap不等于这个执行线程已释放busy/claim**。
4. `delegation::cancel_children:674–775`：遍历durable child links；没有task ID时串行分页List按message ID找任务；再Get，非终态才Cancel。每个链接都串行，未设整个sweep的总期限/数量预算，也未提供owner join/取消handle。错误被收集成Result，但上述三个调用点均丢弃它。
5. 本地ACP子任务：delegation_here Cancel（259–266）记 exact task cancellation intent；gui_host的200ms delegated tick（1175–1183）观察它，按同代次operation取消并有超时退役；终态由真实Turn completion / 收口路径发布。Store::request_cancel（crates/a2a/src/store.rs:450–474）明确Working只记intent，executor确认终态，非working quiescent态可直接cancel。这部分不能误报为“收到Cancel就伪造所有远端Canceled”。
6. native A2A HTTP：a2a_http Cancel（803–824）先持久cancellation_requested，再发cancel_task，核返回task ID后存真实task状态；本地请求的success/父cancelled不是远端工作停止证明。接收远端真实terminal receipt仍是必要条件。

## 已核的实际期限与锁边界

| 路径 | 目前实际保护与缺口 |
| --- | --- |
| Pi MCP initialize/list/call | pi-mcp默认每请求30s；call覆盖为30/60s；progress会重新arm相同timer，无整个operation绝对上限。人的批准等待通过每5s progress续期是现有产品语义，不能把它直接判为bug或偷偷加“人必须30s答复”。 |
| resident IPC | ipc::Op::Delegation（535）用DEADLINE_SLOW=300s（462）；call_refreshing（829）确有端到端async timeout。与Pi父停止10s、JS工具30/60s不是同一取消域。**不是IPC完全没有timeout**。 |
| mesh请求 | ask_on / exchange_on / request budget已有网络期限和RetireUnheard取消guard；不可笼统说所有远端请求永远等。现问题是没有把本次MCP取消传入这条链，多个动作/子任务串行累计仍可长于本地Stop。 |
| 本地delegation Wait | 单次8s（delegation.rs:230–242）；不是无限任务等待。 |
| native A2A | HTTP6s/connect3s（a2a_http.rs:316–322），handle外层async timeout12s（691–695）。task_gate（899–908）已经try_lock，**不会在这里永久阻塞**。 |
| native A2A同步锁 | Files::transaction ledger.lock（146–160）、connect_a2a connect.lock（378–380）是阻塞File.lock。refresh先task_gate再receipt→transaction（864–873、167–173）。一个已持ledger.lock的协作者可让当前线程卡在同步锁，async timeout不能抢占这个函数调用。此为待运行复现的静态风险，不是实测死锁。 |
| detached cancel sweep | 每远端调用有上述部分保护，整个循环无deadline；分页唯一next可不断增长，只有next==after防止重复游标。缺全局预算与重复Stop去重，不应把单请求300s当整个sweep上限。 |

## 锁定 SDK 能力，避免第三方猜测

apps/agent/package.json及lock都锁 `@earendil-works/pi-mcp`、pi-agent-core、pi-coding-agent **1.4.0**。本地node_modules经symlink指向现有主树的已安装包；已读package版本与lock integrity，没有安装/升级/执行代码。

- pi-mcp `dist/client.js:112–151` connect安装监听器、start transport、initialize、initialized；connect签名不收signal，但失败会close client。189/218–232的listTools传McpRequestOptions给每页request，重复cursor与1000页有限保护。
- `client.js:244–280` request options signal/timeout/onProgress；370–396收到progress重新arm timer；398–421 cancelPending先reject本地Promise/移除pending，再best-effort send notifications/cancelled，关闭transport在237–242拒绝pending并close。
- pi-agent-core `dist/agent-loop.js:574–598` 将signal传给真实tool.execute并await它；它不是通用强制终止不合作工具的sandbox。
- pi-agent-core `dist/agent.js:221–230` abort只abort controller，waitForIdle等待activeRun promise。pi-coding-agent `dist/core/agent-session.js:1971–1987` abort先中断agent再await idle；dispose（1011–1024）触发abort并断监听，不是已执行Rust副作用已回滚的收据。
- 项目adapter已经正确把tool.execute signal传callTool；所以无需先升级Pi或编造“SDK完全不能取消”。修复重点是Rust执行owner/取消传播与remote intent receipt。

精确SHA256：

| 文件（相对上述代码树；SDK文件在apps/agent/node_modules/@earendil-works下） | SHA256 |
| --- | --- |
| apps/agent/src/mcp.ts | 42f6ceecd360b580088657a0c5e70201659c1f45bc7411a82b0f78d1a7ef6442 |
| apps/agent/src/host.ts | a9f768ba719f564c225011ecc39eda60c58abcab2268fa12ec1e83d36702493d |
| apps/agent/package-lock.json | 9293fb43673dcb0176ccda7d9e20f73361fa4ba4e5876aa3250204d30f353e58 |
| pi-mcp/dist/client.js | f39e0dccc7823995c22ea7936f3f32cac64d818599efdb217b5e0cf0ff070216 |
| pi-agent-core/dist/agent.js | f45ae001e2b14fced8ae151f0a00303b5fbdb5fc4b25418d45de6a5546f48a8c |
| pi-agent-core/dist/agent-loop.js | 22ee81f9977e211044c1ad41a763876c793ca7fc1c5b39b9fa7f76f3cb281129 |
| pi-coding-agent/dist/core/agent-session.js | 436e8be3bbab2db475ce30bb5d0bb30f0046b19a3476f8cd57b8133e68891162 |
| crates/node/src/khor_agent.rs | e85e02d8ba79f846d40cf3f7ed83245dd70b2e3cfa52b5877fbca854fe0240a7 |
| crates/node/src/mcp.rs | 94dbac07ec1b93e61c00797887776481a92179b63dd141154b4f61f40d25089f |
| crates/node/src/delegation.rs | 17bb35898908c22b4e6d9cedc75879c40e6b276af26bf09ab4ad2732ecd4e448 |
| crates/node/src/a2a_http.rs | 88960fc11179a771f0036b7be7782a06a1b1e582b5a44181624e4688db139417 |
| crates/node/src/link.rs | 62ebbe6b46fb29dc52a832d406553a0c840c97c9758cfb4075525d9ac378ea1e |
| crates/node/src/gui_host.rs | d2b3e99fc7353a346e7ed7851424ac99db262c57c7c1b0493353a633b1f6ea80 |

Cargo.lock锁tokio **1.53.1**。本机registry源码 `tokio-1.53.1/src/task/blocking.rs:105–119` 明确started spawn_blocking不能abort；runtime shutdown等待它完成，shutdown_timeout只是停止等待，任务继续运行。该文件SHA `22a0cffd06dd0accdeb4f332c9636205a51214ab45d27444d623639eea4ae54b`。不能以spawn_blocking+timeout+drop作为已取消/已join的替身。

## 审查结论与建议优先级

这些是源审确认的结构事实与待实测风险；本轮没有新的生产真红，不能标已修。

1. **P1：Pi进程退役与Rust MCP线程/claim不是同一终态。** 同步调用期间取消通知不被处理；业务期限可300s，或落同步锁。Stop独立kill/reap只解决Pi进程，不能证明execute已完成，可能继续持busy或执行在途动作。应先真红这个窄边界，而非重复已绿的单纯Pi不读stdin测试。
2. **P1：远端取消sweep无单一owner与总预算。** 重复Stop每次新线程；前面一个慢peer延后后续；execute终态还同步等另一次sweep，错误被忽略。当前durable child links与exact task cancellation设计可保留，需把本地收口与远端pending/confirmed分开。
3. **P2：MCP取消域/资源归属统一。** 外部 `mcp::serve_stdio:223–280` 也逐行同步call，所有无id通知（包括cancel）直接略过，call期间也不能观察stdin EOF。嵌入Pi与CLI不应未来各造一套不可join worker。已有库/SDK支持通知，不是升级依赖即可修。
4. **P2：启动finally与批准公平性后续窄验。** connect/list在finally之前；批准poll仅空队列时推进，持续合法事件可拖延批准/续期；HostTransport写忽略backpressure。当前真实Pi等待单个sequential工具时未证会持续自产无限事件，不把合成flood当真实SDK已出现故障。这些边界单列，不抢先扩大首批。

## 最小真实复现方案（未执行、需下一次授权槽）

### A：现有Rust host + 实际IPC + 一个受控无回应者

复用 khor_agent_cancel_tests 的私有scratch/专用worker/自有sidecar seam，不用真实模型、外网或用户凭据；合成sidecar只发真实start/mcp/done协议，**不替换Rust mcp_request/call_with_parent或IPC调用**。

1. Scratch Node按既有测试设本地agent配置，凭据只用fixture常量或直接私有execute入口；不读用户HOME/keys。写scratch `.khor/endpoint.json`，pid为自有fixture/worker活PID、port为127.0.0.1:0 listener实际port，cookie自有随机值。endpoint_file仅检查该PID活（link.rs:11059–11069），不会启动/夺取真实resident或端点key。
2. 真实sidecar发一个MCP task_status。假resident使用实际IPC frame/cookie解码，**收到确切Op::Delegation后**给parent barrier，随后扣住reply；不用sleep猜调用是否入场。
3. 直接调用同一scratch Node的 khor_agent_here(Action::Stop)，避免测试Stop本身被假resident路由。观察Stop入场时刻、sidecar PID立即/最迟原10s deadline后ESRCH、record/claim/running状态、下一Prompt是否仍busy。预期旧实现在第13s仍占claim，而不是伪称Pi还活；断言不抬原Pi10s要求。
4. Parent无论断言成功/失败都显式关闭假IPC连接/释放gate，使300s等待提前结束，再等待原execute、I/O和fixture join；外层专用worker墙钟35s、自有PGID、双盘护栏、所有fixture自退保险。不能直接丢线程让它留300s。
5. 修后要求local process+MCP owner+IO都收口才释放claim，下一Prompt成功；原在途有副作用的结果保持unknown，不返回伪“从未执行”。假resident仍扣reply时本地取消应能关闭自己请求，不靠测试先放回应才绿。

这是当前最容易取得确定入场barrier、无网络环境依赖的首红。它不冒充真实mesh/远端agent停止，也不冒充真实PiSDK。

### B：不能靠drop Tokio future修好同步锁

第二窄回归用自有scratch A2A Files的 **ledger.lock**（不是已经try_lock的task gate）由独立fixture持有，实际task_status→local spawn_blocking→refresh→receipt→transaction进入File.lock。锁/fixture由parent可释放，永不依赖杀用户进程。需可靠“实际交易已经开gate并尝试锁”的观察barrier（可申请test-only只观察hook），不能sleep后碰运气Stop。固定等待超过原Stop grace后释放锁并join，证明是否仅Pi已reap而Rust owner仍busy；同时验证async12s timeout不会神奇抢占同步锁。

### C：实际Pi SDK取消能力/最终桥接控制

直接导入锁定的实际McpClient与项目HostTransport，用受控in-memory消息端完成initialize/list，保留call回复；AbortSignal触发后核Promise拒绝、准确requestId通知、late response不交给下一请求。progress测试只检验实际SDK计时续期。它是SDK/adaptor能力控制，不能当Rust首红。

最终再复用 `apps/agent/test/host.test.ts` 或 `crates/node/tests/util/delegation_pi.rs` 已有loopback SSE模型fixture运行真实Pi SDK；其现有工具回应均立即给出，已有remote-cancel流程也未覆盖扣住RPC/锁的场景，不能借早期绿关闭本风险。所有令牌均synthetic，listener强制loopback，HTTP/HTTPS代理不接入外网。

### D：重复Stop和远端取消收据

真实track_child生成自己的durable link；自有假IPC/A2A目标扣住首个取消相关请求，连续16次Stop同一个run。测实际请求/worker数量、首deadline是否保持、local claim是否独立释放、错误/远端pending是否可查询。旧实现预计启动16个独立sweep；仍须真实红，不以源码计数冒充已跑。释放全部gates并join。另测慢child不阻止其他child被尝试；Cancel返回Working+cancellation_requested时不得宣称远端Canceled。

## 建议最小边界，待首红后由 root 定接口

不要先把mcp_request简单thread::spawn然后丢JoinHandle；也不要只包tokio::timeout或把runtime.shutdown_timeout当join。每个MCP request应有由host持有的operation owner（run identity + request id + generation）、有界admission与结果通道，主事件loop持续处理Stop/cancel/批准/EOF；完成仅能更新对应代次，取消不得把旧结果转给新请求。批准等待与真正执行是两个阶段，人的决策等待不重用业务执行deadline。

两条可审路线：

- **协作式async执行**：共享dispatch接受取消域，取消传到IPC/mesh/A2A；所有阻塞锁改为try_lock+取消/期限的有限等待；本地阻塞I/O仍要明确可中断owner。变动波及多业务叶，必须逐个真红，不能在只修IPC后承诺所有工具硬收口。
- **有归属的MCP执行子进程**：对无法中断的同步业务，用同一Session/Pipes监督单次/有限MCP执行child；控制帧含parent TaskRef及不可混淆generation，取消只退役自己的child，确认reap+IO join后回结果。可复用已验证process supervisor，不复制平台叶。进程退出仍不回滚已接受的网络动作；durable request/child link与unknown receipt保留。不要直接套Q045的1MiB stdout cap到所有MCP合法结果；新的frame/输出预算要核真实任务list/receipt语义。

建议顺序：A首红 → 选最小owner路线并用B防止假取消 → C真实SDK控制/桥接 → D单owner远端cancel sweep。远端owner按parent run去重，重复Stop不产生新无限线程；本地终态不等待整个多peer sweep，远端结果明确pending/failed/confirmed并留durable可恢复路径。离线远端无法获得确认时如实unknown，不自动重发有副作用请求，不把lease/孤儿回收塞进本批假保证。

本轮只读审查结束时SSH树仍clean@a162；没有新运行证据、没有关闭Q037整体任务。
