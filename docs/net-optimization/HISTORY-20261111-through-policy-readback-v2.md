# feat/net 当前状态与接续入口

更新于2026-11-11 16:49 UTC。完整目标“完成计划”仍 **active、未完成、未阻塞**，范围为 [PLAN](PLAN.md) 的 N0–N5，root串行，不派发子智能体。本页顶部是唯一当前入口，后续时间段保存历史，不据旧“在途”重发。上一份STATE逐字保存于 [完整历史](HISTORY-20261111-through-kernel-calibration.md)，SHA `8172caf6900ae2196273ecdf86a4b8d17572537299392bfa1b5b1322eefbe540`；它继续链接此前所有历史与失败原件。实际worktree、收据、日志优先，不按摘要覆盖未提交内容。

## 当前执行入口：source-v2诊断编译在途，尚无策略计时结果

协议仍 [SCHEDULING](SCHEDULING.md) 的`workload_response_priority_v1`，v1源码只有格式失败、**零行为/策略计时样本**。原Mac **43060911492**（requestf1b86733b63a43d3b757ef8fd73f5fac、control68cad）已failure，exact artifact **13872249427**、ZIP **51add361cc992a2c49f1587db9ad55ba6d9f6e754e302b4813776f3628b9f5b8**，9files/11controls/identity；Linux **turing-20261111T164124Z-bd59e496** 已54.723秒1、10files/identity/原group空。两者format.patch逐字一致，只调整一处priority_applied换行，已准确check/apply提交 **1df76f64**；两个`format-only-verification-v1.json`和ledger均终态。不能当Node编译或实际priority读回通过。

修正后冻结 **B/source-v2：8371dbb024502c8a0fea952b0ea96fbff6a41f39** / tree **51310731fd24ed9e18fa27060770c3ccee9570ce** / snapshot **5d989256ae5e0362440141dff578ea2c94e87bb7**，`candidate-response-priority-source-v2.json`，clean/0excluded。匹配 **A：f324d9f84bcb9f001b18e924bebf6fd101dbe26f** / tree **edb11e743b349732c38543bacc2297dad6e92cbc** / snapshot **19f7a1426b2cbf6a52ecd3fc724295a4ccd56f55**，`control-response-priority-source-v2.json`。完整树只有link.rs优先级const一行增/一行删，PULL_AHEAD仍8，全部fixture/parser/协议SHA一致、实际工作区保留B；两臂source/snapshot各隔离ref pin，独立核验 `response-priority-source-pair-v2.json`。这是匹配控制，不是旧版历史基线。source-v1 refs/原件不覆盖，v2也尚未产生性能结果。

新control **5f1242f5f4d216dde60ad27be0d17a5958b7ef46**，本地未push；SSH传送准确Git控制文件，原资源/UID/flock/netns/期限门保持。source-v2另补实际启动取消回收回归：`lab::resident_ready`在未poll或等待中都持有Reaped，w3用真实sleep子进程验证两种取消后进程不存在；parser必须看到实际startup guard。fixture默认60秒启动门未动，不证明产品keeper后代或启动延迟。新metadata `test_netlab*.py` **11项1.392秒绿**，含缺startup guard/仍有残留的拒绝检查；无本地Rust/格式器/产品执行。

**唯一Turing在途：turing-20261111T164701Z-a5469ffc**，PID **252606**、child **252828**，16:47:23 UTC launch，tasknetlab-workload-wires / revisionworkload_response_priority_v1-source-v2-B-readback / diagnostic / control5f1242f / 上述B。stage exec77921已终态，准确前驱bd59已收集/identity/原group空。16:49同receipt日志：格式已过、完整netlab实际编译65秒成功，w3与w3-startup两项实际绿（0.06秒，含未poll/等待中子进程回收），正在第一轮Buffered诊断。**整体仍在途，尚未collect/最终identity/三轮priority业务核实**；不把live日志当最终接受。不重发、不穿插Turing任务。

