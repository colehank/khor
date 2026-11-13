# 空闲恢复后的首次操作与成本

本协议在新增首次操作数据前保存。旧p7及其active10秒/idle20秒选路门保持；
它们量两端重新选路的时间，不代表用户空闲后首次读取的响应。
源码备用keepalive15秒、STREAM活跃刷新最多每peer5秒一次，与旧idle16–17秒量级相符，
但不能独立证明旧尾部的唯一原因，也不预选缩短生产计时器的方案。

## 所有权准入

`LabNode.with`当前直接await设备JoinHandle。调用者future被丢弃时，内部设备任务可能继续执行；
这是需要真实验证的fixture所有权假设，尚不能凭静态代码把它算作已重现或已修复。

先添加固定filter `paths::idle_action::p8g_`的两个ignored诊断，必须显式`--ignored`执行：

1. 实际LabNode.with启动一个受控pending future，收到started后丢弃调用者；
   在1秒fixture界内必须观察到future Drop。旧实现即使红，也先释放受控等待并收尾后再断言。
2. 原始Device.spawn句柄被丢弃的负控必须能被检测；保留独立abort控制并回收，
   防止一个失灵的观测器把“没观察到任务”解释为自动取消。

两项各用原speed_pair的真实resident/设备运行时，5秒startup/cleanup只是fixture存活界；
测试线程1、诊断关闭、原shared guard/namespace/资源门不变。先取得准确源码的真实红和负控，
再修正fixture所有权，以完全相同两个测试复验。生产Noq/iroh/Node策略和旧场景参数不因此改变。
这只证明测试调用者对设备future的所有权；服务端任务、RPC取消、独立host/child与真机仍另验。

## 随后的首次操作测量

所有权准入通过后才实现并执行新测量。沿用speed_pair：慢Direct300ms、可用Relay约80ms，
两端准备阶段确认使用Relay，STREAM静默至少10秒，然后将原Direct恢复至0ms。
恢复后等待名义0/5/14秒，每方向各一个独立fresh，共六场景；不能先从另一端发业务预热。
首次前台目录读取前不查询Node状态或发其他业务；原echo的30秒caller与全部结果记录保留，
这是测量存活界，不是新UX达标承诺。记录实际单调时间、首请求服务/总时间、错误/超时与前后路径事实。

分开idle和首次操作的wire总字节/包区间；接口只读快照自身的时间与命令失败也须记录。
不能把网络总量当probe-only、把CPU当唤醒或把服务器结果当手机能耗。
观察工具/设备任务须由调用者拥有；计时前不得新增无主任务或预热动作。
需要原始配置/计数单调性、收据、进程组空、全部源/报告hash和source-owned复算；
缺测和失败保留，不按好看的首请求筛样本。

prefetch9组已按原冻结源/control全部结束，guard随后单独执行，未混入计时。
Linux实际format/编译成功、主所有权断言红、detached负控绿；旧红原件见STATE。
调用者持有的AbortHandle Drop修复已以逐字相同guard复验绿；旧红与新绿的控制入口相同。
实际run/source/proof见STATE；这仍只准入fixture所有权，零新增首次操作数据。
完整N0–N5、严重/quiet/跨连接质量、peer relay与移动、最终UX与真实手机能耗范围保持。


## 首批六场景的准确执行协议（首个数据前）

固定filter `paths::first_action::p8_`，显式ignored、线程1、诊断off；按Rust串行名称顺序
alpha0、alpha14、alpha5、beta0、beta14、beta5，每个均独立fresh speed_pair。
control先执行原两项p8g所有权guard；任一失败则不进入首次操作。所有旧p7门不改。
准备阶段分别记录两端实际Relay事实，然后采before_idle；10秒静默后恢复Direct，再等待名义phase秒，
采before_action、只发一次原30秒目录读取、采after_action，最后才查询两端路径。
实际restore前后、请求前后、命令前后均用同一单调epoch记录；没有准备后的Node状态轮询或另一端业务预热。

每次只读快照记录ip -s -j link及tc JSON/text。v1只采两个端点，错误地要求alpha命名空间内有两个netem；该轮metadata失败原样保留。
v2读回包含两端eth0/eth1及second路由器各端口的原始字节/包/错误/丢弃计数。
初始alpha eth1出口一个150ms netem，其入口对应second路由器的peer veth一个150ms netem；
beta无netem、无额外TBF；恢复后两个端点和second均无netem。
须以两端ifindex/link_index双向匹配绑定router peer，不能仅靠名称或队列数猜测对应端口。
实际JSON秒单位与text 150ms交叉核验，不用请求模型参数代替安装读回。
每条命令原2秒观察界、kill_on_drop、调用者持有Device任务；命令失败原样保留。

