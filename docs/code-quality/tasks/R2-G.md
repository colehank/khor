# G 共用滚动材质

2026-10-29 实现已封口；等待root最后片段集成与原生最终组合验收。用户“开始吧”授权本轮实施，不main/ship。owner round2_input从C稳定checkpoint轮换，作者树仍/Volumes/disk0/Khor/worktrees/current/khor-round2-content，fix/round2-content，G基线e58f21a1（C生产+隔离Lab已固定，root负责生产集成；Lab不进产品）。中央仅本卡由G作者写，其它总账root负责。

权威范围：ROUND-2-EXECUTION、STICKY-GLASS、SESSION-CONTENT，R2-013菜单材质、R2-022差异sticky标题、R2-024所有滚动固定组块。最终已验参考4d541e5eb83c242304c96a4aeb07682082a59a45，旧浓度方案只历史。不把首片段通过冒整G完成。

## 唯一写权与顺序

root批准首片段：apps/gui/src/lib/scroll-edge.ts现有band owner；app.css仅band/material/transparent相关块；必要新纯几何与真实定向检查；独立Lab入口复用正式实现。先交主标题/正文渐隐契约，再接分组/commit/sticky表头等消费者；新增具体consumer路径先报root协调。禁止第二全页observer；不凭空多拆geometry模块；不改tokens/native/ChatView/WorkspacePane/TerminalPane结构。设计系统规则先读，采用现有token/icon；真机/native仍root唯一槽。

R013先在真实ContentPathField上下文采样最终菜单背景/叠层，不以band透明度修菜单。网页地址推荐继续关闭。R022/R024共享同一材质所有权，W后续仅消费稳定属性/契约，不建第二套。

## 目标与约束

- 固定主标题、分组、提交文件标题、表头无底线或下沿阴影，整体自然过渡；普通内容边界保留。
- 标题文字/控件不透明。正文自下而上模糊/淡出，在最上端完全不可见；需保留二级sticky标题自己的满值文字，不把整scroller全局mask误伤内部sticky。
- 窗口底与正文分开：solid回主题实色，clear保用户opacity，frost保原生模糊；窗口底只画一次，不给标题叠固定tint/黑白块。沿既有设置，不加生产外观滑杆。
- 无blur/reduced-transparency时主题实色兜底；主题/透明切换不能改变阅读位置、首行/scroll range、键盘定位。
- 复用band当前scroller发现、测高、settleBands、observer与清理；真实终端scroller不进入band。迟到/卸载要移除仅自身样式，保留其它owner。

## 当前准备与证据

已只读核设计README/30-behavior及参考Lab，图谱对scroll-edge/PaneBar缺覆盖后才读源。当前band已拥有scroller、测高、Mutation/Resize observer；app.css现有scrolled tint+hairline及transparent取消overlap，后者与新契约冲突。参考Lab按局部正文/二级标题分别mask，不能直接复制sg-*全页扫描。

C3与终端Lab唯一23835/PID92646/exec53679已完成root当前批次；首片段会在不动旧入口的独立Lab复用产品实现。G evidence根/Volumes/disk0/Khor/evidence/round2/glass，片段按source SHA/ref、red/green/真实DOM、原生限制记录；非作者reviewer独审。当前尚无G产品修改或完成声明。


## 首片段实现中（未提交）

C2中文末列wrap已由root复现→40a828a3修复/独审/实际DOM红绿，G基线以此为准。当前G改scroll-edge.ts/app.css两生产文件与check-sticky-fade脚本，未拆纯geometry。band现owner增加局部fade target缓存：无显式targets默认正文scroller，data-sticky-head/body/group时分开二级标题与正文；主头不mask，二级头无tail，held组正文从组头顶端到下沿gap-pane渐隐。滚动单rAF合并read/write，Mutation发现targets、同ResizeObserver监听；同步find取消旧待paint，清理按fadeOwner移除自身样式。并非没有Mutation rediscovery成本，流式childList仍复用现observer扫描本scroller，未新全页observer。

app.css仅band/material/transparent块去tint/底线，blur伪层逆mask、正文参考curve；transparent不再取消overlap/padding，reduce/no-blur由既有window plane实色兜底。7项真实band函数受控layout/observer检查绿，types绿；不是DOM。初red6项旧约束记录保留，之后增加同步find取消pending生命周期检查为第7项。

正式实现Lab /lab/sticky-fade.html已ready，唯一23835/PID92646/exec53679，纯UI无API。窗口三外观/opacity仅模拟已有root属性与window plane、frost模糊在Lab模拟桌面；不改App配置。Lab types绿，root正在实际明暗/三外观/宽窄/滚动与二级头验证，验收期间冻结源。独审当前源码无阻断，重点待看二级head伪层未sticky时是否提前blur首行及嵌套滚动裁剪。证据glass/band-checkpoint.json（源码SHA）、red/green/types。首片段未定稿，不标G/R013/R022/R024完成。


