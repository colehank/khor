# feat/net 当前状态与接续入口

更新于2026-11-11 21:48 UTC（本地11月12日）。完整目标“完成计划”仍 **active、未完成、未阻塞**；完整范围是 [PLAN](PLAN.md) 的N0–N5，root串行，不派发子智能体。此页是唯一当前入口；历史逐字保存在 [上一状态](HISTORY-20261111-through-burst-setup-v4.md)，SHA `a6629e7ae3e3688e85871ae12827b0bcde28aa76bad3931ed5d9a68ae34a243f`，它继续链接此前全部证据/失败。实际worktree/收据/原始日志优先，不按摘要覆盖未提交内容。

## 当前在途与下一步

**Turing在途：turing-20261111T214616Z-c50ab6ef，PID553115/child553331**。准确UDP v2 source a6a119a4d5d3e38197ba1f0da495c6fa8ddf9cef / tree29217fb5fb5fa4cafe58c3d69435e4907e9bd2dd / snapshotd796dc95fe85326521ff4ed128a7164d7dfefd4d，candidate-kernel-capacity-v2-source-v1.json，clean/0excluded；control1a6eee06e2e5df3220acc6e3c6ddcae363ebc3fc，control-kernel-capacity-v2-input-audit-v1.json核控制面只改revision literal、source packet/receiver/valid门与v1逐字一致。stage session12779已终态；最近execution running且同PID runner live、format过/编译8.80秒，首fresh双向sent均145833/145833、各4000ticks、max late1666/1515us、无>10ms迟到，receiver均48.309120Mbit、observations true。其余两fresh仍在途，只观察/收该run，不重发；无Mac任务。 UDP v1 **turing-20261111T213626Z-343630a3** 已188.648秒101、16files/identity/原group空，三个fresh六方向完整收件与ledger终态。准确8f2d059ee7ac810df66aa0a57e71fefc888fb098 / tree6e2edb494b48ee03b56010b9b824bb8ddab04a4e / snapshot2d29f27467db05990beebc83155fd5ffd8f24a75；control ce3e0671c9fe5fc397120ebd00d96e889c7c558e。kernel-capacity-pacer-failure-v1.json独立核序号/tuple/bitmap/全部计数及24份实际读回：计划874998、sent708209、unique601205；发送失败/内容/重复/越界均0，但每方向仅发108219–136200/145833，全部不足99%。六方向接收窗48.309120Mbit，**pacer输入不合格，容量不接受**。原metadata还因服务器Python3.8.10缺int.bit_count抛AttributeError，未产出contract；原件保留，本proof只是独立raw reanalysis。已按实施前v2协议实现5ms实际slot、每slot36/37包≤44400B及兼容bin/count，不改70Mbit/20秒/计划包数、99%发送、65–75输入/40–60接收、期限/拓扑；new revision workload_kernel_udp_capacity_v2。source metadata10项1.704秒，兼容popcount专项1项0.542秒，control相关dispatch1项6.696秒（9tasks×3模式）绿；v2已按上述唯一run提交，未接受整体模型或N3。 分离队列对照 **turing-20261111T210442Z-80ceec22** 已329.214秒、原退出 **101**、19files/identity/原group空；collect、独立原始人口/字节/TC及路径计数核验、kernel-queues-verification-v1.json与ledger终态。stage session2762和PID519753/child519963都已实际终态，不重发。准确source-v2 **1f4d90dc7c31aaf7c315727cc66105ac6f1c8f9f** / tree **a0c19bc7c976f2b631b556b5a3001bcdabd76e5e** / snapshot **384dec5a7233307d778e116e337ab28b0a621b63**，candidate-kernel-queues-source-v2.json，clean/0excluded；control **dde989632732550725f163a1511b0e5ba7d7511b**。control-kernel-queues-input-audit-v1.json证明只添固定入口/分支，原资源门无变化。

