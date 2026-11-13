# 阶段对照报告（未完成整体验收）

2026-11-10拓扑更正：用户已澄清同局域网的是MacLab本地电脑与Turing，构建macserver是另一台。cohort4cee410c/Mac42757732776的目录/固定正文120/120及公共Relay101.200.80.77:3340事实保留；该cohort依U10为用户报告异网的构建macserver↔Turing，保留真实双机公共Relay业务结论；公网Direct/NAT穿透仍未验收。历史wan-*任务名不是拓扑证据。用户现已恢复localhost iPhone现场换网采样，结果以当前STATE为准。

2026-11-09。当前结论：最新Watch版42532849927动态27/27，p3三轮最坏508–656ms、零错/退役，21相位绿；闲置phase0仍18.42–19.53秒贴近20秒。42532770351的p5短停顿门绿，p1s仍3/1次移动；42532877270转发8/9，f4s一轮1513ms超1500ms，原门保留。ff版与baseline同准备v2的p1/p1j均0/0、p1s均7/0，ff版打洞更多，不能称全面胜出。六项任务已终态，下一步用默认关闭的诊断构建记录实际选路输入；日志模式不作为SLO。旧15秒黑洞失败仍保留其准确源。N1和整计划均未完成，下列历史各组不能互相替代。

## 正式三次对照

同一fixture/工作负载，每个job内单场景顺序、全新Lab/居民重复三轮；baseline与candidate在不同GitHub托管VM。control419b5f0；完整收据见RUNS和证据source.json。三轮同VM不能当作独立机器统计，下面报告全部结果而非汇总显著性。

| 指标 | 旧策略6485cb83，42509612220 | 新采样候选，42509632238 | 判定 |
| --- | --- | --- | --- |
| p6r 慢Direct/快Relay，请求p95 | 323.93–325.44ms，3轮仍Direct | 110.15–117.42ms，3轮Relay | 旧3红、新3绿 |
| p6r 固定Relay参照，请求p95 | 106.55–108.08ms | 110.45–112.63ms | 两版均3绿；候选自动选择接近本机固定好路 |
| p6d 快Direct，请求p95 | 25.70–27.79ms | 29.29–32.26ms | 均3绿，不能因此声称零额外开销 |
| p6d 固定Direct参照，请求p95 | 25.30–44.98ms | 30.00–32.03ms | 均3绿 |
| p2 旧协议，两端必须LAN | 3.4 / 3.1 / 3.1秒 | 13.0 / 16.4 / 6.9秒 | 原门2红；拓扑中Relay同样快，不能单凭接口名判体验回归 |
| p3 黑洞后最慢请求 | 680.0 / 708.8 / 683.2ms | 755.4ms / 9.9秒 / 15.3秒 | 新版2轮超过1秒，最后1次请求超时 |

p6每轮30次目录请求，200ms间隔，零错误、p95<200ms且期末两端保持目标路径。Relay单腿40ms配置，完整端到端约80ms，实测83–84ms。固定参照在同拓扑准备完后屏蔽竞争路径再预热；它只代表可达到的稳态，不是首次建连或故障切换。

候选tree4549630f，baseline treef95f6e54；两者fixture与netlab线程开关一致，生产源码不同。完整18个结果、失败日志与各轮source/environment/mode已保存证据目录，summary.json保留全部报告，不删除失败。

## 其他已取得证据

- p7旧版动态两场景45秒未回快Direct。首轮新采样交互2.6355秒、闲置18.1609秒；另三次顺序重复交互2.5261/2.7413/2.7497秒、闲置19.6758/19.6386/19.7252秒，零错误。闲置贴近20秒，仍需恢复相位/负载与空闲成本覆盖。
- 完整旧版matrix42507224815：31场景8并行、1908秒，23pass/8fail。其中6个测量断言红，2项配对document-transfer busy未进入测量；p2i还有报告red但Rust test ok。候选完整matrix42509652259为21pass/14fail（35场景，1968秒），p1/p1j/p1s明显抖动，另有转发/NAT/p2/p5等红；它早于共享API/ACK校正/失败Backup重排，不能充当最终源码验证。
- 首版统一bind/选路单元62pass；采样/看守统一门槛后63pass；共享quality输入回归先63pass/2fail，随后66pass/1ignored。底层旧采样noq431/iroh路径21通过，共享缓存API版431/22通过。ACK-delay校正候选是后续不同源码，尚待证据。

## 当前判断与下一步

保持速度优先。ACK样本现与平滑器使用同一校正值；同速但黑洞的Backup未释放在飞流字节已取确定性红432pass/1fail，修复后433Noq/22iroh全绿（42513546687），健康同速不额外重排。当前三轮42513580645的p3为530.5/732.0/740.9ms，零错/退役；真实卡顿已得到重复修复证据，后续仍需最终源完整回归。

原p2的Relay与LAN均未加延迟，强制LAN违背用户决定；旧失败及工作流结论保留，但不能把它解释成已证实用户响应慢。v2保留10秒/零错/零退役，加故障前确在second的前置与全部回显p95<200ms、max<1秒、收敛后至少30次；以明确拓扑快集合验路径。没有用旧平滑RTT冒充新鲜样本设门，也没有回填旧数据通过。

## 最新相同v2 fixture三轮对照

controlc6a20c5，单场景顺序三轮，准确tree/snapshot见RUNS。baseline42516040900为4/2、4/2、3/3；ACK/Backup修复候选42516056972为6/0、6/0、6/0，早于后加的真实样本时间窗4fa3f1b3。两job在不同托管VM，不声称独立机器统计显著。

| 指标 | baseline6485cb83 | 当前候选 | 判定 |
| --- | --- | --- | --- |
| p2快集合收敛 | 3.1/3.1/26.6秒 | 1.3/2.9/2.9秒 | 旧1轮超10秒，新3轮绿 |
| p2全部回显p95 / max | 42.1–49.4 / 52.1–61.7ms | 38.6–47.4 / 62.4–90.1ms | 全零错/退役；新n35/41/41，旧42/42/137，不能称所有延迟都改善 |
| p3最坏回显 | 476.8/598.2/546.0ms | 431.1/539.7/463.8ms | 均3绿，零错/退役 |
| p6r慢Direct/快Relay，p95 | 330.4–333.0ms，仍慢Direct | 113.4–128.9ms，Relay | 旧3红，新3绿 |
| 固定Relay参照，p95 | 110.9–113.9ms | 114.4–114.8ms | 旧三轮期末held路径断言红，不能称整项通过；新3绿 |
| p6d快Direct，p95 | 29.6–33.6ms | 35.2–35.8ms | 均3绿，不声称零开销 |
| 固定Direct参照，p95 | 29.3–32.1ms | 34.5–40.4ms | 均3绿 |

新fixture未改变p3/p6的拓扑、负载或门槛。候选18项全绿证明本批恢复和静态选路，不替代后续转发、长时间、混合负载或真机验收。最新p2第一方远端检查42517477095只报格式差异，已应用patch；它未执行单元，单独记工具输出，不算产品红。

共享质量/年龄/数量输入已有66单元与vendor证据；剩余相位、CPU积压样本、转发、取消/写语义、混合负载、中继故障域、真实Mac/手机与最终准确源码Quality仍需完成。N0–N5完整范围见PLAN/REQUIREMENTS。