root首轮真实DOM已1150宽明暗×三外观6种＋两次主题切换anchor共8项绿。原Lab“标题增高”两行仍被min-height容纳（52→52），停止的是Lab控件非band产品故障；原band-check.json/失败保留。root准许仅Lab增第三行caption后续验未验部分，不重跑已绿8项。

二级head伪层已按独审/root提醒定向修正：band按on且head实际held几何写data-sticky-held，CSS只held时opacity1，静止/未粘住不提前blur首正文，cleanup清状态；nested受控检查含rest false→scroll held true。7项checks/完整types/Labtypes绿，源与Lab再次冻结待root实际未验部分。

root已批准后续具体consumer唯一权：views/CommitPage.tsx::FileChange attrs/正文边界及对应CSS；views/MachineCard.tsx data-top-heads实际表头与对应CSS。SideList当前组头不sticky，不为了材质新增吸顶功能。W尚未启动，G稳定契约交W消费。消费者尚未修改；CommitPage standalone scroller及display:contents机器表头需实际owner接入，不简单给无box节点mask。


root增量实际标题已真实70px并保持相对阅读锚点。nested阶段停止后核nested-inspect：port.top72＋padding70、main.bottom142而h2.top212，Lab原top:var(--band-h)重复补偿70。root准许只改Lab secondary top0（与真实CommitPage top0一致），未改band held比较/阈值追Lab；原band-failure/nested-inspect/截图保留。源再次冻结，root仅继续未验nested部分。


## 首片段固定checkpoint

生产提交`62eb3c980d995015e7397af2dbcb6ad33eb41a1e`，独立Lab提交`1519629749b610015928f1e3cf2030c78c3e5a75`，树干净。生产3文件：scroll-edge/app.css/check-sticky-fade；Lab四文件独立，不进入产品集成。7受控实际band checks/types/Labtypes均绿；source未变不重复跑。

root实际browser/band-check.json最终21结果：明暗×三外观×宽窄12，主题相对anchor4，真实标题70/窄屏动态2，nested标题满值/伪层rest0-held1-nextgroup0两项，reduce单opaque plane1；pageerrors0。前两次Lab控件/offset失败与inspect原样保留，不回填旧通过。band-receipt.json含固定refs/源SHA/范围。只共用band首片段就绪，不代表G全部或原生窗口/物理触摸完成。

接下来的实际consumer约束：CommitPage窄屏无主顶部band、独立commit-scroll；MachineCard进程表独立load-top-box且display:contents容器无box。须在同scroll-edge owner提供局部sticky组的scroller边界，复用几何/清理，不复制第二套/新全页observer；给最小接口/DOM布局后再写消费者。已授这两consumer路径，仍禁tokens/native/ChatView/WorkspacePane/Terminal结构。


## 第二片段：局部sticky口与CommitPage（未提交）

root已授实际接口：watchBand私有共用注册体，band既有API，stickyGroups仅无主head局部口；唯一owner在每次find first-wins，后注册不夺旧owner，不造未来fallback协调器。local用data-sticky-scroll纯标记排除旧edge mask，不继承band-h/加padding或overlap。10受控checks=原7＋local边界/布局不变＋两种先后注册归属，types绿。

CommitPage desktop现有头ref=band，narrow port callback依[narrow] detach cleanup后重新注册，cleanup仅同node清scroller.current；metadata/stat与diff正文为body，section/group和file-bar/head分开。共享paint/observer/生命周期没有复制。独审初diff无阻断。实际CommitPage Lab /lab/sticky-consumers.html已ready，同23835/PID92646，仅本地git_commit两文件固定diff；Labtypes绿，root正在验宽窄owner、静止首diff/held、标题满值与跳转，不改Machine源干扰此批。

Machine盘点更正：load-top-box只是clip盒，Top明确流在页面中，没有自己的overflow口；不得新增内滚动。已标table targets会使现marked优先分支漏掉charts/metadata的问题必须随consumer解决（对regular blocks显式body边界或自动拾未标兄弟），不mask含sticky的祖先，不逐cell目标。实际布局/最近primary port待继续核，Machine尚未改。


### Commit实际遮挡→定点修复中

root真实W→N→W抓新primary跳文件遮挡：primaryBottom124/bar124–156，bodyTop104（52px被吃），narrow local bodyTop152=barBottom152正确；原jump-inspect.json/图保留。jump现扣实际getComputedStyle(box).scrollPaddingTop，local auto解析0；真实callback4项旧2fail→新4pass，重复跳转/无目标保位置。

