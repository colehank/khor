# 第二轮有限结案独立审查索引

2026-11-01，reviewer：round2_reviewer。源码基准为 `integrate/round2@a228baef4540ac4fe03c40879d4cb50f89da69c4`，核对时 Root worktree clean。本文是供 Root 核账的独立索引，不取代 [34项验收总账](ROUND-2-ACCEPTANCE.md)、[原反馈](ROUND-2.md) 或 [STATE](STATE.md)，不授权 main 合并或发布。

本次没有启动测试、Cargo、浏览器、App、OS probe、安装或下载，没有改产品、STATE、总账或任务卡。实际 UI 来自 Root 的 CUA 记录；reviewer 只读核对文件、来源及尾核收据，不声称亲见界面。前端actual4570、normal3 actual55538及后到的Mac Processes actual93241均已独审通过。初稿按当时live/待验写明的地方已随独立实际结果最小更新；**构建通过不代替业务，93241有单独实际App记录**。

## 审查结论与最小剩余集合

现有源码独审没有留下新的生产源阻断。此前逐片发现的所有权、迟到接纳及真实产品反例，均有精确修复或明确既定 NEXT 处置。这个结论不等于34项全部完成。

最新闭合：a228三normal producer 55538真实尾核通过；原88916 Processes真实Mac红，经77782→91894同几何Chrome反例后，最终同源App93241也真实通过原Showall200/scroll1＋0.35代表。两settled帧标签清晰、自然渐隐/无divider，正常CmdQ/ALL/TMP净；不再重列当前Mac G待验，也不重目录菜单/diff/全主题矩阵。以下是真正剩余：
1. **真人 Mac 中文 IME／Shift选择及组合输入保存** 仍未验，主要归 R2-027；三文件 owner 的 ASCII Save失败／Cancel／最后字符正常Quit已闭，不重列为缺失。用户当前不方便，不复用旧方便答复启动计时实例，不反复询问。
2. **真实手机原范围**：R2-017网站可用视口及 fixed/sticky底栏、R2-018左滑两动作、R2-019窄工作区布局、R2-024手机固定区、R2-025三入口连续候选、R2-026／028软键盘与自定义键行。须实际合格设备／App，不能由桌面、coarse DOM、iOS-sim check或设备库冒通过。它们可沿现有路线组合，不新增物理双机、每glyph或40厂商全安装矩阵。
3. 手机产物前置仍未闭：physical16142首次 cold300只到SDK／metadata0，Cargo剩余294.868s超时，整体303.356s false，无GUI库；原unknown TMP与部分cache保。后续仅离线库producer的共享600预算已独审，App／Node交互300不变；还不等于库完成、Xcode链接、独立Lab签名、安装或设备交互。Root fresh资源和单槽准入后再执行，不为清理或预算绿重跑原业务。

R2-001可交付“限定调查完成、原卡死唯一因果未知／未证修复”；R2-016可交付“当前真实WK路径和错误分类已证、原历史拒绝唯一因果未知”。这符合原要求，**没有额外用户接受unknown的审批前置**，也不借此无限调查。

## 34项逐行核账

“已闭”仅指本行所列原范围和证据，不代表所有平台、所有组合或旧失败结果被改绿。手机／真人部分明确留待验；历史任务卡的早期 source-only/pending 语句以其日期理解，不能覆盖后来的实际结果。