当前生产代码三轮p7交互2.4233/2.6044/2.4252秒、闲置19.5757/19.5113/18.7489秒，零错误；闲置裕量仍小。最新短矩阵42513843436为7pass/2fail：f4/f4i本次绿，f4s应答后14.0秒才回carried且最坏回显10.1秒，另一红为旧p2类型门。f4i曾有setup/恢复失败仍保留，不能用这次绿称稳定。完整生产代码的第一方格式/单元42514729034为66pass/1ignored；后加fixture尚待复验。

## 真实时间窗版与完整负载

上述p7/短矩阵为当时生产版，不是当前4fa时间窗版。后续三个“初始估计/延迟读stats/积压旧包”回归在42518379039确定性取红；4fa修复版42519038936为434Noq/26iroh全绿。6a45e488的短矩阵42519803507为9/9绿：p2 1.0秒、最坏45.7ms；p3最坏609ms、零错/退役；p6r p95 106.05ms，固定Relay106.27ms；f4/f4s应答后0.9/1.0秒回carried，f4i9.3秒回m。它只有一次、2场景并行，不冒充正式三轮。

221b98fc独立转发三轮42517915206为9/9绿，但同源完整35项42517959505为23pass/12fail、1864.48秒8并行。4项前置未达（3个f4、p1j），其余原断言红包括f3、Plain NAT d2/d4、p1移动5/3、p1s7/9、p2收敛13.5秒、p5切换/打洞及固定Direct参照p95 1.56秒/max2.63秒。不能将这些全归CPU，也不能将“未选Direct”直接当作“未打开Direct”。完整压力与独立转发数字分别保留。

新增path测量诊断的实际Entry回读另有兼容产品红42519750741：核心66/hop9/wires10通过，journal27/1失败，原因是serde内部tag缓冲直接解u128不支持。6b3644f5按JSON u64毫秒读入原算术类型，并测持久化Entry，随后42521515341仅签名格式失败；修正后42524575725核心66pass/1ignored、hop9、wires10、journal28全绿，实际Entry回读验证通过。Netlab读原始JSON Value，既有路径测量没有经此损坏的Entry链路过滤。

上述三项均已终态。相位42519778236为26/27，21个p7全绿（交互2.26–2.60秒、闲置4.39–18.56秒），p3三轮最坏均<704ms；p2末轮16.2秒选路超限，但最长回显63.4ms、零错/退役。full42521539154为28/40，含p3最坏1.1秒、p7i20.054秒、p6r223.21ms及稳定性/前置等红，不能全归CPU。最终源完整检查、CPU/置信、期限/取消、混合负载、能耗、中继故障域及真机仍未齐。f4的最长回显包含人为断腿且备用超慢期间，恢复后1.5秒/无回退目标不变，不能拿p3已有快备用的1秒目标套整个断网阶段。


## 独立稳定性与最新候选

同e480源、单场景顺序的42524625685为2pass/2fail，1880.44秒：p1空闲600秒移动0/0，p5各组短停顿均零Relay切换/打洞；p1j仍卡在要求Direct的准备阶段，Relay实测相近，需审计前置与速度优先的冲突。p1s含自身忙CPU与20次300ms停顿，600秒移动14/2、最慢回显658.8ms、零退役，真实抖动仍需修复。不能用短矩阵通过或full负载竞争掩盖它。

同源独立恢复42524600966三轮6/6绿：p2约0.73–0.75秒且最长回显<33ms；p3最坏0.58–0.71秒。此前p2的16.2秒失败保留，不以不同VM三绿判偶发已消除。

4ec6ab2e修正“窗口旧点已过期、quality未重算”：确定性回归42525350514为Noq434绿/iroh26pass1fail，修复后42526508917为434/27全绿；不刷新ACK/时间/确认、不增加探测。修复后42527316922短矩阵9/9绿（373秒2并行）：p2 1.0秒/max40.55ms、p3最坏642.6ms零错/退役、f4/f4s应答后0.3/0.4秒回carried无回退，p6四项绿；动态27项42527295249仍在途。尚无证据将原p2/p7超限归因于该bug，也尚未对修复后长稳定性作结论。N1和整计划未完成。


随后确认取证另取得确定性红42528150054（434Noq绿、iroh27pass/2fail）：旧快速分支会使用决定前两个包直接切换，而且候选未答时不复测当前路。6eabe635要求决定后包，并在既有确认tick内双路复测；不加固定2秒等待，不改变黑洞leave和收益门。42528714010为434/29全绿；实际长稳42528746575、转发三轮42528783380、相位三轮42528841695在途，尚不能说p1s已修好或恢复无回归。


最新过期修复版42527295249三轮25/27，p2收敛1.5/4.0/1.1秒，最长回显均<63ms，21个p7相位全绿；但p3最坏612.4ms/1.3秒/15.3秒，末轮1次15秒请求超时。原不应答重排只在status改变时触发，已新增42529470019确定性回归核查“移路之后才失效”的窗口；尚未改Noq生产逻辑，也未证明实际长卡顿的全部原因。历史三绿不能抵消这次用户体验红。


后移路故障重检ff399658已42529931673 **435Noq/29iroh全绿**，相同虚拟黑洞时序先434pass/1fail。候选沿既有Backup PTO事件重检，未改RTT/收益/静默门槛、未增加独立timer或应用重放；<300ms保护及健康路完整交付通过。实际恢复三轮42530014989与短矩阵42530060891仍在途，当前15秒实际卡顿尚不能宣布已消除。


最新结果：6e确认版动态三轮27/27，但闲置phase0最慢19.67秒；独立稳定性1/3（两项Direct准备失败，p5多余打洞），转发三轮8/9（末轮f4恢复2.2秒超1.5秒）。ff Noq版独立恢复6/6，p3 429–602ms；短矩阵8/9，唯一f4i准备Busy未进入恢复测量。最终稳定性与转发尚未达标。准备v2已先冻结再实现，正以相同fixture对照baseline648和候选；Watch单次慢quality立即打洞正用新增回归取红，不能把当前未测的p1s说成通过。


519ad4aa探测后打洞修正已435Noq/33iroh绿，首慢值probe/punch同时触发先取得确定性红。没有增加原探测预算；普通质量需要2秒与决定后证据，极慢RTT和真实故障仍有原快速探索。新control6af仅增加原p1s/p5定向入口，实际忙CPU/短停顿、相位、转发三项正在运行，最终N1仍未完成。


42555870179 Mac根格式、438Noq/36路径/9选家全绿，准确tree4a9c4d89/snapshotb6a4e924/controldd57be6。37无接收copy快速离开用例从42551075037实际红转绿；有提示/旧包/兜底覆盖保留，Noq2始终启用reporting恢复实际通过。API bundle远端还原源码与可信控制后复用原guard，旧Git HTTPS两attempt执行器失败不记产品红。Linux真实p3与f4s红尚未复验，不代表N1或整计划完成；N3测量记录器只有格式证据。


## 2026-11-09：已校准Noq层首批基线

42560015454（tree33d5ef6d47cde04620db1baa470bd7fefdf0c99d、snapshot9a6f9717470e0a741aedbdd9dd1ecf0a32d18fc8、control0cff9671fb2f80945423132e0df46995f6ff3198）在Mac26.6.2 arm64/Rust1.95、2构建任务/测试单线程，根格式/校准1pass、三轮各2pass全绿。
校准先取得42559476637共享FIFO模型实际红，再只修独立方向派发；模型independent_fifo_v2，记录schema v2。原几何300ms RTT、每向2%损失/20Mbit、200ms队列、20秒bulk、两固定seed保留。没有跨模型计算产品收益，未改产品CC。

