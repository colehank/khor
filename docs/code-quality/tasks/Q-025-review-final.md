# Q-025 最终第三方复核

审查者：agent_redaction（未参与 Q025 生产实现）。审查时间：2026-10-25。
对象：`fix/quality-native-io`，基线 `b3142d338b33454e16fc81536bfb0a9e9719c4d8` 的未提交 diff 与新增文件。
证据：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q025/review-final`。

## 当前结论

**独立源审与本轮调度/IPC回归通过，可由 root 收齐最终 core 复跑后提交 Q025。** 原 `native-arguments-ready-20261025/cargo.log` 的 E0282 已由显式 `fn(Invoke<_>) -> bool` fallback 修复。审查者核对 `native-arguments-typed-20261025/{cargo.log,result.json,source.json}`：真实 `native_io::tests` **2 / 2 通过**，退出0，source_stable=true；其7项 source/lock 指纹逐项与当前文件一致，没有用源码存在代替运行结果。

除此之外，对本轮目标“同步入场、异步完成、保序、不持网络锁阻塞主线程、最后 leave 打断连接”的生产差异未发现新增阻断。以下 Stop 取消问题为独立生产红，父基线也存在；root 已决定以 Q033 承接，发布前必须解决。paste 锁等待问题已经在 Q025 当轮修复并由审查者复验绿。Q025 的 Stop 测试只证明 Stop 能插入分片之间，不证明用户取消已经生效。

## 发现

### P1：Stop 在上传中或首 Part 前到达，之后仍可开始原 turn（Q033）

`crates/node/src/gui_host.rs:221–244` 的原生产 `Reassembly::take` 对 Stop 直接透传，不取消 held 内容；主循环 `1184–1202` 在尚无 in_turn 时回复 NO_TURN。后来的 Parts 仍产生 SayWith，`1128–1155` 正常启动 prompt。客户端双队列及 prior-controls barrier 没有为尚未落线的 turn 建立取消身份；Stop 也可先于首 Part 到达。

独立 probe `stop-production.rs` 原样提取当前 Reassembly，使用缓存真实 `khor_node::gui_host::GuiOp` 与 `khor_node::proto` 编解码，非重新实现协议。实际运行 **0 通过 / 2 失败**：上传中 Stop 后仍输出 SayWith；首 Part 前 Stop 后仍输出 Say。`stop-production.log` 保留断言和实际结果。该 probe 直接证明生产重组行为；NO_TURN 与 prompt 启动分支另由源代码核对，未声称启动了真实 ACP host/agent。

修复必须保留连接：停止当前 upload 后明确丢弃残余 Part，直到 Parts 恢复正常状态；不能仅清 held，再将后续片段的解码失败/断连当作取消成功。还须覆盖取消后同一连接下一 turn 成功。

### P2（已修复）：paste 已出队但等锁时取消，仍会落入终端

`apps/gui/core/src/term.rs:935–955` 只在排队工作开始时检查 stopped，然后可能等待 parser/writer 锁；锁等待期间 cancel_drop 设置标志后，paste 不再检查，最终写入并返回 Ok。

`paste-production.rs` 原样包含生产 `term_paste` / `paste` 以及实际 CommandQueue。为确定性触发窗口，仅用最小 Parser 和内存 writer 适配器，在取得 parser 锁之前发测试同步信号。实际 **0 通过 / 1 失败**，取消发生在尚未写入时，释放锁后依然写 19 字节并返回 Ok。此结果不能替代真实 TCP/应用取消验收，但明确证明检查位置与锁等待窗口。

原有 `cancelled_queued_paste_sends_no_input` 的测试结论成立，仅覆盖尚未开始执行的排队 paste。Q025 不应将该用例描述为所有未落线输入均可取消。后续需明确取消线性化点；首字节已进入阻塞 write 后的行为与尚未写任何字节分开约定，不能承诺回滚已经发给 PTY 的内容。

root 已将 stopped/left 再检查放在取得 writer 后、首次 write_input 前，审查者未改该生产修复。新增驱动 `scripts/check-native-io.mjs` 每次从当前 term.rs 提取 **原样函数**，插入 `scripts/fixtures/native-paste.rs`；只对 Parser 和写入 sink 适配，并在真正阻塞锁前同步测试线程，无 sleep。相同新回归使用修复前已保存生产函数真实运行 **1 通过 / 3 失败**（parser 等待取消、writer 等待取消、writer 等待 leave 均落线；正常发送成功），当前修复后 **4 / 4 通过**。日志 `paste-regression-red.log` / `std-final-green.log`。修复后的线性化点是取得 writer 后、允许首次写入的检查；之后的已在飞输入无法回滚。

## 本轮生产差异复核

- Chat 的 preceding_controls 哨兵在同步入场时插入 controls FIFO，turn worker 仅等待此前控制完成；后到 Stop 不等待整 turn。已核对 SetMode/SetOption→Say 的真实红绿测试及生产调用，无全 turn 控制 barrier。
- Chat/Term 在入队时捕获当前 attachment Arc。最后 leave 先移 registry，再 close 队列并独立 shutdown；旧队列未开始工作得到错误，在飞 socket 写被打断，同 id 重开不改变旧捕获对象。
- Connection 的 interrupt 独立于 writer；replace 在取得 writer 前先 shutdown 旧 socket，安装前再次检查 closed。没有发现 writer/interrupt 反向持锁环或 leave 后晚到 rejoin 复活连接。
- Term key/resize/ready/forget 使用同一 FIFO；forget 与 Ctrl-L 在同次工作内完成，paste 保持一组 bracketed 字节原子排队。term_open 的 initial keep_history 等待已经移出 registry 锁，取得 attachment 不再间接等该网络写。
- NativeIO 14 项 COMMANDS 逐项对应 parse/submit 与原命令参数。dispatch 的实际 invoke_handler 注册存在，未知命令返回原 Invoke 进入 generated handler。CommandItem 保留 name/key/message/acl，Option 参数默认值与原签名一致。
- 已直接核对本机 Tauri 2.11.5 `webview/mod.rs:1741–1845,1907–1912`：invoke key、origin 和 ACL 拒绝发生在 run_invoke_handler 前；当前 build.rs 仅调用 tauri_build::build，未发现本次新增绕过权限入口。真正参数/remote ACL mock 回归仍需 native Cargo 绿。
- 命令普查确实将实际注册的 COMMANDS 加入对照，并拒绝空列表、非法字面量、重复注册；移动静态门也识别这14个入口。它们是入口完整性检查，不能独立证明命令内部不阻塞。

## 实际验证与限制

审查前后源指纹一致：`review-final/source-sha256.json`。graph-first 调用了图上下文/变更/语义搜索；服务固定 main，未覆盖本 worktree 当前节点，因此以当前生产 diff、调用点和真实证据复核，未把空图当安全证明。

独立执行：

- `node scripts/check-native-io.mjs`：**7 + 4 全通过**（直接包含生产 CommandQueue/Connection，并提取当前生产 paste 函数；预期 fixture panic 被隔离并正确完成错误）。
- `node apps/gui/scripts/check-mobile-static.mjs`：退出 0。
- `git diff --check`：退出 0。
- 两份生产取消 probe：**2 个 Stop 红 + 1 个 paste 红**，没有修改生产源码。

核对已有实际证据：

- `core-io-20261025/result.json`：真实 Cargo core io_tests 退出 0；结合 `Q-025-review.md` 的 barrier 修复后复跑，当前同源码组为 **8 / 8 通过**。
- `command-tables-final/tests.log`：真实生产命令普查 **2 / 2 通过**，其 source.json 与当前对应源码指纹相符。
- `native-arguments-ready-20261025/result.json`：原真实 Cargo native 测试编译失败，退出101，不计测试红或绿。修复后的 `native-arguments-typed-20261025` 真实组 **2 / 2 通过**，18.16秒编译；参数/缺参/类型/optional/raw/u16与原宏对照、同步admission、unknown fallback、remote ACL拒绝在两项测试内实际运行。当前 source/lock 全部匹配。

本审查没有运行第二个 Cargo，没有修改生产 Rust 源码，也没有执行 main 合并/发布。经 root 单独授权，仅新增上述重复提取生产函数的脚本/fixture 回归。审查后 root 两处生产修复另记录 `review-final/post-fix-source-sha256.json`。Q025 完成建议：native 已真实绿，root 收齐生产 paste 修复后的最终 core8 复跑并复核新增驱动后，可独立提交该调度改造；Stop 取消闭环的发布门由 Q033 承接，不因其是既有问题而遗漏。

## Q033 接口与兼容草案（需独立授权后实施）

客户端在 submit_say 同步生成并注册 attachment 内发送状态，payload 仅由 turn worker 所有。Stop 入场取消此前已入场的相应 turn，不影响后入场的新 turn。每个单帧/Part 在同一 writer 临界区检查取消，消除检查与写线之间的竞态。已发 Part 的取消，应先确认 Stop 实际写线，再发仅用于结束丢弃状态的 Parts；控制队列不能反向等待整 turn。

Host Reassembly 遇到有 upload 的 Stop 进入 discard-until-Parts，控制消息继续处理；残余 Part 不再积累，Parts 仅复位且不产生 Say。未发首帧的客户端本地取消，不再发送旧 Say。需明确 NO_TURN/取消反馈与成功完成回包的语义，避免 UI 把已停止的 turn 当发送成功。

不增加 wire variant 时，新 host 可接旧 client（继续收到的残片被丢弃）；新 client 对旧 host 的部分终止仍可能得到 NO_TURN/无显式取消回执，不能宣称同等体验。实现前需决定能力/版本边界或明确受限兼容，并增加实际 host/agent 与同 socket 恢复测试。
