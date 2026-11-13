# feat/net 当前状态与恢复入口

更新：2026-11-09 16:29 UTC；root串行，无子智能体。活动目标“完成计划”仍active，完整N0–N5未完成。没有合main、发布、中继采购或生产部署授权。

**182th原hop三轮已终态采证8/9，N1仍红。** `turing-20261109T145455Z-205ad096`，原准确source3948872d/tree3ffbc0f7/snapshotab6cbce2/control7534，总1590.25秒、rc101、原group无存活。三轮2/3、3/3、3/3；首轮f4从0.056秒即Relay且未恢复，m腿3.7秒听见、最长请求4.9秒、结束Relay193ms。f4i三轮8.9/22.7/19.6秒回carried，无旧回退；f4s最长8.4/8.5/9.9秒。所有raw/source/control/inputs/hash核留。首轮完整失败journal见observations/first-f4-red-all-journal.json，y→x同wire由carried转relay相隔5936ms，且carried候选消失；旧source无准确t0，不能断言发生在故障窗口或归因旧watcher。下一步低量生命周期诊断，原门不放宽。

181`turing-20261109T145440Z-735ffd55`（同179/180准确source）已采证**原稳定性2/2**，1190.05秒；22条CPU64准入/20spike/end、身份/所有线程mask及两busy逐期进展实核。与176同完整fixture/control/CPU/allowed集合：p1s均1/0移动、零退役；候选最慢728.3ms、18punch/4369包1641546B，对照537.2ms、3punch/2801包1211229B。p5三档都零relay/punch。单个顺序pair不证明单补丁因果或总体提升；候选更高探测/流量是必须追查的信号。comparison在fixed-core-original176-candidate181-comparison.json，旧原echo错误遗漏不能推零失败。

**N0 v4真实节点校准183已过；184 Mac仅format红，185修正后fmt/net66已绿。** 183`turing-20261109T160336Z-88cf4c78`准确baseab2b939b/treef8336c08/snapshot4c56003b/control5c67b39，99.22秒、ignored real Node case1/1；五请求10事件完整配对：ok2/error1/timeout1/cancelled1。实际peer SIGSTOP+状态T、200ms期限记录201053us，已poll未来取消52273us，CONT后成功17367us。独立consumer在远端校准valid；本机仅Python读取实际183事件的11项完整性/缺失/重复/未知revision/错context等检查通过，非本机产品测试。

v4共用netlab/attempts.rs记录paths echo与hop typing逐次started/finished及全部outcomes，原30秒外层预算不加长，内部重试仍一logical_request；f4保留故障/观察准确时间点。取消只证明caller future结束，不代表远端停止。旧v3日志不可补出失败分布。184 Mac42651079158同完整source/control，只有一个assert重排format红，未编译/测试；完整transport/source/9 control blobs/hash已核，原patch应用后185 Mac42654241724新源base2747e8d1/treeb461bc39/snapshotfa2cbdd7/control5c67b39已root fmt/net66+1ignored绿。控制5c67b39已push SSH-only measure-check，原timed profiles不变；Python driver2/runner4为stub检查。两端当前无在途heavy job。

**共享载路178真实policy红已按原整fixture转绿：179 Linux24/wires11、180 Mac20/wires11。** 179`turing-20261109T144404Z-49c374ac`总219.77秒、dial contracts91.99秒；180 Mac42637760137同完整snapshot。两端root fmt/net66+1ignored/hop9/wires11/journal29保持，Linux24含四namespace合同、Mac20排除四项，完整source/control/transport/group/hash采核。178整v2 fixture SHA逐字同，hop.rs和net/path.rs也逐字同；五CLI timed fixtures及scripts/netlab.sh与176基线逐字同，证据`shared-owner-red178-green179-180-fixture-identity.json`。这收口一个释放判断反例，尚非三机/WAN/App/手机或原162因果证明。