704原始请求（每轮235/234/235）、51摘要、15bulk case全部保留，零失败/超时；对每组索引/计数/成功与全尝试分位数从原样本逐项复核。原日志SHA256、合并分位数方法及详细数据在mac-duplex-congestion-42560015454/sample-audit.json。下表合并三轮bulk-echo；吞吐按实际bulk秒数计算范围。

| CC / 优先级 | echo样本 | p95 ms | max ms | bulk Mbit/s范围 |
| --- | ---: | ---: | ---: | ---: |
| bbr3 / off | 78 | 449.6 | 710.5 | 9.307–10.742 |
| bbr3 / on | 75 | 612.4 | 821.1 | 9.310–11.841 |
| cubic / off | 70 | 890.3 | 1081.8 | 0.322–0.344 |
| cubic / on | 77 | 389.9 | 808.2 | 0.334–0.338 |
| newreno / off | 66 | 973.2 | 1108.7 | 0.211–0.245 |

该固定模型下优先级没有呈现一致改善：Cubic尾部下降，BBR3本轮p95反而更高；这是后续定位线索，不是因果置信结论或产品策略选择。各CC固定顺序、同一丢包seed且仅三轮，不能泛化到真实路径。typing各CC159/161样本，screen仅9样本/CC，原样保留但不报p99或尾部SLO。
下一阶段必须测真实Node准入/排队、SSH/hop外层流、共享/分连接资源公平和后台能耗；N1 Linux原p3/回退红、N2其余边界、N4/N5不由本结果收口。


## N2操作期限与Accepted回答所有权：Mac红绿

42562932053的真实ask/持锁回归两红（40ms未传达、300s被28s截断），由42566727617首次转绿。42567527752随后在实际Accepted回答路径取得取消/期限两行为红，五预算控制绿；两fixture均先通过同wire独立Sessions/相同stable_id/严格三请求/零dial_failed后才在目标断言失败。旧is_0rtt前置失败42566727617不当所有权行为红。

42567946141修复源2fc650b30a3ab16792adb7babe566b8cec1bfeb5 clean、tree314894b33ed72a5ff2aeed47504d6c9e7f575bd9、snapshot78c6fb8eabf86ff755800188d9f92e64f8d4e3cf、control0cff9671fb2f80945423132e0df46995f6ff3198：根格式/net66+1ignored/hop9/wires10/journal28/dial_budget7全绿。Mac26.6.2 arm64/Rust1.95/jobs2；预算子集1.67秒为测试运行时，不是性能SLO。Accepted交接保留Aborts owner，回答等待沿原绝对by；取消/到期停止本流并保留共享健康wire。

这批证明已测的锁、scope与Accepted回答合同，尚未覆盖普通交换/真实死wire/一次读重试/写入失答/跨IPC剩余预算/完整后台及并发释放。未执行Linux最终矩阵、全Quality或真机/能耗，不宣称N2或全计划完成。原始证据在mac-accepted-owner-candidate-42567946141，失败与fixture修订原因保留RUNS/ACCEPTANCE。


## N2普通wire与配对探针

42569954515新增四项普通wire覆盖实际绿（dial_budget11、其余既有子集绿）：普通1秒到期仅STOP本流、同时等待的第二请求在同stable_id成功；真实读重试沿原2秒by；失答Sessions严格2次/Mkdir严格1次。生产不变，未宣称这四新增用例在旧源实际红；长黑洞F7尚未验证。

42570772550以原pair/doctor取得三行为红，旧11绿；完整GET/配对pending/fast已204且在单HTTP2.6秒前等前置已过，失败仅在取消未关闭HTTP、pair取消残留与快second被slow first挡住。
42571393016 based4a2e115b898fc337f5091b51fa71e45740c667f clean、treef278b9330f51268182b142729db2c86687f4af4b、snapshot3459894c81cb994310a97bfcba4ca39ae97e569b、control0cff9671fb2f80945423132e0df46995f6ff3198：根格式/net66+1ignored/hop9/wires10/journal28/dial_budget14全绿，预算子集9.93秒是运行时不是性能SLO。Mac26.6.2 arm64/Rust1.95/jobs2，真实取消后TCP EOF及首成功均在冻结观察内；测试server未提前关闭制造EOF。

修复仅闭合probe两层任务所有权、按完成首成功并取消输者；未增加并发/改变2.6秒预算或proxy。ears_heard也是该函数的直接caller，HTTP boolean映射保留，完整echo/故障场景未由本批证明。原始source/env/raw/classification均保留；没有Linux最终矩阵、全Quality或真机/能耗结论。


## Turing首轮恢复（U9，2026-11-09）

用户授权Turing作为Linux主通道后，candidate turing-20261109T075531Z-b6277f48准确treeef392bb7/snapshot8ff966db/control59dc223，3轮recovery全部6/6绿，原p3最坏<1s保持，实际518.5/597.9/706.9ms、均零失败/退役；p2改变慢路后分别3.1s/6.1s/930.4ms达原断言、typing errors空。每轮样本/原始scenario日志在对应evidence/report/netlab/，非跨host改进结论。首编译6m19s，后续repeat0.58/0.53秒；基线同Turing同工具链/同driver原门正串行运行，首次编译55.57秒。

源码全量tuple、原字节SHA、toolchain、namespace、host资源、执行状态、三轮全部结果、原job group检查均留存；采证collector576c127独立于执行control59dc223。只证明此source在Turing recovery profile绿，N1 CPU/hop/full、N2–N5和真实设备未结。历史托管Linux42542984111两个p3红不删除、不据换硬件称已定因。


Turing recovery同机对照已完整采证：

| 角色 | p2 p95三轮(ms) | p3最慢三轮(ms) | 结果 |
| --- | --- | --- | --- |
| candidate(c55/treeef392bb7) | 70.704 / 71.865 / 70.794 | 518.5 / 597.9 / 706.9 | 6/6 |
| original baseline(6485/tree52a44260) | 73.692 / 71.256 / 247.204 | 787.0 / 787.1 / 704.3 | 5/6，p2第三轮p95超原200ms |

两源同Turing/固定Rust1.95/原fixture/control59dc223，串行candidate→baseline；两源p3均满足原1s。基线第三轮host负载变动（candidate end load1=9.12，baseline end=46.12）与时序未隔离，不能把尾延迟差异直接算源代码或某patch收益。保留基线原失败、全部样本/断言/环境；数据不是稳定性统计证明或完整N1收口。下一为两源hop/stalls与CPU受控/必要复验、dynamic/full；N2–N5仍未结。对照JSON含两边load before/after与全部tuple路径。


## Turing修正占用计量后的hop（136th）

准确source tree78d063d915f033d9bffa58516488b65de9f0f14b/snapshota828463e/control59dc223，原三场景单线程三轮、hop_occupancy_started_v2，结果8/9，三轮2/3、3/3、3/3。新reducer此前独立实际2红→4绿仅证明started纳入占用，旧candidate8/9完整红仍保留。

| 场景 | 三轮结果 | 限制 |
| --- | --- | --- |
| f4 | 3/3 | 场景最大按键3.5/3.3/3.5秒，原门通过不是交互极致 |
| f4s | 3/3 | 回carried相对腿恢复0.2/0.0/0.0秒，无后续回退；最大按键8.6/10.2/10.1秒 |
| f4i | 2/3 | 首轮恢复后X carried→relay→carried，to_relay=1而原门0；后两轮绿，第三次回carried91.4秒 |