root批准无间距diff/表格显式role data-sticky-body="flush"：仅group真实held时正文tail0，在header.bottom满值；同group:has(>flushbody)派生head伪层bottom0，免默认24px blur仍糊首行。无新增空白/不猜文件类型/不新增同步状态；普通段落保原gap-pane。新增flush边界先红→owner11绿，types/diff绿；真实增量DOM待root，源再次冻结。Machine尚未写，不因Commit验收扩大并行实现。

root同时已批准按含sticky分支递归的discovery：有markers时深入该分支，普通无sticky兄弟整块body，显式head/body止步；不mask含sticky祖先，不逐cell。等当前root批结束才写，解决Machine charts/metadata不能漏fade。G稳定后按docs50做必要设计规矩/正式脚本入口最小接线，不起工具治理；旧band tint+hairline/透明取消overlap规则需更新。


## 局部owner/Commit consumer固定checkpoint

生产`660f7e2fa872d8e578ba2c5a3f5d199c323bc7e6`（5文件），独立Lab`babb78218aad32038526b26515f51e7c1bb8f86b`（5文件），树干净。11owner/flush检查、jump4（旧2fail→新4）、types与Labtypes绿。commit-receipt.json含5源SHA、原red/green分开及实际范围。

Root真实宽1200→窄390→宽1200增量jump-verified：wide bodyTop156=barBottom156（旧104），narrow152=152；只一port marker、primary52/local0 padding，flush head::before bottom0，body mask none为已满值、未加空白。首次harness误要求必须有mask，原first-mask-expectation失败保存；没有放宽产品。随后commit-fade-states四项静止/小滚也绿：宽窄静止首diff无提前material、标题mask none/opacity1；jump后多滚12px正文fade endpoint=header.bottom，标题仍满值、伪层bottom0，unknownAPI空。

当前root浏览器批结束，后续继续Machine+frontier regular siblings，不重跑21主band。root已授权check-face机器头旧rule约19007–19327最小适配，先取得真实consumer/原旧预期红证据；保列对齐/清晰度/裁剪/滚动，观察新无边线/材质/fade，不能直接删判据或跑38k全face。


## Machine consumer下一检查点（尚未改Machine源）

共用frontier discovery已按root授权实现：包含markers的分支才递归，普通兄弟整块body，显式head/body止步、nested primary scope不抢；无markers仍默认单scroller。regular sibling先红→12全部绿、types/diff绿，保charts/metadata、不mask含sticky祖先、不逐cell。

真实MachineCard Lab /lab/sticky-machine.html已ready，同唯一23835/PID92646，正式MachineCard+外层band，本地about/load120固定process，只有原data-nav-clear页面滚动。actual MachineCard/表头CSS仍旧，root先采列tracks/cell几何/clip/排序与旧下沿rule不符新契约的red；可点data-top-all展开长表。Labtypes绿，source冻结待基线后才做header/body真实grid/subgrid；并非已完成Machine consumer。machine-checkpoint.json记当前3源SHA与状态。


## Machine/frontier固定checkpoint

生产`7c9af9f06b29c9914189ecc72fc71b1f8456f893`（4文件），独立Lab`23a18c90210a24072216f5ed1424c172b5e014a5`（5文件），树干净。machine-receipt.json含4源SHA与13 checks/types/Labtypes、原red分开。原列几何/row tracks/clip已过后发现held阈值漏Machine原padding16：port72+68padding真实header140–168，而main.bottom124；原failure/inspect保留。

common owner现按每头物理edge=port.rect.top+clientTop+computed paddingTop+computed heading.top，必要style在find/同RO失效后缓存。main正文边界不动、held正文仍真实group rect/flush0；没有改table top/padding/滚动迎合假设。新增border2/original16/stickytop8受控反例先红→13全绿，原local0/普通band对照保。

root browser/machine-check五结果全绿：wide列/row旧结果保留，真实padded edge140=旧位置、标题满值/body端点168、普通兄弟fade；CPU descending→ascending首PID改变；narrow旧cards tracks/clip、无新scroll/overflow；unknown/pageerrors空。旧hidden mobile sort是harness点隐藏头，失败独存；header由真实box/subgrid承载后去旧shadow和gap填线，body tracks/文字列x保，不把head bbox旧填gap扩宽当列错位。

当前root继续用workspace-return实际入口采R013路径推荐，运行源/配置冻结。仅提交refs未改字节，不HMR；docs/package/旧face断言只读准备，待其批次结束再写。G未done，原生WK/窗口/真人验收仍最终组合负责。

## R013：路径推荐的实际交互阻挡（实施中）

