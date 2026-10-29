# W 工作区正式接入

状态：working；W的选择/草稿/位置、正式UI、完整历史服务与分页UI已分片交付，watch/自动更新和正式门适配/最终原生验收未完成。Owner round2_input；唯一集成root、独审round2_reviewer。main/ship未授权。

当前恢复入口：watch实现新树 `/Volumes/disk0/Khor/worktrees/current/khor-round2-watch`，分支 `fix/round2-workspace-watch`，base `6bd7023b2961f5e63ae0e4ece8c92babf1df888c`（Root E deferred两片已含），从tracked clean开始；旧git树固定74028700继续clean冻结，不带5Lab进此树。最近产品仅 `31d1336d8eeeb7a71c18ed131a5b7544ba2f95fd`（7分页consumer文件）；独立5文件Lab在740，不进产品。Node/core服务leaf c6已Root集成7bfc5164，A4共享入口/catalog/gen已固定，E完整4链与standalone末片均精确依赖取入。最新证据 `evidence/round2/workspace/history-ui/receipt.json`，全部ownbrowser关、23839已owned停止/旧23837未动。原workspace作者树与其0f/2d UI历史Lab继续冻结，详后文。

当前写权：此卡由本作者维护；Root已核准watch §7进入leaf：node/watch.rs/link.rs/git.rs（私有Git plan）、必要node/ipc.rs/proto.rs/serve分派；core/files.rs DTO/转发。S作者 round2_history 当前唯一持有 shared api.rs/Tauri/export/gen，watch如新增/扩dirs_wait接口先交Root分配一个writer；不争native/App/browser槽、不自行gen。现分页consumer固定后不再扩其他功能。FileView accepted-owner/E生命周期由Root维护，未经授权不改。下一实现应从Root新的固定基线开独立作者分支/树，不把5Lab依赖带进产品。

最初W选择片段基线b84187b7、首作者workspace树、其App/WorkspacePane/state唯一权和163真实A2产物证明，保留在下述历史检查点。首次G转W时G实现封口（仅最终原生材质待验），W之后始终只消费其band/sticky合同。

## 进入与交付顺序

1. 接手先核 STATE、实际集成 HEAD、A/G 共享文件和 native 槽。设计决定读 [WORKSPACE-LAYOUT](../WORKSPACE-LAYOUT.md) 第2、10–13节；最终参考 c501029490bf5997a271e252941582911d421a8f，只提取已验能力，不整支 cherry-pick Lab。设计系统与本轮 Clean Code/独审约束继承 [ROUND-2-EXECUTION](../ROUND-2-EXECUTION.md)。
2. 首片段先统一当前选中工作区与内容展示：取消选择右侧留空，保留标签/草稿/位置，重选恢复；已选无标签显示最近。完成真实宽窄四态、返回/重选与脏文件保留后固定提交。
3. 文件与 commit 使用同一预览槽：单击预览、双击固定、编辑自动固定，提交身份为完整 hash。左栏树/日志就地切换、正式列表布局和操作入口以参考为准。先记录状态归属，再改 UI；保留正式搜索/新建/上下文菜单。
4. 当前分支完整可达 Git 历史采用真实有界分页。先确定稳定游标、源变化/断线错误、批次/响应/缓存上限，再实现；覆盖合并历史、超过200条、分页并发与HEAD改变。Git 历史与聊天历史各自实现，不制造通用分页框架。
5. 实时更新接 E 的读取/组词/草稿接纳契约，覆盖外部保存/目录增删/branch/HEAD/重连。刷新入口取消以自动更新真实可用为前提。菜单只留一个滚动 owner；G 的 band/sticky 属性直接消费。

## 共享边界

派发时 root 指定唯一文件 owner；预计入口 WorkspacePane、App 工作区选择接线、FileTree、FileHistory/CommitPage、工作区 UI 模型、GUI/Node Git API。此卡不预授权修改全部这些路径。A2 当前持有 Rust/GUI API/生成器与多处 Session UI；native、生成器只由唯一槽串行。G 当前持有 app.css/scroll-edge/CommitPage 等，转交需固定提交/实际diff清点。

E已集成的 file-refresh 以编辑代次/请求序列/composition 拒迟到回包；merge 接纳保护、草稿 ledger owner 计数保留。FileView/CodeEditor先读 R2-E 对应契约；观察频率增加不等于可覆盖编辑缓冲。WorkspacePane当前草稿owner延续，不因隐藏/未选而卸载或清计数。真实Mac IME/退出验收仍由整合实例完成。

## 证据与收口

正式视觉验收按最终参考逐项核：无重复顶部标题；右上[会话 VS Code][更多]双玻璃、标签与文件标题无分割/间隔；commit标题正文、默认全部展开及手动状态按标签保留；SideList正式槽位、树/日志文字同起点、branch与摘要合行、头像1em与两段等间距；下载仅右键。文件编辑参考Lab使用textarea/内存API，正式CodeEditor阅读位置、composition/草稿接纳及编辑自动固定必须重新验真实组件。

evidence/round2/workspace 按片段保存源SHA、实际预算、原失败、定向检查、非作者审查及浏览器宽窄结果；每片段集成后只验受影响组合。独立commit标签阅读位置、收起状态、长文件跳转/唯一菜单滚动、透明主题、脏文件在切工作区/自动更新后不丢需有真实组件证据。原生/手机单列，不用Lab当正式交付。

新增独立问题入后续轮次，当前四条缺口继续修。每次交接/compact前更新本卡固定ref/diff、进程/端口、未完成和下一明确动作，源码/新增文件同步内盘恢复集。


## 首片段：选择与保留 owner（已固定，待集成）

图谱入口已核：新树 code-review graph 为空、无 `.codegraph`，之后才定点源码 fallback，未新建索引。实际卸载点是 App 的 WorkspacePane identity key；旧 tabs/recent 只保 storage，draft Map、EditorHistory、active/face 属于 pane 实例；旧 back 不清 openWs，空选择仍画 WorkspacesNow。

已实施局部 typed WorkspaceMemory。App registry 用 JSON `[device,path]` → memory，实际只挂当前 Pane。memory 保留 tabs/active/preview/recent/drafts/face 等字段和原每文件 EditorHistorySlot；没有隐藏 Pane、EditorState、DOM、watch、timer 或第二份正文。局部 useSyncExternalStore 字段订阅不把每次按键发到 App。draft ledger reporter 跟随 model 存续，并同步订阅 owner 更新；取消选择不能清仍在的未保存草稿。原 FileTree 缓存继续复用，App 的阅读位置 key 同样带 device/path。

实际数据/位置红例均保留原 artifact：

- `last-edit-red`：真实 CM dispatch 后同批切工作区丢末尾17字；FileView 在现 onText 接纳后用同一 publishDraft 派生逻辑同步报告 owner，`last-edit-green` 完整3428字恢复，by/was 与 E revision/composition 不变。原 effect 继续发布读/存/merge 接纳状态，未读时不清 owner 草稿。实际键入回声回归 `real-typing-check` 绿。
- `handoff-count-observation`：同批 model1 / Quit 所读 registry0；reporter 改同步 model 订阅后 `handoff-count-green` 为1/1。它只证明本帧数据链，不证明 macOS 最后一键立即 Quit。
- `reading-red`：CM scroll700→0；同 key+同 text 的原 history 接纳分支保存 immutable scrollSnapshot，并经 EditorViewConfig.scrollTo 恢复，green700→700、selection 不变。独立 fresh goLine 优先。
- `preview-reading-red`：真实 Markdown scroll800→0；`use-kept-scroll` 仅抽出原45/60帧+height/anchor恢复核心，FileView 实际 ready 的 preview node ref 复用此 core，同每文件 slot 只加 JSON `[machine,path]` key/top/height 数据。宽 green800→800。窄 FilePage 自有 face 在重挂后回 source 的第二红例 `narrow-preview-red` 已用可选成对 faceControl 接同 memory.face，原800/height15530精确恢复；独立调用继续默认 local。没有声称窄 revision/compare 全部保留，那些随后标签实体片段再核。

