# feat/net 当前状态与恢复入口

更新：2026-11-09 14:20 UTC；root串行，无子智能体。活动目标“完成计划”仍active，完整N0–N5未完成。

**当前176th原版完整stalls在Turing运行，PID570285/child570324。** `turing-20261109T141037Z-b813a5b3`，原生产6485、仅七fixture/source helper变化；tree49211b55/snapshot31db523a/control7534，role=baseline。已核/proc实际runner和execution running/network-driver。fixed-core exit-aware v3，原p1s600秒＋p5、单线程1轮；运行中不并跑Turing编译或另一负载。此轮入场还须实际核CPU ID、每thread mask/CPU ticks，未预报通过。

177th Mac42632331558已终态采证：basec716c916/tree5ba04b34/snapshota1981daa/control7534；root fmt/net66+1ignored/hop9保持，wires10绿1红。新合同两条真实连接均echo过，但停在“incoming selected-carried”前置，未到ownership目标；journal/dial-budget未执行。不能称为产品清理红。

源码核对vendor/iroh `remote_state::apply_selected_path` 与 `path_watcher::record_selected`：**peer共享selected FourTuple，各connection只持自身open paths**；全局selected tuple未在该connection打开时，其watcher selected marker可为空。不同wire的paths差异不证明独立selector。v1未打印incoming具体facts，不能据此断言177实际marker值；v2改用真实echo、server仅Custom paths、carried-only client确选Custom＋outgoing确选IP的物理前置，等待snapshot最多2秒并留完整三方facts。ownership目标与sole-direct/closed-incoming两个控制不改，生产清理策略仍未改。

176 full baseline五actual fixture与prepared candidate逐字一致，见`cpu-v3-fixed-core-paired-fixture-identity.json`；新v2只动wires::tests，不动CLI计时fixture。旧基线WIP保留。176当前已实际记录CPU64/allowed_midpoint_v1准入及前2spike，两resident每次全部线程mask=[64]，两个busy CPU ticks持续前进；完整600秒/p5仍在运行，不预记通过。

**174th真实Node短CPU准入4/4绿；175th Mac仍仅format红，标准patch已应用。** 174`turing-20261109T140109Z-0973b7c9`，base86bb24cd/treecfcde850/snapshotad2fee6e/control7534，101.18秒含编译、四case41.38秒，完整source/control/group/bundle/hash采证。CPU89上真实Node561853/562203分别274/275存活threads，全mask=[89]，真实echo回答；492ms内两contender分别增加9/10ticks（100Hz），PID/start稳定。sleep-child与负例/解析控制保持。172线程退出导致的读/proc task status ENOENT由exit-aware完整实时线程快照修正；原短case除formatter布局外断言/负载不改。此绿不覆盖600秒完整p1s或整体体验。

175 Mac42631155176与174完整source相同，只有两个新CPU helper的format hunk，未编译/执行共享载路合同。原format.patch已应用。接下来的完整A/B另把CPU选择从harness PID取余改成真实allowed ID集合的固定中间项，并标allowed_midpoint_v1，保证同host/同allowed集合时同一核；实际admission仍核所有线程和CPU进展，原600秒/两busy/尖峰/门不改。新选择尚待真实完整对照，不回写174的CPU89为CPU64。

共享载路合同已格式化，生产清理策略保持；下一准确源Mac net-node先取得可重复实际红，再决定修复，不能把162根因预先归定。

173 Mac42629466427与172完整source相同，只有CPU新case的assert布局format红；无编译/多连接行为执行。原format.patch已应用。N1共享载路合同仍待实际红，生产清理策略保持；不能把格式红说成产品红。

**169th实际Node压力准入失败，170th原版对照从未启动，已superseded。** 169`turing-20261109T133049Z-d4827d09`完整采证，rc101，581.57秒；p1s在CPU准入处因无具体路径的ENOENT失败，没有admitted记录、未进入600秒测量；p5原门绿（最慢295.9/497.0/972.6ms）。不能视为产品性能红或压力通过。新补proc读错误路径与真实Node短准入case，先定位fixture；原压力/时长/门不改。170原版已stage的source/ref/input全留，不花计时轮盲重跑。171 Mac42627527851仅formatter红、无编译/行为测试，准确报告已采核，原format.patch已应用。

