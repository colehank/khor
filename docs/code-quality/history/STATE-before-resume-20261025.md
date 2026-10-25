# 当前状态

## 当前暂停与内盘目录整理（2026-10-25，最新）

用户再次明确暂停治理，随后同意整理 working 下的工作副本。本次只迁移目录与维护恢复入口，没有继续功能实现、编译或原生验收。下方“恢复执行”是此前历史记录。

内盘主仓仍是 `/Users/zgh/Desktop/working/khor`（main 8704a503 clean）。另外 7 个内盘工作副本迁至 `/Users/zgh/Developer/Khor/worktrees`：4 个 recovery 放 recovery/，3 个旧功能分支放 history/。迁移前已备份当前未提交内容，迁移后校验 HEAD、暂存区、差异和内容一致。外盘目录布局不变。

最新映射与备份：`/Users/zgh/khor-backups/working-20261025`；内盘目录入口：`/Users/zgh/Developer/Khor/README.md`。旧的内盘路径不留兼容链接，working 下只保留 khor。恢复治理时使用新路径核对现场，不能依据旧日志中的绝对路径重新创建旧目录。

原生构建按暂停要求终止，未安装/启动隔离 App；其任务卡 Q-010-native.md 保存未完成边界。其他任务的源码、未提交内容和已交付提交都保留。


## 恢复执行：2026-10-25

用户在讨论外置硬盘散热后明确“没事 继续吧. 先回答, 大概还有多久完成目标”。已先答完整范围约 5–10 个工作日的粗估，然后恢复治理。该明确指令替代此前暂停；范围仍为 PLAN P1–P5，main 合并/发布边界不变。

当前现场：main 8704a503 clean；外盘集成 9b175b42 clean；内盘 recovery 75e1afc3 保留 Q021b 未提交修改。外盘原 Q025/Q030/Q031 以及内盘 Q023b/Q020b 均已备份，待按实际 diff 串行收尾。旧快照不得当作当前完成证据。

当前分工：root 收尾 Q021b 并负责集成/总账；native_acceptance 优先准备首批真正原生 App 验收、只读核对 Q025；store_owner 续作内盘 Q023b；agent_redaction 续作外盘 Q030，Q031 不混入提交。重 Cargo/native 只有一个构建槽，须 root 明确授予；主缓存保留。外盘风扇故障且无法读取温度，用户允许继续，减少并发重 I/O。

首要交付：先让首批达到独立可合并的审查与平台验收状态，不把它长期绑在全面治理后面；其余修复分批独立提交。当前没有重启过期 Lab 服务，不引用旧 PID 作为活跃证据。


## 2026-10-25 目录整理后的当前状态

用户已暂停代码治理，并单独授权“先把这整理好”。目前仅完成磁盘目录整理，没有恢复功能开发、合并 main 或发布。

