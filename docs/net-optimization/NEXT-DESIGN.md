# 下一批网络设计与证据计划

2026-11-09 · 本文主体保留基于33202c22的历史设计与源码事实，不能当当前源码。N2期限/取消已部分实现验证、N3 Noq模型已执行；最新证据以STATE/REQUIREMENTS/REPORT为准。
活动目标/执行器状态只查[STATE](STATE.md)，完整范围见[PLAN](PLAN.md)，验收协议见[ACCEPTANCE](ACCEPTANCE.md)。
本次八个源文件blob收据在`evidence/net/design-source-audit-20261109/inputs.json`；下面区分源码事实、设计提议和待测项。

## N1当前边界

36普通p1s已0/0移动，但p3两轮1.1/1.2秒超原1秒；f4s一轮回carried后又退Relay。
它们是独立未解决项，不由单元或日志模式绿覆盖。
37现已实施并取得Mac36路径/438Noq/9home绿；Turingrecovery6/6，corrected hop仍8/9，有独立f4i回退与91.4秒慢路滞留。下述提议已实现的部分见STATE，不能重复作为待开发项。

下一修正的提议：当前路任一live copy在故障开始后且最近一个原观测tick内收到包，才启用36的双路探测与决定后备用ACK要求。
没有这条提示时，故障后已获得的备用往返可用来快速离开。收包只触发验证，不能证明正向发送可达。
不新增计时器、不改变PTO/收益/正常两样本、4秒兜底或应用重试。

远端顺序：回归实际红→最小修复单元绿→原recovery三轮→原stalls与hop→最终准确源完整矩阵。
f4s回退先在现有三端诊断中关联回退时Silence/Reading/副本/实际Go；测试会在断腿解除1.5秒后恢复Relay正常延迟，不能把整个30秒观察期当成Relay一直有5秒RTT。
恢复后仍要求30秒稳定与1500ms门，历史红不回填。若要修正实验解释，必须先明确拓扑和实际完整路径，不因某版不过删门。

## N2：总期限与取消的实施设计

### 源码事实

- `ipc::Op::deadline`按操作分30秒、300秒、3600秒等档；`run_handoff_within`当前另起该档减500ms的相对timeout。
- `ask_on`的固定28秒by现在只约束早拨和读重拨；普通首拨没传，交换又使用`Request::budget + RTT余量`。
- Request有15/25/30/60/280/3600秒不同工作预算；因此把整个ask强制28秒会破坏长操作，不能当成预算贯通的正确实现。
- `line_to_inner`等拨号锁、`hop_ahead`都先于受by约束的dial。hop现有3秒是两个middle合计预算，不能误读为每个各3秒。
- `dial_with`最多2个JoinSet任务。首试包含已知IP/Relay/Custom；第二identity尝试的1500ms不等于“中继延迟1500ms”。
- 早数据任务在握手前由Aborts拥有，Accepted后take并裸await；调用者此时取消会丢掉JoinHandle所有者。
- `is_reading`不全是纯读取：含按token去重的OpenOnce/Drop等既有准入。`rides_early`是更窄白名单，二者不得合并。
- RetireUnheard原本覆盖取消时真正无应答的坏wire，不能为了取消安全一律禁用；操作到期本身也不能证明wire坏。

### 提议的边界

在操作入口建立一次本进程单调时钟截止点，沿锁等待→hop准备→有界拨号→握手→交换→允许的一次读重试显式传递。
每段截止取操作剩余预算与该段现有安全上限中更早者；不得在第二次调用重新获得完整时长。
保留请求本身的工作预算和现有慢/传输IPC档，不新设统一28秒总上限。
现有无前台操作上下文的后台入口自己建立一次有界期限，不继承已经结束的前台请求。

跨进程不能序列化Rust Instant。首批在resident操作入口确定本地截止点并向下贯通；若以后扩展IPC预算字段，应传剩余时长并保留版本兼容及调用方原外层限制，不能伪称跨进程共享同一个Instant。
不通过一个包住所有工作的timeout来代替分类：到期要能正常返回操作期限错误并解除该次guard，不将短操作超时写成peer失败退避。

| 阶段 | 到期/取消的所有权 | 保留的共享状态 |
| --- | --- | --- |
| 等同peer/proto拨号锁 | 只释放本等待者；到期后不进入hop/dial | 正在服务别人的锁持有者、已有wire及peer失败次数 |
| hop准备 | 总期限与原3秒中较早者；不再尝试下一个middle | 已成功建立且可共享的健康middle wire；调用方到期不计作middle/peer故障 |
| 两个拨号候选 | 同一截止点；胜出后中止输者，测试临时连接最终释放 | 已adopt的胜者；不竞速应用写请求 |
| 0-RTT Accepted后等待回答 | Aborts所有者持续到回答结束；到期/取消中止本回答task与流 | 已adopt且仍健康的连接；原rides_early/is_reading门 |
| 普通交换/一次读重试 | 共享截止点；到期不启动新stream/拨号；保持原重试次数 | 其他正常请求与wire；真全路无应答仍由原独立活性证据退役 |

内部需要区分“调用方期限耗尽”和“拨号自身达到失败上限”，再分别映射用户文字、journal和退避。
禁止把前者伪装成DialFailure::TimedOut后执行Wires::failed；具体错误类型在实现时按调用面最小调整。
到期与成功同时ready时，测试明确截止语义；不承诺非实时runtime在墙钟上绝不晚醒，记录实际调度滞后。

首批回归应调用真实line_to/ask_early/exchange边界，用barrier控制等待、握手接受与回答，不只测一个remaining函数或Aborts包装器。
至少证明：锁到期零新拨号、另一个调用照常完成；早数据Accepted后取消流确实关闭但共享wire仍可请求；一次读重试不能重置期限；token写操作只生效一次。
对通用计时使用可控时钟，若现有std/tokio时钟混用不能暂停，应先统一内部边界或用事件同步，不能把本机sleep误差当成网络结论。

### 缓存、退避和恢复合并

先复用Wires/roadbook已有近期往返与失效机制，测地址/端口变化和冷启动，再决定新增缓存；身份键稳定不代表旧地址稳定。
保持现有最多两个拨号候选，先测失败提示地址对首份响应的影响，不直接调1500ms或再加并发。

随机扰动只能在一次实际失败确定时选取一次，journal和retry_at共用同一结果；不能在next_backoff这个预测读取函数中每读一次重新随机。
推荐让失败提交返回实际次数/延迟供journal记录，或者使用该次失败的唯一预留记录；先审计同peer/proto锁与其他失败更新的竞态再定接口。
睡眠/relay离开已有静默规则、前台不受后台退避拒绝的语义保留；扰动范围需设备同时恢复基线后冻结。

## N3：混合负载设计先定位四处排队

| 当前源码事实 | 要测的因果对照 | 设计决策的依据 |
| --- | --- | --- |
| Node::handle每连接32个处理席位，accept_bi后先acquire再读请求 | 长请求占满席位时，短控制/目录请求在哪一段等待 | 先分清准入排队与网络排队；仅加QUIC stream priority不能解决席位等待 |
| content Window/Record另有32个响应预算，并会提前放掉普通seat | 单独测内容读取及其他bulk，记录permit拒绝/持有时长与内存 | 不假定所有bulk都共享一种席位；不盲目再加同用途Semaphore |
| Custom承载1024帧队列、256KiB BufWriter，满则丢；write_frames尽量排空后flush | SSH/hop外层慢读时的队列深度、丢包、flush间隔与交互尾部 | 测外层按序阻塞与内部流优先级的边界，不能把1024帧当固定字节上限 |
| 现congestion测量有固定丢包seed、bulk+250ms echo、priority与idle→burst | 先按当前精确源复跑，再加真实Node/SSH/hop端到端请求 | 已接入net-congestion三轮profile和v2完整记录候选，尚未编译/执行，不是N3通过证据 |

第一轮比较固定相同路径/带宽/队列/损失、相同工作量：无bulk、原共享连接、共享连接调度/背压候选、有限分连接。
每组记录首份内容、echo所有样本/失败/取消、批量字节与完成时间、队列/permit/重传/总开销；报告p50/p95/max和样本量，样本不足不报p99。
先顺序三轮固定seed定位，再多seed评价稳定性；不把同时编译造成的争用归因网络策略。

决定优先级时先解决实际阻塞层；高优先级只能影响尚未发送的字节。分连接会改变拥塞公平与每连接32席位总量，不能把增加总资源伪装为调度胜出。
候选需要有界总并发/排队字节、取消释放、控制及时性与批量持续进展；具体权重、块大小和保留席位在baseline后冻结。
后台成本另测前台/闲置/恢复、1/3/5台个人设备，记录CPU/字节/连接/探测；手机能耗只由真实手机证据确认。

## N4：现有中继与登记的真实故障域

以下是默认配置源码，不是在线探测结果；用户自定义Relay列表须另按实际配置检查。

| 入口/服务 | 源码所属故障域 | 判断 |
| --- | --- | --- |
| 北京HTTP与北京TLS入口 | 同一机器；不同进程/端口 | 两个URL不能当两台独立中继 |
| 香港TLS入口 | 另一机器 | 可增加地域选择，但不能假定每一端都可达 |
| 默认自家pkarr登记 | 北京HTTP所配置服务；香港入口未配置pkarr | 关闭n0兜底时，默认自家登记集中北京；已有地址/已有会话可用性与冷启动发现分开测试 |
| n0预设（用户开启时） | 另一套发布/查询与备用Relay | 是受用户配置约束的兜底，不能为过门绕开“只使用自己的中继” |

已存在：所有配置登记并行发布/查询、home变化触发Wake::Home、home_first发布、同机HTTP→TLS successor、home dwell。
因此不重复新建发现/换家机制；先实测它们在整机故障、单端不可达、旧地址/重启与冷启动中是否闭环。
显式公共DNS resolver也已配置，源码承认只允许内网DNS的出口可能失败；属于受限网络场景，不能由多个公网DNS地址推定已解决。

下一实验以A/B双端为观察点，逐一切断北京整机、香港整机、A单侧出口、登记服务、仅数据面转发；区分TLS/HTTP可建与真实加密请求可完成。
每种故障同时测暖会话和清缓存冷启动，记录每端home/发布位置/查询来源/完整A→Relay→B时延/首次有效响应。
“接入第二个Relay”只有在两端能互通且发现不依赖同一个失效域时才构成替代路径；单端到Relay延迟排名不构成证明。

当前设计倾向：先让登记与Relay具备独立失效域并验证客户端实际发现/使用，再按双端数据选地域/线路。
是否给香港加独立登记、增加第三地域或第二运营商、同时维持第二家，都仍是待实证选项；不以此文授权采购或部署。
容量按实际同时中继会话/方向流量/每端服务字节、CPU/FD与排队水位建模；相同源三轮压测和真网测量后再给机器规格及余量，不编造QPS或可用性承诺。

## 验收和交接

U9已恢复Turing Linux主通道，账单不再阻断当前Linux验证；PLAN范围未变。本文历史N2源码事实需按STATE对照，已实施并部分验证；N3真实Node与N4仍待实测。
恢复后仍先完成N1必要红绿与普通复验；可并行安排独立N3 baseline，但root不派子智能体，也不让性能测量与构建争用同一执行器。
每批绑定产品tree/control/原始证据；适当短回归通过后再最终全矩阵、Mac/手机/多机与可信Quality。无合并/发布授权。


## 执行器接续

