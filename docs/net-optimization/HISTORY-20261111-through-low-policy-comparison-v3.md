# feat/net 当前状态与接续入口

更新于2026-11-11 17:48 UTC（本地11月12日）。完整目标“完成计划”仍 **active、未完成、未阻塞**；完整范围是 [PLAN](PLAN.md) 的N0–N5，root串行，不派发子智能体。此页是唯一当前入口；历史逐字保存在 [上一状态](HISTORY-20261111-through-policy-readback-v2.md)，SHA `07431681a2838546487bace0ed68dbc314cd68469cc33f3f98bf88ada80e539b`，它继续链接此前全部证据/失败。实际worktree/收据/原始日志优先，不按摘要覆盖未提交内容。

## 当前在途与下一步

**目前没有Turing或Mac在途任务。** h0 **turing-20261111T173405Z-3747b66c** 已302.367秒、原退出101，17files/identity/原group空；准确源 **96ae5eb73b32b49710e885bfcbb5d4cf531774ec** / tree **dcf57e97618d55fa326819a1008e91985f4c2cf2** / snapshot **d988d415695f69bd2dc4c2651c02449144e56482**，candidate-kernel-high-source-v1.json、clean/0excluded；control **abfadd4b13b705b4c56aa0a828b9f1bf684e95c8**。stage exec44671终态，已经collect/identity/独立failure proof/ledger终态，不重发。

高profile三轮全部204echo成功、六份96MiB全字节/server成功、min RTT300483–300514us；实际rate6250000B/s/delay0.15秒/lat40000微秒。**upload25.214–25.301、download26.049–26.144Mbit/s，三轮全部未达40–60，校准不接受。** 原job/contract/red/物理门不改；high-kernel-bounds-failure-v1.json独立保留人口/单位/计数。TBF实际burst4093B，x drops287/252/211、gateway236/227/241，netem也有28–45drop；根因仍未证，不称CPU/Noq问题、不将零配置loss当零drop。

同准确源LOW回归 **turing-20261111T174224Z-40546e83** 已95.427秒0、17files/identity/原group空，stage exec65714终态；原三轮68echo/双向8MiB、12份实际TC读回、min RTT40265–40373us、双向约7.519/7.544–7.553Mbit，全在原LOW门。kernel-low-model-regression-v1.json/ledger终态。它证明Model保留LOW实际条件，不证明HIGH通过。旧profile-verifier的源字面调用断言不适合Model函数指针；新的证明核源码LOW->原condition，同时复用未改/未放宽的实际profile checker，原旧工具与记录不改。

下一步按[HIGH-BDP诊断计划](HIGH-BDP.md)实现**只观察、不改参数**的h0窗口cohort：client/server两端、upload/download，在3/8/15/25秒对实际连接采ss -tinm，前两次应有，后两次随实际完成结束；由原父future借用并poll，命令kill_on_drop，不启动游离task，不改caller/配置/原burst/门。明确诊断开销，不与旧h0或Node策略混算。先看cwnd/rtt/接收窗/socket缓冲/重传/交付率，再考虑另版单因素模型对照。**诊断未实现/运行**，当前无下一run ID；不要按旧在途记录重发。

h0实现/校验器已提交96ae5eb7，control任务已提交abfadd4b：固定三fresh h0、单线程、关闭诊断、首非零保留；OPS单task allowlist审计control-kernel-high-helper-audit-v1.json，UID/资源/flock/netns/期限门保持。source metadata **12项1.392秒绿**、其中高校验21合成子例；control metadata **13项14.078秒绿**；无本地Rust/格式器/产品。高profileNode混合场景及prefetch矩阵尚未实现/运行。完整目标仍active、有可执行工作，不设blocked/complete。

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

[HIGH-BDP](HIGH-BDP.md)保持预注册：300ms/50Mbit、96MiB、400ms计划、先独立kernel校准，再Node策略及独立prefetch1/2/8。目前仅h0校准实现/入口已编译并启动，**高profileNode混合场景及prefetch矩阵尚未实现/运行**。不改变原v4/2秒/120秒门，不混用cohort，也不把本次局部结果当最终设计。

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
- **N3**：完成当前18场景对照，再独立比较prefetch1/2/8、大BDP300ms/50Mbit（先kernel校准/预注册新模型）、前台完整文件/预览、连续控制下bulk公平、共享/分连接、背压/取消、独立host/child退出、1/3/5设备CPU/字节/唤醒/能耗。小链路收益不能自动接受全部范围。
- **N4**：两腿Direct/Relay/peer relay/SSH、唯一/多middle、prepared/混合版本、middle自身v4/v6移动、整机/出口/登记故障域及基于数据的中继布局/运维设计。
- **N5**：全部59 vendor的上游/退出、Noq1/物理Noq5、最终准确集成源码Quality/native/proxy、多机/iPhone与能耗、统一决策报告。

## 执行约束与用户状态

本机source-only，仅Git/Python stdlib metadata/shell语法/合成工具测试；不跑本地Rust/格式器/Node/Xcode/产品。Mac走macserver+私库khor-build/原共享guard；Turing严格原UID1110/flock/user-netns、Rust1.95/jobs16/nice10/50GiB开始40GiB停止/90分钟，SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。构建不混入同机计时。无主机网络修改、无sudo；仅授权scratch fixture。未授权main合并、发布、购买/生产部署或已装App/config/store更改。

MacLab与Turing同LAN；macserver为另一台构建机，沿用户报告与Turing异网。iPhone是localhost、旧0.2.0已配对；历史反复开关App才peer relay恢复是原始问题，不是当前候选验证。没有候选安装/手机采样/能耗通过；旧手动测试窗口已过期。准备可信具体构建后再约最终真机交接，不能假设用户仍在等待切网。

16:41起给用户的估计：当前对照1–2小时、完整收口剩余2–4天，**低置信度**，新失败与真机验收会影响。此估计不是承诺或验收，不随compact/session重置起点。目标保持active，有可执行工作，不设blocked/complete。