首轮f4i首行started relay条件通过，失败来自后续回退；精确X事件carried→relay1697ms、relay→carried5803ms。journal没有connection stable_id，不能只凭该报告区分连接或定因；未归咎reducer或改门。3轮完整log、source/driver/bundle/environment、原PGID无live与逐文件SHA已采；执行1652.41秒。尚无corrected baseline完成数据，不给因果加速/稳定性结论。新Linux net-node与基线hop/CPU对照仍待，完整计划未完成。


## Linux真正长静默与取消覆盖（141th）

Turing141th turing-20261109T095017Z-93c2c1a2，base9eafba74/treef93c32b1/snapshot6708accb/controlb998479，固定Rust1.95，根格式/编译/net66+1ignored/hop9/wires10/journal28/预算取消探针16全绿，完整原始报告已采。新F7在单独unshare -Urn的user/netns核隔离后安装两向UDP端口丢包规则，真实request/probe命中drop计数、heard不再变；ACK+all-path grace后abandon真实Pending future恰一次unanswered退役，远端仍open、其余失败计数零。长ACK阳性同wire并发请求仍成功。

前Mac UDP代理前置12→15收包以及140th nft语法错误均按fixture红保留，没有当产品退役红；新覆盖通过不声称旧源实际红。该结果只证明已测长静默合同，未证明grace之前取消的备用可达性、跨IPC期限/取消、完整N2或全Quality。21.24秒为子集运行时，不作性能SLO。当前基线hop137th已准确启动，仍待完整对照。


## Turing corrected hop两源观测对照

固定相同五fixture/control59dc223、hop_occupancy_started_v2、原三轮/门；candidate136th tree78d063d9，baseline137th tree97040f31（6485生产原样），完整报告均采核。

| 场景 | candidate | baseline |
| --- | --- | --- |
| f4 | 3/3 | 3/3 |
| f4s | 3/3 | 1/3；晚回退两红 |
| f4i | 2/3；一次回退红 | 3/3 |
| 合计 | 8/9 | 7/9 |

baseline f4s首轮Link已恢复、稍后约35.6秒Relay/37.4秒Forwarded，原after-leg门失败；第二轮carried后26.2秒再选Relay、约0.7秒后再carried，原30秒无回退门失败。两源均有约8–10秒按键卡顿，candidate f4i一次回carried91.4秒。结果只证明该批观测，不能按总通过数认定体验完成或单patch原因；多连接事件缺身份，进一步诊断候选已加wire字段、尚待验证。所有tuple/round/host load在turing-corrected-hop-comparison.json，原红不覆盖。


## F8：操作到期过早退役共享连接的真实反例

Turing143rd（tree42050b56/snapshoted287397/controlb998），root fmt/编译及旧16合同绿，新F8真实红。新私有user/netns、原生UDP握手后lo每向2400ms；两个真实Sessions请求在2401ms到达并被server回复。首调用4101ms正常CallerExpired，此时heard16→16、回包仍需经过固定延迟，原全路径grace尚未到；wire已被Drop退役（unanswered1），控制请求也失败。所有隔离/时序/两请求前置都先通过，才在共享wire保留断言失败，不能归fixture红。

根因边界：RetireUnheard::drop仅凭4秒ACK floor与计数不变关闭wire，而exchange_inner会先probe所有路径并等grace；取消不应跳过该证据。尚未修复；保留F7真正长静默控制，不采用一律禁用退役或简单凑新阈值。

Mac146th（treebbed6503/snapshotc8035ce2/controlb998）另取得17便携/29journal全绿，包括真实update_dial200ms早操作by、30秒操作下10秒自身cap，以及wire身份字段旧日志兼容/roundtrip/人类句子不变。其17排除Linux F7/F8，与Turing143rd17并非同一集合，不可替代真实F8红；30.57秒为单元子集用时，不作产品SLO。


## F8共享确认修复：Linux21 / Mac19同源绿

base25d0c803/tree6805b7b7/snapshote0a42573/controlb998：Turing150th fmt/编译/net66+1ignored/hop9/wires10/journal29/预算取消探针21全绿；Mac151st42605391960同一snapshot对应19便携全绿，Linux-only F7/F8排除。F8函数与143rd实际红逐字相同，接收计数/两请求/4.1秒到期前置保持，修复后同wire控制请求成功且零误退役；函数SHA与完整原始报告可追溯。

正常exchange和取消共享Slot拥有的一份全路径probe/count/grace，调用过期不跳过宽限；fresh receive、replacement、wire清理中止旧确认，任务仅Weak持有，原runtime handle允许在std thread Drop发起确认。F7保留同真实黑洞及一次退役/远端open/零其他故障，仅观察改为owner自身确认后退役，已先记录v2协议；replacement后真实Sessions成功、pending不保留owner也通过。独立确认仍可在黑洞证据充分后记Unanswered，不能把CallerExpired本身当peer故障。

150预算子集46.37秒、151子集32.31秒是测试耗时，不是体验SLO。只是这一边界红→绿，resident tick较长grace协调、Accepted长取消、其余N2与原hop回退/长卡顿、N3–N5仍待；新源CPU对照152→153正在推进，不提前给性能结论。


### 2026-11-09 Turing 修复后候选CPU停顿：原两门绿，体验仍有缺口

152nd turing-20261109T110456Z-dee800e0与Linux150/Mac151同准确源码（base25d0c803/tree6805b7b7/snapshote0a42573，controlb998），原single-thread/repeat1的p1s/p5 **2/2**。600秒20×300ms CPU停顿，alpha/beta0/0路径移动、零退役；但按键最慢3.8秒，alpha12打洞/3449包/1335065B。p1s原门只约束移动/退役等原assert，不因绿色忽略长尾与开销。p5 150/300/600ms各4次，三档均零relay移动/零打洞，最慢279.9/518.6/992.0ms。

完整report与source/driver/bundle/hash/原进程组已采核，classification存run目录。总1193.92秒，host load before4.90/6.36/7.33、after5.58/8.26/8.90；baseline153仍在途，尚无本轮前后因果结论，不跨旧硬件/源比较。原报告只保留scenarios/task/build日志，无额外逐包journal；打洞/长按键定因需后续独立诊断。完整N0–N5、hop/f4i、交互批量、真实手机/能耗仍未收口。


### 2026-11-09 F9/F10共享确认红转绿，Linux24/Mac20同源

158th在全部真实namespace/UDP静默/drop>0/remote-open等前置之后，两项owner确认目标失败（原21过）。生产候选df592ad7对齐probe计数基线、巡检等待pending proof，并给Accepted响应等待同样的取消guard；长等待历史ACK不再等同永久健康，但取消只请求全路验证，只有owner原grace内仍无新包才退役。正常完成的guard早返，不额外读取paths。

160th Linux24全绿（合同92.30秒）；161st Mac20全绿（50.01秒），两者准确tree3c8bc03a/snapshote30f6377/controlb998。新增16秒健康Accepted取消，等全路grace后再发独立控制仍同wire、零退役；原短取消/期限/读写/探针/update及LinuxF7/F8保持。F9/F10core helper/F8相对红158逐字一致，身份文件f9-f10-f8-red158-candidate160-function-identity.json。全部source/env/control/transport/hash/原group已采核。

这只收本批Node合同：无pending时tick/served静默、完整resident/跨进程期限、长预算/候选资源、N1切路与N3实际应用/能耗等仍未完成。CPU152/153原两门均绿，但3.8秒vs732.4ms及12vs5打洞不支持整体提速；绑核/busy有效负载仍未被旧fixture断言。新准确源hop162三轮在途，不能预记稳定性或最终体验通过。