Mac source-v2 **43061931813**，request **cd1cb21f0dcb4cdb8cd595609ccda8f0**，原builder-ref feat/net、net-node、实际control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**。submit exec84214已终态、register exec24561已终态，ledger pin；准确收据 `.git/khor-development/runs/cd1cb21f0dcb4cdb8cd595609ccda8f0.json`，证据目录`mac-43061931813`。尚未核行为/终态；查同workflow，不重发。

下一步：收准确v2 Mac/上述B实际priority getter/取消guard，成功后同control串行执行A真实readback；两者均满足前置后才按 **A B B A A B** 六block、每block三fresh场景做关闭诊断的计时，原完整人口/实际TC/probe/字节/期限保持。任何entry/source失败保留原件并明确新版本，不选择性重跑性能差的轮。原035九场景6绿/3失败、四残留手工回收不当自动清理绿；启动/配对根因、严重同故障fallback参照以及全部N0–N5仍待收口。

最近向用户明确：**未完成**，本轮对照预计1–2小时；完整计划剩余2–4天、低置信度，受新失败定位及真实iPhone/能耗验收影响。它是预估，不是承诺或接受结论；不要压缩后重新起算或声称已完成。

## 16:40 UTC工作记录：参照失败与策略准备

**目前没有Turing或Mac在途任务。** 原唯一Turing **turing-20261111T161025Z-0354726c** 已 **1248.401秒、退出101**。21files/source/control/identity核实；原九场景 **6绿/3失败**：首轮fr-carried配对90秒超时、fr-relay启动60秒超时；第三轮fr-carried启动60秒超时。其余六场景实际测得384对started/finished全部ok（含bootstrap/setup），不能用这些成功人口掩盖三个未进入业务测量的失败。`steady-reference-failure-v1.json`保留逐轮分布/失败原文，ledger终态；不是f4s同故障窗口对照。主机load记录保留，不据此认定CPU根因。

首次collection正确拒收：原group163255还活着四个scratch keeper/serve，均继承原163013的build.lock。确认UID1110、原PGID、start ticks、exe、scratch home、精确NETLAB_RUN_DIR及锁inode后，逐pidfd TERM，四份stderr原件和`fixture-residue-v1/cleanup.json`先后留证，最终group空才重新收集/identity核实。第一次cleanup helper本地embedded-null停止，第二次因原后代仍持锁而未进入修改；没有删除锁或改原日志。手工回收不算自动清理通过。**bc519eeb** 修复fixture在spawn后立即拥有Reaped、先停keeper再删home；仍待Linux实际编译/取消回归，原60/90/120秒门未改。

新的优先级候选和协议见 [SCHEDULING](SCHEDULING.md)。**尚未有该策略计时/收益结论**：固定PULL_AHEAD=8，只比较后续PathSlice响应priority0/-1；首切片/目录0，两树只允许该const一行不同。先Mac固定net-node和A/B实际setter/getter/lifecycle readback，再关闭诊断按预先固定 **A B B A A B** 六block、每block三fresh场景，共18场景/每arm九轮。原v4 fixture、完整失败人口、实际TC/探针/字节门保持；高BDP/prefetch/前台完整文件/饥饿/共享分连接/设备规模/能耗仍独立待验，不把这个候选当最终设计。

本机只运行metadata：control `test_network*.py` **13项12.951秒绿**（三个task各success/case失败/guard失败），source `test_netlab_workload*.py` **3项0.268秒绿**（包括真实priority readback的12合成子例与policy的8合成子例），bash语法/diff过。新control task `netlab-workload-policy` 显式NETLAB_DIAGNOSTICS=0/unset KHOR_NET_DIAGNOSTICS，保持UID/锁/namespace/资源/期限和失败继续/首非零状态；OPS allowlist audit `control-workload-policy-helper-audit-v1.json`。没有本地Rust/格式器/产品执行；合成绿不能替代远端编译或真实策略结果。