178 Mac42634473431，baseb9e7d4a3/treef7a24347/snapshot0872b5bb/control7534：root fmt/net66+1ignored/hop9保持，wires10过1真policy红。两真实连接都echo过，outgoing选IP，incoming只有Custom且selected=false，carried-only client选Custom。不同wire IDs、sole-direct允许释放与incoming关闭允许释放两个控制先通过，最后“live incoming仍用carrier不得释放”目标失败。完整身份/transport/control/hash采核；test整段SHA `3a03d41a0cafcbf0e061e072db2a4a424d9fef876d6b99a3f60b47da05ad77b0`见`shared-owner-red178-function-sha256.json`。**这证明一处共享资源判断错误，不证明162原f4i因果已定。**

已过179/180合同的修复只改Wires::selected_beats_carried：检查live Main、Tunnel及served incoming，去重stable_id，closed/expired不计；任一有活跃Custom且不能证明有可替代快路就保留共享carrier。没有Custom的非空path集合不占用该carrier；空快照保守保留；全无live消费者可释放。仍用原path::selected_beats_carried的质量/gain，原2×5秒门、selector/CC不变。诊断新增shared_comparison（all_live_connections_v1，release与各wire facts）；178整fixture和五CLI timed fixtures均逐字不变。179/180已证明该合同红转绿；181旧稳定性原门已过，182原hop8/9仍红；不将合同绿扩大成整体体验绿。

176 turing-20261109T141037Z-b813a5b3，原生产6485/tree49211b55/snapshot31db523a/control7534，1189.88秒，原stalls两门2/2。CPU64（allowed_midpoint_v1）共22条admitted/20spike/end，实际610846ms；两resident各275–280存活thread全部mask=[64]、PID/start稳定，两busy每期CPU ticks严格前进。p1s alpha1/beta0移动、零退役、最慢537.2ms、3打洞；p5三档零relay/打洞、最慢273.8/501.0ms/约1.1s。所有report/group/source/driver/bundle/hash已采核。**p1s原门仅稳定性、原代码丢弃echo error值，不能宣称完整失败分布或延迟SLO绿。** 181候选完整原门对照已留，但失败分布仍缺，探测成本更高待追。

完整范围：[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)。原始运行：[RUNS](RUNS.md)、[REPORT](REPORT.md)。本次重写前STATE逐字保存在[历史](HISTORY-20261109-before-shared-owner-fix.md)，新增校准前快照见[历史](HISTORY-20261109-before-attempt-calibration.md)，更早状态沿HISTORY；历史在途只代表当时，以本页及实际进程为准。

## 授权、环境与工作区

速度/体验优先，个人可信设备网。U9优先Turing Linux；U10明确两台异网电脑就是macserver（Mac）与Turing（Linux），加iPhone为P/A/B。先做macserver↔Turing实际跨网，再补手机；物理矩阵已确认，尚未执行真实跨网验收，不能拿同机namespace代替。仅手机型号/iOS/网络等余项待采，不重复问已确认设备，也不阻断服务器独立工作。

本机source-only：不运行Cargo/Rust/Node/Xcode编译、产品测试、formatter或下载可执行物。Git/Python stdlib/脚本检查可用；本机控制Python检查只用stub。重型任务远端；Turing Rust1.95/jobs16/nice10，原profile线程/重复/门；Mac沿macserver/khor-build既有guard单槽/jobs2。不改installed policy，不执行ensure-budget，不碰现装App/生产store/个人会话，不转发个人agent/凭据。

