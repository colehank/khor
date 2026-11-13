# feat/net 当前状态与接续入口

更新于2026-11-11 22:19 UTC（本地11月12日）。完整目标“完成计划”仍 **active、未完成、未阻塞**；完整范围是 [PLAN](PLAN.md) 的N0–N5，root串行，不派发子智能体。此页是唯一当前入口；历史逐字保存在 [上一状态](HISTORY-20261111-through-capacity-setup-v5.md)，SHA `6079915ace9d27daea77a43499663ce352d51597ad32c296a02a32846f482efe`，它继续链接此前全部证据/失败。实际worktree/收据/原始日志优先，不按摘要覆盖未提交内容。

## 当前在途与下一步

**当前仅一个Turing任务在途：turing-20261111T221821Z-2e4c439f，真实高BDP业务预检。** 无Mac任务；只轮询这个回执，不因观察超时重发。 [MODEL-DECISION](MODEL-DECISION.md) 已在实现/首个高profile Node数据前以ff3d0074提交，明确分层校准准入v2，旧冷TCP门及原失败全部保留，不视为通过。新业务fixture已提交、clean/0excluded冻结并提交Turing；当前format/实际编译已过，两个真实w3守卫通过，第一轮low回归进行中，高profile尚无业务结果；原低profile参数和入口保留。

高profile固定300ms/50Mbit、192MiB完整文件/240秒、400ms计划/每请求2秒/各operation单在途，三phase各目录/真实PTY各120，保留≥30完整传输重叠门、720全人口/失败分母/调度滞后和原20ms借用probe。复用同一个分离队列构造器；每次实际八份端点/router读回，按y→x下载路径核对文件字节，反向ACK/control完整记录。没有产品runtime/vendor修改。固定新task先真实两个w3取消/启动回收守卫、三fresh原low w2回归，再三high wh2预检；前置失败阻止high，一旦开始某阶段则完成该阶段全部三轮并保留首个非零退出。

本机只做源码/Python元数据与合成调度检查：source全11项3.222秒绿；此前同组首次失败为合成数据的嵌套bulk在revision更新前复制，严格validator正确拒绝，修复fixture后通过，并非真实业务失败或放宽门。control全13项20.620秒绿，另补高阶段中间失败仍执行后续轮次/保留退出码的调度专项。图谱部分/滞后，0flows不作覆盖。共同构造器与已测c50源码已逐字核同，控制仅固定task与allowlist变化，原UID/flock/netns/资源/总期限门不变；high-resident-input-audit-v1/v2.json完整留证。下一步观察原回执，终态后collect/identity/逐原始人口、实际配置和路径字节核验，再更新ledger；不要恢复已终态15cba7f8或旧90703编排。**高profile真实结果/优先级18轮/prefetch矩阵尚无数据，不能宣布N3或最终完成。**

| 当前预检输入 | 值 |
| --- | --- |
| run / control | **turing-20261111T221821Z-2e4c439f** / **30d7a35db340201eed9f80a6161d9de2ce7cfbae**，pid578110/child578324，stage session61281已终态；22:19:33Z实际running、format.log空、编译9.49秒；两个w3实际通过(0.06秒)，low第1轮idle_before240/240 ok、进入bulk，高profile未开始 |
| source-v2 | **f428627b354a447f169d86d0843f79b2cf810bd3** / tree **3707ffd537c5d8d3c758f83d1bbf7edfb62bf57f** / snapshot **fb9442b69da624075bc596f6e6ed9178a5254636**；candidate-high-resident-source-v2.json |
| 预检协议 | MODEL-DECISION / resident_mixed_workload_high_v2；guard2→low3→high3，不混作18轮priority比较或N3接受 |
| 证据复用边界 | 共同calibration_queues.rs SHA256 50a03d9530b2b6d5400f953cee355333691a257b1fd05a0534aefc8cca63ca3c，逐字同c50；相对Mac8371dbb0全Git差异37个只fixture/metadata/docs，零生产runtime差异；新Linuxfixture必须实际编译/执行 |