下一步：提交/冻结准确候选、原control68cad Mac net-node；隔离Git对象构造相同fixture的priority0控制并核唯一行diff。Turing空槽先验证fixture回收和两arm真实priority readback，再开始预注册计时。当前没有新run ID，不凭本段预调度重发。严重同故障fallback对照、原启动/配对原因、完整N0–N5继续保持。

## 16:13 UTC及此前已核结果

**turing-20261111T145336Z-bc4dbca9** 已389.740秒0、25files/identity/原group空，固定六轮全绿，共4320请求全ok、六份16MiB完全一致、实际TC/探针/全部人口已按原始行独立复核。`observer-comparison-v1.json`、helper audit、ledger均终态，不重发。两模式bulk目录p95均109–113ms、终端81–83ms；记录方式没有稳定改善。probe多数p95 1–2ms；目录数百ms计划迟到主要是前一请求未完成形成的串行积压，残余dispatch p95约1.2–2.1ms。完整数表/分解与限制在WORKLOAD；不当完整N3、CPU/I/O、p99或手机接受。

最新生产长稳 **turing-20261111T150352Z-de04bcc1** 已 **2486.648秒0、11files/identity/原group空**。同生产3de/source-v3，原p1/p1j/p1s/p5全4绿：p1/p1j/p1s各原600秒、均零切换/零退役；p1s20次300ms spike、32个实际目录RPC中该case20个，最慢651.2ms；p5的150/300/600ms各4次，共12 RPC，全ok，最慢318.6/504.5/966.2ms，各组均零Relay切换/零punch。`stability-result-v1.json`独立核全部32个logical started/finished与原30秒caller、22份CPU64实际affinity/进程start身份/CPU ticks推进。它是原稳定性门，不新加或暗称1秒UX门，RPC不是PTY，严重f4s/quiet/手机/能耗仍未闭。proof/ledger终态，不重发。

业务流诊断 **turing-20261111T154837Z-0ff8c054** 已 **259.658秒0、25files/identity/原group空**，04c67d8e/source-v3/control71f1eeb4，三轮全2160成功、三16MiB逐字节一致。实际双方每轮360目录+64切片/共1272 stream逐一匹配，目录/文件均为单一实际Main，唯一终端为另一实际Tunnel。`wire-stage-verification-v1.json`独立核原始人口/响应流/ALPN/offset及日志SHA；原TC/probe/guard也全绿，ledger终态。bulk目录API p95约113–115ms、native exchange101–103ms；server accept→本地write p95仅8.02/8.05/8.64ms、seat等待10–11us。不能用p95相减作逐条归因，未测accept前/每包交付，日志有扰动，不当N3策略收益/CPU/能耗。原始表及下一候选见WORKLOAD。

极端中继 **turing-20261111T155852Z-d8c22be9** 已 **175.621秒0、14files/identity/原group空**，同生产3de/source-v3/原control68cad。原f4s功能门绿，唯一实际目录RPC **8712.927ms**；原件中中继腿被故意封锁 **8193ms**，从请求开始至恢复8150ms、**恢复至完成563ms**。不能把8.7秒全称为不明原因的恢复后停顿；备用公共Relay同时被加2500ms/方向，尚缺同故障窗口固定备用路业务对照，无法据此定量认定可避免时延。`severe-hop-result-v1.json`保留全部9phase/真实hold/attempt与三个诊断log SHA，ledger终态；初始metadata审计误预期只有6个观测phase而停止，已把另3个held前置保留/验证，没有改动原件或门。功能绿不等于全部严重6aa/quiet/首响应/手机问题关闭。

**唯一当前Turing：turing-20261111T161025Z-0354726c**，PID **163013**，16:10:47 UTC launch；task **netlab-hop-reference** / revision **effective-route-steady-reference-v3** / baseline / 原control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34** / 同生产3de/source-v3。stage exec55450已终态，原fr-auto/fr-carried/fr-relay三轮顺序矩阵，每个真实双向各30目录、原准备120秒/等待60秒/hold10秒/30秒caller/间隔200ms保持。**尚未终态**，只查同receipt/PID，不重发、不穿插Turing构建/计时。它更新已知好路径的稳态参照，**不是f4s同故障窗口对照**；两者缺口不可混算。

