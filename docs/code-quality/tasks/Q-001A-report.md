# Q-001A GUI / 原生交互初审报告

日期：2026-10-25。负责人：gui_audit。基线：`8704a503a27b36ad58f922690de50e766f8bd485`。源码目录：`/Users/zgh/Desktop/working/khor`，只读、HEAD 与任务一致、没有未提交差异。

## 结论和范围

首轮完成模块清单与风险抽样，未做全文件深审。范围内有 525 个 Git 跟踪文件：src 408、core 38、src-tauri 72、native-files 7；包括平台资源、项目配置和已有测试。逐文件登记见 [Q-001A-inventory.md](Q-001A-inventory.md)。风险排序依据行为影响和可触发时序，文件长度只用来定位维护边界。

发现 3 项 P1、2 项 P2 行为问题和 2 类治理风险。GUI-A01/A02 在真实保存模块中运行复现；GUI-A03 以真实 hook 和最小 hook host 复现；GUI-A04 的资源 ledger 时序已运行复现，真实原生句柄释放仍需 App 检查；GUI-A05 由前后端控制流确认，实际请求速率未测量。没有修改业务源码、运行构建或发布。用户报告的键盘跟随、commit 跳转由 Q-010/Q-011 独立复现，本报告不抢先把候选机制认作根因。

## 发现

### GUI-A01 · P1 · 内容回到历史值后，自动保存制造虚假冲突

- 证据：`src/lib/save-road.ts:146–155,213–215`；同一编辑线接收新 digest 而保留 by：`src/views/FileView.tsx:720–724`；内容摘要契约：`core/src/files.rs:767–775`。
- 触发：同一编辑线依次成功保存 A→B→C→A，再保存 D。A/B/C 表示内容与其确定性摘要，而非递增版本号。
- 机制：`moved[by,was]=digest` 构成环；`forward` 固定上限 1000 步，在三节点环中把实际头 A 错转为 B，后端正确拒绝，GUI 显示 clash 并停止自动保存。草稿尚在，后续保存受阻。
- 验证：真实模块在内存转译后 mock compare-and-write；前三次 OK，第四次收到 was=B、disk=A，D 被拒绝。脚本和日志见下文。
- 建议：用编辑线的单调请求代次及已确认头处理推进；不能通过增大遍历上限修复。保留不同编辑线互相冲突的边界。
- 验证缺口：A→B→A、A→B→C→A、长链、排队、卸载重挂、两个编辑线并发，以及真 App 编辑后 undo/redo 再继续保存。
- Q-001C 交叉证据：`scripts/check-face.mjs:31479–31482` 的 digestOf 使用递增序号，`write_text` 每次返回新摘要，无法形成真实内容摘要环；`check-editor-history.mjs:50,56,66–85` 的写入 fixture 不校验 was，现有 undo 测试也不检查后续 D 写入。参见 Q-001C。

### GUI-A02 · P1 · 放弃编辑后，旧失败请求重新安排重试并写回文本

- 证据：`src/lib/save-road.ts:235–281,300–326`。UI 关闭入口在 `src/views/WorkspacePane.tsx:1070–1075,1090–1103`。
- 触发：写入正在飞行时 discard/forget；请求随后失败、文件仍为原内容，后续重试成功。实际 UI 入口之一：已有保存失败、自动重试正在飞行时选择关闭并“不保存”；旧 trouble 在 start 时未清除，关闭流程可直接显示确认。
- 机制：forget 只清除当下的 failed/retry；旧请求 catch 重新填 failed，finally 安排 2000 ms 重试。重试直接 start，绕过 write 对 hushed 的检查。
- 后果：磁盘最终写入用户已经放弃的文本。这里不否认已提交写入可能结束；问题是已知失败后仍发出新写入，违反本模块“nothing is retried after it”声明。
- 验证：真实模块 probe 中 discard 后第一次受控失败，仍产生 2000 ms timer；执行 timer 后第二次 save 将 disk 从 original 改为 discarded edit。
- 建议：取消代次或任务所有权必须跨越 catch/finally；forget/discard/discardAll 以后旧请求不能创建新重试。
- 验证缺口：飞行成功/失败/冲突、重试等待、再次打开、丢弃后新编辑与旧回调交错，以及 UI “不保存”完整路径。

### GUI-A03 · P1 · 配置重读失败可直接清除已锁定状态