| 轮 | 布局 | min RTT us | upload Mbit/s | download Mbit/s |
| --- | --- | --- | --- | --- |
| 1 | combined | 300368 | 35.368370 | 28.599077 |
| 2 | separated | 300381 | 35.399825 | 34.485373 |
| 3 | separated | 300378 | 35.406192 | 34.532369 |
| 4 | combined | 300369 | 35.383369 | 28.088189 |

全部272echo（含16预热）/八份96MiB/server成功；A每轮四capture，B每轮八capture、两endpoint无netem/TBF、两个router各root TBF在实际WAN ix、root netem150ms在LAN v7/v9，互不嵌套，actual burst65531、rate6250000B/s、lat40000us、limit1000及每方向实际byte增量均核。B两轮的pubx/pub y rate drops分别79/80与80/78，**两router传播netem drops均0**；计数不相加。**四轮物理40–60门全红，不接受新模型或N3。** 分离不是本次吞吐缺口的充分解法，不能据此认定唯一TC/CUBIC/CPU根因。原format-v1 turing-20261111T210238Z-47ae288c已45.943秒1、10files/identity/原group空，仅格式红零行为；准确单文件空白patch已1f4d90dc应用，原件/format-only-verification-v1.json保留。Rust实际编译12.23秒，source metadata9项1.057秒/control13项13.004秒见下，未本机跑产品。