冻结源 **0b9454472767d16bc4ff0e44b14bcf3a10256e3c** / tree **725b442271fd07d85a8aeaaac84598cd3f5028e5** / snapshot **2a969d8cb0b75d48cba18792d40bffbd04df024c**，clean/0excluded，证据根 `candidate-workload-observer-source-v3.json`。控制 **7e439456cd5941607270add201d33589ebb371ba**。本页随后docs-only提交不改变冻结实验。Rust fixture与已在Linux编译通过的a9ef逐字相同；新增最后提交仅严格parser/18子例，不能把编译或kernel运行当w3/w1/w2运行绿。

先真实执行 `workload::observer::w3_` 校准拥有者/取消记录；若失败，不执行六轮，原失败及parser结果保留。通过后依预先固定顺序 **Streaming、Buffered、Buffered、Streaming、Streaming、Buffered** 串行执行六个完整场景；一轮失败仍保留后续人口，首个非零退出码不覆盖。source-owned `scripts/netlab-observer.py` 校验原始日志、720人口/轮、真实20ms探针、容量/溢出、实际TC JSON/文本/接口读回和独立guard，不以总job绿单独接受行为。

协议 **resident_mixed_workload_v4** 详见 [WORKLOAD](WORKLOAD.md)。真实目录/PTY/16MiB，原三阶段各两类120请求/100ms计划/每类一个在途、调用2秒/bulk120秒/程序就绪与预热10秒总界均保持。两模式相同内存与父future定时探针；仅逐条打印或阶段末打印不同，flush单独计时。Attempt guard保留ok/error/timeout/blocked/cancelled/aborted，drop拥有者abort真实任务；无后台游离测量任务。缺样本/缺summary/探针上限/溢出不能通过。三次/模式、奇数配对只部分平衡，**不是产品加速、p99、完整N3或手机能耗接受**。close仍只是API成功，独立host/child退出待补。

control纯Python/shell stub **13项9.707秒绿**（含固定顺序、guard失败阻止后续、case失败不中断全人口/保留首错误），source metadata **1项18子例0.698秒绿**，bash语法/diff过，零本地Rust/产品执行。精确SHA/检查见证据根 `control-workload-observer-tests-v1.json`；OPS仅加入固定task allowlist，原UID/锁/namespace/资源/期限/launch/ledger门不变，helper审计 `control-workload-observer-helper-audit-v1.json`。

## 新业务流诊断源码与Mac在途

当前业务流诊断冻结源 **04c67d8ea0a41988bf070d9f7b66d328bca85874** / tree **acc229f8aa046ac6c1bdac041434351e3bbb6693** / snapshot **35e4e3c016c2bf26fbfc593c72995acd3838e2d4**，clean/0excluded，`candidate-workload-wires-source-v3.json`。此前source-v2的泛型Connection<T>不能直接读alpn；修正为已知具体状态的既有serve入口传入实际ALPN，普通/early分开取值，原握手完成/认证/0RTT门不变。只在原有opt-in lifecycle诊断下增加实际业务wire/stream/协商ALPN、slice offset/字节数和server accept后阶段clock；没有改队列/选路或写入语义，无新task/timer/map，默认env查询成本仍须测量。工作负载仅在计时后保留双方日志。

初始源baeef657的Mac **43046964107**：exact13868680296/ZIP SHA1813c2b93b74c3fb0c988ce8dc65ef1832e4a6a35d4ede31a59362cab4c3a434，9files/11controls/identity，仅link.rs格式红，精确patch已应用1a4816fc。source-v2 Mac **43047897939**：exact13866949683/ZIP SHAb8d627cf19cf6c200a27fa4da751907b839be496eab9832f2a138843ef045957，10files/11controls/identity，格式通过、net68绿+1原ignored，但Node在泛型ALPN处E0599，**0 Node行为样本**。原失败/partial proof/ledger均终态；首次identity检查早于artifact解包完成报FileNotFound，接受的是collection终态后的成功identity检查，不改原件。旧v1/v2不重发。