| ID | 当前结论 | 直接复用的证据与源码边界／最小余项 |
| --- | --- | --- |
| R2-001 | 限定调查可交付；原因未知 | [D卡](tasks/R2-D.md)及 `diagnostics/freeze-current-20261030` 有界事实；无旧卡顿瞬间栈，不从H长历史通过推断已修。无新增取样矩阵或审批。 |
| R2-002 | 状态分层有限闭合 | App A42497同host/provider重挂、B17620全退后显式新run；Node81606 hostAbsent/providerPresent拒重复启动；boot69866纯identity。GUI连接／宿主／provider／历史各自证据，不需物理重启Mac。 |
| R2-003 | 导入／PTY归属有限闭合 | [A卡](tasks/R2-A.md)真实native reader与readonly链；87058 ended／无attach控制资格，62253冷App可见两旧1100B screen；不伪造外部PTY或自动接管。 |
| R2-004 | 两个已证锁机制闭合 | [N卡](tasks/R2-N.md) actual15681两FD-fenced currentCLI positive，8父＋2helper；原ID全保、预算与锁序保。原集群唯一原因不外推。 |
| R2-005 | 能力映射与观看链闭合 | A的40预置源码矩阵、managed ACP统一journal与实际声明native reader、A4真实DetailPane/reader五组；能力不是预置名字。原要求不等于40个厂商全安装实测。 |
| R2-006 | 已闭有界读取及出口主例 | Reader28／Find32后增量33、DOM原7＋Range全过；80153巨条184613B完整OS Export/Save；96355／17256原生67MiB／53262记录分页、HEAD搜索、取消；28765小14B系统Copy及17415 giant明确too_large＋38B sentinel保。跨mount bookmark既定NEXT；无WebContent峰值测量，不新增峰值门。 |
| R2-007 | 既定恢复组合有限闭合 | 67MiB Store／旧代拒写、85431组件8父＋2helper、public Load70988／Resume57844／Unsupported4303／Shell9775／Offline63373、hostcrash81606、App A42497/B17620、boot69866。原业务、原overallfalse与精确诊断留存分列；不新增真实模型／物理双机／OS重启矩阵。 |
| R2-008 | GUI＋终端意图主例已闭 | GUI11244＋14739四intent；终端42114／62032实际内部与Chrome／TextEdit／Finder exact目标。纯意图／客户端IO分离，不作网络可达预检。 |
| R2-009 | 软折行原主例已闭 | 29379真实1080→904→1544，四行末／两行首同fullfile；选区不导航、两printf硬行不误拼并紧随软行正控。联合42114／62032和未改pure guards；不是每glyph/appmouse/remote新矩阵。 |
| R2-010 | 原导航要求已闭 | 80119离开Session cwd再Back，14739冷树／右内容与Back清旧内容；推荐未新增工作区捷径，dirty owner不随picker销毁。 |
| R2-011 | 原拖选主例已闭 | 80119真实长路径／URL中段至末端松手保Selected与编辑态，真正外点退出。原整个场末timeout仍false；单个已到行为可独立列证，不冒IME。 |
| R2-012 | 原退出搜索主例已闭 | 80119首次blank click／Escape关闭目录搜索并恢复files，输入／候选内部归属有定向门。该场末timeout未回填。 |
| R2-013 | 桌面宽窄菜单已闭 | Light50409／Dark3807宽三材质；904原红后PathField修复，68689实际宽→窄→宽、候选点击／Return／外点正常Quit。只需复用，不因Processes改动重测菜单矩阵。 |
| R2-014 | 当前导航进度主例已闭 | 原native/pure身份规则＋B60137同一pending→真实CIBOL完成后bar消失，失败／Retry不伪成功。无需新增连续微秒trace。手机范围沿原手机组合，不把桌面当真机。 |
| R2-015 | 桌面主题底色主例已闭 | Dark80153及Light/Translucent90 B60137，首帧／pending／真实closedport失败与Retry，保网站自有底。手机原生组合仍待。 |
| R2-016 | 当前行为已证；历史因果未知 | 80153实际WK原lan访客页可用、真实closedport失败Retry；不是HTTP200代替WK，不宣历史唯一原因已修，不增加用户批准门。 |
| R2-017 | **待真实手机** | 同页／同登录状态的网页frame与甲板几何、fixed/sticky底栏可见可点；源修／桌面都不能销账。 |
| R2-018 | 桌面规则闭；**手机左滑待验** | B60137 Pin/Delete及Unpin/Delete实际专用row删除，另一原URL／页和网站数据保。无需重桌面；待真实触屏两动作。 |
| R2-019 | 桌面布局／冷恢复闭；**窄屏待验** | 80153真实布局、91690常驻Reload移除、14739冷树＋右restoredfile／Back。窄场不要求重71提交流程。 |
| R2-020 | 原commit主例已闭 | 80153当前分支71按40＋31加载，70固定、69/00预览独立、标题正文／展开保。相关组件后续未变，复用实际源映射。 |
| R2-021 | 原单滚动／跳转主例已闭 | 80153真实39变更jump菜单一根内部scroll，跳35后位置保持；无新第二scroll owner。 |
| R2-022 | 桌面diff代表已闭 | 91690深浅／三材质、标题tab无divider、后续file06/07位置保。首次dark→light file05→06原因未知，不虚报像素不动，也不为它扩新矩阵。 |
| R2-023 | 选中工作区实时／草稿主例已闭 | W分层kernel／38远机流／DOM证据＋5794真实同file外写V2、tree建删、privatebranch/path-onlycommit刷新，os13保稿、V3后CAS冲突→显式Save mine。IME归027；通用非工作区浏览实时既定NEXT。 |
| R2-024 | 共享源与桌面diff闭；当前Mac表格已闭；**手机待验** | G局部owner／Machine／TermPeek等逐片证据；88916真实Processes红→同几何77782红→91894 unit15/15及direct glyph0绿。a228最终Mac93241独立实际闭环；手机仍待，未由Chrome或构建冒代。 |
| R2-025 | 引擎／正式DOM闭；**手机待验** | [M025卡](tasks/R2-M025.md)原engine反例→新18及相关门；79769正式三入口coarse DOM五组。真实方向／阈值／循环／回中／松手手感仍需手机，不重pure。 |
| R2-026 | 配置／持久化／隔离闭；**手机键盘待验** | M真实catalog、9组组件／双window／xterm sequence／锁与隔离试按；保存取消／坏态拒绝有原门。软键盘焦点及入口按原要求实测。 |
| R2-027 | **真人系统IME未验** | E accepted/composition／迟到协调及三文件退出定向证据只支持实现；脚本typeText／Paste、ASCII qr不代表真实中文组词与Shift选择。用户当前不便，不重问。 |
| R2-028 | 配置到真实Codex/PTy字节闭；**手机入口待验** | M026两独立草稿、键帽与标准键盘press＋keyup原始字节同／focus同，原oracle误判另保。第三方无选区语义不等于Khor发送失败；不新增vendor选择支持要求。 |
| R2-029 | 原文件公式主例已闭 | 同一marked/Markdown数学链定向代码／转义门，80153文件预览四种定界符实际渲染。无第二renderer，不重数学矩阵。 |
| R2-L01 | 原exact有限闭合 | [S卡](tasks/R2-S.md)与原666 actual：旧retry不早于due、fresh不饿死、reap完成。原040唯一原因未知不改写；不扩四case或全SDK。 |
| R2-L02 | 原两WK关闭交接闭 | 原666真实late-close／close-reopen、旧owner退新owner活、最终handle全空；0f606以后Quit草稿修独审另证，不把两者互相冒代。 |
| R2-L03 | 三实际文件入口已闭；IME另列 | Workspace88179含SettingsCancel、Session81735、Machine33349各自可见接纳稿→Save os13→Quit阻止→Cancel保稿→恢复权限→q保存+r立即Quit，disk exact且ALL/TMP净。无需再重复三owner ASCII。 |
| R2-L04 | 来源机制已闭；**当前a228产物已尾核，手机产物未完成** | 历史V反例／严格生成来源门、f23 normal86710已过；a228 frontend4570与normal55538实际通过并独审。physical16142原false，后续builder600仅准备，不冒App/sign/install。 |
| R2-L05 | 原正常Quit持久宿主闭 | [V卡](tasks/R2-V.md) focused App正常Quit后同shell/host认证，139B旧marker，无Input/Ready，再显式清fixture；完整S A/B另证，不重复原拖放/退出场。 |

