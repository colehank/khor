# Q-025 独立审查

审查者：store_owner；状态：阻断，修复后需另一审查者复核。
工作副本：`/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io`，基线 `b3142d338b33454e16fc81536bfb0a9e9719c4d8`。审查的是实际未提交实现，未运行第二个 Cargo。

## 阻断发现

1. `chat::submit_say` 仅提交 turns FIFO，`submit_control` 仅提交 commands FIFO；两者无顺序依赖。ChatView 的模式/选项 onPick 是 fire-and-forget，随后的 Say 可先于此前模式或模型配置出线。独立队列能让 Stop 插入大 turn，但不能因此丢掉此前配置的因果顺序。root 已授权审查者仅接手 chat.rs 与 chat_io_tests.rs 修复：以真实 Chat/TCP 帧回归先红，再在同步 admission 加控制 FIFO barrier，turn worker 等此前控制完成后写；后续 Stop 不排在整 turn 后。修复前不得交付。
2. 原有 `apps/gui/core/tests/command_tables.rs` 只从 generate_handler 读命令；移入显式 dispatcher 的 14 项全部成为 only_served。直接 rustc 编译并运行原测试，实际 2 项：1 通过、1 失败。证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q025/review/command-tables.log`。这是普查失配，不能把运行时 handler 误报成缺失，也不能靠 ONE_SIDED 豁免修门。root 接手适配。同类 mobile-static 的 #[command] census 也必须覆盖 dispatcher。

## 已独立核对的边界

- CommandQueue admission 仅拿内存队列锁，不拿 writer；任务在 worker 上运行，Pending 成功表示 work 已完成，close 对尚未启动任务回闭合错误；丢弃 Pending 不撤销已入队输入。
- Connection shutdown 有独立 TcpStream 副本，不等待 writer 锁。replace 关闭旧 interrupt 后才等 writer，最终检查 closed；最后 leave 的旧 attachment 不可被迟到 rejoin 复活。term 任务同步捕获 Arc；close/reopen 同 id 不会重新查表落到新 attachment。
- DropJob 在同步入口捕获 cancellation registration 和 attachment，cancel flag 同步设置，磁盘清理异步；队列 paste 在执行前检查 stopped。尚未声明远端实际投递可撤销或取消原始传输 promise 有硬时限。
- 本地已固定 Tauri 2.11.5 的 Webview::on_message 在自定义 invoke_handler 前校验 invoke_key、origin 和 runtime ACL；dispatcher 使用原 CommandItem + 原 acl 解码。未找到自定义 handler 绕过 ACL 的证据。
- gen_bindings 只导出列出的 ts-rs 类型，不扫描 command 声明；这次移动本身未显示生成器命令缺失风险。移动门和命令表是另一个入口，须实际通过。

## 测试证明范围

原 core socket 5 项检验控制 FIFO/完成、阻塞 turn leave 中断、term input-resize-clear 协议顺序、queued paste 取消、旧 attachment 输入拒绝。它们没有跨 controls/turns 顺序用例，已补真实回归等待红/绿。std-only 手动分片的 a/S/b 只能证明调度帮助模块，不能替代真实 write_op 的 Stop 插片验收。native 参数 oracle/dispatch admission 测试需要实际 Cargo 结果；审查不把未完成的运行计绿。

审查期间不修改 command_queue、connection、term、native_io、command_tables 或全局 STATE/PLAN。root 暂停 Q025 Cargo，待 chat 回归及固定源码交接后继续。

## 配置顺序阻断的实际红绿（2026-10-25）

root 授权后，审查者仅接手 `chat.rs` 和 `chat_io_tests.rs`，未改其余生产文件。真实 Chat fixture 注册真实 attachment/TCP，先让控制 FIFO 的前置任务停住而不占 writer，然后依次提交 Mode、Option、Say。旧候选能立即读出 Say：精确 Cargo 测试实际 1 项，0 通过/1 失败，125 filtered，断言 `turn passed controls still waiting in their FIFO`。红日志 `review/chat-order-red.log`、exit101，未以模拟调度替代实际 core。

修复：`submit_say` 在同步 admission 时先向 commands FIFO 提交 no-op completion；turn worker 等该 completion 后才调用真实 send_turn。等待不在调用/main 线程；后来的 Stop 仍走 commands FIFO，不等完整 turn。原 controls 的错误完成及 leave 关闭 barrier 语义保留。

随后同一固定源实际运行：

```
cargo test --locked -p khor-gui-core --lib io_tests -j2 -- --nocapture --test-threads=1
```

实际 8 通过/0 失败、118 filtered。新增三项分别验证 Mode→Option→Say、16MiB 真实 write_op/TCP 中 Stop 出现在最终 Parts 前、leave 拒绝等待此前控制的 turn；原五项一并通过。编译5.81s，测试0.43s。证据 `review/chat-order-green.log` 和 `.json`（记录命令、退出码、源文件前后指纹一致）。diff --check 通过。此次 Stop 用例证明本次实际调度下可插片，不把 std Mutex 调度器说成有严格公平性/固定等待时限。

固定指纹：
- chat.rs: `ead3a475af4c7680b43a1e97405bec080dcde3de714fdd0dff20f3d98771d180`
- chat_io_tests.rs: `ede4a578e93cf98fafa4ff4fa97394871dded860854c3632ecdddba7505a4902`

唯一 Cargo 槽及两文件实现所有权已交还 root。由于审查者随后成为该修复作者，这两文件的最终实现需要其他人非作者复核；本卡不构成对自身修复的独立批准。命令普查/mobile 静态门由 root 适配，需其实际验收；本卡不提前解除整批交付阻断。
