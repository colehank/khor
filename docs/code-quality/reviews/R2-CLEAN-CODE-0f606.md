# 第二轮 0f606 增量 Clean Code 独立审查

2026-10-31，reviewer：round2_reviewer。当前源码为 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`0f606225ff669a5394ec68bd2fca7b4019a05841`；核查时 worktree clean。本文补充 [17f6 独审](R2-CLEAN-CODE-17f6.md)，不覆盖或重写原报告，不标记34项全部完成，也不授权合并或发布。

## 范围和结论

17f6→0f606 实际仅5个源码路径：原生菜单及窗口关闭两叶、GUI ghost 子进程及其回归测试两叶、WorkspacePane 清洁文件按钮一叶。复用 [逐片独审](../tasks/R2-REVIEW.md)，只再核当前组合的所有权和 IO 接缝。知识图谱构建于 main `e7c335c8`，不覆盖当前集成，故使用固定 git diff 和实际源码补足；没有重新审411路径、建立索引或执行测试、构建、OS probe、浏览器、App。

本次增量未发现新的源码阻断。两处真实产品反例已有对应最小修复和限定的实际绿；原失败保留。其余未变领域沿用17f6的逐项结论，不从这些局部证据外推平台通过。

## 改动到原则及实际证据

| 改动 | 当前源码与原则判断 | 已有验证及边界 |
| --- | --- | --- |
| Ghost 子进程生命周期，`crates/node/src/gui_host.rs:576–627`、`gui_host_cancel_tests.rs:26` | 私有线程先建立，再持有 `Command → Child → wait`；仅一次 PID/IO结果经 bounded channel 返回。调用方丢弃或超时不丢 Child。无跨等待 RowGate、全局 reaper、进程 registry 或重复 spawn。`BeforeSpawn` 与 `Unconfirmed` 显式分开，未知结果不落确定失败。 | 原85368实际拒绝 host 为 Z、provider 已退出，证明 `drop(child)` 缺口；15850当前单 libtest focused exact实际1/0/0。4303公开 Unsupported 两次不同意图及各自重放全到，拒绝后原5秒 ANY PID/group 退出门实际通过；不是把测试自己的 waitpid 当修复。 |
| 原生菜单 Quit，`apps/gui/src-tauri/src/menu.rs:205/548` | 用同 catalog 文案和标准 CmdQ 的显式菜单项，向现 main window 发 `close()`，复用现有 JS draft owner；不复制 draftCount、不新增 RPC/状态，也不直接 `destroy/exit`。main缺席只向现存窗口各发原有关闭请求，不伪造任意多窗草稿事务。 | 原44266有可见接纳草稿、实际 Save os13后仍退出；锁定 muda 的预定义 Quit 为 `terminate:`，不同于 JS 所保护的 close-requested。不能用旧纯 UI 绿否认原生反例。 |
| 批准后的辅助窗口清理，`apps/gui/src-tauri/src/lib.rs:5411–5436` | 原 overlay、`code:`、Settings关闭块从 main `CloseRequested` 迁到 `Destroyed`。请求不等于批准；Cancel不提前破坏辅助窗口，主窗口真正销毁后仍由既有 owner收尾。关闭集合与 `.close()` 操作原样，未扩通用 close 或重构 shutdown。 | Workspace88179从 Settings CmdQ进入未保存对话框，Don't quit保留主编辑器和Appearance两窗；最终批准退出时全部原归属清理通过。其他原生终止入口、任意辅助窗口聚合保存不在此证明内。 |
| 清洁文件条栏，`apps/gui/src/views/WorkspacePane.tsx:1147–1170` | 仅退掉清洁文件的常驻 reload动作和过期注释；dirty Save、confirmed Saved、菜单重读与 watcher原 owner保持。没有为按钮再造正文状态或刷新服务。 | 91690当前874 App实见清洁文件无常驻重读、More仍可重读。该变化不解决或掩盖44266；后者由独立原生 Quit修复处理。 |

## 当前来源与实际业务闭环

三 normal producer actual37775结果 `2ed3567fd1e1686eae86c435589571d8de50f1183758bf9513bb8afa39c00159`：当前2340源文件、五阶段退出0，三份各自 no-prior、非 test binary、本次 CargoJSON 唯一选中行及 provenance 已独审。CLI/bridge虽然字节与874相同，使用当前新 proof；GUI为 `6017b27a78505bb15d31817e0df1d904f8b8d912d9a92f3bb8bda08d0f7e1d40`。实际21839前端仅复用未变188生成文件和278 dist，完整 source delta恰两处原生 Rust叶；不是冒新生成或重新通过 fulltypes。新 GUI OUT完整来源、默认 custom-protocol/devtools、嵌入 bundle和严格清理均已分别尾核。