## Clean Code：复用独审与当前增量

基础逐领域源码与状态/IO接缝复用 [17f6独审](reviews/R2-CLEAN-CODE-17f6.md)，ghost Child及原生Quit/草稿处置复用 [0f606增量独审](reviews/R2-CLEAN-CODE-0f606.md)。本次再核0f606→a228只有16路径，包含既定G/H/Node源与测试；没有重审main→当前411路径，也没有索引全仓。图谱仍为511adbd的19文件旧覆盖，已明确fallback固定diff与现有逐片审查；不把图谱零flow当当前无影响。

| 领域及责任 | 已核实际边界与原则 | 已有独审／失败语义及剩余成本 |
| --- | --- | --- |
| D 调查 | 进程名不代表身份；有界诊断与产品状态分离，不读用户正文造因果。 | D卡＋17f6报告；原栈缺失诚实unknown，不建常驻watchdog。 |
| A 观看／能力 | DetailPane/ObservedSession只持UI lease/query身份，native fullidentity及 published control资格仍由Node；owner-only变化不卸载正文，迟到A不落B。 | A卡A4五组及能力矩阵；readonly不授control，所有adapter不是一套假native parser；不为40预置新增安装框架。 |
| S 恢复／进程 | `live/recovery_attempt.rs`单bounded文档分execution/latest action与高水位；exact payload replay先于新执行资格；`session_recovery.rs`在同物理Journal File与短RowGate重核，握手/OS采样在gate外。JoinPending不可重试成Native；三alias不能绕Unknown。 | 17f6及host-crash ownership-fix独审（443bb973→fb01）；`replacement_confirmed`只Exited/ForegroundAbsent（原NotStarted/Refused特例保），只有typed OwnershipBlocked变Unknown，IO/损坏仍原Err；81606真实反例红绿支持。无新registry/通用close重构。 |
| S Child／UI投递 | `gui_host.rs`私有waiter持Command→Child→wait；单PID结果，timeout为Unconfirmed，不重复spawn；UI offer按row revision退休，但已提交action仅target/mount/action generation退休。IDB完成事务后才RPC。 | 874c exact15850及Unsupported4303；Recovery17 actual13057；AppA/B actual强接缝。ECHILD只说同ghost已reap，不发provider退出authority。长private OS fixture有真实角色/HTTP/退出义务，不为DRY抽通用执行框架。 |
| N 并发 | `doc_transfer_state/io.rs`复用同一余额；等待metadata前卸root，拿metadata后root仅try，反向busy先drop metadata，最终gate重读恢复事实。构造/注册副作用不重放，不持metadata等待root。 | N95acd/actual15681双FD正例及6IO门；typed Busy到期仍Busy，未吞错返回空列表。 |
| H 单IO／完整性 | source/reader/worker分别持lease、单请求及Abort；paint唯一增量fold。query replay不重计coverage，wholecopy绑定真实贡献/physical ceiling；raw边界用同Core projection `starts_message`，未知SDK/partial仍Incomplete。 | 旧raw14B真红→AA Find33；`projectRecord`机械复用scaffold/投影，不建第二parser或decode整全文；64KiB cap、source/cancel/reset守卫保。 |
| H 系统Copy错误 | `HistorySearch`当WK把body拒绝包为generic clipboard error时等待**同一body Promise**找回typed inspect原因，迟到只由同owner投递；ClipboardItem仍即时用户手势创建。 | 38171原红、49554smallIncomplete/giantTooLarge诊断、AA小14B实际成功、f23 DOM8组/17415明确too_large；无Mac新clipboard API、未知权限fallback或剪贴板读取服务。 |
| E 文本／退出 | FileBody单accepted {file,edit,clashes}，React只投影；same machine/path及revision拒迟到。draft集合同步发布、卸载只清自身；merge/clash/CAS与显式Save mine仍原owner。原生menu只发main.close，aux收尾等Destroyed。 | E20回调/同步/刷新门＋0f606三文件真实退出，44266真实业务红不被旧pure绿抹掉。真IME仍是证据缺口，非再造global正文store的理由。 |
| C 意图／坐标／注释 | `content-link-intent`只纯决策；`client-opener`独持客户端IO；term-links保xterm logicalcell映射与硬行边界，selection/button/application-mouse门不消失。 | C卡软折行CJK反例、C009最终有限原生场及pure guards。a228的TerminalPane两处过期⌘click类比已单7ad4454e删除/更正，只保为何不走xterm默认confirm/window.open；图片行为无改，无全仓注释治理。 |
| B 导航／错误 | `page_navigation`发真实view/navigation事实，loading rule按issued revision接纳，终态不复活，呈现timer不生成native成功；网页自身底色与host背景分别归属。 | B卡及60137/80153真实WK；原错误unknown不字符串猜原因，不HTTP探测兜底。手机viewport仍需原设备验证。 |
| W 当前文档／监听 | tabs按path/commit ID保存preview/pinned，watch单physical cohort/lease、有界scope与单latest intent；先watch再枚举，plan/revision交接不丢事件。foreign/迟到结果不覆盖稿；非text preview独立单请求，原text accepted未复制。 | W分层负红/正例、38流CLI/DOM＋5794实际外写/Git/CAS；不为按钮建立第二刷新状态。90f804仅wsTree初始化跟place.ws，既有Back/open owner不重构。 |
| G 材质／几何 | scroll-edge WeakMap first owner，RO/RAF/属性由同owner清理；PathField仅overlay宽度clamp到pane/visual viewport，销毁释放观察，不碰输入/IME。当前Processes只held+flush+data-top-heads推透明起点至居中label下沿，恢复点仍header.bottom/CSS原渐变。 | d4b窄菜单68689、a228 source＋91894独审c83f7d1f。未加实底/硬clip/divider/theme token，test只隔离blur诊断，不要求真玻璃所有色差为零。Mac原表格已由后到93241实际闭合，手机另待。 |
| M 手势／键行 | deck phase/clock owner单一，release走cancel；配置解析与执行编码分层，试按零真实写，custom不叠锁；手机编辑TaskOverlay不与键盘viewport互相销毁。 | M/M025卡的真实producer/组件/PTY字节及DOM五组；代码/字节等价不冒vendor选择或物理触觉。保原设备待验，不增加新输入框架。 |
| Q 数学 | 复用现marked块/inline raw保护代码区域，普通/流式同一数学步骤，未知raw保原文。 | 029卡原反例＋80153四公式；无第二lexer/renderer/依赖或全CommonMark新承诺。 |
| V 来源／测试与资源 | 原`native_provenance`唯一V门，managed gen/fulltypes/Vite/normal CargoJSON来源分列；CLI和Codex测试相似prefix-pump分别属不同测试binary的独立receiver，保局部helper比扩产品框架更明确。 | 实际生成28762六阶段／后继frontend、Core44421、C59004、后续normal链及当前4570；旧failure不回填。builder600仅独立库模块参数，原helper/App/Node300与normal900不动；ALL4096/单job/offline/50cap/空间门保。 |

