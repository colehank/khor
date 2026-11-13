# feat/net 当前状态与恢复入口

2026-11-09 · owner=root，串行，无子智能体。

恢复速读：上一批六项已终态；最新Watch动态27/27，但p1s移动3/1、转发f4s1513ms超1500ms，N1仍红。默认关闭的诊断候选已写；42536677230（netlab-diagnostic）在途，42536711138（net-vendor）已SUCCESS 435Noq/33iroh及根格式。两者同tree1113605d、controlc3b117d；原时长/负载/门槛保持。

## 当前活动任务

| Run | 任务 | source tree / snapshot | 状态 |
| --- | --- | --- | --- |
| 42536677230 | netlab-diagnostic | 1113605daf5597c2663d5e6468c45dade34031c9 / 72b0f7e13c0cb7142db033434cbd8182a1ba610a | 已调度，原p1s/f4s各一次1线程，日志模式不作为SLO |
| 42536711138 | net-vendor | 同1113605d / 71a5b250ed41979a7e8d66d01991b06254b813d3 | SUCCESS，根格式与435Noq/33iroh；decision-diagnostics-unit-42536711138已下载 |
| 42537392595 | net-vendor，扩展home/QAD覆盖 | d7f8f38cdeaabefc398f5580a747c7e599209c49 / a256f6708e9ad3920c1d01561b785e6e1425d1bd | 在途；based5e926bf clean、control322835b，较上一run只增文档/执行覆盖，生产源码相同 |

两者basedf5b0864 dirty，controlc3b117d（已push feat/net）。CLI可选net-diagnostics依赖已锁tracing-subscriber0.3.23，无新版本解析；显式环境开关才安装subscriber，默认构建不开日志。现有9项产品Python、1项control准备、sh-n/bash-n与diff-check通过；Rust格式/锁/编译待远端，不能记绿。图部分覆盖，3个受改函数测试缺口由原p1s/f4s及可选CLI实际启动验证，不新增镜像单元。

42536711138已通过，提交5f69db3f全部vendor/CLI/脚本与其快照源逐文件无diff；可选CLI feature的实际编译仍由诊断run确认，不从Noq/Iroh单元推出。源码锚点现77份。VENDOR-AUDIT逐项盘点全部45条，明确Noq1/4/5/6覆盖缺口；当前33过滤器没执行net_report/socket的khor_测试。control322835b已push，后续net-vendor追加该过滤器并单独存iroh-home-unit.log；其覆盖尚未取得新结果。上游逐函数等价与真实设备验证继续保留，N5未完成。

补齐覆盖的42537392595已调度，连同诊断共两项在途，源码锚点现78份。当前目标先等诊断原日志再定因，默认日志关闭的SLO复验仍需做。ACCEPTANCE另已冻结N2七类预算/取消/写入语义行为协议，仅静态事实与测试要求，无N2实现或行为红；不改变N1优先顺序。

## 授权、位置与完整范围

活动目标“完成计划”已授权PLAN实施/验证，取代最初仅规划状态。速度/体验优先，个人可信设备网。未合main、未发布、未采购/部署中继。本机source-only；Linux用私库GitHub job，Mac沿macserver。其他GUI/治理worktree、现装App和用户原会话保持独立。