Root实际 CUA操作、reviewer只读核对记录及磁盘，不声称reviewer亲自观察界面：

| 真实文件 owner | 实际业务记录 | 独立磁盘与进程尾核 |
| --- | --- | --- |
| Workspace＋Settings，88179 | 接纳 FAILED-DRAFT、Save os13；从Settings CmdQ同PID仍活，Don't quit保草稿及两窗。恢复权限后q显式保存，r后立即CmdQ。业务SHA `82527a38deaac4e0ce80fad24a8b2157ef7004718a1a1c5ad4e741c0903bcbf6`。 | 全文含 `R2-QUIT-FAILED-DRAFTqr`，文件SHA `230d68bcb71459fbc0006012bd597367ae61b1a265c6763e3e5dddf3b8dbbaa8`。原父目录inode36351611/0755恢复；native `09fb2b2e…c006` 严格净。 |
| SessionContent/FilePage，81735 | 经 readonly history→front-tab toggle→Session内容侧文件入口；同样完成normal Save、可见拒绝、CmdQ阻止、Cancel和最后键保存。业务SHA `0a9f632a5598538a3966d0a7cf4b15a917bb806c86a9ea405587e6bf6b565b2b`。 | 全文含 `R2-SESSION-FAILED-DRAFTqr`，SHA `d05206896fe85633b7050080fbf9539dad36975a8fa03ed021a05b80ea0c38d2`；原父inode36351614/0755恢复，native `b238e228…a091` 严格净。 |
| MachineFiles/FilePage，33349 | 经Machines→Mac-mini→Browse files→实际filesystem EditorMachine；不是Workspace同入口换标签。失败、阻止退出、Cancel、恢复及最后键保存分别到达。业务SHA `77bd186e4ab40aaebb1b13596eb18d6c5ac1532aa3610842dfd5c7ff9a4e01ee`。 | 全文含 `R2-MACHINE-FAILED-DRAFTqr`，SHA `76474a8364320a0773007250c37a04c17d30b47f18498020c6e403f526bbea6b`；原父inode36351617/0755恢复，native `96dfa7fd…2ca3` 严格净。 |

三份原生结果与业务/file收据逐SHA互绑当前源、App6017；全部正常退出0、无 forced/timeout/interruption，owner/group/footprint清空、TMP absent、环境与来源稳定、日志SHA一致。退出后未再调用CUA以免对dead绑定App触发ensurelaunch。原70472输入未证＋晚AX重启的失败，与44266明确业务红分列保存；本次三入口业务绿没有改报那些原件。

## 错误、重复与注释的增量判断

- 子进程 `wait` 的 ECHILD只解释既有特定 `pid_running` 已reap同ghost，不能生成 provider Exit receipt；未知 provider、出生事实、恢复代次仍由原Attempt owner决定。30秒未知 spawn结果不会被新线程机制转成可重复执行的确定失败。
- 本修复没有在菜单、Rust和React各实现一套草稿判断。菜单只传关闭意图，JS现 owner处理 flush/Cancel，Rust仅在实际Destroyed后收辅助窗口。原先“请求就收尾”的生命周期错误已通过移动同一原块消除，不靠新增状态机补偿。
- 注释现在说明 `Child` 的真实持有者、确定失败与未知差别、关闭请求与实际销毁差别。没有引入泛用框架或为了去重扩产品接口。现存大文件成本仍是原报告的维护边界，不能因此再开壳层重构。

## 保留发现与未完成边界

17f6报告的 F1 ready诊断文件生命周期、F2 generic close仍按既定NEXT保留。本次线程wait不修 F1，不把 exact `.why` 留存当产品诊断已修；新恢复保证也不外推所有通用 close。其他未变的领域结论和34项标准原样。

公开 Load、Resume、Shell、Unsupported各有本地受控peer/PTY的限定业务证据，详见逐片卡；本报告不把它们改成真实模型或所有OS分支通过。双home Offline前两次真实失败94947/27250及后继计划仍独立，完整 return、host-death/provider-still-live、physical reboot未由当前报告证明。IME、selection、真人输入手感、WK所有paint场景、手机/Windows等平台未知保留。跨Session阅读点回到tail的现行为与bookmark NEXT，不能被同实例Range或最后键验收代替。

来源：`platform/l03-native-quit-source-1`、`history/session-recovery/s3-ghost-child-owner-source-1`、`ghost-child-native-source-1/actual-node-1`、`public-unsupported-current-source-2/actual-restore-1`、`production-current-quit-prepared-1`；三入口记录在 `platform/mac-current-acceptance-data-1/runtime/currentquit-editor-*-business-green.json`。均相对于 `/Volumes/disk0/Khor/evidence/round2/`。原状态由Root中央总账管理；本审查只新增本文，没有产品改写、重跑矩阵或扩大完成声明。
