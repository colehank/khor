# 下一轮候选

本页只记录本轮开始后发现的独立问题，不加入当前34项实现或验证。

2026-11-01最新安排：用户已允许第三轮开工，四项实现现已完成并在integrate/round3提交，等待体验验收；原生真机仍未验，未授权合并／ship。执行与恢复入口为 [ROUND-3-PLAN.md](ROUND-3-PLAN.md)。下列候选没有因登记全部自动纳入；第四轮仍需另行对齐。

## 诊断日志轮转的UTF-8边界

2026-10-30 Root在R2-D只读定位现有日志时看到：Root5c的 `crates/node/src/doctor.rs::note_face` 超256KiB后，以字节计算keep，随后直接 `text[keep..]`。包含中文的行可能让keep落在UTF-8字符中间而触发字符串切片panic。此为源码风险，尚未运行复现；不把它推断为旧GUI卡死根因。下一轮用私有多字节日志定向复现，再按字符边界处理轮转；当前不改代码、不追加测试。

## 非WK导航事件关联能力

Wry0.55.1 Windows/GTK暴露的Started/Finished没有真实导航身份，且Finished不能证明成功。当前B只保有view/logical stream观察，Settled不显示成功100；旧A终结事件仍可能落入逻辑B。此SDK能力限制已明确，不以URL/FIFO猜配对。下一轮若需要与WK同等精度，再单独评估实际平台SDK适配并真机验收。


## 初始Load期间的交互暴露

2026-10-30，在退役无现产品caller的旧Replay API时，经当前ACP opening调用链和原私有extended fixture核到既有独立限制：`start_with` 等待初始session/load后才把Handle/EventReceiver交给消费者；若加载期间的agent请求Elicit并等待用户响应，UI尚不能消费该问题。当前是源码确认的能力缺口，未用真实vendor复现，不推断为此前GUI卡死或数据丢失的唯一根因。

按“新独立问题只记下一轮”，本轮不为它新增opening API或第二套重放；现有Prompt/Control环境、MCP和互动能力的真实断言继续迁保，不能以移除旧Handle.replay就宣称初始Load互动已支持。后续用专用ACP peer在Load内提问并等待回答，确定事件流与ready future的最小分离；保持同一writer/同一Load、完整canonical前不Ready、Stop/Close有界及真实进程退出事实，再决定产品适配。


## GUI生成缓存的冷重建准备

2026-10-30，GUI实际Cargo成功但原严格来源校验发现Tauri hash资产沿用旧时间：5个第一方script marker均current，唯一GUI OUT有349个旧输出，279被本次dep-info实际引用。当前不改变时间/任意接纳旧文件；最终构建前对精确自有OUT可逆保留再由真实producer生成。后续单独改进准备阶段对这类生成缓存的隔离，让既定来源门仍严格且减少重复编译；不扩成新构建系统。

## 构建失效范围与无关测试源

现有native source_frame包含整个受控源码集，修改仅测试fixture也会使旧生产proof的全frame检查失效。本轮按原helper守规则，先收完消费脚本再做最终构建，不修改旧proof或放宽require。下一轮可按真实生产输入/生成声明与测试目标分别研究失效范围，仍纳入实际使用的Git元数据/环境/依赖；不能仅按扩展名排除，也不能把当前未实现的精确失效当已有能力。

## 已发布GUI宿主退出后的ready诊断生命周期

2026-10-31，公开Load实际70988已完成New/Join/退出/Load/旧历史验证，但两个成功发布的自有ghost在ACP断开后留下`khor-gui-ready-<parent>-<nonce>.why`。源码`gui_host.rs`成功路径删除ready；结束路径以`!ready.exists()`判定尚未就绪，因而成功后也会写启动失败诊断。两文件各49B，内容为agent closed connection；真实provider/host均已退出，不是Load失败/进程泄漏证据。原actual result false保持；两exact诊断另经所有者消失、identity/SHA/lsof门可逆保留，postrun收口receipt31a4c020…9a9cd。

下一轮应明确区分已发布与未发布的启动握手状态，限定`.why`给仍在等待的启动者，确认Ready前拒绝仍可见、Ready后错误由既有持久状态与事件渠道呈现；不以临时文件存在性推断状态。本轮只记录，不追加产品改造或新测试矩阵。

## 跨会话的轻量阅读书签

