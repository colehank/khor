# Q033 SendJob.complete 警告独立核实

结论：本次疑点未成立。`job.complete = true` 的 unused_assignments 警告可原样复现，但 Drop 实际读取的是被设为 true 的同一个 SendJob，未产生“Copy bool 脱离 guard”或 finish 丢失。无生产修复、未屏蔽警告、未运行 Cargo。

检查集成：`bd58566a6a31dc3864c5b3d90f5ce381a4592abd`，`apps/gui/core/src/chat.rs:300` 的 SendJob/Drop 与 :678 的完整 submit_say_with_token 闭包。chat.rs SHA `7c503a161da3a2f7d6ed3b6887e815eeb67a17938da893880530582a7d304618`、chat_send.rs SHA `5017cb830a61665985aa6f04572b556637f713eeadb61a21c3d6890906540ab4`、chat_io_tests.rs SHA `1622b571b2d32ca68654a10c0ad339bada43f151680fc0382fa807da4838376a` 均与原 Q033 SendCommitted 交付 manifest 一致。

## 最小独立验证

证据 `Q033/review-drop-capture/`（根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`）。`probe.rs` 用同样的 move/FnOnce 闭包、内层 and_then 借 job 字段、赋值末尾 complete=true、SendJob Drop 按 complete 判 failure 并 finish；只将网络操作替换为可控 Result、计数器和 drop 观察日志。rustc1.95.0 edition2024（与仓库 edition 一致）构建实际发出相同 unused_assignments 警告，随后程序三条真实断言通过：

1. 正常执行：提交后 guard 尚未 drop，调用后 Drop 只一次且 complete=true，failure0、finish1。
2. 显式错误：闭包先记一次失败再置 true；Drop 读取 true，不重复错误，finish1。
3. 排队 closure 直接丢弃、不执行：Drop 读取 false，失败1、finish1。

另外 `rustc --edition=2024 --emit=mir probe.rs` 输出 `probe.mir`：closure 的 `debug job => (_1.1: SendJob)`，正常路径先 `((_1.1: SendJob).2: bool) = const true`，随后 `drop((_1.1: SendJob))`。这直接证明该 compiler 捕获的是完整 guard，赋值和 Drop 指向同一 capture，而非另一个 bool。编译 stderr 在 rustc.log/mir.log，运行 stdout 在 run.log。

理论相符：Rust RFC2229 的精确捕获不会细化到需要从 Drop 类型非法 move-out 的字段，所以此 move closure 不能按普通无 Drop struct 的 disjoint capture 直觉处理。[官方 RFC2229](https://rust-lang.github.io/rfcs/2229-capture-disjoint-fields.html#reference-level-explanation)

不把 lint 的“never read”诊断解释为已证明 runtime 未读取；本例显式源码末尾不再读该字段，但隐式析构会读。保留警告供后续明确 API 表达治理，不为本调查随意 allow 或加无意义 read。

## 现有真实实现测试

直接复用已有已编译 core 测试二进制 `.../debug/deps/khor_gui_core-d3a0aad91eaddf52 io_tests --test-threads=1`（没有 Cargo/重编），本轮 **15通过、0失败、118过滤**，包括12项真实 chat socket/queue 测试与3项 terminal FIFO。日志 `core-reuse.log`，元数据 result.json。既有原 Cargo 交付 manifest 能对应上述三个精确源码 SHA；本次重用不冒充重新编译整个当前集成。

其中成功带 token/no token、HTTP admission→真正 commit、队列close旧token与新成功token精确 singleton frame断言均通过；成功提交后没有额外 false failure frame。failed_send 的 claim_receipt 本来也防重复收据，故单看 frame 测试不足以证明 guard complete，已以上述独立 Drop 日志与 MIR 补齐；Sending.finish 赋 Stage::Finished 并非发送错误。

图先行：作者集成图仍停97e1a200，SendJob/submit符号搜索没有覆盖结果，因此回落到固定实际源与执行证据；没有拿图的低 risk/缺覆盖当成正确证明。生产文件只读，报告不宣称跨所有 Rust 版本或无 Drop 变体保持同样捕获语义。