前置真实 **turing-20261111T221614Z-15cba7f8**：source725ce682df56e716ad891b2aa2dd72cf97326cc5/tree902c1f37ece5ee327d0acd0940e6a5ea31d1a964/snapshotbe6aa14c43f69d5910f693440addff0278e55124，同control；43.848秒原退出1、10files/identity/原group空、**仅format失败、零行为样本**。原format.patch仅四文件空白/可选尾逗号及两use语句顺序，逐token核验后f428627b应用；format-only-verification-v1.json及ledger终态保留，未把旧失败改绿。本机未跑Rust/格式器。control最终dispatch1项7.830秒、31条task/mode路径绿，包含高阶段中间失败后仍执行第三轮并保留首退出73。

### 已完成的独立容量诊断

独立UDP容量诊断 **turing-20261111T214616Z-c50ab6ef** 已186.409秒、原退出 **0**、17files、identity/原group空，collect/逐原始位图与本地窗口人口核验/source-owned metadata复算/ledger终态。stage session12779与PID553115/child553331均已实际终态；只复用证据，不重发。旧policy编排90703也已终态，不恢复。

| 最新准确输入/证明 | 值 |
| --- | --- |
| source | **a6a119a4d5d3e38197ba1f0da495c6fa8ddf9cef** / tree **29217fb5fb5fa4cafe58c3d69435e4907e9bd2dd** / snapshot **d796dc95fe85326521ff4ed128a7164d7dfefd4d**；candidate-kernel-capacity-v2-source-v1.json，clean/0excluded |
| control | **1a6eee06e2e5df3220acc6e3c6ddcae363ebc3fc**；control-kernel-capacity-v2-input-audit-v1.json：相比ce3e仅measurement revision literal变化，runner字节相同；原资源/UID/flock/netns/期限门不变 |
| 原协议 | [CAPACITY-DIAGNOSTIC](CAPACITY-DIAGNOSTIC.md)，先293aa396冻结v1，1fa4e9df在v2结果前冻结5ms修复；v2 revision workload_kernel_udp_capacity_v2 |
| proof | kernel-capacity-verification-v2.json、原workload-kernel-capacity-contracts.json、identity-verified.json与source-refs终态 |

三fresh、六方向均 **145833/145833计划包成功发送**，共874998；每方向4000个5ms实际slot、无catch-up/跳过/发送失败、max lateness1336–2052us、无>10ms迟到。源packet内容函数/receiver/valid门相对v1逐字未改，metadata只加兼容bin/count与slot关联的4000/37上限，未降低99%/65–75/40–60/期限/拓扑等门。

| fresh | upload sender/receiver Mbit/s | download sender/receiver Mbit/s | upload unique/missing | download unique/missing |
| --- | --- | --- | --- | --- |
| 1 | 70.000320 / 48.309120 | 70.000320 / 48.309120 | 99454 / 46379 | 100929 / 44904 |
| 2 | 70.000320 / 48.309120 | 70.000320 / 48.309120 | 99453 / 46380 | 100929 / 44904 |
| 3 | 70.000320 / 48.309120 | 70.000320 / 48.309120 | 99455 / 46378 | 100929 / 44904 |

全部tuple、bitmap全集/子集、逐包内容检查计数、25/21个本地1秒bucket与人口一致；源实现对每个收到包逐字节检查，独立复算原始bitmap与窗口，未保存/重放网络payload。unique共601149、missing273849，发送/内容/重复/异peer/越界错误均0。70M输入故意大于50M整形，缺失分母完整保留，**不把这一缺失率当用户loss SLO**；不跨host时钟相减。三轮24份实际TC JSON/文本/interface及6次初始root rate安装齐全，profile角色/真实路径byte/drops核验通过。metadata在服务器Python3.8.10也已实际成功产出，完整复算与原contract相同。

