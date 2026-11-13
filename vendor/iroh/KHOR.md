# vendor/iroh —— 为什么这里有一份 iroh

## 1.4 升级候选（2026-11-12，尚未验收）

本隔离树已将发布源码从1.2.0三方迁移至1.4.1；下文为原补丁历史，不代表仍基于旧版或全部必须保留。当前迁移与退出账目见[IROH-UPGRADE](../../docs/net-optimization/IROH-UPGRADE.md)。所有旧专属回归保留；新源编译和行为证据尚待，原绿不能沿用。

原补丁22的手工IPv6方括号处理已改用上游`relay_tls_server_name`，同时保留上游域名/IPv4/IPv6和真实QAD测试；行为退出待新源测试确认。原补丁1的有限队列与path-ID释放后重试语义保留，不能仅因上游含dedup删除。

## 来源

- crates.io 上的 `iroh 1.2.0`，逐字拷自 cargo registry 解开的那份
  （`.cargo_vcs_info.json`：上游提交 `17c0612f80f78f5288e97b818b1360ae6ea0a51a`，`path_in_vcs = "iroh"`）。
- 只去掉了 cargo 自己的 `.cargo-ok` 标记；另从上游仓库根目录（tag `v1.2.0`）补了
  `LICENSE-MIT` 与 `LICENSE-APACHE`，因为 crates.io 的包里没带（许可证：`MIT OR Apache-2.0`，不变）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] iroh = { path = "vendor/iroh" }`，
  以及 `[workspace] exclude = ["vendor"]`。依赖声明仍是 `iroh = "1.0"`，lock 里的版本仍是 1.2.0，
  只是来源从 registry 换成了这个目录。

## 补丁

### 50. 隔离诊断的STREAM计数与连接集合见证（2026-11-12，默认库/诊断准入过，完整N1未接受）

N1首次前台操作已有有效配置/成本数据，但十秒没有前台调用并不证明传输层STREAM静默。
复用原250ms reselect中的refresh_candidates与其原有每连接stats读取，仅在khor_net_samples DEBUG开启时，
记录对端所有受跟踪连接的stable id、STREAM tx/rx累计计数、actor序号、连接增删代数、单调/墙钟时间。
连接集合代数覆盖采样间新建又关闭；actor序号防止重建与计数复位假装稳定。
原STREAM合计、ActiveRefresh、探测/选路、队列及全部生产计时器保持；无新增任务或计时器。
普通不开日志时不分配快照Vec、不读取观测时钟、不累加诊断序号；两个诊断字段保持零值。
不记录密钥或包载荷。日志有成本，真实静默见证必须与普通计时/CPU/能耗数据分开。

新固定诊断六fresh保持原方向/phase/30秒caller，静默候选窗12秒，在两端日志内核十秒完整内窗、
稳定actor/registry/连接集合/STREAM计数及真实请求后的计数增长。缺失、时钟跳变或STREAM活动保留为失败。
纯Python见证校验合成红绿已过。Linux真实v1十二阳性对照过、十二静默见证均红（准备流量尚未排空）；原失败保留，新增有界准备层已以独立v2复验：turing-20261112T033334Z-a09450d0原0，十二准备/十二静默（≥10秒）/十二真实请求阳性见证全过；原v1保持失败。
准确源09374004已Mac43146243502默认库回归：Noq447/Iroh184通过、原1忽略保持；58路径/11home专项与184重叠，不相加。
只证明默认库与观测器实际运行；不是静默/N1/N5接受。
目前主候选增至60项（Iroh1–41/45–50与Noq1–13；42–44仍隔离），本项是观测补丁。
未核上游同语义替代；诊断结束可删除本观测器，或在验证等价的全连接计数/生命周期代数后替换。
删除不改变选路参数，但须保留原始记录与普通/诊断分层，重新验证依赖该观测器的验收工具。

### 49. 单一实际路径不被另一连接的首选路径遮住（2026-11-11，候选未验收）

PathStateSender.record_selected原本把peer共享的preferred四元组在每条conn的open path表匹配；另一活IP连接保持首选而新conn只有Custom时，后者实际已完成端到端业务，观察却selected=None，Node路型Unknown。真实Node四合同中源166的两端普通/early均answered=true、requests=1但routeUnknown，旧573则answered=false/requests=0；原断言和旧原红保留。

修正仅在共享preferred不属于本conn且它只有一条established/open路径时报告该路径为有效数据路。Noq PathStatus::Backup文档（vendor/noq-proto/src/connection/paths.rs）明确无Available路径时Backup承担数据；该单路径即当前实际可用的唯一调度选择。不改变peer共享选路、Noq path status、数据发送、质量算法或其余连接。零路/多个路径未匹配时仍Unknown，不能把候选地址当已选。无新常驻状态、任务或计时器。

保持Node四个完整TLS/业务/快Direct负控与原1秒gate逐字不变复验；随后同冻结四peer1.5秒/30请求/双端wire和outer连续性、vendor筛选及其他N1恢复。rtt_of会读到这条实际路的RTT，须核原预算/恢复与quiet成本，不称纯显示无影响。上游等价实现未核，退出需保留上述实际多连接数据/零多路边界并验证等价effective route语义。主候选现59项（Iroh1–41/45–49+Noq1–13），42–44仍隔离；尚未验收。

### 48. 新承载握手有独立的初始路径（2026-11-11，候选未验收）

四真实resident双middle场景874eb4c4三轮均实测：N在约300ms准备成功、实际Hop为viaN/tried2，但1.5秒目录请求超时；middle未收到承载字节，目标3秒首帧lease结束。只解除“无活连接”的selected_path候选d521在9d655e7f仍红；诊断live=true/registered=1、旧选路为Relay、reset=false，另一条活连接的共享初始选路不能被全局清除。该行为候选已从后继源码撤回，原失败/同源vendor绿保留，不当作修复。

ConnectOptions新增with_initial_custom_addr：地址必须同时显式出现在EndpointAddr中，当前这次握手的初始Noq目的地址直接映射该Custom transport；其他连接、共享候选/选择和握手后质量选择均保留。TLS仍认证EndpointAddr.id，指定载体不改变身份。默认连接行为不变，不丢旧IP/Relay提示、不加全局模式/常驻状态/任务/计时器。可选旧路诊断保留且普通路径不计算诊断内容。

Node在新carrier准备完成后最多给一个初始载体握手机会。普通拨号最多两个活动future，原Relay任务取消并join后才复用其名额，保留原Direct竞争；早发只读路径保留原0-RTT竞争者，替代握手不发送early业务，赢后取消原早发任务并沿已有只读正常请求门继续。原握手先赢不重复业务；新增替代失败不提前决定原路。全部使用同一原总期限，不在替代失败后领取第二次carrier重试，不扩大rides_early/写请求门。已有早发读取允许的普通回退仍有原读取重试边界；非幂等写仍只在胜出的认证连接上执行。

新增Node真实合同：保持一个活IP连接，同时确认新连接PathId0实际为Custom并完成TLS/echo；错误目标身份仍拒绝，旧IP连接仍活；未显式提供Custom地址时API拒绝。尚待同源双OS Node/vendor、原四peer1.5秒合同与连续性/取消/成本验证，不称N2/N4完成。上游未提供同语义证据；具备等价的每次初始载体和有活连接控制、TLS/写边界/预算及真实移动矩阵后再退出。主vendor为58项（Iroh1–41/45–48、Noq1–13），42–44继续隔离。

### 47. 质量和确认次数来自同一个连接副本（2026-11-11，feat/net候选）

`road`和`observation`选同一个代表副本，旧`samples_after`却取任意副本的新样本最大数。
真实Reading状态合同b0146293：原55/代表改变控制绿，两个旧质量只能证0/1个决定后样本的case却借另一副本报2，
56pass/2fail；Noq443绿，home未执行。此缺陷不等于f8 7.307秒或6aa单向停顿已定因。

`samples_after`仅用同一代表的实际发送时刻与changed记录；代表算法、质量窗口、真实post-decision条件、
25%收益、原Moving时限、全部探测配额及生命周期保持。现有Moving、第一恢复判断、慢路验证、
有限恢复episode的自然样本停止逻辑复用一致证据。其他副本的样本不再提前耗尽代表采样资格，
因此实际可用掉原预算中的更多预约，仍最多3次、至少250ms、2秒内，需实际成本/稳定性对照。
无新增map、task、计时器或强连接；原fixture及三新合同whole冻结。尚待验证，不作N1/phone接受。

未提上游；上游具有同源质量/时刻/确认计数且通过健康、代表变更、恢复及原预算/移动矩阵后再退出，
不据方法同名或单元绿删除。


### 46. 已知路回到故障前实测水平时，只刷新首次恢复质量窗口（2026-11-11，候选未验收）

主923固定三轮2绿1红，慢方向5514.252ms/4个四秒请求失败；恢复后双样本101/85ms的上中位数被保留到旧样本过期，未触发41的25%向下重置。de03诊断另一次3432.844/3391.236ms绿且发生较大下降刷新；该诊断不是第四次验收或配对因果证明。新独立whole合同在c775/eec09ec1实际Noq439绿、路径53绿2红，原50及3个负控过，home未执行；返回85仍报告100、其后新110仍报告101。

已有Silence episode首次判坏时保存实际ACK处理早于故障待包时刻的Reading质量，排除初始估计，以原active-copy优先级取值；转交有限RecoveryCheck。只有原hint/已知历史/非当前silent/2秒窗口内，真实packet sent在episode开始后、相较当前质量确有下降且不慢于该故障前实测水平，才允许首次质量刷新。之后仍按41的25%单调改善/最多3阶；新packet epoch规则不变。历史值只是触发重测窗口，不作为新样本、可达性或切换收益证据。

原3次probe（失败/合并也消耗）、250ms、两秒采样窗口、Moving两秒连续确认/两次决定后实测/25%收益、重hold及所有transport同规则均保持。单个快脉冲后的新慢包照常进入质量；健康/初始/过期及仍未回到旧水平不得用此分支刷新。没有新增map/task/timer/强连接owner，两个Option随原episode回收。42–44仍隔离；主vendor暂为55项候选（Iroh1–41/45/46、Noq1–12），不能记已验收。

待同whole合同vendor绿、双OS Node和原物理四秒固定重复、quiet/dynamic/stability/探测实际成本与真机。上游等价的有限恢复质量epoch/已知历史确认尚未核；同一行为/调用者/成本覆盖齐全才退出，不据单元绿宣称N1或手机问题已修。

### 45. 无live连接时按失败边界撤回旧IP提示（2026-11-11，分项已验证）

42–44在隔离worktree试验，编号预留且未合主。本条是主分支第54项（Iroh1–41/45、Noq1–12），只接受下述实测窄范围。初始discard函数返回0，cf2352f7实际Noq439/path50绿、home10/1红；新来源控制过，旧IP应撤1实际0。当前实现按source时间过滤，原whole fixture保持；Node只在cold未发early数据且新carrier已就绪时调用，仍在同一绝对dial_by内。

3a7424b7的真实stale-IP不同身份场景：可信middle准备完还剩1985ms，新carried握手仍收UnknownIssuer。Socket将地址加入共同RemotePathState，传carried-only不等于撤回旧IP。只在actor执行时没有任何live连接的条件下，移除所有来源时间均不晚于首次失败边界的IP提示；之后新提示及Relay/Custom保留，当前连接不被关闭。无安全校验放宽、探针/任务/永久timer；重新输入地址可再学习，不是永久blacklist。

153263c2/treeee002acd/snapshotf4c5f390：Linux1e1264ef Noq439/path50/home11，Node Linux8adb26d3与Mac42869844599同canonical全12组/hop44，exact13790105312/20files/11control blobs核。旧stale-IP/旧hint状态红均保留，真实live wire负控与较新来源保留通过。仅该scope，不是全部iroh测试或N5最终矩阵；未覆盖网络/地址的完整逻辑epoch，fresh来源时间只是本次边界。上游若提供等价的候选失效/受限握手来源及安全并发保护，按同一真实合同退出。不得把该分项验证写成整包/手机恢复成功或缩减原N0–N5。

### 41. 恢复中的显著向下变化开启新质量窗口，旧packet不得刷新新窗口（2026-11-10，feat/net候选）

2026-11-11诊断扩展：主923固定三轮2绿1红，慢方向5514.252ms且quality_since=None，旧isolated重启假设不适用。仅在显式khor_net_samples DEBUG下记录Custom真实stats/Reading/recovery/Moving及恢复期代表copy，核对探测结果归属和近期中位数/过期；不改变探测、排序、计时、预算或确认。该诊断不是新行为补丁或四秒已修证明，原失败保留。

40的fc435ccd预约已满足3次/250ms/2秒，但两次探测早于外层Direct恢复，近期中位数把早期慢结果保留2秒，再等正常2秒确认，实际Go5392/5630ms，原采样门48/51。原物理、两个四秒checker及失败保留。

只在已有hint且有历史的有限recovery episode里，新sample的packet sent必须在本轮采样开始之后；新RTT较当前road质量至少低25%才重启质量窗口，后续重启还必须较上次重启值再低25%，最多3次且限原2秒采样窗口。把新窗口的packet-send epoch带给该road各copy及新copy，epoch之前晚ACK不改质量/年龄或fresh计数；真实Noq ACK总计数与transport SRTT照读，不改Noq运输数学。单个快脉冲后慢数据仍进入中位数；新旧包不能重复制造fresh observation。

健康/初始/无hint路中位数保持；不增加PING、timer或连接所有者。Moving仍从实际开始考虑竞争的时刻等待原2秒、决定后的两fresh和收益门，不借历史marker回填确认时间。新增4个状态合同（健康对照、阶段恢复、迟到ACK、跨copy epoch、噪声/重启预算），尚待远端与原物理两门同源至少3轮、quiet stalls/黑洞/成本。上游等价窗口/时间保护未核，不能据本条删除vendor；同一协议与行为覆盖齐全后退出。

### 40. 恢复采样保留同一live副本，有限决策到期精确唤醒（2026-11-10，feat/net候选）

Iroh39的fa99首轮124/124和原采样四秒门51/51，但两端实际恢复Go约4060/5799ms。y的3次探测相隔约500ms且copy A→B→A；Reading质量按copy取中位数，混用延迟健康样本积累。严格250ms预约遇到略早的普通tick会再等250ms；2秒确认也可能跨下一tick。新独立双端决策门在下一代码修改前取得c397/ba7真实红，原物理/采样checker字节不变。

同PTO优先已用的live copy，关闭或更低PTO可换，只存conn/path ID。既有actor用MaybeFuture为待发恢复预约及Moving的2/4秒边界持有限sleep；无pending为None，无永久新timer/task/强连接。仍silent/用尽/自然样本足够/过期时disarm防past-due spin。醒来重新读真实stats，保留3次/2秒/至少250ms、sent>judged、2秒两fresh与原gain；timer不充当样本或Go证据。日志追加预约Instant/attempt/epoch以核真实节奏与合并/无copy消耗，不记正文或票据。

新增4个纯状态合同，尚未远端验证；必须两四秒门同源至少3轮、实际probe预算、原stalls/黑洞及成本合验。上游等价到期调度与副本采样策略尚未核；同协议/行为/成本及所有者覆盖齐全后再退出。本条不是已验收或新增永久轮询的许可。

### 39. 已知路收到恢复提示但PTO已清时，有界确认新的往返（2026-11-10，feat/net）

002954真实peer relay移动的70条conn/path快照显示last_received>judged，但latest_rtt_sent<=judged、PTO0/unacked None；旧copy wake不会发送新包，内层直到约5.3秒才取得新往返。原267/272在冻结四秒业务选路合同为红，原可达/最终hold绿不替代此门。最早hint还在外层恢复前，故不能用一次旧ACK或一次过早慢样本确认新网。

Measured按peer/road/实际silence.began持有有限episode，需真实历史样本、判定后回包hint、当前无silent copy才申请。两秒内最多3次、至少250ms间隔，失败/合并也消耗预算；重复hint/同一未解故障不续费，无hint黑洞/健康闲置/首次未建立不触发。自然3次新观测可停止；与现有Moving或已选可用路合并。选择低PTO、较新收包的已有弱连接copy发PING，无新timer/task/强Connection owner；关闭路或已恢复窗口结束回收。

保留sent>judged、原2秒/两fresh ticks、收益门、PTO/重hold/现有copy-waking规则；所有transport同规则，无Direct优先。新单元只是预算/生命周期合同，原四秒物理至少3次、CPU短停顿/黑洞/探测包成本、准确源vendor/portable均待远端。当前不是已通过/性能完成；上游等价恢复提示与有界确认及同一行为/成本证据齐全后退出，本条尚未向上游提交。

### 38. 被动地址迁移后按真实FourTuple更新缓存与测量归属（2026-11-09，feat/net）

247在真实NAT端口重绑中124/124请求可达，但慢方向约123ms（另一方向约21ms）；同一Noq conn/path1实际remote已从:50001迁到:50002，iroh缓存仍指旧:50001，69次不一致。独立数据合同warm_remote_requests_select_the_rebound_tuple_v1在原整fixture上30次warm remote请求0次选中新tuple，journal也没有新selected；输入/原文/身份与合同哈希见net优化证据账本。

在既有250ms reselect前核Noq真实网络tuple，只重登记确实改变的path、清该path旧Reading（其他copy不动），旧remote没有其他live copy才撤其open事实。tuple变化时强制把现行选择应用到各conn，避免全局选择未变时个别conn仍全Backup；正常收益/新鲜确认/安全验证/重试/并发/定时与probe节奏不改。不把前地址的历史质量无条件挪给新地址。Noq自身验证/放大限制保持。

候选尚待同整NAT fixture/data合同、完整vendor与便携回归，以及IPv6/中继移动/原矩阵与成本验证。没有改Rust fixture去放宽门；上游若已提供等价被动迁移事件或持续tuple同步，须相同真实合同与调用者覆盖后再退出。本地诊断默认关闭，tuple已及时同步且验收留证后可单独移除诊断输出；不得凭去掉日志认定根因解决。

### 37. 无当前副本接收提示时保留故障后的快速备用证据（2026-11-09，feat/net）

36的双路确认修复了CPU短停顿误切，但普通p3在42542984111两轮1.1/1.2秒超过原1秒。
42551075037在相同待红源码上已实际得到437Noq绿/路径35pass1fail：所有当前copy没有接收提示时，故障后已有备用应答仍返回Hold，而非Go。

候选让Moving.leave按当前路live copy的接收提示区分：提示只触发当前故障副本探测与决定后备用样本要求；
无提示时使用began之后的已有备用往返。提示本身不证明正向路健康；旧包不能确认，正常迁移两新样本/收益、4秒兜底和原探测节奏保持。
新回归及额外false旧包/兜底覆盖待候选绿；实际p3/CPU/转发仍须原门复验，不能据单元宣布恢复达标。
本地新增，未提上游；上游若提供等价可达性/副本判据，以这些协议和同源真实恢复/成本复验后再退出。

### 36. 故障确认也取决定后的备用应答，并探测当前故障副本（2026-11-09，feat/net）

observer v1诊断42538759858的4次故障Go，同一路另一连接已收包，旧PTO副本仍使整路静默。
一次Hold只探测候选，另三次用决定前备用ACK立即Go；有一次本来已允许wake，也来不及在同tick Go前验证当前副本。
42541459177已436Noq绿/iroh34pass1fail，新回归期望Hold双向探测，实际Go。

Moving.leave要求备用样本包在max(本次since, 故障began)后发出；候选探测tick也不早于此界。
等待期间探测当前路；选当前最高PTO的live copy，避免只PING已经健康的另一连接。
候选则优先最低PTO copy；所有实际选择/结果可用可选confirmation probe日志核对。
原4秒兜底不延长，无新timer、应用请求重放或类型优先；正常质量确认/收益与routine wake旧规则不变。
可能比旧故障分支多当前路PING，需同时验证p3<1秒、p1s/p5防抖及包数成本，不能只拿单元绿收口。

42542123882已SUCCESS 436Noq/35路径/9选家与根格式；普通动态/转发/防抖及诊断证据仍需验证，不能单元收口。
上游等价故障确认/副本探测与这些回归齐全后逐条退出。

### 35. 首次恢复的可用性不依赖质量与承载类型（2026-11-09，feat/net）

诊断42538717126中，Custom在决定后应答，X因旧quality不能通过quick又等1秒；Y通过quick则当场恢复。
新回归42539860644取得436Noq绿/iroh33pass1fail，首次新往返在quality仍5秒时仍被可用性hold门住。
这证明可用性/质量职责混在一起，不证明全部f4/p1s失败都只有此原因。

Measured首次故障在最后一次静默判断后有新包往返、且所有副本当前均未判静默时恢复可用性，
不再比较当前路类型或快慢。Moving/RoadOrder仍负责质量/收益/新鲜确认。
REHOLD内重复故障保留原SILENT_HOLD一秒，旧包ACK不算恢复、已关路径仍忘记。
去掉quick回调及两份临时向量，无新增timer/探测；copy-waking旧off_direct限制本条未改。
原hold回归明确模拟近期已有恢复；首次恢复/重复故障/旧包及闭路覆盖继续保留。

42540515220已SUCCESS 436Noq/34路径/9选家与根格式；普通p1s/p5的42540582927在途，f4/动态仍待准确源复验。
不能以诊断或单元绿收口；上游等价可达性/迟滞与调用者覆盖后逐条退出。

### 诊断说明：迁移与恢复的实际输入（2026-11-09，feat/net）

`khor_net_samples` tracing target在迁移第一次Hold/Go与当前路已关闭的直接选择处记录实际
Reading/Silence及确认起点；静默恢复记录决定后应答与hold解除。未改变策略/探测/计时，
默认CLI不安装subscriber。仅`net-diagnostics`可选构建且显式环境开关启用；
netlab-diagnostic保留原p1s/f4s完整负载并将serve.log作为artifact保存。
日志开销不可算普通SLO；根因确认并有更持久的低开销事实记录后可移除这些诊断调用。

### 34. 普通慢质量先验证，再触发打洞（2026-11-09，feat/net）

近期中位数可能暂时只有一次慢ACK。Watch原来读到慢quality即打洞，与同时开启的2秒探测并行，
未等探测区分持续退化和短暂停顿。42531529275取得435Noq绿/iroh29pass1fail，
150ms单次慢质量同时返回probe与punch；实际p5也有150/300ms组额外打洞，但本回归不是全部定因。

普通质量必须连续高于原floor门2秒，并有两个采样包在这次确认开始后发出，才可打洞；
复用Measured.samples_after的真实发送时刻，不用actor恢复后的旧值或积压ACK补证据。
质量恢复取消该次确认，不重置原打洞/探测节流；原每30秒至多一个2秒探测段不变，没有新增timer。
近期探测预算用过时，前台STREAM样本仍可确认；不为普通闲置短波动额外发探测。
真实无应答满原1秒的独立通道保留，RTT已超过整个2秒探测窗也保留立即探索，
避免为极慢路径等待放不进预算的新往返；这些都不直接强制选择另一条路。

回归包含先探测、无新证据的暂停、恢复后重新确认、极慢路径与原真实故障/30秒节流；
42532303734单元435Noq/33iroh全绿；p5/p1s/p2/p3/p7/f4实际行为仍需新源证据。未提上游；上游具备等价探测后确认行为，
以这些回归、动态恢复、后台流量与CPU覆盖复验退出。

### 33. 快速迁移也使用决定后的往返，并同时复测当前路（2026-11-09，feat/net）

补丁28将快分支用于全部承载后，Moving仍接受决定前250ms内的两个观测，而且只探测候选路。
42528150054已确定性红：Noq434绿、iroh27pass/2fail；决定前的两个包直接得到Go，
候选未答时Hold不要求复测当前路。单场景p1s的14/2次切换是实际待解问题，不等于已证明这两处就是全部原因。

快速分支改为要求包在since之后发出；确认期间两路缺少决定后/本tick证据时沿原250ms tick探测，
当前路原RTT超过2秒时仍不要求其答复。快候选有原两个新观测即可提前切换，不加固定2秒等待；
黑洞leave通道、收益/静默/冷却阈值和闲置探索节奏不改。新增两条回归，旧“决定前一tick即可切换”
断言改为等待再用决定后证据通过；原慢路不阻挡快速迁移的覆盖保留。42528714010单元434Noq/29iroh全绿；p1s/p2/p3/p7/f4实际恢复尚待新源验证。
没有新增独立timer；确认期间当前路可能比旧快分支多PING，仍受既有4秒确认/30秒冷却边界约束，开销须实测。
尚未提上游；上游具备等价决定后证据/共同停顿确认后，用这些回归与实际网络目标复验退出。

### 32. 窗口样本过期时重算同一份质量（2026-11-09，feat/net）

补丁29的record在新ACK时算中位数，但read_paths只删除过期点而不重算。窗口先含300ms/1ms，
300ms过期后，公开样本数已为1，质量却仍300ms；直到下个ACK才更新。42525350514已确定性红：
Noq434绿、iroh26pass/1fail，期望1ms实际300ms。它可能影响稀疏备用恢复，但不是p2/p7实际卡顿的已证实定因。

Reading::expire保留原清理与闭路保留规则，窗口缩小时复用record的中位数计算；仅清空时保留最后已知质量。
不改样本时间、ACK计数或确认历史，不新增timer、探测或paths_stream通知。只在窗口变化时重算最多8点，
不为每次闲置tick分配排序数组。回归同时验证过期不是新ACK；42526508917独立vendor已SUCCESS，实际网络另需对应源码证据。
未提上游，本轮未查最新上游实现；上游提供一致的窗口质量/计数/时刻后，以此回归及p2/p7/防抖场景复验退出。

### 31. 新鲜性使用采样包的真实时间窗（2026-11-09，feat/net）

补丁29仍以actor读取时刻记录样本，且初始RTT改变会入fresh。42518379039取得确定性产品红：
Noq434通过，iroh22pass/3fail，分别复现初始估计变化、延迟读stats和故障前包的积压ACK被算作新鲜往返。

Reading改用noq补丁10的发送/ACK处理时刻；只有ACK计数变化且时间有效才入观测。旧于2秒的ACK不能进入当前窗口或确认，
quality仍可保留最后已知值用于探索，age不因读取刷新。初始估计可更新探索值，但不是已测质量。
迁移确认要求采样包在确认起点后发出，故障离开仍1个观测、正常迁移仍2个独立tick；静默恢复要求包在最后一次静默判定后发出。
原选路收益/确认/冷却/探测频率不改，原传输数学不改。另加旧包ACK不能结束静默的覆盖，原测试改为提供明确的样本时间。

修复版42519038936为434Noq/26iroh全绿（含额外静默覆盖）；真实网络尚待验证，不由单元绿推断恢复或稳定性通过。ACK处理不是内核收包时间，CPU长尾仍需实测。
未提上游，本轮未检查最新上游状态；上游具备同等时刻/确认语义后，以本组、p1/p1j/p1s、p2/p3/p7与f4复验退出。

### 30. 不应答判据与在飞重排共用（2026-11-09，feat/net）

`unanswered_for`委托noq-proto补丁9的同名PathStats方法，保留原两次PTO、max(300ms,3PTO)
等待与最近收包保护。选路和切换后在飞数据重排按同一事实判断，不能因为旧RTT仍快把黑洞路上的流字节留下。
既有iroh不应答/静默/恢复单元与noq同速黑洞、健康同速、慢Backup测试共同覆盖；实际网络结果另见STATE。
本地新增，尚未提上游；上游若提供等价路径活性/重排API，复验后可退出此委托。

### 29. ACK样本新鲜度、近期质量与有界备用刷新（2026-11-09，feat/net）

`RemoteStateActor::Measured` 使用noq-proto补丁8的最新RTT/样本计数，而非从平滑值变化推断新样本。
选路质量为近2秒、最多8个观测tick的中位数；同值新样本仍更新时间，反复读同一份stats不会。
正常迁移确认需要两次新观测，故障离开只需一次新应答；原传输RTT估计、拥塞控制和线上协议不变。
STREAM活跃期跨连接去重、轮转刷新已有备用，每对端最多5秒一个PING；PING/ACK不延长活跃期，闲置无新增timer。
选中Relay/Custom也监测退化，沿用现有探测/打洞节流。

后续同轮补充：通过既有PathState共享只读 `Path::measurement()`（质量、样本年龄、窗口内观测tick数），
诊断和hop看守读取与选择器相同的代表副本。质量与时间戳不再分别取不同连接的值。
没有ACK样本时返回None；`Path::rtt()`仍是传输平滑估计。发布缓存不通知paths_stream，读取不发包。
补充回归 `shared_quality_and_age_come_from_the_same_representative_copy` 与khor-net两个相反历史RTT的清理回归。
首次p7与431/21单元已通过的是补充API前版本；新API/看守接线的结果需单独对应source tree。

回归：noq最新样本计数测试、iroh `equal_rtt_samples_are_fresh_and_recovery_does_not_use_the_old_average`、
`active_refresh_is_bounded_rotates_and_does_not_sustain_itself`，动态netlab `paths::p7_`；完整p1/p1j/p1s/p2/p3/f4检查噪声/恢复。
旧版p7两场景45秒未回快Direct；新候选尚待验证，不以本条记录宣告通过。上游未提交；上游提供可区分真实新样本的路径测量与
有界探测后，按上述场景复验并退出补丁。独立iroh单元使用本包lock和本地noq-proto patch（lock只改来源、版本不变），不能误测registry副本。

### 28. 速度优先时所有路径都确认新鲜测量（2026-11-09，feat/net）

`RemoteStateActor::confirm_move` 不再无条件放行“从中继/转连到直连”。Khor 的
`RoadOrder` 已按有效 RTT 比较所有承载，直连也可能较慢；绕过确认会让一个新路径的
初始样本压过旧路径的新鲜数据。所有仍开放的路径都经过同一个 `Moving` 确认，
`from_rtt` 对所有承载取实际测量，明显快得多时在新样本答上后快速迁移。

回归入口：`khor-net` 的 `measured_speed_wins_for_every_transport_pair` 与
netlab `paths::p6_`；既有 p1/p1j/p1s/p2/p3/p5 与转连场景检查防抖和恢复。
本条是实现记录，通过状态与对照证据见 `docs/net-optimization/STATE.md`，不能仅凭此处宣告验证完成。

以下保留既有补丁的原因与证据。

### 27. `src/socket.rs`：容器和虚拟机的网桥地址不进直接地址（2026-10-21，net-home）

每台装了 Docker 的 Linux 都有 docker0，地址都是 172.17.0.1；对端拿它去打洞，打到的是自己的网桥（hinton、turing、lecun
2026-10-20 每一轮打洞都给了它，各三百来轮）。补丁：`collect_local_addresses` 把名字是网桥的网卡（`virtual_bridge`：
`docker0`、Docker 自建网络的 `br-` 加 12 位十六进制、libvirt 的 `virbr*`、podman 的 `podman*`/`cni-podman*`）上的地址
剔掉，它们既不发布、也不当打洞候选。按网卡名不按网段：172.16.0.0/12 在机群里就是真局域网（MacLab 的 172.16.185.x，
连接记录里还有 172.18.137.x、172.23.250.x、172.25.49.x），172.17 也可能是；OpenWrt 的 `br-lan` 不是 Docker 的。
绑定到某个具体地址时不管（那是自己要的）。改动约 30 行。测试：`socket::tests::khor_container_and_vm_bridges_are_not_offered`。
上游没有对应改动，升级 iroh 时要重新打。

### 26. `src/net_report.rs`：QAD 连接真的复用；一族的探测拿到地址才算完（2026-10-21，net-home）

两处，turing 一天 420 行「中继告知的公网地址」（v6 地址有、没有、又有）的来源。

一、`run_probe_v4`/`run_probe_v6` 开的 QAD 连接存进 `QadConns`，观察值却从 `None` 起，只有地址变了才写；
`current_v4`/`current_v6` 于是一直是 `None`，每一轮报告（25 秒一次）都对每台答 QAD 的中继重新握手，旁边握着的那条
从没被用上。补丁：观察值的初值就是第一次回答。上游 main（2026-10-21）还是 `Watchable::new(None)`。后果：增量报告
只读握着的连接（它的往返是活的 `rtt`，地址变了它的观察任务会写），不再重新握手；QAD 照旧会重测——完整报告（5 分钟，
或网络大变）先 `qad_conns.clear()` 全部重拨，连接断了（`close_reason`）也重拨。
二、`spawn_qad_probes` 一族的探测里任何一个结束（失败也算）就把 `ipv6_pending` 清掉；北京的主机是 IPv4 字面值，它的 v6 探测
查不到 AAAA 当场失败，凑够 3 个 v4 回答就收场，aps1 的 v6 回答还没到——这一轮报告就没有 v6 地址。补丁：拿到地址才清，
或者这一族都结束了（`qad_family_done`）。代价：有 v6 网卡、v6 却不通（只有 ULA）的机器，没握着 v6 连接时每轮报告最多多等
`PROBES_TIMEOUT`（3 秒）。测试：`khor_the_qad_connection_is_reused`（本机中继，第一轮之后握着的连接有观察值；打补丁前红），
`khor_a_failed_qad_probe_does_not_end_the_wait_for_an_address`。khor 这边 `journal::Learnt` 另外把「一轮没有」不算变化。
上游可以提（第一处是明显的疏漏）。

### 25. `src/net_report.rs`：「量到」只认 HTTPS；老断的接班者停用一小时（2026-10-21，net-home）

补丁 3、5、11 和 23 判「这一轮量到了它」统一只认它的 HTTPS 回答：QAD 是 UDP，家却握在 TCP 上。补丁 3 的首选中继
要 HTTPS 答上才当家，已经是家的，任何回答都留住（握着的连接就是 TCP 的证明）；补丁 11 的增量报告没有 HTTPS 量到家就
不动家；挑家时，这一轮只要有中继答了 HTTPS，就只在答了的（和现在的家）里挑，一个 HTTPS 都没有的报告（增量报告、被
QAD 捷径截断的）照上游全都排。

接班者（补丁 5）只凭 HTTPS 答上就搬家，可 MacLab 2026-10-20 15:32 搬到 pk 以后家连接每 20 秒被重置一次，15:35 回明文，
15:37 又去 pk。补丁：`Client::follow_home`（`socket.rs` 绑定时接上 `home_relay_status`）记下家连接每一次从「已连接」掉下来；
某个接班者两分钟内掉了两次，接班规则对本机停用一小时，这期间它也不当候选（除非没别的可选）；网络大变（`is_major`）时恢复。
选「断两次就停」而不是「websocket 稳住 60 秒才算量到」：后者要在中继连接上等一分钟才知道能不能搬，每一台每一次搬家都要
等；前者只罚真断的那一台，而断的样子（TCP 被重置）是那张网的性质，一小时内不会变，换了网才可能变。测试：
`khor_a_successor_that_keeps_dropping_is_left_for_an_hour`、`khor_reached_is_an_https_answer`（都打补丁前红）。

### 24. `src/net_report.rs`：家的滞回对等、最短驻留（2026-10-21，net-home）

上游的滞回不对等：挑战者取近 5 分钟各次探测里最低的，现任只取这一轮最后遍历到的那一个样本；完整报告在开始时清掉
`reports.last`，家从那里读，于是每次完整报告都没有现任、按 5 分钟最低重挑。hinton 只连得上 n0 的 aps1、euc1（往返
177–1216、428–5152 毫秒），2026-10-20 一天换家 12 次，每次都重写设备表那一行。补丁：家记在 `Client::home`（连同
当上家的时刻），不随 `reports.last` 清掉；现任也取近 5 分钟最低（`best_recent`）；`NetReportConfig::home_dwell`
之内家还答着就不因延迟换（khor 传 10 分钟，`endpoint::HOME_DWELL`；默认 0 是上游的样子，上游自己的表测试不变）。
家不答了当场走。一轮什么都没量到时照上游不给家（iroh 于是清掉家，中继回来那一轮报告再把家交给中继，促它当场重连；起初让它留着死了的家，`crates/cli/tests/journal.rs` 的断网测试里两头重连差了几秒，回来后第一次拨号 6–10 秒），但 `Client::home` 不忘，驻留接着算。测试：`khor_the_home_is_weighed_by_its_best_recent_latency_too`、
`khor_a_home_that_answers_is_kept_for_the_dwell`、`khor_hinton_changes_home_rarely_between_two_far_relays`
（hinton 的往返按记录顺序，两小时：打补丁前换 8 次，打补丁后 3 次——剩下的是 aps1 那个 42 秒的样本让它一次完整报告
没答上，走了，驻留满了再回来）。上游没有对应改动，升级 iroh 时要重新打。

### 23. `src/net_report.rs`：兜底的那档只在别的都没答上时当家（2026-10-21，net-home）

负责人 2026-09-30 定「n0 可以兜底」，可 iroh 挑家只比延迟：n0 的中继答 QAD（一个往返），khor 自己的只答 HTTPS
（TCP 加 TLS 握手，几个往返），一比就是 n0 近。`NetReportConfig` 多一个 `fallback: Vec<RelayUrl>`（照补丁 5 的写法）；
挑家时这一轮只要有不在名单里的中继 HTTPS 答上了，就只在它们里挑，都没答上才轮到名单里的。在补丁 3（首选）、
补丁 5（接班）之前：人选的首选中继哪怕在兜底那档也照样当家，接班照旧。khor 这边 `endpoint::bind_pinned` 把
n0 那档（或测试的 `KHOR_PUBLIC_RELAYS` 替身）传进来，自己名单里写了的不算兜底。hinton 只有 CERNET v6 出得去，
自家三台只有 v4，它的家仍然只能是 n0，这是对的。测试：`khor_the_fallback_tier_is_home_only_when_no_other_relay_answered`
（打补丁前红）。上游没有对应改动，升级 iroh 时要重新打。

**跑这几条测试**：vendor 的 iroh 单独编译时用的是 registry 的 noq-proto，补丁 17 要的 `PathStats` 字段没有，
要把根 `Cargo.toml` 的那条 patch 带上（会改 `vendor/iroh/Cargo.lock`，跑完 `git checkout` 回去）：

```sh
cargo test --manifest-path vendor/iroh/Cargo.toml \
  --config 'patch.crates-io.noq-proto.path="<仓库>/vendor/noq-proto"' --lib khor_
