# Q037 调用方关闭边界审计

状态：已确认缺口、待设计/修复/实际验收。Owner root。依据 R1 c81/8292 源码与集成8395同一调用面；node_transfer_owner完成只读追踪，未改代码。不能把底层 runtime-close 测试通过写成全部 App/serve 退出已干净。

- 当前 commands/CLI 没有单数 `agent` 命令，也没有 `khor_agent_on` 调用；`retry` 是文件接收的另一业务。因此不存在已审的 `khor agent … Stop/Retry` 一次性 CLI 命令尾留案例，不新增该命令。
- 真入口 GUI core→`Node::khor_agent_on`，先经 resident IPC，无 endpoint 可本机 `khor_agent_here`；不能假定总经 resident。本机 Stop 需要本进程 running executor，RetryCancellation 可自行准入 metadata。
- Tauri Exit 已明确调用 `close_agent_runtime`，当前所审 `serve_as` 只有启动 acquire，没有 close。其正常 return/cancellation 与 keeper orphan/deaf/watchdog 强制 `process::exit` 需要分别审查；强制路径的可用性契约不能因等待卡死清理而丢失。
- static Scheduler/driver 不会在 `process::exit` 自动析构。Native child 有原有5秒 fuse，这是有界尾留，不是 join，也不证明退出后零逻辑写入。Step 尾留在 driver 之前已存在，fastStop又允许metadata未完就返回；新driver允许返回后启动已准入后续Step，未增加新的child种类或取消原fuse。

下一步：沿实际 serve/GUI teardown 找出正常退出与强制恢复边界；确定停止新准入、仍允许运行中 owner 完成、Root closing/closed 与 transport 关闭的顺序。现 runtime close 只管理本进程 agent producers/取消 owners，不等于整个 Node IO shutdown；不要把全部异步服务清理塞进它或用网络结果冒根目录写入回收证明。正常退出验收须包含真实 API local fallback、并发已准入请求、所有 owned Native PID/PGID/claim 与 Root 不再晚写；强制退出的保证另列，不能伪称正常 drain。

本卡不阻止已完成的source07取消检查点提交；是完整P1/P5仍未关闭的明确调用方风险。

## 当前源码进一步核对（集成36e3918f，root）

实际实现位于 `crates/node/src/link.rs::serve_as`（不是lib.rs）；graph旧快照只作定位，实际源码已读。入口先acquire，然后bind/publish/listener等多处可早返；非resumable endpoint accept返回None是正常break，循环尾当前直接Ok。连接handler/本地handoff/pump等异步任务另有Arc，不能只在尾部加close便宣称所有入口停止。`remote_cancel::acquire`允许显式新工作重开已Closed的Root；closing期间会拒绝，CloseGuard结束后新工作可重开，符合当前API用途，但说明退出要先停止/收拢调用方，不能把Closed当进程永久退出闸。Tauri真正Exit当前close的Busy只记录日志；不提供整个运行中executor已停止的保证。

下一实现仍需沿现有服务owner确定正常drain与强制watchdog的边界；当前没有在未收拢服务入口时添加假关闭，也没有把强制恢复改成可能无限等待。此为已确认待修调用方风险，未提交生产修改。
