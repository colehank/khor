# Q037 Stop 后已准入取消的自主推进

状态：working，尚未修复或验收。实现 owner：node_test_review；独审：root 与 node_transfer_owner。树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，基线 c81d0a18 加完整保留的 source04 五路径。driver 仅授权 `remote_cancel.rs` 及必要纯策略回归源，不改原 MCP/R0/close fixtures；无新 Cargo/native 授权。

## 真实红与已通过范围

`Q037/remote-r1/fast-stop-validation-run04` 实际正常CLI build0/69.34s、Node libtest build0/66.30s，并通过 Q055 candidate05 的完整来源验证；没有使用旧17b4。zero/2s metadata 两业务父各内外1/0/0，first Stop 均1ms、真实 Status 最后确认 DurablyQueued；slow 约2035ms后观察到持久化。原6个 runtime close 各1/0/0。合计8个 distinct 业务父通过。

R0 第0个本地 Pi 已退出、map/claim 释放且 Stop 为 PersistenceUnknown，但原2秒窗口内没有进入真实 held Cancel，内外父均0/1/0。根因链：OwnedWorker 会自行完成和回收 Native 进程；Scheduler 只有被 pump 才取其结果并 enqueue/start 远程 Step。fast Stop 退出等待后无独立 pump，所以后续进度依赖 Read。这是产品红，不能通过向 fixture 增加 Status/Read 驱动来掩盖；尚未到四 held/fifth-sixth 窗口，不预判其结果。

整 run 168.060s、整体非绿，source2353/helpers/dist 稳定，所有 owned PID/PGID 退出，无 forced，TMP清；root独读 raw/result 并核28dual。CLI实际SHA35cee3e6c87dc9a43b23fb0b30430952e0c28ea472ee97a53e34ee905f6b50d4，Node test c7a08fe054b84e9ea0cdb6f2e6733a9e3ae8575845493463652d47d8d22f1c85。未来合法共享缓存覆盖不回填本次原收据。

## 最小实现边界与审查意见

单一 process-owned driver 静态持唯一 JoinHandle，idle 用 Condvar 无限等待；无 Root 捕获、无 Root IO、无自动 Discover。start 在启动 Native worker 前确保 driver 可用并唤醒；失败必须诚实不准入。仅 active metadata/steps 用有限5ms重查，避免 notify 早于 JoinHandle::is_finished 的尾部竞态。future ready 按最早 deadline；paused/closed/closing quota 满不能零超时忙循环。控制锁必须释放后再取 Scheduler，bool/epoch 处理通知避免 lost wake。

将已排队 retired→Persist 与 next_home/Discover 拆开，driver 可推进已准入持久化，不能据 pump(false) 原行为声称已涵盖该路径。保现1 metadata、4 Step加closing配额、Home/Closed/Closing union256；Root close 继续锁外 join 仍在运行的原 owners，不 join 共享 idle driver。CloseGuard.drop 释放配额/解除 fence，allowed/queue 变化要 notify。闭合/已关闭 Root 不得因迟到唤醒重新准入。现 Step 自身 network_allowed 双检查和 Home.allowed 保留；不声称新增即时 UI resting 保证。

先做最小 driver，不推测新增持久化排队模型；若后续真实 R0 到5/6时暴露 Busy，需要依实际证据再定，不能任意接受 NotPersisted 或删 durability/delivery oracle。原源04 backup 与原 run02/03/04 各失败分类全部保留。