### 已知未解决项的处置

- 17f6报告 **F1**：成功Ready文件被parent消费删除后，ghost退出仍可留“未Ready” `.why`。这是既定NEXT诊断生命周期问题；成功业务与exact诊断冷留存分列，不把文件存在当运行活跃，也不把postrun收口改成原overall green。
- **F2**：通用close尚未消费新Attempt full birth/run authority，原已明确NEXT；新恢复/Child waiter不能替它背书，本次不扩通用shutdown专项。
- 跨session卸载后阅读点回tail／轻bookmark是既定NEXT，不把同实例Range anchor冒跨mount保证。LongHistory旧blank capture唯一paint因果仍未定，后续raise/真实指针可读事实分列，不猜WK根因。
- 本轮已改边界未发现必须再开产品改造的新问题。现存live/gui_host/TerminalPane大文件和局部测试重复是维护成本，不以行数或“零重复”为由新增服务/registry/全仓注释清理。若随后发现独立新问题，仅按既定NEXT登记，不增加本轮门禁。

## 精确证据与交付索引

下面均为 `/Volumes/disk0/Khor/evidence/round2` 下既有证据，完整hash以所链收据为准；未把早期pending文案当当前事实。

| 证据束 | 位置与关键指纹 | 本次可复用范围 |
| --- | --- | --- |
| 跨包逐片独审 | [R2-REVIEW](tasks/R2-REVIEW.md)、[17f6](reviews/R2-CLEAN-CODE-17f6.md)、[0f606](reviews/R2-CLEAN-CODE-0f606.md)及各包卡 | 单owner／IO／迟到／清理／错误的原细节；报告里的当时平台pending已由后续实际逐项补，不覆写旧报告。 |
| S完整App与诊断 | `history/session-recovery/full-app-current-source-3`；input actual独审11f1aa14；exact postrun retain7f450784 | A42497/B17620业务强断言、最终normalQuit/ALL；原A1462bfe1/B3e0aafa8仍false，不冒原strict整体绿。 |
| C009最终两场 | [62032/29379独审](/Volumes/disk0/Khor/evidence/round2/review/c009-62032-29379-readonly-review.json)，SHA2a615088…6085 | 两finite native场＋旧42114普通动作与pure guards，exact目标/软行/硬行/选区与正向reap；不需新增全glyph矩阵。 |
| Copy及boot | [17415/69866独审](/Volumes/disk0/Khor/evidence/round2/review/copy17415-boot69866-readonly-review.json)，SHA047c3586…bca4 | f23 giant typed提示与实际38B保留；一个原regular boot/name/group纯exact1/0/0、no-prior实际来源及copy退役，不冒OS重启。 |
| G同几何绘制 | [91894独审](/Volumes/disk0/Khor/evidence/round2/history/sticky-processes-paint-green-1/review.json)，SHAc83f7d1f…70be | 原77782 direct3165/max87→0/0，blur5249/max41→0/0，geometry/脚本a789/fixture4dd相同；unit15/15、ALL/TMP净。仅受控Chrome；Mac缺口由下一行独立实际收据补齐。 |
| G最终Mac实际 | [93241独审](/Volumes/disk0/Khor/evidence/round2/review/a228-93241-G-native-readonly-review.json)，result91e6205b/UI48e75459 | 原两硬链stage59892成功，同一App36364898/GUI365cfdc9，原22748出生身份、正常CmdQ/source/env/ALL/TMP净；Root真CUA与保存双settled图支持原88916表头代表绿。非reviewer现场操作，不重矩阵。 |
| 当前前端/留存/入口 | [4570及留存独审](/Volumes/disk0/Khor/evidence/round2/review/a228-4570-frontend-retention-App-source-review.json)，SHAe9194f8d…7d4e；receipt6a4200fb | 当前2340源/188gen/278新assets、fulltypes/Vite0、日志/环境/ALL/TMP；原OUT283/6603311B精确留存809b7044/合同a831。source_absent仅rename时事实。该前端/留存收据本身未推55538绿；55538后到终态由下一行独立尾核。 |
| 当前normal3实际 | [55538独审](/Volumes/disk0/Khor/evidence/round2/review/a228-55538-normal3-App-pins-readonly-review.json)，result a5129794 | 五phase0/274.749s，三normal各no-prior/currentV；CLI09cdf574/bridgee60c7614/GUI365cfdc9，原bundle26及新OUT283完整来源、2340源/188gen278assets稳、ALL/TMP净；stage/scene实际JSON与22producer环境绑同，未执行App。 |
| 当前source准备 | [前端/normal源独审](/Volumes/disk0/Khor/evidence/round2/review/a228-frontend-normal3-source-review.json)，SHA826d9492…691a | 正常三producer算法与原预算不变；App原stage2ec38227、Processesd37c37c4仅有限来源/namespace差异。actual新role/proof/环境绑定后Root另授。 |
| iOS-sim与physical库 | `mobile/ios-simulator-check-prepared-3/compile-and-cold-retention.json` SHA503dbc81；`mobile/physical-library-f23-prepared-2/result.json` SHAbd3d8539 | sim Cargo check业务0但原overallfalse/TMP冷保；physical原cold300未完成，不混库/App/签名/触屏。 |

