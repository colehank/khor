# Q037 MCP cancellation 首红夹具非作者只读审查

结论：冻结源码未发现阻断首次隔离运行的问题，可由root授权唯一Cargo/native槽实际编译/运行。**尚未编译、尚未执行夹具、尚未取得红或绿。** 本审查不修改source/生产语义，不扩Q037实现范围。

## 固定来源

树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，分支fix/quality-mcp-cancel，HEAD `a1625c7fc475b28e14d23600ccd9e0d95b2add0d`。唯一dirty为khor_agent.rs 4行cfg(test,unix) wiring，两新文件untracked；审前后SHA均与作者卡一致：

| 文件 | SHA256 |
| --- | --- |
| crates/node/src/khor_agent.rs | 79277dcafcd1703ce53fec3b263f9dc9d8c757bc20464486ddc436c97a746797 |
| crates/node/src/khor_agent_mcp_cancel_tests.rs | 22573d23ffa4acec8e7174c1c25b955dcfb7805b74f6cf0c1dbf9c07278bbcc3 |
| crates/node/src/fixtures/mcp-cancel.py | 74d7ba92c6b773f0a889951e7d77162ade77a7255e2e0b3d68e85ec10a7efb88 |

已读中央Q-037-mcp-cancel卡。图优先minimal/detect结果仍main8704索引，未覆盖当前树；回退actual source、真实调用方与fixture逐段核验。所有动作只读，未Cargo/Python执行/编译。

## 实际入场链与隔离

- Python是可控私有pipe peer，不冒充PiSDK。先读实际start(protocol1)，记录自PID；发initialize并等Rust response，再notifications/initialized与tools/list，确认真实tools包含task_status后才tools/call。task_status为reads=true，能绕过写审批分支而经过实际mcp_request。
- 实际Rust链为McpSession.message → mcp_request（validate、确认配置home）→ mcp::call_with_parent → task_status映射Action::Get → sync runtime调用Node.delegation_on → via_serve → ipc::call_refreshing →真实Handoff编码/TCP发送+写半关闭/读reply。没有替身mcp_request/call/IPC codec。
- 自有worker Node根是fresh scratch/store；账户为synthetic-unused-fixture-key，URL127.0.0.1:1，不调用模型。endpoint.json只指worker活PID、自有cookie、127.0.0.1随机端口，实际via_serve优先交给该listener，不bind网络mesh，也不落用户Node。Python无socket/requests/子孙进程，只读写自身目录和stdio；wrapper绝对Python路径以-I -S启动，避免Python用户site/startup，非login shell无用户profile。
- HeldIpc实际接收写半关闭的有界frame，proto::decode<Handoff>核cookie与Op::Delegation{machine:"",Action::Get{agent:"fixture",task:"owned-mcp-task"}}，通过后才发entered barrier。不是用sleep模拟“已经开始MCP”；前5s无法到barrier明确setup失败，不算目标红。

## 独立终态oracle

实际node.khor_agent_here(Action::Stop)绕过假resident，调用产品request_stop；测<1s admission。Pi grace另给11s观测ESRCH，且必须有真实abort receipt；再以同一Stop起算13s分别测Saved.status=cancelled、running map释放与executor claim try_lock可得，不能用Pi消失替代Rust退役。

claim每次独立open File/try_lock，is_ok检查临时File立即drop，不持锁阻产品。PID读取后>0校验，kill(pid,0)只观测，不信号未知PID。第二Prompt只在上述三条件成立后发；原IPC gate仍扣住，要求新的done/map+claim释放且第二PID也ESRCH。避免靠先放晚回复把修复判绿。

这条假Pi已合作退出，因此若实际输出包含barrier/abort+ESRCH/cleanup成功，并且13s仍running/map/claim占有，才支持“同步MCP阻住Rust executor”的目标红。编译错误、初始化/list失败、未进barrier、Pi没有reap都是不同失败，不能混用。

## 失败与资源收口

- HeldIpc拥有listener、server JoinHandle、entered/release容量1channel。accept非阻塞，read100ms syscall timeout、总接收MAX_FRAME、启动/half-close5s界；进入后recv_timeout25s最终保险。release_and_join先stop并try_send release，再显式join；持有stream shutdown(Both)使产品实际read_to_end收到EOF，已连接请求不重放，因此不会继续等Op::Delegation的实际300s DEADLINE_SLOW。
- exercise返回Err（包括13s）后，worker先cleanup.finish再assert原结果。finish只request_stop同root+同run的active，释放IPC/join后最多5s看map/claim；Drop幂等再次兜底。map/claim是产品owner释放观察，不是拿到了生产execute JoinHandle；作者卡明确这个限制。IPC fixture线程的JoinHandle则实际join。
- Python SIGALRM30s退役保险，正常abort后立刻发cancelled+done并退出，无后代；held socket25s自退。父专用worker40s guard，拥有Child并Drop kill/wait，日志走scratch文件不堵pipe。外部真正运行仍须自有PGID/容量/墙钟保护，不能以该单个Child guard宣称所有跨PGID子进程普遍都可回收。
- 新测试不裸杀未知group，父只kill自身worker。Node Stop原会spawn cancel_children，fresh scratch无已委派子任务；本夹具不冒称覆盖阻塞delegation ledger/file.lock/远端cancel sweep等后续问题，也没有生产线程全join证明。

## 运行交接要求

允许root安排首次真实运行，当前3源保持冻结。需≥5GiB/jobs2/shared quality-target/跨树同源freshness核验，实际编译依赖SHA与binary SHA留证据；不靠旧target 0-test绿。

专用父过滤当前实际模块路径为 `khor_agent::mcp_cancel_tests::stopping_during_a_real_mcp_request_retires_the_executor_after_pi_exits`，child过滤 `khor_agent::mcp_cancel_tests::mcp_cancel_worker`。运行报告必须核父真实1 test及child worker执行标记/barrier，不能仅test binary exit；已有source模块路径匹配，当前无0-test已发生的证据。

目标红最小可接受日志链：真实Handoff barrier → Stop迅速返回 → abort文件+Pi ESRCH → 13s owner三条件失败 → explicit IPC release+join与map/claim drain成功 →父非零原样保留。若cleanup失败或被40s杀，记录完整setup/cleanup失败，不计目标红；不用把13s改大或先放reply得到假绿。

审查到此结束，未启动任何编译/fixture/服务，审查槽可回收。main不合并/发布。