- 证据：`src/hooks/use-lock.ts:72–102,159–162`；App 只按 locked 分支展示锁屏：`src/App.tsx:3758–3784`。
- 触发：已成功读到 facts.on=true 且 locked=true；另一窗口的 LOCK_TOLD 或显式 reread 使 fetchLock 再读，此次失败。
- 机制：错误分支无条件 setLocked(false)，即使已经有“启用锁”的既知事实。原有 facts 仍为 on=true。
- 后果：UI 可从锁屏进入主界面而没有成功 PIN/系统验证。这是界面锁状态问题，不宣称服务端授权被绕过。
- 验证：真实 useLock 模块配最小 useState/useEffect host，先读成功，再让 reread 拒绝；输出从 {locked:true,on:true} 变为 {locked:false,on:true}。尚未做浏览器/原生集成复现。
- 建议：将“未知配置”“已知启用”“认证结果”“读取失败”区分；已知启用且锁定时读取错误应保持锁定并允许重试。首次启动未知配置的产品降级策略应单列决定，不能与刷新失败共用解锁分支。
- 验证缺口：锁屏跨窗口改设置时断网/IPC 失败、配置从开到关、成功 PIN 后刷新失败，以及窗口恢复时序。

### GUI-A04 · P2 · 关闭标签后的迟到网页打开只隐藏，未及时释放

- 证据：`src/views/BrowserPane.tsx:274–313` 先 holdBorrow/holdPage，再在 stopped 分支仅 hide 并 return；`src/views/held.ts:39–40,64–73,132–135,145–162` 的 hold 不按已登记 wanted 集合重新清理。
- 清理触发：`src/views/PagesKind.tsx:173–181` 只在机器/标签 ID 集合改变时 sweep；`src/views/SessionContent.tsx:187–188` 同样按集合依赖。onOpened 的补扫不会执行，因为 stopped 分支已经返回。
- 触发：open_page 拨号中关闭标签，集合先清理；原生打开随后成功。前端记录新资源，但已没有新的 ID 集合变动。
- 后果：已关闭标签的原生页和远端借网资源可一直隐藏保留，直到另一轮集合变动/进程退出。这里没有声称页面持续可见。
- 验证：真实 held 模块依照上述顺序执行，close/drop 调用为空；再显式 sweep 才出现 close(opening-tab)、drop(borrow-session)。原生代码也不在 open 后自动关闭：`src-tauri/src/lib.rs:1642–1727,2347–2362,2850–2891`。
- 建议：资源取得后立即按最新所有者集合 reconcile；关闭标签与仅切离标签必须保留不同语义，同时避免迟到 cleanup 关闭新 owner 的页面。
- 验证缺口：延迟借网后关闭、快速关闭重开、两个 owner、多个 tab 共用出口、释放失败；真 App 量 native page/借网句柄数量。

### GUI-A05 · P2 · 聊天长轮询在立即失败或 ended 回复时以 0 ms 重试

- 证据：`src/views/ChatView.tsx:887–900,1024–1033`；`core/src/chat.rs:570–582` 在 gone=true 时不等待，直接返回。
- 触发：已打开的可见原生聊天，chatWait 立即拒绝，或 attachment 已 gone 但组件尚未因上层状态更新切换。
- 机制：catch 不记录失败，gone 仅 setGone；finally 对 pushed 始终 setTimeout(tick,0)，把“成功等待过”与“立即失败/结束”混为一谈。
- 后果：短时间高频 IPC/请求及无用 React 更新；实际频率受 runtime、IPC、上层卸载时点影响，本轮没有测 CPU 或吞吐。
- 同仓对照：`src/views/TerminalPane.tsx:2095–2123` 已对 ended 停止，对 failed 指数退避，且注释记录相同机制曾造成 CPU 问题。
- 建议：把 tick 的结果区分为 live/ended/failed；失败有有界退避，结束停止，重连/显式刷新可重启。共享调度应在这两个功能的差异明确后再提取。
- 验证缺口：立即失败、gone 无新帧、正常等待、后台恢复、重连成功后退避复位；断言请求数/时间窗而非仅“会话能打开”。

### GUI-R01 · P2 设计风险 · 大入口横跨多个状态所有者

