# feat/net 当前状态与恢复入口

更新：2026-11-10 20:18 UTC（新加坡11月11日凌晨）。本页记录当前状态，完整此前段落逐字保存在[历史](HISTORY-20261111-after-scope55.md)，其中链接到[上一历史](HISTORY-20261111-before-current-state-compaction.md)及更早记录。失败、旧源、fixture与审计不覆盖、不删。历史中的运行句柄不当当前。

## 授权与工作位置

完整**N0–N5尚未完成，用户已要求持续实施**；不得因分项绿、compact、状态问句或旧goal服务paused停工。root串行，无子智能体。速度/体验优先，个人可信设备网；peer relay、IPv4/IPv6随时变址为硬范围。没有main合并、发布、采购、生产部署或改已装App/config/store/resident的授权。

- 产品：`/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；先核HEAD/status/diff，不按摘要覆盖WIP。
- 控制：`/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，固定`68cadb57a0ed72d40a4ca9a843e86b9c6797bf34`；两处既有untracked `__pycache__`保留。
- N1隔离：`/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines`，detached `2ef6bee5a5aaf841af066f4edb5c894769d4539d`，clean，未接受。Iroh42–44在该树预留、未合主，不覆盖它。
- 证据：`/Volumes/disk0/Khor/evidence/net`，ops在其`ops/`；`source-refs.json`单写者，实际receipt/ref/tree/control优先。
- 完整范围：[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)、[NEXT-DESIGN](NEXT-DESIGN.md)、[REPORT](REPORT.md)、[VENDOR-AUDIT](VENDOR-AUDIT.md)、[审阅](REVIEW-20261111.md)。

## 当前候选窄范围：1c847229（N1仍待物理验收）

1c8472299b1c94e2c7813d4356d0412cf34a8fc4 / tree90f62f507b7546cf0a2eb3313db602238671d928 / snapshot4c18fa477f91b913726069f661dd265c06c2d1fb。

- vendor5c2e4ff2：Noq439/path55/home11，45.781秒/14files；旧eec09ec1 path53/2实际红，新whole五合同及旧50尾相同。
- LinuxNode dc23e338：315.188秒/21files/group空/identity；Mac42892759864 exactartifact13799886372（zip30923c27…）/20files/11control；同canonical全12组，hop55/wires16/budget30Linux26Mac/resume1。
- 新Iroh46只允许已知路的首次有限恢复quality窗口在真实新sample重返pre-fault实测水平时刷新；历史不作新样本/切换证据。原3probe/250ms/2秒、gain/Moving、packet epoch及重hold不改。主vendor55含候选46，42–44仍隔离。仅单元/便携scope，不宣告N1或手机修复。
- 当前唯一重型physical run6/pair03 baseline：**turing-20261110T204400Z-2135d831**，PID2873797，20:44:21 UTC启动，精确c775/treef21b/snapshot7ca590/control68cad。前五次均采核；run5 candidate6aa可用124/124但warm52/52坏/双端无Go，不能验收。此次为预定第六次，结束后固定集合即止，不盲加第七轮。Mac重型结束，计时中不派其他重型。
- 三轮旧main55固定集合仍2绿1红，详下节；诊断55d1为独立机制观察，不并成第四轮验收。物理对照还没结果。

## 最新已验证分项：923baaeb

**`923baaeb6a6e3b5d7bfe49e17ff28330ae89d29b`**，tree **`e8893d457afe920ad6852f84fb280780f233a111`**，snapshot **`37d77b42a8ea001002ed3fec8a2d09436a656d74`**。

- Linux `turing-20261110T191324Z-828ae604`：全12组/hop55/wires16/budget30/resume1；315.098秒、21files、group空、identity及全部实际summaries核。
- Mac `42883511676`：同canonical全12组/hop55/wires16/budget26/resume1；exact artifact **13794773913**、20files、11control blobs核，目录`mac-42883511676-hop-epoch-packet-age-v1`。已终态success，重型已结束。
- 新窄范围：MISSED只对原routing-wait epoch有效。Wires唯一持有Arc<()>，操作/cache仅Weak；原forgive_all替换token；open_hop首await前捕获，MISSED同锁校验活token且只清同epoch。无变化仍按原30/60/…/600秒退避。每(me,to)仍一项，不按换网增长keys，不留Node活；不得让读取者clone strong来延长旧epoch。
- 另闭合incoming packet年龄：使用当前selected IP/Relay真实Noq `last_received`与原Patience；Unknown/Custom/缺timestamp不猜。真实握手/echo后显式now/future快照年龄对照通过，旧计数假新鲜红保留。**这是年龄模型，不是实际OS/iPhone暂停证明**；睡眠时钟/网络路径世代仍待N4。
- hop-miss与packet-age两个freeze仅scope true/field false。旧98b997de为hop53/2红（网络前/后NO_ROUTE均挡新网、无变化控制过）；旧7bb0e664为hop55绿/wires15/1红（旧包被计数当新）。格式31ca/713f/bd43全部原样保留，无行为结论。
- 相对489477的图谱detect8files/24entities/22gaps、flows0，build落后；定向diff/真实tests为依据。egress出口状态变化也调用forgive_all，已核它不是每个ACK失效。只关闭这两个scope，LOST/peer与middle逻辑地址context、所有入站admission、N1物理时限/N3/N4/N5未闭合。

