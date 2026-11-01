# 第二轮验收总账

本页是34项的验收索引，不是完成声明。最新源码/所有权/运行槽见[STATE](STATE.md)，用户决定见[原反馈](ROUND-2.md)及专项文档。编表基线为4f7e732e，当前Root a228baef。f23前端actual35387和normal三产物actual86710已通过；最后G最小修与过期注释已整合，当前最终产物尚待构建。G定向绘制91894已绿，不冒Mac/WK复验通过。实际行为按各行限定范围及STATE，不能由构建推导全项通过。源码实现、pure/DOM、Rust定向测试、真实WK、真人IME与手机触屏各自证明自己的范围。

## 逐项最小完成证据

| ID | 包 | 用户目标 | 必须观察的结果 | 当前仍需注意 |
| --- | --- | --- | --- | --- |
| R2-001 | D | 旧卡死调查 | 复现时留下确切GUI/WK身份、栈及有界资源/IPC时间线；若无法复现，保留未知，不能用H通过替代。 | 限定调查已留证；原现场缺失，仍未证修复。 |
| R2-002 | S | 状态分层 | 分别验证GUI连接、终端宿主、agent执行和历史恢复状态；GUI退出不把独立任务误报为已停止。 | 真实App A42497强退重挂、B17620全roles退出后显式恢复均观察sameSID/原canonical正文，原host/provider与新run事实分开；最后正常Quit/ALL净，三已知诊断单独精确保留，原整体false不回填。Boot69866当前regular exact纯判定1/0/0，不冒实际OS重启。 |
| R2-003 | A/S | 导入与PTY归属 | 外部导入显示实际能力与可读历史；无原PTY时不伪造可交互画面、不自动启动第二writer。 | 能力/有界链及外部只读已有定向证据；实际87058只读两旧Shell ended/attachablefalse/can_controlfalse，62253冷App可见两旧1100B屏，无attach/输入。实际App A/B验证持久身份与canonical正文，未伪造旧PTY。 |
| R2-004 | N | 状态查询并发 | 当前CLI在真实root/meta锁竞争后成功查询，原ID保留；try/等待预算及锁序仍成立。 | 8父/2helper实际通过且6源整合；不外推原集群唯一根因。 |
| R2-005 | A | 全部已接入ACP观看 | 按每个adapter实际能力提供正文/只读/控制入口；外部观察不接管。 | 源码能力矩阵与定向链通过；不是40个已安装agent实测。 |
| R2-006 | H | 长历史 | 有界分页/渲染/缓存/传输；可取消并保持阅读锚点；搜索、复制或原文出口不把截断冒成完整。 | 基础读取/出口与既有browser已通过；真实chunk分组、读取意图及最新思考定位已修；当前28项reader、完整32项Find worker在864实际通过，Reader真实28过但caller误计29的原false保留；跨页query/wholeplain-copy已实现，完整DOM消费者actual28700原7＋全部Range真过；实际17f默认App80153原生1201记录及184,613B巨条完整Export→系统Save逐字相同；96355与17256实际67,110,120B/53,262条原生历史分页、未加载HEAD全文搜索并定位、取消、WindowRaise/指针返回与滚轮可读。原paint不足现象因果未知；跨mount阅读bookmark不属本轮，仅NEXT。系统Copy28765小14B实际成功；17415 f23 giant明确too_large且原38B sentinel真实Paste/Save不变，normalQuit/ALLTMP净，独审047c3586。无真实WebContent峰值证明，不把parent单点RSS冒峰值；不新增峰值验收要求。 |
| R2-007 | S | 统一持久化恢复 | 同一会话身份恢复canonical正文；存活重挂、已退任务和离线明确；不重复load/writer；长合法base无丢失且启动内存受控。 | Store真实67,174,400B payload及有界启动已通过；运行代次拒写5exact已实际通过；前置reservation、Load新display base与Resume实际Note的8项组件验证已通过；真实New/Attempt及known-native生产组件原8父＋2helper已54318/14117通过；公开显式恢复/新Shell源码与当前Recovery17组件已接并通过受控UI验证；真实Node当前85431原8父＋2helper完整通过并独审，真实CLI/Core公开Load70988业务断言全到且双审：Join与重放无重复writer、退出后同SID新run、Load1/New不增、旧binding仍读；整场false仅两个自有诊断，另据31a4c020收口，不回填。真实Resume57844业务全到（同native新run/Resume1/Load0/New不增/原opaque输入与旧record/raw保）；其已知诊断另收口原false保。当前874c完整Unsupported4303实际0/6.260s，原5s两次host/provider全部退出、replay无额外启动、fresh recheck Init-only Refused、最终Init4/New1/Load0/Resume0/Prompt1、无forced/TMP净；独立尾核已通过。Shell9775真实0/8.756s＋独审，原A仅Ready后执行一次、DirectoryRequired无spawn、用户新cwd建立唯一默认Shell B、同payload live/ended均同B、旧A保持及两PTY实际回收。两同机真实privatehome Offline4 actual63373完整离线无本机执行/Unknown、samekey重开后原payload Join原SID/run/host/provider已通过并独审；不外推物理双机。Host死亡但provider仍活已在43ff实际35829复现业务红：hostAbsent/provider精确Present，Unknown成立但actionNative不满足None；后续显式恢复/replay断言未到。原false与冷现场保；最小修复fb01已实际81606通过：Unknown/None＋显式sameepoch Unknown/replay无增，原Meta/Attempt/binding/raw/history保持；同artifact原R7正反例通过，各1/0/0，独审及strict清理通过。完整App A42497存活重挂先读FIRST再SECOND、B17620全退冷读旧body再显式recover一次均已真实闭业务；原false仅已知why，另以7f450784精确留存诊断，原值不改。Boot69866纯exact另证，不冒OS重启。 |
| R2-008 | C | 打开意图 | 同一路径/URL普通点击内部，Cmd点击系统；不作可达性预检或额外改写。 | GUI四intent11244＋14739原生已到；终端42114普通wrappedfile/OSC8，加62032普通目录、plainURL普通/Cmd、fileCmd/dirCmd及OSC8Cmd均真实到Khor/Chrome/TextEdit/Finder的exact目标/marker。专用外部tab/doc/window已关，正常Quit/ALLTMP净。独审2a615088；不新增网络可达预检。 |
| R2-009 | C | 软折行路径 | 同一生产终端入口长路径在宽/窄屏均可识别并命中，网址对照；不误把硬换行拼为同一路径。 | 原42114＋62032＋29379有限主例闭合：真实1080→904→1544，904四物理行末及1544两行首都打开相同fullfile；实选区不导航，两printf硬行不误拼，随后软行正控立即命中。原非login sh/uniqueSID、正常exit0/reap/原rows保持、App正常Quit/ALLTMP净已独审2a615088；不是每glyph/remote/appmouse新矩阵。 |
| R2-010 | C | 回当前工作区 | 离开session cwd后左上动作回当前工作区，不在推荐列表追加同类项。 | 原80119实际离开EditorSession后Back回原cwd已观察；14739当前相关源码冷App的Back清旧右内容/恢复工作区及正常Quit/ALLTMP净另证。推荐中未加工作区捷径，不把旧80119末超时改绿。 |
| R2-011 | E | 路径/URL拖选 | 从中间拖至末尾松手保持选区和编辑态；真正外点正常退出。 | 0f606实际80119：长Directory path与长URL都从中段拖至真实末尾/框外松手，实际Selected尾段保留且仍编辑；真正外点才结束。该批末300超时原false保，Root原生事件不冒真人IME。 |
| R2-012 | E | 退出搜索 | 首次外点或Escape退出药丸搜索，过滤不残留；输入/候选内部操作保留。 | 0f606实际80119：不存在查询→Nothing here，首次blank click或Escape均关闭目录搜索并恢复原files；该批末300超时原false保，个别已到行为单列。 |
| R2-013 | G | 推荐菜单材质 | 工作区建议可读、有足够底色和单一叠层；浏览网页仍不显示推荐框。 | Light50409与Dark3807宽窗三材质已实际观察；Dark904窄窗原红经d4b835ba修复后actual68689已正常Quit/独审闭环。G其余固定区域另列R2-024。 |
| R2-014 | B | 真实加载进度 | 同导航进度单调，真正成功后结束；失败/取消不伪100；非WK能力不伪造比例。 | 规则/原生定向证据已有；真实B60137同一导航pending可见、实际完整CIBOL后进度消失，closedport失败与Retry不伪成功；不外推任意网站或手机。 |
| R2-015 | B | 网页初始背景 | 首帧、加载、失败/重试随主题底色，网站自有背景仍保留。 | 深色80153既有实际；当前fb01 B60137实际Light/Translucent90 pending首帧offwhite/完整页自有浅底，真实closedport失败＋Retry浅底，无成功bar。Root CUA与严格退出尾核分列，手机另待。 |
| R2-016 | B | 连接拒绝 | 采真实WK错误/导航身份/访问运行路径；区分失败，不能用独立HTTP200宣称修复。 | 当前App80153真实WK已加载原lan.cibol.khor.cloud/guest访客页面；闭端口56757显示连接失败+Retry。证明当前路径行为，原历史拒绝唯一原因仍未知。 |
| R2-017 | B | 手机网页视口 | 同一网站/登录状态对照，fixed/sticky底栏可见可点，网页frame不进入Khor甲板操作区。 | 真实手机同页几何/点击待测。 |
| R2-018 | B | 浏览器左滑 | 置顶、删除都可操作，置顶项可按决定关闭；不清网站数据或其它标签。 | 当前fb01 B60137实际桌面context Pin＋Delete，置顶后Unpin＋Delete，专用row/tab删除且另一原URL/页面保；正常Quit/ALL/TMP净。手机左滑仍待，桌面不冒触屏。 |
| R2-019 | W | 工作区最终布局 | 未选右空白；选后头/机器头像:路径/branch+commit/搜索新建与右顶独立玻璃符合已验Lab。 | 17f实际80153已验桌面侧栏标题/返回/新建搜索/头像:路径/branch+commit、未选文件No file open及无重复文件标题；874实际91690已验无常驻Reload/More仍有。90f804冷启动树初值修复已fb01实际14739验证：初始lefttree＋rightrestoredfile同时恢复，Back清旧右内容，normalQuit/ALL-TMP净独审。窄屏仍待，不重commit流程。 |
| R2-020 | W | 独立commit标签 | 当前分支完整历史按需加载，左栏原位返回文件；单击预览/双击固定，各commit独立，标题正文/展开/阅读位置保持。 | 17f实际80153：当前分支71提交按40+31加载；70双击固定、69及00单击预览各自独立，70保留；完整标题正文、文件默认展开/收起及无多余返回均观察。该组件后续源码未变，复用此实际证据。 |
| R2-021 | W | 菜单单滚动 | 长变更列表只一根滚动条，跳转保持焦点和阅读位置。 | 17f实际80153：39变更文件的真实jump菜单仅一根内部滚动条；跳至35后后续AX与截图保持下方阅读位置。组件后续源码未变，复用此实际证据。 |
| R2-022 | G/W | diff固定标题 | 标题与标签栏一体，无细分割线；滚动时正文自然隐去，标题不被穿透。 | 874实际91690：39文件diff下深浅及三材质代表组合可读，标签与固定标题无分割线；后续材质变化保持file06/07。首次dark→light位置file05→06原因未知，不冒精确像素稳定；目录建议材质/其他sticky入口另待。 |
| R2-023 | W/E | 实时更新与草稿 | 树、当前文件和Git外部变化及时更新；组词/脏草稿不被覆盖，迟到结果按身份拒收。 | 当前d4b actual5794真实同打开文件V2自动刷新、expanded tree空目录建删、private branch+path-only commit自动刷新；0555真实os13保exactdraft，外写V3保稿，0755后原CAS冲突→显式Save mine→22B原文落盘。normalQuit/source-env/ALL-TMP净且独审，其他dirty/index保；不冒IME/远端FS。 |
| R2-024 | G | 共用固定区域材质 | 统一连续渐隐；最上正文完全隐藏，窗口底按不透明/半透明/毛玻璃独立变化，阅读位置稳定。 | 874实际91690已验39文件diff共用固定标题及六外观组合。新真实Processes88916发现labelghost；最小修a228的同几何绘制77782红→91894绿（direct3165→0，unit15/15），原脚本/fixture未改且独审c83f7d1f；最终当前native表格复验与手机仍待。 |
| R2-025 | M | 候选连续跟手 | 三入口方向一致，连续拖动/循环/回中暂停/松手吸附符合参考Lab，触屏不提前直跳。 | 生产DOM/共用引擎已过；真手机手感待测。 |
| R2-026 | M | 自定义键行 | 编辑、保存/重开组合键及序列；焦点/修饰键/隔离试按正确，使用既有键行外观。 | pure/UI/真实PTY已有证据；手机软键盘待测。 |
| R2-027 | E | Mac中文组词 | 真实系统IME组词不丢拼写；外部/迟到更新不覆盖composition与草稿，最后输入可保存。 | 定向逻辑/专用程序已准备；真实IME仍待。 |
| R2-028 | M | Codex组合键配置 | 自定义Shift+Left保存后真实TerminalPane/PTy发送与常规键盘事件路径一致，独立草稿保持；不冒vendor具有选区能力。 | 真实隔离Codex/PTy完整press+keyup与常规键盘事件路径同，原oracle误判另保；手机入口待测。 |
| R2-029 | Q | Markdown括号公式 | 文件预览同时支持\(…\)、\[…\]及原美元分隔符；代码/转义不误渲染。 | 同一数学链/代码与转义定向检查已过；当前默认App80153实际文件预览四种分隔符全部渲染，normal CuaQuit0及独立盘尾核通过。 |
| R2-L01 | S | 取消retry deadline | 原ignored exact不改时限/断言；真实匹配CLI worker启动不早于due，fresh run不饿死，进程均reap。 | 当前666 Node actual producer＋原exact实际1/0、fresh不饿死/旧retry不早于due；原040唯一原因未知。 |
| R2-L02 | S/B | 原生关闭交接 | 原late-close与close-reopen两失败case实际WK通过；旧owner退出、新owner保活、最终handle全空。 | 666真实GUI原两case及正常CUA退出/全清理已通过并独审；首收尾超时仍未知，不扩原四case。 |
| R2-L03 | E | 所有入口草稿退出 | workspace、session内文件与机器文件同一保护规则；保存失败可取消退出，最后一键保存后正常OS Quit。 | 共用owner已有定向证据；当前0f606真实Workspace88179（含SettingsCancel）、Session81735、Machine33349各自Savefailure/Cancel/恢复权限/最后字符立即Quit已通过并独审，正常退出及TMP净；仅Root ASCII，真实系统IME/真人交互另列R2-027。 |
| R2-L04 | V | 原生构建来源 | 实际当前GUI/CLI来源完整、环境/ACL目录/生成文件规则守住反例；旧失败不回填。 | f23当前normal3 actual86710五phase0/339.578s、前端35387 fulltypes/Vite/188gen278assets已真实通过且独审；最终a228 G/comment后正准备一次当前构建。iOS16142原300冷库未完成，无compilererror，原false/未知TMP保；后续builder源3独审共享600仅库且readyfalse，仍不冒physical App或设备通过。 |
| R2-L05 | V | 正常退出与持久宿主 | 实际正常OS Quit后同私有host认证重连并读旧marker，随后显式清理自有fixture，无信号冒正常退出。 | focused真实App/重连/清理已通过并独审；不冒完整S恢复或E输入。 |