- `App.tsx` 同时持有会话轮询与已读（884–998）、内容/文件路由（2117–2422）、workspace 选择与屏幕骨架（3059 起）、锁屏分支（3758 起），主 App 函数从 361 到文件末。
- `TerminalPane.tsx` 的会话 effect 横跨 attach/replay/poll、输入法、手势、主题、滚动与销毁；单一 cleanup（2842–2923）需要手工配对数十个资源。
- `src-tauri/src/lib.rs` 同时为命令适配、原生网页、窗口、iOS 生命周期与后台任务提供实现；`core/src/api.rs::answer` 是超过千行的分派表。
- 可观察成本已有例证：TerminalPane 修过轮询退避而 ChatView 未携带同一约束（GUI-A05）；资源取得/展示/集合释放分散后出现 GUI-A04。
- 建议下一批按稳定所有权边界拆出 attachment lifecycle、native page registry、navigation coordinator；先建立行为测试与资源台账，再搬代码。仅按行数拆文件没有验收价值。

### GUI-R02 · P3 可维护性 · 历史解释覆盖了已经失效的事实

- `save-road.ts:148` 声称 digest 链不可能有环，GUI-A01 已反证。
- `use-lock.ts:119–122` 声称“一次性 timer”，实际 136–138 是 15 秒 interval；`core/src/lib.rs:3–6` 和 `core/src/api.rs:2–7` 仍将 LAN/web caller 称为未来项，而 `src/api.ts:327–345` 已有 web face 路径。
- 新导航代码如 `use-content-navigation.ts:84–260` 与 `session-nav/state.ts:4–107` 把多个转换压成单行，难以逐项审查状态前置条件；旧文件则有很长的墓碑/轮次叙述夹在实时流程中。
- 建议随对应任务修订事实错误；保留“平台限制、所有权、为什么不能这样做”，将已失效过程迁入决策记录。格式迁移单独提交，不与行为修复混在一起。

## 模块覆盖与未知项

“抽样已审”表示列出的代表流程已读，不代表同模块全部源文件安全。“结构登记”只说明完成模块位置、职责与下一步的登记。逐文件表比本表更保守。

| 模块 | 深度 / 代表流程 | 发现、已有保护及缺口 |
| --- | --- | --- |
| App / 主壳 / 会话列表 | 抽样已审；App 轮询、当前选择、workspace key、锁屏出口 | sessionRequest/sessionApplied 抑制旧响应，设备 asking 避免重叠；GUI-A03/R01；新建/导入/关闭所有分支未逐一跑 |
| session/navigation | 抽样已审；use-session-tabs、use-session-content、use-content-navigation、session-nav/state | write queue、epoch、版本与 alias 保护明确；复杂 optimistic/持久状态需要多操作时序回归，未判定具体新缺陷 |
| chat / composer / receipts | 抽样已审；ChatView poll/open/leave，MachineChat read receipt/send，use-chat-scroll | GUI-A05；键盘与 reading anchor 交由 Q-010；权限卡片与附件 UI 仅结构登记 |
| terminal / tmux / renderer | 抽样已审；TerminalPane attach/replay/poll/refit/cleanup | generation/seq、单飞、ended/failed 策略与对称 cleanup 明确；GUI-R01；所有 xterm 补丁、GPU fallback、tmux full history 未深审 |
| workspace / Git | 抽样已审；WorkspacePane 搜索、git status/log、key、草稿关闭、use-repository-history | live/asked 序号防旧响应；GUI-A02；commit diff 跳转 Q-011，checkout/restore 行为未深审 |
| files / editor / autosave | 抽样已审并模块复现；FilesPane、FileView 保存接收、save-road、MachineFiles | GUI-A01/A02；FilesPane path-only cache 候选未报告缺陷，因为常见父层按 workspace/machine remount；需更多 caller 核对 |
| browser / retained page | 抽样已审并 ledger 复现；BrowserPane、held、native-page-open、PagesKind、SessionContent | GUI-A04；展示 revision/owner/队列、overlay yield 防护明确；native URL 导航全部异常分支未深审 |
| common UI / focus / viewport | 抽样已审；dropdown-menu、dialog、viewport、use-chat-scroll | Dialog 监听可见视口并保存 opener；Q-010/Q-011 作本轮验证；viewport delayed timer 清理、所有 overlay 交叉组合未运行 |
| settings / lock / cross-window | 抽样已审并 hook probe；use-settings、use-lock、App 锁出口 | GUI-A03；settings alive 和 storage notification 成对清理；theme/font/palette 参数验证仅结构登记 |
| devices / mesh / add / invite | 抽样已审 load poll；MachineCard::useLoad；其余结构登记 | 单飞 self-scheduling + stopped 保护；AddMachine、pairing、网络操作交给后续专门流程审查，未造无证据风险 |
| quota / usage / day | 抽样已审 QuotaLanding poll；其他结构登记 | QuotaLanding 1134–1151 仅 live 而无请求序号，潜在乱序覆盖，尚未注入复现，不列确认缺陷；chart/day-core 算法未深审 |
| GUI core API / rows / types | 抽样已审；api::answer 参数读取、lib 模块入口与 SessionRow、tabs shaping | bool/optional 参数显式拒绝坏形状；GUI-R01/R02；生成器与 command_tables 由 Q-001C 核对 |
| GUI core chat / terminal | 抽样已审 chat_wait 边界，terminal 仅结构和 GUI 调用契约 | GUI-A05；Rust attachment/rejoin 深层交由 Q-001B 补证，未声称全量所有权审查 |
| GUI core files / handoff | 抽样已审；file_handoff 完整主流程、files digest、tabs 边界 | handoff ID 不可换绑、canonical app-owned 路径、cancel、缓存软上限明确；文件大小/类型全组合未实跑 |
| GUI core domain adapters | 结构登记；acs/add/agents/code/console/console_complete/git/import/launch/miss/qr/recents/rejoin/subscriptions/workspaces | 全部纳入 inventory；不把 wrapper 存在视为验证，后续按业务任务与 Q-001B 对齐 |
| Tauri command / desktop | 抽样已审；lib open_page/close_page、native menu lease；其他窗口/编辑菜单等结构登记 | GUI-A04/R01；owner 只允许 main/settings，lease 校验过期动作；OS 事件与主线程调用未实机运行 |
| iOS native | 抽样已审；lib front_worker/resume 与 phone_files security-scope/worker 配对；phone_page/quick-look/share 结构登记 | lifecycle batching 与 2s 等待边界、scope 成对释放明确；未验证真机后台、系统弹层、取消时 file provider 行为 |
| Android native-files | 已读小模块主流程；Kotlin ACTION_VIEW + Rust plugin 结构 | canonical app cache/files 限定，read-only URI grant，错误返回；未运行 Android chooser/未知 MIME/应用退场 |
| assets/styles/vendor/platform projects | 结构登记；设计入口已读；生成及第三方从边界审查 | 本轮不改视觉。未逐像素验收，未深审 vendored math 或原生项目生成文件；工具链交 Q-001C |