root 分次授予的必要共享窄路径：FileView onText/publishDraft 与 preview ready ref；CodeEditor 现 accepted history 分支 scrollSnapshot/scrollTo；lib/code/history.ts 纯 slot；use-kept-scroll.ts 共用原恢复核心与 ref 入口；FilePage.tsx optional faceControl。公共 FileView API、CM external-text/revision/composition、全局退出判定不改。check-file-merge.mjs 仅 fixture 执行实际 publishDraft 接线，原9条业务断言完整保留。

## 首片段隔离与验证（进行中）

唯一 W Vite：23837，PID76388，exec46226，`apps/gui/lab/workspace-state.vite.ts`，cache `/Volumes/disk0/Khor/caches/vite/round2-workspace`。入口 `http://127.0.0.1:23837/lab/workspace-state.html`。真实 WorkspacePane/Region/SideSeats/FileView/CodeMirror/model；两个机器同 `/fixture/project`，第一文件320行、Markdown180节；所有 API 本地 fixture 终止，未知400记账、write_text409拒绝，不代理真实用户后端。`?standalone=1` 仅验证默认 FilePage 分支。

`retention-check.json` 实际8组已绿（1200/390各4组）：空选择无 Pane、所选空文件态、同路径两机器 tree 位置隔离；两个 dirty 文档各自保留、取消选择 ledger 仍2、Undo/Redo/selection；卸载后已过 in-flight 窗口不再启动 API、⌘B 无旧 handler；fresh line30 优先旧700；inactive 外部文件更新不覆盖 dirty。errors/unknown 空。`kept-scroll-check.log` 执行实际私有 core 的5项受控检查，保45帧宽限、60帧总预算、迟到高度、同步 band settle、单实例取消/anchor清理。`model-final.log` 5项 owner/实际 App back callback 检查，不能冒整壳 UI。

E 受影响回归：refresh8、merge9、draft-count9 均绿。第一次 merge fixture 未注入新真实 publishDraft 导致 ReferenceError 的收据已单列，修 fixture 后9绿，不冒业务红。误选现有 editor-overwrite 门的13项已跑完并清其临时服务，作为额外证据，不再重复。

正式 tsc 和 Lab tsc 都已明确进程 exit0（final-types.log / lab-final-types.log）；窄 source700→700也绿。默认独立 FilePage/最后clean tab最近打开/宽分支不回退3项实际绿（default-recent-check）；FilePage保原functional turn语义，optional控制setter复用Dispatch<SetStateAction<Face>>。生产 `45f6126149c7ffad73b4fd17279a0c5aaa383e82` 与独立Lab `60bcea9a65df80662453dcac515d909b7348d168` 已固定，tracked干净；非作者初审无阻断，最终SHA核后交root集成。实际 App 整壳侧栏返回→空白→重选、native/WK、真实 Mac IME/退出与第三方嵌入页内部滚动不在本 Lab 已验范围。此首 checkpoint 之后按 root 调度短插 V L04/L05，再做 W 剩余布局/标签/Git/watch，不以首片段绿关闭整 W。


固定收据：`evidence/round2/workspace/first-slice-receipt.json` 含17源SHA、全部原失败与当前绿报告、精确检查exit。生产只cherry-pick45f61261，60bcea9a是隔离验收Lab，不自动进入生产。提交hook正常增量更新图谱（原进入为空）；无手工index或Cargo。23837/PID76388保持原实例冻结供复核。源码小恢复包 `/Users/zgh/Library/Caches/khor-code-quality/recovery/workspace-w1-20261029`。

## 第二前端片段：正式布局与混合内容标签（2026-10-29，实施中）

首片段已由 root 集成 3a9ae447。原作者分支 fix/round2-workspace@60bcea9a 和所有原 Lab 证据保留；23837/PID76388 与 C/G 的23835/PID92646 已核 birth/cwd/config 后只终止自有服务器，端口和原进程均已退出，见各 evidence 的 lab-stopped-20261029.json。未操作用户 App/Chrome。

新作者分支 fix/round2-workspace-ui，从 integrate/round2@2a5a416b2e08679ed576118ca54a7823a2481eb8 开始，切换前 tracked 干净。164 个真实生成文件逐 SHA 复制自冻结 Root，依赖仍复用缓存；ui-setup.json 保留精确来源。不跑 Cargo/gen/原生 App、不启动新服务器。Root 另有 native build 和 Markdown 小补丁 owner，此树不改 renderer/API/Rust。

Root 已授：App 工作区 wrapper；WorkspacePane/workspace-memory 与必要 workspace 小模块；CommitPage/CommitDiff tab 展示、受控折叠；FileTree/WorkspaceHistory；icons 仅用户历史 SVG；app.css 仅工作区。另明确授 TabStrip 的可选 onPin 和按钮双击；PaneBar inline（保 side 语义但跳过本组件 portal）与 PaneAction.afterClose（仅菜单实际关闭归焦边界执行已选择的新建动作，卸载清 pending）。默认其他调用不变。

当前实现方向：

- `WorkspaceTabs` 一个 content owner 统一 file/diff/commit 的 items/active/preview。提交以完整 hash 区分，不用截短 ID。混合关闭仍选相邻目标；既有 pinned 不降回 preview；文件编辑沿 E 同步 onDraft 发布后固定。原 drafts/history 和 ledger owner 保留，不挂隐藏 editor/watch。
- 右侧共同 Tabs，移除工作区重复 Region 标题。移动端同一可读 tabs，沿真实 FileView/FileVerbs/readFirst/handoff，不复制编辑缓冲。原文件 face/revision 控制仍由同 owner 管。
- commit 的展开 Set/阅读 slot 纯数据按完整 hash 保留，默认全部文件展开；workspace 模式 full unified 内容不默认折叠未变化行；文件头切换可折叠。跳转先展开，不移动到另一个提交；移除 DropdownMenuContent 内第二个 MenuViewport。独立 CommitPage 默认 page/back 行为继续保留。
- 正式列表 SideList 不改；选中工作区标题/返回/搜索/新建使用既有 PaneBar/SideSeat；机器头像:完整路径、branch+commit 摘要同一行；左侧 log 就地替换树。真实旧 Git cap 仍如实提示，尚未冒充完整分页。刷新留在更多菜单，待下一后端片段证明自动更新后再删。
- 历史 icon 直接采用用户历史2.svg 原 path/currentColor；树 hover download 移除，仅右键动作保留；文件字母和目录状态点复用同 trailing slot。

当前仅受控模型/接线证据：ui-tabs.log 6 项、ui-memory.log 5 项；第三 types 已实际 exit0。首两次 types 机械替换/未清导入失败留原日志，未当业务红。真实有 SeatsContext 的 DOM/菜单新建焦点、宽窄混合标签/dirty/Undo/阅读状态尚未验；独审正读当前 diff。新 Lab 尚未启动，等待 native/内存窗口协调后在原唯一23837 做定向验证。Git 完整分页/实时 watch 后端仍留本包后续片段，未收口 W。

### W UI 实际检查点（续）

已实际启动唯一23837，PID98386（Thu Oct29 17:18:42 2026）、exec3152、cwd作者树 apps/gui，配置 lab/workspace-state.vite.ts。原服务器不复活；这是新本地 fixture 实例，unknown400、write_text409、make_file/make_dir仅fixture响应，不改真实文件。真实 SideSeatsContext 将首行标题放侧栏 head、metadata 留 list body 上方，避免拿无 portal 的旧参考 Lab 冒正式组合。所有本轮 own browser 均已关闭，Root正占唯一native槽；下一浏览器批等Root交还资源窗口。