- 产品 `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；真实样本时间窗4fa3f1b3、p7相位6a45e488、journal数字回读6b3644f5、恢复/稳定性独立模式与路径诊断e480dc2e、窗口质量同步过期4ec6ab2e；确认取证6eabe635、稳定性准备协议309c1bbc/fixture f8292b59、探测后打洞519ad4aa；Noq后移路故障重检ff399658（已提交，tree6cbec16a是提交前快照）与p3失败诊断；相应文档/实验状态继续更新（活动表tree86619b97是该修正提交前的快照）。更早ACK/Backup修复及p2协议仍保留。恢复先核git HEAD/diff，不能按摘要覆盖。
- 旧生产基线main/rc8 `6485cb83d3b1f0bb93d4633825dc956623d78af9`。`khor-net-baseline` detached该提交，只改fixture与远端测试脚本，生产源码原样。
- 控制 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，最新6af0ac3已push（新增netlab-stalls：原p1s/p5完整时长单场景各一次）；此前fdd1b3f与0e5788c补核心单元，fdd1b3f新增独立netlab-recovery（三轮p2/p3）、netlab-stability（p1/p1j/p1s/p5单轮），full也显式写mode8并行。两个main不动，产品feature尚未推origin。
- 证据 `/Volumes/disk0/Khor/evidence/net/`；source/environment/mode与原日志是准确版本证据。
- 权威索引：PLAN、REQUIREMENTS、ACCEPTANCE、DESIGN、RUNS、REPORT、NEXT-DESIGN、UPSTREAM。历史不是当前状态。图谱是部分增量图；先图工具，缺失后定向读，不擅自建立CodeGraph索引。

N0部分；N1仍有产品红；N2–N5未完成。完整范围保留，不能以单元或p6绿替代整计划。

## 已完成的修复及适用证据

1. 全承载速度排序/统一bind/门槛/迁移确认，hop发现与清理门槛统一。首版63pass，node hop9/wires10通过。
2. ACK样本计数、近2秒最多8观测中位数、有界STREAM活跃备用刷新。p7原45秒不回→首次绿；3次交互2.53–2.75秒、闲置19.64–19.73秒（接近20秒，尚缺相位/负载裕量）。但这一版正式p2/p3重复恢复回归，不能收口。
3. 共享quality/age/count输入：先63pass/2fail取红，再66pass/1ignored（42511021381）、431Noq/22iroh（42511084014）绿。缓存读取不探测/不唤醒paths_stream；质量与年龄来自同一副本。age仍是actor观测时刻，非实际ACK接收时刻。
4. ACK-delay校正：latest_sample与原平滑器用同一校正输入，原raw latest/PTO/拥塞数学不改。42511866017为432/22全绿；短矩阵42511895143为8pass/1fail，仅f4i初始未通过m前置红，p2 973ms、p3最坏696ms且零错/退役、f4/f4s/p6均绿。仅一次，不替代正式重复。
5. 同速黑洞在飞数据：42512359677为432pass/1fail，旧快RTT的黑洞Backup在移路时不释放数据。当前Noq补丁9/iroh30将原不应答判据移到PathStats::unanswered_for，迁移时坏Backup也重排；健康同速不重排，MTU排除/原丢包与拥塞计量/QUIC去重保留，不重放应用请求。**42513546687 SUCCESS：433Noq/22iroh全绿**，tree6465d105，controlc6a20c5，证据blackhole-backup-green-42513546687。新增用例含真实虚拟PTO与流交付，非只改RTT模拟故障。
6. 当前生产代码的动态三轮42513580645：p3最坏530.5/732.0/740.9ms，均零请求错误/退役；p7交互2.4233/2.6044/2.4252秒、闲置19.5757/19.5113/18.7489秒，六项全绿。此run整体FAILURE：旧p2的LAN-only门两红一绿，不可改写工作流结论。
7. 第一方格式修正后42514729034 **SUCCESS 66pass/1ignored**，tree7ba78370，证据backup-source-unit-42514729034。其后p2 v2 fixture和f4_stall失败日志尚未由这次检查覆盖。
8. 最新同v2 fixture正式A/B已下载：baseline42516040900为4pass/2fail、4/2、3/3；candidate42516056972三轮**18项全绿**。候选p2快路收敛1.3/2.9/2.9秒、全部回显p95 38.6–47.4ms/max62.4–90.1ms，零错/退役；p3最坏431.1/539.7/463.8ms；p6r p95 113.4–128.9ms，旧版330.4–333.0ms。旧版p2末轮26.6秒超限；旧固定Relay参照虽回显110.9–113.9ms但三轮期末路径断言红，不能称整项绿。见REPORT/RUNS。
9. 样本时间窗：42518379039为434Noq绿、iroh22pass/3fail，确定性证明初始估计变化/延迟读stats/旧包ACK被误判新鲜。4fa3f1b3修复：Noq发布采样包发送和ACK处理时刻，Reading窗口以真实处理时刻计龄，确认要求包在确认起点后发出；正常2个观测tick、故障离开1个不变，静默恢复也排除旧包。42519038936 **SUCCESS 434Noq/26iroh**（tree5b137c71，snapshot997e58ac，control781a907，证据ack-time-green-42519038936）。其中额外1项是静默覆盖；不能说4项都在旧源取红。此后网络短矩阵见第11项，不用先前动态结果覆盖4fa源码。
10. 221b98fc的独立转发三轮42517915206 **SUCCESS 9/9**（hop-isolated-42517915206）：f4应答后0.1/0.1/0.6秒回carried；f4i回来后24.8/19.0/31.2秒通过m且无回退；f4s三轮绿。该源早于时间窗修复。
11. 时间窗修复版6a45e488短矩阵42519803507 **SUCCESS 9/9**（ack-time-smoke-42519803507）：p2 1.0秒/max45.7ms；p3最坏609ms、零错/退役；f4应答后0.9秒、f4s1.0秒回carried，无回退；f4i9.3秒回m；p6四项绿。2场景并行，仅一次，不能替代正式三轮或长稳定性。
12. 42519750741第一方格式通过，核心66pass/1ignored、hop9、wires10；journal27pass/1fail暴露新诊断的真实兼容红：serde内部tag缓冲不支持直接读u128。6b3644f5保留PathFact算术类型，按JSON的u64毫秒数字读取并扩展到真实Entry持久化回读；尚待42521515341验证。42520842668仅新assert格式失败，patch已与当前源反向校验一致，未跑单元。Netlab的journal助手直接读原始JSON Value，不经过Entry解码，这两类日志链路不同，原始实验数据不因该兼容错误消失。
13. 时间窗版三轮相位42519778236 **26pass/1fail**（9/0、9/0、8/1）：21个p7观察全部绿，交互2.26–2.60秒、闲置4.39–18.56秒；p3最坏703.7/573.3/458.6ms均零错/退役。p2为2.1/3.0/16.2秒，末轮超10秒；末轮max63.4ms、零错/退役，Alpha LAN@16s/Beta LAN@1s。目标保留，不把选路超限说成16秒请求卡顿。证据ack-time-phases-42519778236。
14. 42521515341仅六方向诊断函数签名格式失败，未跑单元；patch已检查/应用。不能当作u128修正仍失败或已通过。新的p2测量末尾始终输出两端路径质量事件，p1失败及双路Direct准备失败也输出；只改测试诊断，不改选路/网络/门槛。source-only轻量产品9项、control1项测试通过，Rust须远端复验。
15. e480dc2e第一方 **42524575725 SUCCESS：核心66pass/1ignored、hop9、wires10、journal28**，含实际Entry持久化回读；证据journal-number-node-42524575725。独立恢复 **42524600966 SUCCESS 6/6**：p2 750.2/727.6/753.1ms，max32.65/31.53/31.88ms；p3最坏640.6/705.8/578.4ms，零错/退役，证据recovery-diagnostic-42524600966。此前16.2秒超限不删除；新模式不同VM，不能用三绿宣布消除偶发。
16. 窗口过期的派生值不一致：42525350514确定性红（434Noq绿、iroh26pass/1fail），窗口只剩1ms样本但质量仍300ms。4ec6ab2e复用record的中位数计算，过期缩窗时重算，清空则保留最后已知值；时间/ACK计数/确认历史不刷新，不新增timer/探测调用。**42526508917 SUCCESS 434Noq/27iroh**，证据quality-expiry-green-42526508917。尚未证明这是p2/p7实际超限定因，真实网络验证见活动表。

17. 4ec过期修复后短矩阵 **42527316922 SUCCESS 9/9**（quality-expiry-smoke-42527316922，tree6f9a77e3/snapshote600c180/controlfdd）：p2 1.0秒、max40.55ms；p3最坏642.6ms、零错/退役；f4/f4s应答后0.3/0.4秒回carried、无回退；f4i19.6秒回m；p6d23.57/p6r123.02/ref-d24.11/ref-r106.38ms p95。2场景并行373.32秒，仅一次，不替代长稳定性或三轮。

18. 确认取证候选 **42528714010 SUCCESS 434Noq/29iroh**（decision-proof-green-42528714010，tree86619b97/snapshotb82b415a/controlfdd）。42528150054确定性红为434Noq绿、iroh27pass/2fail。代码6eabe635要求快速迁移也用决定后包，确认等待时同探测当前路；原黑洞leave/收益/固定等待不改。已核当前vendor/Noq/策略源码与绿快照一致，后续只更新验证文档。真实防抖/恢复尚待活动三项，不以单元绿完成N1。

19. 过期修复、确认修正前的三轮 **42527295249 FAILURE 25pass/2fail**（quality-expiry-phases-42527295249）：21个p7相位全绿，交互2.26–3.10秒、闲置2.51–18.41秒；p2收敛1.5/4.0/1.1秒、max58.15/62.52/50.23ms，零错/退役。**p3为612.4ms/1.3秒/15.3秒**，末轮1次15秒请求超时、无退役。旧“已重复修复”只是其准确源的历史证据，不能称当前黑洞目标达成。优先调查Backup在移路后才满足不应答时，重排是否因只在status改变触发而漏掉；新确定性回归42529470019为434pass/1fail；修正候选与活动任务见下段。

20. 后移路Backup修复 **42529931673 SUCCESS 435Noq/29iroh**（late-backup-green-42529931673）：旧434pass/1fail已红→绿，额外验证<300ms不重排、该虚拟时序<1秒释放并在健康路收到完整流。代码ff399658将原内部status标记改为恢复重检，Backup LossDetection也置位，下一次poll_transmit用同一判据处理。提交前核全部vendor/策略/p3源与绿快照无diff；p3失败末尾加两端缓存诊断。实际恢复仍待42530014989/42530060891，不能据单元声称15秒已消除。


21. 确认取证版6eabe635的 **42528841695 SUCCESS 27/27**（decision-proof-phases-42528841695）：p2 976.7/752.8ms/3.0秒，max32.29/32.11/31.55ms；p3最坏461.2/613.8/493.3ms，零错/退役。21个p7全绿，交互2.38–2.79秒、闲置5.62–19.67秒；phase0 19.61/19.58/19.67秒，裕量仍小。早于ff Noq后移路修复，不互相替代。
22. 同6e源 **42528746575 FAILURE 1pass/3fail、1311.33秒**（decision-proof-stability-42528746575）：p1空闲600秒0/0移动；p1j和p1s均在Direct准备门失败，未进入对应600秒；p5零Relay切换，但150ms组打洞0/1、300ms组2/3，原短停顿门红。600ms观察组2/2、最慢回显1.1秒，原门不变。不能把这次p1s准备失败说成CPU防抖改善。
23. 同6e源 **42528783380 FAILURE 8/9**（decision-proof-hop-42528783380）：前两轮全绿，末轮f4腿应答后2.2秒回carried，超1.5秒；另两轮1.0/0.6秒。f4s三轮0.5/1.0/0.8秒且无回退，f4i22.7/29.8/18.8秒且无回退。ff Noq版转发三轮42531477192在途，不能用8绿删除末轮红。
24. ff Noq版 **42530014989 SUCCESS 6/6**（late-backup-recovery-42530014989）：p2 2.9/3.1/1.3秒、max<37ms；p3最坏591.9/428.7/602.2ms，零错/退役。已取实际重复恢复证据，但不能据此把旧15秒全部定因或称最终源长回归达成。
25. ff Noq版 **42530060891 FAILURE 8/9**（late-backup-smoke-42530060891）：f4i准备在LabNode.with/Node::open_as处document transfer state busy，未进入恢复测量；其余p2/p3/p6四项/f4/f4s绿。p3最坏594.5ms、零错/退役，f4应答后0.2秒回carried，f4s不晚于最后一腿记录heard，均无回退。Busy来源是文档状态真实非阻塞门，原生产保护不改；不能把该准备失败称网络恢复红，也不能称整项绿。
26. 稳定性准备v2在309c1bbc先冻结，f8292b59实现：只在one_speed三场景准备时给Relay设备100ms往返，达两端Direct后清掉双向条件，再原30秒settle/600秒测量；观察期不限制类型，忙CPU/20次停顿/原切换门保留，p5/p2/p3不改。42530871452第一方SUCCESS 66pass/1ignored（stability-bootstrap-unit-42530871452）。同4个fixture/脚本已复制baseline，生产648未动；A/B在活动表，旧红不回填。


27. Watch首慢值立即打洞已 **42531529275产品红：435Noq绿/iroh29pass1fail**（probe-before-punch-red-42531529275），实际probe与punch同时true。519ad4aa修正后 **42532303734 SUCCESS 435Noq/33iroh**（probe-before-punch-green-42532303734，tree63ddfeff/snapshot5da5fad0/controlfdd）：普通慢质量持续原2秒窗且两个采样包在该次开始后发出才打洞，恢复取消本次确认；不增探测预算/独立timer。原无应答1秒通道、RTT超2秒立即探索和30秒节流保留。另3项新覆盖不是都在旧版取红。提交前核全部vendor/策略/fixture与绿快照一致。实际忙CPU/短停顿、相位和转发验证见活动3项，不能由单元收口。
28. control6af0ac3新增netlab-stalls，保留原p1s 600秒/20次300ms/忙CPU与p5三组各4次30秒间隔，不缩短门槛；完整稳定性A/B和full仍保留。source-only产品9项、control准备1项及bash-n通过；新task实际prepare已成功、Linux network在跑、Mac development跳过。新3run同tree5f40be2e（base519 dirty，仅后续接口/文档变化），源码已锚定，共75个refs，清单source-refs.json。

42513039282是首轮候选编译错误（path_stats引用/可变借用）；已改先收集backup ID再读状态。独立分类，不能记产品红。

## 最新终态（2026-11-09，替代上一轮在途状态）

以下六项已下载全部收据/原日志，当前无活动网络任务。75份源码ref逐项校验；准确tree/snapshot/control见RUNS及source-refs.json。

| Run | 实际结果 | 证据与解释 |
| --- | --- | --- |
| 42531361022 | FAILURE 2pass/2fail，2451.63秒 | ff版、准备v2；p1/p1j移动0/0；p1s移动7/0，13轮打洞，最慢734.6ms；p5短停顿有额外打洞。早于Watch修正 |
| 42531408799 | FAILURE 3pass/1fail，2451.42秒 | baseline648、相同准备v2；p1/p1j移动0/0；p1s移动7/0，2轮打洞，最慢802.1ms；p5绿。不是候选全面胜出的对照 |
| 42531477192 | FAILURE 8/9 | ff版转发；f4恢复0.8/0.1/0.2秒，f4i43.7/36.8/17.1秒；f4s0.5/1.7/0.1秒，中轮超1.5秒；均无回退 |
| 42532770351 | FAILURE 1pass/1fail，1120.40秒 | Watch版；p1s移动3/1，零打洞/退役，最慢671.8ms；p5 150/300ms原门绿，均零Relay切换/打洞。CPU稳定性仍红 |
| 42532849927 | SUCCESS 27/27 | Watch版动态三轮；p2 1.1秒/831.8ms/6.2秒，max50.38/52.60/75.08ms；p3 626.1/655.5/508.1ms，零错/退役；21相位绿，闲置phase0 18.42/19.53/19.20秒，裕量仍小 |
| 42532877270 | FAILURE 8/9 | Watch版转发；f4恢复0.8/0.0/0.3秒，f4i34.3/15.1/12.6秒；f4s0.0/1513ms/0.2秒，中轮原门红；均无回退。饱和0不代表真实零耗时 |

下一步先加可选诊断构建：记录ACK原值/ACK-delay/校正值、迁移初始与Go的实际Reading和Silence输入、恢复hold的起止。默认构建不安装subscriber；原场景时长/负载/断言保持。诊断模式因日志开销只用于定因，不作为SLO成绩。p1s剩余3次/1次移动及f4s1513ms尚未证明根因，不能凭selected事件后的缓存快照调阈值。先做该诊断再调度，避免重复提交已终态任务。

## 上一检查点活动表（历史，以下均已终态）

| Run | 任务 / 源码 | 最后观察 |
| --- | --- | --- |
| 42531361022 | netlab-stability，ff399658生产/准备v2，treee4912fda，snapshot591b7399，controlfdd | in_progress；p1/p1j/p1s/p5单场景完整观察 |
| 42531408799 | netlab-stability，baseline648生产，tree0e9bf325，snapshot1b3f7a1b，controlfdd | in_progress；相同fixture/脚本，未动baseline生产 |
| 42531477192 | netlab-hop，ff399658生产，treee4912fda，snapshota432378e，controlfdd | in_progress；新Noq版f4/f4s/f4i三轮9项 |
| 42532770351 | netlab-stalls，Watch版519ad4aa/tree5f40be2e，snapshotb23eef27，control6af0ac3 | in_progress；原p1s/p5完整时长单场景各一次 |
| 42532849927 | netlab-dynamic，同tree5f40be2e，snapshotdc805277，control6af0ac3 | in_progress；p2/p3+七相位三轮27项 |
| 42532877270 | netlab-hop，同tree5f40be2e，snapshot43c7f6ee，control6af0ac3 | in_progress；f4/f4s/f4i三轮9项 |

只查询上述六项，不重复提交旧任务。新增文件先stage，snapshot排除untracked。实验源已锚定到本地`refs/khor/net-evidence/<run>`，不依赖七天artifact保留；数量及清单以证据根source-refs.json为准。快照含source/子树，读取例：`git show refs/khor/net-evidence/42512359677:source/vendor/noq-proto/src/connection/mod.rs`。校验`git rev-parse REF:source`必须等于收据source_tree；不能把parentless快照直接覆盖工作树。

42517477095只报p2格式，42517864599和42518102550只报journal字符串格式，均未跑相应单元。patch已检查/应用；第二次与第三次是同一格式问题，不能算三项新鲜性产品红。证据分别speed-v2-unit、path-measurement-node、ack-time-format对应run目录。

## 仍需解决的证据与下一步

- **p2协议纠错**：原拓扑Relay与LAN都没有延迟注入，强制两端回LAN违背用户速度优先。旧13.0/16.4秒或90秒未回LAN不能单独证明实际请求慢，旧日志缺延迟分布不能回填新协议通过。ACCEPTANCE在v2首跑前冻结：至少一端实际走将变慢的second，10秒内两端进LAN/Relay快集合，包含收敛过程的全部回显p95<200ms、max<1秒、零错误/退役，再保持30次。新候选三轮已绿，旧结果全部保留。
- 当前时间窗版同协议又有p2一次16.2秒超限。优先读新诊断的真实质量及确认/迟滞输入，尤其一分钟内10ms绝对收益底线是否拒绝持续5–10ms改善；Alpha连接年龄69秒不是selected_at，不能只凭年龄定因。现未改阈值或确认。独立恢复3轮与独立稳定性结果要一起看，不能修恢复又放大抖动。
- **真实黑洞恢复**：旧候选42509632238的p3为755ms/9.9秒/15.3秒（末轮1超时），这是真实体验回归。当前三轮已均<0.75秒、零错/退役；继续用相同fixture的正式A/B和长回归验证，不用p2纠错消除p3原失败。
- **转发**：共享API短矩阵f4i正确started后两次selected relay、回hop58.6秒；ACK校正短矩阵f4i最初180秒未通过m。最新42513843436为7pass/2fail：f4绿；f4i本次绿（22.4秒回m、无回退）；f4s回carried23.0秒，较腿恢复应答晚14.0秒，最长回显10.1秒，红；另一红是旧p2类型门。全部日志已下载backup-hop-diagnostic-42513843436。先前诊断只挂f4i，恰逢其绿，没有实际diagnostic行；当前fixture补到f4_stall失败，也读x/y/m四对最后50个hop/dial/settled/path/retired。仍需真实质量/探索/清理证据，不把Direct硬优先加回来。
- f4物理配置：LEG为每设备20ms；relay设备44ms，完整Relay约128ms而非44ms；两跳carried至少约80ms另有封装调度。25%门槛可能临界，必须读真实候选质量，不能只凭注释“hop20ms”要求类型胜出。保持原失败与门槛，未改拓扑。
- **稳定性**：旧采样/API前完整candidate42509652259已21pass/14fail（35项1968秒），p1/p1j/p1s抖动、转发/NAT/p2/p5等红；baseline full为23pass/8fail（31项，含2配对busy未进入测量）。最终候选仍需同条件长回归，不能把所有失败归为CPU，也不能用旧源结果证明新源。
- ACK/Backup修复后、真实时间窗前的full42517959505已**FAILURE 23pass/12fail、35项1864.48秒**（ack-backup-full-42517959505）。4项准备未达：f4/f4s/f4i180秒未经m、p1j beta未Direct。8项进入测量后原断言红：f3未声明no_relay、d2/d4无Direct selected、p1移动5/3、p1s7/9、p2快集合13.5秒、p5短卡顿期间切换/打洞、固定Direct参照p95 1.56秒/max2.63秒。p2i15.7秒报告red但Rust未assert，不计为达标。d2/d4“没选Direct”尚不足证明打洞未打开，需审计可达性与类型门。full8并行与独立转发/性能比较不可混算。
- full的18行f4准备失败诊断只含x/y、只有Relay/拨号/settled，未记录hop或Carried，缺M腿/道路书事实，不能据此确定未探索或CPU根因。6b3644f5把三个失败出口都改为x/y/m六个方向，新的40项矩阵在途。下一步可补p1/p1s失败质量日志与独立稳定性/CPU场景，先区分全矩阵交叉负载与单场景，不直接扩滞回掩盖问题。
- 时间窗版full42521539154已**FAILURE 28pass/12fail、40项2001秒8并行**，证据ack-time-full-42521539154；早于窗口质量过期修复。p1移动5/11、p1s5/5；p1j、f4前置未达；f4s应答后0.4秒回carried，但30.5秒后又Relay且期末Relay；p3最坏1.1秒零错/退役；p6r p95 223.21ms；p7i20.054秒；f3/d2/d4/p5仍红。其余6个p7相位绿，p2本次绿，p2i报告red79.8秒但未assert。不能当作4ec修复后完整验证，也不能将这些全归CPU。独立稳定性结果见下段；先结合新保存的质量/样本/年龄判断，再改策略。
- 独立稳定性42524625685已FAILURE 2pass/2fail、1880.44秒：p1空闲600秒0/0移动，p5三组短停顿零Relay切换/零打洞；p1j在Direct准备前置失败，未测稳定性；p1s含自身忙CPU/20次300ms停顿，600秒移动14/2、零退役、最慢回显658.8ms。早于4ec过期修复，但说明抖动不能全归full交叉负载。p1s若干selected事件快照质量接近/反转，快照非决策瞬间，先复现确认流程，不直接调门槛。证据stability-isolated-42524625685。
- Reading时间窗缺口已确定性红→绿，21相位/三次p3绿，但p2又一红；长稳定性/CPU长尾仍需在途结果。ACK处理不是内核收包时刻，置信/波动未齐。p7各phase新建同拓扑，门10/20秒不改。新完整fixture40项，旧35项不覆盖新增相位。journal解码修正不改网络选择；继续定向必要修改，不能以短绿完成N1。
- 新确认回归42528150054已确定性红：434Noq绿、iroh27pass/2fail，旧快分支用决定前两个包直接Go，Hold跳过当前路。当前候选要求since后包，Hold的探测起点也不早于since，并在等待快候选时复测当前路；快候选有原两个新观测仍可提前Go，不强制等当前慢路；黑洞leave、门槛/闲置探索不改。vendor补丁33记录原因/退出，图工具不覆盖vendor，已定向读调用/测试。目标是补证据缺口，不声称p1s已消除。新源单元42528714010已SUCCESS 434Noq/29iroh；长稳/转发/相位三项在途。代码已提交6eabe635。
- 后移路故障回归42529470019已434pass/1fail，恰为新用例，未执行后续iroh单元。当前Noq将原status_changed内部标记改为recheck_backups，Backup的既有LossDetection timeout也置位，再由下一次poll_transmit用原判据重排；没有独立timer/每次发送扫描/应用请求重放。新用例保留故障前后两个时序，并检查<300ms不重排、旧字节从健康路收到。p3失败末尾附两端缓存路径事件，不改测量。候选42529931673已SUCCESS 435Noq/29iroh，代码ff399658；真实恢复/短矩阵在途；不能把确定性bug当作15秒实际卡顿的全部定因。
- 新最高优先：Watch在第一份慢quality时就may_punch，未等既有PROBE_FOR探测确认；这与p5多余打洞一致但尚非全部定因。42531529275已取得确定性红，519ad4aa/42532303734已435/33绿；普通慢质量原2秒+两个开始后观测，旧值/actor暂停不算新证据。现有探测预算不变，前台STREAM可提供新证据；已超整个2秒窗的极慢RTT仍立即探索，原故障1秒通道/30秒节流保留。真实结果仍待新3run，不以单元替代。
- N1达标后按PLAN推进N2期限/竞速/缓存/取消/随机退避/0-RTT、N3混合负载/能耗、N4故障域/中继方案、N5补丁退出/真实设备/最终Quality。REQUIREMENTS逐项审计保留。

本轮p2只纠正测试的体验定义、补实际回显测量；Entry兼容已红→绿，过期修复434/27绿，实际动态25/27且p3含15秒超时；不能以单元或短绿完成恢复。f4最长回显含人为3/8秒断腿及超慢备用，不能套p3“已有快备用时<1秒”的目标；其原恢复后1.5秒/无回退门保持。详细收据/失败分类在RUNS，阶段报告不代表最终完成。

外盘缺失，主仓只读git show feat/net:docs/net-optimization/STATE.md与PLAN恢复已提交计划，不覆盖原未提交内容。


上游只读核查已开始：UPSTREAM.md记录iroh已发布1.4.1、Noq/proto已发布1.4.0，源码包与registry checksum核对并固定VCS；#4522实际9月29日关闭未合并，1.4.1有简化去重。已修正旧“等补丁1后删整个vendor”的过时退出条件。未升级依赖，45条补丁尚未逐项审完，不能用发布版本名证明当前N1已解决。原始证据在evidence/net/upstream-20261109。