2026-10-31实际64MiB原生历史场：HEAD搜索定位后切到别的会话再返回，会重建ObservedSession/ChatView/reader并从tail读取。原R2-H/CONTINUITY约定为同实例跨页锚点与切换取消，未约定跨mount/App重开书签；按收口规则只记下一轮，不在本轮增持久窗口/完整历史缓存。未来如实现，只保存每个view/home的有界轻量来源身份+锚点，由当前history source重新验证并按可用范围恢复；不得保存全正文。原黑截图与AXtail不同尚无唯一因果；focused raise/实际指针/滚轮场正文可读，不凭该观察做paint架构改造。诊断LONG-HISTORY-RETURN-DIAGNOSIS在当前evidence/platform准备目录。


## 正式发布发现的检查入口登记缺口

2026-11-01 main eca实际types／unit均0，额外`npm --prefix apps/gui run gates`报告35个check脚本未被npm入口或静态import可达，另chat-recovery已有入口但50-gates文档未列。原`gates.log`与失败结果保留。此为检查发现机制的真实缺口，不冒这35项被unit执行，也不是本次已审功能行为失败。下一轮统一命名入口、组合脚本与文档，并核实际可达性；不把旧发布记录改成所有门绿。

## 安装脚本成功分支末尾的空日志状态

2026-11-01单次onset实际退出1，日志已确认serve up；三节点实际listener/exe SHA与新UI另证均已采用。只读源码定位：serve.sh成功分支最后一条`[ -n "$said" ] && printf...|sed...`在无新日志时返回1，成为远端sh／SSH状态，本地set-e提前退出，缺最终done。下一轮让成功就绪与附加日志显示各自表达结果；本次不修脚本、不重复安装，不回填原onset1。

## lecun既有会话读取busy的调查记录

2026-11-01升级前后同一CLI `sessions --all`均返回1／`document transfer state busy`；实际5467服务已采用新SHA/UI，未观察到_khor持久host进程身份丢失。hinton/turing各2ID升级前后相同。lecun此读取结果保持未知，不宣称新增回归或会话验证通过；后续沿既有doc transfer／CLI read路径单独调查，不扩大本次发布。


## 健康Shell持续显示会话恢复与运行状态未知

2026-11-01 用户在TestFlight61截图反馈：Khor-main终端正常可用，但底栏长期显示“会话恢复”，菜单标“运行状态未知”、恢复项禁用。Root只读实际确认`khor sessions --all`返回0，`shell/44f90aa6ff732de8`／Khor-main显示忙碌；原宿主87421／出生2026-10-28 14:14:14仍运行（仅查进程，不发任何信号）。

源码两处独立缺口：DetailPane.tsx对所有active gui/tui/observe/shell无条件挂RecoveryActions；RecoverySurface不按实际恢复需要收敛入口。session_recovery.rs::session_recovery_here的SHELL分支，仅有ending则Ended／NewShell，否则一律Unknown／None，未表达仍健康存活的Shell。菜单“恢复会话”又是无action可用时的默认标签，用户易将恢复资格未知误解为终端故障。该分支不读取Codex hook执行结果；截图hook审批报错不能解释此入口常驻。

下一轮目标：恢复只在需要重新接入／确认结束后重开／存在待决恢复intent时显著呈现，正常已连接会话无常驻告警式入口；统一会话行与恢复offer的运行状态投影，使用现有可信宿主身份／连接事实区分live、ended、offline、unknown，不能拿标题、历史、PID存在或can_control展示字段当作新启动授权。不以简单隐藏控件掩盖后端分类缺口，也不能隐藏未决intent或对未知归属自动重启。Shell结束后的新run要明确是新Shell，不冒延续旧PTY。验收覆盖健康旧／新Shell、断连重接、确认结束、未知归属与待决intent；未实施、未追加运行矩阵或修改main。


### 同项补充：移动底栏工具区位置漂移

用户随后报告重新进入后“会话恢复”移到最右侧。本次只读追到HeaderSeat→BarSeat(trail)→SessionBar的data-session-bar-trail；终端MobileKeyRow的keyboard／KeyRowSwitch也通过独立portal写入同一trail容器。容器只是flex row，没有按工具职责建立稳定次序；组件挂载／重挂次序影响追加顺序，结构上允许所报位置漂移。当前未再进行真机重入复现，不声称已确定那一次完整挂载时序。

