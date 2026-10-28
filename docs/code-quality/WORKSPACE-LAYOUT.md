# 工作区布局讨论与 Lab 交付计划

日期：2026-10-28。状态：**第二版独立 Lab 已完成制作和定向验证 / 待用户验收 / 产品接入进入下一轮**。
调查基线：main `e7c335c8dd81de091bd12b92cd704a63f25d03f3`；实际发布构建 `320a48d95d18222aaa4216702408caa9eaffca51`。
所有者：主智能体。总账见 [ROUND-2](ROUND-2.md)，继承 [PLAN](PLAN.md) 与 [ENGINEERING](ENGINEERING.md)。

## 1. 首版用户要求及授权顺序（布局修正以第二节为准）

用户要求先把涉及多页的工作区布局讨论清楚，**而后制作独立 Lab，由用户验收，将验收后的 Lab 作为参考一并交给下一轮**。这不是立即修改产品 main 或 ship 的授权。

- 没有选中工作区时，右侧不显示内容；选中后左侧显示该工作区的文件树。
- 用户已确认：左侧返回列表时取消当前选择，右侧清空；保留各工作区标签、位置和未保存草稿，重新进入恢复。用户补充：已选工作区但无内容标签时显示该工作区“最近打开”，替代此前正文留空建议。
- 右侧最上方就是与 session 一致的标签页栏，不再有固定工作区标题。
- 标签栏右侧仅保留 session、更多、VS Code 三个图标，取消工作区图标；顶部常驻刷新希望取消。
- 左侧文件树标题第一行工作区名字，第二行 branch（如有）；无 branch 不留无意义空行。
- commit log 入口保留在文件树第一行。点击后在右侧完整标签页显示提交列表，点击提交后在该标签内显示完整详情，不再弹窗。
- 用户已回答提交范围为 **“当前分支完整历史”**，按需加载；不默认扩到整个仓库所有分支，也不能只展示最近 200 条。
- 修复跳转文件下拉两根滚动条、diff 文件标题横条透明；用户要求确认当前运行的是最新 GUI。

## 2. 第二版用户修正与当前行为

2026-10-28，第一版之后用户明确要求恢复正式文件树/list 与搜索、新建入口；以下决定替代第一版“左侧名字/branch 两行、右侧列表与详情内部返回”的布局方案：

1. 工作区选择仍为两侧共同状态。返回工作区列表清空右侧；各工作区标签、草稿、阅读位置保留。选中但无标签时显示该工作区最近打开。
2. 左侧身份第一行名字在左、branch 在右；第二行机器：路径。复用正式 SideList/PaneBar/SideFind/FileTree/Hits；保留搜索、新建文件/目录、文件图标、Git 标记、缩进与上下文菜单。工作区列表保留搜索、添加、置顶和删除。
3. 文件树上方用原有紧凑 HEAD 摘要行，点击只将左栏原位切换为提交日志，不改变右侧当前内容或草稿。日志可搜索、分批加载；搜索旁的文件夹按钮返回文件树，各自保留位置。范围仍为当前分支完整历史。
4. 点击具体提交，打开或更新右侧同一枚稳定提交标签；目标为不可变 commit hash，标签标题为提交摘要。左侧日志持续可选，右侧没有第二份列表，也没有提交详情返回按钮。不因连续选择提交堆叠标签。
5. 内容标签已经表达标题，文件正文不再重复路径标题，提交正文不再重复提交标题或独立页的返回栏。作者、时间、hash、说明、统计与完整 diff 保留。右顶仅 Tabs，操作按“会话、VS Code、…”排列。
6. 窄屏保留“查看文件树/查看提交记录”的全局左右栏切换入口，这是手机导航；不增加提交页内部返回按钮。当前标签文字保持可见，多标签横向浏览。
7. 跳转文件菜单单一滚动所有者、sticky 文件标题稳定背景继续保留；文件与提交的阅读位置、草稿跨标签/工作区保留。

用户授权的是独立 Lab 迭代。第二版已经制作并完成定向检查，尚待视觉与体感验收；不等于产品合并、发布或其它下一轮专项开工。

## 3. 当前运行 GUI 的核验结果

2026-10-28 本次只读核验：

- 主 GUI PID `79598`，启动于本地时间 13:59:41，与最近一次 ship 的 `mac-install-result.json` 一致。
- `lsof` 记录该进程加载 `/Volumes/disk0/Khor/caches/rust/khor-target/release/khor-gui`，inode `36154987`；用户启动路径 `.../khor/target/release/khor-gui` 指向同一文件。
- 文件 SHA256 `a6ab09abead752200911d6534eb852fc09d1b7e705b5d922dd0855430b7f9af2`，与发布收据完全相同，文件时间早于该进程启动。
- 当前实际 5467 服务响应 `X-Khor-UI: khor-ui-assets:/assets/index-BjJjcA2m.js`，与发布的 `ui-receipt.json` 相同。未认证响应 403 是本机 Web 入口认证结果，不是用户此前网站连接失败。
- 发布构建 `320a48d9` 到 main `e7c335c8` 的 apps/crates/Cargo 产品差异为空。当前运行的是最近一次 ship 的完整 GUI/前端，这两个视觉问题不能用打开旧版解释。
- 原收据目录：`/Volumes/disk0/Khor/evidence/releases/20261028-code-quality/`。未重启 App、杀进程或改用户会话。