外部source-owned校验需六个完整场景与十二条逻辑请求起止记录、实际Rust六项计数、原p8g两项全绿；
每case校验配置、方向、阶段/静默实际时间、唯一caller、结果、前后配置与计数单调性。
请求错误/超时仍算完整失败样本，不从分母删掉；缺日志、缺case或配置不符标不完整，不补跑漂亮样本。

idle/action按各设备和接口分别计算差值，包含快照间隙，时间用实际命令界给范围；
两端重复观测同一批数据，不能相加当唯一字节，也不能称probe-only、CPU、唤醒或手机能耗。
6个样本仅作首次操作探索，无p99、最终恢复SLO或完整N1接受。


## v1配置缺口与v2修正（首个v2数据前）

v1六次读取均成功，但整轮因缺少入口安装读回而失败；不能按原Rust六项绿宣称模型通过。
实际vendored patchbay src/iface.rs::apply_condition把Egress装在device命名空间eth1，
Ingress装在所属router命名空间的veth。该文件SHA256为
`2e819ab15361340e8558b281627f6a768626169955ceafa9641f1fbd925c475f`，
根Cargo.toml用vendor/patchbay；读取到的registry同名文件逐字相同，但实际证据以vendored源为准。
v2只增加保留second Router句柄与调用者持有的只读采样任务，以及对应配置/计数核验。
原网络配置、六fresh顺序、准备和静默/phase时间、30秒caller与操作数量不改；revision显式升到
first_foreground_after_idle_v2，原v1数据不充作已通过的v2样本。
新增router读回会有独立观测开销，记录各命令时间与快照间隙，不能当作零开销前后对照。


## v2六场景观测结果与静默证据边界

实际run turing-20261112T023151Z-8d7b0ea1，source5a5d7e84、control2cfce93d；
原0/224.643秒、17原report文件身份/hash、原进程组空、准确冻结metadata及独立原人口/时间/计数差值均核验。
原两guard与六个首读取全部通过，共6个逻辑caller/12条起止、162条只读快照命令，均有原件。
每个方向/phase只有一个fresh，不能由此推p95/p99、显著性、最终SLO或手机能耗。

| 方向 / 名义phase秒 | 恢复到实际调用秒 | 首读取ms | 操作后alpha / beta路径 |
| --- | --- | --- | --- |
| p8-alpha-0 | 0.019104 | 297.685 | Relay / Relay |
| p8-alpha-14 | 14.025917 | 327.577 | Relay / Relay |
| p8-alpha-5 | 5.020709 | 321.867 | Relay / Relay |
| p8-beta-0 | 0.021565 | 230.688 | Relay / Relay |
| p8-beta-14 | 14.020498 | 265.406 | Relay / Relay |
| p8-beta-5 | 5.025363 | 250.937 | Relay / Relay |

完整接口差值见first-action-verification-v2.json与原first-action-contracts.json；
所有端点/路由器都是同一流量的不同观察点，不能相加为唯一字节。
原v1配置失败不覆盖，不拿v1替代v2。操作后仍Relay只是一项路径事实，
不能单独证明Direct实际不可达、某一计时器或IPC是唯一瓶颈。

**原协议的连续STREAM静默10秒尚未独立证实。** 本fixture只证明没有显式前台预热调用；
resident后台流量未被关闭或逐帧区分，总字节/包不能证明其中没有STREAM。
quiet-scope-audit-v1.json绑定本次proof并保留该缺口；这六个有效配置/首次调用/成本观测不等于完整quiet恢复接受。
后继校验输出显式continuous_stream_silence_verified=false与full_quiet_contract_accepted=false；
完整N1仍须独立观测/建立连续STREAM静默、严重尾部与完整原范围。


## 被动STREAM见证协议（首次诊断数据前）

新增独立revision first_foreground_stream_witness_v1和固定task netlab-first-action-streams，
沿用原六fresh名称/顺序、拓扑、Direct300→0、Relay约80、phase0/14/5及原30秒caller。
先以诊断off执行原两p8g；全部绿后才以NETLAB_DIAGNOSTICS=1执行六场景并保存双方resident原日志。
本层是full诊断，不充当新的普通性能或能耗样本。

Iroh补丁50只在原有250ms采样周期读取原连接stats，增加STREAM tx/rx、actor序号、
连接集合增删代数与单调/墙钟快照，不发请求，不新增定时器。全连接集合在采样之间新建又关闭也会改变代数。
候选静默窗为12秒（原合同至少10秒）；固定取起止各留500ms边界的首末快照，
单调跨度须至少10,000,000us，并要求整个候选窗的两端所有受跟踪连接、代数、actor和计数不变。
不从变化窗内挑好看的子段；最大快照间隔1秒，单调/墙钟相邻增量差最多100ms作为观测有效性门。
每个方向随后原真实目录读取必须在双方相同actor/原连接上产生STREAM增长，形成阳性对照；
请求完成后多等1秒仅供现有采样周期观察并保存日志，不算操作延迟。