| 用途 | 位置与恢复注意 |
| --- | --- |
| 产品 | /Volumes/disk0/Khor/worktrees/current/khor-net，feat/net；核实际HEAD/diff。本批功能3948872d已有179 Linux24/wires11、180同源Mac20/wires11；最新HEAD可仅含文档更新 |
| 控制 | /Volumes/disk0/Khor/worktrees/current/khor-build-net，feat/net，5c67b39已push新增measure-check；原7534及timed profiles保留，Python驱动2/runner4 stub检查绿 |
| 旧基线 | khor-net-baseline detached6485，原六fixture WIP未动；immutable source90126283/tree97040f31保留 |
| 当前CPU基线 | khor-net-baseline-cpu-v3 detached6485，仅七fixture/helper变化，staged树49211b55/source31db523a已用于176；前版186d/63043993也留ref，不覆盖旧WIP |
| 证据 | /Volumes/disk0/Khor/evidence/net；source-refs.json185个run refs，184终态/明确未启动，snapshot/tree双ref全批核；另有fixed-core源准备ref |

Turing root /tmp/khor-netlab-feat-net-1110，每run新目录/owned资源，flock单重型槽；计时场景不并跑编译/其他负载。无sudo/宿主网络改动，user/netns隔离；50GiB准入/40GiB停止/90分钟预算，原pin/hash不变。SSH BatchMode、ForwardAgent=no、StrictHostKeyChecking=yes。collector终态核原group后留完整报告；101也可能采证成功，不能把采证rc当作未下载。

## 关键证据与下一步

- N2 df592：158真实F9/F10红→160 Linux24/161 Mac20绿（相同core helpers/F8逐字），旧F8真实143红→150/151绿保留。当前所有权修复必须新准确源重验这些合同。
- N1 162原hop三轮8/9，第二轮f4i真same-wire carried撤回，X相隔7347ms、carried候选消失；Y前后wire不同。f4s旧门过但三轮最慢按键9.9秒。168 diagnostic3/3未重现，27生命周期事件无measured release/detach，不能据绿收口。
- CPU169实际Node admission ENOENT、未进600秒窗口，170从未launch已superseded；172真实短case定位消失task status，174 exit-aware修复4/4（CPU89），176固定CPU64完整原门2/2。166只是sleep-child3项校准，不回写成Node验收。
- Mac171/173/175仅format红，patch已应用；177 fmt/net66/hop9过但incoming selected-carried前置红，目标未到；178 real_connections_v2全物理前置/两清理控制先过，再policy真红。
- **API事实**：iroh peer共享selected FourTuple，各connection有不同open paths；record_selected无法在某conn找到共同tuple时该watcher selected可为空，仍可经可用Custom收发。不能将per-connection paths误说成独立selectors。v2同时证明server只Custom、client确选Custom与真实echo，且留三方rawfacts。
- 179/180准确源net-node已绿；181与176对照已留更高punch信号，182原hop三轮8/9且首轮f4红；再按结果定因及recovery/dynamic/full。比较前核五CLI fixtures、CPU ID/allowed集合、control/线程/重复一致；同host仍需记录load，不宣称单补丁因果或整体最优。
- p1s丢弃echo error值是现有量测缺口，旧绿仅稳定性。已写v4尝试/失败/取消/超时记录与独立真实校准，183真实远端校准已过，184只format红待修正重验，另记measurement revision，不能从旧日志伪造零失败；保留旧原门与所有失败。

## 完整未完成范围（保持N0–N5）