**下一步：先预注册不依赖TCP拥塞控制的数据报容量诊断，区分链路容量与冷TCP单流行为；不继续盲改burst/queue/算法/窗口，不放宽旧门。** [RFC6349](https://www.rfc-editor.org/rfc/rfc6349.html)将接近瓶颈带宽的TCP度量限定在equilibrium，明确排除slow start瞬态；[RFC9438](https://www.rfc-editor.org/rfc/rfc9438.html)定义拥塞窗口增长/恢复。它们支持把IP链路容量与TCP payload平均率分开测，但不证明此处的唯一原因或自动让旧门失效。拟在相同分离几何用固定有界paced UDP、独立双向/接收端本地稳态窗、完整sequence/内容/发送失败/接收缺失/队列字节和drops核验；先固定输入/pacer有效性/物理门/总期限与重复数再实现。旧TCP原门/全部红保留，该诊断本身也不自动接受模型/Noq/Node/N3。**UDP已在293aa396提交[实施前固定协议](CAPACITY-DIAGNOSTIC.md)，实现与metadata完成；首run **turing-20261111T212922Z-c5265bb8** 已43.774秒1、10files/identity/原group空，仅format红、零UDP样本。准确5e04ec3cc2d8369b3a3253c711e69058ee13a48c / tree58a4e705641100c1016dc482587208ec10ede121 / snapshot06c80c0b7ff729a12866917406be79efbb90614e，control ce3e0671c9fe5fc397120ebd00d96e889c7c558e，control-kernel-capacity-input-audit-v1.json原资源门不变。e92ffbb4应用原format.patch；其含两处可选函数调用尾逗号调整，最初纯空白断言触发后已逐token核只此二逗号及空白变化，详format-only-verification-v1.json；原件/ledger终态。source-v2 **turing-20261111T213324Z-75020c6f** 已33.779秒101、14files/identity/原group空，format过但CLI无直接serde导致Serialize派生编译失败，三入口零UDP样本；准确38df2f3beb896d047fe928640e0bce3e32c21b91 / tree6d37f1479ee6576ed5d4f64cdbf24de221aae0ba / snapshot29f729402b91eecbe5119deac901ee18086a17ff。compile-only-verification-v1.json/ledger终态。改为已有serde_json显式记录同字段，不加依赖/不改包、pacer、期限或门；source-v3已按上述唯一run提交，不重发旧收据。** Turing只读预检无iperf/iperf3/netperf/nuttcp，不安装全局软件；固定uq0三fresh、20秒70Mbit/1200B每方向、145833计划序号，实际1ms slot/Skip不追赶，发送/失败/接收完整bitmap及错误/缺失分母，两端各自[5,15)秒窗口，原分离几何八份实际读回。2秒ready/28秒总方向界、父abort owner、命令2秒/kill_on_drop。pacer有效性与capacity40–60独立记录，不改TCP旧门或自动接受模型。source test_netlab_*.py **10项1.640秒**（新增capacity25合成变体），control test_network*.py **13项13.904秒**（9tasks×3模式），bash/diff过；queues validator仅抽出同一install校验复用、旧queues/burst/high/socket/long回归全过，Rust格式/编译/实际UDP仍待Turing。 高profile Node优先级与独立prefetch1/2/8未开始，当前priority -1仍候选。完整N0–N5不变，目标active/非blocked/非complete。

### 已完成的burst前置对照

此前burst **turing-20261111T185007Z-6e04ad5e** 已352.084秒101、19files/identity/原group空，kernel-burst-verification-v1.json与ledger终态。下表输入/结果属于这一轮，不是最新queues run；全部旧编排90703/各收据已终态，不恢复重跑。

| 冻结输入 | 精确值 |
| --- | --- |
| product/source-v3 | **c23cd79e01b49ae5e28961ee7a294ba7a55d4c1a** / tree **32620a623433af46c9f9ea6e4c59066253f4883c** / snapshot **1fbc641b9770352d851291352fbf09293e664a9f**；candidate-kernel-burst-source-v3.json，clean/0excluded |
| control | **0d573ad369cd328ee5115085eb04ed4ea8562580**；control-kernel-burst-input-audit-v1.json：只添固定task allowlist和ABBA分支，原UID/flock/netns/资源/期限门不变 |
| 固定协议 | 同源码/二进制，A B B A四fresh；A4096B/B65536B，均同tc change路径，50Mbit/150ms单向/40ms，双向各96MiB，68echo，原2秒调用/240秒整段界；无传输中ss |
| proof | kernel-burst-verification-v1.json；source-owned workload-kernel-burst-contracts.json；identity-verified.json；ledger终态 |

| 轮 | arm | 实际burst B | min RTT us | upload Mbit/s | download Mbit/s |
| --- | --- | --- | --- | --- | --- |
| 1 | A | 4093 | 300358 | 25.422003 | 26.094119 |
| 2 | B | 65531 | 300378 | 35.385418 | 28.670762 |
| 3 | B | 65531 | 300358 | 35.390903 | 36.228836 |
| 4 | A | 4093 | 300292 | 25.383528 | 26.101217 |

**272echo全部ok（256 steady+16 warmup）、八份96MiB逐字节/server成功，八次实际两端shaper change及16份前后TC/interface读回有效；四轮均未达40–60物理吞吐门。** 观察完整性true不覆盖原Rust101，physical_calibration_accepted/full_N3仍false。B有局部改善但下载两轮28.67/36.23的差别保留，不能择优或报显著性/p99。固定latency时burst参与派生FIFO limit，因此不是固定队列字节容量的token-only因果隔离；各层drop计数不相加冒充独立损失。原LOW/HIGH/patchbay默认/生产配置未改。

分离模型按[实施前协议](QUEUE-TOPOLOGY.md)及eef0001a实现、8423dafd提交、1f4d90dc仅格式修复，已按本页顶部准确run实际执行。默认HIGH/LOW及patchbay未改；原HIGH parser增加固定profile callback、默认四读回分支逻辑保持，旧high/socket/long/burst元数据回归通过。source test_netlab_*.py **9项1.057秒**（新queues21合成变体，初始fixture漏schema已修），control test_network*.py **13项13.004秒**（8tasks×3模式），bash/diff通过。图谱部分/滞后，0flows不作覆盖证明。更早burst/long证据沿本页和历史链完整保留。

### 近期诊断链与前置失败（全部已终态）

- h0三轮 **turing-20261111T173405Z-3747b66c**：96ae5eb7/control abfadd4b，302.367秒101、17files/identity/原group空；204echo/六份96MiB完整，upload25.214–25.301/download26.049–26.144Mbit，物理全红。high-kernel-bounds-failure-v1.json。
- 同准确源LOW三w0回归 **turing-20261111T174224Z-40546e83**：95.427秒0、17files/identity/原group空；204echo/双向8MiB/12TC读回及原35–80ms/6.4–9.6Mbit门通过。kernel-low-model-regression-v1.json，不证明HIGH通过；Model函数指针按实际源核，未改旧verifier/原profile checker。
- socket **turing-20261111T180530Z-c21b02bc**：31d30ed834711bf8c6de63011ae008d6dcfad24e / tree b3ee9ebdb57d6a11c7070b4fe40c46b55d94ff5e / snapshot 0bfcf23f3d9fda3065e937bbe24735bad9b8886a；control d4752d05e04d63116f278904282541c358cb5fff。300.255秒101、17files/identity/原group空；三hd0/204echo/六份96MiB及48实际tuple快照完整，upload25.192–25.324/download26.035–26.095Mbit仍红。kernel-sockets-verification-v1.json与socket-flow-age-summary-v1.json；原source-v1 02b仅format红零行为，准确换行patch31d30ed8，原件保留。
- extended **turing-20261111T182504Z-cde36230**：c5ec8b6f90b79869e9128da9bacf5a1947752c09 / tree 67888ea203bf588807130f1df9e3be518437d586 / snapshot 53132d6975a75e43e955d9cfcafb66aa6f2919f1；control c731fb00f21dc4913928daa64bb8ec25f349895d。266.067秒101、13files/identity/原group空；单hl0/68echo/双向384MiB/24两端快照完整，min RTT300551us、平均32.900532/32.432250Mbit仍红。kernel-long-verification-v1.json与socket-flow-age-summary-v1.json。upload25–50秒38.213–38.245、50–80秒34.802–34.829Mbit；download34.899–34.931、35.293–35.332。晚期仍不足40，不能只用96MiB太短解释。CUBIC状态/重传是事实，不是唯一原因；rcv_space不是对端advertised window、缺失字段unknown、不跨host时钟相减。
- burst source-v1 **turing-20261111T184428Z-32451d75**：ba850688146fadcdca1a28501a35bf09cba23835 / tree f4be97b11d5cdf48ce5c4b1258c2a4451c297dec / snapshot a20e84831b2a62d8f48e8a031acc32335ed68a55；45.958秒1、10files/identity/原group空，仅格式换行红、零行为。准确one-file/one-assignment非空白相等patch2fb0265f12f3b86b0f508d04de180311cf4600f79f79ea20310784f9da226543，c71a5efc应用；format-only-verification-v1.json/ledger终态。
- burst source-v2 **turing-20261111T184643Z-81f9abe5**：c71a5efc0e730285714d9aa6d72647d89be3c643 / tree 0f2053bd330a1360d2606ea9a3fc7084068af0df / snapshot 31b29d7eee50fe72252c2c96fd65c76770b4950c；37.835秒101、15files/identity/原group空。格式通过，E0308 helper预期Device却传Router，四入口均编译失败、零业务。compile-only-verification-v1.json/ledger终态；c23cd79e修为实际两端各自spawn、公用namespace函数+父abort owner；新source-v3才有行为数据。

本次source `test_netlab_*.py` **8项1.325秒**（burst19合成变体），control `test_network*.py` **13项14.658秒**（7tasks×3 stub modes/ABBA/首非零），bash语法/diff通过；Rust格式/实际编译8.55秒与四轮行为在上述最新Turing任务实际执行，物理门全红。上一批14项1.516秒source/13项14.143秒control结果沿历史保留。本机未运行Rust/格式器/Node/Xcode/产品；无在途Mac。图谱在此树仅部分/滞后，zero flows不证明完整覆盖；已按实际源/原始日志复核。

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
