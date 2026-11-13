# 真实 Node 混合负载基线

2026-11-11，当前观察器协议 `resident_mixed_workload_v4` 已取得六轮日志方式对照和三轮实际业务流诊断，功能/人口/TC与各自观察器合同已复核；v3已取得下列三轮功能基线，不能视为v4证据。这不替代完整 N3 或最终用户体验门。沿用 Linux namespace / scratch resident / 原资源门，运行时冻结产品 tree、control、原始日志。产品调度和连接策略不随此实验修改。

固定两个真实 resident，公开 IPv4，配对后先达到 Direct。仅 x 接口双向配置：单向20ms、每方向8Mbit/s、40ms队列、零随机丢包。该配置的实际吞吐尚未独立校准，日志明确 `calibrated:false`；第一轮是测量入口和功能合同验证，不能把名义速率当实际容量、测得差异当拥塞控制收益。

实际调用链：目录 `ls_of → resident IPC → Main`；16MiB文件 `pull_path → resident IPC → FetchPath slices → Main`；终端 `open_on(OpenOnce argv) → reach → resident Tunnel → remote PTY`，使用现行 cookie/Version/Ready 协议。**当前终端本来就走独立 Tunnel ALPN，不能假定终端与文件在同一条 QUIC connection，或把创建第二连接当作尚未实现的优化。** 目录与文件是 Main 竞争候选，需随后按 journal/真实 wire 身份复核，而不是只看 UI 路型。

每轮按 idle_before / bulk / idle_after 顺序；每阶段目录与真实 PTY 各120个逻辑样本，100ms闭环调度，总720个。记录每个计划/开始/结束时刻、延迟、调度迟到、成功/错误/超时/前序终端失败导致blocked，原始记录不删除。2秒是实验调用边界，不是用户批准的SLO；终端失败后不隐式重连或把剩余样本丢掉。三轮用于定位，不声称足量p99或消除了顺序效应；前后空载阶段用于观察漂移，最终比较需要交叉顺序与多seed。

终端在 scratch home 打开真实 shell，关闭本地输入回显后 read/printf；请求不含响应前缀，每个响应含唯一phase/index，避免把PTY输入回显、启动命令或旧环形缓冲当成功。整个测试仅向新建scratch会话输入；现有会话和用户resident不参与。协议、流与测量future由父调用拥有，结束显式关闭会话，scratch清理继续兜底host pid。

文件由固定xorshift64 seed `6d697865646c6f61`生成16MiB，接收后逐字节全量比较；不得只以文件大小或请求返回认定完整。bulk最多120秒，记录完整开始/结束/字节/错误。两类交互至少各30个样本完全落在bulk真实在途区间内，否则本轮不是有效混合负载。每阶段前后记录resident wire计数，末尾保留两端journal。日志本身有开销，明确是diagnostic baseline。

首批只判测量完整性、所有逻辑操作成功、真实重叠、文件完整和scratch会话关闭；**不以其functional green声称延迟、吞吐、CPU、能耗或队列设计达标**。后续必须补独立链路校准、完整实际wire/stream身份、真实队列/许可/背压事实、同条件对照与候选公平调度、SSH/peer relay双腿、1/3/5设备、CPU/字节/唤醒及真实手机能耗。缺项保持在 STATE / PLAN 中，不把本协议缩成 N3 的完成定义。


v1首个可编译版本在三次PTY预热中均10秒未取得预期回复，计时样本为零，原因未确证，不当产品性能红。v2修正测试前置：Host Ready只释放启动脚本；先等程序实际输出就绪标记，再发送预热输入；标记通过printf分段生成，整串不在会被本地回显的启动命令中。Ready、程序就绪和预热应答仍共用原10秒绝对预算，不叠加等待。输入使用产品现有shell_start::input生成真实Enter/CR。增加仅启动期的有限原始PTY输出诊断，计时样本不打印内容。该修正尚未验证，不提前归因LF、产品或CPU；实际关闭仍只证明API回复，独立host/child退出验证待补。


v2仍三轮预热超时，有限原始输出仅为shell提示符、没有脚本就绪标记。静态调用链找到CLI差异：`spawn_shell_at`设置KHOR_HOST_STARTUP，而`crates/commands/src/lib.rs::host`调用host_main，后者明确传startup=None；Node自带host_args则会读取该环境变量。这个宿主入口问题另记，不将它归成网络延迟，亦不在网络基线中顺带修改命令引擎。v3沿仓库现有`crates/cli/tests/term_echo.rs`的真实`open_on`命令会话入口：OpenOnce固定token、显式/bin/sh -c运行同一个read/printf程序、同样的实际PTY/cookie/Version/Ready/Tunnel路径、同10秒就绪+预热总期限，样本/负载/正确性门不改。此版尚待远端验证。