Mac便携入口已经实际执行回归，标准Linux托管lane仍因GitHub账单被拒绝。37候选当前run只查STATE，不能用Mac单位结果替代命名空间SLO。
同时只读采集Mac Hypervisor/已装工具/内存/空闲盘信息，评估在既有macserver内使用独立Linux开发VM；这是执行器候选，不是已部署或已证明可用。
[Lima VZ官方说明](https://lima-vm.io/docs/config/vmtype/vz/)支持基于Virtualization.framework的同架构Linux guest；仍须核实际宿主能力和资源。
若具备条件，先在共享build guard与现有空间门内做隔离Linux namespace/netem/nft能力探针，再考虑矩阵。VM状态/缓存归khor-builder独立目录，源/driver只读挂载、报告目录单独写，不挂个人home或传个人密钥/令牌，不改宿主网络/空间策略、不采购新机器。
基线与候选必须在同一后端/资源/协议重跑，不跨GHA Linux、Mac loopback和VM混算性能。如果无可用Linux后端，则该验收仍需外部执行条件恢复；不通过猜阈值完成计划。


## 2026-11-09执行器恢复设计候选（尚未安装/启动）

Mac实际host-capabilities见42555870179：VZ=1，16GiB、10CPU、约76.2GB空闲，未安装Lima/QEMU/容器工具。可在既有共享guard内用项目专属Lima VZ Linux，避免依赖被账单拒绝的托管prepare。此节为具体设计候选，不是已运行证据。

- 锁定Lima v2.3.0官方Darwin-arm64资产717479690，50317981字节，SHA256 b7af444bd8f91d3c307477e562f2cf3bd7595d62b9429185e4fa841563578cfc；官方release元数据发布日期2026-11-06。工具仅装khor-builder/netlab项目子目录，不用brew全局安装/不改launchd。
- Ubuntu24.04 arm64固定release-20261017官方cloud image，SHA256 50e24e1fb79e89116e939d90aa7738f7af48c442fa1bc2510131e0b39c50348e；不退回无digest的latest。2vCPU/6GiB/30GiB磁盘为候选上限，以42573745492实际50/40GiB空间门和单槽为准，旧35/20不再适用且不改installed policy。先证明namespace/veth/netem/nft/conntrack能力，再编译。
- 显式无host home挂载/无个人公钥载入/无agent或X11转发/无任意端口自动转发；只拷本run已校验source.git、可信driver与返回报告，guest只用项目生成控制key，不携GitHub token/个人SSH key。guest Cargo/registry缓存独立于Mac和可信发行。
- VM生命周期必须由guard持有。官方v2.3.0 start.go及pkg/instance/start.go确认--foreground会以前台SysProcAttr执行hostagent；不能用脱离guard的后台默认start。实现前核foreground/信号关闭链路，trap正常退出/取消停止该实例，最终报告确认不残留；不影响他人VM/worker。
- 首次先工具/镜像checksum+启动/停止+能力探针，不称产品通过；候选与6485基线使用同一VM配置/后端重新测，不能把旧x86托管结果与新arm64差值当策略收益。

依据：[Lima安装](https://lima-vm.io/docs/installation/)、[VZ](https://lima-vm.io/docs/config/vmtype/vz/)、[默认home挂载及禁用](https://lima-vm.io/docs/usage/)、[固定2.3.0 Ubuntu24.04模板](https://github.com/lima-vm/lima/blob/v2.3.0/templates/_images/ubuntu-24.04.yaml)、[前台hostagent实现](https://github.com/lima-vm/lima/blob/v2.3.0/pkg/instance/start.go)。


N3在首个真实测量前新增模型校准审查：congestion.rs双向Lane共用单个按入队FIFO sleep_until的发送任务，存在跨方向队头阻塞，不能代表独立全双工调度。ACCEPTANCE已在改派发器前冻结20ms/1000Bps/两个500B包+反向1B的因果回归及模型版本要求；尚未写测试/修模型或取得运行证据。v2记录器与该物理模型问题分开，不能凭记录完整认定模型可信。


N3模型校准42559476637已实际取红（类型笔误轮42559155803仅编译失败，不能混算），v2记录器已编译但零计时轮。候选独立方向FIFO保留各Lane几何/seed，显式independent_fifo_v2；schema仍v2，所有采样记录带model。产品CC不动，下一准确源先校准再三轮全量测量，不能跨物理模型算产品改进。


## N2接续核查：IPC边界与取消（2026-11-09）

准确源hash/摘录存evidence/net/deadline-continuation-20261109/inputs.json：ipc::exchange明确write_all后shutdown发送端，再read_to_end回复；Node::handle_handoff读取请求至EOF后才run_handoff_within。正常请求本来就半关闭，不能把服务端read EOF当用户取消，否则正常操作全部被撤销。要传播调用方取消，先定义和验证独立协议/所有权信号；不得静默改变半关闭协议或重发已发送操作。

客户端call_refreshing现以Op::deadline包住连接/刷新/交换，serve run_handoff_within在请求完整解码后重新取该档位-.5s。只有拒绝TCP连接可刷新重试，连接后一律不因失答/写失败重放。后续贯通需明确caller/serve各自起点和剩余预算，而不是在每层重新开始同一duration；长档仍保留。

RetireUnheard已经同时要求armed、heard_count自请求开始不变、elapsed>=Patience::of(conn).ack，短取消不构成退役证据；不是“所有取消都退役”。后续普通交换的deadline/取消测试应同时证明短取消保留wire/其他并发请求、真实足够久全无收包仍退役。不要以caller到期不是peer失败为由移除F7保护。此核查仅静态，无新端到端运行或期限总链路绿。


N3模型校准与v2首批三轮已由42560015454实际通过：704全量请求0失败/超时，摘要与raw逐项一致。当前固定Noq模型的priority效果不一致，先以REPORT基线定位真实Node32席位/content预算和外层流排队；不直接上产品CC/分连接或新权重。N2接续继续遵守上文半关闭/取消/RetireUnheard合同。


## N2本轮实现选择：操作期限载体与真实ask回归

为避免在百余操作分支重复生成duration，resident入口用任务局部只承载一次建立的不可变Tokio Instant；网络阶段仍显式传递绝对by，并与自身上限取早。它不是Node全局可变状态；并发操作互不覆盖。跨spawn不自动继承：属于同一前台操作的子任务须显式带by，独立后台工作另立预算，不能继承已结束的前台上下文。IPC半关闭与外层Op档不变，不声称跨进程序列化Instant。

先仅添加deadline载体（current/scope，嵌套不延长父期限）和真实Node/Endpoint持锁的ask_on回归；网络消费者/Op入口暂不接入，原28秒行为原样，以取得实际红。测试绑定后才暂停Tokio时间：40ms操作/推进50ms必须完成，300秒操作/推进29秒必须仍可等锁；用5ms观察窗口驱动ready任务后清理。Tokio test-util仅dev feature，无新crate/本机构建。测试覆盖载体到真实ask边界，不把载体存在等于全链路完成。红后再接入口和各阶段，长工作预算与七类合同保留。


本轮scope双回归已由42562932053实际取红。候选连接Op入口的原档-.5s截止点、锁/hop/拨号/普通交换重试；网络阶段显式取早，拨号保留独立安全上限并区分CallerExpired。独立ask无Op时以Request自身工作预算加现有dial25s/hop3s/最大road120s组成外层上限，各阶段自己的上限仍可先结束；原套IPC档位提议经复核撤回，以免压缩工作加路程预算。实际Op保留其原档截止点。更新拨号另保留原10秒上限，不把它误归为CallerExpired。退役仍凭原RetireUnheard证据；IPC半关闭/准入未改。该候选尚未编译/验证，Accepted后回答所有权故意保持原状，下一独立真实0RTT测试先取红后修，前台spawn/后台入口继承审计继续。

## N2跨spawn所有权初审（本轮静态，未作通过证明）

图谱先查run_handoff_within/ask_early；doctor::any_relay_answers不在当前图谱，随后定向源码读取。此次只覆盖link.rs显式spawn及配对探针，不代表全仓异步任务审计。

| 任务 | 目前归属与接续 |
| --- | --- |
| ask_early回答 | 属于正在等待的前台ask；握手Accepted后原take裸handle会丢失abort owner，真实回归先取红。回答显式沿用by，不依赖task-local自动继承 |
| dial_with两候选 | 已是JoinSet，父未来退出会abort输者；候选显式by、延迟候选在开始connect前检查；尚需真实资源释放与不额外拨号的验证 |
| pair_with中继探针 | 父任务手动abort只覆盖正常分支；外层取消会drop裸handle。any_relay_answers还创建Vec裸JoinHandle，成功/取消也可能留下其它HTTP探针。需验证并贯穿取消所有权，不能只包最外层就称解决 |
| reach/code/borrow代理 | 已建立的常驻listener归Node映射/HostedBorrow；前台操作结束后应继续服务，不继承已结束by。创建阶段/中途取消的资源释放仍需测 |
| nudge/Pump/Visit/配对后visit | 独立同步工作，现有频率/合并机制不改；不自动继承前台by。后台多阶段的整体预算与风暴测试仍缺 |
| drop_here/pending_takes | 持久任务/传输job独立于第一次查询；不能因前台完成就取消。run_drop/run_take整体预算/取消边界继续核查，不能将每次ask的新预算当完整job上限 |
| serve_tunnel/handle/serve_updates | 远端响应任务归连接/服务器生命周期，不继承本机前台Op；应保留各自服务预算、信任与0RTT准入 |

上述表把“要随调用停止”与“有意常驻”按已有产品语义确认，没有新增全局任务deadline或扩大请求重放白名单。

42567527752已取得Accepted回答取消/期限双行为红，且同wire独立请求/请求数/peer控制全部通过。最小候选以Aborts作为Accepted返回值保持任务所有者，adopt后的await显式被原by包住；该任务只做流IO、不再发起Node操作，因此由父等待的by和abort owner覆盖，毋须假设task-local继承。父未来在握手前、adopt前或回答期间被drop均保有abort owner；CallerExpired不重放/不记peer失败，已adopt健康conn保留。42567946141七测试与既有子集已实际绿，仍不能收N2全包。

## 配对探针：三红后的最小所有权候选

42570772550已实际证明直接取消、配对外层取消均留下HTTP请求；第二URL已204、首URL仍等待时整体也不及时返回。旧11预算/流控制全绿。候选用doctor JoinSet按完成取首成功，并在赢家/父drop时取消输者；pair_with再持有Aborts，闭合两层所有权，不能只修一层。诊断探针不做Node操作，因此由owner生命周期覆盖，不依赖task-local继承。原并发量、2.6秒单HTTP上限、proxy策略、全部失败/空false及pair超时措辞语义保留；不把HTTP可达当真实中继数据面性能。42571393016十四专项及既有子集已实际绿；ears_heard亦直接await该helper，boolean映射保留，未据此收echo故障场景。


## Linux执行器：镜像工具依赖的固定源审查

Lima v2.3.0 Git树800a96b14e96b5d2ceb20ab2602147c1e79224ea，9份源码已与tree逐blob/SHA256核对，保存在evidence/net/linux-vm-source-audit/verified-source-manifest.json；本机仅下载/读源，没有执行Lima/Go或安装工具。
Linux VZ [CreateDisk](https://github.com/lima-vm/lima/blob/v2.3.0/pkg/driver/vz/vz_driver_darwin.go)调用[driverutil.EnsureDisk](https://github.com/lima-vm/lima/blob/v2.3.0/pkg/driverutil/disk.go)；raw转换经[proxyimgutil](https://github.com/lima-vm/lima/blob/v2.3.0/pkg/imgutil/proxyimgutil/proxyimgutil.go)在qemu-img缺失的ErrNotFound时回退[nativeimgutil](https://github.com/lima-vm/lima/blob/v2.3.0/pkg/imgutil/nativeimgutil/nativeimgutil.go)，native也支持创建/resize。不是仅用Mac guest的native分支推断Linux；不需要为该方案先全局brew装QEMU，其他转换错误仍正常失败。该静态链不是实际镜像可启动/磁盘预算证明。
[默认配置](https://github.com/lima-vm/lima/blob/v2.3.0/templates/default.yaml)支持显式ssh.loadDotSSHPubKeys/forwardAgent/forwardX11=false及networkOpts.hostLoopback=false（限VZ默认用户网络，不能配共享user-v2）。后者只限制默认网关到宿主loopback，不阻断宿主其他LAN地址，不能误当完全网络隔离。仍需明确无个人home/key挂载、仅指定只读source/control与独立报告目录、关闭自动portForward/proxy传播、项目专属LIMA_HOME及原guard下资源/进程生命周期。
`instance/start.go`确认foreground走ForegroundSysProcAttr，但真实guard对子孙进程、SIGTERM/SIGKILL、正常退出/失败清理仍需审查并探针验证；不将该flag本身当生命周期已证明。下一步先isolated guest能力探针，再同后端基线/候选与netlab原门；现未启动VM、不宣称恢复Linux。


## 实际guard与VM生命周期约束（42573745492）

已只读取得run-build（115B，SHA83c9aa2c98dd60d9eaab670d0fbcd1ae4e20460320a963f1db7d480f4d36b7ce）和build_guard.py（2138B，SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb）。实际空间门50/40GiB、Mac target100GiB；原35/20设计文字撤回，不修改实际policy。guard以新session启动命令并传递build.lock FD；中断/低空间时TERM整组，10秒后KILL。若leader已退出，stop直接返回，因此只有foreground+finally还不足以防控制器SIGKILL后的VM残留。
固定Lima非WindowsForegroundSysProcAttr为空、start_unix.go使用syscall.Exec替换为hostagent，故foreground可继承guard组。实现将用同组lease supervisor持有从guard继承的lock FD：控制器关闭lease或死亡EOF即停止自有VM，正常退出也停止并wait；所有hostagent继续同组，不启动后台默认start。实际探针需证明正常清理及受控杀死子控制器后lease清理，最终Stopped/hostagent退出；这还不是已实现或已运行证明。首capability guest不挂载任何宿主目录，不进入产品编译/个人网络；保持2CPU/6GiB/30GiB候选上限和实际50/40门，磁盘增长仍由原guard监测。


## VZ能力与生命周期已实际证明（42579909366）

controlbedd896，source tree2b5bfce2/snapshot6a046588，固定工具/镜像已实际校验。短实例名修复macOS临时socket限制；按本次IID与/run/lima-boot-done相等验完整初始化。Linux aarch64 6.8.0-147、namespace/veth/netem/nft/conntrack实际成功；正常停机与子controller真实SIGKILL后lease EOF→stop rc0→Stopped均成功、同guard PGID、无本次helper。空host mounts与无个人key/agent/proxy/自动应用端口保留，未改installed guard/策略。前两执行器红在RUNS保留。

下一具体实现为把原scripts/network.sh放入该隔离guest，沿同源码/可信control回执，固定Rust1.95四component校验和，先实际构建/冒烟；随后新后端基线和候选同条件原门。首构建不预设额外全局缓存或资源扩张，计实际磁盘/时间后再决定项目专属缓存。当前能力任务product_tests_executed=false，N1原p3/hop红和N2–N5剩余范围均未结案。


### 2026-11-09续：取消与全路径活性的未验证边界

当前RetireUnheard::drop在无收包且超过ACK floor时退役；exchange_inner在同一floor之后先发全路径PING再等grace。下一反例应在Linux独立netns建立真实wire后，对loopback注入约4.8秒往返、让一个调用4.1秒到期且另一请求同wire仍等待；验证第一调用只STOP本流，活备用在grace内回包后第二请求成功。不是仅用ACK一直活跃的长等待替代该反例，也不能直接把4.1秒无ACK当全路死亡。具体实现需统一活性证据所有者，同时保留真正长黑洞取消后的退役，避免重现F7长期重复交出死wire；不增加每请求无谓探测或异步清理泄漏。此项尚未实现/运行，不冒称行为红。


### F8若取得真正行为红，修复的所有权约束

不要仅把Drop判死阈值加一秒来追当前测试，也不能一律禁止长静默退役。共享wire的活性应有单一证据所有者：接收计数变化清除当前怀疑；quiet达到原Patience后发一次全路径probe，按该probe的grace判断；旧连接/旧probe不能退役替换后的新wire。取消本调用只STOP其流，不能跳过probe宽限判共享wire死。

可沿Wires已有Arc/Slot与resident1秒tick承载，但必须覆盖直接Node调用没有resident tick的情况；若用独立有界确认任务，应每wire合并、只持Weak、不继承已结束前台deadline，且replacement/新收包/Node退出时取消和释放，不能一请求一套无界任务或重复PING。正常exchange和取消路径应共享同一proof，裸未持有wire的配对连接保留本地边界。原F7长静默测试需要验证完成确认后的快速退役，不把“无probe证明的Drop必须同步关闭”当不可改实现细节；修改观察协议须明确记录并同时保留真黑洞和F8阳性控制。此为待F8实际结果后收敛的设计，尚未实现/预记通过。


F8现已在Turing143rd取得真正行为红（2401ms到达、4101ms过期、零回包、两请求、unanswered1、共享wire None），上述修复约束开始实施，尚未改生产。既有Wires为Arc、resident tick1秒；直接Node调用无resident tick也须完成有界确认，不能只依赖tick。建议将当前wire的probe proof/count/grace与有界任务所有权放到Slot，正常exchange和RetireUnheard委托它；任务仅持Weak<Wires>/WeakConnection，按stable_id防替换误伤，收到新包/替换/退出则清除和取消。实现时检查在自身完成路径移除JoinHandle的行为、锁顺序和无runtime的Drop，验证无额外每请求PING/无无界后台任务。


### 共享确认本批已实证，后续仍须保留的边界

150/151同源Linux21/Mac19全绿，F8不改函数，F7 v2与Weak owner/replacement通过；实现与证据见STATE。只覆盖本批直接Node及其流/确认生命周期，不能把下列复核遗漏：resident tick仍有独立silence/network条件，当全路grace比其silence门更长时需验证协调；Accepted后长无应答的取消也需独立真实覆盖。Idle、正常运行中多次quiet epoch、替换和退出后的任务/流量成本在N3计量，不用字段数量替代实际成本。


### 2026-11-09 11:30 UTC：两处剩余活性边界静态复核

准确生产源25d0c803中，Wires::tick对held slot仍直接依据quiet/silent或network change门close，未先检查/加入Slot::probe；独立确认的grace可长达30秒，而network change低RTT门可为3秒，因此必须以真实连接/网络场景验证两者不会抢先判死。这是静态风险，尚未取得行为红，也未改生产。served入站弱连接也有独立静默close，不能只检查held路径。

ask_early的Accepted分支在adopt后继续持有Aborts并等待原by，已有短取消/到期证明STOP本流和保留健康wire。该分支尚未建立RetireUnheard/Slot确认，内部spawn只以req.budget等read_to_end；长时间无收包后取消的死wire保护不能由普通exchange F7推定。下一独立回归应先到真实Accepted/adopt/响应pending，再私有netns按该UDP端口双向drop，确认heard不变/真实包命中/drop前远端open，取消后观察owner全路grace内恰一次退役，同时保留现有健康Accepted控制。不要把局部静态发现记作已重现故障或修改已在途CPU/hop快照。


### 2026-11-09 CPU对照的负载证据缺口

原p1s实际为持续CPU争用加20×300ms道路延迟尖峰，不能写成20次CPU暂停；门是每端最多一次selected move、零retired，未约束最慢按键或punch数。152/153都2/2但候选3.8秒/12punch、原版732.4ms/5punch，尚无整体加速证据。原busy_beside以pid % available_parallelism得到CPU编号（不一定是有效allowed CPU ID），忽略每次taskset status，并filter_map忽略busy contender启动失败。因此当前report只证明该fixture观测结果，没有强制亲和/2busy进程有效负载的证据。保留原运行，不改其门或回写为通过新负载合同。

最终性能复验前需新测量revision：从真实allowed CPU集合选ID；断言每个resident全部现有thread的taskset成功与实际亲和、两个busy contender存活且同CPU；观测前后留PID/亲和/状态与CPU时间证据。不得只增加打印、改变竞争总量或改道路/时长/原门；同一新fixture逐字应用原版/候选并各自锚源。先短校准实际强制成功及非法/失败配置不继续计时，再独占串行新对照。此为待实施的实验有效性修复，当前产品归因保持未证。


### 2026-11-09 F9/F10实际红后的候选

158th两项前置先过后实际红。源码显示，启动probe时slot.rx仍可能是旧采样，下一tick会将probe开始前的ACK当新回答并清task；这是控制流解释，未直接打印私有rx，不冒充运行字段观测。候选在proof开始同步slot.rx到count，不伪造heard时间；tick读到真正新count仍清proof/mark heard，否则有未完成proof时不由独立silence/change期限抢先关闭。

Accepted在adopt后持有与普通请求相同RetireUnheard，正常响应disarm；取消只触发/加入owner确认，不直接判dead。长等待期间可能收到过早期握手ACK，不能将一次历史count变化当永久健康：原ACK时间无进展或整个等待达到原silent时间后，允许用当前count做全路验证，是否退役仍由之后原grace内有无新包决定。此条件可能为仍健康的长取消增加一次验证，按wire合并有界；新增16秒健康Accepted阳性并等待grace后才发独立控制，N3实际探测开销仍需计量。

这不等于所有tick/served静默策略已统一：无pending确认时现有silence/network条件、入站served的独立close、长期后台与跨进程期限等剩余合同继续保留，不能用本批绿结束完整N2。


CPU新helper的首轮Mac163只因模块路径解析停在formatter，无编译/运行。已增加显式path，路径相对声明文件目录，依据[Rust模块路径规则](https://doc.rust-lang.org/reference/items/modules.html#the-path-attribute)。三项短校准明确ignore，单独profile加--ignored执行全部，避免full netlab并行运行短校准的busy loop污染计时场景；这不是忽略产品失败，N0校准必须独立有证据后才开始新CPU对照。


### 2026-11-09 162nd实测与168th待因果诊断

162nd第二轮X同wire139750384102992的carried(quality85.61ms)在7347ms后消失，仅relay133ms，故这部分不是journal聚合假象。Y两事件wire不同，仍不能合成同连接回退。x/y open_hop后的两次5秒检查时间与两次撤路相近；源码Wires.selected_beats_carried只挑Main再Tunnel一条，而Link/close_carried按peer共享；未知/缺carried时目前返回true。这是待证的主动清理解释，不排除stream结束或旧attachment清理。

当前只加opt-in生命周期事实：Attached.serial、Link attach/detach/IO side/current serial、outgoing comparison wire/facts、hop2look release与close request，默认不构建details、不改变政策或journal词。f4i诊断在断言前复制x/y/m serve.log，新增SSH-only profile三轮diagnostic，非SLO；168当前在跑。实际触发点/所属generation须先与Path/Retired wire对齐，再决定共享连接证据范围与cleanup所有权，不能仅把所有未知路径永久保留来骗过门。


## 178共享carrier所有权反例与候选

peer级质量/selected tuple共享，不代表每条connection都有共同selected路径。入站可能只剩Custom且watcher无selected，仍在传真实数据；178 v2已证明只看Main会错误允许释放。候选释放条件须对Main/Tunnel/served所有live消费者成立，并去重stable_id、排除closed/expired。非空无Custom集合无需carrier；空快照保守；有Custom仍用原测量gain。2×5秒节奏不动，诊断shared_comparison留全连接事实。实际红→绿、完整Hop、未知快照/多quiet epoch/资源成本与generation竞态仍须逐项验证；没有据此宣布162根因或N1/N2完成。


## 193/194后的设计接续：已证、待证与实验顺序

已证的旧generation错误189→193/194整fixture同源红转绿：实际release只能删除仍属该attachment serial的link，只有成功释放当前owner才记10分钟lost。它不解决190四次current-generation release。190第三轮双端释放在t0前81.4/54.8秒，旧准备helper仍返回Relay；新held_precondition_v2先加强连续reported hold与绝对准备期限，保留原故障/恢复/稳定门，并独立记revision。

N1下一步先核新版准备有效性，再做同fixture的baseline/candidate/固定好路参照。190 current release的两次比较曾给出relay quality约131ms、carried约109ms，部分窗口samples0；现有quarter-RTT gain会把这对差距当作不足以换路。必须结合实际操作长尾、sample年龄/数量和固定路参照评估gain、置信与清理策略，不能见raw carried RTT较低就宣称违反已有公式，也不能把未知质量当作已证明无价值或仅放宽1500ms门。固定路参照可研究只在自有namespace控制候选可达性，保留真实Node/同负载；尚非批准或已执行的产品选路参数。

N2另有源码支持但未复现的close-vs-offer窗口：let_path_go先Links.has，再await Endpoint.close_carried；后者直接向remote actor队列发送CloseCustom，可能在新attach＋offer之后才入队。当前四个生产attach入口（SSH入站、SSH主动、hop主动、hop入站）与三个close调用点应统一审查；仅加serial保护detach不能证明该窗口消失。候选设计是同peer的attach＋offer与check-unused＋enqueue-close序列化；close方法目前等待入队而非actor处理完成，需验证队列/resolve消息顺序，不随意套全局锁或声称返回即远端完成。先做能控制这个合法交错的真实合同，再改；无实际反例前仅记待证风险，不归因182/162。

N3 CPU181多punch仅单pair信号。vendor remote_state已有三类低量debug触发事实：left a silent direct path、selected path degraded、connections not good enough；高频khor_net_samples是另一target。可优先研究仅启用已有decision模块日志定位18 vs3来源，避免先加新的vendor策略或把本机CPU卡顿一律视为网络退化。任何诊断额外日志均另标非SLO；仍需同core/affinity/原负载、独立基线与全部outcomes。


## N4真实服务器接续：Mac主动配对，不依赖SSH回连

用户已确认A=macserver、B=Turing异网。20261109T172201只读Mac SSH在banner超时，GH构建通道仍正常；这是一次访问通道观测，非Mac离线结论。可复用GH受guard构建/运行Mac主动客户端，与Turing临时resident配对，避免等待Mac SSH或新公网控制端口。Turing先在owned目录建立fresh identity/store与短期配对能力，私库workflow只将该临时能力交给可信controller/测试客户端，个人PAT/SSH key/模型/签名/生产store均不转移；能力不进普通日志、源码/通用artifact，回执只记hash和public ID。双方都核实际snapshot/control/binary/fixture身份。

两端完成编译后再开始测量：使用真实Node复用client及自身IPC、独立测试目录；Mac主动pair后，Turing从新scratch roster识别唯一Mac测试peer，双向读取双方fixture。先测冷/暖请求与选路，再逐步增加断线/不对称/背景流。Mac测试resident须在guard生命周期内保持到Turing结束，异常、期限/取消后只清本次资源；不能后台脱离guard或把本机已安装Khor当测试产物。具体短期能力传输/私有输入字段、角色同步/终态资源证明需先做合同并查实际API，再实施；这是接续设计，尚未实现或运行。


N4实际API接续核对（c85ed1b1，静态）：Node::invite是同步API，需endpoint文件，能力包含issuer id/name/live direct/relays与fresh token；发行端保存token hash与mint instant，15分钟、成功pair后烧毁，不能延长该窗。Node::pair在resident上执行且返回对端name，并非ID；成功后应另核fresh roster中的expected Turing public ID/双方唯一测试peer。Node::links只返回cached/aged Wires reading，不等于当前Links carrier map；不得拿它当原子attachment证明。

物理测试不能设置KHOR_NETLAB：该env仅让198.18.0.0/15 lab地址绕过代理TUN过滤。应保持off-the-segment（KHOR_NO_MDNS=1）和每角色独立KHOR_HOME/TMUX_TMPDIR/socket，沿实际全网可达性、正常信任/0RTT规则；Node.scratch现有Reaped可按keeper PID TERM/等待/KILL，但现有scratch(tag)会删同PID目录，新物理controller应create-new owned root/owner marker，拒绝覆盖旧目录。冷/暖读只读双方新fixture，不碰既有store/session。要在测试本身/控制器证明lease EOF/截止/异常清理，不因进程state文件消失就推已清。


## U11/U12继续审计：cold peer relay与移动地址（2026-11-09）

当前hop::middles_with_cost已对Legs::Measured按两腿总ms排序，实际端到端由iroh再测；但Legs::Dial等冷起点仍将direct第一腿放在前面，并限制relay→middle→direct的冷候选。它是未测时减少尝试的旧启发式，不能据Measured排序或单middle201参照宣称整个peer-relay冷/暖都已速度优先。下一步需要两个middle不同速度、relay首腿更快、估计过期/未知、双向不对称等反例及有界竞速/背景预算；尊重非递归、可信设备、暂停/用户中继配置，不盲改TRIES/时间门。

journal::watch_ears把完整DirectAddr集合经过interface_addrs过滤，Local及此前已通知同址才算接口；IPv4/IPv6都用SocketAddr比较。Interfaces原1秒settle负责合并接口变化。静态发现saw即使filtered集合不变也重设due，故QAD-only候选持续更新可能推迟真正已稳定的接口通知；210先加生产状态机合同，不修改策略，实际红/绿待远端。QAD-only新NAT映射被排除在“接口变了”之外是当前事实，不能据此断定映射迁移/发布失败，也不能把每个NAT hint都当整网故障触发全peer重连。需分别验证接口集合、外部映射/候选提示、整网/前台恢复这三类事件的更新/合并/预算与旧完成事件归属。

现有p4只证明地址来源Local↔QAD时不该误报变化，静态观察稳定不等于移动重编号验收。完整IPv4映射、IPv6地址/前缀、独立/同时变化、peer-relay移动与iPhone切网的物理注入/首个业务/最好路/成本矩阵保持未完成。计划中的epoch用于隔离旧结果，不能把健康共享连接跟随任意候选变化全部清掉。roadbook::note_roads记录的是道路类型/RTT成本，不是endpoint拨号地址发布；移动地址的发布/发现链仍须独立跟踪，不将这个方法当覆盖证明。

N4真跨网可优先用公开Node::serve在隔离测试进程内启动resident，Node::open_as观察client复用其IPC，再用Node::invite/pair、devices核public身份、ls_of核指定fixture内容；khor_node::link::stop_serving在同进程有按home的shutdown door。这样有机会避免CLI keeper子进程的额外收尾，但角色同步、临时ticket私有传递、lease EOF/绝对期限、真正join和owned-root清理仍须合同验证。此处仅API审计/接续建议，没有实现或真跨网运行，不能将首次配对后的读取称为冷网络建连。


## 208当前owner释放的设计边界（原始数据已留，尚不改策略）

208第三轮Y端committed serial1在两个5秒look后真实detach/mark lost，X接到reader ended后close；原owner没有被serial2替换，serial2在释放约16秒后才重建。两个live wire都有Custom：Carried质量约110.692/109.433ms，Relay132.754/130.884ms，samples1/0、age约2/4秒。相同fixture固定Carried稳态约145ms业务/85ms端到端质量，但不能把跨case差值当该时刻反事实。完整selected/path/lifecycle时序在208/measured-release-trace.json。

当前path::selected_beats_carried的逻辑是“没有carried满足selector gain”，并不要求selected反过来显著更快；也不单独核quality.age/samples。故上述约110对131ms确实会按现行quarter margin得到release，不能称实现违反现有公式；问题在确认/滞回/开路试用与成本策略是否符合速度优先目标。两次age/rtt不同，不无依据声称是同一旧样本计了两次。不能仅降低gain门或让所有候选无限保留。

收敛时要区分：①新候选仍在有界验证/估计不确定；②有足够新鲜端到端证据确认既有替代路更好；③无消费者/试用到期应收回成本。确认失败后10分钟LOST与“收益未证实/样本稀疏”不应未经合同混为一类。给foreground更快且有上限的验证机会、保留健康共享消费者与旧epoch所有权、idle有明确资源回收；先构造稀疏/静默/真正较慢的正反控制，再同时测速度/探测/流量/能耗。当前只留设计边界，完整N1/N2仍未完成。


## N4 SSH恢复后的具体控制协议（2026-11-09 22:06UTC，尚未实现/运行）

用户要求重试后macserver SSH已实际成功，停止沿用旧banner timeout作为依赖。仍沿Mac installed run-build guard与Turing owned单槽；双方编译结束、计时无其他任务后才启跨网数据面。临时root使用短路径、create-new owner/UID/run-ID，拒绝覆盖，避免macOS Unix socket路径上限；KHOR_NETLAB始终不设置。

拟以dev-only Rust example运行Node::serve与本进程公开stop_serving，另open_as只通过该resident IPC发操作。控制走owned root内Unix socket、JSON request ID及固定命令白名单：ready(public ID/fixture path)、mint(私有ticket)、pair(只接临时ticket)、devices(核expected ID)、read(只读对方新fixture)、links、stop。私有ticket只经两个可信SSH控制器内存转交，不打印/保存普通artifact；日志仅ID/hash/请求outcomes，pair后票按原规则烧毁，不扩15分钟。首次pair后的读取明确不是冷建连。

Python lease supervisor持有Unix socket端，每次controller EOF/绝对期限/自身错误关闭lease，Rust看到EOF即stop_serving并真正等待serve JoinHandle；进程kill时kernel自动关闭lease也触发相同清理。选择可取消的Unix socket读取，避免tokio::io::stdin的不可取消blocking线程拖住runtime退出。Rust本身也有不依赖controller的绝对资源期限；正常/EOF/受控kill supervisor分别做合同，完成需核子PGID空、resident已join、只本次owned root/socket生命周期结束。未知目录/UID/旧owner保留并拒绝，不以state文件消失冒称进程已停。此节是执行前设计，未创建resident或开始跨网验收。


## 267实际内层恢复等待的接续诊断（2026-11-10，尚未新改策略）

所有相对时间统一以原middle_replug_finished phase，不能用result.after（它是读完后的地址快照）。原误标timeline保留原字节，修正结果outer-copy-timeline-replug-reference.json。

Noq12修复后的267：外层Relay→Direct约+1.244/+1.287s，inner Custom在+5.317/+5.326s才解除silent可用性并开始确认，+7.326/+7.567s Go。inner-recovery-hold-timeline.json显示两端rehold=false、answered当tick即解除（immediate=true）；所以中间约4秒**不是SILENT_HOLD**（实际1秒，也未启用），之后约2秒才是现有确认。mark_silent已经要求样本packet sent>最后judged，旧ACK不能直接清可用性，不能误修掉这项保护。

下一先核Noq在外层恢复后是否收到了旧packet的ACK并清PTO，却还没有新的post-judgment packet：既有wake仅处理pto_count>0的copy，mark_silent对新样本的正确要求可能让已无PTO的恢复路等下一个keepalive。此为待证假设，不能据时间相近就改5/15秒保活或缩短2秒确认。

若原始stats/实际actor合同证实，候选设计是每个已判silent的恢复epoch，在收到判定之后的真实回包提示、当前无silent copy且尚未取得post-judgment往返时，只安排一次新的恢复确认PING；保留原total budget、发包归属、stale owner防护、未应答继续silent、原2秒/两fresh observation ticks及收益门。重复hint不重复发包；无hint/健康路/首次未建立不探测，独立下一真实故障才可重新申请。不得让旧包本身确认新网，也不加永久轮询timer或每tick全peer探测。具体API/红例/发送失败与已存在在飞probe计量仍须审计/冻结后实施；还要原p1s/短停顿/黑洞与探测字节成本证明，不能只追单次恢复速度。


## N4目录基线后的正文校验扩展（2026-11-10）

263是实际paired-warm目录list，仅核marker名/类型/大小。dev-only WAN worker增加read_content：复用真实resident IPC→Node::read_all/FetchPath，固定读取对方fresh fixture的wan-<run>.txt，cap等于已知29-byte标记并逐byte核对total/cut/正文，控制回复仅公开ID/时间/字节数/verified布尔，不把正文、票据或私钥放artifact。它不改产品Node API、不创建新peer连接/协议、不称大文件吞吐。先Mac远端build-only，Linux与双向实际正文验收待独占槽；263原目录结果不回填为正文绿。


N2本机期限/取消的具体兼容提议见[IPC-LEASE-DESIGN](IPC-LEASE-DESIGN.md)，源码blob/SHA在evidence/net/ipc-lease-design-20261110/inputs.json。先闭合既有客户端short caller deadline，再分批做新旧codec、caller租约、回复/permit回收及分process验收；提议尚未启用，不把caller完成当server/remote已经停止。


### 已证实恢复无PTO等待与有界候选（2026-11-10）

002954诊断124/124可达且reported Carried最终hold；实际70条缺新往返快照均PTO0/unacked None，外层Go+1187/+1253ms，内层开始确认+5302/+5313ms、Go+7552/+7562ms。旧ACK清PTO后的等待假设已获实际stats支持，不能删除mark_silent的sent>judged保护。由于最早hint早于外层稳定，前文一次PING提议由ACCEPTANCE中的最多3次/250ms间隔/2秒有限窗口替代，尚未实施。状态归既有actor的Measured所有，不spawn任务/持强Connection/新永久timer；路已关/新epoch/已有Moving/足够自然样本必须有明确回收与合并。Noq12现代对照stalls现有门均绿，但候选worst约1秒、16轮punch高于baseline539.2ms/13轮；保留该负面观测并做matched repeats，不能称整体性能成功。


## Iroh39首轮后的时限与采样副本接续（2026-11-10，下一策略实施前）

fa99准确ba7源/ef0b，124/124可达、原采样请求四秒门51/51；但业务窗口先AB后BA，不能据此称双方四秒恢复。实际完整日志x/y内层Go为+4060.292/+5799.202ms；同版本c397 baseline为+7674.436/+7913.427ms。新增独立scripts/netlab-peer-decision.py在改下一策略前冻结：原物理/原采样checker字节保持，结合原请求门，要求两端实际Relay→Carried Go均<=4000ms；缺事件不自动绿。两旧run均真实红，证据peer-recovery-decision-four-seconds-red-green.json。

真实probe日志在report/netlab/*/diagnostics，不能只搜scenarios.log（那里仅有诊断路径提示）。fa99 x三次同copy，约+1060/+1310/+1560ms；y约+1050/+1550/+2049ms，copy A→B→A，均PTO0/Ok。y在+1299ms已新往返解除silence、无当前silent copy；中间两次预期250ms槽未发，原strict now<last+250ms门会因微小调度抖动跳过一整tick。副本交替还使Reading每copy的近期中位数难以积累3次新样本。x确认从+1809ms到+4060ms，2秒窗口也被250ms轮询跨边界拖慢。此处是实际日志与现有控制流的解释，未量得线程栈/future字节数。

下一Iroh40候选：同一有限恢复episode优先保持已选live copy，同PTO时不随latest received切换，关闭或更低PTO健康copy可接替；只存ID，零持久强连接。保留3次/2秒/至少250ms实际预约节奏、sent>judged、收益门、2秒与两fresh观察。由既有actor的MaybeFuture为待处理恢复预约和Moving的2/4秒边界设置有限sleep；没有pending decision时None，无新永久timer/后台task。早到的普通tick不取消随后准确due；仍silent/已足够自然样本/合并/用尽/过期时disarm，不能让past-due制造busy loop。到期醒来重新读取真实stats并走原选择规则，不能把timer本身当新样本或Go证据。

纯合同先覆盖copy粘附/降级failover、250ms边界微抖动不漏预约、无pending无deadline、silent/expired disarm、2/4秒确认仍需fresh与原gain。vendor绿后仍须原物理与两个四秒门至少3轮、实际probe预算、原CPU/黑洞/成本。为隔离新IPC v2，物理候选在15f45e6b的独立source worktree构造，再把同一代码补丁带回feat/net；不能把它当最终分支整源全验收。此处尚未实施，不能预记通过。

Noq12稳定性诊断用现成netlab-diagnostic，准确旧a42源（无Iroh39/新IPC）、原p1s600秒/20次尖峰+f4s，role=diagnostic；全样本有观测开销，不能替代quiet p1s红或quiet最终验收。


### Iroh40物理红与下一测量窗口候选（2026-11-10，实施前）

fc435ccd准确c673源：124/124可达，但原采样门48/51（3次AB仍Relay）、双端决策5392/5630ms，原门不改。实际预约3次/每次>=250ms/总<2秒均满足；两端首次probe约+0.88/0.89秒、第二约+1.13/1.14秒，外层Direct Go到+1.218/+1.289秒，因此前两次仍可能测得慢承载。第三次约+1.38/1.39秒测新路；当前近期中位数保留旧恢复样本2秒，直到约+3.38秒才开始另一个2秒确认。该解释由真实probe/outer/decision时间与现有median实现支持；尚未逐包绑定所有早期Noq样本，不冒称raw唯一因果。

下一Iroh41候选在已记录的有限故障恢复episode内识别显著向下质量变化：新包必须在本恢复采样开始之后发送，真实新RTT至少比先前质量低25%，才为该road的副本开启新的质量窗口；每episode最多3次，后续重启还须较上次重启值再降25%，避免高低抖动反复清坏样本。复制窗口epoch到同road各Reading，迟到的epoch之前packet只保留真实传输计数/时间，不进入新质量窗口；新copy继承已有epoch。健康/初始/无hint路不触发，不增加PING或定时器。原可用性sent>judged、2秒持续候选确认/决定后两fresh样本、收益门、3probe/2秒/250ms保持；一次偶发快样本后变慢应在原确认前撤候选，不能直接Go。

纯状态先覆盖两次慢恢复样本后快样本、健康对照不重启、epoch前晚ACK不污染、各copy与新copy继承、单次快脉冲及重复摆动不无限重启、原ACK计数/changed与时间不伪造。仍须两门同源物理至少3轮、原quiet stalls/黑洞和成本。当前只提议，未实现或预记通过。

## 2026-11-10：Iroh42提议——恢复与确认复用同一个copy优先规则

诊断8c90acc0准确5a39a39b/tree0cd3fe0c/snapshot82976112只加诊断，不改Iroh41策略，原124/124、请求51/51仍绿，决策X3565ms/Y7549ms红。Y在1549ms于conn140618974140672取得84.038303ms并开始Moving，1550ms普通confirmation实际选择conn140619108789456（PTO0）；1799ms后一copy的101.627832ms单样本成为同road代表，低于25%收益门而取消Moving，5299ms才重新开始。另一copy的同刻PTO未记录，不把它武断归为错误故障判定；已静态确认recovery与confirmation在相同PTO时使用不同tie规则。

下一最小候选：普通candidate confirmation复用已有recovery_copy_rank（低PTO优先，同PTO优先已开始恢复的copy）；current探测仍选最高PTO的可疑copy。只在现有RecoveryCheck仍存续时有偏好，不增连接强owner、不改采样新鲜性/收益/2秒确认/4秒兜底/3恢复probe预算/250ms节奏。已关闭或更高PTO的旧copy不能凭偏好胜出。已有rank真实状态单元保持；以未修改完整物理/两个四秒门、实际copy与packet预算验前后，不以新诊断源凑三次性能绿。若延伸到正常无恢复的confirm copy稳定性或持久Moving偏好，应独立设计，不在此最小修正偷带。


## 2026-11-10：Iroh42失败后先证明实际PING链路（策略不变）

2c9d5898实际X三次API成功后RTT count/timestamps不变；精确缓存noq1.3.0 Path::ping只lock_and_wake并设置proto pending_ping。proto发送还会经过space/scheduling/pacing/congestion；ACK只有largest新确认且包含eliciting才更新sample。这是源码事实，尚不能指认哪个环节失败。Y无RecoveryCheck也未被原局部观测覆盖。

下一只补诊断：full-debug模式对每条Custom copy记录同一原子PathStats（PING/ACK/PATH_ACK的tx/rx、datagram、PTO/last_received/unacked、RTT计数与sample timestamps/cwnd）以及对应Reading、silence/recovery/Moving状态；普通模式不采日志，不增加PING/计时/新owner。固定旧tests与物理门。先新增独立Noq模型合同，确认健康backup的PING确实发送、原路ACK并使新packet RTT样本前进；黑洞负向则API可Ok、tx前进却样本不得假刷新。新文件单独登记，不改旧multipath whole fixture。

Mac可在Turing原quiet计时期间独立做net-vendor；Turing不夹编译。diagnostic源只用于因果定位，不算quiet性能或prospective三轮接受。之后依据实际tx/rx/ACK事实才设计最小修复；不先调250ms/2秒/4秒或去掉largest-newly-acked保护。


## 2026-11-10 06:18UTC：两次诊断未复现与暂停恢复红

full-counter同源1a两轮6d83/ff132实际124/124、双端3585/3578及3285/3518ms，两门/state/probe预算绿；前次2c9d的5825/5870ms红仍未解释，不能用两轮正常counter反推旧失败。不盲刷第三诊断。Turing先用5a39/Iroh41与既有8d/Iroh42 quiet原stalls建立同一小补丁前后对照（同4eeee控制、原600秒/20尖峰+p5），期间Mac推进独立恢复回归；这不关闭PING缺失诊断。

Linux c2ec同Mac已绿源a00，原busy-wire在主动pause→resume后立刻content_cancel失败，主I/O/cancel早已完成；STOP层未执行。静态事实：pause::set_on同步更新POLLED，但serve接收门另用半秒poll维护的AtomicBool，存在本地resume后旧门仍拒绝的窗口；poll旧磁盘读覆盖本地新写亦是另一未证竞态。先独立合同冻结实际旧门窗口，不能改旧731行加sleep/重试。

拟测试仅加cfg(test)poll发布后的可控暂停点：B实际观察on并更新接收门后阻止下轮poll，set_on(false)立即更新真实缓存，再经原A resident IPC向真实B发送只读list，3秒内应成功且barrier未释放；当前旧接收门应真实Refused红。原canonical整个函数/新STOP整fixture保持。取得红后最小候选仅让接收准入读既有已初始化缓存（无accept loop磁盘I/O、无新轮询/重拨/放宽暂停），再验旧pause拒绝与原canonical/STOP/portable。任何poll发布generation保护或更广单一缓存重构需单独证明，不偷带。


## 2026-11-10 06:55UTC：发送队列诊断，先不改拥塞或恢复策略

1a同source两次完整计数物理正常，旧2c9d慢恢复仍缺tx/PATH_ACK counter，不能倒推因果。旧raw另见X低PTOcopy保留旧unacked_since且近期仍收包；另一copy为PTO2但unacked None，同样近期收包。不是“两copy同PTO”的证据；当前不实施轮转、放宽gain或提前PTO。Linux+Mac现有两个PING模型441/50/9均绿。

下一仅扩展Noq3既有stats观测：同一PathStats快照增加bytes_in_flight、当前space的ping_pending、global非path-specific pending_other_frames；后者不等于此path可以承载业务。既有full-debug会自动打印这些值，区分PING排队/确实发送、窗口占满、ACK/样本未更新；普通selector仍不读取新字段，不添PING/计时器/重试/强owner，不改丢包/拥塞/ACK/RFC时间计算。新模型在原两case保留所有断言并补“API后pending但tx未增、发送后pending清除”的真实传输合同；旧whole multipath/paths/物理门仍保持。先Turing单槽vendor，再一次新版完整字段物理诊断，不把正常参照当旧失败已解决。

已核[标准RFC9002 §7](https://www.rfc-editor.org/rfc/rfc9002.html#section-7)：普通在飞受拥塞窗口约束，PTO到期/进入恢复有指定例外；[§6.2.4](https://www.rfc-editor.org/rfc/rfc9002.html#section-6.2.4)与[§7.5](https://www.rfc-editor.org/rfc/rfc9002.html#section-7.5)描述PTO探测。应用层path.ping成功不能直接等同PTO到期、实际发送或ACK；不得为达到4秒而一律绕过拥塞窗口。本节是协议边界与新增观测设计，不声称已证明2c9d被cwnd阻挡。


## 2026-11-10 07:09UTC：Iroh43自然恢复epoch的具体候选（修复前）

新v2 7bf72701真实124/124、请求门绿，XGo3658/Y7404ms红；sourcef0a/tree1cfb/snapshot92af/control ef0b，原group空/16artifact核。Y judge=1778813.572639，首次合法post-judge sample sent=.574443（晚约1.8ms）、ACK得到485.85ms；随后263.72ms。read_paths先mark_silent恢复可达并移除Silence，再recovery_probes迭代剩余Silence，因此没有RecoveryCheck。+1.4s后pendingPING=false/inflight0/cwnd约28k/globalOther=false，直至+5.4s自然keepalive得83.79ms，才开始原2秒Moving，+7.404s Go。此轮Y不是观测到的cwnd拥塞；旧2c9d X缺采样仍为另一未证分支。

先引入actor真正调用的Measured统一bookkeeping入口，旧顺序不改，新增独立状态合同：既有故障+新合法但慢的自然RTT+judgment后真实hint，同tick应恢复可达并保留原有有限恢复episode；随后post-episode快样本才能重建质量窗口。healthy无故障不建episode，旧包不恢复，3预约/250ms/2秒、2秒Moving/两fresh/gain/4秒门全部保持，原whole tests模块5c4da4b4与物理fixture不改。取得真实单元红后，仅让episode登记先于mark_silent清除Silence，再恢复可达，不让第一次ACK直接证明优路、不放宽判断。已有episode采样充分照旧停止额外PING，selected+available或已有Moving仍合并，closed/新epoch/过期回收沿原规则。

新helper避免测试在旁边镜像顺序；actor与单元使用同入口。候选必须旧50+新合同vendor绿、原同版物理两门、至少3次/原quiet成本后再评估；不预记已解决7.4秒或旧5.8秒。


## 2026-11-10 07:56UTC：Iroh44有限采样与迁移确认分离（实施前）

43同source首物理f2b832dc仍124/124但请求11bad/decision X7728/Y3470ms红、6预约2episodes预算绿。X同copy actualPING/ACK均前进、pendingfalse/inflight0/cwnd空余；+1478ms 85ms启动Moving，+1727ms新112ms使仅两样本的uppermedian变112，quarter gain暂失，Moving与补测一起取消。直到+5727ms自然keepalive才85.8，再确认至7728。不是copy翻转/cwnd；旧2c9d缺RTT仍单列未证。measurement_stops_after_noisy_quality.json保存counter/transition及推断边界。

下一先做旧行为不变的helper/探测代码抽取及独立状态红。候选设计仅在已有RecoveryCheck有限窗口、原pending候选仍open/可达、current quality recent只有1或2样本、选择器暂留当前路时，保留**采样模式**至原pending开始+2秒内，沿既有confirmation双路probe规则补足median证据；不批准迁移。>=3样本仍无收益、关闭/再次silent、无恢复episode、过期即按原清除；健康场景不延长。恢复额外3预约/250ms/2秒不变、不补预算，合并规则不变；补测使用已有有限confirmation探测途径，原quiet/实际packet成本必须重验，不能以通过预约门替代总成本。

Moving新增私有sampling_only模式，Road/原since仍是值、不持强connection；现有MaybeFuture在采样模式只守2秒边界，无永久timer。**候选重新被选择时必须重启原连续2秒确认**，不得把不合格期间算入确认；旧2fresh/收益/2秒/4秒上限不变。重新失败、另一候选、rest或fault leave均不得复用旧sampling时钟。actor真实调用与新单元同helper；独立fixture覆盖稀疏噪声保采样、>=3慢样本/健康/关闭/silent/过期负向、不能越过连续确认和fresh要求。旧50及43新3整fixture保持，旧红与控制不覆盖。单位红绿后原两四秒门至少3轮/quiet/黑洞/成本仍待，不预记成功。


## 2026-11-10 08:27：恢复采样发生在承载层仍在换路时（先验证假设）

Iroh44同whole单元54/2红→56绿、Noq441/home9绿，但first physical a4ed770d仍124/124且X7663.773/Y7774.490ms、请求10bad。两端初三次恢复RTT约280/131/130及219/129/129ms，均未满足相对132ms Relay的原收益，因此从未进入Moving，44采样保持没有被行使；之后等待5秒keepalive才出现85ms。原总9预约3groups中一组为准备Relay，replug Custom仍6预约2episodes，不能混称重插增加预算。

实际attachment1指明x receiver outer wire140179679991456、y initiator140183035096736；middle splice incoming139900775062032/onward139900842190912。+1206ms x actualouter快IP存在但无selected；+1226ms middle actualincoming仍Relay、onward为Direct；+1277ms y actualouter Direct。journal显示x与middle又有其他copy的选择事件，不能据此推断实际承载wire已切换。约130ms与混合两腿一致，但尚非逐packet证明。旧44及43物理红完整保留，不再盲重刷同profile。

下一步只加opt-in诊断：在hop生命周期内订阅这四个实际outer weak handle的PathEventStream，事件驱动记录选择/开关/lag及当前PathFacts；不持有新强Connection、不增常驻task或timer，生产关闭诊断时无订阅，不改变路由/probe/协议/原fixture/4秒门。先对实际outer事件与inner PING/ACK/RTT/quality/Moving时序做同hash关联，再决定是否需承载path epoch通知或其他最小修复；目前不实施跨层协议、反复refill恢复预算或缩短迁移防抖。若需要新测量epoch，必须明确去重/有限总成本/旧packet隔离、两端及middle不对称/闭合所有权，不能仅用本端leg事件假称两腿均已恢复。


08:38诊断首轮e0d70376：原124/124及request/decision/state全绿，X3222.682/Y3227.305ms。actual4 carrier事件完整，41ms IP选择于826/861/926/969ms；inner Moving sampling_only始终false，不能当44采样保持的实际覆盖。相同e819/tree2e52/snapshot2a10/control ef0b的4dc22c7e在跑，用于观察慢分支。下一实现按证据分岔：若实际carrier恢复晚，先追承载地址恢复与测量epoch的先后；若四实际carrier早已快而inner RTT慢，核ACK/测量copy与quality；若API排队但未发，则核真实pending/inflight/cwnd/传输条件。不得根据一轮绿撤销a4红，也不提前选择增加探针或缩短确认。


## 2026-11-10 08:46：cold dial阶段诊断先于修复

actual carrier repeat2 4dc22c7e并未进入replug：首目录10,033,849us超时，Node x→y Ask拨号10002ms、offered direct3及Relay198.18.1.2:3340、failures1/retry5；无hop bound/splice、仅1error outcome。source e819/tree2e52/snapshot2a10/control ef0b、56.019秒/16artifacts/原group空核。属于既有10秒cold-start独立红，不能作四秒恢复重测或丢弃。诊断模式下的实际业务失败，尚未证明无诊断时同样复现。

源码dial_with最多两ep.connect候选，known hints候选立即、identity候选headstart后；Endpoint.connect先await connect_with_opts（内含before_connect hook与resolve_remote），再await Noq Connecting握手。当前日志只见总dial，目标y在selector诊断零出现，无法区分解析/actor排队与Noq握手，不以offer有Relay当已发包。下一仅在已有khor_net_samples debug target添加connect start（目标身份/ALPN及offer计数）、connect_with_opts完成和handshake完成阶段/结果；不改变await顺序、错误传播、超时、并发、重试、探针、协议或fixture，不记录票据/请求正文。root格式/vendor原回归后原物理运行；若再次cold红，先按阶段定位。成功轮仍只作参照，无诊断性能与原stalls/成本最后另验。


## 2026-11-10 09:33：新连接已收包，旧关闭却重新标休眠（先真实回归）

现场用户已明确：回Wi-Fi后能通Turing/hinton/lecun但MacLab不通，Relay/peer relay未及时接管，反复关开iPhone App后某次才经Turing成功。不是自动恢复通过。MacLab被动phone journal有多次why=asleep退役、carried/relay切换及连接空窗；所有生产进程/配置未改。CLI采样是Turing→localhost、混合已装版本，App重启时点未控、busy/paused归属未证，不能替代phone→MacLab或精确恢复SLO。

当前源码Wires::tick分别收集heard与resting，末尾先back(heard)再asleep(resting)。同一peer旧连接晚到suspended关闭、另一条较新live连接已经收到真实新数据时，后处理旧休眠可覆盖刚确认的新可达，恢复背景pump又被left_alone抑制。这是静态可见候选，并未证明就是现场唯一根因；after_retirement并发、新地址epoch、LOST/MISSED候选冷却、远端resume准入等另查，不能靠此单项关闭整案。

先新增独立tests_wake_epoch真实loopback Noq fixture，原wires tests whole逐byte冻结。两个不同稳定wire、同peer身份：先注册old/new incoming，再关闭old为suspended，确保new实际收一条新stream数据，调用真实tick后仍应不left_alone、new可继续echo。负向两条都suspended时必须仍被left_alone，不变暂停/耗电语义。新模块以tests前缀由原net-node wires::tests filter自动覆盖，控制68不改。先旧行为真实红，源/whole新fixture冻结；然后仅纠正同tick冲突的证据顺序，原net-node全回归双OS、late-close/epoch独立覆盖后再评估，现场手机根因仍需时间线与准确源码验证。


## 2026-11-10 09:53：retire旧关闭提交必须早于新连接发布

3a3 same-tick已实际85d/7da wires12/1红→Turing1ea+Mac42793316303同source13绿/full net-node绿；仅此分项，现场自动恢复仍失败。相邻源码窗口：retire在slot所有权检查后retire_slot、释放slots锁，再after_retirement(asleep)。其间新adopt可发布新wire并back清旧状态，随后旧after_retirement又写asleep。原Unanswered已在retire_slot内提交失败，注释明确新adopt应在后清除；Asleep同样需要一致提交顺序。

先只加cfg(test)单次gate：真实Wires::retire在释放slots后/after_retirement前暂停，gate属于该Wires实例，阻塞只在测试spawn_blocking内，生产编译无字段/锁/等待。独立新fixture：真实old suspended close，retire进入gate后真实新wire adopt并echo，再放旧retire，不能重新left_alone；没有新wire时真实paused退役仍应留下休眠。原wires whole、新wake whole不改，new模块tests前缀走原net-node过滤。先旧行为实际红/新fixture冻whole，再只将Asleep提交放入retire_slot持槽锁阶段，after_retirement不再写迟到asleep；确认无反向锁顺序，原全node双OS继续。直接DialFailure::Asleep、新地址epoch与MISSED/LOST仍需各自因果验证，不将此测试扩称整体根因。


## 2026-11-10 10:44UTC：新鲜对端接触不能被旧休眠拨号撤销（修复前）

用户明确恢复依赖反复关开App，仍是自动恢复失败。f9旧退役提交修复已Linux61aa与Mac42798075914 attempt2同source全12项绿、wires15，首Mac attempt1 API截断失败原产物保留；仅关闭该退役竞态，未部署、未证明现场根因。

下一静态窗口是line_to_inner与ask_early：持拨号互斥的旧尝试得到Asleep后，无条件缓存失败并asleep；期间另一条真实incoming已serving/back，也会被旧结果重盖。等同一次拨号的排队者只按失败完成时刻复用，不检查期间是否有该peer的新鲜正向接触，可能把已恢复机器继续视作休眠。该假设需真实旧行为红，不扩大成所有地址/进程epoch问题。

先只加cfg(test)实例级、单次异步gate，在真实Asleep结果提交前暂停（owner仍持原dial lock，无生产等待/任务/字段）；新增dial_budget子模块自动走原net-node预算filter。真实loopback peer以实际Incoming拒绝/关闭产生Asleep，排队者实际poll到互斥等待，随后同peer真实反向握手/echo并serving；释放旧结果后不能再left_alone，排队者应重新实际拨号且可echo。负向：无新接触或只有另一peer接触时仍共享原Asleep/保持休眠、不额外拨号。两个实际拨号入口分别覆盖，先冻whole/旧行为红，再改生产。

候选限每peer正向接触版本：与gone标记同一锁，back清休眠并递增；持拨号锁开始尝试时捕获版本，Asleep只在版本未变时提交，缓存同带开始版本且Asleep复用要求版本相同。旧owner仍返回它实际收到的失败，不重放请求；非Asleep失败/计时/退避语义暂保持。无新timer/probe/常驻task，原120/600秒安静、5–300秒backoff、两候选上限、总deadline/取消、暂停/信任/accepted写jobs不改。正向版本不当作IPv4/IPv6网络epoch，也不能据此抹除所有LOST/MISSED或关闭所有相邻竞态。原whole wires、wake、retirement与旧dial_budget测试保持，修后原net-node双OS同source。


10:53 N1诊断补充（不改变策略）：bf6105dd的actual-carrier-path-events已由原冻结auditor验证，四条实际绑定outer在+1092/1176/1197/1274ms选IP，窗口无lag/end。X真实inner RTT为174.598534→103.166817→89.551779ms，明确日志“recovery quality window restarted”在103.166817处重开；随后最新89.55而reading quality仍103.17，至+3502ms仍未Moving，+3752ms才见Moving，原Go5505.782ms红保留。不能简写成始终三点median：重开后recent不足三点，44又没有既有pending因此未进入sampling_only。carrier-quality-restart-correlation.json锚实际raw/数据SHA与10行证据。这提供了“从未先合格时的有限新测量”设计问题；先独立合同和总包成本，不直接增加原3预约、删质量样本、缩短确认或据此猜手机根因。


## 2026-11-10 11:21UTC：先证明半成品hop的取消所有权，再并行寻路

第三Asleep修复959旧fixture已Linux全12绿；Mac setup问题只改fixture，两端显式loopback/原5秒期限。portable旧行为543/e07/d16在Mac42806129212实际24pass/2状态红，四负向与全部setup通过；候选fccc/714/2b5在Turing99534d56执行，Mac候选提交本地exec99550需取实际receipt，不重发。新旧whole冻结，phone手动恢复仍不算自动恢复。

相邻N2静态事实：line_to_inner先await hop_ahead，再开始普通dial；ask_early亦然。中间设备准备慢可能耗尽前台预算，即使其他路径本来可用。单纯select并行会放大已有取消窗口：open_hop在Links.attach提交后await ep.offer_addrs，之后才spawn watcher；Attached本身只有serial和ended receiver，drop不撤回Links条目，而attach已有自持Links的I/O task。取消准备future可能留下尚未交付watcher的attachment。该窗口需真实回归，不直接认定现场因果。

下一先冻独立hop::tests前缀fixture，经原net-node自动运行：实际Node::open_hop、真实loopback Tunnel QUIC流完成Ask::To响应、cfg(test)一次gate在attach后/offer前，记录serial后abort/join真实准备owner；应撤回仅自己那代attachment、stream真正停止而Tunnel连接保持、ASKING归零。负向覆盖正常交付watcher不被提前撤回、取消旧准备不能删掉同peer/同middle的新serial。新gate不进生产，原hop whole/旧所有fixture保持；先旧行为真实红，再最小所有权guard。

候选guard须以Links.detach_attached现有serial匹配撤回，不能改公共Attached的Drop破坏其他使用者，watcher接手后明确disarm。不添加无归属清理task。**offer开始后/已送actor但await未返回**的窗口仍要单独证明：移除Links不自动证明iroh已关闭公布的Custom path，不能以offer前用例关闭整个close-vs-offer矩阵；需追现有let_path_go/transport闭路事件与替代代际保护。

上述所有权完成后，再建立“慢middle正在实际准备、普通可用路径不能被它挡住”的端到端Node回归（非注入假拨号），再设计有界并行。原普通dial最多两候选、hop两middle总3秒、原caller总deadline/取消优先/输者收尾/accepted早期读取与写job不重放全部保持；正常路径先成功时，未交付的hop准备必须被回收，已交付且仍有用的hop须有明确Node owner。MISSED/LOST本地网络/两腿测量epoch另做失效合同，不因每个包或一次Wi-Fi通知清空远端休眠及所有冷却。


## 2026-11-10 11:50UTC：hop准备取消真实红后的最小所有权候选

0fb7/f42/f954在Turing5ea56c46原net-node实际hop14pass/1fail：取消gate内真实open_hop后orphaned=true、stopped=false，原12和正常watcher交付/同middle新serial保护两负向绿；134.349秒/11artifact/原group空核。前一538仅3处远端格式，不算运行红。整个新fixture与原hop/wires whole冻结，先有真实红。

候选同时安排流和路径的既有owner，避免只在Drop里删Links而不通知底层：attach后、任何可取消await前建立serial匹配的PendingHop guard，并把**既有一个watcher**提前启动，先只等attachment ended或handoff channel，不启动LOOK计时。准备future仍持Asking，offer返回后才经oneshot交给watcher并解除guard；取消时guard只撤自己serial，Asking同步Drop、handoff关闭，I/O ended由该watcher沿既有let_path_go撤路径。正常交付后的原比较/间隔/弱Node所有权不变；没有额外cleanup task、新timer、协议或探针，不能改公共Attached的Drop。cfg(test) gate仍是attach后/offer前，新增不可中断的owner登记必须在该gate之前，反映生产首个await前建立所有权的真实修复。

这使取消期间也有既有watcher接收stream结束并请求close_carried；并不证明既有let_path_go“check links.has然后await close”与新offer的所有并发均安全。已核resolve_remote经RemoteMap发送ResolveRemote到同peer actor，actor同步登记/open_path；CloseCustom是另一消息；pending Noq open、late close/new replacement、重绑ears仍须实际代际/offer后合同。先同whole上述三例和原全net-node双OS，再补这些窗口，未证明前不并行普通拨号与hop准备，也不声称完整field接管已修复。


## 2026-11-10 12:16UTC：完整open_hop准备期限与记账（实施前）

c071/eda/dfc已Linux147ab8a0+Mac42812850230同source全部12项绿，hop15、wires15、budget30Linux/26Mac；旧0fb实际14/1红与新whole/旧whole保持。仅取消所有权scope闭合，尚不证明所有offer后代际/并行接管，未部署phone。

下一先附加独立prepare_budget子模块：旧offer_tests.rs整个文件逐byte保留为前缀，仅尾部新增模块声明，原whole证据仍锚旧source，另冻prefix长度/hash及新增whole；不改旧三个case或helper。真实Node::open_hop和Tunnel Admission后，在原gate保持不释放：无caller时应从入口起原HOP_WITHIN=3秒内返回false/回收；短caller scope应取更早期限；正常短caller放gate仍可成功。另一真实未admit场景关闭fixture旧cached Tunnel、提供实际loopback地址但不accept新handshake，caller到期返回应不记MISSED/peer-down。所有case收尾/join/STOP与原wire语义保持；先旧行为实际红。

候选使用入口冻结的一个绝对by=min(原3秒,caller)，前段tries和后段gate/offer都等待同一个by，不因阶段重置总预算。保留实际middle尝试失败的MISSED/原退避；caller先到期时不把它计作middle失败，admit后本机offer未完成亦不记“没有middle可达”。已有PendingHop+watcher负责超时清理，不新增无主任务或永久timer。hop_ahead入口前的同步设备表/文件IO不被这个异步期限虚假宣称可抢占，仍属N3调度/IO工作。

之后继续late close/new offer、pending Noq open/ears epoch的真实合同；这些与总体3秒门未证明前不并行寻路。不能以新deadline测试替代手机自动恢复或路径快慢选择。


## 2026-11-10 12:39UTC：先保护已在使用的载体，再收紧完整准备期限

8475/ffd/ceaa在Turing9484048e真实hop16pass/3fail：原15+正常短caller控制绿，gate内完整3秒及caller1秒两用例分别3.3016/1.3014秒仍未返回；真实新Incoming保持不admit时，caller1秒到期被记MISSED。123.098秒/11artifact/group空核，旧whole/prefix不变；原0b仅格式。完整by修复尚未写。

先补一个此前gate未覆盖的所有权边界：offer已返回而准备future尚未handoff时，另一个真实连接可能已使用该carrier；随后等待者取消不应拆掉共享路。否则收紧post-admission deadline或未来select普通dial先赢时，可能取消正被赢家使用的载体。现在是源码推断，需实际业务链路回归，不据此宣称现场原因。

拟新增post-offer cfg(test)单次gate，位置为实际ep.offer_addrs返回后/handoff前。独立真端点fixture使用owned Node身份key（不导出）、A的Links Custom transport、实际A↔scratch-middle Tunnel QUIC stream和B的Custom-only endpoint；middle只适配已接纳stream到B Links，不声称两个跨机腿或真实WAN。先让A→B真实TLS handshake、固定字节echo、path=Carried并adopt该live wire，再取消准备owner；载体/Asking应交给既有watcher、同wire第二次echo继续，正常handoff作负向控制。原整个offer文件仍是准确前缀，新prepare whole不改，先真实红。

候选方向是由实际live wire使用事实决定缓存所有权：PendingHop可持Asking/handoff sender，在取消时若Main/Tunnel/served真实live连接选中carried，则交既有watcher保留；没有使用者仍撤自己serial，保持原pre-offer取消合同。Wires连接收集与compare_carrier共用，字段读取不探测/不读磁盘/不新增timer；取消旧serial不得影响替代。并行实现以后必须先登记成功连接的事实，再取消其可能依赖的hop准备，caller过期仍不能交付/重放操作。此方向在新回归前不是已实现或已接受；如果实测否定则重新设计。

该保护完成后再合完整by和正确MISSED记账，之后仍需late-close/new-offer/pending-Noq-open/ears epoch合同，不能直接以一个tokio::select结束准备。


### 13:02UTC 已使用载体旧行为实测

a75/a038/871在56d2ca30实际hop17pass/4fail；真实TLS、首个固定echo和Carried断言均到达，取消后kept=false/owned=false/echoed=false，正常publication控制与原15通过，三个已知prepare期限红原样保留。126.342秒/11artifact/identity核/原group空。下一最小PendingHop持Asking与handoff sender，正常交付或Drop发现真实live selected-carried使用者时都把所有权交既有watcher；无人使用时仅撤本serial。随后同一批补单绝对期限，不改冻结wholefixture，完整net-node双OS。不是现场手机唯一根因证明。


### 13:09UTC 首候选保留的栈溢出回归

921/06db/23db在Turing a1b1dbe9新增hop21/0全部绿，整套继续到原leased IPC32出现tokio worker stack overflow/SIGABRT，248.432秒，原group空/identity核，后续profile未执行；不能接受该source。Mac42821367661同source仍在执行，不盲重发或提前视为绿。原fixture与原栈配置不改，先在hop_ahead与compare_roads两个实际冷准备调用边界Box::pin open_hop，避免把完整准备状态嵌进每个上层dial/resident请求future；不加task、不改栈限制或时钟。以原完整net-node原场景复验，尚未证明修好，不把此失败丢弃。


## 13:22UTC：incoming-only使用事实与selected标记不是同义

a35/95e/c74已双OS全12组绿，完成已登记selected-carried与完整准备期限合同，但源码中已有whole Wires真实fixture证明：同peer的一条outgoing Direct可作为公共selected tuple，而另一incoming只开Custom，后者snapshot无selected标记。它仍真实收发carried数据，不能被新Drop误当无人使用。需独立真实回归，不能改原whole Wires测试。

在既有published fixture尾附incoming子模块，保留当前整个文件准确前缀：同一个fresh B身份分别绑定IP-only和Custom-only端点；A先持实际outgoing IP wire，再完成真实hop、B→A Custom TLS/echo，A登记serving incoming且assert仅Custom、无selected，B客户端selected Carried，A outgoing仍selected IP。取消post-offer准备者后carried echo必须继续，Direct也继续；正常handoff作控制。所有身份仅scratch，原deadline/whole fixtures不改。旧行为真实红后，最小扩展使用事实为selected custom或非空且全部custom的live wire，空/未知snapshot不臆测使用，主compare逻辑仍不改。之后再close/new-offer代际，不以这个合同关闭其他边界。


13:29UTC incoming旧行为实测：a2ed/19ed/f118在07e04308 hop22/1红，真实TLS/echo/仅Custom无selected及对侧Carried/独立outgoing IP前置均达成；取消后kept=false/owned=false/echoed=false，但独立IP echo仍成功。正常incoming控制与原21全过，198.966秒/11artifact/identity核/group空。候选仅carrier_in_use扩展非空all-custom事实，不把空snapshot当作使用、不改comparison/时钟/fixture。先原完整net-node同source双OS；after-offer载体保留不等于所有新旧offer代际已验证。


## 13:37UTC：late-close/new-offer独立合同（先复现）

当前incoming候选69a6在Linux4c316093全12绿/hop23，Mac42825268619已出artifact待身份与原门核，不预记接受。下一原源码行为不变，仅cfg(test)单次gate在真实let_path_go检查links无连接之后、读取ears/发送close之前。独立scratch Node当前ears为真实Endpoint；A/B先实际IP握手/echo，再通过duplex frame流开真实Custom路径且有测量；断旧stream并join两端Attached后暂停old-close。replacement控制先建立新stream并实际offer同Custom tuple，随后release旧close。真实remote_info对同actor作队列处理barrier，观察Custom Closed事件/当前paths与仍可用IP echo。无replacement负控应真实关闭Custom且IP继续。原gate不进生产，不使用GUI/store或个人身份，不把内存流声称实际SSH/两腿/WAN。

若旧行为真实红，后续修复需处理完整顺序：新offer已完成时迟来的旧close不能赢；close已发但Noq Abandoned未回时的新offer不能被“已开着”吞掉；pending Opened旧完成和换ears也需合同。单独在异步close前再check一次links或只包应用层mutex不足以关闭这些矩阵。候选可考虑以Links单一所有者产生每peer单调revision，Iroh actor接收带revision的desired custom状态，并在实际路径事件后继续收敛；这是提议，不是已实现或已验证，不能新增无界task/probe或把网络epoch和remote Asleep混为一层。


### 13:46UTC revision方案设计备忘（尚未实施，等旧行为实际结果）

- 单一事实来自Links的实际提交；不要用当前Attached.serial直接作为按时间有序版本：现在serial在取得links锁之前fetch_add，两个线程的取号和commit顺序可以相反。新revision必须在同一links锁内分配，attach、匹配detach、当前serial的自然I/O结束才推进；旧serial结束不得推进新载体。
- 现有last map已经跨断线保留每peer carrier，可扩展carrier+revision；snapshot同时读取active map和last，固定links→last锁顺序。在现有map内保留事实，避免第三个无界表，且不在数据包热路径/磁盘读取版本。只为当前owner实际删除推进revision，不因旧Future完成/重复close推进。
- Iroh actor接收每CustomAddr的(version,available) desired state；老版本不生效，等版本相同状态幂等。不要简单warm ResolveRemote后无条件Close；取消在这两个await之间仍须有既有owner保证最终reconcile。应优先通过Socket actor单个带ack消息到RemoteStateActor，激活可按现有resolve创建actor，失活没有actor时不为无连接peer白建task。
- desired false要约束generic ResolveRemote、pending_open_paths、path-id等待者、Noq Established/Abandoned后的收敛；desired true在旧path关闭但Abandoned未处理时不能被conn_state“已存在tuple”吞掉。不能把last-open-path拒绝关闭当已关闭，不能靠新永久timer/probe修顺序。单次check或只应用层async mutex不能证明这些性质。
- 同peer新ears是新endpoint容器，重新同步当前Links状态；旧watcher迟到不能改变新的desired generation。载体generation、underlay实际路径变化的measurement epoch、本地IPv4/v6网络epoch、peer正向contact与authenticated resting事实分层，不能全局清缓存来混同。N1可研究underlay变化使inner质量窗口进入新epoch，但不得提前合未接受42–44、重置拥塞/放宽收益和2秒门、增加未计量包预算。
- 新vendor行为需独立actor旧红/新绿（late close/new offer、pending Noq开/关、actor重建/ears、无变化负控）与原whole Iroh/Noq合同，及真实Node全net-node双OS；之后才并行普通拨号和hop准备。Noq/API根因若证据否定该提议，先改设计，不把此备忘当实现结论。


### 13:53UTC 首个late-close合同实测后候选

8048/73ff/5e19在761c4291真实hop24/1红：新frame stream实际offer、Custom有测量后，旧close仍导致真实Closed、carried=false；无replacement确实关闭Custom且IP继续的负控与此前23全过，126.109秒/11artifact/identity核/group空。原fixture全部冻结。

第一候选先把条件检查放在实际RemoteStateActor执行关闭的点：新增close_carried_if携带仅检查共享Links的同步guard，既有close_carried保持无条件，复用同一个CloseCustom消息；actor执行前读最新has，变为存在则不关。guard只活到有界消息被处理，不新增task/timer/probe/版本表或IO，信任/关闭最后一条path原语义不变；按既有Iroh13扩展记录，不与隔离42–44编号冲突。这个最小合同尚未验证，且**不能凭它关闭“已排队的close之后才有新carrier”、pending Noq open/Abandoned、ears与网络epoch矩阵**；要再用真实actor排队窗口合同决定是否升级revision设计。前一revision备忘保留为提议，不提前落入实现。


### 14:02UTC 真实actor排队合同（测试已写，尚未运行）

close_offer原整个文件保持70af完整前缀，只尾附queued_close模块；新模块用scratch endpoint的透明RoadOrder wrapper一次同步暂停真实RemoteStateActor，其他选择/测量不变，RAII始终释放。旧frame结束后暂停actor，真实Node let_path_go已将close入队并join，而actor仍停在回调；随后才attach替代frame流，再释放actor/实际offer、用同actor barrier和真实Closed/paths/echo观察。无replacement控制应仍关闭。该gate不进入生产、不注入测量/时钟、不重放业务。为证明执行点guard而非提前check有效，需独立scratch baseline仅把Node调用恢复到原无条件close_carried，Iroh候选加法API保留但不使用；两份源新whole测试一致、原API None分支即旧关闭行为，先真实红再候选绿，不伪装成旧完整Git提交。原25全部继续覆盖。


## U17/NET-16 第一片：真实hop_ahead准入合同（14:29UTC）

新增published子模块sole_peer_tests，旧published整个文件保留为前缀。fresh Node仅一个实际Direct Tunnel邻居，目标无当前Main/Tunnel/Custom；两端仍声明有Relay配置，复现配置存在不等于可达。分别写middle第二腿Relay提示、不写提示、目标历史Direct提示，调用真实Node.hop_ahead，经实际offer gate/admission/inner TLS/固定echo判断是否尝试。已知Direct第二腿为正控、已被其他peer转发的第二腿为单跳拒绝负控，原27继续。中间stream到目标由既有fixture脚本适配，**不声称第二段真经过公共Relay**；真实两腿Relay场景仍属NET-16/N4。先实际旧红，再设计“没有当前目标可用线”的Recovery准入，健康已连通时的Measured性能比较保持；第二腿未知只允许有限真实准入，不信任其旧样本当作当前数据面证据。旧MISSED/LOST随候选/本地网络/地址epoch正确失效仍独立待做，不能靠每个包或全局清空来绕过quiet。


### 14:45UTC U17第一片实际29/3红后候选

102b/4d6512/f7d794在fe6ce1de实际32项29/3红：Relay提示、缺第二腿、历史Direct均admitted=false/echoed=false，真实已知Direct正控、递归拒绝负控和原27全过，140.253秒/11artifact/identity核/group空。原whole新旧fixture冻结。

候选新增Legs::Recovery：目标当前无live road时不以历史Direct否决；两端声明Relay不等于当前可用，因此允许可达Direct/Relay首段接Direct/Relay/SSH第二段；缺少/None第二腿提示可在原两middle/同一个3秒及更早caller预算内实际准入，仍拒绝via递归和paused/satellite等原门。实际连通目标已有road时保留原Dial，Measured比较不变；NoRelay/ToNoRelay原配置约束仍保持。旧MISSED/LOST/两条候选及所有时钟不改，无新timer/probe/task，不能宣称epoch已修。**此片只修准入，当前hop准备仍在普通拨号之前，须补并行寻路合同以避免扩展准入拖慢可用快路；未部署。** first-live与旧缓存候选排序、第二腿真实Relay数据面、网络变化使旧失败失效仍独立待验。


## 并行寻路的下一合同与所有权边界（15:19UTC，提议待实测）

先真Node.line_to_inner：唯一middle的真实Tunnel已读Ask但故意不回OK，目标另有真实可用IP endpoint；普通Main握手的accept在middle实际到达后放行，原caller1秒内应能返回Direct并固定echo，不能等middle或caller到期。取消输掉的准备必须观察真实STOP、外层Tunnel不关闭、ASKING/未交付carrier清理，原目标和middle身份均scratch。之后反向覆盖普通路失败但middle后来可用，原2个活跃普通候选与总预算不被阶段重置，无写/accepted早数据重放。

实现设计需让准备future留到成功连接被adopt之后再Drop；网络连接已完成但尚未adopt的窗口应有pending-use lease，而不是只查看已登记wire。可用per-peer弱lease＋关闭通知供既有hop watcher等候使用者结束；Caller取消且无wire即撤本serial，实际wire存在才交既有比较逻辑，不新增清理task/永久timer。跨Noq未登记使用事实、pending Opened/Abandoned和ears仍需各自合同，不凭已修条件close推定全部安全。

普通dial可把新carrier作为一次有界候选更新：复用同一个DialWindow和journal起点，最多2个活跃连接尝试，新carrier到达时回收/替换一个待定候选；全失败先等有限准备结果，peer Asleep/failed只在最终失败提交。原直接/identity两个候选可有一个新carrier重试（生命周期总次数需显式记录，不能把2活跃冒充2总次数）；没有业务帧时的重握手与已接受early请求的禁止重放必须分开。0-RTT路径仍须独立处理和验证，不复用普通连接重试去重发accepted操作。

2026-11-10T15:44:10.421147+00:00：ordinary并行候选WIP已按33/3真实红实现，仍须远端fmt/compile/合同/全回归。原2活动握手，允许所有普通尝试失败后新carrier补1次（总lifetime最多3，绝不能写2total）；原caller/dial window不重置。使用weak pending owner+watch关闭通知，现有hop watcher接交接，成功adopt在准备future Drop前。0RTT、旧Asleep先返回、pending state最后释放→新operation准入→旧watcher回收之间的原子性、freshness/address/network epochs仍未闭合；不得把本candidate当整包最优或可部署。

2026-11-10T15:55:07.266891+00:00：99d8ec5c Linux1446794a全12/hop36绿，Mac42849256788同源待结果；并行/未登记scope仅候选。新增successor fixture在旧pending完成→清理决策间真实gate，先完成后新建同peer lease并保持实际未登记carried TLS；确认旧watcher决策已执行后再echo，含无successor释放控制。7810仅测试点，下一步先取真实红，再把pending准入与精确serial撤回放到同锁原子检查；其余0RTT/Asleep/候选新鲜度/full N0–N5继续。

2026-11-10T15:58:10.645955+00:00：静态确认cached-first缺口：road_for_hop在无live road且未down时回退mine历史Road，middles_with_cost统一按两腿ms排序，未知ms=u16::MAX；因此两个历史低RTT候选可能挤掉当前唯一live neighbor（TRIES=2）。下一独立合同必须用真实两个未admit的cached endpoints与实际live Tunnel验证选择，不只测试排序函数；恢复按当前可达性与速度估计排序，健康路性能比较保持实测gain，不能偷换成Direct优先。当前尚未写fixture或改排序。

2026-11-10T16:17:29.340928+00:00：2cb56/06d07ccd actual hop39全绿，full/Mac待结果。读取路径仍ask_early串行hop_ahead，新增actual ask_on Sessions cold与TLS-resumed两例：原1秒caller、真实未admit middle、真实IP server返回SessionRows，服务端is_0rtt与真实计数（1 optional warm+1 actual），原race whole逐byte为prefix。先取旧行为红，后让准备与原early handshake合作poll；必须adopt先于准备Drop，已accepted response owner/STOP/不重放原合同保持。普通握手早失败→hop后成功、0RTT失败阶段与无隐式重放、相同绝对dial budget/journal起点的完整fallback仍需独立合同，不能假定一个好IP例覆盖。

2026-11-10T16:38:12.108690+00:00：读取v2把resumption witness改为真实client ZeroRttStatus Accepted/no-session/Rejected（只cfg(test) one-shot观察、不阻塞握手），原因是原accepted-answer fixture已记录server accept-time is_0rtt可被快握手清false。0f8两读取成功但warm marker false的40/1仍保留，不算0RTT接受；b154主与9b680隔离旧serial共用v2 fixture/observer待新红绿。后续P1合同拟用真实同identity错误ALPN direct端先产生client真实连接失败，再允许唯一middle转发；不只server错误，因为另一路可能已救成。只读cfg(test)失败事件可作时点证据，先失败后新hop应在同一绝对预算完成一个真实Sessions；cold未发early数据可安全启动fallback，已产生early request的失败阶段必须保守，不隐式重放。保留总window/journal起点、两active/生命周期上限与原accepted-owner/STOP合同。此后仍需initial PendingHop Drop与normal watcher释放的独立原子性红；这些是review明示未覆盖，完整N0–N5不缩减。

2026-11-10T16:44:04.567291+00:00：后续early fallback合同优先用真实换址case：目标旧IP现属另一个EndpointId（正确ALPN、不同新key），客户端必须真正收到身份校验失败（不能只看server事件）；收到client失败只读test one-shot后，才放唯一可信middle的转发许可。用实际Sessions响应/单次count、同一绝对caller budget和无route拒绝负控；原middle Ask在首失败前已实际进入。优先stale-IP/wrong-identity，避免把有意resting/paused的CONNECTION_REFUSED当可绕过。若Noq仍记旧IP妨碍新carried握手，保留真实红并按地址/epoch修，不把错误地址改为恰好静默或放大期限。cfg(test)失败事件只观察真实Err、不等待/替代错误。此fixture/helper尚未写。

2026-11-10T17:02:19.073476+00:00：stale-IP client真Refused→middle准入的旧263e5ce8实际42/1红，negative和原41绿。候选只对未曾发early task的cold Refused和初始无carrier补1次新carried普通握手；先等待已有hop准备，都沿同一dial_by，而不是重新调用dial重开10/25秒预算或另起两个ordinary候选。原started/journal一次；early_data在spawn前保守置true，之后失败不走fallback。当前常量DIAL_TIMEOUT=10s、DIAL_MOST=25s，以源码为准，早文档5/25只是错误口径，已不再用。若candidate仍被旧IP选择影响，保留红再查实际Noq路径，不放宽原2秒caller。


## 2026-11-11：当前批之后的恢复合同

153263c2的cold错误身份→peer relay接管已在同canonical双OS全12/hop44验证；801dff9e的initial Drop/normal release消费者保护仍候选，实际回执查STATE。以下是下一合同，不写成已实施或手机事故根因。

1. **仅入站可达的middle**：Wires::link/live只查看本机adopt的outgoing slots；serving单独保留incoming weak handle。road_for_hop和Recovery live排序因此可能只看到旧roadbook。真实fixture先只建立middle→本机的认证连接、登记serving且不adopt，保持实际字节往返，再让本机在原总预算中经该middle救通目标；另留incoming carried/recursive、关闭/陈旧和错误ALPN控制。不能把任意incoming连接直接充当客户端Tunnel：出站端未必运行接收请求的serve_tunnel。可利用已验证的入站路径事实选候选，再建立原协议要求的有限Tunnel；只有实际双向服务合同成立才复用连接。按实际测量排成本，不恢复Direct硬优先。
2. **本机网络代际与旧完成事件**：Wake::Network(Addrs)目前只forgive_all，其余Path/Wake/resume调用network_changed；hop的MISSED/LOST与这些状态分离。先真实建立一次NO_ROUTE/miss，再在未满原冷却时发实际换网入口/恢复事件、让同一middle许可，要求新尝试能成功；同时在旧尝试仍未完成时换网，旧失败不得在新网重新写冷却。不是无条件清空所有全局表，也不能让每个普通ACK刷新代际而取消后台退避。
3. **peer/middle IPv4与IPv6改变**：身份保持，区分地址变化、仅来源标签改变和网络变化，按实际候选/可信地址世代失效；需要单独v4、v6、同时变化、同地址来源改变控制。Iroh45的本地source时间边界只解决一次idle cold失败，不是签名地址世代，不把重新发现的旧IP当作永久禁用。
4. **正常watcher成本与更广admission**：两个新pending合同只覆盖有限调用owner。无pending的入站登记、正常准备完成但未使用carrier、unknown path snapshot与旧比较结果需各自实际合同；必要时扩同一个明确owner/admission边界，不能靠新常驻task、无限lease或减短LOOK让问题消失。仍测原N3字节/唤醒/能耗。

构建加速只读复核（18:36 UTC）：固定control68cad的network-ssh-runner.cached_source已用独占锁内stable checkout并保留未变路径/mtime，核tree/clean后复用target；并非每run重铺所有源码。维持jobs16与冻结test-threads/网络观察窗；当前以同source连续验证相关合同减少重复编译，不因CPU强而并发计时物理netlab。不改guard/control，未取得编译阶段更细profiling时不编造缓存瓶颈或性能收益。


### 冷却代际：下一真实红与设计约束（提议，未实施）

先补三个真实Tunnel合同：第一次Ask::To由middle回NO_ROUTE并真实计数；(a)无变化时原冷却仍挡第二次，保持成本控制；(b)第一次拒绝后调用Addrs分支所用forgive_all，原冷却内应可再次准备并carried echo；(c)第一Ask已进入但尚未回复时调用Path/resume所用network_changed，再允许旧NO_ROUTE完成，旧完成不得阻挡新网的下一次准备。用原3秒hop/2秒观察和owned cleanup，不能先清表再称旧完成安全。这里只验证实际核心入口，OS信号、独立IPv4/IPv6及手机仍归N4。

实现选择尚待上述证据：冷却context至少区分本机路由失效版本、peer有效新联系/地址信息、可用middle集合，并有一次尝试的顺序标记。RTT抖动、地址仅来源标签变化、普通ACK不应让冷却每轮失效。新middle从缓存变为真实可达可触发一次重新判断；不能永远等原30秒或每RPC重新广播探测。

若复用现全局MISSED，应将context与entry同锁比较，context持弱owner，不能让cache留住Node；旧失败/成功不能覆盖或清掉更晚context，实际写入前复核本次context仍有效。网络版本与尝试顺序是不同值，不能把每个新尝试序号放进相等判断而自动绕过退避。cache键和值均须有界，不能按每次换网永久增加一份记录。若移至每Node/Wires owner，应继续让原纯状态测试覆盖同一生产核心，不为保留旧测试建立无生产调用的平行shim。

LOST当前按(to,via)全局保存，未带本机身份，也与换网/候选变化分离；下一设计须同时明确其owner与何时重新比较。先闭合真实MISSED合同不等于LOST/全部地址代际已完成，禁止静默放宽原收益/确认和十分钟抑制规则。


19:07 UTC候选边界：本机MISSED首先实现原forgive_all入口更换唯一Arc<()> token，操作和cache仅存Weak，原全局每(me,to)一条仍有界；旧token无strong owner后不能写/清新miss，原纯退避测试经tokenless入口使用同一生产核心。当前尚未行为接受（真实55旧53/2红已取，下一run以STATE/ledger为准）。这只解决本机all-waits-forgiven，不替代peer/middle候选版本、LOST owner和全逻辑地址context。必须维持“仅Wires持有strong token”的不变量，读取者不能克隆strong来延长旧世代。

packet-age下一合同已加入：真实认证/echo使rx计数确实改变，只有cached reactor heard回拨；显式now快照仍应可达、显式future年龄过期则应不可达。旧谓词未改，先取红；这是快照年龄模型，不冒充OS暂停。现有Iroh Path.stats().last_received可直接读取真实Noq时间，拟只按当前选中IP/Relay路径的实际收包年龄与原Patience判断，避免把“自上次tick计数不同”当作刚收包。网络切换后的路径世代、OS睡眠时钟与用户手机仍独立待验，不以年龄模型覆盖。

真机候选构建入口复核：REMOTE-DEVELOPMENT明确便携Mac executor不运行native，开发机不生成正式iOS签名包，iPhone继续用可信GitHub TestFlight。现阶段无iOS安装/发布，未为测试修改该边界。可提前准备原生/iOS检查与准确源记录；最终TestFlight分发仍需届时明确授权及可信Quality，不能把网络unit绿当App产物。


## 20:24 UTC N2候选变化缓存：继续设计、未实现

当前hop_ahead在构造picked前检查MISSED，只有本机Wires hop_epoch变化会使旧miss无效。新live middle而自身网络未变仍可能被30秒门挡住；此为静态缺口，尚未有新真实红。下一合同必须先实际NO_ROUTE，再改变候选实际可达性、建立新Tunnel/carried TLS/echo；无变化/Rtt-only噪声不能解除原退避，旧候选未完成的失败不能覆盖新候选上下文。

设计倾向是按目标持有稳定的候选context，以eligible ID集合/确实可用的首腿/地址事实区分变化，排除排序与每个ACK/RTT变化；真实网络epoch仍保留。不能直接在所有adopt、正向RPC或每个routebook更新调用forgive_all，这会让在线middle对睡眠目标不断NO_ROUTE。新context由Wires唯一持强token，操作/缓存只Weak的方案仍需对目标数量、删除、并发和late完成取证；尚未添加字段或map，不是已批准的最终实现。LOST还缺本机identity namespace与地址context，不能把MISSED局部修复当整个缓存完成。


2026-11-11 NoRelay 缺提示恢复入口待验证

当前 no_relay 是 Egress 对中继可达性的观测标志。hop_ahead 先按本端或对端标志选 NoRelay/ToNoRelay，只有两端均未置位才按目标缺 live wire 选 Recovery。前两分支在 roadbook 没有第二腿记录时筛掉候选。五个新回归从真实入口验证三个标志组合的缺提示情况、已知 Direct 正控和递归拒绝控制，正在 macserver 执行。

ff743已复用已有 Recovery 分支，使目标缺可用 wire 的判断先于中继观测标志。已有分支负责未知第二腿的有限准入及 live middle 排序，实际第二腿由中间节点验证。现有 hop 开关、可信设备、暂停、单跳、两候选和总预算继续负责准入。同一五项fixture已在原行为57/3红后于Mac候选60/0及全12组通过，Linux结果待验。完整两腿 Relay 与移动换网仍由 N4 实际网络场景验证。


2026-11-11 05:46 UTC：五个真实候选变化合同已在92c530旧行为取得63/2红，原60与无变化/成本变化/旧失败晚回三控制通过。17346候选将候选context合入同一MISSED表，表唯一拥有Arc token，操作只持弱引用；不另加Presence字段/全局map或定时任务。候选ID集合+实际首腿可达性排序规范化，不含RTT和尝试顺序。原网络弱epoch继续独立保护本机换网；旧完成只可写其仍有效的context，成功删除同时使旧操作token过期。Mac42967869816实际验证待核。该窄合同不包括同ID新地址、LOST namespace/地址context与实际第二腿；还需审查多线程旧候选快照的登记顺序。原fixture未改，源审计与行为证据见STATE。


2026-11-11 06:22 UTC：candidate context缓存的首个五合同已在1d448/Mac42968959558实际全过，但新增旧picked晚登记合同66/1红。生产6e5在roster/roadbook读取前捕获monotonic观察序号与弱网络token，原MISSED锁内拒绝旧快照覆盖；两个生产入口均携带其原观察，cfg(test) open_hop保留旧四参适配到真实core。Mac42969959757准确源6e5/tree3ac48/snapshot12ef已实际全12/hop67/wires16/budget26/resume1绿，原fixture相对1d448不改，Linux待跑。尚欠成功made删除entry后观察高水位寿命合同，同ID新Tunnel与IP/Relayhint内容变化、v4/v6独立变化和LOST本机namespace/context。DeviceInfo.addrs是hint Vec<String>，dial_addr统一解析并合并nearby；后续应按有效地址内容而非来源/排列/RTT噪声构建事实。当前bool可达性不等于地址世代。完整N0–N5保持。


2026-11-11 06:44 UTC：6e5同源Linux52af3091实际全12/hop67/wires16/budget30/resume1过，与Mac42969959757准确同源。随后按原三轮netlab-hop矩阵1501a97c执行f4/f4s/f4i，无新参数。成功后两个旧selection合同在0a7（生产逻辑仍6e5）Mac42973738072验证中；格式only429728原件保留，fixture仅原样rustfmt修正。若实际红，候选方向是在原MISSED entry中count归零但保留最新observation，并换代candidate弱token以作废仍未完成的旧回答；纯backoff helper的None语义继续删除。不能加新map/timer或Node强持有，也不能让正常成功延续旧miss count。此为待实际结果的方案，不写成已实施/验收。还须实际覆盖同ID新wire与地址变化。


2026-11-11 06:58 UTC：成功后旧selection两实际合同在0a7/Mac42973738072为67/2红，均额外问旧middle一次；287仅made保留最新observation/candidates、count置0、原entry换candidate Arc使旧回答过期，None纯helper仍删除。相同whole37aed942在Mac42975178687实际69/0、全12绿，latest Linux待原netlab-hop三轮计时后跑。零count metadata按活网络peer-pair保留、弱网络过期后下一次注册清理，无新map/timer/task/Node强引用。接下来同ID真实新Tunnel、IPv4/IPv6有效地址内容/来源/顺序控制，以及LOST本机identity和相关context；不能由成功寿命窄绿关闭这些问题。f4i原17.7/31.6秒是中间节点重新可达后回peer的路径收敛，先已走Relay；后续补首个有效业务响应与收敛分解，原first-road日志oracle不作性能SLO。


### 2026-11-11 08:05 UTC：实际admission范围与并发提交边界

首腿换址候选1bd9692在Linux全部12组/73hop绿，Mac同源run42983120010已success、exact归档，identity和逐组核尚在完成。不将此当完整N2；d965→12cb仅添加两个实际admission合同并应用远端格式，生产不改，Turing f15d8284待旧行为。

接下来先按证据拆解两项：两次实际NO_ROUTE不应压制未试第三middle；第一middle等待status不应吃光整个3秒预算而饿死第二个实际可用middle。fixture不改缓存，走生产hop_ahead、实际Tunnel与carried TLS/echo，并观察输家STOP_SENDING和outer仍open，原73测试保持。

作用域设计候选：在原MISSED候选记录上保存每middle失败count/at，退避30秒→10分钟不变；仅实际尝试的middle受失败影响，admission先排除该context下仍冷却的middle再取TRIES2。新增候选/新真实wire可进场；RTT排序不能清掉失败。复用原network和candidate弱token防旧完成写回，成功仍保留最新观察并退役旧完成。真实hint变化/同wire移动/LOST身份仍是后续独立合同。此段为设计提议，尚无实现/候选结果。

不能直接把两个tunnel::dial同时select当成完整有界竞争：serve_hop Ask::From当前在回OK后立即attach target的同peer carried transport。两个middle同时获准可能在目标侧互相替换，即使initiator只公布赢家；输家取消时旧B可能仍在await其Onward，目标侧晚attach也须被约束。需要覆盖双OK/迟到OK/取消后晚到From等实际双middle合同，明确哪个端点拥有最终提交；先做每middle失败范围不意味着并发提交已安全。禁止以只测一个OK+一个挂起来宣布竞争完成。

避免无依据固定hedge延迟；总预算/候选数/任务和stream生命周期必须有界，并记录工作量与业务收益。可研究并行准备首腿、明确admission/commit阶段及向后兼容，依据实验选择；不在此文把具体协议/参数写成用户已批准。


2026-11-11 08:38 UTC：上文“每middle失败范围”已实现并在Turing真实第三middle救通（74/1，剩第一挂起红）；7d进一步实现原deadline公平分配和完整首帧receiver提交，Turing全部12组/hop76过、Mac同源success待逐组核。并行prepared候选8ecc已实现未验证：flat hop_staged只作hint，新hop-ready To/From双端正向协议识别，不信过期能力；最多2 future、原总期限一半保留legacy fallback，失败/取消不发布carrier，真实首帧才提交，保持加密与原TRIES2信任边界。三个真实流量合同覆盖挂起、双Ready单提交、stale hint。物理完整两腿/旧版本混合与成本仍待。此前“不能只select两个dial”风险由receiver首帧提交及双端新词识别共同处理，原旧红和分项证据仍保留。


2026-11-11 08:51 UTC：下一步comparison admission设计。当前compare_roads在任何实际事实之前受进程全局LAST_COMPARE/30秒阻塞，一个peer甚至不同Node会消耗彼此时间；原f4i回peer尾巴13.6–31.6秒尚未闭环。拟去掉全局gate，把比较时间放原MISSED每(this,target)记录，只有当前network/candidate弱token匹配时登记；相同事实维持原30秒成本门，实际网络/候选/首腿事实改变则同cache entry换context并立即可比。沿原resident beat与note_roads，不增map/task/timer，不把每次ACK/RTT读数当连接性变化，不变30秒周期值。made成功保留该比较时间；旧epoch/旧观察不能重置它。完整物理矩阵c2b此时正在跑，作为修改前准确基线；不要在其Turing计时期间构建。此段是设计待验，未实现/未声称解决全部尾巴。


2026-11-11 09:01 UTC：目标/中间设备有效hint上下文仍欠。源码实核contact_version只是正向contact，Wires::back每次heard都会递增，并非网络/地址epoch；不应纳入MISSED/LOST造成持续解除退避。effective hints应沿net::endpoint::dial_addr解析有效IP/Relay并合并nearby，规范化去重；不能因原字符串顺序/重复/无效项或来源变化取消冷却。目标hint变化影响原第二腿失败，须重置目标下相关middle；只有某middle有效hint变化则仅重置该middle，保留其他实际未变的失败和比较工作。读all/roster前的观察序号约束继续，不能用更新前的DeviceInfo覆盖较新上下文；禁止在MISSED锁内读取文档/await。

先将比较候选19cceb格式/单元与原C物理基线隔离冻结，再追加此合同和实现，避免把两组改变混作一个性能归因。源19cceb此时只提交独立Mac（Turing C矩阵在途），未验证。拟实际合同：同wire同本机network epoch，NO_ROUTE后真实同身份新IPv4/IPv6有效hint可以重新准入并完成carried TLS/echo；纯顺序/重复/无效项不取消冷却。新有效hint的真实TLS前置与物理完整第二腿仍分层记录，不用adapter代替N4。


2026-11-11 09:43 UTC：静态待真实合同验证的下一N2所有权问题：open_hop_in_epoch在status OK后调用made_in_epoch；旧token在made中只早退，但调用方继续到carry策略/attach。已有旧快照/旧failure合同不独立证明“旧ask的成功回复”也会被拒绝。优先补真实旧ask先pending、新middle完成真实TLS/echo并正常释放后才返回旧OK，要求不发布旧carrier/不误记成功，保留caller3秒与原fixture。释放后变体可避开现有低ID互让门掩盖风险；再覆盖仍有消费者和SSH竞争。若真红，提交需要在原路由所有者锁内原子验证context并发布，只有检查cache还不够；同时审查Links策略检查/attach是否有并发空隙。此段为静态风险与验收方向，不是已经复现或已修原因。先完成当前hint87合同和a73物理全9；不把该风险悄悄删出完整N2。


2026-11-11 11:36 UTC：原scoped握手候选963已Mac全12和vendor443/58/11绿，Turing四peer三轮首内容1.22–1.25秒/后续30请求全成功，但原连续性判据缺wire仍三红；原f4控制2059/2325ms超门、业务最长5.2/4.9秒也保留。下一测量须绑定实际Ls/Dir已完成响应的双方wire与stream，而非去重后的Path started/selected标签。新诊断仅环境开关开启时构造，不记录请求路径/响应内容，不加常驻状态/任务/计时器；normal在decode成功后，early仅在握手Accepted且响应成功后，服务端完整write+FIN成功后记录。fixture要求31事件/不同stream两端一一匹配、同wire、首响应与carriercommit早于M恢复、每次客户端成功时carried、两端单一外层serial无I/O结束/替换，未满足仍红；原1.5秒/10秒/30请求门不变。旧a653原红不可追认。此为待远端编译/真实验收的测量增强，不作行为修复或整体验收。

已存在carrier时仍欠独立initial-route机会：新准备分支的!had_carried保护使旧carrier可能仍被共享Relay初始选择遮住。拟在原两slot/原caller+拨号期限内保留genericDirect与显式Custom的有限竞争，early替代仍普通握手且先取消原早发owner，无写重放扩大；必须先补真实已有carrier/更快Direct与取消合同再实施，不把newly-prepared场景覆盖外推到这里。旧f4红也须按实际请求/路径/第一腿恢复时间分解，不能通过放宽门关闭。


2026-11-11 11:51 UTC：b1a物理前两轮的新业务wire/stream收据已能匹配，但第一持续样本两端Link条件失败而早停，不能外推30样本成功。首响应时只有Custom path且selected=false，现有Wires::carrier_in_use也处理此情况；先区分actual received wire、served/outgoing slot和cached settled word，不直接把v2判据放宽。静态已有carrier initial gap假设与反向Link新拨一致，新增真实四合同在旧活IP旁安装只提供Custom的同身份端点，分别只许Direct或Custom接受新handshake，用Node普通/early完整SessionRows验证原caller内首响应、路由、唯一请求与旧IP/既有carrier保留。先取test-only旧行为，负控/fixture需真实通过。


2026-11-11 12:21 UTC：旧573在Linux5df和Mac43017591561已到真实最终业务断言，两项answered=false/pathNone/requests0，快Direct负控及旧IP/既有carrier均过；原412清理失败不再称产品红。候选166已实现有限installed-carrier竞争，正在同源两端net-node，不提前接受。4个合同没有放宽。

下一独立N1/N4观测问题需正式决策：iroh PathStateSender.record_selected把peer共享preferred fourtuple投射到各connection，找不到则selected=None，即使这个已认证conn只有一个实际Custom路径并已完成数据。Noq1.3.0公开Connection没有selected-path getter；vendor/noq-proto paths.rs1085–1095规定没有Available时Backup承担数据，PathStats有每path frame_tx/frame_rx.stream和udp计数。根business-continuity-decomposition-v1.json三轮中的第三轮有31匹配实际业务流/同wire/单outer，但首回复唯一Custom且selectedfalse，v2原红必须保留。可选方向：修provider在共享preferred未落到本conn且仅一个已open/已承载数据路径时报告有效数据路；或另记录effective data route/每path流帧事实，保持共享preferred和实际传输概念明确。前者需要先查path_watcher现有空/多路/不存在preferred的合同，并以实际数据/Noq调度证明，不能把“有候选”伪装成已选；后者须冻结新观测版本而非直接宽松selected gate。无论哪一种，旧v2和当前166物理原门保留，不提前重记绿。先收166全12/四新合同，再做独立变更与同物理对照，不把元数据修正当真实stall/能耗解决。