- **N0部分**：exit-aware真实Node短校准174已4/4，原版fixed-core176完整原门2/2，候选181相同fixture完整原门对照2/2但探测增多；原p1s遗漏echo error须补完整尝试记录。最终各场景目标/容差/样本量与p99充分性仍要收口。失败/取消/超时不能从分布删除。原门改动须另记revision及理由。
- **N1仍红**：178共享所有权修复179/180已绿，继续追182新f4红及原162撤路原因（168三轮未重现）；准确修复源重跑hop/recovery/dynamic/full，慢直连/快relay及反向、稳定小噪声、恢复后不滞留/不回退。旧recovery候选6/6、基线5/6有host load变化，不能算单补丁因果。Iroh37/Noq2旧vendor scope绿只覆盖其合同，不能覆盖当前应用表现。
- **N2部分**：df592已有Linux24/Mac20预算/流/探针/共享确认合同。仍缺长工作与多阶段原总by、两middle合计3秒、dial输者/延迟候选资源和过期、独立后台预算、近期地址/重启、退避随机扰动/事件合并。无pending proof时resident tick silence/network与served入站独立close仍要核；长期多个quiet epoch/probe开销也未测。
- **IPC边界未贯通**：client与serve的期限起点不同，剩余预算/取消未跨进程传递；正常发送半边shutdown不是取消，不能把EOF当取消，也不能重放已发送写请求。普通读最多一次重试、Mkdir失答一次、Accepted/0RTT准入与信任边界保持。
- **N3部分**：Noq独立FIFO_v2模型校准绿只提供低层数据，Cubic优先p95约890→390ms而BBR3约450→612ms，不据此改CC。真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平、共享vs分连接、1/3/5设备后台、真实手机CPU/字节/唤醒/能耗仍需实际负载；不增加总资源伪造胜出。
- **N4未完成**：源配置显示北京HTTP/TLS同机、默认自家登记集中北京、香港无默认独立pkarr；不当实时部署事实。双端/不对称路径、冷暖发现、整机/登记/出口故障域与容量实测、替代路径及有测量支持的拓扑方案仍待。复用多登记/选家/继任，尊重only_mine/n0；不采购或生产部署。
- **N5部分**：48初步补丁（Iroh37+Noq11）仍需覆盖/保留退出条件，上游Noq1.4逐行为等价、Noq1/5专项与剩余边界缺；UPSTREAM静态hash不是运行证明。证据前不升级/删补丁。最终准确集成源可信Quality、原生Mac/代理/多机/iPhone与能耗验收仍缺，实验室绿不替代真机。

## 证据与恢复规则

最新全量审计source-refs-audit-20261109-attempt-calibration-184.json（184 run refs）。每个refs/khor/net-evidence/<run>必须等于账本snapshot，:source等于tree，不checkout/push这些本地refs。CPU固定核fixture身份见cpu-v3-fixed-core-paired-fixture-identity.json；178 fixture基线SHA见shared-owner-red178-function-sha256.json。后续准确候选需再次核同fixture/hash。

辅助编排evidence/net/ops/submit-turing.py仅Git/Python/SSH：stage可--source复用完整snapshot、--control显式7534、--role先标baseline/candidate/diagnostic；默认control仍b998。launch必须已采证前驱，再核原group/flock/输入hash。helper新增measure-check白名单、旧版/SHA保留，观察超时重查同句柄不重发。当前Turing队列为空；182/183与Mac184已终态采证，local exec sessions都已完成。

Mac临时source/bundle分支只在终态、完整留证、所有权/SHA一致后清理；复用者不删原请求分支。外盘缺失只读Git恢复，不按摘要覆盖WIP；其他治理任务沿中央STATE/RESUME，不接管其树。完整goal仍active；工期暂估数天、最近对用户粗估3–5开发日，取决回归与真机，不是完成承诺或缩范围。

## 当前诊断准备（185后，未预记结果）

185 Mac修正后的准确源已采证，后续工作副本仅增加低量lifecycle诊断：native samples=off且初始化成功后写mode header，lab保留header＋实际lifecycle事件；原1/full模式与timed profiles不变。Links.attach新增map锁内link_attachment_committed（serial/previous_serial），原link_attached仍仅代表意图，不按它推断挂接完成。f4在此模式即使通过也留全部六方向journal，v4故障窗口/逐次请求保留，所有原门不变。尚未编译/运行这一批source。

control39633806已push新增SSH-only netlab-hop-ownership：原f4单线程三轮，lifecycle模式，非SLO；所有轮即使失败也继续，并在结束解析所有请求，parser非零不放过。Python driver2/runner4共6 test（含新profile两个子例）为stub检查绿。下一步冻结新源，Turing诊断与Mac格式检查；不盲改退役计时。