## v3已收原件的三轮功能基线（13:42 UTC）

源3c1e852002e0ade43215c248e45caaf8a780b791 / tree66c5744b067f0a594da54d04485868938af55f07；control ebe8c8243ee639aa63aaa08a3202cbc9d1c0c8d8；Turing turing-20261111T133630Z-2aff8ae2，216.845秒终态0、17files/identity/原group空。三轮各720（合计2160）全部成功，三份16MiB逐字节一致；每轮目录/终端各120个样本完全在真实bulk在途期间内，API关闭成功。`workload-functional-verification-v1.json`和独立`workload-latency-decomposition-v1.json`与source ledger均已收齐，full_N3_accepted=false。

下表p95只按本组120个成功样本计算；每组失败/超时/blocked都是0，不能扩称p99或最终可靠性。单位ms，空载取idle_before；后空载原始数据也完整保留。

| 轮 | 目录服务 空载→bulk | 目录计划→完成 空载→bulk | 终端服务 空载→bulk | 终端计划→完成 空载→bulk | 文件有效负载Mbit/s |
| --- | --- | --- | --- | --- | --- |
| 1 | 64.349→107.011 | 65.417→296.407 | 43.767→80.774 | 45.115→81.886 | 7.120 |
| 2 | 61.795→111.742 | 62.889→319.670 | 43.363→82.103 | 44.747→83.314 | 7.127 |
| 3 | 66.466→110.273 | 140.546→289.425 | 43.821→83.887 | 120.563→97.632 | 6.735 |

文件完成18.850/18.833/19.929秒。有效负载率是实际字节/实际耗时，不是独立测得的链路容量。第三轮空载和后空载已有约85–93ms调度迟到；具体原因未定位，不能按系统load猜CPU、磁盘或产品网络。目录计划→完成约300ms含每类最多一个在途的观察器积压，不直接证明Node内部排队300ms。原始数据没有被“只看服务时间”或删除失败过滤；本轮没有足够原因直接采用stream priority、固定批量限速或新连接。

## 后续校准与定位协议（历史提议；实际进度见下文）

先在同一namespace几何和资源门、复用同一条件构造器下，独立验证内核TC瓶颈：明确TCP连接的64字节echo与双向各8MiB完整数据，nodelay、固定payload、预热另记、每步2秒/传输120秒总界，保存所有错误与原始时刻。读取实际qdisc/接口计数核单向20ms/8Mbit/40msbuffer，不能只信builder参数。校准作为模型/执行器证据，不是Noq或Node吞吐基准，也不替代UDP/中继双腿。校准门与运行版本在执行前冻结，不依据结果追改。

同时增加观察器运行时的独立定时探针，分解计划迟到是否出现在发起RPC之前；比较逐样本同步记录与有界内存记录/阶段末落盘的观察器成本，两种都有完整失败人口与取消/进程中断的缺样本判定。它们是测量模型对照，不当产品加速。之后再关联实际Main/Tunnel wire和队列/许可/发送缓冲，选择有因果支持的调度候选，按同条件前后对照复验；仍需多seed/交叉顺序、SSH/peer relay、规模与手机成本。

## kernel TCP v1 校准入口（实现与实际三轮复核见下文）

`workload::calibration::w0_` 为独立 ignored 测试。w0和w1复用同一个condition构造器，仍为单向20ms/每方向8Mbit/40ms队列；不运行resident或Noq来冒充独立校准。一个父调用拥有的服务任务、三个明确TCP连接，4次预热与64次64字节echo全部记录，随后upload/download各8MiB固定payload完整字节验证；upload要收到服务端全量验证后的确认才结束计时。连接/echo两秒，传输120秒，总界不在分片时重置；失败/blocked仍保留。服务任务所有退出路径归父调用，正常结束另核三个连接结果。

本版在实际运行前冻结模型有效区间：去除但单列4次预热后，64次完整echo的最小RTT在35–80ms内；两方向实际payload率各在6.4–9.6Mbit/s（名义8Mbit的80%–120%）。这是独立TCP/执行器校准的宽有效区间，**不是用户体验SLO或Noq性能通过线**；不能看到结果后追改。所有原始echo/失败/字节/时间都保留，不以最快一项代替其余分布。