root真实ContentPathField入口发现菜单背景已厚底（light rgb255、文字对比14.92）、完整未裁剪，但frame/两个candidate中心hit-test全部命中后方空目录DIV。path-menu-inspect/check原失败保留；path-menu-layer及minimal-layer单属性诊断中，关闭location glass filter仍失败，仅Toolbar或pill positionrelative+z1即命中。根因不是背景浓度：SessionContentPane的NarrowBox使默认PathField below菜单静态内联，固定工具条的候选未脱离布局且所在层落在body后方。

root授新增精确写权PathField.tsx、ContentPathField.tsx、session-content.css。当前三文件窄修：共享PathField新增menuLayout adaptive(default旧窄表单inline)/overlay，ContentPathField显式overlay、手机仍above；ContentToolbar局部relative/z1。未改输入选择、键盘、路径提交、网页推荐或菜单底。13字段dismiss回归与types绿、diffcheck绿，独审当前源码无阻断；root正在同真实入口原10case续验，运行源/配置再次冻结，未提交不标完成。

收口准备另有已授docs/Face最小窗口：README/30/50改旧band tint+hairline/透明取消overlap的过期说明；Face保字重/墨色/chart-table内容seam/列tracks，把必须shadow改固定头无分割线，原clip盒无内滚恒跳过的分支改真实nav-clear滚动/实际padding阈值、标题满值/held、body endpoint/flush材质及排序。当前Face仅node --check，不称整Face通过。旧headRule精确谓词对root实录shadow none的红证据在face-old-rule-red.json/log。现有scroll-surfaces Lab/runner最小新契约适配与正式13+4 npm入口已获授权，等root实际菜单批结束后才改，避免HMR污染证据。


## G 实现收口与下一槽交接

R013生产`52415f2e060d4ece08b735636e06a401681ef176`已固定／独审，root集成f6755103；实际原入口12项绿，path-menu-receipt.json记录3源SHA／field13／types／真实范围。随后Toolbar改共用band后又针对实际宽窄候选点击、焦点、Enter与同一脏CodeMirror DOM回归2项，不把旧12冒新源全矩阵。

门／设计规矩`8658c564`已固定并root集成867c1446：既有scroll-surfaces新正文渐隐契约36整入口绿，正式owner/jump脚本登记。Face只精确no-divider谓词和sticky段实跑；首次2rAF采到.815295为180ms动效中间态，原失败保留。真实0/100/240ms采样.815758/.999314/1、几何与scroll不变后，仅门加有界等held且opacity精确1，保所有后续严格断言；settled段绿，非全Face。gates-receipt.json列范围。

额外真实消费者生产`547a9496`、Lab`5a229057`已固定，独审无阻断：ContentToolbar page显式locator与同owner data-band-under overlap，默认邻接不变，native无DOM landing不加padding／margin；TermPeek保持真实table，只有table挂owner，JSON不挂。受控owner扩到14项先红绿，types／Labtypes绿。extras实际12＋table mask真实paint对照、原会话内容宽窄菜单／dirty DOM2绿，原缺CSS夹具和CM synthetic fill假设失败均保留；extras-receipt.json含源SHA与限制。

最后两消费者生产`eaaf7c39`，独立Lab`dd9e7751`，树tracked干净。SessionSide仅import/ref/data-band纯增量，word那行未改；root必须onto已集成A2 nullable源cherry-pick，不能整文件覆盖。CodeDiff只wide既有标签行接band，narrow无新标题、不改MergeView／EditorState。真实基线是1个.cm-mergeView有效scroll口（内部2个scroller虽computed auto但range0）；初去边线头高23→22／range1519→1518，现用既有hair padding保原23／首行／1519，不画线。side-diff6项＋SideBody包真实内头pane归属1项实际绿；内头独占owner1/pad52，外头不抢。最终types／Labtypes／diff绿，非作者独审无阻断；side-diff-receipt.json记录2源SHA、原失败与范围。

完整消费者适用／排除清单：`/Volumes/disk0/Khor/evidence/round2/glass/CONSUMERS.md`。覆盖现App／Region／PaneBar／Tabs／Workspace搜索主band、Commit二级、Machine表头、ContentToolbar、TermPeek、CodeDiff、SideBody；SeatHead／SideFind在滚动流内明确排除，不加sticky，普通SideList组头／底部任务控制／横轴sticky／终端画布不扩范围；无调用的AddressBar与embedded fallback不按旧名字造新能力。

G实现到此封口。原生窗口三外观／WKWebView paint／物理iPhone触摸留最终组合验收，不用Chrome代替；新独立问题只登记下轮。作者C/G树HEAD dd9e7751，唯一23835/PID92646（exec53679）所有G/C Labs继续冻结供最终验收，不重启用户App、不清用户会话。root准备将W树ff到最终集成HEAD后再正式交接W写权；目前不写W、不启新服务，保留C Lab源与旧E23834恢复点。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