修正源 **Mac43048859146**（requestef579fec765949ce808ff51114beb9a0，tasknet-node，原control68cadb57）已 **success、20files/11controls/identity**；exact **13869072265** / ZIP SHA **fe354a6d94271e347926b11c1162e8318b57a23135907f877b438be54878e580**。固定任务全12绿：budget32/hop89/net68+1原ignored/wires16/journal30/IPC18+六单项；六initialCustom/两incoming真实执行，新增诊断已在Mac编译，`node-suite-verification-v1.json`逐原log重核，ledger终态。它不是所有1378 Node测试、Linux业务流诊断或手机接受。

Turing唯一仍 **de04bcc1**，15:31原PID/child活，p1、p1j各原600秒已绿，均零切换/零退役；p1j10轮punch，仍需p1s/p5完整原门。不缩短观察或注入，不穿插Turing构建/计时。结束后才按**诊断source-v3 / control71f1eeb4**运行workload-wires；本段没有重发或提前调度Linux新任务。

新控制 **71f1eeb4e18d5ea6e4c04ac5348c43eb52b4bd89**（本地未push）增加SSH-only固定 `netlab-workload-wires`：先w3 guard，成功后三个Buffered/lifecycle真实完整场景，一轮失败不丢后两轮、保留首非零状态，source-owned parser核全部原件。guard失败跳过三轮但仍保留parser失败；原UID/锁/namespace/资源/期限门不变。control stub **13项10.765秒绿**（两task各success/case失败/guard失败）；wire metadata **14子例0.034秒绿**，源9项metadata/control stub总0.989秒绿，bash/diff过，无本地Rust/产品执行。`control-workload-wires-tests-v1.json` / `control-workload-wires-helper-audit-v1.json`保存SHA/范围。此新诊断已在de04准确终态后提交0ff并独立核完三轮，实际范围见顶部；不把它当产品调度优化或完整N3接受。

## Noq5专属unit合同与隔离单门去除对照均已核

源码ea986be7新增四项，仅cfg(test)模块/入口，不改Noq生产采样或保留策略，主59补丁数量不变。准确候选source **b7171bcf9e4a4042e80d07a499eadf8e0a99c738** / tree **d877e1b305bbb68b16fbb5e1aeec860869d3079c** / snapshot **12b0f3ca54e29e4f4c817d2303d2097bb8ae5e3c**，`candidate-noq-ack-contracts-source-v1.json`。

**Mac43050978778**（request15b87066a422467593706941ba1bdbd5，net-vendor/原control68cad）已success，13files/11controls/identity，exact **13869222421** / ZIP SHA **424418d0dcd1b4f81e707c17c4c5d96d07a2a42debd4b9d0679ee14750a36f5a**；Noq **447**、Iroh路径58/选家11均绿。四个新ACK测试名称逐原件核实，`ack-contracts-verification-v1.json`/ledger终态。真实TLS/实际PING后，用原allocator/retention构造ACK-only单位尾账并回收最大记录；合法旧最大号不造样本、正常最大一次采样、重复/仅ACK-only不采样、未分配最大必须PROTOCOL_VIOLATION。尾部不是1000份线上发包，不证明上游或物理性能。