**UDP独立容量诊断已通过；完整模型决策/旧TCP门/Noq/Node/N3未接受。** 原contract和独立proof始终physical_calibration_accepted/full_N3_accepted=false。这证明在该分离几何与负载下接收端确实获得约48.3Mbit payload，不能把旧冷TCP平均35Mbit直接等同IP链路只有35Mbit；但也不证明CUBIC、队列或CPU某一项是唯一根因。

分层模型判读已按MODEL-DECISION明确版本/理由/旧新结果；高profile业务fixture已实现待冻结和远端运行。后续按同条件priority A/B和独立prefetch1/2/8、完整预览/公平/背压/成本推进，当前priority -1仍实验候选。UDP通过不等同旧TCP通过、应用SLO或N3接受。完整N0–N5保持active、有可执行工作，不设blocked/complete。

### 本轮前置失败与旧诊断（原件全部保留）

- UDP v1真实 **turing-20261111T213626Z-343630a3**：8f2d059ee7ac810df66aa0a57e71fefc888fb098 / tree6e2edb494b48ee03b56010b9b824bb8ddab04a4e / snapshot2d29f27467db05990beebc83155fd5ffd8f24a75，controlce3e0671c9fe5fc397120ebd00d96e889c7c558e；188.648秒101、16files/identity/原group空。计划874998，sent708209、unique601205，六方向只发108219–136200/145833全不足99%；虽raw receiver窗48.309120，**pacer输入不合格，容量不接受**。原metadata又因Python3.8缺int.bit_count崩溃，无contract；kernel-capacity-pacer-failure-v1.json是独立raw reanalysis，不重写原report。5ms slot/兼容popcount是另版修复，不能改绿v1。
- UDP前置 **turing-20261111T212922Z-c5265bb8**：5e04ec3c/tree58a4e705/snapshot06c80c0b，43.774秒1、10files/identity/原group空，仅format红零行为。e92ffbb4应用原patch包含两处可选函数调用尾逗号调整；初始纯空白断言触发，后逐token核只此二逗号与空白变化，format-only-verification-v1.json完整说明。**turing-20261111T213324Z-75020c6f**：38df2f3b/tree6d37f147/snapshot29f72940，33.779秒101、14files/identity/原group空，format过但CLI无直接serde，三入口编译失败零UDP；compile-only-verification-v1.json。8f2d059e改用已有serde_json显式同字段，未增Cargo依赖。
- [队列拓扑ABBA](QUEUE-TOPOLOGY.md) **turing-20261111T210442Z-80ceec22**：1f4d90dc7c31aaf7c315727cc66105ac6f1c8f9f / treea0c19bc7c976f2b631b556b5a3001bcdabd76e5e / snapshot384dec5a7233307d778e116e337ab28b0a621b63，controldde989632732550725f163a1511b0e5ba7d7511b；329.214秒101、19files/identity/原group空。A/B/B/A upload35.368/35.400/35.406/35.383，download28.599/34.485/34.532/28.088Mbit；272echo/八96MiB完整，物理四红。B rate drops79/80与80/78，传播netem均0；kernel-queues-verification-v1.json。旧format-v1 47ae仅格式红零业务、准确patch1f4d90dc，原件保留。
- [burst ABBA](HIGH-BDP.md) **turing-20261111T185007Z-6e04ad5e**：c23cd79e/tree32620a62/snapshot1fbc641b，control0d573ad3；352.084秒101、19files/identity/原group空，272echo/八96MiB全。4KB A upload25.384–25.422/down26.094–26.101；64KB B up35.385–35.391/down28.671–36.229，四红。固定latency使FIFO limit随burst变，不称token-only因果；kernel-burst-verification-v1.json。324格式零行为与81f Device/Router编译零行为等前置沿历史原样保留。
- extended **turing-20261111T182504Z-cde36230**：c5ec8b6f/tree67888ea2/snapshot53132d69，controlc731fb00；266.067秒101、13files/identity/原group空。单hl0/68echo/双向384MiB/24两端快照完整，平均32.900532/32.432250Mbit仍红；kernel-long-verification-v1.json/socket-flow-age-summary-v1.json。upload25–50秒38.213–38.245/50–80秒34.802–34.829，download34.899–34.931/35.293–35.332，不能只用96MiB太短解释。rcv_space不是对端advertised window，缺失字段unknown、不跨host时钟相减。
- socket c21三轮48快照/204echo/六96MiB完整但25.192–25.324/26.035–26.095红；原h0 374三轮25.214–25.301/26.049–26.144红；同准确源LOW 405三w0原35–80ms/6.4–9.6Mbit通过，不能接受HIGH。全部准确SHA/TC/前置格式红/证明沿历史链和ledger保留，不重复旧run或旧自动编排。