### 2026-11-09 同源三轮hop仍8/9，载路消失需定位

162nd turing-20261109T121345Z-8ba7e561同Node160/Mac161准确源（df592ad7/tree3c8bc03a/snapshote30f6377/controlb998），三轮3/3、2/3、3/3。第二轮f4i：X wire139750384102992在1794227442646选carried（quality85.61ms、8samples、age141ms），1794227449993选relay133ms，只有relay留在measurements，间隔7347ms；这是同连接真实回退，不能由跨连接聚合解释。Y该两聚合事件wire不同，必须分开。open_hop后约10秒与2×5秒清理节奏吻合但非原因证明；下轮独立diagnostic记录attachment/IO/清理决定，不先调排名阈值。f4s三轮门过，最慢按键均9.9秒，不是体验绿。

### 2026-11-09 CPU有效性校准3/3，不代替完整性能对照

166th source886b084c/treeed97f217/snapshot349bd7ba/control7534，短校准3pass（0.50秒），Linux实际CPU14允许集合0–127，两个sleep模拟resident和两个busy contender全thread绑同CPU；305ms各增加15CPU ticks，clock100ticks/s，且负例/reap断言过。新机制支持测量前/尖峰/结束的实际证据，尚未跑真实Node600秒p1s或原版同fixture对照。167th Mac同完整source根格式/net66+1ignored绿，仅net-unit。所有report/环境/driver/transport/hash已采。168th同源f4i生命周期diagnostic三轮在途，明确非SLO，清理政策仍原样。


168诊断3/3没有重现162原same-wire撤路，全部27生命周期事件留证，不能用无复现替代修复。N1继续验证peer共享载路的连接所有权范围。CPU169/170用checked_affinity_v3相同真实Node fixture作原版/候选串行对照，生产基线纯净；结果未出，不预报性能通过。


CPU169未进入测量：真实Node CPU admission返回无路径ENOENT，不能与170构造性能对照；170原版未启动已保留并superseded。下一步只增加错误路径上下文和真实Node短准入，不放宽压力/进展/affinity断言。用户U10明确跨网实机A=macserver、B=Turing（Linux），P=iPhone；设备身份已确定，A↔B可独立推进。


172真实Node短准入红把169类失败定位到枚举与读取之间的线程退出（具体task status ENOENT）；先修fixture快照，不改产品或压力条件。修正仍必须覆盖每个存活thread、PID identity与真实CPU ticks。173仅format红，无多连接行为结论；共享载路清理假设仍待可重复合同。


174实Node短准入4/4绿，所有549个存活线程实核CPU89，真实echo与两busy CPU进展通过；172线程退出竞态修复取得真实短case绿，未覆盖600秒p1s。完整A/B将明确同一allowed CPU中间ID，原门和压力不改。175仍仅formatter红，N1多连接合同尚无行为结果；生产清理判断不提前改。


共享资源判断在178取得真实反例：peer共同selected tuple在outgoing上选IP，但incoming仅Custom仍实际收发、其watcher selected为空。只比较Main即允许释放peer carrier，违反现存消费者合同。候选改为检查所有live Main/Tunnel/incoming；非空且无Custom不占carrier，未知快照保留，含Custom按原gain判断。178整fixture字节冻结待红→绿，原162因果与完整hop仍须复验。176固定核原版2/2与22期实际CPU证明已留；p1s未留echo error的旧缺口明确，不能推零失败或延迟SLO。


178真实共享载路policy红→179 Linux24/wires11及180同完整source Mac20/wires11绿，整fixture/两清理控制/所有物理前置逐字不改；质量gain/2×5秒及vendor不变。只是一个已证实判断缺陷的合同修复，162原f4i因果与完整体验仍待。181以与176逐字同CLI fixture/netlab.sh及CPU64做候选原门压力对照，182同源原三轮Hop随后串行；结果尚未出。


176原版/181候选同固定CPU64、相同fixture/control的单对顺序观察：两者旧稳定性门均2/2，候选18punch对照3、最大请求耗时728.3ms对537.2ms，流量亦更高；单对不是因果/总体改善证明，探测成本需追。182首轮f4新红但f4i过，整个N1不能收口。N0 v4量测源码新增完整caller outcomes/故障时标及真实五请求独立校准，尚未远端证实；旧无记录日志不能推零失败。


## 2026-11-09继续：移动通知反例与构建复用

用户U11/U12要求把peer relay和移动IPv4/IPv6变化作为完整最优连接验收，U13要求利用Turing加速。补充要求已入PLAN/REQUIREMENTS/ACCEPTANCE，均不以单middle/IPv4-only实验替代。210在原生产Interfaces/interface_addrs上实际29旧合同绿、新1红：接口集稳定时，QAD-only候选更新仍把原1秒通知期限往后移。功能10ae1ca0仅让相同seen集合不重置timer；211 Linux/212 Mac同准确source原整fixture转绿（journal30、预算24/20）。原1秒和空网/同址恢复/来源翻转合同保持；物理移动与手机仍待。

Turing稳定源码缓存首/复用同快照206/207均fmt/net66+1ignored绿，build17.05→0.41秒，总51.52→35.84秒；jobs16和网络门不动，一组对照不外推普遍加速。Peer relay201参考9/9与205/208诊断各3/3虽全请求成功，仍见慢轮/双向不对称和208当前owner在稀疏质量下释放再重建；当前公式不要求selected反向显著更快，置信/候选试用/成本回收须继续设计验证。完整N0–N5未完成，证据与限制见STATE及原始报告。


215进一步取得物理前置：fresh双栈namespace真实IPv6同/64换址，三阶段内核记录确认旧址退出、新址存在且可达，IPv4地址及连通性保持；8packet正反控制全过。该校准为后续Node迁移实验建立注入几何，不冒充业务恢复或手机体验；IPv4/NAT映射、IPv6前缀变化、peer-relay移动、实际macserver↔Turing与iPhone仍未完成。执行器format错误路径也经小Git/stub合同红→绿，避免formatter污染稳定源码缓存；后续新mobility Linux任务自行先格式检查，减少不必要跨机往返。


## 2026-11-10：手机自动接管失败与三个休眠状态竞态

用户回Wi-Fi后MacLab不通而Turing/hinton/lecun可达，并明确中继/peer relay未及时接管；恢复依赖反复关开iPhone App，某次才经Turing成功。记为**自动恢复失败/人工恢复**，并未独立证明OS process restart。已装混合版本采样与CLI总耗时不能替代phone→MacLab准确恢复SLO、当前候选或能耗验收。

源码找到并各自真实红绿验证三个有限问题：同tick旧suspended关闭覆盖新wire已收包；旧retire释放slot后迟到Asleep覆盖新adopt；旧拨号Asleep提交/排队失败复用覆盖同peer真实反向连接。前两项3a3及f9各自同source双OS全net-node绿；第三项fccc/tree714/snapshot2b5/control68，Mac旧543/42806129212实际24/2红→42806745387的26绿，Linux61121d16的30绿，双方全部12 profile绿，原whole tests与新whole fixture保持。peer-contact-fixture-freeze.json仅该scope true，field_phone_issue_resolved=false。修复无生产timer/probe/重放，120/600秒安静与原退避/总期限保留；没有安装到手机或改生产进程。