source-owned校验要求全部六case/十二caller/十二resident日志、原配置/计数与实际Rust人口齐全；
见证失败与读取失败分开，Rust六读绿不能覆盖STREAM见证红。缺日志/错对端/时钟跳变标不完整，
有完整观测但业务或见证失败则完整保留为失败，不自动补样本。普通模式仍原10秒/diagnostic off。
已通过11种见证合成输入与原19种first-action、9种guard输入；6种调度stub验证先guard off再action full、
guard红不进业务、首原非零73/79保留。它们都不是实际STREAM静默证据。
本次只补静默观测合同，不是严重/quiet全尾部、迁移、手机恢复、能耗或完整N1接受。


## 首轮STREAM见证失败与有界准备阶段（v2首个数据前）

first_foreground_stream_witness_v1实际run turing-20261112T030608Z-15ebb293已305.229秒/原1，
六读取与原两guard全绿，十二真实请求阳性对照全过，但十二静默见证全红。
29原report文件/身份/原进程组空/准确冻结metadata与原人口核验及ledger终态完整保留；
stream-quiet-verification-v1.json记录十三次变化，见证tick观察在候选窗开始后307642–645086us，
actor/registry不变而STREAM计数增长。它们是观察时刻，不是精确发包时刻，尚不能称唯一业务来源根因。
原整窗恒定门保持，不把原失败轮的后半段挑出来算通过。

v2保持全部原静默/配置/时间/阳性门，在候选窗之前增加被动准备阶段：
只读双方serve.log中原被动快照；全连接计数/actor/registry连续至少1,000,000us相同、
快照距当前最多1,000,000us且属于此准备期，双方都满足才继续。
总准备期限10秒，到期失败；不发请求、不查询Node、不关闭后台任务。
记录准备起止的fixture单调/墙钟及每端实际稳定起止快照和完整签名，外部校验逐条绑定原日志，
再执行原12秒候选窗及十秒内窗；只准备一次，候选窗若有活动仍失败，不在结果出来后滚动挑窗。
普通模式仍10秒/off且无此诊断准备；新revision first_foreground_stream_witness_v2，旧v1数据原样保留。

新准备校验增加6种合成情形（真实稳定、过短、缺快照、过时、错签名、准备时序错误），
与原11种见证、19种first-action、9种guard一起通过；控制仍只换MODE literal，原6调度路径通过。
尚未有v2真实结果，不能提前称静默硬合同已满足。


## 有界准备后的STREAM静默准入已验证（2026-11-12）

run **turing-20261112T033334Z-a09450d0** / source **5064fe2a435d9cf68389c97d7dbb95d5bdf2c715**
/ tree **9bf94eed4355b0eb9b96f6c99a0f9b284dd07923** / snapshot **05a7bab4a6b24a324aa2906d8bdc86c8a51c63d3**
/ control **73fa35f5954175e5e20373c858691315d4a3e6fe**，339.545858秒、原0。
29原report文件身份/hash、原进程组空、准确冻结三metadata以JSON序列化键规范复算、
独立六case/十二caller与原Rust计数、原始计数时间窗/actor/registry/连接集合/阳性对照均已核验。
原两guard与六读全过，十二准备见证、十二静默见证和十二真实请求阳性对照全部通过；
proof为stream-quiet-verification-v2.json，ledger终态。

| 场景 | alpha / beta准备稳定秒 | alpha / beta无STREAM增长见证秒 |
| --- | --- | --- |
| p8-alpha-0 | 1.002434 / 1.000365 | 10.747470 / 10.751444 |
| p8-alpha-14 | 1.000145 / 1.000421 | 10.755941 / 10.755724 |
| p8-alpha-5 | 1.250605 / 1.250594 | 10.751693 / 10.751840 |
| p8-beta-0 | 1.000603 / 1.253903 | 10.750574 / 10.750118 |
| p8-beta-14 | 1.251461 / 1.247227 | 10.758235 / 10.749672 |
| p8-beta-5 | 1.250413 / 1.247686 | 10.752393 / 10.750554 |

每端见证跨度10.747470–10.758235秒，整个原12秒候选窗的计数/actor/连接集合均不变；
准备1.000145–1.253903秒的稳定读数也逐条绑定原日志。没有放宽旧静默门或把v1失败重标为绿。

**接受范围只有诊断准入。** 这是双方受跟踪的彼此连接在Direct恢复前真实静默的证据；
full日志、准备等待和12秒窗口会改变时序，不能混入普通模式的延迟/CPU/能耗分布，
也不把这批证据倒灌为旧10秒/off六观测已满足静默。
严重/quiet恢复尾部、跨连接质量、独立/同时双栈换网、peer relay、手机及完整N1–N5仍未完成。
后续按既定N3协议继续匹配batch/rolling源与真实RPC/取消合同，不以本层全绿宣告最优设计完成。
