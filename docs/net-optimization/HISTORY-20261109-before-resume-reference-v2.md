# feat/net 当前状态与恢复入口

更新：2026-11-09 17:12 UTC（本地2026-11-10）。root串行，无子智能体。活动目标“完成计划”仍active；完整N0–N5未完成。速度/体验优先，个人可信设备网；没有合main、发布、中继采购或生产部署授权。

## 当前执行与下一步

- **195 held_precondition_v2已终态采证3/3**：`turing-20261109T165750Z-d0af45b2`，491.06s、原group清空。准确base467336c2/treecd986018/snapshot503781bc/control39633806。三轮实际连续hold10444/10460/10455ms后才注故障，原1500ms恢复/30秒稳定断言通过；73/73请求全ok，分轮23/25/25，最大4043195/3390784/3332251us。source/control/bundle/group/九诊断文件/hash核留。只校准正向hold，不冒称reset/timeout负例已跑；诊断非SLO，尚无同版本baseline/固定路参照。196 Mac同完整source root fmt/net66+1ignored保持。

- **190诊断已终态采证0/3**：`turing-20261109T163727Z-f72bdaf4`，663.76s、原group清空。准确base656e274e/tree70acbbc1/snapshota13dce7d/control39633806。三红分别：after-answer1558ms>1500ms；180s未达到carried前置、未进故障窗口；第三轮准备函数在双方当前owner已于t0前81.4/54.8s释放后仍返回，观察从Relay开始并停留。后者证明现有准备助手忽略until超时且没真正要求连续10秒hold，必须独立修正/记量测revision，不能假装有效的stalled-carried起点。40/40测量请求全ok，分轮21/0/19，第二轮零请求代表缺覆盖。九个三角色诊断文件完整；四次实际释放都匹配当前committed serial，不能归因189旧owner。full-round-timeline/current-owner-release-trace及全部报告已留，非SLO运行。
- **193 Linux已同源全绿**：`turing-20261109T164452Z-f0c1a5db`，219.72s、原group清空。base d077d020/tree49b1aeb7/snapshotb4eef0cc/control39633806，root fmt/net66+1ignored/hop10/wires11/journal29/Linux24（92.02s）全过；189整fixture逐字不变。与194完整source相同，proof stale-owner-red189-green193-194-fixture-identity.json；194 run refs双引用全批核；195已终态见首条。

- **194 Mac42658228868同193完整source已成功**：root fmt/net66+1ignored/hop10/wires11/journal29/portable20全绿（49.93s），source/transport/全部9 control blobs/hash已核。189整fixture逐字一致真实红→绿，当前owner释放控制保留；CLI全部实际fixtures与netlab.sh、net/path.rs均逐字不变。Linux193也已绿；不冒称N1/WAN/手机/能耗完成。

- 191为前版源仅stage、实际remote无execution/launcher，已superseded_not_launched；192同源Mac仅format红（observer一hunk），原patch应用得到当前193/194源。不得重发191或计产品失败。

下一步依序：193/194已完成准确源/全部子集/fixture核对；195实际正向校准已完成，下一步补N0同拓扑的automatic/relay-only/carried-only稳态参照，再按结果完善负例/全矩阵，保留原故障时长/1500ms/30秒稳定门；N1继续查当前owner release的quality/gain/confidence，与旧182/162区分，再跑hop/recovery/dynamic/full。N2还有close-vs-offer竞态、IPC剩余预算/取消等独立缺口。不能因合同绿缩掉完整范围。

## 本轮已确证的变化

**189真实旧watcher所有权反例**（Mac42656499764）：base656e274e/tree70acbbc1/snapshota13dce7d/control39633806；root fmt/net66+1ignored过、hop9过1红。生产两次losing后的release尾部只作逐句抽取，原策略/计时不改。两个当前owner Relay/Direct释放控制先过；旧serial1已ended，新serial2真实duplex actor存活、同via、未lost，实际旧release使alive_after=false/lost_after=true。whole fixture SHA `f620fc97b204ca6a2e78a45715e9b6205f026c8a4e9069ea4354b51590318c5f`留在189 fixture-sha256.json；完整source/control/transport/hash核留。wires/journal/dial因前项失败未到。

当前功能修复62cf1cb3（格式收口d077d020）只加Links::detach_attached：同一map锁内核carrier＋serial再删除；原kind-only detach行为保留。实际watcher只有成功移除当前owner才标lost；过期owner记hop_release_not_current。189整fixture逐字不变，原2×5秒/gain/selector/CC不改。193 Linux与194 Mac已按原整fixture同源转绿；这是独立真反例，不能冒称182/162因果已定。