`ui-memory.log` 初轮其实4/5：迁移脚本遗留一处旧 tabs key，最后同shell又跑 tabs 命令导致外层exit只反映最后检查；作者逐日志核出后已向Root更正。原日志保留，修 fixture 为 content owner 后 `ui-memory-green.log` 明确5/5、独立命令exit0。tabs原6项未重复。各次types皆以原handle确认终态，`ui-types-toolbar.log` 与 `ui-types-final-candidate.log` exit0。

真实宽首批 `ui-browser/workspace-ui-check.json`：有SeatsContext的新建菜单关闭归焦→name输入、共享file/commit preview、源行双击pin和真实键入pin、两个同38前缀fullhash commit独立折叠/scroll600、dirty文件来回及真实Undo已绿。原 jump 项仅回调/展开存在性，尚未冒实际目标几何。first-inspect/png显示三行位置并无unknown/pageerrors；空tabs capsule已按参考隐藏。

窄首批 `workspace-narrow-failure.json/png`：新建name焦点/输入已绿；随后源行连续dblclick期望3tabs实际2。首click即转内容屏卸载log行，浏览器第二click/dblclick失去原target；FileTree同既有导航形态。Root裁定保持首tap立即打开，不新增等待双击延迟或隐藏hitbox。窄下一批采用真实可达的“打开preview→双击该tab固定→返回log开下一项”，宽源行双击合同仍保留；不把这个改测路线冒所有窄源行dblclick都已验。

Root随后授 FilePage.tsx 仅FileVerbs/useFileVerbs窄组合：FileVerbs.toolbar传leading+附加actions，复用原菜单/IO，workspace文件态恰[会话/VSCode][…]；face切换、刷新、关闭其他并入同一个more，原share/history/locate/背景等保留。默认FilePage调用不变。原初稿文件态多一枚more在截图前审出，最终文件态截图待补；commit截图不能代替。mixed close-others已遍历所有typed targets，shut读memory最新draft集合，原dirty flush/失败确认不改。PaneBar pending卸载清理使用layout cleanup。Workspace实际首行沿现WINDOW_DRAG与WindowControls保拖窗口/Windows系统按钮，不加第二标题。

下载复核发现旧FileTree只有hover按钮、原右键objectActions/remove并无take；仅删hover会丢能力。现同FileTree内将take接原pullFile Promise并放右键，沿原HereHosts/remote/non-dir能力门，RowMenu处理拒绝/重试；不再占文件M/目录dot trailing slot。没有真实下载执行。末批将验证两玻璃、菜单原动作、唯一viewport/宽窄jump几何、两commit slots、保dirty的mixed close-others及重选恢复。独审两轮当前无源码阻断，尚未固定提交/收口UI，完整Git分页/watch及最终App/WK验收仍后续。

### 最后组合增量（尚未收口）

`ui-browser/workspace-narrow-tabs-check.json` 已真实7组绿/exit0，ownbrowser已关闭：窄即时单击后双击内容tab固定、dirty Undo；窄last-file jump 222=222，回宽98=98且唯一菜单viewport；目录dot/文件M trailing right334=334；右键take存在而hover按钮不存在（未下载）；文件态两玻璃及原history/refresh菜单；close-others实际拒写409后dirty仍在、Keep与重选保留。此报告真实但当时 Lab 尚缺 FullScreenProvider，不能外推实际App全屏宿主分支。

提交前已核App真实外壳Provider/Slots：Settings/SettingsReread、HereHosts、SideSeats、FullScreen、Tooltip，以及wide RegionLead。前两项为偏好/能力；此Lab真实复用了SideSeats/useSideSeats，但最初没复用FullScreenProvider，导致NarrowHead在Lab有host而实际App useFullScreen后卸载sideHead、host归空。这是源码组合缺口，已改narrow返回直接在共同Tabs.head，tree/sessions保现NarrowHead；文件屏用既有safe-top，避免第二头。新Lab补真实FullScreenProvider与App同样的claim→隐藏head边界，无代理用户App。

补测 `ui-browser/fullscreen-failure.json/png` 原样保留：前2组实际绿（head被卸载后文件Back可见可点回树、collapsed空flush body的computed bottom0且无隐藏CM），第3组实际失败：commit滚动后z10 sticky文件头穿出未建立stacking context的Tabs body，拦截z3主band的Back。正常click超时30s，未force click；未报UI全绿。Root用户限时OS验收正在独立ca83实例，此作者无新browser/无App操作。

已获root明确授权最小修：workspace Tabs.bodyClass加relative/isolate，约束sticky子层而不抬z数字、不改G算法/owner/几何。去外Region还会丢wide folded RegionLead；现沿原Region语义一次消费到Tabs.head、向内Provider置null，与narrowBack互斥，不新建global入口。Lab追加可观测RegionLead probe，不复制整个App。types-stack-lead.log待原句柄79382确认；下一唯一增量只重验正常Back hit-testing/受影响窄jump及foldedLead单消费，原7组和失败不覆盖。App全壳快捷键/真实原生窗口/Native VSCode能力/真实IME仍最终集成实例验，不冒浏览器已覆盖。

已完成一次Region职责清点，避免逐个漏宿主：原外Region的首行/title由本次Tabs取代；旧PageAction/Subtitle/Aside自此不再消费；WindowControls与WINDOW_DRAG已迁同首行；wide folded RegionLead一次消费并向内清空；FullScreen仍按实际App去全局头，由Tabs.head自带返回；safe-top归新窄内容头；旧backend stale caption经App→WorkspacePane.status仍在同首行显示；原capped/忙状态在该旧工作区分支本就未启用。Settings/HereHosts/Tooltip与全局shortcut owner保现Provider，不在Lab伪造其原生能力。Lab Web面不会提供原生VSCode按钮，故两玻璃检查不冒VSCode原生服务可用；实际图标顺序源码为会话→VSCode，默认原FilePage动作不变。

`ui-types-stack-lead.log`(79382)与前FullScreen types(29893)已实际exit0。增量失败的正常Back hit仍待修后浏览器；source只加Tabs.bodyClass隔离。RegionLead probe和Provider已备。Root已授权待其用户限时OS验收结束后一个增量；本作者不操作该App、不再开浏览器，Vite唯一实例98386保留。生产/Lab皆尚未提交，不能把之前7组绿回填此组合失败为通过。

下一服务片段仅只读事实（未开写/未跑Rust）：图谱query无Git节点后才查实际源码；Node用gix而非git2。crates/node/src/git.rs 的LOG=200/WALK=20000，GitAsk::Log{path,limit}→log() 当前硬first_parent_only，repo root只是不做blob比较，仍漏merge另一父线；GUI core/src/git.rs::log 硬发limit200，GitLog只有commits/more，frontend fetchGitLog仍无cursor。完整当前分支分页必须真实处理可达merge历史、稳定HEAD/cursor与变更失效、有界响应/遍历/缓存、并发晚回包；不能只把200切slice或把more提示当分页完成。本段不改Node/lib/web/Tauri/gen；待前端checkpoint交付后由root明确服务/API/唯一native槽授权。

候选已先固定：生产 `0f5a75afcf3e91682b781080fb149d5956d7f57d`（15源/脚本），Lab `2d85a50d2f0d074bb162877c6cc7bff490ec7a7a`（5文件）独立不进生产。正常graph-only precommit（无Cargo），tracked clean。`ui-receipt.json`含逐源SHA、类型终态和各stage报告边界；尚未标最后正常Back hit为通过。Root已结束focused L05成功并把native交A4 candidate2，W等A4真终态后一个增量，不能空等后扩新专项。源码/UI冻结，后续完整Git分页/watch保持下一片。