所有负面记录保留：最初fixture格式、cleanup panic、pulse字段编译遗漏、Mac反向握手setup超时与portable fixture修订；Linux99534d56原remote32在private-node pairing准备60秒超时，观察过Cargo DN等待但因果未证。原源码/控制/fixture/期限复测611全绿不抹去该失败，也不关闭独立pairing稳定性。下一正验证取消半成品peer relay的所有权，之后才实施并行寻路与网络/地址epoch缓存失效；这些尚未接受。


## 2026-11-10 13:20UTC：已使用载体所有权与完整准备期限

真实旧行为a75/a038/871在56d2ca30为hop17/4红：新Custom-only目标经真实Tunnel完成TLS、固定echo且路径为Carried，adopt该live Main wire后取消准备者，载体/Asking/后续echo一起消失；正常publication与此前15例通过，三个prepare期限红保留。更早8475/ffd/ceaa在9484048e已实际16/3红：admit后默认3秒或caller1秒均不结束，真实持有新Incoming不admit时caller超时误记MISSED。没有用假拨号或放宽原期限。

候选PendingHop持Asking和handoff，正常交付及取消时已被registered live selected-carried连接使用的载体都交给既有watcher；无人使用时仅撤本serial。Wires提取原live连接收集复用，compare评分不改。open_hop入口单绝对by=min(原3秒,caller)，tries和offer共享，不因阶段重置；caller先到或已admit的本机公布等待不记middle miss。无新生产task、timer或probe。

首候选921/06db/23db两端新增hop21/0绿，但Turing a1b1dbe9及Mac42821367661原leased IPC32均栈溢出/SIGABRT，后续profile未执行，该source不接受且原件保留。仅在两个实际cold open_hop调用处Box::pin，原线程栈、fixture、时钟与业务语义不改。最终a35/95e/c74/control68在Linux1952febb（265.399秒、21artifact、原group空）和Mac42822434248（exact artifact13766619208、20files、11controlblobs）同canonical全部12组绿，hop21、wires15、budget30Linux/26Mac；原IPC32、remote32、32明确STOP、resume与canonical caller均通过。

原offer整个c071前缀、新prepare/published whole、原hop/wires测试尾及前几轮fixtures逐byte保持，详两freeze JSON和boxed-fixture-audit。此证据只接受这些合同，未覆盖incoming-only无selected标记、late-close/new-offer/pending Noq、ears/network/address epochs、并行寻路、WAN/真机自动恢复或能耗；没有安装到手机。高host load下普通功能回归不作p99性能数据。


## 2026-11-10 13:39UTC：incoming-only共享载体使用事实

旧a2ed/19ed/f118在07e04308实际hop22/1红：同peer独立outgoing真实IP连接继续echo，incoming真实TLS/echo且仅Custom无selected标记，客户端实际Carried；取消post-offer准备者后kept=false/owned=false/echoed=false。正常incoming与此前21例通过，198.966秒/11artifact/identity核/group空；52310a20仅格式失败不算运行红。新incoming whole冻结，原published文件准确前缀、prepare whole、原hop/wires tests等均保持。

候选仅carrier_in_use扩展为selected-carried或非空且全custom的registered live连接；空snapshot不当作使用，原compare评分不改，无新生产timer/task/probe。69a6/tree8536/snapshotbb754/control68在Linux4c316093（249.068秒、21artifact、原group空）与Mac42825268619（exact13768467392、20files、11controlblobs）同canonical全12组绿，hop23、wires15、budget30Linux/26Mac，原1 ignored保持。hop-incoming-fixture-freeze.json仅该scope true，field_phone_issue_resolved=false。之后的late-close/new-offer/pending Noq/ears/network/address epoch及并行仍未闭合。


## 实际actor关闭guard：排队顺序与原vendor验证

Iroh13加法close_carried_if在实际RemoteStateActor执行之前检查最新Links.has，原无条件close_carried语义保持。旧8048/73ff/5e19的761c4291真实hop24/1红；独立baseline仅恢复实际旧Node无条件调用，其余加法API未使用，817a/35e9/036a在2cf6b5ef实际27项25/2红：late和已入队后新stream两个场景都Closed=true/carried=false。真实actor暂停、Node已enqueue/join、后续实际replacement与两无replacement关闭控制均到达，126.082秒/11artifact/group空，初格式失败保留。新旧whole queued及原close完整前缀保持。

最终47aaa/tree4a4055/snapshota61643/control68在Linuxb9f10cb1（250.769秒/21artifact/group空）与Mac42833751584（exact13772163647/20files/11controlblobs）同canonical全部12节点组绿/hop27；同source Linuxvendor99315801全Noq439/Iroh路径50/home9绿，61.826秒/14artifact/group空。原vendor测试尾逐byte未改，两个freeze仅actual late/queued关闭scope true，field false。pending Noq开/关/Abandoned、ears/网络地址epoch、N1稳定性和手机验收未被这些证明替代。

## 2026-11-10 ordinary并行与pending消费者分项

99d8ec5c/tree7f6c0b7b/snapshotef1ef86b在Turing1446794a和Mac42849256788同canonical通过全部12组，hop36/wires15/budget30（Mac26），没有安装或发布。旧f51/0e9314a5快IP真实32/1红，旧839/085ea57e未登记/pending真实33/3红，fixture原样通过。hop准备与普通拨号共用一个期限，原两项均失败后可补一次新carrier握手（最多2活动、3 lifetime），普通成功先adopt后drop准备；pending消费者交给原watcher等待并在取消后回收。Iroh/Noq未加新补丁/任务/探针。

此分项不含后来42fc5e3e已复现的successor新owner清理竞态（37/1红）、0RTT并行、旧Asleep提前返回、cached候选新鲜度、network/address epochs、真实第二publicRelay或phone换网/能耗；原854 core transfer Busy保留未归因。14ea初版Send编译失败两端原件保留，99以cold hop显式boxed Send边界解决，不放宽栈/期限/测试。

## 2026-11-10 pending successor清理分项

cbf3a8d4/tree38430683/snapshotff8e3735同source在Linux47439bfb（252.734秒/21files/group空）与Mac42851130886（exact13779833868/20files/11controls）全12组绿，hop38。旧58e0/42fc5e3e真实37/1红，旧owner结束且watcher已暂停后，新owner入场仍被旧清理拆carrier；无successor控制正常释放。现将最后检查与精确serial撤回放入pending admission同锁，新owner则原watcher订阅后再检查；不加task/timer，不改正常publication两LOOK。仅这个取消清理scope验证，不覆盖initial PendingHop Drop、normal watcher释放、Noq opened/abandoned或network/address epochs，更不证明phone现场解决。

## 2026-11-10 held middle恢复排序分项

2cb56cce/treeb9bd9865/snapshotb8610837在Linux06d07ccd及Mac42853193309同source全12组/hop39绿，wires15/budget30（Mac26）。旧8d0328/4e280e8e实际38/1红：两个cached1ms peer保持真实Incoming未admit，实际live Tunnel被排到有限TRIES外，caller2秒内won=false/echo=false/tried_cached=true，cleanup完成。现只Legs::Recovery先稳定分组当前held road，再沿原两腿cost顺序；不将Direct排在Relay前，不改期限/TRIES/单跳/策略。incoming-only first leg、假live僵尸与NoRelay其他策略/epoch仍独立未证。

## 2026-11-10 cold/resumed读取准备并行分项

