# 当前状态

更新：2026-10-25 · 首批浏览器/Lab版本已交付，原生验收与下一批治理待继续

## 目标与授权

用户在规划后指示“开始做吧”。按全仓审计、分批修复、独立审查与Lab验收执行；主智能体集成并维护本总账。当前没有合并main或发布动作。首轮范围和协作/压缩恢复规则读 [PLAN](PLAN.md)，风险与后续优先级读 [AUDIT](AUDIT.md)。

## 权威路径与版本

- main基线：/Users/zgh/Desktop/working/khor，`8704a503a27b36ad58f922690de50e766f8bd485`，tracked工作树保持clean。
- 权威协调目录：/Volumes/disk0/khor-code-quality-plan/docs/code-quality，plan/code-quality分支。其他worktree副本是快照；恢复先读这里。
- 集成实现：/Volumes/disk0/khor-code-quality-wave1，integrate/code-quality-wave1。
- 源码提交：`4d3c52f9` 自动保存，`dc843dd2` 聊天滚动/提交焦点/回归/Lab。后续文档提交不改变上述已验证源码。
- Q020原实现worktree：/Volumes/disk0/khor-quality-save-road，fix/quality-save-road，原提交d9865696已cherry-pick；工作树保留。
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

全部子智能体完成本轮交接。下一次派发仍使用PLAN的独立worktree、文件所有权、非作者审查规则，先核对现场再修改。

## 已有证据

证据根：/Volumes/disk0/khor-code-quality-evidence。manifest.json记录源码、锁文件、环境和检查边界。

- baseline-final：原始commit三种选择均回顶部、聊天键盘第一步失去tail、draft增高尾差40；Escape控制通过。
- scroll-lifecycle-final：Chrome155.0.8059.85、WebKit26.5，各13场景，共26/26；未捕获pageerror=0，源文件指纹开始/结束一致；与baseline只有两生产文件不同。
- gen通过（独立Cargo缓存由暖缓存APFS clone，使用完仅清理本任务缓存）；完整tsc、默认test、gates登记与build通过。build有既有大chunk警告，gen有vendor/node/ts-rs警告。
- save-road集成24/24，非作者补3边界通过；editor-history Chrome/WebKit各8场景通过。
- ai-elements-final2四组通过；首次final运行mobile工具展开超时，保留日志。已补失败截图/状态诊断，不宣称已确定偶发原因。
- Q001C曾误跑mobile，静态10失败，后续cargo被它自行中断；不是通过。具体基线漂移与进程处理在审计报告。
- 图谱在基线虽HEAD匹配仍不完整；集成worktree初始图为空，commit钩子才增量建立修改文件。risk0或test gap数量不作正确性依据。

## Lab与保留进程

- 服务：node PID79697，exec75507，cwd /Volumes/disk0/khor-code-quality-wave1/apps/gui，port23820。
- 桌面： http://127.0.0.1:23820/lab/quality.html
- 当前同网入口： http://172.23.250.213:23820/lab/quality.html （本机HTTP200已核验，地址变化后重查）
- 全部虚构会话/commit，同源API，无真实模型/用户文件操作。键盘模拟原位改变visualViewport，手机全屏入口用于真实键盘操作。保留Lab服务供用户检查。
- 各测试临时服务已由所有者清理；WebKit浏览器在/Volumes/disk0/khor-playwright。iPhone17Pro模拟器E52DFE43-E03C-4FBE-99D4-C8CEDEE87ED9由本轮启动，Safari只访问隔离Lab。

## 尚未完成与下一步

1. Khor原生App/物理iPhone及辅助技术未验本提交。模拟器Safari基础键盘末尾可见，但硬件输入后切软件键盘出现外壳位移异常，未分清Safari与App，不能作为原生通过；单列原生验收。
2. Lab反馈如需调整，在此分支继续，不自动合并/发布。
3. 后续按AUDIT优先级：锁定刷新失败、identity并发首建、卸载dry-run/purge；再修验证退出码/移动端普查漂移、A2A context、错误脱敏和生命周期。
4. Q020独立审查补出的“用我的覆盖”历史digest假冲突，父/子都有；另立任务处理fresh pin来源，不让所有asked绕过冲突保护。
5. 大入口拆分、统一格式/注释、协议恢复和构建可复现性均在后续批次，不能把第一轮Lab交付写作全面重构完成。