正式旧Face盘点（只读、未改门/未运行大矩阵）：check-face.mjs约4929/6568/6829旧Region R0+tree/session pair，24390附近hover take，29993/30066桌面/手机take能力，30660文件face eye入口。用户新布局明确改变这些预期，最终W门适配应保本来的能力/拒绝/对齐判据，迁到共同Tabs/更多与右键，不能简单删除断言追绿；本候选不宣称全Face通过。新tracked脚本可直接 `node scripts/check-workspace-tabs.mjs`、`node scripts/check-workspace-memory.mjs`（不触发npm pretest/gen）；包级门登记与这些旧断言的最小适配需root单写权协调。

独审候选逐SHA已核：15生产+5Lab与git-show一致、树clean，owner/toolbar/flush/isolate未见新源码阻断；最终DOM增量未完成，尚不封UI。close-others验收精确范围是1个dirty file+2个readonly commit，真实409→Keep后文本仍在、其它readonly关掉；不能外推多dirty批量关闭完整完成。既有files-only循环shut共用单waiting/closing，多dirty可能覆盖，导致早先保存的tab仍留；不是数据丢失。此既有独立问题按本片收口约束记录后续轮次，不新增批量关闭队列、不宣称multi-dirty close-all绿。

压缩恢复下一步已经固定，无需重查源码：待root明确A4 native真终态释放后，仅执行 `/Volumes/disk0/Khor/evidence/round2/workspace/ui-browser/scripts/playwright-test-workspace-fullscreen-stack.js`，用现Playwright skill run.js；命令与脚本SHA写在ui-receipt.pending_increment。它含已授权真实FullScreen/Back、3tabs选中标题、collapsed flush、滚后normal Back hit+body isolate/窄jump以及wide RegionLead一次消费；不force、不跑原7组、不新起服务或App。唯一23837/PID98386保持原冻结实例，源为生产0f5a75af+Lab2d85a50d，tracked clean。成功/失败按新文件名保存，不覆盖fullscreen-failure原件。真实终态后关ownbrowser、交source/SHA给非作者核收据，然后只集成生产候选（Lab不cherry），完整Git服务仍下一片另授权。

## UI checkpoint 最终收口（本包服务部分仍未完成）

最后 scopedbrowser 增量 `ui-browser/fullscreen-stack-green.json` 实际6组全绿、exec91354 exit0、ownbrowser已finally关闭。源运行前后20文件SHA与生产0f5a75af/Lab2d85a50d全同、tracked clean；新报告/脚本SHA记录于ui-receipt.final_increment，原fullscreen-failure及所有旧失败不变。

已验证真实FullScreen host卸载时同Tabs Back可见可点回树；3个窄target下当前标题可读（沿c501选中tab最小宽度token，不缩成icon）；收起flush marker computed bottom0且无隐藏CM；commit滚动后Back中心hit owned=true/body isolate、正常click成功；受影响窄last-file jump head158=band158且返回当前log；wide RegionLead仅消费一枚并在滚后可点击。unknown/pageerrors空。前面7组staged行为、纯tabs6/memory5及types收据边界保持，未把Chromium结果冒WK/native/IME/原生VSCode服务。

非作者最终逐源/脚本/报告SHA与实际断言/cleanup均核，无阻断，已告root可仅cherry生产 `0f5a75afcf3e91682b781080fb149d5956d7f57d`；Lab `2d85a50d2f0d074bb162877c6cc7bff490ec7a7a` 不进生产。ui-receipt的pending旧叙述已同步完成，无源码变化、无需再测。唯一Lab仍23837/PID98386、exec3152冻结保留供Root复核；没有运行用户App/native。下一工作是Root授权后的完整当前branch分页/merge历史与实时watch、最后W门适配；多dirty批量关闭既有局限只留后轮，整个W不因本UI checkpoint绿关闭。

## Git 分页服务片段（2026-10-29，已授权实施）

Root已集成UI3f16f146、A4服务4cf7cb91，tabs6/memory5/types集成检查均0。为保持UI Lab与23837冻结，本作者新建 `/Volumes/disk0/Khor/worktrees/current/khor-round2-git`，分支 fix/round2-workspace-git，从 `4cf7cb914bccd870d5bdcb49c04d019113ec1a85` tracked clean开始。未装依赖、未复制cache或gen。新树无AGENTS/.codegraph，code-review graph入口0节点后才fallback实际Rust源码，沿用户AGENTS和同轮工程规矩。

唯一写权仅 `crates/node/src/git.rs` 与 `apps/gui/core/src/git.rs`（含各自既有cfgtest）。共享core/api.rs/core/lib.rs、Tauri/lib.rs、gen/导出由A4唯一owner，稳定接口后交其一次接线/真实gen；本作者不改。Node/link.rs::git_on已泛型转发GitAsk，无需新路由。watch下一片、refresh继续保留；不跑Cargo/native直到给出精确目标并由root授唯一槽。

已采纳一个新RepositoryLog/git_history，文件按路径git_log保持独立语义。Page={head:string|null, commits:GitCommitSummary[], next:string|null}，Summary={id,author,when_ms,subject,cut}，不把完整body塞进列表/不伪装完整GitCommit；详情仍git_commit，后续组件改用shown.commit显示完整message。默认40/max100，subject1024 UTF8 bytes、author256，整页序列化≤256KiB，UTF8边界与cut如实；条目按真实序列化字节准入，超页的下一项不跳过，next仍绑定last accepted。游标不透明字符串，fixed full head+last emitted full oid；全部parents、同时间重试顺序确定；HEAD移动保持原快照不混页，无first-parent/20k总截断。后页会重走固定head前缀，成本随深度增长，不能声称只遍历N；先无常驻cache/线程框架。对象缺失/非法cursor明确错误，不用假empty结束。

验证先fresh packed fast-import夹具：merge完整父线、>200与越旧20k边界、分页重试/无重复遗漏、HEAD移动固定snapshot、空repo/无效cursor、limit与真实序列化字节/UTF8边界。禁止逐commit spawn git/fsync制造无谓IO；生产只gix，无git进程。实际gix=0.87.1，本地primary源码已读其all-parent walk/sorting定义；将检查同时间跨页顺序，不靠记忆猜API。下层实现/测试固定后交A4精确DTO和签名，本包前端分页消费/实时更新/旧Face门适配仍后续，不把下层绿当UI完整交付。

两leaf初稿已实现，生产walk全部parents，按commit time走且显式关闭commitgraph（避免跨请求新增graph改变排序或掩盖缺失object），cursor只原HEAD和最后接纳OID。按空RepositoryLog完整JSON envelope（预留next最大同kind长度）+各summary实际序列化bytes+comma累计，不假定UTF8字节等于JSON字节。实际shallow查询若有边界明确报历史不完整，root已采纳；旧path log原实现不动。新8项cfgtest只packed fixture，尚未Cargo（A4持唯一槽）；不能称测试已过。固定旧route两真实反例源码在evidence/round2/workspace/git-history/git-baseline-tests.rs及baseline.json，未运行。获槽后可在本独有作者树临时换此固定snapshot测真实旧merge/200失败，再恢复当前实现测green；不触Root/A4源。精确DTO/core::git::history签名已直接发A4，等待其统一共享接线，不手改gen。