## 4. 两个 commit 显示问题与既有修复的关系

截图：`/Users/zgh/Desktop/截屏2026-10-28 17.01.07.png`、`/Users/zgh/Desktop/截屏2026-10-28 17.01.21.png`。

| 问题 | 当前明确机制 | 历史与验收缺口 |
| --- | --- | --- |
| W3 / R2-021：跳转菜单双滚动条 | `CommitPage.tsx:162` 自包 MenuViewport，但 `dropdown-menu.tsx:63` 已自动包一层；两层均可滚动且有高度限制 | `749c4925` 先增加共用包装，`f96a8087` 又在调用方嵌套；`dc843dd2` 只修跳转后焦点/回顶。现有少量文件样例及滚动焦点测试没有覆盖双滚动条。 |
| W4 / R2-022：diff 文件标题透明 | `CommitPage.tsx:199` sticky 条用 bg-card；card 引用 background，`tokens.css:3208` 在透明窗口下将 background 置 transparent。当前用户配置 frost/70，与截图吻合 | `aa9f50ea` 修共享列表标题，`4fc98b76` 修补全菜单玻璃，不是该 commit sticky 条；通用 menu-material 门也不覆盖它。 |

这些相关提交都在发布构建祖先中，上述四个源文件在发布构建与当前 main 相同；没有找到两个确切入口已修复后又漏发布的证据。是源码机制与截图相互印证，本次没有操作用户 GUI 独立复现。

根治约束：菜单由共用包装层唯一管理滚动，调用方只给菜单项；sticky 材质不依赖随窗口变透明的 card，不用提高 z-index 代替背景。Lab 与正式验收覆盖长文件列表、短视口、键鼠选择、已有跳转/焦点行为、深浅色和实色/半透明/毛玻璃下的长 diff。

## 5. 正式实现不能被 Lab 掩盖的能力缺口

| 项目 | 现状与下一轮要求 |
| --- | --- |
| W1 / R2-019：布局与选择归属 | `App.tsx:3144` 未选工作区仍显示 WorkspacesNow；openWs 与 wsTree 分开，左侧返回仅 side.set(false)，右侧还保留内容。名称/branch 在 R0，右侧已有共用 Tabs。需明确选择/展示状态，保护标签草稿，不靠卸载丢状态。 |
| W2 / R2-020：提交历史内容标签 | 入口 `WorkspacePane.tsx:1996` 打开 HistorySheet，commitShown 分支 `:2042` 替换整个 Tabs；应纳入内容标签模型。gui-core git.rs:236 与 node git.rs:662 上限 200，node git.rs:843 只走 first parent，前端 more 不能继续翻。需让当前分支完整可达历史可分页浏览，不能将“还有更多”提示称作完整历史。 |
| W5 / R2-023：取消刷新后的自动更新 | 文件树有 dirsWait，未覆盖目录靠 focus/60s 兜底；Git 状态/branch/HEAD 只随 gitAt 重读，外部 commit/切 branch 不保证即时触发；FileView 当前文件内容没有持续监听。可以取消顶部常驻刷新，但必须补齐外部保存、目录增删、切分支、新提交、断线恢复后的自动更新；有草稿不得被外部内容覆盖。 |

新历史分页与实时更新属于下一轮正式实现，Lab 用受控数据表达行为和状态，不假装完成了后端能力。与 SESSION-CONTINUITY 的长历史问题遵守相同的有界加载原则，但不把两个实现专项绑成无限前置。

## 6. Lab 交付清单与恢复

- 布局对齐后，在隔离分支/实例制作 Lab，覆盖未选工作区、文件标签、提交列表、提交详情四个主要状态；包括 workspace 返回/重选、并列文件标签、左侧文件树/日志切换位置、长列表和长 diff。
- 宽屏为此次主要布局；窄屏检查可滚动、控件不遮挡、返回含义一致，不擅自增设另一套移动导航。
- 明确记录真实组件与模拟数据的边界，用户验收前不宣称后端分页/实时更新完成。用户选定后保存 Lab 入口、代码提交、截图、交互决定、已验/未验清单及下一轮接入说明，一并交给下一轮。
- 沿用设计系统、Clean Code、独立审查、单一共享文件 owner 和 native 构建唯一槽；不污染 main，不替换正式 App，不动真实 session。
- 压缩恢复先读 STATE → 本文用户决定/讨论建议 → ROUND-2 / PLAN，再核真实授权、分支与 diff。本次已确定当前分支完整历史；其它三项下一轮专项仍按各自文档，不用本次 Lab 流程授权扩大产品实现或发布。