此前已接受窄范围与准确红绿源均在历史和freeze：

- 48947721：同源Linux95f8da6b/Mac42878663761全12/hop52，exact13792758066；unused prepared carrier无owner释放、pending期间保留/结束释放、真实incoming-only首腿建立新Tunnel及carried TLS/echo、closed负控。
- 801dff9e：同源Linuxaa81601a/Mac42873590614全12/hop48，exact13790762470；initial Drop与normal watcher撤回含新有限pending消费者，保留两个无消费者释放控制。
- 153263c2：同源Linux8adb26d3/Mac42869844599全12/hop44，exact13790105312；首次错误身份旧IP拒绝后新peer relay在原2秒caller内Sessions一次，NO_ROUTE无业务请求、live wire拒绝hint撤回。vendor1e1264ef Noq439/path50/home11绿。
- 主vendor总54（Iroh1–41/45、Noq1–12），42–44隔离未合主；45只在无live连接且旧来源时间<=首次失败边界撤旧IP，新来源/Relay/Custom保留，不是完整地址epoch或永久blacklist。923相对此vendor代码未再变，无无因重复vendor运行。
- 更早sole-peer Recovery准入、ordinary与cold/resumed读取并行、pending lease/cancelled watcher successor、live胜旧cached排序、取消/响应owner/STOP/contact/actor conditional carried-close等按各freeze保留，不能升级成全部入口/代际安全。

## 当前执行状态与下一步

三轮固定集合已全部终态、采集并核identity/group空，当前没有重型任务。三轮准确复用923 source/control68cad/原profile、20ms腿、两次10秒hold及124业务请求，无第四次同源盲重跑。

| Turing运行 | 业务 | 原四秒warm合同 | 内层Relay→Carried AB/BA |
| --- | --- | --- | --- |
| 192533Z-0aeceb60 | 124/124 | 50/50通过 | 3450.867/3667.913ms |
| 193252Z-1cb99869 | 124/124 | 51/51通过 | 3665.036/3373.595ms |
| 193636Z-b158e418 | 124/124 | 47/51通过、4失败 | **5514.252**/3668.365ms |

**N1物理四秒重复性仍红**；372业务成功不替代选路时限。原固定方案及完整结果见证据根main55-peer-mobility-repeatability-{plan,result}.json，各run的peer-selection-contract/peer-decision-contract与校准保留。第三轮168.332秒/16files，首AB/BA恢复后RPC870273/808771us，不能当故障起点恢复或p99。没有丢弃失败或放宽四秒。

当前先诊断第三轮：慢方向在+1514ms已清silence，rehold=false/immediate=true；+3513ms才开始Moving，+5514ms Go。quality_since始终None，无quality-window restart。已排除重复故障三秒hold和该quality重启机制。多copy近期样本数决定representative，约99.688ms与Relay131.830ms刚好不达25%收益，另copy85.178ms达标；是否因样本过期后代表copy改变仍为待证假设，不当结论。证据main55-red-rehold-correlation.json及第三轮outer-copy-timeline-replug-reference.json保留。

后续先建立准确诊断/合同再决定修正，不盲合隔离42–44。原netlab-dynamic/stability保持待做，计时任务无Mac重型重叠。其他N2候选/地址context、incoming admission、静默黑洞、accepted jobs/late losers继续按NEXT-DESIGN，不能以分项绿停工。

## 全计划仍欠的完成条件

| 包 | 未闭合 |
| --- | --- |
| N0 | 最终体验SLO/容差/采样量与前瞻paired校准、足量p99，失败/超时不删分布 |
| N1 | 原物理恢复/quiet成本/全矩阵；隔离2ef bf6105dd虽124/124仍5bad，X5505.782/Y3496.330ms；quality确认窗重启及cold start红保留。main55固定三轮2绿1红，最慢5514.252ms，不称通过 |
| N2 | 上述更多admission、所有地址/network contexts、全部入口ownership；当前仅各自实测scope |
| N3 | Custom1024frames/256KiB公平性，SSH/hop混合、shared/separate 1/3/5，交互/CPU/字节/唤醒与手机能耗 |
| N4 | 实际两腿Relay/唯一middle/断腿/多middle，独立v4/v6/同时变址、Wi-Fi↔蜂窝与peer/middle移动，手机自动恢复和拓扑验真 |
| N5 | 原53+45、隔离42–44逐项审计/上游退出，Noq1/5缺口，最终Quality/native/proxy/多设备/手机 |