旧route反例已真实终态：git-history/red/result.json，Cargo101、0passed/2failed、总101.588s（夹具运行0.23s，主体是Node编译/链接），旧merge仅4/6与linear260仅200，源码前后SHA一致且TMP空。不是缺API/编译错误冒红。候选由finally精确恢复后开始同filter新9项green，exec89068/Cargo99612；预检无其他Cargo/rustc、内外盘≥5GiB，jobs1/incr0/sharedtarget、各自owned外盘TMP，未运行App/gen。新加第9项实物缺ancestor错误，不把坏repo当空完成。当前仍在实际编译测试，未称green。

独审候选暂无源码阻断，已明确两预算边界：每页重走固定head前缀，深页成本增长；20005夹具为深seek无硬截断证据，不冒完整20k跨页或常数成本。256KiB是返回JSON（Node envelope含在内）预算，gix已读commit/summary再clip，不冒巨单commit内存/IO已限。未扩cache/线程框架。

Git leaf checkpoint已固定 `c6b0c766188196a4b1875ca9599c6bd6348a6830`（仅Node/core两授权文件），tracked clean、正常graph precommit无Cargo。`git-history/receipt.json` 含最终两源SHA、源前后manifest、单命令终态、旧2红/新9绿及边界；green Cargo0/66.737s、实物fixture1.03s，TMP空，ps无cargo/rustc/git遗留。rustfmt/diff check各0，唯一native槽已明确交还Root。公开Core DTO/history mapper尚未本树编译，不把Node9冒Core验证；给A4固定ref后其唯一owner统一api/Tauri/export/TS真实gen与编译。Root先集成下层，前端分页消费/watch仍本包下一片，refresh不删，W不关闭。

## Git分页正式UI消费片段（服务leaf后，已授实施）

Root将c6集成为7bfc5164，A4持唯一shared入口/api/Tauri/catalog/gen作者权与native槽，本作者不并写。当前同Git作者树继续前端，只有WorkspaceHistory、WorkspacePane、workspace-memory/tabs、CommitPage，以及新workspace-history纯owner/check脚本获授。用户参考平面log行/日期链/preview和pin不变，采用列表末尾40条页的较新/更早轻导航，无新toolbar/title；完整历史无总页数cap，watch另片、refresh仍保留。

缓存明确：永久保page0作为固定snapshot重启锚点，其余最多7页LRU；cached序列化payload≤8*256KiB（2MiB）。不保无界cursor链/每页scroll表，只保有限8页及其next/scrollslot、当前页号、snapshot代次、一个op/retry metadata。淘汰页可从最近≤目标的仍缓存页next逐页重取，最坏由page0.next重走到任意目标，固定原HEAD不会null cursor抓新HEAD。代价随深度增长，非O(1)返回；当前页受保护，失败仍留当前页/slot。物理最多1在途page临时payload，JS对象/字符串开销不冒2MiB整个memory上限。

模型attach lease与单operation归属保证旧回包不接纳，unmount后不继续串页。重复点击不额外发请求；refresh while pending将其置stale但等此物理请求settle再发新null请求，避免靠多票并发无限排队。明确更多菜单刷新invalidate，成功才替换cache/snapshot/代次，失败留当前页可retry；gitAt还担mount/treeChanged/agent stop/status，不能拿它暗中清日志快照。当前页及阅读slot留在现WorkspaceMemory；复用useKeptScrollRef，snapshot/page/query换键仅重挂这枚log scrollroot，回当前page保位置，inactive无新IO（已发出的不能取消，丢回包且不再续页）。

搜索现PaneBar searchLabel来自WorkspacePane，无条件tree_search同时作placeholder/aria/icon label。Root采纳historyOpen时准确“搜索当前页提交”，普通树不变；本片没有全history搜索服务，不冒当前页filter已完成全分支搜索。A4负责三条精确catalog与真实producer：git_history_newer/older及git_history_search_page（避免已有H history_newer归一化撞名），本作者无硬编码/放宽words/手改gen。

纯模型9项、原tabs6/memory5分别实际exit0，日志workspace/history-ui/{history,tabs,memory}.log。第一写检查脚本命令因cwd已在apps/gui仍写apps/gui/scripts而失败，根本未建文件/未执行测试，是执行路径错误，不冒业务红；改正路径后9项真实绿。此仍只证明纯owner/接纳协议，实际React mount/页导航/菜单/回文件记位/完整body未DOM验。Core/A4生成物尚未收到，不手造types，也未称tsc通过。

### UI分页实际DOM收口（watch仍后续）

共享依赖精确取入：catalog241e81e3、APIfa3取为4d1f3615；A4 shared-2真实165产物逐SHA复制，generated-copy.json，不手改/不自gen。E初取最后两片冲突揭示缺早依赖，已abort保7UI稿；Root补全a0/725/1f/4aa精确链无冲突取为38d5/4760/394b/815，未手解E。browser后再取其standalone discard末片8c（仅无上游分支，不影响W），finalprod/Lab tsc原handle66590/54797均exit0。

唯一新Lab23839/PID16311（birth Thu Oct29 19:57:01、cwd本Git/apps/gui、exec42650）复用正式Pane/SideSeats/FullScreen/model，unknown400、write_text409、opener只记不外开；旧23837保持冻结未动。初browser首marker组过后因textbox/searchbox locator误设exit1，原run/failure留存；是harness，不改产品焦点。正式autosave在真实键入时触发自有fixtureA的409属预期拒写，不假称无写尝试/无真实OS写入。只读fixtureinspect读取实际CM全doc与ledger，后续两宽度实际各2Undo还原完整320行基线且count0；不能把marker消失当全doc。

continued/history-failure.json实际13组绿：宽8全（真实CM同task末键切workspace/Undo、当前页搜索、503保持rows/top并retry、连续280越200、13页后淘汰回访仍fixed旧HEAD、回文件/log/重选位置、fulltitle>1024与正文、held旧包/双击单请求/明确refresh新HEAD）；窄前5同绿。它最后窄重选裸scroll360→396失败保原。随后只读narrow-position与narrow-slot定点证实port/client/height/padding/band最终相同但同row实际偏移36，真实slot引用相同；setterstack显示reselect第一次ref恢复时头高仅24，然后local head facts回包长至60，锚点+36。不是可放宽的像素误差或应有header补偿。

Root授最小修在现owner保留branch字符串与真实headCommit摘要，不缓存全status对象，不改共享scroll/G、不加延时observer。原live/失败清除照旧，机器+root tuple归属沿既有memory。narrow-header-green真实360→360，同slot/key/top/height、首可用row88相对band下沿offset-4完全相同，首次setter已padding60；types-header-retained0。只有这两个产品文件改变，宽8/窄5仍按旧SHA，不冒整组已新源重跑。字段在真实外部变化/错误清空时几何仍可能变化，watch/更新提示合同留下一片，不能用缓存冒自动更新完成。

最后仅续窄未验detail与held两组，narrow-final/check.json实际2绿/exec58568 exit0：详情完整标题正文、返回当前page3；单held请求跨log卸载、明确刷新后释放旧A:520从不渲染旧page13，只有旧page12→新B0，真实MutationObserver有据，外开/unknown/pageerrors空。所有ownbrowser均finally关闭。当前每page只有一个scrollslot，改query重置该页阅读，不保每query无限位置；淘汰页可重取但不保无限scroll表，current/page0受保护。

生产分页UI独立commit `31d1336d8eeeb7a71c18ed131a5b7544ba2f95fd`（7文件）；隔离Lab独立 `740287003abe497f541fffb42588e7ea59813255`（5）不进产品。正常graph commit、tracked clean。receipt.json含逐git-show SHA、纯9/tabs6/memory5、finaltypes、各browser原源/失败/修后边界。独审尾核中。23839已按原PID/birth/cwd/port正证仅SIGTERM自有Node，PID已gone/portfree，原exec143；lab-stopped.json证明旧23837字节相同仍运行，未碰用户App。Native/browser槽均已释放。下一只读W watch最小方案，暂不写新Rust/共享API/gen（S持viewlease单owner），refresh仍留，W未收口。

