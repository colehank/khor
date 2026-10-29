# G 共用滚动材质

2026-10-29 working，用户“开始吧”授权本轮实施，不main/ship。owner round2_input从C稳定checkpoint轮换，作者树仍/Volumes/disk0/Khor/worktrees/current/khor-round2-content，fix/round2-content，G基线e58f21a1（C生产+隔离Lab已固定，root负责生产集成；Lab不进产品）。中央仅本卡由G作者写，其它总账root负责。

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