c88653f9/tree4c7c81cd/snapshot6448871c两端同source通过12组/hop41（Linux2b3b076e 256.985秒/21files/group空；Mac42857032142 exact13782014733/20files/11controls）。旧serial+同v2 fixture9b680/b08656c7真实39/2红，cold实际请求0、warm只有预热1。现原early握手/回复task与hop准备合作poll，同一个timeout，adopt后即drop准备和pending lease，accepted response owner与原STOP/取消/预算合同通过。没有新增业务尝试、探针或生产timer。

v1曾在0f8a81d0两读取都成功却warm server is_0rtt=false，40/1失败保留；该marker在accept时评估，快速完成握手可清false，不能由此推出没恢复TLS。v2用真实client ZeroRttStatus::Accepted/普通握手/Rejected分支的cfg(test) one-shot，只观察不阻塞；warm Accepted=true/cold=false，RPC总数warm2（含1预热）/cold1和原1秒caller均断言通过。v1/v2 whole/旧serial与candidate各source单独冻结。初始串行校准临时wt已clean后非force移除，固定引用与全部原件保留。

尚未覆盖普通早失败→后中继成功、旧Asleep提前返回、initial Drop/normal publication release原子性、NoRelay其他策略、地址/网络epochs、真实两腿/手机换网/能耗。主53 vendor不变，完整N0–N5继续；没有安装App、合并或发布。


## 2026-11-11 main55 peer relay物理换网checkpoint（单轮）

923baaeb/treee8893d45/snapshot37d77b42同canonical先通过Linux828ae604和Mac42883511676全12组/hop55/wires16；main vendor54，不含隔离42–44。随后Mac重型结束，Turing0aeceb60在control68cad下按原netlab-peer-mobility-samples/profile运行，228.871秒/16files/group空与identity核；真实中间节点IPv4 replug、原20ms腿和两次10秒hold均核。

124/124业务全成功，replug结束后首AB/BA请求分别594472/584202us；独立原peer_relay_replug_warm_selection_within_four_seconds_v1合同50个warm请求全选carried，0失败。该记录不是手机/p99/每包路由，也不是从故障开始计时；旧隔离2ef四秒与cold-start红不改。预先登记再做两轮同源重复，未得到重复结果前只称单轮checkpoint，不归因某个补丁或宣布N1/N0–N5完成。


## 2026-11-11 main55物理重复性：固定三轮2绿1红

预先登记的main55-peer-mobility-repeatability-plan.json固定三轮均已完成，无丢弃/第四次重跑。准确923/control68cad/原profile与物理hold，0aeceb60与1cb99869原四秒合同50/50、51/51通过；b158e418原51个warm请求中4个失败，AB内层切回5514.252ms、BA3668.365ms。三轮各124业务全成功，共372，但N1时限仍红。所有identity/终态/group空及原合同核，结果main55-peer-mobility-repeatability-result.json保留。没有归因某个补丁、宣告手机已修或N0–N5完成。

第三轮慢方向+1514.299ms answered、+1514.370ms immediate hold ended，rehold=false；+3513ms初次Moving Hold、+5514ms Go，原2秒确认。quality_since=None，无recovery质量重启；重复故障hold假设被实际日志排除。99.688431ms与Relay131.830007ms不达原25%收益、另copy85.178119ms达标；代表copy由近期样本数优先选，是否等样本过期后才获选尚待证，不能写成已定位原因。没有改原期限、收益门或样本合同。


## 2026-11-11 Iroh46单元与双OS窄范围（物理待验）

1c847229/tree90f62/snapshot4c18在5c2e4ff2 vendor439/55/11通过，对应旧c775/eec09ec1路径53/2红（旧50及3负控过），冻结whole ae62及原测试尾相同。Linuxdc23e338与Mac42892759864同canonical全12组/hop55/wires16，Linux315.188秒/21files/group空，Mac exact13799886372/20files/11control均核。只确认有限首次baseline-return质量刷新与既有回归，未证明四秒/N1/phone/energy。物理三组源对照前瞻计划已登记，首9cb240b7在20:18:34 UTC启动；不覆盖旧三轮失败。


## 2026-11-11 Iroh46固定三组物理源对照：未接受

预登记C-B,B-C,C-B六次全完成，无第七次或丢失败，control68cad、原topology/20ms/holds/124请求/两个四秒门保持。C=1c847229，B=c7750419，同显式DEBUG。C结果9cb3855.026/4068.484ms红、71ea3851.354/3820.801绿、6aa双向无Go且未观察到carried红；B为f793557.596/3732.647绿、32663751.098/5469.355红、21353521.250/3484.977绿。744业务全成功不替代恢复门，C1绿2红/B2绿1红；known-quality-physical-pairs-through-6.json保留全部源和原始差值。样本少、copy数4/6与随机时序不同，不宣称显著收益/p99。

6aa质量epoch未刷新、steps0，X仅零星收到Custom包而Y未见故障后收包或新RTT。四端实际绑定outer快照曾选41–42ms IP且未关闭，middle onward质量年龄3.526秒；0tuple mismatch。仅据此不能定因46/Noq/应用转发。载体约21秒后结束，接下来补真实stream/frame推进诊断。探测预约合同全保留；inner Custom实际计数按两个窗口给上下界，6aa长窗口缺边界保留unknown，不能补0或算手机/outer能耗。单元439/55/11及双OSNode全12仍仅对应窄scope；候选不作为最终N1方案接受。


21:46 UTC诊断校准审阅：v4两次原prepare超150秒、未replug，已停止原max3计划，不以额外重跑掩盖；v5 ef0b单次883原124/124、warm51/51与decision3435.866/3412.927ms通过。独立v2 counter解析使用累计偏移与实际last progress，8个方向均单调且换址后继续推进；最终两端/中间匹配42752与43072字节。Ready事件bytes=0不能作无IO证据；写/flush不是ACK/交付。此次未重现6aa，不证明同步诊断输出导致旧准备失败，不作为N1接受。未新增正常策略/Noq补丁/探针。下一步测显式path PING在ACK延迟下的真实及时性，先独立合同，不能以推测冒充6aa根因。


21:58 UTC Noq13窄范围：wholetest v2e057与原multipath/parent还原SHA冻结，旧efb2/8a4a7521 Noq442/1红→候选2892/33bc9584 Noq443/0、irohpath55/home11绿（59.799秒/14files/group空/identity）。仅显式path PING Established+Data+peer支持时附IMMEDIATE_ACK，其余connection/mod.rs逐字相同；保留普通delayedACK、Initial与真实accepted0RTT交付三负控。v1候选4e0编译SpaceId/SpaceKind错误保留，没有把它算行为红。图谱0结果/旧HEAD不是风险证明，人工源audit另存。后续真实未协商对端负控、物理重复/quiet额外ACK datagram成本、双OS/最终native仍欠；不能当6aa根因或N1/phone修复。双OS新源Node正在执行，固定C-B,B-C物理计划已登记尚未开始。


22:24 UTC Noq13固定C-B,B-C四次终态，未接受N1。C f8 7307.560/3517.794ms、warm10/51失败；B 8f3 原业务/hold/replug通过但x最后非决策IO记录截断，原严格解析失败保留，独立Go5437.441/5401.818ms均晚，warm/cost未算；B fe3 3386.396/3596.237ms与C e712 3703.047/3670.630ms均双门通过。各124成功，496/496只是可用性。完整3次流均见字节推进，未再现6aa单向停顿。C1实测返约90.353ms的probe copy后另一copy100.025ms代表，采样预算用尽；仍需独立验证代表质量/确认一致性，不据相关性归因。第二对同4copies，reads_finished enclosing C353/B313 innerTX datagrams，IMMEDIATE_ACK C47/B16；样本少、选路阶段不同、非逐探针或能耗归因。没有第五次，未安装/合并/发布。