### 下一watch只读事实与接口待裁定

图谱仅24文件，无dirs_wait/FileTree节点后精准fallback。FileTree现dirs_wait→core/files.rs→Node::dirs_changed：仅native远端经resident WatchOn→Watches→far notify真实事件，64个非递归目录/GATHER150ms/seq log1024；它只刷新listings，未向WorkspacePane/FileView传external。local machine分支硬unavailable；HTTP core/api.rs也硬unavailable（同步串行bridge不允许held长poll）。FileTree hidden/unmount stops续问，inflight仍可回但stopped拒收；unsupported/unreported focus+60s fallback。Watches KEEP60清理只在wait入口，没有最后consumer消失后的独立idle回收，不能按注释冒已自动结束。

WorkspacePane external coverage还缺：status/head由gitAt（自有操作/treeChanged/agent stop/回前台）驱动，不因外部commit/branch直接触发。FileView无external订阅，只readAt→again和自己save-road onLanded；again已有E dirty/composition/revision/lifetime guard。现treeChanged仅进程内khor自己动作，不能偷偷把它冒真实filesystemwatch。WsFacts有workspace递归nativewatch+5s汇聚，但只是列表facts/无当前文件路径通知、无完整coverage证明，linked worktree `.git`外部commondir也不能凭目录名猜。

已向Root/S报最小方向：扩既有watch owner/seq而不另造watch框架；workspace tree变化+当前file parent+gix实际gitdir/common/ref parent，local也进同residentWatches；真idle释放/unknown-unreported事实不伪绿。一个可取消订阅hook复用FileTree/WorkspacePane，事件只invalidate并合并gitAt/readAt，沿E拒dirty/markedtext/迟到；历史固定snapshot只提示新HEAD不自动重建。HTTP零等待读seq+client有界节流（不能held占同步bridge，也不能0wait busy loop）。失败/不覆盖保可见期fallback，直到自动更新真实验过再撤refresh。候选leaf路径node/watch.rs/link.rs/git.rs、core/files.rs/git.rs；前端FileTree/WorkspacePane/new hook。递归target/owner release若须扩dirs_wait/NodeIPC/Request::Watch/DTO，是共享接口，等待Root裁定最小schema并由S单owner接，不擅写。未做任何新watch实现/native/服务运行，现只是方案，不能当watch已开始通过。

Root已集成分页31d为0478df63，集成9/6/5/types0；正式转watch接口**设计片**（仍无新Rust/公共API写权）。耐久更窄schema/分片/预算在 `/Volumes/disk0/Khor/evidence/round2/workspace/watch/interface-plan.md`：沿同Watches推荐dirs_begin/dirs_wait/dirs_leave三动作、server lease、epoch+seq opaque cursor、有限日志/dirty burst与主动idle release，HTTP零等待+有界续问，workspace/entries scopes由后端gix解析实际Git控制路径。E composition失效待办边界Root已认可并接手，只保fresh-stat待办不排旧text；我不改FileView/file-refresh。Native预算只是待授计划，未编译/跑任何新watch代码。所有候选共享注册仍S唯一owner，leaf路径/方案等Root裁定后从新集成基线开独立实现树，不把740 Lab带过去。

Watch接口设计第二次收紧已落interface-plan.md §7：Root认可三动作方向后追问路由/初始化预算。已选wait/leave显式machine_id、begin返回stable hex；仅begin resolve_or_here，wait直接校验near resident Watches lease.pin（当前resolve不认hex），无Core全局map/自解locator/新daemon。S已收到并确认接口，无抢写。source init事务、plan generation、注册后再验轻量Git plan fingerprint（防setup间HEAD换nestedref）、4inflight/64queue、128lease/256source/4096当前watch Linuxdir/16Gitcontrol/64KiBdirty+1wake、log1024且256KiB、batch128KiB、60s主动expiry都已写清。Wsracts原watcher不并入新owner、不冒整个process FD总预算；Mac supported full要实测，partial不销W5。Root已接fresh-stat待办与在途coalesce建议；本作者metadata cohort也需要singlephysical+dirty，不可让150ms burst排多个旧gix_status。此仍设计，无产品新写/新Rust/native。

## Watch leaf正式开始（2026-10-29，Root已授）

新树 `/Volumes/disk0/Khor/worktrees/current/khor-round2-watch`，fix/round2-workspace-watch，从Root实际6bd7023b/clean创建；新图谱0节点后fallback。旧git740源/Lab/全部证据冻结，不整支迁入。当前无Cargo/native/browser/server；native验证待固定实际命令/fixture预算后Root另授。E FileView/file-refresh由Root继续单writer，本作者不改（pendingfresh-stat与coalesce交接继承）。

本作者唯一允许的原有source路径：
- crates/node/src/watch.rs：同Watches的scope/lease/shared-source/seq/budget/producer；
- crates/node/src/link.rs：本机与远机实际resident路由/既有Watch stream分派；
- crates/node/src/git.rs：私有Git watch plan，真实gitdir/common/ref，无公共GitStatus字段；
- crates/node/src/ipc.rs、crates/node/src/proto.rs：同watch lifecycle三操作及scope/event协议；
- 必要时crates/node/src/lib.rs仅既有resident/serve清理胶水（不是newdaemon/第二map）；
- apps/gui/core/src/files.rs：公开DirWatchScope/CoverageMode/Coverage/Batch/Opened与转发；
- crates/cli/tests/watch_tree.rs：现唯一Node::dirs_changed外部caller，迁到新三动作并保实际双Node watcher验证；各leaf现cfgtest可写。
后片前端已预授FileTree/WorkspacePane与单directory-hook及history提示消费，但不先切半套API。shared core/api.rs/Tauri/lib.rs/gen_bindings/src-api/catalog由S唯一owner，给固定schema/小补丁后一次接线与真实gen，不并写。其它新增source先报Root。

第一个最小source checkpoint先做Node git私有watch plan+真实FS夹具（复用已有packed Checkout，含linked-worktree/common-dir、unborn/detached/nested refs）；同watch.rs做scope/owner/seq/预算确定性核心，不先改活路由/使未接shared Core断编译。接着同合同串接provider/local/remote/IPC/proto，再与S完成Core转发/共享桥原子迁移；旧API不作为长期兼容壳保留。每片独立source收据/实际filter，不能把control-plane测试冒real native watch。

Root已单独批准新增私有 `crates/node/src/watch/owned.rs`，限定同watch模块纯lease/source-ref/init/log预算，由watch.rs唯一runtime消费；不并存第二runtime registry/服务。当前第一片尚未接新route，控制测试不得冒local/remote/provider接好。普通mod owned按标准Rust布局，无额外#[path]。

首leaf候选三源已耐久于 evidence/round2/workspace/watch/lease-candidate/source.json（含原字节快照）。Git plan四例仅gix/owned临时repo事实；Owned八例只测试纯lease/ref/init4+64/log1024且256KiB/过期函数，不冒实际deadline timer或local/remote/provider建立；原watch arithmetic一例加Access/AccessTime筛选两例，共待授Node lib15。当前runtime仍旧implicit Watches，Owned尚未挂入，不并存第二活registry；实际迁移片会删除旧隐式状态/旧接口，并沿同owner处理local与remote。计划精确filters git_watch_plan(4)、watch::owned::tests(8)、watch::tests(3)，未Cargo。已发现remote Watch stream拿普通STREAMS_AT_ONCE整段不释放的现状：后片须admit+转到有界watch owner后释放普通permit，不能只drop导致无界stream；nearIPC目前每连接独立spawn。原真实kernel只读不触发回环、主动idle、held wait控制通路仍下一actual验证片，不拿EventKind注入冒过。