**N0 v4记录已真实校准**：183`turing-20261109T160336Z-88cf4c78`，baseab2b939b/treef8336c08/snapshot4c56003b/control5c67b39，99.22s、真实Node ignored校准1/1。五请求10事件：ok2/error1/timeout1/cancelled1，独立consumer远端valid；SIGSTOP状态T实核、200ms timeout记录201053us、已poll后取消52273us、CONT后成功17367us。本机仅Python按实际事件11项完整性/负例检查通过。取消仅指caller future，不证明远端停止。184仅format红，185修正源Mac fmt/net66+1ignored绿。

v4为paths echo与hop typing保留所有started/finished、outcomes/原因/耗时；原30秒总预算不加长、内部重试仍一次logical_request；f4另记故障/观察实际时间点。旧v3遗漏的echo错误不可补造为零。N0最终SLO、固定好路参照、样本充分性仍缺。

**诊断采集186曾fixture红、修正后190可用**：186`turing-20261109T162719Z-9c283711`，base3bc6ba59/tree3253636f/snapshotcc34bd29/control39633806，458.77s，三轮在M诊断校验处失败。三个f4计算结果行绿，但末尾原断言未到，不能算原测试通过。73请求146事件完整且全ok；六个x/y文件各仅attach意图＋committed serial1，无释放/结束；M文件未在assert前复制。修正为先保存原日志再核mode header，splice-only M允许无端点载路事件；事件按角色单独核对。190九个三角色文件已实核保留。187同186源Mac fmt/net66+1ignored/hop9/wires11/journal29/portable20绿；188只有新合同eprintln格式红，无行为测试。

## 必须保留的基准与结论

- 共享消费者所有权178真实policy红→179 Linux24/wires11、180 Mac20/wires11同整fixture转绿。功能3948872d检查live Main/Tunnel/served incoming并去重，closed/expired不计，空快照保守；有Custom而不能证明快路可替代则保留，无消费者可释放。iroh共享peer selected FourTuple，但每connection open paths不同，selected marker可空而实际只Custom传输；不是每条conn独立selector。
- 182原hop三轮终态**8/9红**（1590.25s、原group无存活），准确source同179/180/181（base3948872d/tree3ffbc0f7/snapshotab6cbce2/control7534）。首f4从0.056s即Relay、未恢复；y→x同wire carried→relay5936ms且Custom消失，但旧fixture无t0。f4i3/3、f4s3/3（最长8.4/8.5/9.9s）不能抹去该红，不能作为本批owner修复因果证明。原162同wire撤回7347ms仍待定因，168诊断3/3未重现。
- 原版176与候选181固定核stalls原门均2/2，五实际CLI fixture＋netlab.sh/control/CPU64/allowed集合一致。22 CPU记录逐期实核身份、所有存活线程mask及两busy ticks进展；真实resident约275–280 threads。候选p1s最大728.3ms/18punch/4369包1641546B，对照537.2ms/3punch/2801包1211229B；单顺序pair、host load不同，不证明单补丁因果/整体提升。探测/流量增多须追查；旧echo丢error，仅稳定性门绿。
- F8 143真红→150 Linux21/151 Mac19；F9 tick/F10长Accepted取消158真红→160 Linux24/161 Mac20，df592原活性修复及健康Accepted控制保持。新的准确修复源须再次运行这些合同；Mac排除四项Linux namespace合同。

## 授权、环境与工作区

U10已明确：P=iPhone，A=macserver（Mac），B=Turing（Linux），两服务器天然异网。先推进macserver↔Turing真实跨网，再补手机；设备组合已知，不再等另找电脑。手机型号/iOS/网络等余项待采，不阻断独立服务器工作。真实跨网验收尚未执行，同机namespace不替代真机。

本机source-only：不运行Cargo/Rust/Node/Xcode编译、产品测试、formatter或下载可执行物。Git/Python stdlib/数据检查可用；控制Python检查只用stub。Turing为重型Linux执行器，Mac原生走macserver/khor-build。禁止installed policy修改、ensure-budget、个人agent/凭据转发、现装App/生产store/其他worktree操作。

| 用途 | 当前位置与身份 |
| --- | --- |
| 产品 | /Volumes/disk0/Khor/worktrees/current/khor-net，feat/net；实际HEAD/diff先核，最新功能62cf1cb3＋格式d077d020 |
| 控制 | /Volumes/disk0/Khor/worktrees/current/khor-build-net，feat/net，39633806已push；仅新增SSH-only netlab-hop-ownership，原timed profiles不改 |
| 旧基线 | khor-net-baseline detached6485，原六fixture WIP未动；source90126283/tree97040f31保留 |
| CPU基线 | khor-net-baseline-cpu-v3 detached6485，仅七fixture/helper变化，staged树49211b55/source31db523a用于176；前版186d/63043993也留ref |
| 证据 | /Volumes/disk0/Khor/evidence/net，source-refs.json与turing-next-matrix.json核实际；196 refs全批snapshot/tree核 |

