# feat/net 当前状态与接续入口

更新于2026-11-11 19:24 UTC（本地11月12日）。完整目标“完成计划”仍 **active、未完成、未阻塞**；完整范围是 [PLAN](PLAN.md) 的N0–N5，root串行，不派发子智能体。此页是唯一当前入口；历史逐字保存在 [上一状态](HISTORY-20261111-through-low-policy-comparison-v3.md)，SHA `5d9c15a79d7e4d3a70e5dc15cf609bbbeabb39a3c271b7cbbf99b0fa248052c5`，它继续链接此前全部证据/失败。实际worktree/收据/原始日志优先，不按摘要覆盖未提交内容。

## 当前在途与下一步

**Turing在途：turing-20261111T185007Z-6e04ad5e，PID408810/child409051**。source-v3 **c23cd79e01b49ae5e28961ee7a294ba7a55d4c1a** / tree **32620a623433af46c9f9ea6e4c59066253f4883c** / snapshot **1fbc641b9770352d851291352fbf09293e664a9f**，candidate-kernel-burst-source-v3.json，clean/0excluded；control **0d573ad369cd328ee5115085eb04ed4ea8562580**。stage session52294已终态，收据/ledger已launched；最近SSH execution=running、同PID命令live，format.log空、实际Rust编译8.55秒成功，进入首个A场景。只等/collect该run，不重发。没有Mac在途任务。 burst source-v1 **turing-20261111T184428Z-32451d75** 已45.958秒1、10files/identity/原group空，仅format失败、零行为样本。准确源ba850688146fadcdca1a28501a35bf09cba23835 / tree f4be97b11d5cdf48ce5c4b1258c2a4451c297dec / snapshot a20e84831b2a62d8f48e8a031acc32335ed68a55，control 0d573ad369cd328ee5115085eb04ed4ea8562580。原件已collect；准确one-file/one-assignment换行patch已check/apply并核非空白内容完全一致，不改任何参数/物理门。source-v2 **turing-20261111T184643Z-81f9abe5** 已37.835秒101、15files/identity/原group空，格式通过但E0308：helper接受Device，gateway实际是Router。四入口均未编译成功、零业务样本；compile-only-verification-v1.json与ledger终态。修复为各自Device/Router.spawn进入实际namespace，公用函数+父abort owner，参数与协议不改；source-v3已按上述收据提交，不重发旧收据。 最新一次延长TCP传输诊断 **turing-20261111T182504Z-cde36230** 已完成、collect、identity、独立原始人口/字节/吞吐核验、ledger终态；不要重发。原退出 **101**、266.067秒、13files、原group空，物理校准仍不接受。完整目标active，有可执行工作，不设blocked/complete。

| 输入 | 精确值 |
| --- | --- |
| product | **c5ec8b6f90b79869e9128da9bacf5a1947752c09** / tree **67888ea203bf588807130f1df9e3be518437d586** / snapshot **53132d6975a75e43e955d9cfcafb66aa6f2919f1**；candidate-kernel-long-source-v1.json，clean/0excluded |
| control | **c731fb00f21dc4913928daa64bb8ec25f349895d**；control-kernel-long-input-audit-v1.json，原UID/flock/netns/资源/期限门不变 |
| diagnostic | 固定ONE fresh hl0；双向各384MiB；原300ms/50Mbit/40ms、2秒echo/240秒whole-file、默认TCP算法/window、32kbit TBF burst不改 |
| evidence | kernel-long-verification-v1.json、socket-flow-age-summary-v1.json、workload-kernel-long-contracts.json、identity-verified.json与ledger终态 |

**最新结果：68 echo全部成功（含4 warmup）、双向384MiB全字节/server成功，min RTT300551us，平均upload32.900532/download32.432250Mbit/s。仍未达到40–60Mbit门。** 两端/两方向的3/8/15/25/50/80秒共24个实际tuple socket快照全部通过；诊断观察成功不把Rust101改成0，也不算N3或HIGH模型接受。