Root已授唯一native首批：plan4→leases8→events3。plan实际exec3113/Cargo77509，watch/plan/{preflight,running,source-before}.json，源冻结至三命令终态。新授权私有watch/native.rs效果草稿已写但**未mod消费/未编译**，manifest包含其真实字节，不计这15项覆盖；首3源仍lease-candidate同SHA。该native草稿是同resident的物理notify handle/ref与有界raw效果，不新增lease/full覆盖真相；正式挂载前还要核同path递归/非递归watch模式升级（notify可能替换同path既有模式），不能把Target不同key误当两个独立OS watch。此处仅记录未完，不在native运行中改源。

首leaf已固定d979ffaae7fa83c414ea549560af0d741c6d9c93，仅三源。实际plan4=0/103.06s、leases8=0/1.278s、events3=0/1.217s，各source前后同/TMP空/无Cargo或rustc剩余，watch/leaf-receipt.json逐git-show SHA=实际输入=耐久candidate；独审无阻断，Root已集成。native槽已交还，未把首15当runtime证明。

Runtime接线候选已固定（尚未编译/实际kernel验收）：Node/CLI `bf11a6ea00f159916f2ef940d62bd2b792e625b0`（10源），Coreleaf `dae8b19cb7b5f9d73f46618e506f34edf472c0e2`（1源），同watch树tracked clean。新增私有watch/native.rs获Root授权，Keeper仅107一行旧doc引用watch::serve改watch模块获Root授权，on_network_fs不动。旧Machine/Dir/隐式dirs_changed/WatchOn/DirChanges已替换，未留下平行活registry/长期兼容壳。

现候选责任：Owned唯一lease/source/init/log/coverage-fence；watch.rs唯一runtime/本机与远机worker/主动deadline/shutdown；native.rs只物理canonical-path notify handle refs与64KiB dirty/1wake、Git plan/alias和Linux4096/2s有界初始树枚举（不略过ignored目录）。远机请求经同admitted stream，先占有界Subscription再释放普通32-seat，stream终止释放owner，批次重验peer。run_handoff三个receiver变Arc为存Weak<Node>，不强持Node成环。本机经同resident，不self-network、不Core全局map。输入path OS判断只在holding机器。物理rec/nonrec更换会reset旧users；不可确认unwatch的handle仍留预算内，最后全pool Drop收资源。Native pool identity避免迟到旧rx覆盖新pool observer。普通multi-thread Tokio下unwatch走block_in_place让出调度；current-thread OS调用仍同步边界，不冒任意FS故障都低延迟。

Node21准入计划已发独审，尚未运行：watch/runtime-v1/{contract.json,run.py}固定11源/两个既有guard helper SHA，复用run-node-acceptance.py的strict owned/remaining/cleanup，无持久者豁免；每命令300s、双盘5GiB、jobs1/incr0/sharedtarget、独立TMP、环境hash、全source-frame前后与实际count硬门。四filter为Owned11（replan/close已触及原规则，非重复未变首15）、native raw3、真实local runtime6、更新wire1；首compile预计100–180s、fixture≤20s，最多packed两commit+一个linkedworktree，未开App/agent/server/gen。真实idle任务以40ms种入deadline，60s默认数学仍纯规则证据，不冒60s墙钟实测；初始close race未强制late interleave，和Owned确定性late case分开。kernel支持/只读无反馈/last handle等仍待实证，不提前称native fully-covered。

既有watch_tree已源码迁三动作并加38远机streams（超过普通32 RPC seats）、两owner及held wait期间begin/leave/普通ls，保原remount/idle成本断言。其khor-cli→khor-web→gui-core依赖真实shared API，必须等history单writer接Core五DTO/三命令并服务端强制HTTP wait0后，才与Core DTO合同、真实gen第二批编译执行；不能stub绕过。S已获精确最终DirWatchScope/DirWatchCoverageMode/Coverage/Batch/Opened与签名，尚未本片改shared API/Tauri/exports/catalog/gen。frontend订阅/cohort/固定snapshot提示仍后续，refresh仍在，整个W未完成。

独审对bf11未运行候选发现初始化fence真实竞态：running/queued replan intent被吞；已有pool observer还可能在新plan入表前drain掉目录事件。Root进一步明确manual/Linux每层必须先watch再枚举，不能只早watch根。已在同三源修正并固定 `e65e8a92c21544af5c5ff932fc2ded1390f99d77`（仍未native）：每目录register成功才read_dir；同pool单调dirty revision不随raw drain消失；交接先将plan放入既有plans，再比revision；Owned单pending、物理完成后只1轮即时fresh且先让已有queued用槽；持续变化转Partial/既有deadline3s重试，不立即自旋，不超过4+64。该窄fence源已获独审无阻断，无第二observer/registry。

新增测试保原21，变23（Owned12/raw3/真实runtime7/wire1）。pure1核100intent只1followup、公平64queue和持续dirty不spin；真实1受控manual策略在root/already/deep（已存在嵌套父）完成watch+枚举后held，已有pool observer先drain其new child实际事件而新plan尚未入表，再放行；要求首次Native代次≥2、new parent真实注册、后续child写仍通知。Mac运行此强制manual路径只证明生产manual策略与FSEvents组合，不冒Linux/inotify实机。

最终离线准入资料在watch/runtime-v2/{contract.json,run.py,positive-owned.rs,negative-owned.rs}，driver SHA b1dc62a381b396c5ceb09d9e51bcc3b44f169d00f928478c4d3a909b52d1ec5a；py_compile0，尚未执行。Root已同意2个受控negative→23positive：负变体**只**恢复bf11原running/queued早return分支，保新producer/harness，不冒整bf11原字节。每variant仅Cargo no-run编译1次，沿既有guard.select_binary选本树actual compiler-artifact并同SHA复制私有exe；负两exact各FAILED0/1且匹配指定断言marker，不能以编译/fixture超时/强清/残留当红。严格owner/group/private footprint全空且负字节仍匹配才恢复正源并clean，再另compile正向23。两variant各全source-before/after、逐命令source/PID/count、300s/5GiB/jobs1/incr0/外盘TMP/envhash/strict owned。旧v1未运行合同保留；CLI/Core/gen仍真实shared后一批。当前无本作者Cargo/App/browser，等待reviewer最终准入与Root唯一槽。

runtime-v2已实际启动negative compile（exec3014、driver48650、Cargo48670）并严格早停：Cargo101/E0277，watch.rs尝试{:?}打印只有Clone的DialFailure。没有运行任何负case，**不是业务红**；没有正向23。原negative/compile/output.log、result/source前后与stopped.json保留：源稳定、无超时/强清/remaining、TMP空、正源e65准确恢复clean，native槽已交还。最小修正固定 `1dc66584211f1754d676dfeed1493d10ba65aeb9`，仅两行：让既有link::unreached_words供同crate消费，watch沿其本地化失败语义，不扩Debug/错误enum合同。新runtime-v3合同/脚本离线ready，driver仍同b1dc…ec5a、正负owned原字节同v2、11源仅watch/link SHA更新，py_compile0；独审快核已通过，待Root再授同23+2批。原fence未重复审、旧编译失败未回填。


### Watch runtime分层实际终态与共享交接（2026-10-30）

Runtime-v3在1dc固定源实际运行：negative仅恢复bf11原replan短路分支（保新producer/harness，不冒整bf11原字节），compile0，owned/kernel两个exact各0pass/1fail且指定业务marker匹配；原strict cleanup全净后准确恢复正源。positive另compile0，Owned12与raw3过，kernel实际4pass/3fail，wire未跑；原三失败日志保留。已确认apply(empty)反复唤醒deadline任务造成调度风险、根parent join(empty)多尾斜杠造成目标拼写错误，最小修固定0f911558；只读旧seq2→4不靠猜测归因，quiet断言仍原500ms。