## 7. 第一版 Lab 交接（历史，第二版以下节为准）

- 工作副本：`/Volumes/disk0/Khor/worktrees/current/khor-workspace-layout-lab`；分支 `lab/workspace-layout-20261028`；提交 `b4b4f6c7dc5f642d146fba15ea8c7d7fd5cbed46`。
- 基线：`e7c335c8dd81de091bd12b92cd704a63f25d03f3`；main 与正式运行 App 未修改/重启。
- 验收入口：<http://localhost:23832/lab/workspace-layout.html>；Vite 服务 PID 初始记录 8330，真实存活状态以证据目录 `server.json` 和端口检查为准，不按旧 PID 杀进程。
- 启动与边界：Lab 内 `apps/gui/lab/workspace-layout.md`。应用原有 Tabs、Glass、menu、图标、token、CommitPage/CodeMirror；文件编辑用样例 textarea，不假装正式编辑器功能齐全。数据/会话栏模拟，VS Code 不启动外部程序，所有 API 被隔离拦截。
- 多智能体分工：root 负责布局/状态/集成；fixtures 负责样例和非作者 UI 验证；commit 负责两处已知视觉 bug 的 red→green；review 独立审查并复现位置/搜索问题。共享文件由 root 统一修改；无 native 构建。
- 自动检查：Lab TypeScript 通过；桌面/390×844 主流程、240 条完整样例批次、独特短 hash 通过。菜单唯一滚动、跳转/焦点保留、透明窗口下深浅色 sticky 材质、无运行错误共 5 项通过；原 red 仍保存。文件/提交跨标签和工作区的阅读位置与草稿保留、恢复后继续手动滚动、搜索回顶 3 组独立检查通过；初始失败按真实观察留存，不伪造旧自动脚本。
- 证据根：`/Volumes/disk0/Khor/evidence/labs/workspace-layout-20261028`；`ui-check/report.json`、`commit-check/red.json`/`green.json`、`position-check/green.json`、截图与 Vite 日志。最后组合 UI 检查在位置修正后的最终源码通过，截图等待真实 diff 编辑器完成渲染。headless Chrome 结果不冒真实原生材质或物理 iPhone 验收。
- 待用户：整体布局、三图标位置、标签命名/详情返回、窄屏内容切换的体感验收。用户未验收前不标任务完成；不自动启动下一轮。
- 下轮接入：W1 选择与内容状态统一，W2 真正完整历史分页，W3/W4 采用此分支的最小修复，W5 自动更新与草稿保护。Lab 的 DOM 滚动适配仅作参考，正式视图应显式提供阅读位置接口。其它 S/C/B 专项范围保持原文。
## 8. 第二版 Lab 交接

- 源码：同一 Lab 分支，提交 `b20eb7384d0222f46de4f13d284e78c07078b8a9`；main `e7c335c8` 未改动，正式 App 未重启。入口仍为 <http://localhost:23832/lab/workspace-layout.html>，已刷新原 Chrome 验收标签。服务继续运行供用户查看。
- root 负责布局、状态、真实组件接入；commit 智能体负责 CommitPage 的 `presentation="tab"`，默认独立页行为保留；fixtures 负责内存 API；review 独立验证布局与阅读位置。无 native 构建。
- 按职责拆分 Lab root、sidebar、content、model、position 与 server。文件树与搜索复用真实组件；编辑器仍为样例 textarea，右侧会话栏为样例，VS Code 不启动外部程序，模拟 API 不读取或写入真实文件系统。
- 支持文件名/内容搜索、命中行定位、新建文件/目录、工作区搜索/添加、删除后按同路径重新添加并保留内存身份；拒绝路径逃逸和跨工作区移动。mock 不代表正式后端行为已修改。
- 最终源码组合验证：TypeScript exit 0；`check-workspace-layout.mjs` 5 组、`check-workspace-commit.mjs --tab` 7 组、`check-workspace-position.mjs` 3 组通过，无浏览器运行错误。原独立页宽窄模式仍保留标题/导航；标签模式移除重复标题/导航。390px 三标签场景当前标签标题可读，整个页面无横向溢出。
- 新证据：`/Volumes/disk0/Khor/evidence/labs/workspace-layout-20261028/v2/`，其中 `ui-check/report.json`、`commit-check/green.json`、`position-check/green.json`、`fixture-check/report.json` 和 `fixture-check/readd-and-cleanup.json`。fixture 原报告记录修正前的同路径重加限制，后续独立记录验证修正，未改写旧证据。
- 最终截图：`preview/final-tree-file.png`、`preview/final-log-commit.png`、`preview/final-light-log.png`，窄屏多标签 `preview/mobile-three-tabs.png`。原第一版收据保留。
- 用户整体视觉/体感验收待进行。下一轮正式接入仍需完整当前分支历史分页、自动文件/Git更新、正式编辑器的显式位置接口及真实设备验收；本次不扩展 S/C/B 范围。