为证明测试能识别缺陷，另构造单门去除的隔离Git对象：**794795e7e4e3f8756dedcdd2db0dbbd494f356d5** / tree **4f9044797ec1d181dd1956a3caff0ec6a6f9edb2** / snapshot **dd73e4ff7d81e1df8618919312e5a29a0d93b05a**，`noq-ack-largest-ablation-source-v1.json`。只去掉RTT采样条件中的largest_newly_acked，实际diff精确1行增/1行删，三个完整测试文件SHA一致；feat/net工作区保护仍在，两个实验ref单独pin，未checkout该变体。`noq-ack-largest-ablation-audit-v1.json`及独立diff proof已保存。它不是官方上游/历史基线；预注册期望reclaimed样本计数断言红、其余三控制绿；以下实际结果匹配。隔离对照 **Mac43052478191**（request23628ba3db4b4b9691e51818ea23d716，原control68cad）已terminal failure，exact **13870301323** / ZIP SHA **3cfa138fc7db0f7ce70c47ccb63b516035dfb771dd3dd7ba56daa92cbea45115**，11files/11controls/identity。**446pass/唯一1fail**，其余3个新控制绿；唯一reclaimed测试正是在RTT样本计数断言实际6/期望5失败，不是编译/前置/非法ACK。`ack-ablation-verification-v1.json`逐门核实，ledger终态。对照保留原红，候选447全绿，证明本unit能识别该门；Noq1及全部59上游/物理/成本范围保持，无Mac在途。

## 最新已核终态

证据根 `/Volumes/disk0/Khor/evidence/net`，以下均已收原件、匹配source/control、核identity/原group为空并写source-refs ledger终态；不重发。

| 验证 | 精确句柄 | 结果和边界 |
| --- | --- | --- |
| 最新生产动态三轮 | **turing-20261111T141304Z-f2f17750** | 1309.406秒0，15files；p2/p3/p7及相位矩阵 **27绿**，每个实际RPC started/finished匹配且ok，p3全在原1秒门。`dynamic-result-v1.json`。空闲回好路仍16–17秒（原20秒门内），活跃约2.2–3.15秒；绿不等于极致体验、严重stall或长稳关闭。 |
| 观察器Linux编译+独立kernel三轮 | **turing-20261111T144048Z-f970f1d5** | 104.751秒0，17files；源a9ef/tree699cf94ccd2dd7f8bc97487019aa032181b27201/snapshota5bb7fe0754f296cd80c3f03713f0bb60dadda9c，controlea44ab27af3011821d188e0a500ad91b20ed17b1。完整fixture编译，三w0 kernel绿；**未执行w3/w1/w2**。实际TC单位/两方向数据另核 `kernel-profile-verification-v1.json`，full_N3_accepted=false。它是bc4dbca9启动时使用的准确predecessor；当前接班以de04终态收据为准。 |
| Mac观察器格式 | **43039285003** | request42116d54606f427d8931f52920d208fc，exact artifact13864796317，ZIP SHAd542cdc8b352f44494f944f83953427b25d89b30b6906da6cca0d00a3bd735e6；9files/11controls/identity，**仅格式失败、0行为样本**。`format-only-verification-v1.json`。准确patch已check/apply提交5b57160c13cf764fd0ff0a111ef7838c1e846335，随后a9ef编译通过。旧失败未改绿。 |
| 独立kernel原三轮 | **turing-20261111T140913Z-141b0ea2** | 94.220秒0，17files，df5b3f6a/controlea44；三w0全字节一致、实际12份TC JSON/文本、minRTT40.2ms和双向约7.52/7.55Mbit/s。JSON rate bytes/s、delay秒、lat微秒相互核对；真实qdisc drops18/21每轮保留，零随机loss不等于零drop。仅校准kernel模型。 |
| 生产恢复三轮 | **turing-20261111T134417Z-ce3b1222** | 568.791秒0，15files，p2/p3全6绿；p3黑洞后228/225/227真实目录RPC、worst644.5/664.6/590.5ms、0失败/0retired。`recovery-result-v1.json`，不是PTY或全部移动。 |
| 旧v3真实混合负载 | **turing-20261111T133630Z-2aff8ae2** | 216.845秒0，17files，3×720=2160全ok、三16MiB逐字节一致、每类每轮120实际bulk重叠。`workload-functional-verification-v1.json`/`workload-latency-decomposition-v1.json`。目录服务p95约107–112ms（空载62–66），终端81–84ms（43–44）；计划→完成目录289–320ms含观察器积压，第三轮空载自身85–93ms迟到，原因未证，不归因CPU/I/O或产品队列。旧cohort未做实际TC读回，不能追填为已校准或当v4对照。 |

