# 第二轮验收总账

本页是34项的验收索引，不是完成声明。最新源码/所有权/运行槽见[STATE](STATE.md)，用户决定见[原反馈](ROUND-2.md)及专项文档。编表基线为4f7e732e，最新冻结HEAD见STATE；没有当前统一App的实际通过证据。源码实现、pure/DOM、Rust定向测试、真实WK、真人IME与手机触屏各自证明自己的范围。

## 逐项最小完成证据

| ID | 包 | 用户目标 | 必须观察的结果 | 当前仍需注意 |
| --- | --- | --- | --- | --- |
| R2-001 | D | 旧卡死调查 | 复现时留下确切GUI/WK身份、栈及有界资源/IPC时间线；若无法复现，保留未知，不能用H通过替代。 | 限定调查已留证；原现场缺失，仍未证修复。 |
| R2-002 | S | 状态分层 | 分别验证GUI连接、终端宿主、agent执行和历史恢复状态；GUI退出不把独立任务误报为已停止。 | 完整宿主退出/恢复组合待测。 |
| R2-003 | A/S | 导入与PTY归属 | 外部导入显示实际能力与可读历史；无原PTY时不伪造可交互画面、不自动启动第二writer。 | 能力/有界链已有定向证据；当前App待测。 |
| R2-004 | N | 状态查询并发 | 当前CLI在真实root/meta锁竞争后成功查询，原ID保留；try/等待预算及锁序仍成立。 | 8父/2helper实际通过且6源整合；不外推原集群唯一根因。 |
| R2-005 | A | 全部已接入ACP观看 | 按每个adapter实际能力提供正文/只读/控制入口；外部观察不接管。 | 源码能力矩阵与定向链通过；不是40个已安装agent实测。 |
| R2-006 | H | 长历史 | 有界分页/渲染/缓存/传输；可取消并保持阅读锚点；搜索、复制或原文出口不把截断冒成完整。 | 基础读取/出口与既有browser已通过；真实chunk分组、读取意图及最新思考定位已修；当前28项reader、完整32项Find worker在864实际通过，Reader真实28过但caller误计29的原false保留；跨页query/wholeplain-copy已实现，完整DOM消费者actual28700原7＋全部Range真过；实际17f默认App80153原生1201记录及184,613B巨条完整Export→系统Save逐字相同；96355与17256实际67,110,120B/53,262条原生历史分页、未加载HEAD全文搜索并定位、取消、WindowRaise/指针返回与滚轮可读。原paint不足现象因果未知；跨mount阅读bookmark不属本轮，仅NEXT。尚无系统Copy与真实WebContent峰值证明，不把parent单点RSS冒峰值。 |
| R2-007 | S | 统一持久化恢复 | 同一会话身份恢复canonical正文；存活重挂、已退任务和离线明确；不重复load/writer；长合法base无丢失且启动内存受控。 | Store真实67,174,400B payload及有界启动已通过；运行代次拒写5exact已实际通过；前置reservation、Load新display base与Resume实际Note的8项组件验证已通过；真实New/Attempt及known-native生产组件原8父＋2helper已54318/14117通过；公开显式恢复/新Shell源码与当前Recovery17组件已接并通过受控UI验证；真实Node当前85431原8父＋2helper完整通过并独审，真实CLI/Core公开Load70988业务断言全到且双审：Join与重放无重复writer、退出后同SID新run、Load1/New不增、旧binding仍读；整场false仅两个自有诊断，另据31a4c020收口，不回填。真实Resume57844业务全到（同native新run/Resume1/Load0/New不增/原opaque输入与旧record/raw保）；其已知诊断另收口原false保。当前874c完整Unsupported4303实际0/6.260s，原5s两次host/provider全部退出、replay无额外启动、fresh recheck Init-only Refused、最终Init4/New1/Load0/Resume0/Prompt1、无forced/TMP净；独立尾核已通过。Shell9775真实0/8.756s＋独审，原A仅Ready后执行一次、DirectoryRequired无spawn、用户新cwd建立唯一默认Shell B、同payload live/ended均同B、旧A保持及两PTY实际回收。两同机真实privatehome Offline4 actual63373完整离线无本机执行/Unknown、samekey重开后原payload Join原SID/run/host/provider已通过并独审；不外推物理双机。Host死亡但provider仍活已在43ff实际35829复现业务红：hostAbsent/provider精确Present，Unknown成立但actionNative不满足None；后续显式恢复/replay断言未到。原false与冷现场保，最小资格门修复及完整App恢复组合未完。 |
| R2-008 | C | 打开意图 | 同一路径/URL普通点击内部，Cmd点击系统；不作可达性预检或额外改写。 | 实现/定向检查已有；当前App点击与Cmd组合待测。 |
| R2-009 | C | 软折行路径 | 同一生产终端入口长路径在宽/窄屏均可识别并命中，网址对照；不误把硬换行拼为同一路径。 | 实现/定向检查已有；当前App窄屏待测。 |
| R2-010 | C | 回当前工作区 | 离开session cwd后左上动作回当前工作区，不在推荐列表追加同类项。 | 0f606实际80119已从EditorSession进入EditorMachine，再用Back回EditorSession；源相关组件未变。该批末300超时原false保，不冒整场通过。 |
| R2-011 | E | 路径/URL拖选 | 从中间拖至末尾松手保持选区和编辑态；真正外点正常退出。 | 0f606实际80119：长Directory path与长URL都从中段拖至真实末尾/框外松手，实际Selected尾段保留且仍编辑；真正外点才结束。该批末300超时原false保，Root原生事件不冒真人IME。 |
| R2-012 | E | 退出搜索 | 首次外点或Escape退出药丸搜索，过滤不残留；输入/候选内部操作保留。 | 0f606实际80119：不存在查询→Nothing here，首次blank click或Escape均关闭目录搜索并恢复原files；该批末300超时原false保，个别已到行为单列。 |
| R2-013 | G | 推荐菜单材质 | 工作区建议可读、有足够底色和单一叠层；浏览网页仍不显示推荐框。 | 当前App深浅/三材质待测。 |
| R2-014 | B | 真实加载进度 | 同导航进度单调，真正成功后结束；失败/取消不伪100；非WK能力不伪造比例。 | 规则/原生定向证据已有；真实WK事件待测。 |
| R2-015 | B | 网页初始背景 | 首帧、加载、失败/重试随主题底色，网站自有背景仍保留。 | 真实WK深浅与重试待测。 |
| R2-016 | B | 连接拒绝 | 采真实WK错误/导航身份/访问运行路径；区分失败，不能用独立HTTP200宣称修复。 | 当前App80153真实WK已加载原lan.cibol.khor.cloud/guest访客页面；闭端口56757显示连接失败+Retry。证明当前路径行为，原历史拒绝唯一原因仍未知。 |
| R2-017 | B | 手机网页视口 | 同一网站/登录状态对照，fixed/sticky底栏可见可点，网页frame不进入Khor甲板操作区。 | 真实手机同页几何/点击待测。 |
| R2-018 | B | 浏览器左滑 | 置顶、删除都可操作，置顶项可按决定关闭；不清网站数据或其它标签。 | 定向检查已有；当前App/手机手势待测。 |
| R2-019 | W | 工作区最终布局 | 未选右空白；选后头/机器头像:路径/branch+commit/搜索新建与右顶独立玻璃符合已验Lab。 | 正式接线已有；当前App桌面/窄屏待测。 |
| R2-020 | W | 独立commit标签 | 当前分支完整历史按需加载，左栏原位返回文件；单击预览/双击固定，各commit独立，标题正文/展开/阅读位置保持。 | 后端/DOM分层已过；当前App连续导航待测。 |
| R2-021 | W | 菜单单滚动 | 长变更列表只一根滚动条，跳转保持焦点和阅读位置。 | 当前App真实长列表待测。 |
| R2-022 | G/W | diff固定标题 | 标题与标签栏一体，无细分割线；滚动时正文自然隐去，标题不被穿透。 | 当前App深浅/三材质/长diff待测。 |
| R2-023 | W/E | 实时更新与草稿 | 树、当前文件和Git外部变化及时更新；组词/脏草稿不被覆盖，迟到结果按身份拒收。 | watch分层Core/CLI/DOM已过；当前App实际文件变化待测。 |
| R2-024 | G | 共用固定区域材质 | 统一连续渐隐；最上正文完全隐藏，窗口底按不透明/半透明/毛玻璃独立变化，阅读位置稳定。 | 共用实现已有；当前App滚动/主题组合待测。 |
| R2-025 | M | 候选连续跟手 | 三入口方向一致，连续拖动/循环/回中暂停/松手吸附符合参考Lab，触屏不提前直跳。 | 生产DOM/共用引擎已过；真手机手感待测。 |
| R2-026 | M | 自定义键行 | 编辑、保存/重开组合键及序列；焦点/修饰键/隔离试按正确，使用既有键行外观。 | pure/UI/真实PTY已有证据；手机软键盘待测。 |
| R2-027 | E | Mac中文组词 | 真实系统IME组词不丢拼写；外部/迟到更新不覆盖composition与草稿，最后输入可保存。 | 定向逻辑/专用程序已准备；真实IME仍待。 |
| R2-028 | M | Codex组合键配置 | 自定义Shift+Left保存后真实TerminalPane/PTy发送与常规键盘事件路径一致，独立草稿保持；不冒vendor具有选区能力。 | 真实隔离Codex/PTy完整press+keyup与常规键盘事件路径同，原oracle误判另保；手机入口待测。 |
| R2-029 | Q | Markdown括号公式 | 文件预览同时支持\(…\)、\[…\]及原美元分隔符；代码/转义不误渲染。 | 同一数学链/代码与转义定向检查已过；当前默认App80153实际文件预览四种分隔符全部渲染，normal CuaQuit0及独立盘尾核通过。 |
| R2-L01 | S | 取消retry deadline | 原ignored exact不改时限/断言；真实匹配CLI worker启动不早于due，fresh run不饿死，进程均reap。 | 当前666 Node actual producer＋原exact实际1/0、fresh不饿死/旧retry不早于due；原040唯一原因未知。 |
| R2-L02 | S/B | 原生关闭交接 | 原late-close与close-reopen两失败case实际WK通过；旧owner退出、新owner保活、最终handle全空。 | 666真实GUI原两case及正常CUA退出/全清理已通过并独审；首收尾超时仍未知，不扩原四case。 |
| R2-L03 | E | 所有入口草稿退出 | workspace、session内文件与机器文件同一保护规则；保存失败可取消退出，最后一键保存后正常OS Quit。 | 共用owner已有定向证据；当前0f606真实Workspace88179（含SettingsCancel）、Session81735、Machine33349各自Savefailure/Cancel/恢复权限/最后字符立即Quit已通过并独审，正常退出及TMP净；仅Root ASCII，真实系统IME/真人交互另列R2-027。 |
| R2-L04 | V | 原生构建来源 | 实际当前GUI/CLI来源完整、环境/ACL目录/生成文件规则守住反例；旧失败不回填。 | 既有实际native批通过；新当前App仍须用同工具真实产出。 |
| R2-L05 | V | 正常退出与持久宿主 | 实际正常OS Quit后同私有host认证重连并读旧marker，随后显式清理自有fixture，无信号冒正常退出。 | focused真实App/重连/清理已通过并独审；不冒完整S恢复或E输入。 |