## 实际执行顺序

当前Root与实际运行以STATE为准；以下17f6afb0为历史执行基线。Known生产组件actual14117原8父＋2helper真过并独审；当前Find32独立真过，Reader实际28过、caller误计29原false保并独审裁定。Range保留字点红基线真复现，e122修复已源审整合；完整H原59083 false保；修正目标IO夹具后actual28700默认8组（原7＋Range全部）真过并独审，受控Chrome不冒WK/OS出口。ACP9255 default缺Turning事实已判夹具，e211补帧后actual38247完整suite已通过且独审（当前Rust/Core/平台不外推）；公开恢复Core/Node/GUI与Shell并行接线，真实平台未完。Source/实际/scope均以STATE为准。

1. H完成真实来源驱动的chunk分组及必要raw/copy/分页消费；S按短row gate与RunKey→provider前reservation→Load/Resume事务→明确恢复/新Shell接线串行关闭既定缺口。两作者分配不重叠源码窗口，固定小片经非作者审后整合。
2. 接口稳定后一次实际生成/catalog、noEmit/Vite、受影响consumer；再C/Core定向原生，最后同源CLI/GUI生产构建。旧16ee合同不能直接当337当前准入；未受影响的既有Store/A/B与关闭用例不因无关改动全重跑。
3. 私有工作区/测试文件完成桌面W/C/G/Q/B组合：导航、实时更新、路径拖选、长diff/菜单、公式和真实WK。每个结果标明实际入口/主题/材质/宽度，未测的组合保留待测。
4. 完整S验证存活重挂、host退出/重启、远端断线、能力驱动显式恢复、旧代次拒写与新Shell不重放旧命令；NoLoad正常Quit重开草案尚未执行，不能替代全S。仅私有fixture，不操作真实用户会话或调用真实agent/API。
5. 新鲜真人可用性后才开限时Mac IME/路径选区/各入口草稿/保存失败取消及最后一键→正常Quit。手机另验同页viewport、连续候选与组合键行；模拟不能替代真实触屏。准备匹配当前observer devUrl GUI及physical私有Lab后才约人。
6. 逐项精确证据与最终完成审计。R2-001/016若仍无原现场/当前复现则按限定调查如实交付因果未知/未证修复，不新增用户接受unknown的审批门，也不以此无限扩展调查。新独立问题仅NEXT-ROUND。