- 外盘已恢复挂载；396 个原根目录项目迁入 `/Volumes/disk0/Khor`。
- 权威协调目录是本文件所在目录；具体路径与存储规则见 [STORAGE.md](STORAGE.md)。
- 迁移前已将所有登记工作副本的未提交差异、未跟踪文件及本地 AGENTS 备份到内盘 `/Users/zgh/khor-backups/disk0-20261025`，并校验归档内容。
- 迁移后 56 个工作副本的 HEAD、暂存区和未提交状态校验一致；外盘 48 个工作副本的 Git 登记已更新，修复 44 个软链接。
- 内盘恢复工作仍位于 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-recovery` 及另外三个 recovery 工作副本。其新增修改尚未与外盘原任务整合，不能依据下面的旧状态表认定已完成。
- 外盘 Q025、Q030/Q031 和中央任务文档的未提交文件已保留。下方记录是中断前执行快照；恢复治理时先读内盘 `recovery/RESUME.md` 和本节，再核对两边 diff。
- 下方旧 PID、Lab 地址和“正在运行”说明不代表当前进程存活。整理前未发现引用这些外盘目录的进程，当前没有重启 Lab 或构建。


更新：2026-10-25 · 全目标持续执行；第二批三路修复与验证治理进行中

## 目标与授权

用户在规划后指示“开始做吧”，首批后再次明确“做完此目标后再结束”。目标覆盖PLAN的P1—P5，不以首批或单个批次代替完整完成。按全仓审计、分批修复、独立审查与Lab验收执行；主智能体集成并维护本总账。当前没有合并main或发布动作。首轮范围和协作/压缩恢复规则读 [PLAN](PLAN.md)，风险与后续优先级读 [AUDIT](AUDIT.md)。

## 权威路径与版本

- main基线：/Users/zgh/Desktop/working/khor，`8704a503a27b36ad58f922690de50e766f8bd485`，tracked工作树保持clean。
- 权威协调目录：/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan/docs/code-quality，plan/code-quality分支。其他worktree副本是快照；恢复先读这里。
- 集成实现：/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1，integrate/code-quality-wave1。
- 源码提交：`4d3c52f9` 自动保存，`dc843dd2` 聊天滚动/提交焦点/回归/Lab。后续文档提交不改变上述已验证源码。
- Q020原实现worktree：/Volumes/disk0/Khor/worktrees/current/khor-quality-save-road，fix/quality-save-road，原提交d9865696已cherry-pick；工作树保留。
- 主仓AGENTS.md是gitignored本地入口，已补本协调目录指针。不要因main无改动而重新实现或重复发布。

## 任务状态

| 任务 | 状态 | 记录 |
|---|---|---|
| Q-001A/B/C 审计 | 首轮结构巡检与重点抽样已交付 | 报告、逐文件清单在tasks/；不代表全仓逐行深审 |
| Q-010 聊天位置 | integrated，原生验收待补 | 第四版保护布局/caret变化，真实输入优先；Q-010.md及独立review |
| Q-011 提交跳转 | integrated，原生验收待补 | 菜单关闭交目标焦点，Escape维持；Q-011.md |
| Q-012 漏检与回归 | 首批浏览器回归完成 | Q-012.md，原始红/候选绿同脚本指纹 |
| Q-020 自动保存 | integrated | 取消epoch、按编辑线队列、内容重复head；24回归和独立审查 |
| Q-013 Lab | integrated | 320/390触摸与1200布局通过；跨iframe键盘失焦也有正式回归 |

## 当前第二批分工

第二批基线 `861652db7e42c2cbb17270946695a417c09a6b69`，所有任务保留第一批修复。

| 任务 | 所有者 / 工作目录 | 当前工作 |
|---|---|---|
| Q-021 已知锁定状态的错误恢复 | root集成；作者gui_audit | 882e3c50已集成，17项hook回归；首次未知降级策略等待用户选择 |
| Q-022 身份原子首建 | root集成；作者backend_audit | e89da2e6，真实Cargo 9通过与64子进程，独立审查完成 |
| Q-023 卸载范围与 dry-run | root接管复审补丁；原作者validation_audit | b241953b后补同名异指纹拒绝；68/68，补丁待非作者复核 |
| Q-024 验证入口可信性 | root；集成worktree | f3fb5254/b3142d33，netlab失败退出码、静态/编译分层、CI；待非作者审查 |
| Q-025 原生命令阻塞与顺序 | gui_audit；/Volumes/disk0/Khor/worktrees/current/khor-quality-native-io | 实际socket写入不得运行在UI线程，保序/隔离/清理 |

子任务拥有自己的任务卡和授权源码，root拥有STATE/PLAN/AUDIT/package.json。各自提交后轮换非作者审查再串行集成。磁盘约4.6GB可用，Cargo/native先协调，不同时构建。

## 已有证据

证据根：/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence。manifest.json记录源码、锁文件、环境和检查边界。

- baseline-final：原始commit三种选择均回顶部、聊天键盘第一步失去tail、draft增高尾差40；Escape控制通过。
- scroll-lifecycle-final：Chrome155.0.8059.85、WebKit26.5，各13场景，共26/26；未捕获pageerror=0，源文件指纹开始/结束一致；与baseline只有两生产文件不同。
- gen通过（独立Cargo缓存由暖缓存APFS clone，使用完仅清理本任务缓存）；完整tsc、默认test、gates登记与build通过。build有既有大chunk警告，gen有vendor/node/ts-rs警告。
- save-road集成24/24，非作者补3边界通过；editor-history Chrome/WebKit各8场景通过。
- ai-elements-final2四组通过；首次final运行mobile工具展开超时，保留日志。已补失败截图/状态诊断，不宣称已确定偶发原因。
- Q001C曾误跑mobile，静态10失败，后续cargo被它自行中断；不是通过。具体基线漂移与进程处理在审计报告。
- 图谱在基线虽HEAD匹配仍不完整；集成worktree初始图为空，commit钩子才增量建立修改文件。risk0或test gap数量不作正确性依据。

## Lab与保留进程

- 服务：node PID79697，exec75507，cwd /Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1/apps/gui，port23820。
- 桌面： http://127.0.0.1:23820/lab/quality.html
- 当前同网入口： http://172.23.250.213:23820/lab/quality.html （本机HTTP200已核验，地址变化后重查）
- 全部虚构会话/commit，同源API，无真实模型/用户文件操作。键盘模拟原位改变visualViewport，手机全屏入口用于真实键盘操作。保留Lab服务供用户检查。
- 各测试临时服务已由所有者清理；WebKit浏览器在/Volumes/disk0/Khor/caches/dependencies/khor-playwright。iPhone17Pro模拟器E52DFE43-E03C-4FBE-99D4-C8CEDEE87ED9由本轮启动并已停止，Safari只访问隔离Lab。

## 尚未完成与下一步

1. Khor原生App/物理iPhone及辅助技术未验本提交。模拟器Safari基础键盘末尾可见，但硬件输入后切软件键盘出现外壳位移异常，未分清Safari与App，不能作为原生通过；单列原生验收。
2. Lab反馈如需调整，在此分支继续，不自动合并/发布。
3. 后续按AUDIT优先级：锁定刷新失败、identity并发首建、卸载dry-run/purge；再修验证退出码/移动端普查漂移、A2A context、错误脱敏和生命周期。
4. Q020独立审查补出的“用我的覆盖”历史digest假冲突，父/子都有；另立任务处理fresh pin来源，不让所有asked绕过冲突保护。
5. 大入口拆分、统一格式/注释、协议恢复和构建可复现性均在后续批次，不能把第一轮Lab交付写作全面重构完成。


## 最新运行与资源（第二批）

- root创建专用Cargo缓存 /Volumes/disk0/Khor/caches/rust/khor-quality-target：只APFS clone debug必要产物，跳过旧incremental/rcgu.o；主缓存不写。该目录仍保留供后续验证，不重复创建。
- backend_audit正在独占Cargo槽验证Q030；其监测器低于1100MiB只中止本任务PGID，具体进程以任务卡/ps核验。
- Q024首次完整interaction：17个hook与4个AI Elements通过；scroll报告24/26后工具运行异常终止，无finished，不能算全通过。相关测试进程已核对结束，保留Q024/scroll-final和interaction.log，等待Cargo后单独重跑。无证据断言异常原因。
- Q024非作者审查已完成卡，发现并已修尾换行quote丢失与Linux不应使用macOS Option-Tab；最终提交/验证待补。
- Q030/Q031在 /Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol（backend_audit），任务卡分别维护；root未编辑相关源码。