186 Turing`turing-20261109T162719Z-9c283711`低量f4诊断已launch PID753802/child753843，实际/proc/driver核live，56.30秒编译后第一轮。准确base3bc6ba59/tree3253636f/snapshotcc34bd29/control39633806。187 Mac42655499657与186完整source相同，net-node已dispatch，实际headSha39633806/in_progress，未预记结果。Mac首次手工import因缺scripts路径在upload前本机失败，note已留，未调度/不计产品红；加scripts到sys.path后才生成187实际run。

后续工作副本正补N2旧watcher世代合同：只把原两次losing后的release块逐句抽成实际helper，原kind-based detach与lost顺序保持，策略/计时不改。新测试先证明当前Relay/Direct owner仍可释放，再用真实duplex挂接旧/同via新两代、等待旧actor ended与新actor确活，调用实际旧release尾部，要求新代不关闭且不错误抑制middle。尚未远端编译/跑，不能预记行为红或182因果。


187已终态成功：准确186同源Mac root fmt/net66+1ignored/hop9/wires11/journal29/portable20（49.95秒）绿，完整source/transport/control/hash已核；不覆盖之后新合同或CLI模式。188 Mac42655862010（basef17f07e1/treea56bdd85/snapshotbefe92e5/control39633806）只有eprintln一hunk format红，无编译/测试；原patch已应用。

186前两轮原f4恢复门绿，但诊断采集校验因M没有端点载路事件而失败；不可记整个任务绿或产品红。M是splice角色，无此事件本来合法。x/y已各核真实committed serial1、没有释放/结束；第3轮仍在跑。工作副本修正为所有resident需正确mode header，实际事件逐角色核对，先保存原始serve日志再assert。原故障/准备/恢复/时长/门不变，当前源待新准确运行。


189 Mac42656499764真实policy红（base656e274e/tree70acbbc1/snapshota13dce7d/control39633806）：root fmt/net66+1ignored保持、hop9过1红。当前Relay/Direct owner释放与cooldown两个控制先过；旧serial1已ended，新serial2真实duplex actor活着、同via、未曾lost，最后实际release尾部使alive_after=false/lost_after=true。whole fixture SHA f620fc97b204ca6a2e78a45715e9b6205f026c8a4e9069ea4354b51590318c5f已留；完整source/transport/control/hash核留，wires/journal/dial未到。

当前修复只加Links::detach_attached：在同一map锁内核carrier＋serial后删除，原kind-only detach行为保留。实际watcher release只有成功移除当前owner才标lost；过期owner记hop_release_not_current并返回，原测试逐字不变、2×5秒与质量条件不改。尚待新准确源验证。这是独立真实资源所有权反例，不冒称182/162因果已定。

186已终态采证458.77秒、3次fixture红：三个原f4结果行绿，但M诊断assert在末尾原恢复断言前，原断言未到，故不能计原门通过。73请求146事件完整且全部ok；六个x/y文件各仅attach意图＋committed serial1，无IO结束/释放。M原日志未复制，这是已修的采集缺口。190`turing-20261109T163727Z-f72bdaf4`已launch PID774320，用189完整source（只有释放helper抽取，无本批所有权修复）/control39633806，修正splice日志后独立三轮非SLO；准确源已固定，不受当前修复影响。


192 Mac42657406850与191 staged源完全相同（base62cf1cb3/tree47413f50/snapshotf6a5c0e4/control39633806），只有observer helper一hunk format红，未编译/行为测试；原patch已应用，189整fixture仍逐字不变。191 Turing从未launch，已实际核remote无execution/launcher并标superseded_not_launched，inputs/ref保留；不浪费一个重型槽重跑已知format红。下版新准确源重新验证。

190修正诊断首轮出现真实f4红：恢复后1.6秒超过原1.5秒门，最长请求5.0秒，没有后续回退；各resident日志均已保存，末尾原断言到达。仍在后两轮；门不放宽，不能与186采集错误混为一类，亦不归因尚未包含的新owner修复。