2026-11-11 05:26 UTC 接续结果

Iroh47准确8cd/tree4c4d/snapshot4e900在Turing3d36cba8的原动态三轮全部通过，共27项，15files与空进程组及身份已核。p2切换937.1–990.5ms，p3最慢481.3–546.6ms且各轮零失败。稳定性088e00f9正在执行。

NoRelay缺第二腿提示在旧2cd/Mac42963573470中三个真实入口合同失败，原55与已知Direct/递归控制通过。ff743调整Recovery guard顺序后，同fixture在Mac42964365563全12组/hop60过，exact13830134613/20files/11controls核。Linux及物理两腿验证继续进行。源码审计核生产只调整模式判断顺序，原N1的7.31秒和6aa单向停顿继续列为未闭合。


## 2026-11-11 06:22 UTC 实际增量

Iroh47冻结8cd/control68cad的原动态三轮27/0与长稳四项4/0已收齐，identity/空group/原测量模式及逐case原行核。长稳2475.166秒：三个600秒场景均0切换0退役，CPU20×300ms最慢578.7ms；原p5各150/300/600ms×4均0Relay切换/0punch，最慢299.1/495.3/948.5ms。不跨源推论后续缓存修改。

候选变化缓存旧92c530在Mac42966906265为63/2红；候选1d448已将原65全过，但新旧快照晚登记合同在Mac42968959558为66/1红。观察顺序修复6e5/tree3ac48/snapshot12ef在Mac42969959757已实际全12组绿，hop67/wires16/budget26/resume1，exactartifact13832933221及zipSHA54b210ac9fa7861e3fdbe7af9b2455db03053d81ac9152a53f4d202de59bf66e核。Linux与同ID地址/成功后旧快照合同待验。

Iroh47的固定物理C-B/B-C仍在第4项：C62030f1b为124业务/warm51全过、双Go3569.426/3536.190ms；Bfe19c4ed在初始业务10.028585秒超时、未replug；Ba43e01ff有124业务但warm11/51失败、Go7499.610/3473.251ms。已测两次换址8流方向均有进展，不是6aa单向停顿重现。第4候选817adf28运行中，禁止追加第5次，当前不作全N1/p99/手机/能耗或统计显著结论。下一步按STATE同源Linux与完整N0–N5推进。


2026-11-11 06:35 UTC：Iroh47固定第四候选817adf28已收齐，warm51/51、124业务、Go3820.207/3533.388ms。四次至此固定结束、extra retries0，报告confirmation-copy-identity-physical-fixed-four-result.json保存所有源/失败/原成本边界。三个实际replug均4innercopies与3+3探测；reads_finished enclosing aggregate innerTX datagrams为C328/B312/C337，字节23357/22403/23740，IMMEDIATE_ACK37/39/37。选路驻留与相邻边界不同，数字是整段inner业务/ACK/control/keepalive汇总，不能全归因补丁/单probe、外层整网或手机能耗。两C小样本不作p99/可靠性或全N1结论。最新缓存源6e5同源Linux52af3091在跑；另外成功后旧selection两fixture仍为test-only、先验证原行为，不把格式失败当行为红。


2026-11-11 06:58 UTC：6e5 Linux52af3091准确同源Mac42969959757全12/hop67/wires16，Linuxbudget30、Mac26，已收齐。后续成功后旧selection合同0a7/Mac42973738072真实67/2红；287/tree2780/snapshot8cdc在Mac42975178687保持whole37aed942不变，已全12/hop69/wires16/budget26/resume1绿，exact13835396432、zipSHA6b34b08ec3f76bfc92613e2018549c204547b68b17aef1718a7e51cef94ed5e0核。287最新Linux待原三轮netlab-hop计时结束，不能继承6e5的同源身份。

原f4/f4s/f4i三轮1501a97c尚在跑，前两轮各3/0。f4i回peer17.7/31.6秒为middle重新可达后路径收敛，已先通过公共Relay恢复；不等于首个业务响应或全程断连。第二轮f4s Y未记录离开/返回carried而原断言green，最长echo9.4秒；不把never/零时长当瞬时恢复。后续N0按真实故障持续、首响应、fallback/收敛单列原指标覆盖与长尾。三轮尚未终态，不宣告整组或N1完成。


2026-11-11 07:55 UTC：287候选现已准确同源Mac/Turing全12通过；原6e5三轮hop9green已收齐但f4i回peer收敛17.7/31.6/13.6秒，非首业务响应。新增同ID真实IPv4/IPv6换Tunnel合同旧行为两OS均仅这两项失败，cold自建Tunnel与不变首腿控制通过。首腿事实候选6c3首轮仅format失败，逐字应用远端补丁为1bd9692，双OS已各提交一次待核；无候选phone接受。完整范围/失败/准确收据见STATE，不能称完成。


2026-11-11 08:49 UTC：每middle作用域、总期限公平legacy准备与完整首帧receiver提交已经7d源双OS准确全部12组/hop76通过，旧第三middle/挂起/receiver覆盖实际红均保留。真正prepared有界竞争候选c2b在Turing084053Z-8d9819ab准确全部12组/hop79绿（343.488秒、21files/identity/空group）；三新合同为挂起第一status仍由第二胜出、两个Ready仅1次真实数据提交并取消输家、过期hint拒绝后legacy实际TLS/echo救通。Mac同源和原3轮物理hop矩阵只提交一次待核，不能外推为完整N2/N4。DESIGN已按实际所有者和证据更新，预算分割/receiver等待仍需高RTT与成本验收；完整N0–N5 active。


2026-11-11 09:30 UTC：比较源a73在准确同源Mac42992730535/Turing091507Z-2f94f130均全部12组/hop81绿；具体identity/controls/原总结与node-regression-verification-v1.json已核。修改前c2b原三轮hop矩阵084720Z-2463e1e5已全部9green、1564.564秒，f4i收敛28.2/17.8/34.6秒（Relay已可用后回peer的口径），不是首响应；原慢Echo/never行都保留。a73同原矩阵092344Z-a59e4d3f在跑，首f4i8.1秒仅初步一例，仍不满足极致恢复的完整接受；不称全部长尾已解决。新两middle实际竞争合同已在PREPARED-PHYSICAL-ACCEPTANCE.md冻结待实施，原三peer矩阵只有一个middle，不能用它证明并行prepared协议。hint上下文候选尚未行为验证，不与这两个冻结源混作性能归因。


比较a73第二轮已原3green，f4i8.6秒；f4s虽green但link采样含Idle（5.277→8.412秒），x到13.1秒才选回carried。保留此瞬态和约8秒尾巴，不由原green升级为最终体验接受，也不在未进一步证据时归因gate。第三轮尚在跑。


最终原物理对照a73三轮已收齐全9green，1285.299秒、15files/identity/空group：f4i8.1/8.6/8.1秒 vs c2b28.2/17.8/34.6。保留f4s第二轮Idle和第三轮最慢9.6秒（baseline9.3），不称所有体验指标改善。完整结果comparison-context-original-hop-result-v1.json保存准确source/控制/原日志和限制；n=3、非p99、不是首业务恢复，约8秒和成本仍待。hint0d1c source-v2（clean）已Mac42997902740/Turing单次提交验证中，不继承a73接受范围。