后续库准备 `mobile/physical-library-g-followup-prepared-3` 为reviewer作者，已由input非作者独审3ee3e970通过。driver `97703e4520570f01025fe38189356c9eaa5111c67732808173e3c43d946417f7` 未变；Root仅JSON绑定当前a228/actual4570，inputs `4c096cc4d768bc9032666e64098322c0bd3d0330451f5231db8b68a7236ff0ee`、contract `4a5b0ad63dd245835a764f7fa7d82dda2cc2ad6ce639e2eb37b295a979afc6bc`，完整source/gen/assets dict与真实前端相等，**readyfalse/authnull**。只构建库共享600含前后核验、每phase取剩余，原helper复原300；可复用先前partialcache，不能宣称新测fullcold600或原cold300通过。Root在当前native/G App后自行freshdu＋8<=50GiB、外10/内2峰值＋20/10保留量与唯一槽准入，reviewer不执行。

本索引只收敛原范围与已证复用，不按任意百分比评价进度。Root实际结果到来时仍须分开记录“构建来源”“真实业务”“退出/诊断收口”和“未验平台”；不存在以新报告覆盖旧false、以准备代执行或以内部审查代真人的捷径。

手机App计划 `mobile/physical-current-a228-preparation-1` 已作非作者source-only核查（[小收据](/Volumes/disk0/Khor/evidence/round2/review/physical-a228-App-plan-source-review.json)）：29小输入/唯一App target/main.mm/现Info用途及私有identity清楚，无ShellScript/remoteSwiftPackage或隐式Cargo。库/.a/App/签名实际字段仍空，readyfalse；外3/内2是待actual.a大小再核的显式提案，非已测峰值。没有因此新增设备或签名治理专项，也未把profile元数据冒实际签名或设备可用。