## 已验证生产候选与实现边界

生产候选 **3de372a026488a64a794935600e47db29f2f6faf** / tree **ae76cd646749ca624d5e3e98af01f96099ebd0e0** / snapshot **a9f46eaa909b8cf45663395912798effb31b7261**，`candidate-effective-route-source-v3.json`。截至0b945447的observer/kernel/parser改动为测试/工具/文档；其后04c67d8e新增opt-in诊断/default env查询，生产文件有diff，已核Mac12与Linux0ff的明确scope。不能把3de动态/长稳原件称为04实际复验；Noq5新增仅cfg(test)，采样保护/锁文件未变。完整原件索引与SHA见 `source-refs.json` 及历史。

- Mac完整Node **43022962905**、Linux完整Node **turing-20261111T131248Z-5c2a0339** 同源全12绿；含六initialCustom、两incoming实际执行。短8 **678b0ebd** 不替代全12。
- Mac vendor筛选 **43023967463**：443Noq/58path/11home绿，不是全部59补丁专项或完整vendor。
- 四peer **turing-20261111T125643Z-6970ad46**：699.061秒0、37files、三轮全6绿。新f4r首内容1.263/1.253/1.223秒，每轮30持续样本约15.3–15.5秒，held/continuous/selected_every/carrier_both true。原始两端31响应/31匹配stream/每端单一business wire/一个outer commit，无detach/I/O-ended、首次在M恢复前。`physical-result-v1.json`。原f4控制业务最长仍3.6–3.7秒，不当所有长尾关闭。
- 已有carrier时原generic和一个显式Custom普通握手有界竞争；early保留原fast0RTT与一个借用Custom普通握手。原总caller/deadline、最多2活动、写/0RTT/认证resting边界保持；缓存失败不提前取消carrier。无新常驻map/task/timer/强Node，额外握手CPU/字节/能耗尚未测。
- Iroh49只在共享preferred不在本conn且唯一established/open路径时报告实际Noq fallback；0/多条仍Unknown，shared selection/发送/PathStatus不改。但rtt_of/Patience因此取实际RTT，质量/成本/长稳仍要验证。主vendor **59=Iroh1–41/45–49+Noq1–13**，42–44隔离不动，上游等价未新核。
- incoming所有权合同保留真实无marker carried-only前置和另一个真实同身份Direct/TLS端点。四installed-carrier原合同/1秒门和快速Direct负控保持。carrier在调用开始后到达/own prep已结束的唤醒仍是待覆盖假说，不凭入站业务活推定出站Link活，不提前加轮询。

## 保留失败与完整剩余范围

完整原始失败沿本页历史链与ledger保留，不删除、不放宽门或覆盖为绿。最近包括：Mac观察器/校准初始仅格式红；bd7e5f6c三项Rust绿但manifest被harness前缀导致总parser红，原件不改，修唯一marker后新141b三轮真实复验；混合负载最初E0599、三轮PTY预热超时、v2仍超时各有独立原件。后者定位CLI host_main startup=None、原Node host_args读环境变量未走CLI；当前fixture沿既有term_echo的显式OpenOnce argv入口，没有顺带改产品命令引擎。观察器格式patch前自己的probe草稿保存 `ops/observer-probe-window-before-format-v1.patch`，SHA32670fa1a6858125b20d947bdb6f862144554165e50e49c245d5ead70b050c37，再以a9ef重新实施，无用户draft覆盖。