x设备与入口网关各自namespace内，在测量前后保存`tc -s -j qdisc show`、`tc -s -d qdisc show`和`ip -s -j link show`；入口方向实际在网关veth出口，不能只查x的eth0。自动功能门只核命令成功、netem/tbf存在与上述数据/物理区间；原报告明确`calibrated:false,qdisc_units_review_required:true`，因为实际rate/delay/limit/burst单位还须核原件。发现缺失/未知单位时保持未校准，不能把builder调用成功当生效证据。独立核对原始读回后才能另留kernel模型校准证明；它不接受完整N3、双腿中继、UDP、CPU/能耗或手机场景。


## kernel v1已核三轮与真实单位（14:13 UTC）

同一源df5b3f6a0ac26aa02dc92a5593735fef35048adf / treea315f0e280e0766b68b48f5c8b15b6f5c2e06aa6 / snapshot97386eb2d87102dc7d14619ef929447385ed677a，control ea44ab27af3011821d188e0a500ad91b20ed17b1，run turing-20261111T140913Z-141b0ea2，94.220秒终态0、17files/identity/原group空，三轮自动原始数据校验全部通过。64个稳态echo的min RTT40199/40216/40277us；upload约7.520/7.520/7.519Mbit/s、download约7.553Mbit/s，三轮两份8MiB全量字节均一致。4次预热始终另存，未丢弃失败人口（本轮无失败）。

另用kernel-profile-verification-v1.json交叉核12份实际读回（3轮×设备/网关×前/后）：JSON rate1000000对应同次文本8Mbit，即bytes/s；netem delay0.02对应20.0ms，即seconds；tbf lat40000对应40.0ms，即microseconds，burst4096B，limit1000。设备eth0与网关v7都实际生效；每轮两侧qdisc字节分别增加约8.98MB/8.98MB，证明两个方向都有实际数据经过已读回的qdisc。**零随机丢包配置不等于无丢包**：实记设备/网关qdisc drops增量各18/21，不删除或归零，不据此臆测具体TCP/Noq因果。

该独立证明只接受共享condition对应的kernel TCP模型，full_N3_accepted=false。原自动报告仍calibrated:false（保留其“需单位复核”的原始阶段），旧Node三轮仍是原来的未校准diagnostic cohort，不能追填旧样本已做实际读回。下一版Node测量也要在它自己的真实namespace保存前后读回，补观察器定时/记录成本、真实wire/排队与进程退出事实。

此前同源run turing-20261111T140243Z-bd7e5f6c的三项Rust功能门均绿，但总job1/原报告false：校验器用startswith漏读被Rust test名称前缀接在同一行的manifest。原件/红状态完整保留于validator-prefix-failure-v1.json；新control修正为唯一标记后解析、仍要求manifest恰1及原样本/全部门，新增harness_prefix正例与其余反例（12个子例），完整control stub12项8.578秒绿。新parser仅在原始日志副本中回放验证（control-workload-calibration-prefix-replay-v1.json）后，以上独立重跑三轮才取得新的0/通过收据；没有改写旧报告或放宽物理门。

## v4观察器对照（六轮及独立业务流诊断已执行）

保持真实PTY/目录/16MiB、三阶段各两类120请求、原2秒请求和120秒bulk上界、原程序就绪10秒总界、同condition。仅改变测量记录方式：w1 Streaming逐条输出，w2 Buffered在每阶段测量结束后输出；两者都持有相同最多512行的内存记录。缓冲flush耗时另记，不悄悄当作零开销；不同phase仍保留原计划与每类一个在途，积压不删除。

实际测量future由测试本身的Owned JoinHandle持有，drop即abort；每个逻辑请求有Attempt guard，完成、错误、超时、blocked或被丢弃时的cancelled/aborted都留同一人口。阶段未正常结束时Store drop输出已完成记录和logger_aborted，进程被强杀时缺summary/缺样本仍使校验失败，不能算成功。新增w3独立校准会真实启动并取消任务，检验两记录模式都保留取消与成功结果；它不是产品网络取消/进程退出证明。