独立原失败保持未解释：pair core transfer Busy854aa6ce、原resume32 timeout、N1物理四秒/quiet/cold-start。后来的分项绿不自动解释或关闭。

## 真机、构建与证据边界

MacLab与Turing同LAN；构建macserver为另一台，异网来自用户报告、尚非独立NAT证据。iPhone localhost旧现场Wi-Fi→蜂窝→Wi-Fi自动恢复失败，反复关开App某次才经Turing恢复；未证明OS force-quit。截图IMG_5608蜂窝5GA下hinton Direct29ms、多peer不通，只证首腿可见，不证第二腿、当时源码或精确故障时长。

**旧手机采样已结束，候选未安装，现在不做phone RPC/sampler、不要求亮屏/切网。** 原79样本47ok/22busy/9paused/1error不作候选SLO。REMOTE-DEVELOPMENT明确便携Mac网络executor不是native检查，开发机不生成正式iOS签名包，iPhone用可信GitHub TestFlight；没有为测试修改该边界或进行发布。当前网络unit绿不等于App产物，后续原生/可信Quality/分发仍独立。

本机source-only：不跑Cargo/Rust/Node/Xcode构建/测试/formatter/下载执行。Git/Python标准库元数据核可用。Turing只走既有UID1110/flock/user+netns/build admission，Rust1.95/jobs16/nice10、50GiB起40停/90min；无sudo/host网络变更。Mac原单cache/jobs2/50–40GiB/100GiB guard不改。无重叠Turing重型；普通unit可Mac+Linux并行，计时netlab不与Mac重型重叠。SSH不转发agent/凭据，无子智能体、无CUA。

图谱先行，仅导航；旧HEAD、flows0、tests_for0不是安全/覆盖证明。避免宽读crates/net/src/endpoint.rs顶部编译relay bearer token。collector仅终态/group空，verify只核identity，必须读真实日志。Mac只下载精确artifact ID，核11 control blobs。source ledger单写者，stage时不并发写；exec session先poll再依赖。

最新只读refs审计`source-refs-audit-20261110T191453Z.json`：457 refs/tree、64control labels、363显式receipt存在，94legacy缺字段保留，ledger SHA前后不变，当时仅828 pending；不是runtime/artifact全审。source-cache已保留未变文件mtime，不编造缓存瓶颈；保持jobs16/原测试观察窗。

证据路径脚本错误已纠正：scope55 freeze相对路径从control cwd查找失败，未改代码/测试；Mac ledger有效，两个freeze已用绝对产品根核hash并scope true，`scope55-freeze-path-recovery.json`保留。物理提交已存在，不重复。

临时early serial/queued817/peer543 baseline worktree已clean非force清理，pins及所有源/run保留；early9b680476 pin为refs/khor/net-baselines/early-serial-calibrated-v2，不按历史重建/再删。

## 估时口径

16:20 UTC告知下一可评审候选2–4工程小时（原锚18:20–20:20），完整N0–N5含真机/换网/能耗3–5工程日量级。18:23已交代本批48窄scope双OS通过，19:25又闭合55/16；在原预计区间内。原13:00 UTC的4–6小时只指下一批，已解释新增缺陷导致延后与口径不清；不得每次compact/状态问句无声重置起点。完整计划未完成，继续执行，不承诺已解决手机现场。

2026-11-10 19:52 UTC状态答复：原2–4小时候选窗口已产出双OS55/16窄范围候选，真实重复性仍失败。新估当前N1修正及复验另需2–4工程小时；完整N0–N5仍3–5工程日量级，含真机/能耗，不能无声重置原窗口。

19:57 UTC诊断准备：从实际RPC记录可见1807–3255ms的Custom质量101.117762ms/2样本，3620ms变85.178119ms/1样本；足以证明近期中位数遮挡与过期相关，仍非所有tick/样本归属证明。新增显式DEBUG下逐Custom copy stats/Reading及恢复road代表copy日志，行为不变；下一次物理运行是针对该缺口的诊断，不是第四次同源重试，不混入固定三轮的验收集合。

当前唯一重型诊断：turing-20261110T195647Z-55d1e669，PID2808204，19:57:09 UTC启动；de03e6b9/tree21ceb5a7/snapshotb083f998，control68cad。仅新增DEBUG测量、行为不变，不归入前三轮验收集合。19:58:57仍running、已编译并进入真实场景；尚未采集/判合同，不重复提交。没有Mac重型。