```

### 22. `src/net_report.rs`：主机是 IPv6 字面地址的中继也能问 QAD v6（2026-10-17，path-fix）

`run_probe_v6` 拿 `relay.url.host_str()` 当 TLS 的服务器名。URL 里的 IPv6 字面地址带方括号（`[2001:db8::1]`），
带着方括号它既不是 IP 也不是域名，握手根本没开始，QAD v6 永远没有回答。一个用 IP 写的 v6 中继（实验里的 QAD 替身，
或者没有域名的自建中继）因此只能问 v4。改为去掉方括号。改动 3 行。测试：`crates/cli/tests/netlab/paths.rs` 的 p4
（双栈，v4、v6 各一个 QAD 替身）。上游可以提（要点：`host_str` 对 IPv6 字面地址带方括号，不能直接当 server name）。
升级到 1.3.0 时要看上游是否已改，没改就重打。

### 21. `src/socket/remote_map/remote_state/path_watcher.rs`：路径事件的缓冲放大，路径可以单独 ping（2026-10-16，path-fix）

两件观测上的小事。一、每条连接的路径事件广播缓冲 8 条（`BROADCAST_CAPACITY`），一轮打洞在有几个候选的连接上就是一串
「打通、选中、关掉」，读得慢一点就丢；丢了只给一个 `PathEvent::Lagged`，khor 原来连这个也不看（`roads.rs`），连接记录里
那几次改走就没了。改为 64；khor 这边收到 `Lagged` 记一行「漏记了 N 条路径变化」，再把当时选中的那条记下。
二、`Path::ping()`：在这条路径上马上发一个 PING。khor 判一条线死之前先 ping 每条路径（`wires::ask_every_path`），
有一条答了就不算死（见下面补丁 17 的「判死和换路对齐」）。noq 的 `Path::ping` 本来就有，iroh 的 `Path` 没露出来。
改动约 15 行。上游可以提：缓冲大小那一处是纯参数，`ping` 是 noq 已有能力的转出。升级到 1.3.0 时两处都要重打。

### 20. `src/socket.rs`、`src/socket/remote_map/remote_state.rs`：直接地址只看地址、不看来源（2026-10-16，path-fix）

turing 的公网 IPv6 就是它网卡上的地址，前面没有 NAT。`update_direct_addresses` 先放 QAD 学到的地址、再放网卡地址，
都用 `or_insert`，于是这个地址在带 QAD v6 回答的那一轮报告里是 `Qad`，在不带的那一轮里是 `Local`，每 20–26 秒换一次。
`DirectAddr` 连来源一起比，集合就算变了：每变一次，iroh 重发布一次地址（`store_direct_addresses`），每个远端的 actor
都被 `local_direct_addrs.updated()` 叫醒、打一轮洞（上一轮在 5 秒以前就当场打）；khor 只数 `Local` 的地址，把它当成
「本机网卡地址变化」，每 40–60 秒一次全网同步（`link.rs` 的 `Change::Addrs`），连接跟着反复开关路径。
改三处：一、网卡上的地址一律是 `Local`（`collect_local_addresses` 里 `or_insert` 改成 `insert`），QAD、端口映射先报了也一样；
二、`store_direct_addresses` 只在地址集合变了时重发布；三、actor 只在给连接的候选地址集合变了时才更新候选、打洞
（`update_local_direct_address` 返回变没变，`State::last_local_candidates`）。来源照旧随 `watch_direct_addrs` 给出去。
khor 这边 `journal::interface_addrs` 也改成按地址集合判断：告诉过的网卡地址只要还在 iroh 的直接地址里，换了来源也不算变。
改动约 30 行。测试：`crates/cli/tests/netlab/paths.rs` 的 p4；`crates/node/src/journal.rs` 的
`an_address_that_changes_only_its_source_is_no_change`。**可以起草上游**（要点：一个地址既在网卡上又被 QAD 报出时来源
在两种报告之间来回变；来源变了不应重发布、不应打洞；网卡地址的来源应固定为 Local）。上游 1.3.0 的 changelog 没有这类改动，
升级时要重打。

### 19. `src/socket/remote_map/remote_state.rs`：选中的直连变差了当场打洞（2026-10-16，path-fix）

上游只在三种时候打洞：新连接、任一方候选变化、每 60 秒检查一次且没有一条 10 ms 以内的直连。没有备份（补丁 18 不保留，
或 iOS 上默认不保留）时，client 已经把别的直连都关了；选中的那条慢下来或不答了，另一条直连要等下一次 60 秒检查才会重新打开。
改为：每个测量 tick（补丁 17）看选中的直连：两次 PTO 没有回音，或者平滑 RTT 超过 10 ms、且比它被选中以来的最低值高出
max(5 ms, 最低值的一半)，就当场 `trigger_holepunching()`。每次变差只打一轮（`State::degraded`，回落到一半门槛以下才复位），
两轮之间至少 5 秒的老规矩不变。平时不多打：只在变差的那一刻多一轮。
**闲着的连接按需多量几次**（10-17 补）：只有保活的连接 5 秒才一个 RTT 样本，平滑 RTT 一个样本只走八分之一，
变慢到 21 ms 要五个样本（25 秒）才看得出。于是从平滑 RTT 这一 tick 的跳变反推出刚到的那个样本
（RFC 6298：跳变 = (样本 − 旧值) / 8），样本超过 10 ms 且高出门槛，就在 2 秒里（`PROBE_FOR`）每个 tick 给选中的路发一个 PING，
约 8 个样本，平滑 RTT 两秒内就说清是真变慢还是一下抖动。每次变差最多一次，且 30 秒内最多一次（`PROBE_SPACING`），
Wi-Fi 那种常抖的路最坏也就是半分钟 8 个 PING 加 8 个 ACK。只 ping 这条路的一份（10-17 第三棒改；原来每条连接上的那份都
ping，两条连接就是两倍——机器忙时 p4 里在用的路因此约 2 秒一个 PING）。改动约 90 行（`watch_selected`）。
测试：`crates/cli/tests/netlab/paths.rs` 的 p2（第二条直连变慢到约 21 ms、同时在打字）和 p2i（同样但闲着）。上游可以提，和补丁 17 一起。
升级时要重打。

**10-21 修订（net-flap）：打洞轮数**。真机 24 小时的连接记录：MacLab→lecun 3555 轮、MacLab→hinton 2530、hinton→lecun 2486
（每分钟 2.5–3.4 轮；上游的规矩是不够好时每 60 秒一轮），MacLab→lecun 那一对两边候选整天没变、只换过两次路，10 分钟一行的
「打洞 N 轮」最多 104 轮，差不多每 5 秒（两轮之间的最短间隔）一轮。三个放大处，各改一处：
一、**一次不答一轮**：「不答」当场打一轮（上面的 `degraded`），闲着的路一个保活的 ACK 晚到 0.1 秒就判了不答（补丁 17 的
10-21 修订修的就是这个），下个 tick 又答了，`degraded` 复位；每次换路也把它复位。改为：选中的路**卡了 1 秒**（`PUNCH_SILENT_AFTER`，
从它没答的那个包发出算）才打；更短的卡顿由选路器决定是熬过去还是换到别的路上，从不答的直连换到中继或转连的那一刻打一轮
（`select_path_on`）；这一处打的轮之间至少 30 秒（`PUNCH_SPACING`，`Watch::punched_at`），换路不复位。
二、**对方的地址一帧一轮**：每个 ADD_ADDRESS（`addr_events`）都当场 `trigger_holepunching`，候选多了一个就不是旧候选的子集，
当场打。新连接上对方的每个地址各一帧，记录里常见同一秒「对方候选 无」一轮接着又一轮。改为 100 ms 内的几帧并成一轮（`ADDR_SETTLE`，
`State::punch_soon`）。
三、**60 秒的检查要每条连接都够好**（`check_connections` 对所有连接取「与」，各用自己那份的 RTT）：对方拨进来的那条连接只有
对方能开路，闲着的那份 RTT 是上一个保活的，任一条不够好就每 60 秒一轮。改为任一条连接上有 10 ms 以内的直连就够（RTT 按
`Measured::road`，即载着流量的那份的读数）。
逻辑抽成 `Watch`（选中那条的看守）、`good_enough`，单元测试见补丁 17 的修订。改前改后的打洞轮数见 docs/NET.md「选路」一节。

### 18. `src/socket/remote_map/remote_state.rs`：选路器可以留一条备用直连（2026-10-16，path-fix）

上游 client 端每次选路都把没选中的直连全关掉，每条连接只留一条（注释里的理由：只让 client 关路，免得两边各关一条、
竞相放弃最后一条）。于是选中的那条一出事，就只剩中继，等下一轮打洞。改为：`PathSelection::set_backup(&path)` 让选路器
点名一条直连作备用，`apply_selected_path` 不关它（仍然只有 client 关路，最多留两条直连，不变式不变），
`PathSelectionContext::backup()` 告诉选路器现在留的是哪条。不点名就和上游一样。
没选中的直连（备用、以及 server 端还没被 client 关掉的那些）换成慢一点的保活：15 秒一个 PING、45 秒空闲超时
（`SPARE_KEEP_ALIVE`、`SPARE_IDLE_TIMEOUT`；选中的仍是 5 秒、15 秒），只在路径状态变了时设（设空闲超时会重排定时器）。
留不留、留哪条是 khor 的 `carry::RoadOrder` 定的（`carry::Spare`：`KHOR_SPARE_ROAD=off|near|on`，默认 near，iOS 默认 off；
near 只留比当前慢不超过 10 ms 的那条）。改动约 60 行。测试：p3（当前路黑洞，备用直连当场接上）。
**中继路径排在直连后面时也 15 秒一个保活**（`pulse`，10-17 第三棒）：上游每条路径 5 秒一个保活，中继也是，两边各 ping，
noq-proto 补丁 4 之后每个 PING 的 ACK 也走中继（走 TCP，到中继还要一个 TCP 的 ACK）。p4 在 PATHDBG 下量得中继路径占
QUIC 包的三到六成。中继被选中时照旧 5 秒；空闲超时不动（`RELAY_PATH_MAX_IDLE_TIMEOUT` 30 秒，容得下两个保活）；
中继本身在不在由中继连接自己的 ping 管（补丁 15），这一处只是 QUIC 路径的保活。备用直连和中继的 15 秒要靠 noq-proto
补丁 6 才作数（连接级的 5 秒保活原来把它们一起 ping 了）。量得（netlab，alpha 一侧两条连接合计，QUIC 包，PATHDBG）：
p4 里中继每份从约 0.4 个/秒降到约 0.14 个/秒，备用直连从约 0.3 降到约 0.15。
上游可以提（要点：`PathSelection` 本来就写着「将来可以选多条」，backup 是最小的一步）。升级时要重打。

### 17. `src/socket/remote_map/remote_state.rs`、`src/socket/biased_rtt_path_selector.rs`：按测量持续选路，不答的路不算（2026-10-16，path-fix）

负责人 10-16：「为什么明明有最优路径却没有走」。上游的 `select_path` 只在三处调用：连接加入、路径打通、路径放弃。
一条直连变慢了，没有人再问选路器；一条直连黑洞了，它的 RTT 还是最后量到的那个，照样被选着，直到 noq 在最后一个包之后
15 秒（`PATH_MAX_IDLE_TIMEOUT`）放弃它——而 khor 判死只要 4 秒，结果总是收回连接、重拨，而不是换路。
改两处：一、actor 主循环加一个 250 ms 的测量 tick（只在有连接时跑，`RESELECT_INTERVAL`），每次按各路径此刻的 stats
重跑选路器；选择或备份变了才 apply，没变什么也不做。tick 不发任何包。二、`PathSelectionData::answering()`：
连续 `UNANSWERED_PTOS`（2）次 PTO 没有任何确认、**而且这条路至少 `UNANSWERED_FOR`（300 ms）什么也没收到**的路径
不再是候选，当前那条也一样（`unanswered`；noq-proto 补丁 3 把 `pto_count`、`last_received` 放进 `PathStats`）。
两次 PTO 本身就是包发出后约三个 PTO，1 ms 的路上约 75 ms，比机器卡一下还短：二十个场景一起跑时两条路都好好的，
PTO 计数也到过 2、3（10-17 第三棒，PATHDBG 打点）；300 ms 的下限只管快路，慢路上三个 PTO 早已超过它。
p3（当前路黑洞）因此多等至多 300 ms，量得最坏一次回显仍在 1 秒以内（见下）。
上游自己的 `BiasedRttPathSelector` 也跳过不答的路；khor 的 `RoadOrder` 同样。
**一条路按它的几份连接合起来算**（`RemoteStateActor::read_paths`、`Measured`，10-17 补）：两台机器之间常有两条连接，
同一条路在每条连接上各是一条 path。一、不答按最差的那份算，而且记住：任何一份连续两次 PTO，这条路就不答，直到某一份
在那之后又取到一个 RTT 样本（一个往返回来了）才算答——原来按每份各算，闲着的那份 PTO 是 0，把死了的那份盖住了，
p3 里连接在黑洞的路上停了 15 秒；PTO 过后计数会被下一个探测清零，不记住就会在两条路之间来回跳。二、RTT 用最近取到样本
的那一份的（`PathSelectionData::rtt`）：闲着的那份只有 5 秒一次保活，还是变慢以前的值，p2 打字时它把变慢盖住了 10 秒。
每个 tick 只读 path stats（每条路一次），不发包。
**PTO 只量这条路自己**，要靠 noq-proto 补丁 4（每条路的 ACK 走它自己）；没有它，ACK 走对方选的那条路回来，
对方那条一断，这边每条路连中继都「不答」，没有可换的。
比较仍用 noq 的平滑 RTT（RFC 6298 的 7/8 + 1/8）和原来的迟滞（直连之间快 5 ms 才换），同速时不换；
平滑 RTT 里混进过 4.7 秒、17.6 秒的假样本（noq-proto 补丁 5 修的），p1 的来回换路一半是它们。
**离开一条还在答的路之前，两边都量过才走**（`confirm_move`、`CONFIRM_FOR`，10-17 第三棒）：一条闲着的路一个保活
才一个样本，只回 ACK 的那一边（noq-proto 补丁 4 让 ACK 走路自己，回 ACK 的包推迟了它自己的保活）一个样本也没有。
二十个场景一起跑时（第二棒那一轮 p1 红，alpha 改走 9 次），打点看到的两次改走都不是「不答」，PTO 计数全是 0：
一次是在用的局域网读到 8.5 ms（负载下 ACK 晚了），第二条路读的是三分钟前的 3.4 ms，差过 5 ms 就走了；
另一次是机器卡了约 100 ms，一个样本把在用那条的平滑 RTT 抬到 17 ms。拿此刻的测量和几分钟前的比，是这类来回换路的根。
改为：tick 想从一条还在答的路换走时，先不换，接下来 2 秒每个没读到新样本的 tick 给两条路各 ping 一次，
2 秒里每个 tick 选路器都还要换、而且两条路在这期间都取到了样本，才换；中途有一个 tick 不想换了就作罢。
当前那条不答了、没了，或者本来没有选中的路，照旧当场换。代价：只在想换的时候才 ping，每回至多 8 个 tick × 两条路，
每个 PING 一个 ACK；p2（打字时在用的路变慢）换回局域网量得 0.6–3.3 秒（打字时样本本来就新，多半不用等满 2 秒），
仍在 10 秒以内；二十个场景一起跑时 p1、p1j 十分钟里两边都没改走过（同一轮改前的树：p1 各 1 次，p1j beta 2 次）。
**路径打通、放弃时的改走也要确认**（`select_path_on`、`confirm_move`，10-17 第四棒）：上面的确认原来只管测量 tick，
连接加入、路径打通（`PathEvent::Established`）、路径放弃（`Abandoned`）这三处事件照旧当场跑选路器、当场换。
第四棒在 PATHDBG 下记下每次改选的调用点（`#[track_caller]`）和当时每条路的读数，二十个场景一起跑了四轮：
直连之间的改走 47 次，30 次来自「路径打通」，其中 29 次是从一条还在答的路换走（PTO 0、几 ms 前刚收到过包）；
读数是新打通那条握手时的一两个样本（1 ms 的路上 0.85–4 ms）对在用那条负载下的平滑 RTT（8.5–17 ms），
差过 5 ms 就走——正是 tick 那里已经挡住的「拿新样本比旧读数」。tick 上的 15 次里 10 次是当前那条不答了（p3 黑洞），
5 次是确认满 2 秒以后的（p2、p2i 第二条变慢）；放弃事件 4 次。一轮打洞在两条连接上各打通两条路，
每打通一条就是一次这样的比较；第三棒那一轮 p1 同一秒里来回三次（`second@284s, lan@284s, second@284s`）就是这样。
改为：任何改走都走 `confirm_move`，事件里想换的只是开始确认（`moving`），由之后的 tick 照同一套规矩做完或作罢；
确认期间要换去的那条路 client 不关（`apply_selected_path`），作罢时再关。只有三种当场换：当前那条没了、不答了，
或者从中继、转连换到直连——两个选路器（khor 的 `RoadOrder`、上游的 `BiasedRttPathSelector`）跨这条线都不比 RTT，
直连开着就走，没有可确认的。改动约 20 行。
**打洞是因不是果**：第三棒那一轮的 13 轮打洞（第一轮 3 轮）是负载把在用那条的平滑 RTT 抬过 10 ms，
上游 60 秒一次的检查（`check_connections`，`GOOD_ENOUGH_LATENCY`）就打一轮；每一轮打通路径、每次打通当场比较，
才有了改走。改走本身不触发打洞（补丁 19 的「变差当场打」只看选中那条自己的 RTT 比它的最低值）。
改走之后负载照旧，平滑 RTT 照旧过 10 ms，下一个 60 秒又一轮，所以两者一起涨。修好以后打洞照样会因负载多打几轮
（这是上游的规矩，不动），但打通的路不再当场把连接拉走。
**一条路的读数由取样最多的那份说**（`Measured::road`，10-17 第四棒）：上一段改完，第三轮全套 p2 红（打字时第二条变慢，
14.2 秒才换回局域网），另一轮 p2 的 beta 8.4 秒。PATHDBG 看到确认没有满 2 秒就作罢、重来，一次换路作罢三次：
一条路原来取「最近取到样本的那一份连接」的平滑 RTT，闲着的那份 5 秒一个保活样本，取到的那一 tick 它就代表这条路，
把已经慢到 21 ms 的路读成 5 ms，选路器这一 tick 不想换了；载着这条路的那份路径一关（对方先换走了），剩下闲着的
那份读 4 ms。只取「最近的」，就是让闲着的那份连接在它取样的那一刻说了算。改为：近 `CONFIRM_FOR`（2 秒）里 RTT
变得最多的那一份（取样最多的，就是在载流量的那份）说了算，一样多取最新的；那一份的路径关了，它的读数也留到 2 秒满。
「这条路这期间有没有新样本」仍看所有的份。改动约 60 行。之后两轮带 PATHDBG 的全套 p2 2.5、3.1 秒，p2i 2.8、7.6 秒，
确认作罢重来 8 份日志里只有 1 次。
改动约 230 行。测试：`crates/cli/tests/netlab/paths.rs` 的 p1、p1j（同速不动）、p2（变慢 10 秒内换回）、p3（黑洞时
回显没有一次等过 1 秒、两边的线都不收回）。**可以起草上游**（要点：选路只在开关事件时跑，测量变化不触发；不答的路
RTT 不变，选路器看不出来；建议在 actor 里按 path stats 周期重选，并在 `PathSelectionData` 上给出「是否在答」；
「不答」要有时间下限，不能只看 PTO 次数；从还在答的路换走前，两条路都要有此刻的样本）。
上游 1.3.0 没有对应改动，升级时要重打。