0f911558 runtime-v4仅受影响raw3/kernel7/此前未跑wire1，共11真实全绿（exec64497终态0；compile0，私有actualexe SHA eadd5393d617fbcd60720a4e1e5f1d39331d6a2a1f28821fc71585c25f0d7cf9）。所有原1s/500ms断言保留、source前后同、strict剩余空/TMP空、无强清。Owned12来自1dc且其源未变，不合称finalSHA全部23已重跑。新增cfgtest trace仅有界kind/path诊断，不把成功libtest默认捕获的输出当无事件证明。

Root另授一次同0f/同私有exe/originaltraceenv的readonly exact --nocapture：exec26417终态0，driver75939/test75950均已gone，1pass/0.55s，日志无eventtrace行；原20reads+500ms quiet准确通过，不推断旧v3唯一原因。runtime-v4/readonly-nocapture/result.json SHA efacb0a8a4099c0be9bad3d1328586feecf3ee494318d78512522089d51bcc19。source unchanged、remaining/privatefootprint空、TMP空；唯一native槽正式交还Root给S续4+5。

总分层收据 evidence/round2/workspace/watch/runtime-receipt.json，逐11源current/git-show/contractSHA一致、6commit链、v2compile失败/v3负红+Owned12/v4runtime11/readonly增量各源边界与actualexe指纹。Node候选与Core files.rs leaf可交共享单作者；Core尚未编译、CLI38streams/held控制、HTTP服务端强制0/Tauri20s、真实gen和前端消费都仍待。manual每dir先注册路径是Mac真实FSEvents运行，不冒Linux内核实机；idle只seed40ms实timer不冒60s墙钟。整个W尚未完成，refresh未删，Root E源不由本作者并写。

分层runtime-receipt独审尾核已无阻断：11源/git-show/contract、actualexe与所有分层证据指纹一致。receipt.review如实更新passed后最终SHA b08d4aedb2e8881ab23698ee483766a3da1fe0f6fdbb561d95ee92405966c3cc。可供Root只集成Node/Coreleaf，待S在最新稳定Root短branch串接shared/actualgen；并未据此关闭W。前端最小消费方案耐久在watch/front-consumer-plan.md（尚只读，无产品frontend写）；alias真实canonical path/64scope overflow要保明确fallback，metadata单物理cohort，fixedhistory只提示成功HEAD变化。


### Watch前端消费实施中（2026-10-30，单作者）

Root授从共享base6c20c13b/e409+五W链，在同树新branch fix/round2-watch-ui（backend0f原branch保留）。Node_modules仅锁定外盘只读cache软链；无手改gen/shared/srcapi/catalog。新增directory-watch controller/hook，WorkspacePane父owner→FileTree观察、Standalone同hook、StorePage旧dirsWait迁移；scope64/64KiB与canonicalalias不覆盖时明确fallback，当前fileparent优先。Shared五DTO/三动作/真实producer由history单writer。

受控controller11（原8生命周期+3scope/canonicalbudget）、actualhook2、metadata coalescer5、actualconsumercallback7、旧tabs6/memory5已实际0。Hook首次renderA→B暴露旧A batch的真实受控红保hook-red.log，修wantedKey renderguard+layoutcleanup后2绿；cachetree begin失败/hiddenreturn无focus会等60s的实际fallback回调红保fallback-red.log，补已知缓存entry立即一次fresh、quietpoll不重置timer后7绿。首写两脚本因cwd双前缀失败属工具路径错误不冒业务红。当前是模型/受控实际回调，不冒DOM/native组合。

SideWorkspace原invisible树获Root授watchActive单行并接可见file的父目录hook/readAt与partial/focus/slow/visibilityreturn，不改draft/布局。Root另精确移交FileView仅nontext effect/Away元数据：私有单physical/1latestintent，单acceptedlook+identityrenderguard，同readAt/reread；nativeOffice仅stat，Away不再自stat，未知尺寸null；失败同图保留并面内可关重试，无autoopen/handoff/协议。该片将独立提交。原同图refresh先清look的实际effect红保preview-red.log，新privateowner/effect8绿；原text E纯acceptance20/refresh18/sync9/merge9各0，未改text accepted/save/composition/refreshcontroller。

一次执行范围错误如实保留：将旧check-editor-overwrite误当纯回调与其它检查并发启动了现Playwright/Vite，未获本批browser槽且缺真实gen导致导入失败，不计任何case；立即向Root报告。exec82774真实exit1，Node97929及9个正证后代全gone，仅对自有PID发TERM；原cwd/birth/ppid树、失败log和fixture按inode移出生产树保在frontend/overwrite-unapproved-start，邻接mkdtemp证据也保原字节。未碰用户App/Chrome/Cargo；后续入口先读、无新browser/native。

当前尚无全types（等H真实共享API/五DTOgen/catalog）、无consumer真实DOM/CLI38，整个W不关闭。Root已明确text readAt seam非text不生效，所以非text片和sidefile owner作为本W023必要闭包补齐，不能用树native冒所有文件实时。

非text独立2源已固定fa62a6ac334945ae320910ae2d58a9487705349e，正常graph hooks，无Cargo。独审最终逐SHA/7证据无阻断；preview-receipt追加review结论与后续组合types记录后SHA fdc924d084b528c5172c8d1b0c4d2a77d2c59cd23e8625c2e62ca7cf9951a13d。

H实际shared7/native/Core+Tauri/check/DTO1/HTTP2/tables4/gen成功并交真实169managed+catalog；本树原gen不存在，按generated.json SHA155c7b80f9909db109a05487ec9415d1261fa06febb21ecacedda5337d7f5544逐字节复制，catalog4b4be54136e0b6092e75a8deb4e1c33cc40660d0aa8f7a230c88c82f4d189241，frontend/generated-copy.json SHA708afb10b9501f4a81218ad728d4e0bf896317ff9aa3b748ba2e8b41362b0dd2。未手改任一生成字节。API依赖57de4543/72527c7b分别无冲突cherry-pick为75903eff/b60649bb，未混consumerdirty。旧facade+新gen时实际tsc2322 exit2仅4条预期旧DirWatch/missingbeginleave/旧wait签名红保留；两依赖后实际tsc66552终态exit0，types-shared.log。此是组合类型绿，不冒DOM/CLI或原fa62独立时已绿。

当前正式consumer尚dirty：controller11/hook2/cohort5/consumer10纯实际回调、tabs6/memory5各0；Side父file hook已Root增授接readAt/partialfallback/visibilityreturn。CLI38streams仍未跑，无native/browser授权；root-empty浏览MachineFiles/FilesPane读链另已具体报缺口，等待Root裁定两路径，不先写。

### Root W组合收口（2026-10-30）

Root5ddc集成backend/shared/consumer与retained active26fd（原effect业务红→13callback/types0）；36源/885生成相关源实SHA核同。shared7actual、CLIcurrent exact1含38far sources/control（51.26s）及6phase来源/clean门全过；WDOM生产组件6组按v4四＋v5二分源通过并独审，原fixture/setup失败保留。正式收据 `workspace/watch/dom-final-5ddc/receipt.json` SHA2c7bdc03…a1d0c；不是native/WK/physicalIME，最终原生/手机组合待全轮。Root后续M025使集成进aaf91，M16源/engine18/types0；不得把aaf91全树声称等5ddc DOM来源。通用MachineFiles/FilesPane非选中工作区浏览实时问题独立登记后续，不扩本轮W5选中工作区合同。