同端ACKed字节计数、当地capture起止不确定性算得：sender upload25–50秒38.213–38.245、50–80秒34.802–34.829Mbit；sender download相应34.899–34.931与35.293–35.332。CUBIC窗口/交付率确实随flow age变化，但晚期仍没达到40Mbit；不能仅用96MiB太短解释全部缺口，也不能由这一轮认定唯一原因。缺失可选字段保持unknown；rcv_space不是peer advertised window，不跨host时钟相减。TBF实际burst4093B、x/gateway TBF drops991/1258、netem159/205原样保留。没有改burst/window/算法/验收门。

**下一步：先单独验证整形/排队模型，再推进高profile Node优先级与独立prefetch矩阵。** 已核原始TC拓扑与上游iproute2：TBF burst较小可能限制整形，且固定latency时burst也参与派生FIFO limit；不能声称固定queue bytes。已按HIGH-BDP预注册并实现4096/65536B固定 **A B B A** 四fresh诊断（ha0/hb0），同一源码/二进制，零ss；两arm都在新namespace实际读回后走同一tc change路径，仅burst输入不同。原HIGH/LOW及patchbay默认不改。本次准确source/control已冻结并提交上述唯一run；control-kernel-burst-input-audit-v1.json证明runner只添固定task allowlist，network.sh仅allowlist与固定ABBA分支变更，原资源门不变。不要直接放宽HIGH吞吐门或把长流结果混入旧96MiB cohort。不重复低profile18场景，不提前选定priority -1为最终策略。

### 本轮诊断链（全部已终态）

- HIGH原h0 **turing-20261111T173405Z-3747b66c**：96ae5eb73b32b49710e885bfcbb5d4cf531774ec / tree dcf57e97618d55fa326819a1008e91985f4c2cf2 / snapshot d988d415695f69bd2dc4c2651c02449144e56482；control abfadd4b13b705b4c56aa0a828b9f1bf684e95c8。302.367秒101，17files/identity/原group空；204echo与六份96MiB完整，但upload25.214–25.301/download26.049–26.144Mbit，三轮全红。high-kernel-bounds-failure-v1.json，不接受。
- LOW Model回归 **turing-20261111T174224Z-40546e83**：相同准确source/control；95.427秒0、17files/identity/原group空；原三w0、204echo与双向8MiB、12实际TC读回通过，min RTT40265–40373us、upload约7.519/download7.544–7.553Mbit。kernel-low-model-regression-v1.json；不证明HIGH通过。旧verifier源字面调用断言不适合Model函数指针，新proof核LOW指向原condition，复用未改的实际profile checker，旧件保留。
- socket source-v1 **turing-20261111T180259Z-02b34436**：3b96b308326d56657e29d5091dd2bc9616e31937，46.9998秒1、10files/identity/原group空，仅format失败、零行为；准确one-helper换行patch按原件校验应用31d30ed8，旧红保留。
- socket source-v2 **turing-20261111T180530Z-c21b02bc**：**31d30ed834711bf8c6de63011ae008d6dcfad24e** / tree **b3ee9ebdb57d6a11c7070b4fe40c46b55d94ff5e** / snapshot **0bfcf23f3d9fda3065e937bbe24735bad9b8886a**；control **d4752d05e04d63116f278904282541c358cb5fff**。300.255秒101、17files/identity/原group空；三hd0、204echo/六份96MiB完整，48个两端/两方向tuple快照有效。upload25.192–25.324/download26.035–26.095Mbit，物理红不变。kernel-sockets-verification-v1.json与socket-flow-age-summary-v1.json。发送端实际CUBIC、持续有待发字节、cwnd/pacing随时间增长；不据此认定唯一拥塞/缓冲/CPU原因。

socket observer由实际transfer父future借用，在同一select中poll；未spawn游离task、command kill_on_drop/1秒界限，实际transfer结束即取消余下观察。只在隔离namespace对匹配实际tuple执行ss -tinm，含命令时钟/状态/stdout，不含业务payload。延长诊断只增加一轮时间/文件量与50/80秒计划；不是同条件性能对照。