**10-21 修订（net-flap）：不答从没答的那个包算起，卡顿短于别的路就熬过去**。负责人 10-21 同意修网络审查的全部问题；
真机 24 小时的连接记录（10-20 15:00 部署之后网络代码没变）：MacLab↔hinton「改走中继 aps1」→「改走直连」成对 212 次，
间隔中位 250 ms；同一对机器两条局域网地址之间 lecun→turing 330 次、turing→hinton 307、hinton→lecun 229、lecun→hinton 227，
64–81% 落在本机 30 秒同步（`link.rs` 的 `QUIET_PACE`）那 5 秒里；「改走直连」记下的往返在 0–1 ms 的局域网上 p90 30–80 ms、
最大 2 秒（换去的那条也正卡着）。根在「不答」：
一、`unanswered` 要「两次 PTO，且这条路 300 ms 什么也没收到」，可闲着的路 5 秒才收到一个保活的 ACK，「300 ms 没收到」
恒真，实际只剩「两次 PTO」——2 ms 的路上 ACK 晚约 0.1 秒（Wi-Fi 省电、turing 在 NFS 上卡一下）就判死，选路器当场离开
（离开不答的路不用确认），下一个 tick 有了样本又当场换回（中继→直连也不用确认），补丁 19 每次再打一轮。
改为：从这条路上**最早还没被确认的那个要回 ACK 的包**发出算起，等了 max(300 ms, 3·PTO) 才算不答（`unanswered_for`；
noq-proto 补丁 3 的 10-21 补露出这个时刻 `unacked_since` 和 `pto`），仍要两次 PTO、300 ms 没收到东西。
代价：真黑洞时判得晚一点。p3（打字时在用的路黑洞）原来最后收到的包就在黑洞前一刻，最快约 0.1 秒就判；现在从黑洞后第一个包算满
300 ms，最慢的回显从 0.39、0.41 秒变成 0.68 秒（门槛 1 秒）。
二、**清「不答」要迟滞**：原来任何一份连接取到一个新样本当场就算答了；改为取到样本、且之后 1 秒（`SILENT_HOLD`）里没有一份
再不答，才回到候选（`Silence`、`Measured::mark_silent`）；中途又不答就重新算。
三、**卡顿还没长过别的路的往返，不离开在用的直连**：选路器拿得到这条路卡了多久（`PathSelectionData::unanswered_for`，
从那个包发出算起；回到答的那一刻停表），khor 的 `RoadOrder` 在它短于其余候选里最快那条的往返时照旧留在这条直连上
（`carry::rides_out`）——此刻从别的路发出的包，不会比这条欠着的回答更早到。p3（黑洞，备用直连 9 ms）照旧过了 300 ms 就换。
四、**确认有上限**：确认期 2 秒里两条路没都取到样本，原来就一直确认、每个 tick 都 ping 两条路，没有尽头（这一节上面写的
「至多 8 个 tick」代码没守）；改为 4 秒（`CONFIRM_MAX`）还没成就作罢，同一次改走 30 秒（`CONFIRM_REST`）内不再试（`Moving`）。
五、**直连之间刚换过一分钟内要多一倍余量**（khor 的 `RoadOrder`，`carry::direct_margin`、`DIRECT_DWELL`）：余量原来是固定 5 ms；
改为 max(5 ms, 在用那条往返的四分之一)，换路后 60 秒里（`PathSelectionContext::held_for`，`State::selected_at`）max(10 ms, 一半）。
真机记录里同步那一阵把在用的局域网抬到 5–8 ms、闲着的那条读 1 ms，差过 5 ms 就换，换过去那一阵跟着过去，30 秒后又一遍；
p2 那种真变慢（1 → 21 ms）照旧当场换。
六、**离开不答的路，只去在那之后答过的路**（`Moving::leave`）：原来不答的路当场离开，去选路器点的哪条都行。机器卡住时每条路都卡，
闲着的那条没有在飞的包，看起来照样在答：p1s（每 30 秒 alpha 两条腿都慢 300 ms、两个常驻挤在一个 CPU 上）第一轮改后十分钟换了
3 次。改为：要去的那条在卡顿开始之后取到过样本才走，没有就每个 tick ping 它，4 秒（`CONFIRM_MAX`）还没有就照走。
为了不在判不答之后再等一个 tick（p3 黑洞时最慢的回显因此到过 0.9 秒），选中的路有个包等了 150 ms（`PROBE_OTHERS_AFTER`，
已过一次 PTO）就先 ping 其余每条路一下（`probe_others`，每条一份、每 tick 至多一次、取到样本就停），判不答那一刻要去的那条已经答过了。试过另一条路——一条路读近 2 秒里最低的样本，好让卡顿留下的高平滑值不算数——p1s 没变好，
p2i（闲着时第二条变慢）从 2.6 秒变成 14 秒，没留。
测试：`remote_state.rs` 的单元测试（合成的 `PathStats`、读数）`an_idle_road_whose_ack_is_late_still_answers`（红例：闲着的
2 ms 路 4 秒前收到上一个 ACK、两次 PTO、没答的那个包才发出 100 ms——改前判不答，改后不判）、`a_slow_road_waits_three_probe_timeouts`、
`a_road_answers_without_two_ptos_or_while_heard`、`a_silent_road_answers_again_after_the_hold`、`the_watch_punches_on_a_long_stall_once_per_spacing`、
`the_watch_punches_on_a_slowed_road_and_probes_a_slow_sample`、`a_move_goes_once_both_roads_sampled_for_two_seconds`、
`a_move_never_confirmed_is_given_up_and_rests`、`a_silent_road_is_left_for_one_that_answered_since`、`one_good_direct_road_is_good_enough`、`rounds_asked_for_together_are_one`；
同样的测试换回改前的规则（拷一份改回去跑）8 个红，改后全绿。`crates/net/src/carry.rs` 的
`a_stall_shorter_than_the_other_road_is_ridden_out`、`a_move_between_direct_roads_holds_for_a_minute`。
netlab 新场景 p5（直连 + 500 ms 远中继，每 30 秒丢直连 150/300/600 ms、其间一个按键）、p1s（p1 + 每 30 秒 300 ms 的延迟尖峰、
两个常驻挤在一个 CPU 上）；改前改后的数见 docs/NET.md「选路」一节。
**跑这些单元测试**：vendor/iroh 单独编译时用的是 crates.io 的 noq-proto，没有补丁 3 的字段，编不过；要把 vendor/iroh 和
vendor/noq-proto 拷到仓外一个目录，加一个 `[workspace] members = ["iroh", "noq-proto"]`、`[patch.crates-io] noq-proto = { path = "noq-proto" }`
的 Cargo.toml，再 `cargo test -p iroh --lib remote_state`。

**10-23 修订（flap）：离开中继、转连时，快得多的路不等两秒确认；被换走的路比确认还慢时不等它的样本**。手机换网、回前台时，
在用的转连（经 turing 约 20 ms）卡住，选路器去了中继（备用路 15 秒才保活一次，看起来在答），中继此刻往返 1743、3468、
4994 ms；转连答上以后要 `SILENT_HOLD` 1 秒加 `CONFIRM_FOR` 2 秒，还要中继也取到一个样本才回去，中继一个往返超过 4 秒时
取不到，到 `CONFIRM_MAX` 作罢、`CONFIRM_REST` 30 秒不再试。改两处，都在 `Moving::step`（`confirm_move` 把两条路此刻的
读数传进去；**只在从中继或转连换走时传**，直连之间传 0，规则和改前一样，p1、p1j、p1s 不受影响）：
一、**目标快得多就当场走**（`far_faster`：目标的两倍加 `FAR_FASTER_BY` 10 ms 仍不超过在用那条）：目标在这次改走开始以后
取到过样本就走，不等 2 秒，也不要求被换走那条有样本、不 ping 它。两倍：目标的读数就算差了它自己那么多，也还是快的那条，
没有可确认的；10 ms 让两条几毫秒的快路不算差得远。原先试过只看读数、不分路的 100 ms 余量（为了挡住 p1 负载下 17 对 3.4 ms
那种直连之间的假读数），f4 里中继 300 ms 对转连 116 ms 就不算快得多，又等满 2 秒；改为直连之间不参与，余量回到 10 ms。
二、**被换走那条的读数超过 `CONFIRM_FOR` 时不等它的样本**：2 秒里它取不到样本，不能因此作罢、进 30 秒的 rest；
满 2 秒、目标取到过样本就走。目标一直没样本，到 `CONFIRM_MAX` 照旧作罢。
没改的（这一轮不做）：在用的转连卡住时 khor 的 `RoadOrder` 不像直连那样熬一熬（`carry.rs` 只把直连放进 stalled），
当场离开；中继作为备用路 15 秒才一个样本，读数是旧的；`SILENT_HOLD` 1 秒照旧。所以卡顿时仍会去中继一趟，
修的是回来那一步。
量到的（netlab f4，turing 上四份并跑，带临时打点）：改后选路器想回转连的那一刻起，确认一个 tick（0.25 s）就走，没再出现过 rest；
改前只跑成一份有效的：m 解封后 4.9 s 才回转连，在中继上 5.5 s；lab 里没复现出 30 s 的 rest（中继单程 22 ms 加 1.7 s，
没到一个往返超过 4 秒）。中间用过 100 ms 余量的那一版有一份在中继上停了 16.8 s（转连读数 124 ms 对中继 810 ms，不算快得多，
等转连样本等了很久），是那一版的问题，不是改前。但 f4 照它的门槛（x–m 那条腿重新听到后 1.5 s 内回转连）仍红：
腿听到后 3.0–4.3 s 才回去，时间花在转连这条路自己的第一个样本上（腿通了以后 2–3 s，转连路径的 PTO 在 3 秒黑洞里退避过了）
加 `SILENT_HOLD` 1 s；另有三份在回到转连后约 10 s 又掉回中继（转连那条路径被关掉，`confirm_move` 里 `open` 为假、当场换），
都是转连这一层的事，不在这一轮。试过每个 tick ping 不答的非选中路（`probe_silent`），没看出变快，没留。
测试：`a_far_faster_road_is_taken_without_the_two_second_confirm`（1743 → 20 ms 取到样本就 Go；30 → 20、300 → 150 ms 仍 Hold；
309 → 116 ms Go）、`a_move_off_a_road_slower_than_the_confirm_is_not_given_up`（4994 → 54 ms 到 4 秒不是 Rest；2500 → 1400 ms
满 2 秒就走、不 ping 慢的那条），改前两个都红；netlab 场景 f4（`crates/cli/tests/netlab/hop.rs`：y 一直在给 x 打字，
x–m 黑洞 3 秒，x–中继黑洞 1.5 秒、中继发给 x 的晚 1.7 秒到 m 回来后 1.5 秒）。khor 这边 `roads.rs` 把每条连接起步那条路
记成「新连接走…」（`started`），不再写成「改走…」（f4i，改前红、改后绿）。

**10-23 修订（carry）：腿通了，转连当场答上，答上就回去**。上一节修完 f4 仍红（x–m 那条腿重新听到后 3.0–4.3 秒才回转连）。
带临时打点（每 tick 每份路径的 PTO 次数、最早欠回答的包、最后收到的时刻；每次改选；每条 hop 链路收到帧的间隔，打点不提交）
四份并跑量下来，时间不在 y 的转连路自己身上，在 m 到 x 那条腿上：
一、**闲着的那份连接把腿判成不答**：黑洞里 m（x 也一样）到对方的连接改走了中继（中继发给 x 的晚 1.7 秒）；腿通了，m 在用的那份
连接一个 tick 内就取到样本，可同一条直连在 m 另一条闲着的连接上还在 PTO 退避里（pto_count 4–5，下一个探测要等 1–2 秒）。
「一条路按最差的那份算」，于是这条路仍算不答，再加 `SILENT_HOLD` 1 秒，m 在腿通后 1.9–3.3 秒才回直连（x 腿通在 +3.0 s，
m 回直连 +4.9、5.1、5.5、6.25 s）。x 那边更糟：一份在腿已通之后（+3.9 s）把它到 m 的连接从刚答上的直连挪去了中继。
改：`RemoteStateActor::wake`——一份路径在 PTO 退避里（`pto_count ≥ 1`、有欠着的包），而这条路在那个包发出之后从任何一份上收到过东西，
就每个 tick ping 这一份，直到它自己的往返回来（ACK 清掉退避，RFC 9002）。黑洞里什么也收不到，不 ping；没有欠包的份不 ping。
改后 m 回直连 +3.44–3.57 s（腿通后一个 tick）。
二、**按序交付把腿堵住**：m 回到直连以后，hop 那条流在 m→x 这一段还挡着 m 走中继时发的字节（晚 1.7 秒到），流按序交付，后面的
全等它：x 收到 y 的帧比 m 回直连晚 1.5 秒（+4.98 s），y 的转连路第一个样本 +5.06–5.44 s。修在 vendor/noq-proto 补丁 7
（离开慢得多的路时，把它上面在飞的东西在新路上重发）。
上一节试的 `probe_silent` 看不出变快，原因在这两处：ping 确实发出去了，但发在 y 这一层，进了 hop 的流，被 m 那条走中继、
又被按序交付挡住的腿压着；y 自己的退避不是瓶颈。
三、**`SILENT_HOLD` 只给时好时坏的路**（`Measured::mark_silent` 的 `quick`、`REHOLD` 30 秒）：在用的是中继或转连时，在用的那条
本身（hold 只会让选路器把一条已经答上的路丢掉，x 那次就是）和比在用的快得多的那条（`far_faster`），第一个往返就回候选；
但若它上次回来不到 30 秒又不答了，就是时好时坏的路，照旧等满 1 秒。hold 防的是一 tick 走、一 tick 回（10-21 的一天 212 次），
那种路第二次不答一定紧跟在回来之后，`REHOLD` 接住；单次卡顿本来就只走一次、回一次，hold 只让它在慢路上多待 1 秒。
取舍：一条每 30 秒以上才卡一次的快路，每次都当场回去，连接记录里仍是一走一回两行（和改前一样多）。在用的是直连时不免，也不 wake，
直连之间的规则和改前一字不差：第一版不分在用的是什么，p1s（两条直连同时卡 300 ms）同批对照改后 6 份红 3、改前 6 份全绿，怕的是
卡顿中途 wake 出的 ping 在一条路上取了一个卡顿长的样本。收窄以后再量，p1s 本身在负载下就不稳：29 个场景同批时改前 5 份红 4、
收窄后 5 份红 3、收窄后再去掉 noq-proto 补丁 7 的 5 份红 4；收窄后合计 11 份红 5，改前合计 11 份红 4。看不出收窄后的树比改前差，
也说不上第一版那 3 份红一定是 wake 造成的。
四、**快得多的目标，前一个 tick 的样本也算**（`Moving::step` 的 `fresh`）：答上的那个样本就是这一 tick 读到的，原来还要再 ping、
多等一两个 tick。
量到的（netlab f4，turing 上四份并跑，带打点；时刻从 x 断开算，腿在 +3.0 s 接回）：

| | m 回直连 | y 转连第一个样本 | 回候选 | 改选 | 腿重新听到（测试量）→ 回转连 |
|---|---|---|---|---|---|
| 改前（4 份） | +4.9–6.25 s | +4.96–5.33 s | +5.96–6.33 s（hold 1 s） | +6.21–6.58 s | 2.1–2.8 s，全红 |
| 只改一、三（4 份） | +3.44–3.54 s | +5.06–5.44 s（3 份），+3.40 s | 同一 tick | +0.5 s | 1.7–2.6 s，三红一绿 |
| 一至四 + noq-proto 补丁 7（4 份） | +3.46–3.57 s | +3.66–3.79 s | 同一 tick | 同一 tick | 0–0.2 s，全绿 |

不带打点的树又跑了八份（和十几个场景同批；四份是收窄前的第一版，四份是提交的这一版），都在 1.5 s 内（0–0.5 s），没有一份回到
转连后再掉回中继。同一版去掉 noq-proto 补丁 7 的四份红二（1.8、2.0 s）：二、是真的。
khor 这边另一处（不在 iroh 里）：回到转连后约 10 秒又掉回中继，是 hop 自己的看守把卡顿时的中继当成了「量过、转连不快」，
见 `crates/node/src/hop.rs` 的 `open_hop`、`khor_net::path::relay_beats_carried`。
测试：`a_backed_off_copy_is_woken_when_its_road_is_heard_again`、`a_road_far_faster_than_the_one_in_use_is_back_at_its_first_round_trip`、
`a_far_faster_road_is_taken_without_the_two_second_confirm`（加了前一个 tick 的样本）；netlab f4（加了「回到转连后 30 秒不掉回中继」）、
f4s（见下）。
**没做到的**：f4s（中继往返 5 秒、腿黑 8 秒、只在前 3 秒打字）想复现上一节修的 30 秒 rest，改前的树四份都没进 rest（最长在中继上
8.2、10.2 秒）；改后的树有时红在另一处：长卡顿以后转连路的平滑 RTT 被积压的样本抬到 500 ms 上下，连接闲着、一个保活才一个样本，
`RoadOrder` 拿它比此刻 450 ms 的中继，换去中继，一待 27 秒。这是平滑 RTT 在卡顿后读数过时的老问题（上一节试过「近 2 秒最低的样本」，
p2i 变坏，没留），这一轮没修。

### 16. `src/socket.rs`、`src/endpoint.rs`：公开「大的网络变化」（2026-10-09，net-r）

上游只有 `Endpoint::network_change()`：交给网络监视器去判断，网卡没变就什么也不做。回到前台的手机多半还在原来那张网上，可 socket 可能被系统收回、中继连接和对端的路径可能在冻住时死了。上游测试里有现成的 `force_network_change(is_major)`（`#[cfg(test)]`），做的正是要的那一套：在原端口上重绑 UDP、查中继连接、重置 DNS、重测网络、通知 QUIC 去 ping 每条路径、打洞。改为：去掉那两处 `#[cfg(test)]`，加公开的 `Endpoint::network_change_major()` 调它（端点已关就不做）。khor 回前台时用它代替换新端点（`link.rs` 的 `Wake::Resume`）。测：`cargo test -p khor-node --test resume a_resume_keeps_the_ears`。

### 15. `src/socket/transports/relay/actor.rs`：一条中继连接不再卡住所有中继；慢的 pong 不算断（2026-10-09，NET-DIRECT，net-g）

MacLab 10-09 约 11:36 起，经中继什么也发不出去：iPhone 的 Initial 从家中继 aps1 进来（13:07 测得约 57 KB），
一个字节也没回；每分钟一次的拨号从没打开过北京或 pk 的连接；家也再没换过。原因是三处没有时限的等待：

1. 连接 actor 空闲 60 秒正常退出时 `client_sink.close().await`。tokio-websockets 的 close 要读到服务器关流为止，
   回程死了（代理 TUN 把 TCP 一直托着，euc1 那条 5233 进 / 50245 出冻结了 40 多分钟）就永远等下去；
   这期间它不再应答任何消息，收件箱却开着，不会被回收。
2. 中继 actor 给一台没有连接的中继发数据报前，挨个问现有连接「见过这台机器没有」（`HasEndpointRoute`），
   `rx.await` 没有时限：问到上面那条就永远等下去，整个中继 actor 跟着停，所有中继的发送、换家都停了。
3. 某条连接的队列满了，中继 actor 也一直等它腾出位子。

改为：close 最多等 1 秒；每条连接的回答最多等 250 毫秒，不答当作没见过；队列满了最多等 1 秒，之后丢掉这批
数据报（同 UDP 丢包）。

另一处（同一文件）：周期 ping 的 pong 时限从「上一次 pong 的 3 倍，至少 0.5 秒」改为固定 5 秒（上游的最大值）。
经 TUN 到 aps1 的 pong 在 124–976 毫秒之间跳，临时节点 6.5 分钟里被判断开、重拨了 3 次，每次约 4 秒，
期间中继告诉对方这台机器走了。网络变化后的那次快速检查仍按 RTT。

测试：`crates/net/tests/relay_stuck.rs`（一条吞掉 Close、此后两个方向都不再转发的路；之后 alpha 要能拨到第三台
中继上的 gamma，回环中继上的 delta 要能拨到 alpha 的家），打补丁前两项都超时，之后 7–14 毫秒；
`crates/net/tests/relay_jitter.rs`（隔一个 pong 压 700 毫秒），两分钟里打补丁前连了 8 次，之后 1 次。
上游 1.3.0 三处相同，升级时要重新打；中继服务器那一侧（iroh-relay 的 `server/client.rs`）对客户端也用 3 倍 RTT，
khor 自己的中继要改得 vendor iroh-relay，没做。

2026-11-14 feat/net扩展：有实际resident请求、尚未收到对端datagram且当前选Relay时，Node在按路RTT缩放的短等待后，通过`Endpoint::check_relay_connections`提示既有Relay actor检查；不做全endpoint rebind、不重放应用写入。socket仍按本地地址有效性处理，实际TCP actor合并同时到达的检查；未完成的ping只由匹配pong或期限结束，周期ping及并发提示不能替换期限。该入口也可能被后台同步使用，不能由“无新增周期任务”推断能耗不变。

显式检查有两个期限：完全没有Relay消息时，按已有RTT早期判断；有实际接收进展时只放过早期判断，仍必须在首次ping固定的原max_timeout内收到匹配pong。接收进展不能证明双向健康，也不能延长硬期限。connected与pending-send循环共享同一个检查状态，早期判断排在已就绪接收之后；无额外ping/周期timer。正常inbox已经被发送阻塞时的处理、单向故障及长请求后半程仍待专项接受。

准确F3 `7a794274`普通Release原双strict NAT/单Relay夹具：固定3次旧TCP双向静默drop的126读全成功，首有效响应0.873–0.954秒（原产品3次约20.04秒、30读超时）；另固定3次健康124→976ms、无drop的42读全成功且原TCP保持。原2秒/5秒caller、两fixture逐字不变。原F2在健康组3次误断的失败保留。十原生actor合同通过。证据及限制见`docs/net-optimization/RELAY-DEMAND.md`与外部`relay-health-progress-combined-runtime-verification-v1.json`。这不是所有Relay、真机、能耗或完整N4/N5接受；若上游提供同等的有界请求触发健康检查与防误断/期限合同，应以原夹具及成本实测评估退出这份扩展。

### 14. `src/net_report.rs`：完整报告要等不答 QAD 的中继量完（2026-10-09，NET-DIRECT，net-f）

上游的完整报告一拿到两个 IPv4 QAD 回答就停掉其余探测（`have_enough_reports`），不答 QAD 的中继（khor 自己的
三台都是 `quic = None`）的 HTTPS 探测还没回来就被丢下，这一轮报告里没有它，于是永远当不了家。MacLab 10-09：
代理改走 DIRECT 后 n0 的 aps1、euc1 都答 QAD，11:36:41 起家一直是 aps1（经 TUN 握 TLS 0.75–4.1 秒），
北京（18 毫秒）再没被量过；补丁 11 只管增量报告，管不到这里。补丁：完整报告里，只要还有不答 QAD 的中继没有
HTTPS 延迟，就不走两个 QAD 的捷径（仍可按「每台都有 HTTPS」收场）；它的探测都结束了这一轮自然收场。
代价：不答 QAD 的中继若是死的，完整报告要等它的三次探测超时才定家。改动约 15 行。
测试：`crates/net/tests/qad_home.rs`（net-g）。两台答 QAD 的替身（`KHOR_PUBLIC_RELAYS` 的 `url|端口`，
`localhost` 自签证书，每个方向加 100 毫秒）加一台不答 QAD 的明文中继；打补丁前家是替身（报告 210 毫秒就收场，
没有 HTTPS 一项），打补丁后家是明文中继（同样 210 毫秒，它的 HTTPS 1–2 毫秒已在报告里）。注意：上游的捷径看
netwatch 的 `have_v6`，一个 ULA（如 Tailscale 的 fd7a::）就算有 IPv6，这时捷径还要一个 v6 的 QAD 回答。
上游 1.3.0 没有对应改动，升级 iroh 时要重新打。

### 13. `src/socket/remote_map/remote_state.rs`、`src/socket.rs`、`src/endpoint.rs`：转连的路断了，当场关掉自定义路径（2026-10-09，NET-DIRECT）

转发（经第三台 khor 机器）的中间那台冻住时，khor 的转连链路结束了，iroh 却不知道：自定义路径还开着，`RoadOrder`
按它最后的往返（1–2 ms）继续选它，直到路径空闲超时（`PATH_MAX_IDLE_TIMEOUT` 15 s）把它放弃，才回到中继。
三台机器的回环测试（`crates/cli/tests/hop.rs`）量得冻住到回中继 19.9 s；把那条隧道线的静默上限从 15 s 降到 10 s
（`wires::HOP_SILENT`）单独不改这个数，因为线退了路径还在。补丁：`Endpoint::close_carried(EndpointId)` 让该远端的
actor 在每条活着的连接上关掉所有自定义路径（`RemoteStateMessage::CloseCustom`）；noq 当场发出 `Abandoned`，
iroh 随即重新选路。连接只剩这一条路径时 noq 拒绝关，照旧等空闲超时。khor 这边在转发链路结束、且没有新链路顶上时
调用（`carry::let_path_go`：`hop::open_hop` 的守望任务、`serve_hop` 的 `Ask::From`、ssh 转连的 `OneCarry::run`
与 `carry_hosted`）。量得冻住到回中继 12.9–13.2 s（三次）；ssh 会话断掉到回中继 1.8 s（三次，未打补丁时 15.9 s）。
路径关了以后，ssh 重连上来的链路由 `carry::offer_path`（补丁 12）当场在活着的连接上再开。
改动约 35 行。上游没有对应 PR，升级 iroh 时要重新打。

2026-11-10 feat/net 扩展候选：`Endpoint::close_carried_if` 将一个只读共享状态的同步predicate带到同一RemoteStateActor，在实际关闭之前判断；Node的`let_path_go`用最新Links.has保护已经出现的替代载体，既有无条件`close_carried`保持原语义。该predicate不得做IO/阻塞等待，只活到有界消息处理完，不新增task/timer/probe。真实Node旧行为761c4291为24/1红（实际replacement offer后仍Closed），候选需原fixture/完整net-node双OS与net-vendor验证；排队期间出现新carrier、pending Noq开关及ears epoch仍待单独合同，不声称整个代际问题已解决。

### 12. `src/socket/remote_map/remote_state.rs`、`src/endpoint.rs`：连接活着时交来的转连地址当场打开（2026-10-09，NET-DIRECT 第四步）

上游收到新地址（`ResolveRemote`，`connect` 时发）只记下，不在已有连接上开路；补丁 6 只在连接加入时补开自定义
地址。于是一条已经落在中继上的连接，永远用不上之后才搭起来的转发（经第三台机器）。补丁：`ResolveRemote` 里的
自定义地址在每条活着的客户端连接上 `open_path_ensure`（已开的不重复开）；`Endpoint::offer_addrs(EndpointAddr)`
不拨号、只交地址。khor 这边 `hop::compare_roads` 搭起转发后调用它，由 `RoadOrder` 按往返决定走不走。
改动约 25 行。上游没有对应 PR，升级 iroh 时要重新打。

### 11. `src/net_report.rs`：增量报告量不到家中继时不换家（2026-10-09，NET-DIRECT）

增量报告只量握着 QAD 连接的那台中继（`have_enough_reports` 一个 v4 回答就够），`add_report_history_and_set_preferred_relay`
只在这一轮量到的中继里挑，旧家不在这一轮里时也不走「新的要快三分之一才换」的滞回。MacLab 10-09 整夜：完整报告
（5 分钟一次）量到北京、住北京，25 秒后的增量报告只量到 n0 的 aps1、住 aps1（audit/net-d.md §1.4）。补丁：
**增量**报告没有量到当前的家，就留在家里；完整报告照旧可以换（家真死了，下一次完整报告或网络大变时离开）。
函数多一个参数 `full`，上游自己的表测试跟着传 `true`。改动约 15 行。上游没有对应 PR，升级 iroh 时要重新打。

**补一处（2026-10-09，net-h）**：`full` 原先只看 iroh 的 `do_full`，可不是 `do_full` 的报告也可能把每台中继都量一遍：
上一轮什么都没量到时，`ProbePlan::with_last_report` 退回初始计划。中继都不答 QAD 时报告就是这样一轮空、一轮全量交替。
家中继停了以后，全量那一轮只量到别的中继，补丁 11 却当它是增量报告，把死了的家留住，直到 5 分钟一次的完整报告。
`crates/cli/tests/relay_https.rs` 因此常红（TLS 中继停了 60 秒还住在它上面）。改为：`do_full`，或者计划是初始计划
且每个探测都跑完了（没被 QAD 捷径截断），就算量全了。临时打点的一次运行：停后三轮全量报告都只量到明文中继，
家都留在 TLS；改后第一轮全量报告就回到明文中继（35–48 秒）。

### 10. `src/socket/remote_map/remote_state.rs`、`src/socket.rs`：对端看到的地址（2026-10-09，NET-DIRECT）

QUIC 的 observed address 报告（QAD 在中继上用的就是它）iroh 默认两边都关，开了收到也不管
（`NoqPathEvent::ObservedAddr => {}`）。于是一台机器只能从答 QAD 的中继得知自己的公网地址；khor 自己的中继
都不答。补丁：直连路径（`to_transport_tuple(..).is_ip()`）上收到的对端观察地址记进一张共享表
（`PeerObserved`，十分钟有效，最多 8 个，回环和未指定地址不收），`update_direct_addresses` 在本机网卡地址之后
把它们作为 `DirectAddrType::PeerObserved` 加进直接地址——从此作为打洞候选给出去、也随行发布。中继和转连路径
上的观察地址是映射地址，不收。khor 这边 `endpoint::transport()` 打开收发两向的报告；旧版对端不协商、不发，
不受影响。改动约 50 行。验证：回环上看不到（对端看到的是回环，不收），要在机群上看连接记录「本机地址变为」
里多出的地址。上游没有对应 PR，升级 iroh 时要重新打。

### 9. `src/socket/remote_map/remote_state.rs`、`src/address_lookup.rs`：打洞只给发布过滤留下的地址（2026-10-09，NET-DIRECT）

上游 `local_candidates` 把所有直接地址都作为打洞候选（QNT ADD_ADDRESS）给对端，代理 TUN 的 198.18.0.1 也在
里面（MacLab 10-09 的连接记录）。补丁：先过端点的发布过滤（`AddressLookupServices::filtered`，就是 khor
`endpoint::unroutable` 那条），发布什么就给什么。改动约 25 行。测试：`crates/net/src/endpoint.rs` 的
`a_punching_round_offers_no_unroutable_address`（读补丁 8 的打洞轮；在有 198.18.0.1 的机器上对照：iroh 的
直接地址里有，打洞候选里没有）。上游没有对应 PR，升级 iroh 时要重新打。

### 8. `src/socket/remote_map/remote_state.rs`、`src/socket/remote_map.rs`、`src/socket.rs`、`src/endpoint.rs`：每一轮打洞（2026-10-09，NET-DIRECT 第一步）

`Endpoint::holepunch_rounds()`：iroh 每开始一轮打洞（`do_holepunching`，连接加入、任一方候选变化、
没有 10 ms 以内的直连时每 60 秒）就广播一个 `HolepunchRound`：对谁、本机给出的候选、对方给出的候选。
上游只有 tracing 事件 `iroh::_events::qnt::init`。khor 这边 `journal::watch_ears` 订阅，写成连接记录的
「打洞」一行（候选变了就写，不变时最多十分钟一行，带上这期间的轮数），`--report` 据此说「为什么不是直连」。
打没打通看随后有没有直连路径打开（`Connection::path_events`，`roads::follow`）。改动约 30 行：
`RemoteMap::new` 多一个参数（broadcast 发送端），`RemoteStateActor` 存一份。
测试：`crates/node/src/journal.rs` 的 `rounds_are_written_on_change_and_then_sparingly`（只测节流）。
上游没有对应 PR，升级 iroh 时要重新打。

### 7. `src/endpoint.rs`：带来源的直接地址（2026-10-08，NET-STABLE 第二步）

`Endpoint::watch_direct_addrs()`：把 `watch_addr` 里那组直接地址连同来源（`DirectAddr::typ`：
本机网卡 / QAD 学到的公网 / 端口映射）给出来。iroh 1.2 导出了 `DirectAddr` 和 `DirectAddrType`，
却没有公开的取法，`watch_addr` 只给不带来源的地址。khor 要的是「本机网卡地址变了」——iroh 自己的
网络监视看到换网时重绑、重探，khor 不知道，退避照等；QAD 学到的公网地址会自己变，不能算换网。
改动 8 行，只是把内部的 `ip_addrs()` 露出来。khor 这边 `journal::watch_ears` 订阅，稳定 1 秒后
告诉常驻（`Change::Addrs`）。测试：`crates/node/src/journal.rs` 的
`interface_addresses_are_told_once_they_hold_still`（只测去抖；回环上没法真的换网卡）。
上游没有对应 PR，升级 iroh 时要重新打。

### 6. `src/socket/remote_map/remote_state.rs`：中继先答时也打开自定义地址（2026-10-08，ROAD-3 轮）

`handle_msg_add_connection` 里，客户端在连接建好后把已知的中继地址补开成额外路径——上游只在
路径 0 不是中继时这么做，而且只补中继。khor 的自定义传输（`khor_net::carry`，ssh 里的 khor
和经第三台机器的转发）跟中继一起拨号：中继先答时路径 0 是中继，自定义地址从此不会被打开，
「先转发、后中继」（`carry::RoadOrder` 的开局三秒）就落空。补丁：不论路径 0 是什么，已知的
自定义地址都补开；中继仍只在路径 0 不是中继时补。改动约 10 行。
测试：`crates/net/tests/carry.rs` 的
`the_carried_road_is_opened_when_the_relay_answers_first_and_the_faster_carries`（回环中继加一条每字节慢
120 ms 的链路：中继先答，补丁前这条链路从没被打开，补丁后它被打开；2026-10-09 起选路按往返挑，
这条慢链路打开后由更快的中继接着走）。上游没有对应 PR，升级 iroh 时要重新打。

### 5. `src/net_report.rs`：中继的接班者（2026-10-04，NET-7 轮）

`NetReportConfig` 多一个 `successors: Vec<(RelayUrl, RelayUrl)>`（旧，新）；`Client` 存下它；
`add_report_history_and_set_preferred_relay` 在补丁 3 之后再加一段：家要是某个「旧」，而这一轮
报告量到了它的「新」，家就换成「新」。为的是北京那台换 HTTPS：同一台机器上两个中继进程（明文
3340、TLS 3341），延迟差不多，iroh 自己按延迟挑会随机落在其中一个；补丁让量得到 TLS 那台时
一律住它，量不到时照旧住明文那台。放在补丁 3 后面，所以首选了旧地址的人也跟着换到新的。改动
约 15 行。khor 这边 `endpoint::successors` 传进来（`KHOR_SUCCESSORS` 加 `KHOR_RELAY_SUCCESSOR`，
两台都在中继表里才算）。

**多久换过去**：iroh 只在「完整报告」里量中继——增量报告的探测计划是空的
（`probes.rs` 的 `with_last_report`，带着上游自己的 `// TODO: is this good?`），于是下一轮看到
上一轮一台都没量到，又从头量一遍。所以实际是每两轮周期报告（20–26 秒一轮）量一次：回环上量到
TLS 那台启动后 44–50 秒住过去、它停了 40–46 秒住回明文那台；绑新端点（启动、手机回前台）时
当场就量。测试：`crates/cli/tests/relay_https.rs` 的
`the_tls_relay_takes_over_renews_in_place_and_gives_way_when_it_stops`。上游没有对应 PR，
升级 iroh 时要重新打。

**补一处（2026-10-09，net-h）**：「量到了接班者」只认它的 HTTPS 回答，QAD 的回答不算。QAD 走 UDP，家中继却要靠
TCP 来连；从 MacLab 到北京 TLS 中继的 TCP 在路上被重置（audit/pk-3341-dump.txt），UDP 却可能通，只凭 QAD 回答就把家
搬过去，会搬到一台连不上的中继上。还没编译、没测（`crates/cli/tests/relay_qad.rs`，`#[ignore]`）。

### 4. `src/socket/transports/relay/actor.rs`、`src/socket.rs`、`src/endpoint.rs`：中继说谁走了（2026-10-04，NET-5 轮）

中继服务器在一个端点断开时，给之前收过它包的端点发 `EndpointGone`。iroh 1.2 收到只是从内部的
`endpoints_present` 里删掉，不告诉任何连接，也不告诉应用。补丁：`HomeRelayWatch` 多一个
`departed: broadcast::Sender<EndpointId>`，收到 `EndpointGone` 时发出去；`Socket` 留一份，
`Endpoint::relay_departures()` 给出订阅。改动约 30 行。khor 这边 `link::Node::set_ears` 订阅，
`wires::Wires::departed` 把走中继的那条线当场收回、记下「刚走」，此后拨它只给短时间
（3 秒或 6 个往返），直到又听到它。
测试：`crates/node/tests/departed.rs`（两台只能走回环中继，一台关耳朵，另一台的状态词立刻离开
「中继」，问它 3 秒内失败；不接这个补丁时要等 10 秒的拨号）；`crates/cli/tests/diagnose.rs`
（进程被杀：状态词 14 秒 → 4 毫秒，第一次问 14 秒 → 3 秒）。上游没有对应 PR，升级 iroh 时要重新打。

### 3. `src/net_report.rs`：首选中继（2026-10-03，NET-2 轮，负责人批）

`NetReportConfig` 多一个 `preferred_relay: Option<RelayUrl>`；`Client` 存下它；
`add_report_history_and_set_preferred_relay` 最后加一段：这一轮报告量到了它（`relay_latency`
里有它，即答上了），就把 `preferred_relay` 定成它——家中继于是住在它上面。量不到时照 iroh 原来
的选法（最近 5 分钟延迟最低、新的要快过旧的三分之一才换）。iroh 1.2 没有选家中继的旋钮，只有
这个函数决定家，所以只能补在这里；改动 15 行。khor 这边经 `endpoint::bind_preferring` 传进来，
值来自 `.khor/relays.json` 的 `preferred`（设置 → 网络 → 中继 → 首选中继）。
测试：`crates/cli/tests/slow_relay.rs` 的 `a_preferred_relay_is_home_while_it_answers`（两台本机
回环中继，偏好远的那台就住远的，远的那台没了就回到近的）。上游没有对应 PR，升级 iroh 时要重新打。

### 2. `src/util.rs`：探测的 HTTP 客户端不读系统代理（2026-10-01，负责人定）

`reqwest_client_builder` 里加一行 `.no_proxy()`。iroh 的 net_report 探测（量各中继延迟、
强制门户）和 pkarr 发布用的是 reqwest；khor 工作区为额度页给 reqwest 开了 `system-proxy`，
cargo 的 feature 合并后这些探测也读 macOS 系统设置里的代理，而 hyper-util 读系统代理时不看
例外表。真正的中继连接只读环境变量（`proxy_from_env`），直连是 UDP 根本不经代理——三条路
三种规矩。`no_proxy()` 只关掉「自动读系统代理」，调用方在有 `proxy_url`（来自环境变量）时
仍 `.proxy(..)` 加上去，所以探测从此和连中继同一条规矩：只认环境变量。
账在 docs/NET.md「代理」一节；这条上游没有对应 PR，升级 iroh 时要重新打。

### 1. `src/socket/remote_map/remote_state.rs`

2026-11-09复核：#4522已于9月29日关闭、未合并；1.4.1已有简化contains去重。
下面OPEN为原始历史，不能当作当前状态。固定来源与逐块退出边界见
[UPSTREAM.md](../../docs/net-optimization/UPSTREAM.md)；本轮尚未升级依赖或证明全部补丁可退出。

- 用的是上游 PR **#4522**「fix(iroh): bound and deduplicate pending_open_paths」的改法
  （PR 头提交 `eaca51d4c003f8a9f3764bd7ff7eca1a8dbdc309`，基于 main；截至 2026-09-21 仍是 OPEN，未合并）。
  它修的是 issue **#4390** 和 **#4509**（两条都还 OPEN）。
- PR 的十个 hunk 里，九个原样打上；第一个（新增 `OPEN_PATH_RETRY_DELAY` 与 `MAX_PENDING_OPEN_PATHS`
  两个常量）因为 1.2.0 在那里少了 `DATAGRAM_SEND_TIMEOUT` 几行上下文而没对上，内容一字不改地手工放在
  `ACTOR_MAX_IDLE_TIMEOUT` 之后。最后那个 hunk（测试模块）打在了文件末尾。
- 没有用 issue #4509 里那段更短的 hunk（只去重+限长）：#4522 除了去重和限长，还让报了
  `MaxPathIdReached` 的连接不再重试（path id 只增不减，那条连接再也开不了新 path），这正是我们遇到的那种情况。
- 想看补丁本身：`git show` 那个只动了 `vendor/iroh/src/socket/remote_map/remote_state.rs` 的提交，
  它的上一个提交就是原样拷贝。
- **10-21 修订（net-flap）：`MaxPathIdReached` 不是永久的**。#4522 把它当永久（「path id 只增不减」），一条连接撞一次就
  再也不开路。可 noq 收到对方的 PATH_ABANDON 时会把自己的上限加一、发 MAX_PATH_ID（noq-proto `connection/mod.rs` 的
  `Frame::PathAbandon` 分支 `set_max_path_id(self.local_max_path_id.saturating_add(1u8))`；收到 MAX_PATH_ID 时
  `remote_max_path_id` 跟着涨），noq 自己的打洞也把它当暂时（同文件 `PathError::MaxPathIdReached | PathError::RemoteCidsExhausted`
  「Temporary error, put back」）。候选多的一对机器一轮打洞就能把一条连接的 path id 占满，之后补丁 12 交来的转连、补丁 13 的
  ssh 重连、补丁 6 的自定义地址都静默跳过，直到连接关掉。改为：撞上了，地址记在这条连接上（`PathIds`，至多 64 个），这条连接
  上一条路径被放弃或丢弃（`PathEvent::Abandoned`、`Discarded`）时清掉标记，记着的地址交回 `pending_open_paths` 重试；不当场
  重试（当场只会再撞上），所以也不回到 #4390 的空转。测试：`path_ids_run_out_until_a_path_is_abandoned`（iroh）、
  noq-proto 的 `max_path_id_reached_lasts_until_a_path_is_abandoned`（三条路径到顶，关一条，第四条开得出来；改前改后都绿——
  它证的是前提：上限会涨）。

## 为什么要 vendor

bug：同一个 remote 有 ≥2 条连接、都开不出新 path 时（`RemoteCidsExhausted` / `MaxPathIdReached`），
每条连接都把同一个地址重新塞回 `pending_open_paths`，每 333ms 一轮，队列按「连接数的 n 次方」增长。
RemoteStateActor 从此在 `open_path_on_all_conns` 里空转，排在它后面的消息（包括 `Endpoint::remote_info`）
一直等不到回复，内存一路翻倍。

在 khor 里的样子（docs/handoff/001.md 里 smoke §25 那一节有全部读数）：beta 对 alpha 同时有同步连接和
隧道连接，两边在同一台机器上，互相公布的候选地址里只有回环走得通（本机发往自己局域网地址、tailnet 地址的
UDP 到不了，连系统自带的 `/usr/bin/nc` 也一样），开往其余候选的 path 一直失败。从这里怎么一步步走到
`MaxPathIdReached` 没有逐帧追，读数只有结果：`sample` 看到 actor 所在的 worker 线程 263/266、389/393、
308/310 个样本都在 `open_path_on_all_conns` 里；bridge 的 `devices` 请求（→ serve 的 `Op::Hops` →
`remote_info`）从几十毫秒涨到 52950ms，然后再也不回来。

最初vendor时尚无可用上游修复，因而本地打补丁。2026-11-09已有1.4.1简化去重；整组补丁是否可退出仍需逐项验证，不能沿用旧结论。

## 怎么验

补丁自带三条单元测试（`pending_open_paths_*`）。这个目录不是 workspace 成员，`cargo test --workspace`
不会跑它们；要跑：

```sh
cargo test --manifest-path vendor/iroh/Cargo.toml --lib pending_open_paths
```

它会用上游自己的 `Cargo.lock` 和 dev-dependencies（第一次要联网拉）。
**在 git worktree 里跑会报「believes it's in a workspace」**：worktree 嵌在主仓目录下，cargo
往上找到了主仓的 workspace。那就把这个目录拷到仓外再跑。

打补丁前（1.2.0 原样 + 同样的测试，排队逻辑按原样抽成一个函数）红句：

```
queue grew to 256 entries after 4 ticks
```

## 什么时候删

这份目录已包含多项独立补丁，补丁1出现上游等价实现不再构成删除整树的条件。
按[上游核查](../../docs/net-optimization/UPSTREAM.md)逐条验证目标行为、接口与覆盖后，
可在隔离升级候选中删除对应实现，并跑相同协议的恢复/稳定性/转发与原生回归。
只有全部剩余本地行为都有已验证替代时，才删除vendor子树和相应Cargo patch/exclude配置；
任何未退出补丁都保留原因、覆盖、上游状态和退出条件。原始来源与失败记录不能随升级丢失。