该区域语义是当前会话工具操作，终端主要为软键盘／快捷键行开关；不是固定的运行状态栏。下一轮一并定义稳定工具位置与显隐条件，不将“恢复资格Unknown”当“会话不健康”。后端仅确认Ended才允许new Shell的防重复执行约束应保留；需要补运行中的可信状态投影，而非删除结束证明或放宽启动权限。本次仍仅记录，不改UI／后端或发布。


### 产品对齐：用户与会话的关系由药丸承载

用户明确：重新连接已有药丸闪烁，恢复也应由药丸给出表象并引导点击操作；取消药丸旁常驻的独立恢复文字，键盘工具仍保持稳定位置。沿现PillNotice／PillNotice展开机制规划：正常为普通身份；系统正重连或恢复为进行中动效；需用户决定为稳定轻量标记／短状态，点药丸展开原因及准确可用动作；成功撤去提示。机器不通／归属未知只表达连接待确认，不据此宣布运行结束或默认新建；正常主动结束不持续作故障式闪烁。具体外观待Lab，不在本轮改UI或token。

现能力只读确认：GUI恢复沿native ID/cwd/环境启动新实例，ACP优先实际声明Resume，缺Resume而有Load则Load，不支持时不能冒可恢复；Khor/Pi host.ts使用SessionManager.continueRecent于原stateDir续已落盘上下文，不能等同重做被中断操作。普通Shell只新建关联run，不能复活PTY内存或自动重放命令。

特别边界：当前Khor-main（shell/44f90aa6ff732de8）里的Codex属于普通Shell托管；session_recovery_here的SHELL早返回NewShell/None，不会自动转到Codex原生恢复。**该入口当前没有一键恢复其中Codex对话**。若产品要该体验，下一轮需明确原生对话ID与Shell/run的可信关联、同一写入归属核验、对应TUI/后端续接方式和药丸呈现，作为待讨论的能力扩展，不把外部适配器能Resume当作现入口已实现。无需向用户解释PID；界面应明确可重连原运行、可继续原对话、仅能新开终端／看旧历史的实际边界。新旧SID的关联目前已存在，是否统一在原药丸呈现运行分界需Lab对齐，不提前新增身份注册层或改持久协议。


### 已确认模板：会话关系药丸（2026-11-01）

用户明确确认：“ok, 以此为模版加入下一轮”。以下为正式实施依据，覆盖本项早期“待Lab／待评价”的状态；本次仅保存计划，不授权立即开始下一轮、合并或发布。

参考源：分支 `lab/recovery-pill-20261101`，产品模板提交 `e53f98a5f03e605422b925b6f32dfb8e6bd33f8e`，worktree `/Volumes/disk0/Khor/worktrees/current/khor-recovery-pill-lab`。原型说明在 `apps/gui/lab/session-recovery-prototype.md`；预览 `/lab/session-recovery-prototype.html`（本地静态端口23830）。这是交互与样式参考，不直接整分支合并为产品实现。

**状态与动作**

| 真实情况 | 药丸与展开内容 | 动作 |
| --- | --- | --- |
| 正常已连接 | 普通身份，无独立“会话恢复”文字 | 无恢复动作 |
| 正在连接 | 原药丸沿用现有呼吸闪烁，展开仅“正在连接…” | 无“立即重试” |
| 自动连接失败 | 稳定轻量提示，展开“暂时无法连接” | 重试 |
| 可安全续接的自有对话 | 进入后自动接回，显示“正在继续…”，随后普通身份 | 无“对话可继续”中间确认 |
| 已确认终端结束 | 稳定结束提示，展开“终端已结束” | 新开终端，过程中“正在打开…” |

**交互与样式**

- 状态、图标、动作同排居中，操作不沉底；同一颗玻璃原地增长与收回，保持原药丸位置，点外部或Esc收起。
- 操作使用原有GlassItem无底文字样式、圆润命中轮廓、共享悬停/按压/焦点反馈。触达区与可见底块分开，不恢复被否决的矩形有底按钮；文字横排不换行，最小触达区按现有指针/触摸token。
- 展开只给简短状态与准确动作，不放时间、宿主/原运行/归属等技术说明。键盘/快捷键行工具保持稳定次序。
- 点重试或新开后，展开层按原有增长动效收回，原药丸立即承接同一套活动呼吸；操作进行中不再显示重复操作。处理中主动再展开，也保留相同活动表现。成功沿现有settling柔和退回普通身份，移除遮罩；失败回稳定可重试状态。尊重减少动态效果设置。
- 不靠新建组件树、重新挂载会话或清空内容实现动效。Lab模拟5.2秒仅为观察两轮既有呼吸；正式操作按实际结果完成，绝不能人为等待动画。