上一批已完成source metadata **14项1.516秒绿**（HIGH21/socket18/long9合成子例）、control metadata **13项14.143秒绿**（固定6tasks×3 stub modes、first-failure保留），bash语法/diff通过；最新Turing实际format/compile/transport已执行，physical gate仍红。无本机Rust/格式器/Node/Xcode/产品执行。高profile Node混合与prefetch矩阵未实现/运行。

本次burst新metadata已通过：`test_netlab_*.py` **8项1.325秒**，含burst 19种合成人口/顺序/实际readback/失败保护变体；control `test_network*.py` **13项14.658秒**，含7tasks×3 stub modes及ABBA/首非零保护；bash语法/diff通过。只做Git/Python数据/合成控制检查，首次Turing仅format换行红、零行为；精确格式修复后编译/物理仍待新任务。新task仍原资源/UID/flock/netns/总期限门。

## 已完成的低profile优先级对照

root串行自动编排 **exec90703已正常终态**，全部六block均原退出0、collect/identity/原group空、逐原始人口核验与ledger/campaign终态。**不要再运行该编排或advance这些组。** 权威campaign `response-priority-campaign-v1.json` 保持预定A B B A A B和每block三fresh，共18场景/每arm九轮；全部12960请求ok、零缺失/重复，18份16MiB一致。六份proof/identity及114个原始文件的hash/source/control已独立交叉核，`response-priority-comparison-v1.json`已实际生成并核。

| block | arm | 准确run |
| --- | --- | --- |
| 1 | A | turing-20261111T170059Z-b3de4646 |
| 2 | B | turing-20261111T170751Z-4fd7ab02 |
| 3 | B | turing-20261111T171328Z-3abf2108 |
| 4 | A | turing-20261111T171702Z-d8d2a9b3 |
| 5 | A | turing-20261111T172209Z-0f937684 |
| 6 | B | turing-20261111T172555Z-52fcee21 |

完整18行表/全部范围与限制见 [SCHEDULING](SCHEDULING.md)。每轮bulk目录/终端各120个均完整落在文件实际传输区间；不是组成变化。九个对应位置B−A目录p95全负，**改善6.725–15.628ms**；A108.931–114.527ms，B97.230–105.405ms。完整文件差值−1.035至+0.857秒，九对中5对B更慢、4对更快，**没有一致文件完成收益**。

A/block1/场景2终端p95 **416.984ms** 原样保留，其余A78.819–83.736ms，B79.896–83.922ms。原因未证；终端在另一Tunnel，不能由一个异常推定同Main优先级对终端的直接因果效果或CPU/I/O原因。全phase/实际transfer区间/九对差值/条件成功分布/完整失败分母/文件时间及payload率均保留。仅小链路目录局部方向性收益，不报p99/显著性、不自动选胜；严格优先级仍是实验候选，公平/前台完整文件/背压/成本和全部N0–N5未闭。

[HIGH-BDP](HIGH-BDP.md)保持预注册：300ms/50Mbit、96MiB、400ms计划、先独立kernel校准，再Node策略及独立prefetch1/2/8。h0校准与两个socket诊断已执行并终态，物理门仍红；**高profileNode混合场景及prefetch矩阵尚未实现/运行**。不改变原v4/2秒/120秒门，不混用cohort，也不把本次局部结果当最终设计。

## 冻结源码与前置证据

外部证据根统一 `/Volumes/disk0/Khor/evidence/net`，OPS为其ops目录；root独占source-refs.json。全部run identity/behavior proof按run目录保存，identity绿不单独证明业务绿。

| 输入 | 精确值 |
| --- | --- |
| B/source-v3（priority -1） | **f20461c6d71c390d58fa6d1bc093d83218fcf978** / tree **f634bd8999e047907eed6fe6f5dc884cdc2b1274** / snapshot **93878569c6c16a7ef18c6ff1c5188bc37973d5a7**；candidate-response-priority-source-v3.json |
| A/source-v3（priority 0） | **d507c4e735419114c0613734bd9315fcad9ca04c** / tree **1f1e92bc078186cbf979b456af691c845fa8cf27** / snapshot **7e3dda2cb01d0eed6eef01e7b6bcf974f2303313**；control-response-priority-source-v3.json |
| 两臂控制 | **5f1242f5f4d216dde60ad27be0d17a5958b7ef46**，SSH准确Git输入，本地未push；原UID/flock/netns/资源/期限门不变 |
| pair核对 | response-priority-source-pair-v3.json：两个完整树只有link.rs的priority const一行增/一行删，PULL_AHEAD=8；所有fixture/parser/协议SHA相同；source/snapshot各有隔离ref pin，工作区保留B |

