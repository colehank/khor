# feat/net 当前状态与接续入口

更新于2026-11-11 10:00 UTC。完整N0–N5目标“完成计划”active，本轮是代码与实际证据进展，未完成/未阻塞。root串行，不派发子智能体。此前逐字STATE在[上一状态](HISTORY-20261111-through-comparison-and-hints.md)，SHA `e0ad47c3c71a3b8c0adecf54314b542bea81544114e2b395c806451663fba867`，继续链接此前全部原始失败与历史。真实worktree、diff、receipt与进程优先，不凭调度paused/观察超时停止或重发。

## 位置、授权与源码

产品 `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch `feat/net`。最新hint候选 **0d1c0330036d42a3529e4e7f073c330fe9abda9e**，tree **a650622220a3d62a59ad4a13049085d1789250f0**，snapshot **84b75533a9a4781cce1c08437aa6be16666ce4af**，candidate-hint-context-candidate-source-v2.json，clean/0excluded。它仅逐字应用80cf首轮返回的hop.rs format.patch；产品逻辑与六fixture不变，已核双OS实际87合同及全部12组；见末尾10:00 checkpoint。80cf source-v1 dirty文档且当时未包含未暂存N4计划，相关准确源/格式-only原件仍保留；不把v1结果外推为行为验证。后续文档提交另核HEAD/diff。

最新已核双OS全部12组的是比较源 **a73b9cae0f9d147cb4ec445d32190e395ca97244**，tree **6a2ec5cfcb0a8b93bf50779aa9178feca2af5eff**，snapshot **337b232e46cf436d534155349aaeff18e51948e0**，candidate-comparison-context-source-v2.json，hop81。它早于hint实现，原物理前后对照冻结在该源，不能混入hint改变。

控制树 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，固定68cadb57a0ed72d40a4ca9a843e86b9c6797bf34，原两个untracked pycache保留。Iroh42–44独立树 `/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines` detached2ef6bee5a5aaf841af066f4edb5c894769d4539d，未动。证据根 `/Volumes/disk0/Khor/evidence/net`，ops同根/ops；source-refs.json由root单写。

用户授权完整实现、Turing和Mac私库构建；main合并/发布、采购/生产部署及已装App/config/store/resident修改不由目标自动授权。本机source-only，不跑Cargo/Node/Xcode/fmt或下载产品，Git/Python元数据允许。无phone采样/候选安装。

## 当前在途：先查同一收据

**Turing turing-20261111T094808Z-fb91c212**，hint候选effective-hint-context-candidate，PID3786112，09:48:29 UTC launch，只提交一次，exec64185已成功退出，准确0d1c/treea650/snapshot84b755/control68cad，前驱a59e4d3f原矩阵已terminal/collected/identity/空group。先核该准确receipt/execution/PID，不重发；此时尚未核新87行为。当前是单位回归，不是新物理计时。

**Mac hint候选 Actions42997902740**，request7220f75337c74afc89eb8b0817c3682a，准确同源0d1c source-v2，已登记，exec94386/83932退出，09:46 API仍in_progress。先查同run。初版Mac42996519985仅format、exact13843469806/ZIP SHA4249b1638d9578fa70f6efb312c314daa54184a26d53a522b38bdba545bf27d6，9files/11controls/identity、0组，精确patch应用为0d1c，已分类，不重跑。

比较a73物理 **092344Z-a59e4d3f已全部9green终态收齐**，不再poll；比较前c2b084720Z-2463e1e5、两OS a73单位和hint旧Mac42994929836也都已终态核。以下的旧“在跑”表述由最新本节/末尾checkpoint替代。

无phone采样/候选安装，完整N0–N5继续active。

## 本轮实际设计与证据

### 1. peer relay准备与提交

c2b源码 **c2b615fedc928dfc87c1cc088ca2c06dfb3cb550**，tree7ebc172d2cb2a77839379beb57390fe7997fd748/snapshot80f65c7fa67534b0a26f1ce953bfe2accb506653，在Mac **42989990177**与Turing **084053Z-8d9819ab**准确同源全部12组/hop79通过；各目录node-regression-verification-v1.json核每组，ledger终态。Mac exact13841233618/ZIP SHA88745ebf8c498eec2356977fd677762be5abe0ea6c1b259d69ffd6c5e144bae9，20files/11controls/identity。Turing343.488秒、21files/identity/空group。

原MISSED从target-wide拒绝改每middle count/at，只给实际尝试者退避，未试第三个可进场。legacy在同一原3秒/更早caller预算内按剩余slots分配，超时不退役outer；receiver OK只准入，收到完整u16首帧才替换Links，最多65537字节/当前原3秒等待，prefix原样送入加密transport。该等待和预算分割仍须高RTT/调度成本接受，不能当最优定案。

c2b再加入最多2个借用父操作的FuturesUnordered准备，赢家才发布carrier，输家流在发布前取消。DeviceInfo.hop_staged只作可能过期的hint；hop-ready/hop-ready-in需B/Y都明确识别并正向OK，不信残留版本标记。阶段失败仍用同总期限剩余的legacy fallback，当前预留半预算，原TRIES2/信任/单跳/写重试门不变，不新增常驻map/task/timer。三个真实TLS/echo合同验证挂起first status时second胜出、双Ready仅一个数据提交/输家取消、stale hint拒绝后legacy救通。它们的middle是adapter，不代替完整实际两腿竞争。

此前旧两个admission合同Turing080317Z-f15d8284真实73/2红，per-middle fcc7在081044Z-ee0e2887为74/1，只修第三middle；新增receiver覆盖合同旧082230Z-308999e8为74/2。7d legacy公平/首帧候选在Mac42987755071/Turing082952Z-dae9993c双OS全12/hop76绿。所有格式-only尝试和失败原件见上一状态，不抹掉。

### 2. 比较工作绑定实际事实

a73移除进程全局LAST_COMPARE。原HopCandidate加入attempted_at，同network同实际活wire继续保留该middle工作；新实际candidate/first leg/network可及时比较。begin_middle在原MISSED锁内原子claim，拒绝旧weak epoch与过期caller；仅比较受30秒间隔约束，前台恢复仍受失败退避/原期限；未试第三个不继承前两个时间，made成功保留比较记录并退役旧完成。

不新增map/task/timer，沿原resident beat/note_roads。原全局门与每middle门总工作上限不同，1/3/5设备CPU/字节/quiet必须验证，不凭保留“30秒”声称总成本不变。候选Mac **42992730535** exact13842831398/ZIP SHAc82f40fe082002cdbf7da97d7a55dc376f2b6fd47b87767ec1cc7904fab49d9a，20files/11controls/identity，全部12组/hop81/wires16/budget26/resume1；Turing **091507Z-2f94f130** 339.179秒、21files/identity/空group，全部12/hop81/budget30。comparison-context-source-audit-v1.json状态both_os_all81_all12_verified_physical_pending。

初版19cceb Mac42991598937仅格式，exact13841773241/ZIP SHA669330aaa09884cd970f935969fdc4b81f458989ee52c20aff4acf078be2541d，9files/11controls/identity、0测试组，精确patch应用为a73，不算行为红。

### 3. 有效地址hint仍阻塞旧退避：旧真红，候选双OS已通过

test-only旧格式源 **e2658a4a3699ca62b49b9b3ff3cfddb5f4dd1071**，treedd847b8ec12554f7f805de00d87e6654125ef1a4/snapshot e1c4333216fa29cad16bd3908ddfb8a6fb215818，candidate-hint-context-old-source-v2.json。Mac **42994929836** exact13843238785/ZIP SHAca31dc7a6ceb91399892f47541d3d2fcaed3911b9e2eb7f6ad1b6067b672ca81，10files/11controls/identity；真实hop **83pass/4FAIL**，原81与两normalization控制过，四个target/middle独立IPv4/IPv6有效hint恢复均在最终assert红，net67+1ignored过，后续组未跑。

每case先由独立probe对同key的新真实endpoint完成TLS/核地址族；不把该probe登记入initiator Wires，第一Tunnel与本机network token保持。真实NO_ROUTE后只发布hint，要求恢复carried TLS/echo；纯顺序/重复/无效项不取消冷却。middle仍adapter，不冒充完整物理第二腿。fixture和hint-context-old-behavior-v1.json保存真实前置与清理后失败。

80cf候选在原HopMiss加target_hints/每candidate加hints，经现有dial_addr（含nearby）解析有效IP/Relay后排序去重。目标有效hint变不继承旧第二腿失败/工作，只有middle变则仅它失效。生产用观察后冻结all，排除caller较旧DeviceInfo；同MISSED锁先拒绝旧观察再读同步facts，不在锁内读文档/await；NEARBY follow只写自身map不反调MISSED/Wires。contact_version每次heard递增，不是网络epoch，已明确排除，RTT/顺序/来源也不取消退避。原81和六fixture正文保持，预算/并发/信任不变。hint-context-source-audit-v1.json与候选source记录细节。

为冻结old格式v2，曾将自己的hint-context-working-v1.patch临时反向移除、应用返回test format并冻旧源，再逐字恢复。SHA **1ce7befad11ccf881f8065059b2651770c087b7ddcfcbdea2b5260d77adb938f**，候选已正式提交80cf→格式0d1c，不需重新重放此patch。初版Mac42993175638只是格式失败，exact13842223815/ZIP SHAdbd2cb6d0b09a218887ce04596f38ae5cb620722de4d640927737d054620dfd6，9files/11controls核，源febb；不得混作行为红。

## 原物理矩阵与完整剩余范围

比较前 **c2b 084720Z-2463e1e5**已1564.564秒终态，15files/identity/空group；原三轮各3green、全部9项，f4i回peer **28.2/17.8/34.6秒**，f4最慢echo3.7/3.5/3.5，f4s9.1/9.3/9.3。original-hop-verification-v1.json核原rows/日志SHA。verify-original-hop-matrix-v1.py只读metadata/log，不执行产品。f4i是公共Relay已可用、middle恢复可达后回peer的收敛，不是首业务响应/整个断连；never/无观察切换不是零恢复。a73物理已收齐9green，f4i8.1/8.6/8.1秒；只3样本、不作p99或最终接受。

PREPARED-PHYSICAL-ACCEPTANCE.md已冻结、待实施：原f4/f4s/f4i只有X/Y/M三个khor节点，不能证明双middle prepared并行。新增四peer actual case须在M status未回复时经N在1.5秒内完成真实业务，恢复M后10秒/30业务同wire且无迟到覆盖；若迟到请求未被观察，不声称覆盖。若放既有f4_过滤器则未来四case×三轮属于新12项集合，保留原三body/拓扑/门，与当前9项归因隔离，不必修改trusted control/host网络。

完整N0–N5都继续：
- N0最终体验定义、足够样本p99与相同条件完整前后证据未收齐。
- N1 a73物理后的8秒剩余尾巴、严重单向stall、quiet/stalls和新比较成本仍欠。Iroh47/Noq13既有固定物理集含红且样本少，不能用新单元绿抹掉。
- N2 同connection实际IPv4/v6移动、LOST本机身份/路由上下文、目标/中间hint候选双OS及完整取消/迟到成功组合仍欠。特别需审查旧epoch的成功准备：made_in_epoch对旧token只早退，open_hop之后仍可能继续发布；这只是静态待真实合同证明的风险，不能当已确认根因/已修复。
- N3 真Node交互/批量共享或隔离、背压公平、1/3/5设备与CPU/字节/唤醒/手机能耗。已有congestion independent_fifo_v2仅底层Noq模拟，不能代替真实Node/hop/手机。
- N4 完整两腿Direct/Relay/SSH不对称、唯一/多个middle、双middle实际竞争、旧版本混合、独立/同时v4-v6与Wi-Fi↔蜂窝、拓扑故障域与运维设计未闭环。
- N5 主vendor **57项**=Iroh1–41/45–47+Noq1–13，42–44隔离；完整上游/退出审计、Noq1/5专项、最终Quality/native/proxy/多设备/iPhone和一致报告未齐。

旧严重6aa（无Go/quality_steps0、RTT不对称/缺失、carrier约21秒结束）、Busy854、resume32 timeout、CPUquiet/f4s、coldstart、Noq13固定物理7.3/5.4秒及截断8f3等全部保留。Iroh47固定C-B/B-C四项两C过、B一次初始超时一次恢复7499ms红，不追加第五，不称p99；准确版本/原件/成本边界见上一历史及VENDOR-AUDIT。8cd动态27green、长稳4green各有准确源证据，不为新a73/hint源或手机背书。

MacLab与Turing同LAN；macserver为另一台、用户报告异网。iPhone localhost旧0.2.0现场反复关开App后某次恢复，5GA截图仅hinton Direct29ms；候选未安装。沿可信GitHub/TestFlight边界最终真机，旧现场不能冒充新候选通过。最新状态曾向用户明确完整仍需数天、无可靠截止；07:44给地址小批1–2小时窗口已在1bd双OS小范围验证完成，不在compact后重置整体承诺。

## 下一步与执行约束

1. a73原9项已收齐并核，结果comparison-context-original-hop-result-v1.json；继续诊断约8秒尾巴与f4s瞬态/成本，不再重跑或把原green当最终接受。
2. 核Mac42997902740与Turingfb91c212实际同源0d1c格式/编译/87行为；同六fixture保留。old83/4真红已取得，不重跑旧源。随后同源双OS最新必须核，不用a73代替80cf。
3. 继续真实迟到成功/epoch提交所有权合同、LOST和同wire移动，以及已冻结双middle物理合同；并推进N3/N4/N5，不能以新87项绿替代全部目标。

图谱先行，本树无.codegraph不建索引；CRG部分覆盖/滞后，0flow不是无影响。debug/explore/review技能此前已读并告知。Turing仅UID1110/tmp/khor-netlab-feat-net-1110、原flock/user-netns/Rust1.95/jobs16/nice10/50GiB起40停/90min，无sudo/host网改动；独立Mac原jobs2/shared cache/资源门可并行。SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking，不转发凭据。stage helper未退出不启动另一个ledger写入；依赖分析顺序，缺数据不补零、原日志不覆盖；观察timeout不等于终态。不标complete/blocked，目标保持active。


09:41 UTC实核Turing a59e4d3f同PID运行17分05秒：前两轮各3green（412.66/410.45秒），f4i回peer8.1/8.6秒；第三轮f4已green，仍有两项在途，不提前全9声明。第二轮f4s原green，但links记录Forwarded→Idle@5.277→Forwarded@8.412，x选择Relay@6.1→Carried@13.1；这是须保留的实际瞬态，不能只凭原green称无可见停顿，也尚未归因新gate。完整比较结果还需核原wire/请求和成本。hint80cf Mac42996519985准确source-v1（dirty文档tree019225/snapshot4e46ba）已取到收据，不重复提交。


原物理对照最终：a73 a59e4d3f已1285.299秒、15files/identity/空group、原三轮各3green/全9；f4i8.1/8.6/8.1秒，对应c2b28.2/17.8/34.6。f4最慢echo3.4/3.5/3.4，f4s8.7/8.5/9.6（第三比baseline9.3更慢，非全部指标单调改善）；第二f4s含Idle@5.277→Forwarded@8.412。comparison-context-original-hop-result-v1.json绑定两source/原日志SHA/差值和所有限制，物理计划已终态。约8秒残余、瞬态与总成本都未接受；原3peer仅一middle不能证明prepared双middle wire协议。之后才提交一次Turing0d1c新单位，未在任何活计时期间穿插构建。


## 10:00 UTC checkpoint：hint候选双OS核齐，迟到成功合同正在补

Mac42997902740已success；exact artifact13845256654，ZIP SHA **2c51a6d97f77a8cf3f304db46e3a05bdd83f4c07aca8c1740db4ef2a23174827**，20files/11control文件核对、identity准确同0d1c/treea650/snapshot84b755。Turingfb91c212已348.004秒终态0、21files/identity/原进程组为空。两端实际全部12组通过、hop87（Mac budget26/Turing30、net67+1ignored、wires16、resume1），各node-regression-verification-v1.json逐组保存日志SHA，ledger已终态。旧四个hint行为红到同合同候选绿已成立；不宣称完整物理第二腿、同connection地址迁移或手机通过。上述旧“在途”段落现由本节替代；不再轮询或重发这两job。

当前HEAD仍d4dd29b7；仅新增未提交late_success_tests.rs与candidate_change_tests.rs模块声明，产品逻辑未改。两合同用实际旧Ask挂起、新middle真正carried TLS/echo及同network epoch，分别要求旧OK不能替换新活消费者、不能在新消费者结束后重新发布。尚未远端编译/执行，不能声称实际红；活消费者case可能已有ID tie保护，结束后case拟测旧epoch提交漏洞。下一步冻结test-only、远端取真实行为，再决定最小原子提交修复，不凭静态风险直接记成根因。

用户再次询问进度与时间：本轮已核双OS新87与物理对照，但全N0–N5未完成，不给虚构百分比。仍需约8秒尾巴/迟到成功、四peer双middle生产协议、真实移动/成本与最终真机。此前按小时的整体估计不足；当前仍只宜按数天估计，不能把下一批回归完成当整体交付。无外部阻塞，goal active；执行继续，未合并/发布/安装候选。