**正式适配边界与验收**

先修上文可信运行事实投影与稳定工具槽位，再将相同事实接到药丸；未知连接不能当进程已结束，不能只藏现有控件掩盖后端分类缺口。自有GUI/ACP续接以真实声明能力与归属核验为准；外部只读不接管，不重发历史提示/命令/工具。普通Shell新开是新run，不冒复活旧PTY；Shell内Codex关联恢复能力仍按上文单独核验。

验收覆盖健康旧/新Shell无恢复误报、真实断连重接、失败后重试、确认结束后新开、自有对话自动续接、未知归属及未决intent。检查点击→收回→呼吸→完成/失败的整条链，快速重复点击无重复执行，切换会话/重入无位置漂移，旧历史和阅读位置保留。真App覆盖Mac宽窄屏、手机键盘/单手触达、深浅主题、较大字体及减少动态效果。

**证据与压缩恢复入口**

Vite静态构建实际0，diff检查通过。Chrome实际观察：重试后展开层folded并消失，原药丸visible且`rejoin/pill-signal-breathe`；成功回普通身份，无展开层/遮罩；处理中主动展开也为同一呼吸且无动作；终端新开沿同一收回机制。没有真实会话/API/命令，不把Lab认可写成后端实现或手机/原生真机验收通过。

证据目录 `/Volumes/disk0/Khor/evidence/labs/20261101-recovery-pill`：最终外观 `refined-retry-dark.jpg`、`refined-retry-light.jpg`，活动药丸 `reconnecting-pill.jpg`。旧 `compact-retry.jpg` 等方块按钮截图仅留修订记录，不作为模板。内盘恢复指针 `/Users/zgh/Library/Caches/khor-code-quality/recovery/current-lab.json`；计划同时镜像至同目录 `NEXT-ROUND.md`。main仍为既有发布基线，下一轮须经用户另行允许后开始。


## 已发布终端点击规则的现场反馈（2026-11-01）

用户追问上一轮确认的“普通点击Khor内部、Cmd/Ctrl点击客户端系统默认应用”为什么未生效。已核该需求在SESSION-CONTENT C1/R2-008明确，main实际包含b865e388等接线，R2-008/009验收记录有真实终端/OSC8/文件/目录普通与Cmd目标证据；不是仅存计划未实现。本机当前Mac GUI为91072（2026-11-01 15:19:04），5467实回X-Khor-UI index-B55HQiJV，与上一轮正式发布指纹相同。此证据不代表用户当前具体点击已复现，也不证明手机当前安装版本。

当前源码额外保护值得核现场：content-link-intent.ts::canActivateTerminalLink在已有选区/非主按钮时拒绝，普通点击在application mouse tracking非none时也拒绝，带Cmd/Ctrl才可绕过鼠标接管。URL addon、OSC8、路径provider及coarse tap均使用该guard。此为确切代码事实；尚未知反馈是单击无反应、modifier仍内部还是识别失败，尚未测当前会话鼠标模式，不能先定为TUI接管或让用户重复升级。

下一轮沿用户实际入口和目标复现，核事件modifier、selection、mouseTracking与provider命中/打开意图的整条链；不以旧验收或pure绿否定现场，不改网络可达预判，不无条件牺牲拖选/TUI鼠标操作。若实际冲突为链接区域的点击归属，先对齐已确认的点击规则，再作局部修复。当前仅核查与存盘，未改产品或启动下一轮。

### 用户已明确复现的症状与测试契约缺口

用户在当前终端点击本次专用URL及文件后确认：“单击没有反应，只有cmd+单击才会在系统app里打开”。目标为https://example.com/?khor-test=20261101和/Users/zgh/khor-drop/khor-link-open-check-20261101.txt。这证明其现场至少目标识别和系统分流可用，普通内部打开未兑现；不再把故障描述列为未知。

Root直接加载当前main生产content-link-intent模块，执行无选区/主按钮四项：tracking none普通true、Cmd true；tracking vt200普通false、Cmd true。与现场模式一致，但未读取现场term.modes，因果仍为优先候选而非唯一证明。旧check-terminal-link-intent.mjs明确把“application mouse tracking keeps a plain click in the terminal”及coarse tracking普通不打开判为通过：既有验收范围漏了用户真实TUI体验，不能拿旧绿结论否认反馈。