Turing root /tmp/khor-netlab-feat-net-1110，owned单flock重型槽；Rust1.95/jobs16/nice10；50GiB准入/40GiB停止/90min预算。计时不并跑编译/其他负载，无sudo/宿主网络改动，user/netns隔离。SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking=yes。实际/proc cmdline＋terminal group核状态，观察超时不等于停止，不盲重发。

控制39633806新增诊断使用f4 filter单线程三轮、NETLAB_DIAGNOSTICS=lifecycle，native samples=off且初始化成功后header；link_attachment_committed在map锁内写入后输出serial/previous_serial，原link_attached仅代表意图。原1/full模式和所有timed gates保持。driver2/runner4共6 Python tests（含new profile/parser失败子例）是本机stub检查，非真实产品。

## 完整未完成范围（保持N0–N5）

- **N0部分**：exit-aware真实Node短校准174已4/4，原版fixed-core176完整原门2/2，候选181相同fixture完整原门对照2/2但探测增多；原p1s遗漏echo error须补完整尝试记录。最终各场景目标/容差/样本量与p99充分性仍要收口。失败/取消/超时不能从分布删除。原门改动须另记revision及理由。
- **N1仍红**：178共享所有权修复179/180已绿，继续追182新f4红及原162撤路原因（168三轮未重现）；准确修复源重跑hop/recovery/dynamic/full，慢直连/快relay及反向、稳定小噪声、恢复后不滞留/不回退。旧recovery候选6/6、基线5/6有host load变化，不能算单补丁因果。Iroh37/Noq2旧vendor scope绿只覆盖其合同，不能覆盖当前应用表现。
- **N2部分**：df592已有Linux24/Mac20预算/流/探针/共享确认合同。仍缺长工作与多阶段原总by、两middle合计3秒、dial输者/延迟候选资源和过期、独立后台预算、近期地址/重启、退避随机扰动/事件合并。无pending proof时resident tick silence/network与served入站独立close仍要核；长期多个quiet epoch/probe开销也未测。
- **IPC边界未贯通**：client与serve的期限起点不同，剩余预算/取消未跨进程传递；正常发送半边shutdown不是取消，不能把EOF当取消，也不能重放已发送写请求。普通读最多一次重试、Mkdir失答一次、Accepted/0RTT准入与信任边界保持。
- **N3部分**：Noq独立FIFO_v2模型校准绿只提供低层数据，Cubic优先p95约890→390ms而BBR3约450→612ms，不据此改CC。真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平、共享vs分连接、1/3/5设备后台、真实手机CPU/字节/唤醒/能耗仍需实际负载；不增加总资源伪造胜出。
- **N4未完成**：源配置显示北京HTTP/TLS同机、默认自家登记集中北京、香港无默认独立pkarr；不当实时部署事实。双端/不对称路径、冷暖发现、整机/登记/出口故障域与容量实测、替代路径及有测量支持的拓扑方案仍待。复用多登记/选家/继任，尊重only_mine/n0；不采购或生产部署。
- **N5部分**：48初步补丁（Iroh37+Noq11）仍需覆盖/保留退出条件，上游Noq1.4逐行为等价、Noq1/5专项与剩余边界缺；UPSTREAM静态hash不是运行证明。证据前不升级/删补丁。最终准确集成源可信Quality、原生Mac/代理/多机/iPhone与能耗验收仍缺，实验室绿不替代真机。


## 恢复与证据操作

完整范围仍以PLAN/REQUIREMENTS/ACCEPTANCE为准；RUNS/REPORT留原始演进。本页重写前逐字状态见HISTORY-20261109-before-scoped-owner-verification.md，更早沿HISTORY链；历史在途仅代表当时，恢复必须核实际句柄。

source-refs-audit-20261109-held-v2-196.json已批核196 run refs；195已终态，当前无在途。每refs/khor/net-evidence/<run>等于snapshot，:source等于tree，不能checkout/push这些本地refs。所有never-launched/superseded保留inputs/ref，不计通过/失败。

ops/submit-turing.py仅Git/Python/SSH：stage先锚ref，可--source复用pure metadata，--control显式39633806，--role先声明；默认control仍b998。launch前驱必须终态完整采证并再次核原group/flock/inputs。collector返回101也可能采证成功，看collection/report/group，不重复覆盖。ops/verify-mac-record.py仅GH/Git/retained artifact身份核验，不执行产品测试；分类仍须读实际日志。

Mac临时source/bundle分支只在终态、完整留证、所有权/SHA一致后清理；复用者不删原请求分支。外盘缺失先读恢复入口，不按摘要覆盖WIP；其他治理工作不接管。完整goal仍active；对用户最近粗估3–5开发日，受回归、真机与能耗影响，不是完成承诺。