20:05 UTC：55d1e669诊断已终态218.633秒/16files/group空/identity，124/124、原warm/decision均绿X3432.844/Y3391.236ms；仅机制观察，不并入原三轮。逐copy日志表明此次较大下降触发41刷新，慢轮没有。准备新独立合同：已知实测pre-fault质量，恢复的真实post-epoch样本重返该水平时只允许首次刷新；健康/仍较慢/单脉冲后慢包/过期为负控，原收益/确认与3次probe预算不变。当前仅加五项合同，尚未远端/未实现，不称修复。

20:08 UTC：eec09ec1旧行为实际41.799秒/13files/group空/identity；Noq439绿，路径53pass/2新FAIL，原50及三个新负控过，home未跑。原whole ae62ab38与旧50测试尾完整冻结。现实现Iroh46候选（主55项），只为首次恢复重返实测pre-fault水平刷新quality epoch，未改probe/切换门/确认；尚未构建验证，不称通过。42–44仍不合主。

20:13 UTC：Iroh46候选1c847229/tree90f62f50/snapshot4c18fa47在5c2e4ff2已终态45.781秒/14files/group空/identity，Noq439/path55/home11均绿；同whole五项ae62与原50 tail不变，仅unit scope。当前普通重型LinuxNode dc23e338/PID2825686和Mac42892759864（exact receipt8ce74d28，control68cad）同canonical运行，未进物理。已预登记recovery-known-quality-physical-pair-plan.json：三组/六次C-B,B-C,C-B，无盲重跑/删失败、原门与profile不改。须两端Node终态且Mac重型结束后才按序开物理，不与单元重叠。

20:23 UTC：pair01候选9cb240b7已终态220.572秒/16files/group空/identity核，124/124、原warm51/51绿；独立双端decision仍红，X3855.026/Y4068.484ms。原4秒不放宽；6预约/2episode预算绿，innerCustom两窗口实际counter bounds完整，非outer字节/手机能耗。按预登记继续baseline对应轮，不改策略/丢失败；N1仍红，不能按sampled warm通过关闭。

20:26 UTC机制核：首候选X84.047959ms相对previous84.218146ms、Y84.573961相对111.680773ms均不满足原25%向下重置，却实际restart，结合精确1c源guard可推为新46一次return分支触发。Y+1568ms清silence、+2068ms Moving开始、+4068ms Go，仍超四秒；这只是机制观察，不是统计/成本收益归因。known-quality-branch-observation.json保留。当前f79 baseline在跑，不重发。

20:30 UTC：首组旧baseline f79已终态216.586秒/16files/group空/identity，124/124、warm51/51及decision全绿X3557.596/Y3732.647ms；candidate9cb为3855.026/4068.484ms（Y红），这一组不显示收益。两者恢复期Custom copy数4vs6、总体episode2vs4（含setup）不同，不能用原始字节差直接归因策略；完整两成本窗与无删样本保留。当前run3 3266eaea是预定第二组baseline，之后run4、run5 candidate、run6 baseline；尚未改实现或门。

20:35 UTC：第二组baseline3266终态174.330秒/16files/group空/identity，124/124、warm51/51绿，但X3751.098/Y5469.355ms独立decision红；6预约2episode预算与4Custom副本两个成本窗完整。原样保留；known-quality-physical-pairs-through-3.json是描述性部分结果，非显著性/p99。run4 71eab46c对应candidate在跑，之后run5C/run6B。期间源码实现/fixture/门未改。

20:40 UTC：pair02候选71eab46c已核216.512秒/16files/group空/identity，124/124、warm51/51及decision全绿X3851.354/Y3820.801ms；6预约2episode、4Custom副本两成本窗完整。对应旧3266的X差+100.256ms/Y差-1648.554ms；只是原始成对差值，不是显著性/p99。known-quality-physical-pairs-through-4.json保留；当前run5 6aa2fe4e，之后只剩run6 baseline，原代码/门不变。

20:46 UTC：候选6aa已终态222.660秒/16files/group空/identity，availability124/124但optimal_road_observed=false，warm52/52失败、双端Go=None。4预约2episode预算绿；4Custom副本四秒成本界完整，至reads_finished的enclosing有缺边界，保持unknown不补0。无postfault quality刷新/answered事件；quality_steps0/quality_sinceNone，X两copy计数48/53不再增长但少量last_received更新，Y计数48/49/收包时刻都停在故障前，PTO增长。故旧“质量重启压住”假设不适用于这次，尚不能归因46。outer实际绑定两腿+6s显示IP选中，middle onward质量年龄3.526s；无tuple mismatch，仅是快照，需实际流/包证据定位单向承载停顿。原失败完整保留。