下一轮修复契约明确：识别出的有效链接/文件区域，单击应内部打开，Cmd/Ctrl单击应客户端系统打开；普通文字区域保留程序鼠标操作。正确区分点击与拖选/右键，不允许发给终端程序与Khor同时执行一次点击。真实启用鼠标模式的Codex/TUI中复验URL、文件、OSC8及软折行，再核shell无鼠标模式、拖选与右键；更新与产品决定冲突的旧断言，不只删除guard掩盖事件归属问题。本次未改产品，也未向用户当前会话发送输入或读取其正文。


## 已确认模板：工作区两行头部与视图导航（2026-11-01）

用户确认：“好, 这个准备进入下一轮”。当前最终Lab作为下一轮正式实现依据，覆盖本项早期的“机器头像:绝对路径”常驻行、branch+commit摘要、整行点击返回及各次待评价候选；本次只保存规划，不授权立即开工、合并或ship。

参考产品模板：`lab/workspace-layout-20261028@dc7adb0b314ed0d83d63b1c8ecd7b6c5362b27da`，worktree `/Volumes/disk0/Khor/worktrees/current/khor-workspace-layout-lab`；说明 `apps/gui/lab/workspace-layout.md`。预览 http://127.0.0.1:23832/lab/workspace-layout.html?workspace=khor 。仅选择性参考该轮头部改动，不整分支合并旧Lab生产基线或样例数据。尺寸与文件导航图标采用下方同日最新修订。

**最终布局与行为**

- 上排保持返回、工作区标题、新建、搜索。下排左侧仅branch；移除常驻绝对路径、提交摘要及时间。机器与完整路径进入原“更多”菜单，可复制路径。
- 分支图标对齐下方文件树文件夹前的“>”列线，分支文字与树文字共用起点；不再对齐返回按钮而浪费树的左侧空间。按同日最新确认，分支图标与文字改用导航药丸相同的 nav-glyph、text-body 层级，工作区标题仍为主标题；独立工作区与会话导航共用这一规则，覆盖早期较大层级版本。
- 下排右侧只有一颗玻璃，左为工作区IconFiles、右为已有IconHistory；两图标分别与上排新建、搜索中心线对齐。点击切换左侧文件树/当前分支提交列表，分支原位保留；不新增“返回文件”行或提交摘要按钮。搜索针对当前视图，新建保留既有行为，右侧内容标签及阅读状态不被切换抹掉。
- 两枚选中底等大，复用实际导航栏LandingStrip的宽高、横向胶囊比例、圆角、材质与反馈；不能退回纯圆形、大小不一或两颗独立玻璃。触达区仍由既有指针/触摸规则负责，不通过放大可见底块凑触达。
- 两排采用与App data-wide-top/SideHead相同的行高和内部留白，当前桌面玻璃边缘间距11px；到文件树的留白也沿同一行体系。窄屏与触摸按共用token变化，不把11px或截图坐标硬编码为所有平台的值。

**实现与验收**

正式实现从当前main接现有工作区/历史状态及实时来源，遵守Clean Code：共享导航外观和栏位计算，避免分别在两排重复算magic offset，不把Lab样例、旧生成类型或模拟数据带入产品。未知/无branch、无提交、长branch、搜索打开状态保持合理，不制造空分支或伪提交记录。

验收包括Mac宽/窄侧栏、深浅主题、UI缩放/较大字体、手机触摸尺寸；检查树箭头/分支图标、两排右侧图标列线、等大的横向选中块、玻璃间距与列表首行。验证文件/历史双向切换、搜索目标、新建、无Git目录、内容标签/阅读位置保留及完整位置菜单。整轮原有工作区功能要求继续有效，只替换这里已否决的头部显示与导航形式。

**保存与边界**

最终截图：`/Volumes/disk0/Khor/evidence/labs/workspace-header-20261101/header-spacing-files.jpg` 与 `header-spacing-history.jpg`。旧圆形uniform、分体玻璃或摘要行截图只保留修订历史，不作为模板。