新的peer共享载路合同已经加入wires::tests，真实loopback直连＋同peer incoming carried两条连接分别传回字节，先断言实际路径与连接身份，再判断是否错误释放，关闭incoming后另有清理控制。生产清理策略未改；需准确源码实际行为红后才修复，不能预先认定162根因。

168th诊断已完整采证，**3/3绿，原撤路未重现**，691.75秒；9份三端日志共27生命周期事件：attach12、IO结束6、close6、attachment结束3，没有measured release/detach。serial1结束位于原有意离线阶段，serial2返回后未结束；不证明原162问题修复。日志开启，非SLO。下一步以可重复多连接合同验证“只看一条连接却释放peer共享载路”的假设，再据真实红/绿决定修复。

162nd原三轮hop已终态**8/9**，3/3、2/3、3/3，1688.88秒；只有第二轮f4i红。X同一wire从carried切relay，间隔7347ms，载路从候选列表消失；Y的两条聚合事件属于不同wire。原因尚未被生命周期事件直接证明，不能归为显示抖动或直接宣布排名算法错误。f4s三轮虽原门绿，最慢按键均9.9秒；极致体验仍未达标。

166th `netlab-cpu-check` **3/3校准绿**：两个owned sleep子进程模拟resident、两个真实busy loop在CPU14；两contender在305ms分别增加15ticks，100ticks/s，实际亲和/存活/负例/清理通过。**不是600秒p1s或真实Node负载验收。** 167th Mac`42622111828`同完整snapshot根格式与net66+1ignored绿；没有跑Node20或App。当前177个run refs=176终态/明确未启动＋176在途，另有两份fixed-core源准备ref；下一步提交v2 Mac合同。

完整范围：[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)。运行与原始结论：[RUNS](RUNS.md)、[REPORT](REPORT.md)。此前STATE逐字保存在[本轮前历史](HISTORY-20261109-before-cpu-calibration.md)和更早HISTORY文件；历史在途文字只代表当时，恢复以本页顶端及实际进程为准。

## 授权、环境与工作区

速度/体验优先，个人可信设备网。活动目标已授权完整N0–N5实施验证；没有合main、发布、中继采购或生产部署授权。用户U9明确优先Turing跑Linux；Mac原生沿macserver/khor-build。用户已补充U10：两台不同网络电脑即macserver（Mac）＋Turing（Linux），与iPhone组成真实验收矩阵；先推进两服务器跨网验证。仅手机型号/iOS/网络等余项待采，不再等待另找两台电脑。

本机source-only，不运行Cargo/Rust/Node/Xcode编译、产品测试、formatter或下载可执行物；Git/Python stdlib/脚本检查可用。控制驱动本机仅跑Python stub检查，没执行真实Cargo/Rust/namespace。重型任务远端，Turing固定Rust1.95、jobs16、原场景threads/repeats/门；Mac既有guard单槽/jobs2。不改installed policy，不执行ensure-budget，不碰现装App/生产store/个人会话，不转发个人agent或凭据。