旧573最终false/None/0、166 true/Unknown/1仍原门红，eb72前置红、412清理panic、旧物理f4r held/selected红、原f4 first-leg2059/2325ms与最长5.2/4.9秒均保留。严重6aa stall/quality_steps0/RTT不对称/carrier约21秒终止、约8秒尾巴、Busy854、resume32timeout、quiet/f4s/coldstart、Noq13物理7.3/5.4秒、截断8f3、Iroh47四项10秒/7.499秒仍未闭环。旧源长稳4绿、Noq704/15bulk不当当前完整接受，3样本不称p99。

完整N0–N5保持：
- N0最终UX门/足量p99/相同条件完整前后对照报告。
- N1恢复长尾、严重stall/质量/跨连接测量、quiet和比较成本；新同源动态27/恢复6绿已补，但最新长稳和严重场景未闭。
- N2真实同conn独立/同时IPv4/IPv6迁移、LOST身份、完整取消/owner/迟到/撤销、carrier在开始后到达。
- N3观察器对照后核真实wire/stream/队列/许可/发送缓冲/背压，比较共享/分连接与公平方案，再覆盖1/3/5设备CPU/字节/唤醒/能耗、独立host/child退出。
- N4完整双腿Direct/Relay/SSH、唯一/多middle、prepared/混合版本、独立IPv4/IPv6移动、整机/出口/登记故障域及运维设计。
- N5全部59vendor上游/退出、Noq1/5专项、最终Quality/native/proxy/多机/iPhone与一致报告。

MacLab/Turing同LAN；macserver是另一构建机，用户报告异网。localhost iPhone历史安装0.2.0，曾反复关开App才恢复，蜂窝截图仅hinton直连29ms。**候选未安装、无phone sampler、旧测试窗口已过期**；可信GitHub/TestFlight具体构建就绪后再交接真机，旧手机观察不是候选验收。

2026-11-11进度口径：上一份完整交付3–5天估计始于11-10，当前仍日级，暂估剩余2–4天、低置信，受严重故障复现、长稳和真机结果影响；不是从每次compact重新计3–5天。当前六轮预计10–15分钟出该批原件，不把单批耗时当完整交付。此前小时级预估过乐观。未宣称整体百分比或已完成。

## 执行约束与下一步

产品 `/Volumes/disk0/Khor/worktrees/current/khor-net` branch feat/net；控制 `/Volumes/disk0/Khor/worktrees/current/khor-build-net` branch feat/net本地HEAD71f1eeb4，旧两个untracked pycache保留。新control仅用于observer等指定SSH任务；原Mac/full/physical/生产dynamic继续 **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**。控制未push，stage用准确本地Git对象；Mac admission/GH dispatch原门未改。

本机source-only，无Cargo/Rust/Node/Xcode/fmt/产品执行；Git/Python源码元数据和control stub允许。Turing严格串行UID1110/flock/user-netns `/tmp/khor-netlab-feat-net-1110`，Rust1.95/jobs16/nice10/50GiB起40停/90min原门，无sudo/host网改动；Mac独立jobs2/shared cache/原门可并行。SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。实现与上述验证已授权，无main合并/发布/采购/生产部署、已装App/config/store/resident修改授权；fixture仅namespace scratch。隔离Iroh42–44树detached2ef6bee5不动。

图谱先行，本树无.codegraph不擅建，CRG部分/滞后，0flow不证明无影响；explore/debug/review技能已使用。source-refs.json仅root单写，stage/register未终态前不并行ledger写；timeout/EOF不是终态，查同receipt/PID/Actions不重发。当前stage已终态。

下一步查0354726c原三轮稳态矩阵/PID/execution，终态核全部双向人口、固定路隔离与原门。N3依据已核wire/stage设计prefetch1/2/8和后续文件stream优先级对照，保留首份内容与大BDP吞吐，补公平推进/1、3、5设备成本；这些候选尚未实现或接受。f4s先补同故障窗口的已知备用路业务参照，分清物理下界与额外等待，不能把8193ms强制断网全记为恢复bug。Noq1/全部59与N2/N4/N5范围保持。目标active，无外部阻塞。