Chrome实际：分支/树箭头中心约25.8；下排文件/上排新建中心250、历史/搜索中心297；选中块均36×30；玻璃间距11；header结束与tree开始均100。以上是340宽样例的测量证据，不是生产常量。当前Vite构建实际0、diff检查通过；旧Lab整包tsc退出2（未改生产源码/生成bindings不匹配）仍如实保留，不冒全绿或原生/手机真机通过。

恢复指针 `/Users/zgh/Library/Caches/khor-code-quality/recovery/workspace-header-lab-20261101.json`，小型源码bundle已保内盘；本计划同时镜像至recovery/NEXT-ROUND.md。main保持原发布基线，正式实现待下一轮开工授权。


## 工作区临时预览标签使用斜体（2026-11-01）

用户指出上一轮要求临时标签用斜体，当前却仍显示标题前的空心圆。已核main27a9：TabStrip.tsx::PreviewMark仍画hollow ring，单行/双行名称渲染均在f.preview时插入该标记；代码旧注释明确为“ring in place of the slope”，相关旧规则来自9117592c。当前没有预览标题italic处理。上一轮实现/验证了单击预览、替换未固定槽、双击/编辑固定等语义，但未落实此视觉要求；不能以R2-020行为验收或版本一致声称已完成斜体。

下一轮按用户明确规则改：临时预览标签的标题用斜体，移除预览专用空心圆；固定后的同一标签恢复正体，保留原内容实例与阅读/编辑状态。文件和独立commit预览一致，单行/双行及宽窄共用标签入口一并核对；路径副标题无需跟随标题斜体。脏草稿、未读/提醒等不同语义标记不得被误删，也不把选中状态当作preview。验证单击替换预览、双击固定、编辑自动固定与既有固定标签不降级，同时检查真实视觉而非仅状态字段。当前仅核查并记录，不改main、Lab或立即开启下一轮。

## 已确认补充：会话导航内的工作区（2026-11-01）

保留整个页面上方贯通的 session 标签栏；再次点击当前 session 标签展开其原导航页，原导航甲板、工作区／浏览器选择、路径、新建、搜索和关闭均保留。只对齐下方工作区的分支与文件／历史行，不引入独立右侧标签栏或改造聊天布局。提交列表就地切换，每个提交可在原 session 内容子标签中独立打开，文件和提交共用预览／固定规则。

原型位于同一工作区 Lab 的 `apps/gui/lab/session-workspace-navigation.{html,tsx,css,md}`，入口 http://127.0.0.1:23832/lab/session-workspace-navigation.html 。Git 图标和分支名使用与药丸相同的 nav-glyph、text-body，由独立工作区与会话导航共享样式；历史图标与原导航甲板关闭 X 同一竖直中心线，独立工作区仍与上方搜索中心线对齐。不同容器依自身既有内边距布局，不写截图坐标偏移。

同日用户要求“工作区那也同步修改，保持一致”，两处 Lab 尺寸已统一；旧 dc7adb0b 继续作为骨架参考，尺寸以此次修订为准。Lab 的可选内容渲染插槽和内存提交类型仅为原型适配，正式实施需从当前 main 接真实来源，不直接移植模拟协议或整分支。

证据目录 `/Volumes/disk0/Khor/evidence/labs/session-workspace-navigation-20261101`。Chrome 会话导航实测：分支文字与药丸文字均13px，Git与导航图标均20px，历史/X中心均1690；这些仅为当前缩放的测量结果，不是生产常数。该 Lab 不代表真实后端、原生或手机已验收。

### 文件视图与工作区图标区分

用户指出文件／历史切换中的 IconFiles 与原甲板工作区 IconFiles 重复，要求参照 `/Users/zgh/Downloads/文件夹.svg` 并适配 Khor 的视觉大小。新候选为“层叠文件＋文件托盘”IconFileTray：仅替换两处共享文件／历史切换里的文件视图图标，工作区身份仍用原 IconFiles，历史仍用原 IconHistory，不全局替换目录／文件类型图标。

Lab 在唯一 icons.tsx 内复用 Mark、STROKE、cutOf 和 data-body，按相同方形视觉占位拟合；继承 currentColor 与原导航尺寸，不带来源 SVG 的固定颜色、1024画布或导出属性。前托盘为实心层次，前后交叠用统一遮罩留缝；保留原控制的文字标签、选中态、命中区域与列线。正式实施归 R3-03，不单独扩大一项功能；该新图标候选仍需视觉验收。静态构建与图标门单独记录，不冒浏览器／真机通过。