| 用途 | 当前位置与版本 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；本批代码886b084c为CPU fixture校验＋opt-in载路生命周期事实，清理/选路政策未变；恢复核实际HEAD/diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`；7534d6e已push，新增SSH-only cpu-check与hop-lifecycle两个profile；Python驱动2＋runner4 checks绿；原timed profiles未改 |
| 原基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83；原六fixture WIP未改，旧source90126283/tree97040f31已锚；不得按摘要覆盖 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json177条不可变run ref，176终态或明确未启动/176在途；SSH run不是GitHub run |

Turing根`/tmp/khor-netlab-feat-net-1110`，每run在`runs/<id>`。只操作此任务owned资源，flock单重型槽，计时/诊断场景运行时不并跑编译或其他负载。无sudo/主机网络改动；user/netns隔离。50GiB准入/40GiB停止/90分钟预算；原工具pin/hash/可信source bundle不变。

## 近期准确证据

| 编号 / run | 源与control | 结果与限制 |
| --- | --- | --- |
| 158 / turing-20261109T115934Z-20aa3073 | base41c50ba7/treed0484c04/snapshot4767c09c/controlb998 | 原Linux21过，新F9/F10真实行为红，前置先过；完整report已采 |
| 160 / turing-20261109T120752Z-38c3c975 | basedf592ad7/tree3c8bc03a/snapshote30f6377/controlb998 | Linux24全绿，92.30秒；F9/F10红转绿，F7/F8与16秒健康Accepted保持 |
| 161 / Mac42614651338 | 与160完全同source/control | Mac20全绿，50.01秒；排除LinuxF7/F8/F9/F10；不是App/native acceptance |
| 162 / turing-20261109T121345Z-8ba7e561 | 与160/161完全同source/control，原hop三轮 | 8/9，第二轮f4i真同wire fallback，完整report/group/hash采核；f4s按键9.9秒仍缺口 |
| 163 / Mac42617636222 | base2b93e4f4/tree70b1f13e/snapshot1a334757/controlb998 | formatter模块路径解析失败，无编译/测试/format.patch；已显式path修正 |
| 164 / Mac42618570173 | base1983c71f/tree928a65f7/snapshot46854ed4/controlb998 | 仅format红，无编译/测试；原patch已应用 |
| 165 / Mac42621362590 | base1f2aa08f/treed3816224/snapshot86926734/control7534 | 仅format红，无编译/测试；原patch已应用 |
| 166 / turing-20261109T130143Z-4777e11d | base886b084c/treeed97f217/snapshot349bd7ba/control7534 | CPU短校准3/3，61.07秒含编译、校准0.50秒；不是真实Node/p1s/性能通过 |
| 167 / Mac42622111828 | 与166完全同source/control，request cc0f8421e7d948d1acace754b2807d0d | root fmt/net66+1ignored绿，完整source/env/transport/hash采核；仅net-unit |
| 168 / turing-20261109T130858Z-465046e7 | 与166/167完全同source/control | hop-lifecycle诊断3/3绿，27生命周期事件，无release/detach；162原红未重现、未解决 |
| 169 / turing-20261109T133049Z-d4827d09 | 与166/167/168完全同source/control | p1s CPU准入ENOENT、未进入测量；p5原门绿；完整采证 |
| 170 / turing-20261109T133425Z-7a9c0dfa | 原6485生产，仅七fixture/helper变化；tree186d3b60/snapshot63043993/control7534 | v3原版同条件stalls从未launch，因169 fixture准入失败已superseded，保留input/ref |
| 171 / Mac42627527851 | basebe31e983/tree42f87d24/snapshot601c3c9c/control7534 | 只format红，无编译/行为测试；原format.patch已应用 |

F9/F10 core helper与F8相对158逐字相同，见`f9-f10-f8-red158-candidate160-function-identity.json`。早前F8真实红143→150 Linux21/Mac15119绿链完整保留。旧hop154以及旧CPU130/131从未launch，已superseded，refs/input全部保留，不能计为运行红/绿。

## 当前发现与下一步

1. **先取得真实Node短CPU准入事实与多连接合同红**：169已采证，170不launch。CPU helper增加proc路径上下文，新增ignored真实两Node admission/echo/300ms CPU进展case，由独立cpu-check profile运行；sleep校准不能替代。Mac171原format.patch已应用，待新准确源net-node。168完整抽取已存`observations/lifecycle-complete.json`，不反写162。
2. **N1因果仍待**：162第二轮X wire139750384102992在1794227442646选carried（quality85.61ms/8samples/age141ms），1794227449993只余relay133ms；Y前后wire不同。X/Y open_hop之后约10秒的撤路与2×5秒清理节奏吻合，但只是时间关联，未记录触发原因。源码只以Main/Tunnel第一条连接比较却能关闭peer共享载路，是需核实的所有权假设；还要考虑stream结束及旧attachment干扰，不先改排序参数或放宽f4i门。
3. **CPU v3已校准，完整对照未跑**：从thread-self真实allowed CPU IDs选择，断言每个resident全部thread和两个contender亲和、exec/存活/CPU进展，开始/每尖峰/结束留结构化证据。admission后才开始原600秒/道路尖峰窗口，两个busy数量不变。三个校准case显式ignore，只由单独profile加--ignored跑，避免污染并行timed场景。169真实Node admission实际红，先以新短case查清；完整p1s与同fixture原版对照仍待；原基线WIP不得覆盖，可另建detached v3基线树并先校验源。
4. **当前观测代码不改清理策略**：Attached只增加serial观测；normal模式不构造诊断details，不写新journal human words；KHOR_NET_DIAGNOSTICS开启时才记。166证明Linux CLI/fixture可编译，167证明根格式/net66；168才验证真实生命周期记录。后续实际修复需新准确源/合同/完整矩阵。

CPU旧152/153原两门均2/2：候选0/0移动、最慢3.8秒、12打洞；基线1/0移动、732.4ms、5打洞，均零退役。p1s原门只要求每端最多一次移动/零退役，没有最慢交互或零punch门。它是持续CPU争用＋300ms道路延迟尖峰，非20次CPU暂停；旧taskset/busy有效性未被断言，不回写成v3绿，不宣称整体或单补丁提速。原对照`turing-shared-probe-stalls-comparison.json`。

## 完整未完成范围（保持N0–N5）

- **N0部分**：新CPU机制校准已过，真实Node/原版候选v3同条件完整对照、最终各场景目标/容差/样本量与p99充分性仍要收口。失败/取消/超时不能从分布删除。原门改动须另记revision及理由。
- **N1仍红**：先解162实际撤路原因（168三轮未重现）；准确修复源重跑hop/recovery/dynamic/full，慢直连/快relay及反向、稳定小噪声、恢复后不滞留/不回退。旧recovery候选6/6、基线5/6有host load变化，不能算单补丁因果。Iroh37/Noq2旧vendor scope绿只覆盖其合同，不能覆盖当前应用表现。
- **N2部分**：df592已有Linux24/Mac20预算/流/探针/共享确认合同。仍缺长工作与多阶段原总by、两middle合计3秒、dial输者/延迟候选资源和过期、独立后台预算、近期地址/重启、退避随机扰动/事件合并。无pending proof时resident tick silence/network与served入站独立close仍要核；长期多个quiet epoch/probe开销也未测。
- **IPC边界未贯通**：client与serve的期限起点不同，剩余预算/取消未跨进程传递；正常发送半边shutdown不是取消，不能把EOF当取消，也不能重放已发送写请求。普通读最多一次重试、Mkdir失答一次、Accepted/0RTT准入与信任边界保持。
- **N3部分**：Noq独立FIFO_v2模型校准绿只提供低层数据，Cubic优先p95约890→390ms而BBR3约450→612ms，不据此改CC。真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平、共享vs分连接、1/3/5设备后台、真实手机CPU/字节/唤醒/能耗仍需实际负载；不增加总资源伪造胜出。
- **N4未完成**：源配置显示北京HTTP/TLS同机、默认自家登记集中北京、香港无默认独立pkarr；不当实时部署事实。双端/不对称路径、冷暖发现、整机/登记/出口故障域与容量实测、替代路径及有测量支持的拓扑方案仍待。复用多登记/选家/继任，尊重only_mine/n0；不采购或生产部署。
- **N5部分**：48初步补丁（Iroh37+Noq11）仍需覆盖/保留退出条件，上游Noq1.4逐行为等价、Noq1/5专项与剩余边界缺；UPSTREAM静态hash不是运行证明。证据前不升级/删补丁。最终准确集成源可信Quality、原生Mac/代理/多机/iPhone与能耗验收仍缺，实验室绿不替代真机。

## 证据与恢复规则

先核worktree/branch/HEAD/diff、receipt/source/ref与实际进程，不以摘要覆盖WIP。所有`refs/khor/net-evidence/<run>`必须同时等于账本snapshot，`:source`等于tree。最近全量审计`source-refs-audit-20261109-real-resident-admission.json`（173条）。不checkout/push这些本地证据refs。

辅助编排`evidence/net/ops/submit-turing.py`仅本地Git/Python/SSH；stage可--source复用完整回执，--control显式选新profile的7534（默认仍b998），launch必须给已采证前驱并重核原group/flock。helper历史及SHA保留。观察超时重查同句柄，不重发。169终态、170 superseded未启动，当前无Turing重型任务。174短校准已结束，175仅format红；接下来完整基线CPU与Mac合同检查按新回执恢复。helper新增--role在stage之前准确记录baseline/candidate/diagnostic；原guard未改，旧helper/SHA归档。

Mac source/bundle分支只在终态、完整留证和所有权/SHA匹配后清理；不删复用者不拥有的原源分支。外盘缺失只从Git只读恢复，原WIP不可按摘要覆盖。其他代码治理任务沿中央STATE/RESUME，不接管其树。

最近对用户工期粗估仍是阶段对照/切路结论1–2小时、完整N0–N5约3–5开发日，取决于回归与真机条件；不是缩范围或固定承诺。前一goal turn为progress（CPU校准/驱动/实际hop红/诊断在途），完整goal保持active，无合main/发布/部署。
