# Q033：当前GUI宿主协议统一

状态working；owner root。用户明确未上线不保旧Khor端兼容，依Q-033-current-protocol-plan最小范围执行；外部ACP能力/认证/数据保护不变。本片不是最终GUI→Rust上传验收。

实现复用 /Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery，从clean fix/quality-install-integrity切fix/quality-current-gui-protocol@686f2e42。没有新建worktree/target；此前installer作者无活PID/写权，旧分支保留。graph-first当前图built a723旧，实际源fallback。

root允许：node/gui_host.rs、gui-core/chat.rs/chat_send.rs/chat_io_tests.rs及当前host/上传/握手定向测试；必要catalog zh/en新增明确协议不匹配消息。不会动Node公共link/proto/lib（N1 owner已接写权）、terminal host/PTY、GUI布局、依赖/锁/公共package/CI、生成器（Q041e独立作者）。

先新增真实TCP gui_greeting测试，旧生产未改，exact empty/unknown/terminal/other-gui拒绝测试运行中；证据Q033/current-greeting-red-01，exec38946，300/420/5GiB且独占Cargo。正常current greeting/no extra terminal negotiation、保拒绝原因和共享15s握手deadline另作适用回归，首次只跑目标拒绝用例。

下一步删upload_cancel字段/legacy_upload分支/Disconnected，所有当前连接走Stop+discard+Parts；保真正IO错误需要reconnect=true、token/null归属、安全残片/同socket下一turn/settings权限和queue-drop完成。旧host矩阵删除记录为范围变更，不当新测试通过数。先冻结再相关current host/core/native检查，最后同源App真实验收仍open。

## 本片已提交集成，整体Q033仍open

旧生产真实exact1普通父拒绝非当前greeting真红；候选01 normalCore+握手4+coreIO12通过，host编译触5GiB门自动停（不是业务红）。storage-audit-10精确回收5个已完成自有test executable缓存1,004,644,176B后，host02揭test Drop attached缺mut的E0596（编译setup失败，未执行host）。02只修该fixture mut，五生产及另外两test source完全相同，host03实际3普通通过。合计19不同普通，未重复已过16充数。全部source1921前后稳/实际3binary/双盘/6PGID后验ESRCH/私有scratch清，非作者Q-033-current-protocol-review已核最终无阻断。

作者提交13a9459f6c2dfd8d12912e6e2325b58de6e06999（8源=5生产+3test），cherry到wave1为767aa23d95da6dc5fe09680d780a6c866062ae3e。两commit完整tracked tree同为87f24fd650a7fd5183e1651bdd5aa96548dcf152，集成clean，8blob=受测freeze，未改源重复19。此次以完整tree身份与已验证调用链核集成，尚未编译新全源App/刷新最终frontend producer；P5仍须组合实际App与真实GUI→Rust→host→ACP上传取消，不能称完整Q033已关闭。
