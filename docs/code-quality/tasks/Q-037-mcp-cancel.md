> root最新交接：已测首红3源25478cc7保留refs/audit/mcp-cancel-red-20261026，现已重放在集成1d2640d2上为7550e4f0、tracked clean。真实首红仅属于旧25478/a162基线；新组合尚未执行，3源SHA原样。执行权已交新mcp_worker_owner，旧fanout_owner停止。下面原始记录不改。

# Q037 MCP cancel：实际 IPC 回执被扣住时的首红

状态：**真实首红已取得，源码/原始结果冻结，尚未修复**。作者fanout_owner，2026-10-26。唯一Cargo/native槽已归还root；不自行开始生产实现。

## 2026-10-26 短交接（当前优先）

- 已按root授权将已测3源独立提交：`25478cc70da48f4217b11cc72f669321ef3ae579`，branch `fix/quality-mcp-cancel`，baseline a162；当前clean。只含2新测试/fixture和khor_agent.rs四行cfg wiring，生产行为仍旧红。下方“未提交”是准备阶段历史。
- 自有runner55924/Cargo PGID55943/Pi56707交接核均ESRCH；无自有活动Cargo/native/fixture。first-red原始结果、21输入SHA、binary SHA在本卡证据路径，3源不变。内盘快照 `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-MCP/snapshot-first-red-result-20261026T131041Z` 保留3源/卡/原始log/result/receipt。
- **精确架构proposal尚未完整落盘**；只有审计卡的路线比较和root/作者消息中的约束，未实现。root停止本作者长turn并安排干净上下文新执行者，不自动重试或编译。
- 已定方向不能丢：MCP worker一次输入EOF/一次结果；Running当前代次在spawn前注册Control并在同锁检查stopped，Stop可独立短促terminate/reap/join，不能复用Pi JSON abort+10s或spawn_blocking假取消。root只要求该owner的本地确认，不扩成所有远端停止保证。
- Windows默认job不能直接套所有MCP工具：open→`host.rs::_host`、local delegate→`gui_host.rs::_ghost`是应继续活的durable host；Unix现setsid，Windows现无breakaway。root已定**只对typed MCP worker job启BREAKAWAY_OK，且仅这两个确认durable spawn显式CREATE_BREAKAWAY_FROM_JOB/handoff**；Pi/ACP默认job不放宽，不用单PID kill遗留临时后代。
- 实际入口：CLI bin名`khor`（crates/cli/Cargo.toml）；CLI与GUI都早调`khor_commands::console_main_if_requested`，可加Node同源hidden worker hook，早于GUI启动/语言/askpass路径；现mcp仅空rest，**没有现成mcp --once**。self_exe实际current_exe，普通Rust libtest不能识别hidden argv。root优先本轮真实CLI build+Node cfgtest确切CLI路径注入/CARGO_BIN_EXE，不解析libtest stdout噪声，不仿worker。现delegation_wire是harness=false，main已有_ghost路径，后续可接同一真实worker hook。
- 尚待根接口确定：输入/输出/尾部cap（不能直接复用SSH1MiB；单task上限4MiB、task list可大于它）、parent delegation身份/授权保持、cancel_children独立run级去重owner/总预算，不再主execute收尾同步等300s。没有追加生产scope。

## 授权与现场

root接收[只读审计](Q-037-mcp-audit.md)后，授权准备一个MCP cancellation fixture/test，`khor_agent.rs`只增加cfg(test) wiring；首红前不改生产。库file.lock、远端cancel sweep、SDK启动等留后续独立轮。当前代码树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`。

从 `a1625c7fc475b28e14d23600ccd9e0d95b2add0d` 新建 `fix/quality-mcp-cancel`；`fix/quality-ssh-lifecycle`仍指a162，Q045提交/旧ACP分支与卡未改。新分支HEAD当前仍a162，未提交fixture。唯一Cargo/native槽归native_helpers，作者未运行Cargo/native、未执行Python fixture，未修改MCP/IPC/Session/业务生产行为。

graph-first：get_minimal_context、tests_for McpSession.message。索引built8a22、HEADa162不一致；图谱只找到既有批准取消测试，未覆盖本次同步调用被扣reply的边界。随后核当前actual source与既有cancel fixture，不重建.codegraph。

## 唯一新增源码与冻结指纹

| 文件 | 变更与SHA256 |
| --- | --- |
| crates/node/src/khor_agent.rs | 仅4行 `#[cfg(all(test, unix))] #[path=...] mod mcp_cancel_tests;`；原文件SHA e85e02d8ba79f846d40cf3f7ed83245dd70b2e3cfa52b5877fbca854fe0240a7 → 79277dcafcd1703ce53fec3b263f9dc9d8c757bc20464486ddc436c97a746797 |
| crates/node/src/khor_agent_mcp_cancel_tests.rs | 新1父测试+1专用ignored worker及owned IPC fixture；22573d23ffa4acec8e7174c1c25b955dcfb7805b74f6cf0c1dbf9c07278bbcc3 |
| crates/node/src/fixtures/mcp-cancel.py | 新合作私有pipe peer，不调用模型/网络；74d7ba92c6b773f0a889951e7d77162ade77a7255e2e0b3d68e85ec10a7efb88 |