## 实际执行顺序

当前Root与实际运行以STATE为准；以下17f6afb0为历史执行基线。Known生产组件actual14117原8父＋2helper真过并独审；当前Find32独立真过，Reader实际28过、caller误计29原false保并独审裁定。Range保留字点红基线真复现，e122修复已源审整合；完整H原59083 false保；修正目标IO夹具后actual28700默认8组（原7＋Range全部）真过并独审，受控Chrome不冒WK/OS出口。ACP9255 default缺Turning事实已判夹具，e211补帧后actual38247完整suite已通过且独审（当前Rust/Core/平台不外推）；公开恢复Core/Node/GUI与Shell并行接线，真实平台未完。Source/实际/scope均以STATE为准。

1. H完成真实来源驱动的chunk分组及必要raw/copy/分页消费；S按短row gate与RunKey→provider前reservation→Load/Resume事务→明确恢复/新Shell接线串行关闭既定缺口。两作者分配不重叠源码窗口，固定小片经非作者审后整合。
2. 接口稳定后一次实际生成/catalog、noEmit/Vite、受影响consumer；再C/Core定向原生，最后同源CLI/GUI生产构建。旧16ee合同不能直接当337当前准入；未受影响的既有Store/A/B与关闭用例不因无关改动全重跑。
3. 私有工作区/测试文件完成桌面W/C/G/Q/B组合：导航、实时更新、路径拖选、长diff/菜单、公式和真实WK。每个结果标明实际入口/主题/材质/宽度，未测的组合保留待测。
4. 完整S验证存活重挂、host退出/重启、远端断线、能力驱动显式恢复、旧代次拒写与新Shell不重放旧命令；NoLoad正常Quit重开草案尚未执行，不能替代全S。仅私有fixture，不操作真实用户会话或调用真实agent/API。
5. 新鲜真人可用性后才开限时Mac IME/路径选区/各入口草稿/保存失败取消及最后一键→正常Quit。手机另验同页viewport、连续候选与组合键行；模拟不能替代真实触屏。准备匹配当前observer devUrl GUI及physical私有Lab后才约人。
6. 逐项精确证据与最终完成审计。R2-001/016若仍无原现场/当前复现则保未证修复，只有用户明确接受延期才调整范围。新独立问题仅NEXT-ROUND。

每包仍须有非作者Clean Code审查：领域名称、唯一状态owner、IO与纯规则边界、取消/失败清理、迟到结果身份、删除被替代分支，以及只解释必要原因的注释。不得以零注释、格式化数量或新增测试数量当完成目标。