候选只在首次write前降低FetchPath/SatFetchPath的offset>0且成功PathSlice响应优先级；首切片/目录仍0；没有增加连接/任务/定时器/共享map/重试，没有修改信任/写入/0RTT/原caller。它是**实验候选而非已选最终设计**：首切片及时不证明完整前台文件及时，严格跨priority排序可能饥饿，必须完成公平/背压/高BDP/共享分连接/成本对照。

前置均已收件/核实：
- **Mac43061931813**，requestcd1cb21f0dcb4cdb8cd595609ccda8f0，实际control68cadb57a0ed72d40a4ca9a843e86b9c6797bf34；exactartifact **13873209484**，ZIP **ba386502cfa04221d5c647575e2ddbbc0dd941645fb53c8a9bf42903f9b226f1**，20files/11controls/identity。固定12门全绿：budget32/hop89/net68+1原ignored/wires16/journal30/IPC18+六单项；真实六initialCustom/两incoming执行。node-suite-verification-v1.json与ledger终态；不是全部1378 Node或vendor测试。
- **B诊断 turing-20261111T164701Z-a5469ffc**：283.548秒、原退出1，25files/identity/原group空。三个实际场景2160请求全ok、三份16MiB一致、1272实际业务stream配对；1080目录和3首切片getter0、189后续切片getter-1且setter全成功。真实w3/w3-startup两个guard均过（含真实sleep子进程未poll与等待中取消后回收）。原parser错把双test guard摘要按单test处理，原report false/原job失败不改；35412360修为guard严格2、workload仍严格1，合成先红后绿11项1.306秒。wire-priority-readback-verification-v1.json独立核原始行，ledger终态并明确metadata_reanalysis_verified，不伪称原job绿。
- **A诊断 turing-20261111T165434Z-9bc952d4**：310.362秒0、25files/identity/原group空；同control/source-v3，三个场景2160全ok、三份16MiB一致、1272实际配对，1080目录/全部192切片getter0且setter成功，两个实际guard过。原source-owned contract true，wire-priority-readback-verification-v1.json/ledger终态。

Mac/B实际编译源是source-v2 **8371dbb024502c8a0fea952b0ea96fbff6a41f39** / tree51310731fd24ed9e18fa27060770c3ccee9570ce / snapshot5d989256ae5e0362440141dff578ea2c94e87bb7。source-v3相对它**只改两份文档、metadata parser与两个合成测试**，完整Git差异严格allowlist及零runtime文件差异已保存于pair-v3，故按计时前协议复用实际B/Mac运行时证据；不称source-v3在Mac重新全跑。两个arm计时均使用同v3冻结源对。此前v1 Mac43060911492/Linuxbd59仅格式红、零行为样本、相同准确patch已1df76f64应用，原件与终态保留。

## 近期生产参照与边界

生产选路/拨号候选基准仍 **3de372a026488a64a794935600e47db29f2f6faf** / treeae76cd646749ca624d5e3e98af01f96099ebd0e0 / snapshota9f46eaa909b8cf45663395912798effb31b7261，candidate-effective-route-source-v3.json；后续04c67d8e opt-in日志/default env查询与当前priority生产文件有diff，不能把旧矩阵当新source完整复验。