每包仍须有非作者Clean Code审查：领域名称、唯一状态owner、IO与纯规则边界、取消/失败清理、迟到结果身份、删除被替代分支，以及只解释必要原因的注释。不得以零注释、格式化数量或新增测试数量当完成目标。


## 2026-10-31 14:05 最新有限闭环

- G目录建议：原50409 Light三材质与3807 Dark三材质宽窗事实保留；3807在904窄窗候选不可见是真红。单PathField共享overlay几何修复d4b835ba后，actual68689新当前App真实1720→904三候选完整可读、EditorS过滤/窄窗点击补全child/回1720重新对齐/Return/外点闭合，正常CmdQ0/source-env/ALL-TMP净，非作者尾核通过。result7eaecd66…5eca7/UIobs45c687ea…67d7。只闭该窄建议问题，不冒其它sticky或手机。
- W023上述actual5794 result37f22150…c75e8/UIobsd18dfe0a…8f87f已独审真实尾核。Root最初typeText漏下划线原因未分类，之后正常native Paste建立exactaccepted draft才测外写保护；不以该场证明中文/按键输入可靠。
- C009 actual49137：已创建真实非login sh/uniqueSID760939224e72f59b，原guard正向观察host84929/child84930；blankline正常exit0/原两PID组无/旧rows保，App正常CmdQ0/ALL-TMP净。但wrappedfile row普通点击和OSC8普通/Cmd未见打开目标，未建立正向hover控制，故链接业务未闭合，需区分CUA坐标事件与产品处理，不先猜唯一因果。result20e5ca5f…d9fc5/UIobsfa4f5cfb…03b46。
- R001按原限定调查要求可交付“调查完成、原卡死唯一因果未知/未证修复”；R016当前真实WK路径/失败分类已有证据，历史唯一因果仍未知。不得新增必须用户接受unknown的审批前置，亦不得冒已修或无限扩调查。