准备阶段新Rust测试由rustfmt(skip_children)解析，Python经ast.parse语法检查。第一次rustfmt发现unsafe block尾表达式比较需括号，已只修新测试语法后重检；该准备错误不是产品红。之后以下真实Cargo轮已编译/运行，3源码指纹保持完全一致。git diff --check过；现有生产diff只有上述4行测试wiring。

## 实际调用与确定入场屏障

测试用已有 `KHOR_AGENT_SIDECAR` 私有seam，但不替换Rust dispatcher/IPC，也不把假Pi当真实Pi SDK。专用worker开全新Node scratch并配置synthetic-unused-fixture-key，URL固定127.0.0.1:1且从不请求；不读取用户key/HOME内容。sidecar wrapper只exec找到的绝对Python路径，`-I -S`关闭用户Python环境/site启动，fixture只访问自身scratch路径。

1. Python读真实start帧，保存自己的PID；真实MCP initialize request1等Rust response → notifications/initialized → tools/list request2等真实catalog并确认有task_status。
2. 发tools/call request3，参数task_status `{agent:fixture,task:owned-mcp-task}`。实际生产调用是 `McpSession.message` → `mcp_request` → `mcp::call_with_parent` → `Node::delegation_on` → `via_serve` → `ipc::call_refreshing`。
3. scratch endpoint.json只指自有活worker PID、127.0.0.1 listener随机port和自有cookie。假resident用**实际proto::decode<Handoff>**，核cookie及 `Op::Delegation {machine:"", Action::Get{agent:"fixture",task:"owned-mcp-task"}}`；不靠sleep或模拟方法返回表示已入场。
4. 只有真实Handoff通过校验，server才向worker发容量1的entered barrier。server随后扣住reply，worker此时才调用真实 `node.khor_agent_here(Action::Stop)`。直接home handler避免Stop也被假resident路由。
5. Python在收到实际abort后记录receipt，发 `notifications/cancelled(requestId=3)` 和done(cancelled=true)，然后正常退出。它是合作peer，避免把“Pi不理abort”与“Rust事件线程同步占住”混成一件事。

## 断言和预期首红（尚未执行）

- Stop admission <1s。沿既有共享process原10s grace观察Pi（观测窗口11s含调度余量），必须kill(pid,0)返回ESRCH，并确认fixture收到实际abort。记录Pi退出elapsed。
- 在同一Stop时刻起最多13s，分别核saved record为cancelled、running map无该root、executor claim可以try_lock取得。Pi已消失不能代替这三个条件。预期旧实现已打印Pi ESRCH，但第13s仍status=running/map_present=true/claim_busy=true，因真实IPC仍按Delegation300s期限等回执。
- 修后若本地收口成功，保持原IPC gate继续扣住，切fixture到只done模式，再下一Prompt；要求3s内done、map/claim释放、第二个Pi PID也ESRCH。不能靠测试先放晚reply让产品变绿。

**只有**实际IPC barrier、Pi abort+ESRCH以及cleanup成功都明确后，13s executor断言失败才算目标真红。编译/API错误、fixture无法启动、initialize/list失败、未进barrier、Pi本身未退役，均分别记setup/其他失败，不冒充这一真红。

## 所有权与失败收口

父测试own scratch、sidecar wrapper/脚本与测试binary Child，日志写scratch文件避免未读pipe堵父等待；专用worker外限40s。Python30s默认SIGALRM自退保险，不启动后代、外网、模型或远端；主测试按自有PGID护栏运行。

`HeldIpc` own listener/server JoinHandle、capacity1 entered/release channel；accept非阻塞、read有100ms syscall timeout，接收帧总上限沿实际proto::MAX_FRAME。启动/完整request屏障限5s。释放设置stop并唤醒release，server显式shutdown(Both)当前连接后返回，caller join；持reply gate另有25s最终保险，正常与断言失败均主动释放，不等保险超时。

