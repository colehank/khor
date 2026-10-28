# Q033：当前GUI宿主协议统一

最新状态：当前协议实现、集成与真实GUI验收完成；原运行器非绿与独立补验边界见本卡末尾P5记录。早期状态按历史保留。

状态working；owner root。用户明确未上线不保旧Khor端兼容，依Q-033-current-protocol-plan最小范围执行；外部ACP能力/认证/数据保护不变。本片不是最终GUI→Rust上传验收。

实现复用 /Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery，从clean fix/quality-install-integrity切fix/quality-current-gui-protocol@686f2e42。没有新建worktree/target；此前installer作者无活PID/写权，旧分支保留。graph-first当前图built a723旧，实际源fallback。

root允许：node/gui_host.rs、gui-core/chat.rs/chat_send.rs/chat_io_tests.rs及当前host/上传/握手定向测试；必要catalog zh/en新增明确协议不匹配消息。不会动Node公共link/proto/lib（N1 owner已接写权）、terminal host/PTY、GUI布局、依赖/锁/公共package/CI、生成器（Q041e独立作者）。

先新增真实TCP gui_greeting测试，旧生产未改，exact empty/unknown/terminal/other-gui拒绝测试运行中；证据Q033/current-greeting-red-01，exec38946，300/420/5GiB且独占Cargo。正常current greeting/no extra terminal negotiation、保拒绝原因和共享15s握手deadline另作适用回归，首次只跑目标拒绝用例。

下一步删upload_cancel字段/legacy_upload分支/Disconnected，所有当前连接走Stop+discard+Parts；保真正IO错误需要reconnect=true、token/null归属、安全残片/同socket下一turn/settings权限和queue-drop完成。旧host矩阵删除记录为范围变更，不当新测试通过数。先冻结再相关current host/core/native检查，最后同源App真实验收仍open。

## 本片已提交集成，整体Q033仍open

旧生产真实exact1普通父拒绝非当前greeting真红；候选01 normalCore+握手4+coreIO12通过，host编译触5GiB门自动停（不是业务红）。storage-audit-10精确回收5个已完成自有test executable缓存1,004,644,176B后，host02揭test Drop attached缺mut的E0596（编译setup失败，未执行host）。02只修该fixture mut，五生产及另外两test source完全相同，host03实际3普通通过。合计19不同普通，未重复已过16充数。全部source1921前后稳/实际3binary/双盘/6PGID后验ESRCH/私有scratch清，非作者Q-033-current-protocol-review已核最终无阻断。

作者提交13a9459f6c2dfd8d12912e6e2325b58de6e06999（8源=5生产+3test），cherry到wave1为767aa23d95da6dc5fe09680d780a6c866062ae3e。两commit完整tracked tree同为87f24fd650a7fd5183e1651bdd5aa96548dcf152，集成clean，8blob=受测freeze，未改源重复19。此次以完整tree身份与已验证调用链核集成，尚未编译新全源App/刷新最终frontend producer；P5仍须组合实际App与真实GUI→Rust→host→ACP上传取消，不能称完整Q033已关闭。

## P5最终当前协议验收（2026-10-28，root与独立审查）

本节替代上文按时间保留的“等待集成/最终App待验”状态。集成`495d096d54ea083304031423ca052f87dd9bc75e`，完整2388输入frame`36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e`，正常GUI executable`f76302af7b61b59c1d01172ee4209c1c9cb6c80bd6381ee760468527624c865d`。当前协议统一已集成；用户“未上线，不考虑旧Khor端兼容”替代历史mixed-version要求，旧兼容证据仅保历史。此前固定后端/组件回归及最终原115组合的确切版本映射见STATE，不冒称旧binary覆盖后续入口。

`P5/Q033-real-app-owned-run05`由root通过CUA实际系统文件选择器逐张加入4张PNG，合计base64 23,048,528B；原稿A上传首Part后真实Interrupt，30s门内explicit release，取消收据reconnectfalse、无commit，正文与4附件恢复后捕a（ACP prompt0）；不改稿不重选直接B，独立新token、正文547B SHA和4图片hash/order均与A一致，同连接commit→真实Allow/chat_answer→EndTurn，捕b（prompt1）；C无附件15B小消息commit→Ask→真实Stop→新Cancelled（prompt2）。真实会话配置选bold和two，原native mode/option成功、新settings/wire/ACP齐；完整原raw matcher确认后CUA CmdQ，实际App45305 return0、launcher未发App信号，host此后才stop，helper45282 return0、connections1、elapsed181608ms。全程未reopen/leave，非bridge。

保原运行**nongreen**：result`64428402c1b965c35f9dc1b2400063b12040d0adf8e4ed95b1b453cecf38c806`/189.108s，唯一末尾harness断言漏当前Vite正常TERM返回143（128+15），只认0/-15。独立审查`P5/Q033-real-app-run05-postreview01/report.json`，SHA`e6c297f0eebd31d713ceb6e85bf6b9fcff68a7c2152cda0b3cb2dab1c6fd075b`，42项补验实过：原纯check_q033、9snapshot长度SHA及live前缀、全部最终raw无晚A commit/unsent、当前source/helper/5producer/GUI/core、132PID/44group全退、原TMP清、双盘908条manifest。正常observer143的代码来源单列，未修改原result/退出政策，未重跑UI或伪造整体green；原5s内poll已由到达该断言证明，当前owner检查不冒称重现历史5s时序。GUI自动来源Q055的TAURI_CONFIG缺口另保nongreen。

当前Q033功能与当前协议真实GUI验收已完成，运行器问题记下一轮；main/ship未授权。Q035/Q043及embedded Pi仍按STATE继续，不能以本项代替整轮完成。UI准确值bold/two由root CUA观察，旁观raw刻意省略值，只独立证明命令/新状态；两种证据边界分别保留。