## f4准备修正：正向校准已过

在193/194绿源之后，只改CLI netlab的f4_lab准备助手为held_precondition_v2：初次Y达到carried后，双方reported Forwarded需连续10秒；丢失即重置；原两次60秒等待改为合计120秒的绝对总准备期限（包含状态查询），过期明确报准备失败、保留全部journal/诊断，不注入故障。phase记录开始/持有/丢失/确证/失败。原故障窗口/请求30秒/恢复1500ms/30秒稳定断言不动。这是加强前置的量测revision，旧源结果不重写、不直接混入新版分布；reported path并非独立物理快路证明，仍需固定好路参照。196已证明root format/net66；195已编译并取得真实三轮正向hold与原恢复门通过，负向reset/timeout仍需独立验证。


## 当前新源码：hop_reference_v1，未远端编译/运行

新ignored reference子模块复用f4几何构建器（默认f4传relay_only=false，hold与故障体不改）：automatic、两端scratch既有peer-hop关闭的relay-only、配对后在两端namespace双向阻断唯一relay的carried-only。三节点均在线、不冻结M；候选可用性改变也会改变后台探测，不把它当纯公式的因果对照。准备120s总期限、稳定10s，Y→X/X→Y各30个原30s预算请求与200ms间隔、完整outcomes/n/p50/p95/max，首错保留且可终止该方向、n不足不通过。只提供稳态操作参照，非故障/能耗/SLO。数字与验证边界已先写ACCEPTANCE。

control614c266已push，仅新增SSH-only netlab-hop-reference，三case单线程三轮ignored，普通采样日志关闭、末尾必须解析attempts，任一轮失败仍继续其余轮；原profiles不改。本机driver2/runner4共6 Python test（含新profile两个子例）为stub验证绿，没有本机Rust执行。新的profile/fixture尚未远端编译、校准或运行。


197 Mac42663152585（basebb7504c7/tree139a305a/snapshot72e1aeb2/control614c266）只有新reference.rs排版format红，无编译/行为测试。source/transport/9 control blobs/hash已核；原patch应用后新准确源再验。原195 f4几何体（剔除仅reference启用的if分支）与完整hold/失败/返回体逐字同，proof hop-reference-default-fixture-extraction.json；源码提取不当运行等价证明。

N4只读preflight：macserver alias映射z13，严格known host/不转发agent；一次SSH banner握手timeout（real-network/macserver-preflight-20261109T172201Z.json），而GH Mac197正常完成，不能记主机离线。后续实际跨网可让Mac在guard任务中主动配对Turing新scratch节点，配对能力仅限临时网络、经私库请求传入，不复制个人token/密钥或生产store；无需Mac SSH作为前置。具体控制/身份/清理协议仍待实现验证，不冒称跨网已跑。


198`turing-20261109T172658Z-50f1621e` v1参照已完整采证**6/9**，587.30s、原group清空；准确basec85ed1b1/treefaee139c/snapshot2c321f78/control614c266。375请求完整配对：372ok＋3error，三次error均carried准备的首个请求，carried稳态0覆盖。Automatic/relay-only各3/3，两个方向每组每轮30个记录与原summary的n/p50/p95/max逐项一致；两端peer-hop=off和实际两张nft表已核。前两者都停在Relay，尚无candidate建立屏障，不能据此断定选择器最优/错误或聚合为产品成功率。reference-v1-calibration.json保留全部失败/分组；199 Mac42664955779同完整source root fmt/net66+1ignored绿，完整identity/control/hash核留。

当前工作副本为hop_reference_v2（尚未远端format/编译/跑）：120s总准备期限内，先在未切断Relay时完成双向bootstrap；automatic/carried等原journal中对正确M的成功hop开路事件（发在attach前，只证明已开始，还要靠后续真实请求验证）；carried此时才阻断Relay。三组均从同一起点至少等60s，再双向真实请求、连续稳定10s后测量。产品选路/gain/时间门与测量负载不改，v1全保留、不混入v2分布。control7598180已push改metadata为v2，driver2/runner4共6 stub tests绿；原timed profiles不改。


200 Mac42668241618（base6fa5b972/treef19337f9/snapshotbc6dc66d/control7598180）只bootstrap错误消息一hunk format红，无编译/行为；完整source/transport/9 control blobs/hash核留，原patch已应用。200 refs snapshot/tree双引用全批核，当前两端无在途。198 carried-bootstrap-journal.json已逐轮核：三次只见setup后约10020ms ask dial失败，没有成功hop开路事件，不能凭设备表等于载路已就绪；v2即针对这一前置缺口，不改产品策略。