`Cleanup` own root/run和HeldIpc；任何普通错误/13s失败先request_stop当前exact run，释放socket并join server，再最多5s观察production running map与claim释放。main worker先调用cleanup，打印结果，最后才assert原产品结果；Drop再次幂等兜底。对production线程不谎称拿到了JoinHandle，确认的是map移除+claim可重取（现public测试seam没有其JoinHandle）。假IPC的JoinHandle确实显式join。

这条路径只用async IPC reader，关闭自有socket使真实300s等待收到EOF/错误，原execute可跑完Session.finish/IO join并释放claim；测试不会13s失败就丢一个等300s的线程。若cleanup自身失败须原样报告并保留log，不屏蔽成通过，也不杀用户PID。父worker最终kill/wait仅针对自己的Child。没有裸信号清理来源不明的PID/group；PID零信号只做观测。

## 下一次运行方案与资源

root明确授唯一槽后再启动：fresh内外盘都>=5GiB，jobs2/shared target/自有PGID/墙钟与连续I/O错误护栏；按共享target跨树freshness纪律核依赖字节和实际重编，不能把顶层源码stable当依赖同源。所有3路径及MCP/IPC/Session依赖SHA、源码snapshot和binary SHA留证据 `Q037/mcp-cancel/`。

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Volumes/disk0/Khor/caches/rust/khor-quality-target cargo test --locked -j2 -p khor-node --lib khor_agent::mcp_cancel_tests::stopping_during_a_real_mcp_request_retires_the_executor_after_pi_exits -- --exact --nocapture --test-threads=1
```

首次600s含编译护栏；实际test parent限40s。编译期间源冻结。保留barrier、两个终态域、13s断言和cleanup结果，不放宽阈值。本轮root只授普通1parent+真实worker首红，执行结束已归槽；没有再起重复测试，尚不提交/实现修复。

## 实际第一轮：目标红，不是setup或护栏失败

root完成新manifest资源清理后授唯一槽。启动前fresh内盘7,297,744,896B、外盘10,747,314,176B；设置CARGO_INCREMENTAL=0，shared quality-target/jobs2不变。sync/core/a2a/acp/net/detect/catalog/node的8个当前树lib entry只更新mtime，前后SHA完全一致，详见 `Q037/mcp-cancel/first-party-mtime.json`。原始log明确8个crate均实际Compiling本树路径，避免复用其他worktree旧trait/API artifact。

运行 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/mcp-cancel/first-red-01`：runner55924、Cargo PID/PGID55943；命令如上，guard600s。Cargo编译58.04s；总71.607s；exit101，1父失败、其专用worker实际执行1次，未触任何资源/墙钟/I/O护栏。21份输入源码source-before/after完全一致，CARGO_INCREMENTAL=0记录在result。最低内盘7,288,020,992B、外盘9,838,673,920B，均高于5GiB。

原始顺序明确：

1. `barrier: decoded actual MCP task_status → IPC Delegation/Get; reply held`，真实入场，不是注入有利dispatcher替身。
2. Pi fixture PID56707 收到合作abort，**24.579ms已ESRCH**。随后打印map_present=true、claim_busy=true，旧IPC回执仍扣住。
3. **13.001s**目标断言：`Pi already reaped, but Rust executor exceeded 13s while IPC reply stayed held: status=running, map_present=true, claim_busy=true`。
4. 原断言报告前已执行 `explicit IPC release+fixture join and execute/map/claim drain: Ok(())`；即先关真实IPC socket、join自有server、等原execute释放map/claim，再以原13s状态fail，没有丢下300s等待。

第一轮已满足前文判红条件，因此记录为目标真红。没有误将“Pi还活”作为原因，也没有把本地Pi终止冒充Rust executor已结束。下一Prompt分支因原13s断言失败尚未执行，不能计通过。

`Q037/mcp-cancel/first-red-receipt.json`保存baseline、全部SHA/8crate实际重编行/guard结果；实际binary `debug/deps/khor_node-0bdf6253bcb5cae0` SHA256 `6357f72c4950cf7774df8481178bea42b8aeaa9b5455d052ef187b2338c7c557`。交付晚查PID56707仍ESRCH。源码仍为上表3份未提交测试候选，无生产实现变化。root已明确接回唯一槽；后续架构需先核实根接口，不能简单spawn_blocking+abort假称join。

## 范围声明

验证实际Rust MCP dispatch和实际IPC收发；不冒充真实PiSDK/model/mesh、远端agent已停止或远端回滚。库ledger/connect File.lock、重复Stop取消sweep、CLI MCP EOF、真实SDK控制仍按审计卡后续单批。本批只构造第一个可观测的同步MCP local-owner首红，不提出已完成Q037的结论。