- f2f17750动态三轮27绿，ce3b1222恢复6绿，de04bcc1原p1/p1j/p1s/p5四长稳门绿（前三各600秒）。空闲回好路16–17秒/活跃2.2–3.15秒仍只是旧门内，不是极致UX/完整N1接受。
- 四peer6970ad46三轮6绿，f4r首内容1.263/1.253/1.223秒，实际31响应/stream/单business wire及一个outer commit；原f4尾巴3.6–3.7秒未闭。
- 严重d8c22be9功能门绿、实际目录RPC8.713秒，其中约8.193秒是fixture故意封锁；从物理恢复到回复563ms。不能称8.7秒都是恢复后不明stall；同故障窗口固定备用Relay参照仍缺。
- **0354726c稳态参照九场景6绿/3失败**（一配对90秒、两启动60秒）。384已进入测量RPC全ok不掩盖三次未进入业务的失败。四scratch后代继承原锁；先核UID/PGID/start ticks/exe/环境/锁inode并保留stderr，再逐pidfd TERM，最终原group空才collect。steady-reference-failure-v1.json/cleanup.json/ledger终态；手工清理不是自动回收绿，底层启动/配对原因未证。bc519eeb/8371dbb0改fixture立即持有Reaped、先停keeper再删home，专项取消现已实际通过，不据此宣称原启动根因已修。
- 0ff8c054实际wire诊断1272配对，bulk目录native exchange p95约101–103ms、server accept→本地write约8ms、seat约10us；不直接归因CPU/磁盘/网络队列。bc4dbca9六轮记录器对照4320全ok，无稳定缓冲收益；日志有扰动，不能混作policy性能人口。kernel141b/a9ef读回/物理校准独立保留。
- Noq5 cfg(test)四专项：Mac43050978778候选447绿；隔离单门去除Mac43052478191恰446绿/唯一reclaimed采样计数6vs5红，其余三控制绿。主59补丁数量与生产保护未改；不是全部上游/物理/Noq1接受。

全部早期红项、准确SHA/任务/统计与限制沿历史链/ledger保留，包括573/166/eb72/412、严重6aa与quiet/resume/coldstart、Noq13物理长尾、Iroh47原门等；没有删除、放宽门或把截断/前置失败算通过。

## 完整剩余工作（不缩小目标）

- **N0**：最终分场景UX门、足量尾部人口、同条件完整前后与成本报告；当前不是p99或最终最优设计证明。
- **N1**：严重/quiet/恢复长尾、跨连接质量与比较成本，原启动/配对失败定位，同故障fallback参照。
- **N2**：同一真实连接独立/同时IPv4/IPv6地址迁移、LOST身份、完整取消/owner/迟到/撤销，以及carrier在调用开始后到达。
- **N3**：当前低profile18场景对照已完成；独立比较prefetch1/2/8、大BDP300ms/50Mbit（先kernel校准/预注册新模型）、前台完整文件/预览、连续控制下bulk公平、共享/分连接、背压/取消、独立host/child退出、1/3/5设备CPU/字节/唤醒/能耗。小链路收益不能自动接受全部范围。
- **N4**：两腿Direct/Relay/peer relay/SSH、唯一/多middle、prepared/混合版本、middle自身v4/v6移动、整机/出口/登记故障域及基于数据的中继布局/运维设计。
- **N5**：全部59 vendor的上游/退出、Noq1/物理Noq5、最终准确集成源码Quality/native/proxy、多机/iPhone与能耗、统一决策报告。

## 执行约束与用户状态

本机source-only，仅Git/Python stdlib metadata/shell语法/合成工具测试；不跑本地Rust/格式器/Node/Xcode/产品。Mac走macserver+私库khor-build/原共享guard；Turing严格原UID1110/flock/user-netns、Rust1.95/jobs16/nice10/50GiB开始40GiB停止/90分钟，SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。构建不混入同机计时。无主机网络修改、无sudo；仅授权scratch fixture。未授权main合并、发布、购买/生产部署或已装App/config/store更改。

MacLab与Turing同LAN；macserver为另一台构建机，沿用户报告与Turing异网。iPhone是localhost、旧0.2.0已配对；历史反复开关App才peer relay恢复是原始问题，不是当前候选验证。没有候选安装/手机采样/能耗通过；旧手动测试窗口已过期。准备可信具体构建后再约最终真机交接，不能假设用户仍在等待切网。

16:41起给用户的估计：当前对照1–2小时、完整收口剩余2–4天，**低置信度**，新失败与真机验收会影响。此估计不是承诺或验收，不随compact/session重置起点。目标保持active，有可执行工作，不设blocked/complete。
