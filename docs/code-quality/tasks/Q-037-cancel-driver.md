# Q037 Stop 后已准入取消的自主推进

状态：source06 driver 已完成双审、实际原9父及策略3通过；完整R1回归批次尚未闭环。实现 owner：node_test_review；独审：root 与 node_transfer_owner。树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，基线 c81d0a18 加完整保留的 source04 五路径。driver 仅授权 `remote_cancel.rs` 及必要纯策略回归源，不改原 MCP/R0/close fixtures；无新 Cargo/native 授权。

## 真实红与已通过范围

`Q037/remote-r1/fast-stop-validation-run04` 实际正常CLI build0/69.34s、Node libtest build0/66.30s，并通过 Q055 candidate05 的完整来源验证；没有使用旧17b4。zero/2s metadata 两业务父各内外1/0/0，first Stop 均1ms、真实 Status 最后确认 DurablyQueued；slow 约2035ms后观察到持久化。原6个 runtime close 各1/0/0。合计8个 distinct 业务父通过。

R0 第0个本地 Pi 已退出、map/claim 释放且 Stop 为 PersistenceUnknown，但原2秒窗口内没有进入真实 held Cancel，内外父均0/1/0。根因链：OwnedWorker 会自行完成和回收 Native 进程；Scheduler 只有被 pump 才取其结果并 enqueue/start 远程 Step。fast Stop 退出等待后无独立 pump，所以后续进度依赖 Read。这是产品红，不能通过向 fixture 增加 Status/Read 驱动来掩盖；尚未到四 held/fifth-sixth 窗口，不预判其结果。

整 run 168.060s、整体非绿，source2353/helpers/dist 稳定，所有 owned PID/PGID 退出，无 forced，TMP清；root独读 raw/result 并核28dual。CLI实际SHA35cee3e6c87dc9a43b23fb0b30430952e0c28ea472ee97a53e34ee905f6b50d4，Node test c7a08fe054b84e9ea0cdb6f2e6733a9e3ae8575845493463652d47d8d22f1c85。未来合法共享缓存覆盖不回填本次原收据。

## 最小实现边界与审查意见

单一 process-owned driver 静态持唯一 JoinHandle，idle 用 Condvar 无限等待；无 Root 捕获、无 Root IO、无自动 Discover。start 在启动 Native worker 前确保 driver 可用并唤醒；失败必须诚实不准入。仅 active metadata/steps 用有限5ms重查，避免 notify 早于 JoinHandle::is_finished 的尾部竞态。future ready 按最早 deadline；paused/closed/closing quota 满不能零超时忙循环。控制锁必须释放后再取 Scheduler，bool/epoch 处理通知避免 lost wake。

将已排队 retired→Persist 与 next_home/Discover 拆开，driver 可推进已准入持久化，不能据 pump(false) 原行为声称已涵盖该路径。保现1 metadata、4 Step加closing配额、Home/Closed/Closing union256；Root close 继续锁外 join 仍在运行的原 owners，不 join 共享 idle driver。CloseGuard.drop 释放配额/解除 fence，allowed/queue 变化要 notify。闭合/已关闭 Root 不得因迟到唤醒重新准入。现 Step 自身 network_allowed 双检查和 Home.allowed 保留；不声称新增即时 UI resting 保证。

先做最小 driver，不推测新增持久化排队模型；若后续真实 R0 到5/6时暴露 Busy，需要依实际证据再定，不能任意接受 NotPersisted 或删 durability/delivery oracle。原源04 backup 与原 run02/03/04 各失败分类全部保留。


## source06实际闭环及旧fixture后续

source06冻结 Q037/remote-r1/metadata-startup-fix-source06（11dual/2353inputs，仅remote_cancel.rs相对04变），生产SHA0c873c445842b32ba46b7b1ef2a034b707cc4ce972a65565d3189e767dd9a467。单一成功driver用Mutex<Option<Driver>>保存，临时spawn失败不永久缓存；所有通知在Scheduler下，driver先释放control再取Scheduler，末尾在Scheduler内清已消费pending防自通知空转。原OwnedWorker与1metadata/4step+closing/Root256配额不变，退役Persist自主推进、Discover不自动启动。root与N1 owner独审均无阻断。

root实际run05正常CLI/Node两build均0，原9父全通过，整体176.634s/green=true。R0原先第0run红已闭环：原六/fourheld/fifth-sixth delivery/全部真实最终Status-intent与cleanup不改，未向fixture添加Read/Status驱动。资源、source2353/helpers/dist、owned groups/TMP均过。

同二进制、0Cargo的runtime-tail-run01随后运行：3个新driver策略及前2个原receipt测试过，在metadata_lock_failure_still_requests_local_stop_and_history_can_retry旧第547行失败（Unknown≠NotPersisted）。该fixture Stop返回即drop(metadata.lock)，异步后并无证据Native producer真正遭锁；只改断言会掩盖负面路径。原失败及5个pass/全部cleanup空已保双盘。node_test_review容量错误后停止，源未改；node_transfer_owner接手仅remote_cancel_tests.rs三处最小适配，保持生产source06字节不变：锁held直到实producer NotPersisted Busy/无intent；invalid-reply先真实确认commit再保原4bad decoder断言；known continuation诚实初始快照但真实ConfirmedCanceled1/Task Canceled/parent/PID及cleanup原门全部保留。无新native授权，完整原回归及paired/Tauri/组合App仍待。


## source07检查点已提交集成

三处fixture07只改变初始快照/实际故障观察时机，生产06 SHA保持完全相同。锁case先持真实OS锁到公共Retry得到实producer NotPersisted/Busy且intent实际缺失，再释放；另两case保真实commit、四baddecoder及最终精确Task Canceled/parent/PID/cleanup。root独审并重做实际构建。

当前同source07的31个distinct全部过：run06两build+9父 green/178.494s；tail02同binary0Cargo的21父 green/66.416s；paired-run09必要normal ACP stub+真实delegation_wire target两build、原paired exact1 green/68.941s。paired入口正确目标是delegation_wire（harness=false但actual Cargo profile.test=true）；旧stub缺旧toolchainSHA不复用，新stub完整proof，环境仅注入测试child，父producer环境不变。root逐SHA核9阶段28dual、tail22dual、paired19dual及全部2353当前输入，所有 owned groups/TMP清，无forced。

root exact6提交8292e6543d26b323e99e7bc99e65ad2524a9bf12，集成8395e52a15bd9c279984dafe3cc60827e391e6b9，两树clean。Graph detect/flows仍报告0flow且build SHA有差异，因此不以它代替实际源码/31测试覆盖。完整R1/R2、组合native、真正App/serve退出仍待；具体调用方缺口见Q-037-shutdown-callers。旧红/控制setup/verify失败不回填成绿。
