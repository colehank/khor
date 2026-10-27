# Q039 真实 checked-flush 后崩溃恢复验收

状态：检查点已集成，整合版本 native/App 验证待。作者 node_transfer_owner；非作者 node_test_review 与 root 独审。

实现 `8f2f1d1b6c126de1ac0b34828d90000208477bd7`，集成 `3a2bb9bfd866ea2c0760bc12bf66aac74982a79e`。仅两条源码：`doc_transfer_state.rs` 的 cfg(test,unix) 模块及真实 apply/drop(sink) 后 hook；`doc_transfer_state/crash_tests.rs`。生产算法不变，两文件 SHA 与冻结 candidate14、作者提交及集成版本逐字一致。

验收从实际配对、认证网络与 public sync 开始，只有本次 Root/不可变 manifest 能安装暂停。receiver 在 checked flush 已完成、journal 仍为 Importing、尚不能保存 Complete/构造完成回执时，由所拥有的 Child 句柄 SIGKILL；wait 确认 signal9。首请求退出后源端仍为 Ready、保留 stage，不能伪称 Applied。独立 verifier 进程重开，证明恢复为稳定的 AlreadyHaveEquivalent、通知去重及并发 Pin/epoch 保留。

旧 ACK 前另留真实新 generation 的 Receiving/256KiB stage。错误角色/hash ACK 后 fresh-open journal 与旧、新两个真实 stage，再发正确 ACK，证明精确释放旧 ID、保留新 ID，迟到 chunk 不会重新创建旧状态。13 候选曾漏掉错误 ACK 与正确 ACK 之间的物理保留检查；14 已补足，两个审查者确认 open_existing(false) 不会以创建缺失文件掩盖误删。

实际证据：外盘 `Q039/node-transfer-n1-crash14-control12`，内盘 recovery 下同名目录。`final.json` 与 `crash/raw.log` 为原件。一次必要 libtest build exit0/71.375s，唯一业务父 exit0/19.310s；verifier 与 parent 各1/0/0，receiver 被实际 SIGKILL，不充第二业务父。整 run 97.513s，source1085/actual binary/dependencies 稳定，两 owned PGID92917/94169 均退、leader reap、无 forced drain，TMP 真清；root 独读 raw/final 并逐 SHA 核 47 份双盘文件。

控制11遗留旧 baseline HEAD，执行前发现，未 launch。12 只将 HEAD 精确更新 d7a4192d 及独立输出路径，1085/manifest 与所有原门保留。原11不回填结果。

范围限制：前半是真实网络，恢复和 ACK 半段是 fresh State API；不冒称第二次网络 pump 或整机断电。原 source11 的17项与 N0 收敛证据仍适用其当时源码，未在此重复执行。N2 大 Pair/Vouch 恢复、优化构建下真实32MiB网络边界、组合 native 与最终 App 均未因此完成。