每阶段同一观察器runtime增加父future借用的20ms定时探针，不新起任务、不逐tick打印，保留计划/实际时间对，明确Skip和被跳过的计划间隔。最多20000个tick（每阶段约320KiB原始时间对），到上限或logger overflow不能通过。v4两模式都有相同探针，**不可把v3/v4不同观察器直接当产品改进**。在实际Node实验自己的x/gateway命名空间，前后保存TC JSON/文本/接口计数，与kernel独立校准分开核对。close仍只是API回复，host/child独立退出与真实wire/许可/排队证据仍待补。

计划在同一冻结source/control跑三对，顺序Streaming→Buffered、Buffered→Streaming、Streaming→Buffered，3次/模式；这是有限顺序对照，奇数配对不能称完全平衡或最终性能接受。先过独立w3拥有者/记录器校准，再执行六个完整场景；后续完整统计仍需多seed/更充分顺序平衡与长尾人口。此处已提前记录顺序，不因结果选择性重跑。

2026-11-11 14:55 UTC：冻结源0b945447/tree725b442271fd07d85a8aeaaac84598cd3f5028e5，control7e439456，实际任务 `turing-20261111T145336Z-bc4dbca9` 已启动。先w3取消/记录校准，再上述固定六轮，尚未有终态结论。Rust与 `turing-20261111T144048Z-f970f1d5` 的a9ef源逐字相同，后者已编译全fixture并通过三w0独立kernel校准，但没有运行w3或w1/w2，不能替代本次运行。控制13项stub及source metadata的18子例仅验证调度/解析合同；全部本机检查未运行产品。原始收据、后续结果与完整范围见STATE。

## v4首次六轮原件与观察器结论（2026-11-11 15:02 UTC）

`turing-20261111T145336Z-bc4dbca9`，源0b945447/tree725b442271fd07d85a8aeaaac84598cd3f5028e5，control7e439456，389.740秒0、25files/identity/原group空。独立guard两模式各1取消+1成功；固定S B B S S B六轮各720、共4320全ok/0错误/0timeout/0blocked/0cancelled/0aborted，六份16MiB逐字节一致，每类实际bulk overlap120/120；24份实际TC读回和双侧流量已核。完整 `observer-comparison-v1.json` 根据原始行独立重算，保留日志SHA与自动contract/source/identity；不是完整N3性能接受。

下表为每轮服务p95（毫秒）；计划→完成仍另报，不删失败或积压：

| 轮次/模式 | 空载目录 | bulk目录 | 空载终端 | bulk终端 | bulk目录计划→完成p95 | bulk定时探针p95 | bulk阶段flush |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 Streaming | 62.642 | 109.986 | 43.537 | 81.529 | 192.635 | 1.780 | 4.088 |
| 2 Buffered | 67.492 | 111.394 | 47.410 | 81.866 | 355.357 | 1.381 | 78.350 |
| 3 Buffered | 62.824 | 109.951 | 45.090 | 81.169 | 388.340 | 2.056 | 81.424 |
| 4 Streaming | 62.736 | 112.573 | 43.525 | 82.671 | 330.387 | 1.323 | 4.144 |
| 5 Streaming | 62.670 | 112.013 | 43.722 | 81.129 | 119.609 | 1.992 | 2.634 |
| 6 Buffered | 62.486 | 109.217 | 43.417 | 82.576 | 290.233 | 1.374 | 76.060 |

bulk实际18.645/18.900/18.928/19.106/19.228/18.666秒。缓冲落盘没有稳定改善目录或终端bulk服务p95；当前三对不证明统计等价或显著性，也不能解释两模式共同的探针/记录成本。Buffered约76–86ms阶段末flush保留为实际开销，位于各计时窗口之外，不能称免费。18阶段探针只有第三轮bulk跳过1tick、max29.116ms，其余max1.875–12.573ms；不支持把数百毫秒目录计划积压全部归因观察器runtime停顿，但不能据此推定resident runtime、CPU或I/O无停顿。

原始逐条重新分解固定100ms计划：`开始迟到 = max(0,前一操作结束−本次计划) + (本次开始−max(前一结束,本次计划))`。bulk目录前一操作积压p95为92.635/255.357/288.340/230.387/19.609/190.233ms，对应残余dispatch p95仅1.764/1.415/2.086/1.246/2.054/1.337ms；每轮有16/26/18/33/18/19次目录服务超过100ms。因此计划积压大部分是这个每类一个在途模型的前一请求未完成，不能直接叫Node内部排队或当作增加观察器并发即可改善用户体验。后续须把实际wire/stream、server permit/read/dispatch/encode/write和网络排队核清，再做策略对照。