本轮v2 source metadata10项1.704秒、兼容popcount专项1项0.542秒、control相关dispatch1项6.696秒（9tasks×3）；此前全control13项13.904秒、source10项1.640秒沿历史保留。实际Turing format/编译8.80秒/三UDP fresh均过，17files校验齐全。本机source-only只Git/Python数据与合成控制检查，未执行Rust/格式器/Node/Xcode/产品。图谱部分/滞后，0flows不是覆盖证明；未运行Mac新任务、未合并/发布/部署/安装候选App。

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

[HIGH-BDP](HIGH-BDP.md) 的原v1/96MiB与旧TCP门失败记录保留；新v2以 [MODEL-DECISION](MODEL-DECISION.md) 的显式分层准入及192MiB输入为准，均在首次高profile Node数据前固定。新高profilefixture已实现，尚未远端编译/执行；prefetch矩阵未实现/运行。原低profile v4/2秒/120秒门保持，不混用cohort，也不把小链路局部收益当最终设计。

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
- **N3**：低profile18场景及独立UDP容量诊断已完成；旧TCP门仍红，分层模型判读v2已明确，先真实高profile预检，再独立比较prefetch1/2/8、大BDP300ms/50Mbit（先kernel校准/预注册新模型）、前台完整文件/预览、连续控制下bulk公平、共享/分连接、背压/取消、独立host/child退出、1/3/5设备CPU/字节/唤醒/能耗。小链路收益不能自动接受全部范围。
- **N4**：两腿Direct/Relay/peer relay/SSH、唯一/多middle、prepared/混合版本、middle自身v4/v6移动、整机/出口/登记故障域及基于数据的中继布局/运维设计。
- **N5**：全部59 vendor的上游/退出、Noq1/物理Noq5、最终准确集成源码Quality/native/proxy、多机/iPhone与能耗、统一决策报告。

## 执行约束与用户状态

本机source-only，仅Git/Python stdlib metadata/shell语法/合成工具测试；不跑本地Rust/格式器/Node/Xcode/产品。Mac走macserver+私库khor-build/原共享guard；Turing严格原UID1110/flock/user-netns、Rust1.95/jobs16/nice10/50GiB开始40GiB停止/90分钟，SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。构建不混入同机计时。无主机网络修改、无sudo；仅授权scratch fixture。未授权main合并、发布、购买/生产部署或已装App/config/store更改。

MacLab与Turing同LAN；macserver为另一台构建机，沿用户报告与Turing异网。iPhone是localhost、旧0.2.0已配对；历史反复开关App才peer relay恢复是原始问题，不是当前候选验证。没有候选安装/手机采样/能耗通过；旧手动测试窗口已过期。准备可信具体构建后再约最终真机交接，不能假设用户仍在等待切网。

16:41起给用户的估计：当前对照1–2小时、完整收口剩余2–4天，**低置信度**，新失败与真机验收会影响。此估计不是承诺或验收，不随compact/session重置起点。目标保持active，有可执行工作，不设blocked/complete。