## 2026-10-31 15:36 当前复制与平台边界

- 系统Copy actual38171生命周期正常而业务红：小14B与giant按钮均generic fail，实际粘贴为已知38B sentinel，不算成功；生产Core/Worker inspect v3 actual49554正常结束后分层确认为small Incomplete、giant typedTooLarge。AA修复先旧源回归红，再focused＋完整Find33 actual38443真绿；当前真实复制复验尚待新包，不以Node/构建冒系统Copy。原2621/49772诊断false与清理事实均保留。
- 用户已明确当前不方便，Mac真实系统IME/Shift选择和physical iPhone交互保持未验；不以早期方便答复开限时人验。自动实施/验证继续，完整34项目标不缩小。当前iPhone连接仍不可用，已完成sim cfg编译仅compile-only，不代真机。
- 已完成ios-sim可重建缓存精确退役actual45169/receipt69a613c6，实际外盘释放约6.74GiB，唯一quality target约35.16GiB；现AA native在唯一target串行，jobs1/incr0/offline，系统13GiB/外盘约110GiB，仍逐重批检查峰值和20/10保留。所有原iOS证据与冷0B现场保，不扫用户文件。


## 2026-11-01 实际恢复与复制新增证据

- R2-006：actual28765真实系统菜单Copy→CmdV/CmdS，小消息14B exact016ac216；giant复制明确失败、38B原sentinel真实粘贴保持378d917d，正常Quit/ALLTMP净。大小原因在WK UI被泛化，最小修已原DOM回归先红59580再整套8组29753绿，尚未最终集成/native提示复验。
- R2-002/007：S A actual42497（83.63s）真实FIRST→精确App-only计划SIGKILL→新App先读FIRST再SECOND，sameSID/run/host/provider/无重复setup断言全到；S B actual17620（69.58s）原roles全退后先真读磁盘history再真实菜单recover一次，sameSID/fullnative/newrun/Resume1/Load0/New不增与earlyNote及prefix全到。input独审11f1aa14。两原整体false仅3已知why，真实业务/正常最终退出/ALL/source均闭；精确冷留存7f450784另证TMP现无，原false不回填。
- R2-003/007 Shell分层：actual87058只读两旧SID的ended状态/1100B screen；actual62253真实冷App两已结束会话旧快照可见，正常Quit/ALLTMP净，无attach或输入。AX辅助Terminal input不是可交互证据，row Idle不是后台仍运行证据。

## 2026-11-01 最终G整合前核账

- f23实际Copy17415/boot69866及终端62032/29379均已非作者尾核，准确边界现已更新上表。旧失败事实保留，不通过后写一段自动覆盖前文。
- Processes88916真实表头重影已用相同fixture绘制77782复现；原脚本/fixture保持，91894候选绿并独审。最小修和两处过期注释已分四小提交整合至a228baef，待最终当前构建和真实Mac表格复验。
- Mac真人IME与physical手机交互仍未验；用户暂不方便，不反复询问或复用旧方便回复开实例。手机当前库原冷编16142超时未完成，不称App/sign/install成功。