## 实际业务流诊断协议（workload_wire_stages_v1，已按下文三轮执行）

保持w2 Buffered整个真实PTY/目录/16MiB、三个阶段/720人口、condition、原2秒/120秒/10秒边界和关闭API，不变动生产选路或队列策略。独立 `netlab-workload-wires` 固定先w3 guard、再三个完整w2串行；新增字段仅在既有KHOR_NET_DIAGNOSTICS开关下取时间/写日志，不增加定时任务、探测、常驻map或强Node。原有list事件名及字段保持，补实际协商ALPN、同进程wire与stream、FetchPath offset/响应长度；不记录文件名、target或payload。实际两端serve.log在测量结束后保留。

每轮源parser要求完整720请求/全部实际profile与probe，以及client/server各360目录和64个256KiB切片成功响应，流ID逐一匹配、slice offset覆盖0到16MiB、wire映射一致。目录按已知每类一个在途/三阶段各120的响应次序分组；切片用唯一offset匹配。记录真实是否共享单一Main，不强迫共享假说通过；唯一终端Tunnel需有实际成功status、正确ALPN、双方同stream且wire与Main不同。异常/重试导致额外或缺少流时诊断完整性报红，原始logical outcomes与全部日志继续保留，不视为产品失败或选择性删除流。

服务端以实际accept_bi返回为零点，记开始等待seat、拿到seat、读完request、dispatch准备完成、编码完成、write_all完成。差分分别包含暂停/准入检查、实际seat等待、读请求与任务调度、解析/认证/早数据等待/dispatch、编码及本地write；不能把它们全叫磁盘时间或网络队列。accept之前的QUIC排队未测，write_all是本地发送缓冲接受，不是收到/ACK。client早数据时长从early dial开始、普通时长从exchange开始，保留origin不混减跨进程时钟。生命周期日志本身有扰动，三轮是定位cohort，不能与此前六轮直接当生产策略收益、p99、完整取消/进程退出/能耗或N3接受。之后基于原件决定是否需要调优stream优先级、批量inflight或连接布局。

## 实际wire与server阶段定位（2026-11-11 16:00 UTC）

`turing-20261111T154837Z-0ff8c054`，准确source04c67d8e/treeacc229f8aa046ac6c1bdac041434351e3bbb6693、control71f1eeb4，259.658秒0、25files/identity/原group空；三轮每轮720、共2160全部ok，三份16MiB逐字节一致，全部profile/probe/重叠和独立guard通过。双方原始serve.log按实际ALPN、wire、stream和offset逐一重核，每轮360目录+64切片=424条响应匹配，合计1272；三轮目录与文件均共享该轮单一Main，唯一终端Tunnel双方匹配、wire不同。`wire-stage-verification-v1.json`独立保留日志SHA、人口和时长；不是生产策略性能对照。

bulk阶段p95（毫秒）：

| 轮次 | 目录完整API | resident内网络exchange | server accept到本地write完成 | 其中seat等待 | 其中准备响应 | 终端完整API |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 114.962 | 102.951 | 8.022 | 0.011 | 7.339 | 82.244 |
| 2 | 112.967 | 101.325 | 8.049 | 0.010 | 7.580 | 81.733 |
| 3 | 112.686 | 101.216 | 8.644 | 0.010 | 7.895 | 81.573 |

同cohort空载resident exchange约51–53ms、server accept→write p95约6.9–7.5ms，bulk为上表。不能相减各自p95当作一条请求的因果分解，也不减跨进程绝对clock。该受控负载下seat等待仅8–11us、server准备略增，未支持把约100ms native exchange主要归因32个serve seats耗尽；尚未观测QUIC accept之前、实际排队/ACK/每包交付或host/child退出。所有目录响应为普通交换、early=false。

下一步先做同fixture、日志关闭的策略对照：比较固定bulk在途1/2/8以及后续文件切片的stream优先级，首切片/首份有效内容与普通控制请求保持及时性；这些仍是候选，尚未实现/选定，不在本文把新的参数当用户SLO。既测40ms/8Mbit小BDP，也必须覆盖原PULL_AHEAD设计依据的300ms/50Mbit大BDP，防止仅为当前小链路牺牲吞吐。后续补共享/分连接、连续控制下bulk公平推进及1/3/5设备成本；严格优先级可能造成饥饿，单cohort延迟下降不足以接受。当前终端本来就是另一连接，继续增加连接须有数据支持。