## 工具与证据可信度

- 按 AGENTS 先调用 get_minimal_context，再 query_graph/semantic_search；缺失部分才读文件/rg。仓库不存在 .codegraph。
- 图报告 build SHA 与 HEAD 相同，但 Git 跟踪的 452 个代码扩展文件中只有 302 个出现在本次完整 file 搜索结果；150 个未覆盖，包括 save-road、use-chat-scroll、CommitPage、多个代码编辑器支持文件。此数字是本次索引查询覆盖率，不能据此推断任何模块未使用。
- 图的 tests_for 对 file_handoff/menu_overlay 也返回 0，但源码明明存在内嵌 Rust tests。因此“0”不是无测试结论。测试覆盖由 Q-001C 的源文件核查为准。
- 部分图节点行号与当前源文本有偏移，报告使用 nl 得到的基线源文件行号。
- 证据目录：`/Volumes/disk0/khor-code-quality-evidence/gui-audit`。
- `save-road-probe.cjs/.log`：A01/A02，运行真实模块、模拟文件 API，无真实文件写入。
- `ownership-probe.cjs/.log`：A03/A04，真实 hook/ledger 的算法验证，无 DOM/原生进程模拟。源码 SHA-256 在日志内。
- 本轮没有测试通过率、构建成功或真 App 验收结论；所有 native、真键盘与完整 UI 验证仍是后续任务。

## 建议后续顺序与交接

1. Q-010/Q-011 先按既定范围修复并独立审查；本审计不改其实现。
2. GUI-A01/A02 作为独立 autosave 生命周期任务，先把 probe 转为内容摘要正确、可控请求时序的回归。
3. GUI-A03 独立锁状态修复；刷新失败与首次配置未知策略分开，已有锁不能被网络错误清除。
4. GUI-A04/A05 以资源数量和请求频率为验收指标，再决定是否提取共享 coordinator。
5. 最后处理 R01/R02 的结构/注释整理，以修改已覆盖的边界为先，不做无验证的大规模搬迁。

交接状态：文档与证据已落盘，源码无改动，无启动的服务/需要保留进程。gui_audit 可接后续独立审查。任务完成核定由 root 进行。
