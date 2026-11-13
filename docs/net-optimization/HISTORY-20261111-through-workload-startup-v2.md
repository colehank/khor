# feat/net 当前状态与接续入口

更新于 2026-11-11 12:58 UTC。完整 N0–N5 目标“完成计划”仍 **active、未完成、未阻塞**，root 串行，不派发子智能体。上一份 STATE 逐字保存在 [此前完整状态](HISTORY-20261111-through-effective-route-v3.md)，SHA `6af9bcf2dfba67b241169932df711a1acdbcec8fba37eb379eaea0ae13e3d3b3`，继续链接全部更早记录。以下为当前状态；历史中的“当前/待验/草稿”仅代表当时。原始日志、准确收据和实际工作树优先，不按摘要覆盖未提交内容。

## 13:22 UTC 当前执行入口：3de双OS全12/四peer绿，N3基线6f4c在途

同一冻结 **3de372a0** 的 Linux **turing-20261111T131248Z-5c2a0339** 已 **391.748秒终态0**，21files/identity/原group空，完整12组实际绿：net68+1原ignored、hop89、wires16、journal30、budget36（含六项initialCustom）、IPC18及六单项。`node-regression-verification-v1.json`/ledger已终态；与Mac43022962905同源12绿、vendor43023967463的443/58/11绿，以及物理6970ad46三轮全6绿构成这一候选的准确已验范围。没有最新手机、完整N1尾巴/quiet成本或N3/N4/N5最终接受。

唯一在途 Turing 现在是 **turing-20261111T132111Z-6f4c9c05**，PID **4108739**，13:21:33 UTC launch，task `netlab-workload` / revision `resident-mixed-workload-v2` / role **baseline** / control **1fac8560dc25fac3a64028ff25ecdb5e786e057e**。stage exec79394已终态，下一步只查此receipt/execution/实际PID与编译/三轮，不重新stage、不另起Turing工作。

它的产品源是 **b0ee2d1ccc94365601623533cb483d72c5d7466f** / tree **67fc3c61688a04634a44cb1b68387cd7f5fc302a** / snapshot **b1d0fcb7478c9057534aed88ef3cd21736ee6550**，`candidate-resident-workload-source-v2.json`，冻结时clean/0excluded。相比3de，Node/net/iroh/noq生产文件与Cargo锁逐字无diff；新增的是ignored真实N3测量代码和文档，不能将3de既有收据伪装成新fixture已验。

初始fixture源 **bf2ea621** / tree02b86afa4a570fd3ad9c21e6c9db7c187358126a / snapshotf9e7fe945cbfffdbed506fce2b73709eeb9bdcf0 只在Mac **43027741141** net-unit取到root格式红（request84f8206545c1475d9fa9d3e8950f7390），准确 artifact **13860120002** / ZIP SHA **b613d7c564bfdfb971f2c59defa35c76b608af0909ef5806caedd03b2d01d745**，9files/11controls/identity、零行为组。精确format.patch仅改workload.rs，已check/apply到b0ee；format-only proof/ledger终态。不称Mac测试通过或Linux fixture编译。控制11项stub检查与固定入口SHA收据在 `control-workload-tests-v1.json`。

N3这轮仍未校准、不设性能通过线：先核实际PTY唯一回复、所有720逻辑样本、16MiB全量字节和真实bulk重叠、API关闭/scratch兜底；`closed`目前表示close_on成功回复，**尚无独立host/child退出证明**，不可用作完整泄漏/取消验收。调度为固定计划时间且每类最多一个在途请求；晚于计划的积压必须保留，分析同时报告开始→结束与计划→结束/调度迟到，不能只看成功服务时间或称开放并发终端模型。原数据包含两种时间，可追加独立分析，不改写原收据。

前述“唯一Turing5c/6970”“尚待vendor/全12”的旧状态均被本节实际终态替代。下文旧红和完整N0–N5范围继续保留，目标active；下一轮若编译/测量入口红，先分离fixture错误与产品红，固定源/协议后再跑，不以完整性校验绿宣告N3性能接受。

## 13:15 UTC 接续：四peer全6绿，Linux完整回归在途；真实N3基线待编译

前述物理 **turing-20261111T125643Z-6970ad46** 已699.061秒终态0，37files/identity/原group空；准确三轮f4r首内容 **1.263034349 / 1.253216299 / 1.222981525秒**，各30持续样本、15.386/15.342/15.544秒保持、held/continuous/selected_every/carrier_both全true。独立核原始两端：每轮31个实际完成响应、31个匹配stream、每端同一business wire、每端单一outer commit/previous_serial null、无detach/I/O-ended，首次响应都在M恢复之前。三原f4控制也绿；**全部6项绿只关闭这份候选/诊断协议的这批场景，不回填旧红或称p99、恢复长尾/移动/能耗完成**。本run `physical-result-v1.json` 与ledger已终态，不重发。

唯一Turing现在是 **turing-20261111T131248Z-5c2a0339**，PID **4097389**，13:13:10 UTC launch；task `net-node`、revision `effective-route-full-linux-v3`、原control68cad、同一冻结3de源。stage exec3843终态；按准确receipt/execution/实际PID继续核全部12，未终态前不穿插Turing构建/计时。前述“唯一在途6970”已被本节替代。

N3真实基线已写源码 `crates/cli/tests/netlab/workload.rs` 和 [协议](WORKLOAD.md)，ignored独立测试，**尚未编译/运行**。真实remote shell采用当前cookie/Version/Ready协议；请求不含响应前缀以排除本地回显，和真实目录/文件API并发。idle-before/bulk/idle-after各两类120样本，总720，不删除错误/超时/blocked；16MiB全量字节对照、两类至少各30样本实际落在bulk区间、关闭scratch PTY。名义40ms/8Mbit/40msqueue尚未独立校准，明确calibrated:false/diagnostic/non-SLO；不是N3完成。终端现有Tunnel与目录/文件Main分开，不能把“再拆终端连接”当新优化。

控制树新本地commit **1fac856**（核完整SHA），新增SSH-only `netlab-workload` 固定三轮与原始样本验证。原门保持：只改TASKS允许列表/独立分支，不改变full或physical任务；新parser以真实720样本/唯一index/时序/完整summary/重算overlap对照completion，缺样本/重复/超时/伪造overlap/缺轮/重复summary不能绿。control stub单测 **11项、8.381秒全绿**（新增1项7subcases，零产品执行），bash语法/diff通过；原2 pycache保留。OPS仅增加固定task allowlist，`control-workload-helper-audit-v1.json`保留前后SHA。该control尚未推送，SSH可直接精确stage；Mac仍remote68cad。

下一步先收同源Linux全12；同时用Mac现有通道取得新fixture的远端格式结果（若需）；新N3源/新control各自冻结后，Turing前驱终态/资源门允许再单次提交workload。不可用新fixture覆盖旧3de source收据或把新source混进本轮性能结果。N0–N5其余范围保持下文；源代码图谱对新untracked文件尚无节点，0flow不是验证。

## 当前候选与本轮确认

产品 worktree `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch `feat/net`。验证候选 commit **3de372a026488a64a794935600e47db29f2f6faf**，tree **ae76cd646749ca624d5e3e98af01f96099ebd0e0**，snapshot **a9f46eaa909b8cf45663395912798effb31b7261**。冻结收据 `/Volumes/disk0/Khor/evidence/net/candidate-effective-route-source-v3.json`。本状态另作 docs-only 提交，不改变既有验证的源码身份。

| 同一候选的验证 | 精确句柄 | 已核事实与边界 |
| --- | --- | --- |
| Mac Node 完整回归 | Actions **43022962905** / request `d951483105d54f8bbfc8d39f8811c854` | 全 **12 组绿**；hop89、budget32、net68+1原ignored、wires16、journal30、IPC18及六个单项。六项 initialCustom 与两个无 marker incoming 合同均实际执行通过。 |
| Turing 路由短回归 | **turing-20261111T124757Z-678b0ebd** | 118.671秒终态0，初始路6+incoming2=**8项绿**，13files/identity/原group空。**不是 Linux 完整12组**，也不是物理或SLO验收。 |
| Mac vendor 筛选 | Actions **43023967463** / request `571e47203a21465283c6c9292719d51e` | Noq **443** / path **58** / home **11** 绿。仅筛选回归，不证明59补丁全专项、完整vendor或上游/退出接受。 |

三份原件均已下载、核源码/control身份与逐组原始日志，独立 behavior proof 与 source-refs ledger 已终态：Mac Node 20files/11controls，exact artifact **13855483361** / ZIP SHA **64ddd87f3f21ce255ae06f06ccf8a305eb435ff799f2a7020838ce8ed879c50c**；Mac vendor 13files/11controls，exact **13855853516** / ZIP SHA **c4c6f98d72a3fffabd974399bcd34d6cd3b23df1d675ca489b9bf12202ded5be**。证据位于 `/Volumes/disk0/Khor/evidence/net/` 对应 `mac-ID` / `turing-*` 目录；不重复提交这些已终态任务。

## 唯一在途 Turing：同源四设备故障验证

**turing-20261111T125643Z-6970ad46**，PID **4064427** / child **4064640**，12:57:05 UTC launch；task `netlab-hop-ownership`，revision `effective-route-physical-v3`，role candidate，原 control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**，准确同一3de源码。stage/launch exec98753已终态；12:57:34 UTC实际ps活、execution running/network-driver，repeat1/3正在编译。只查这一 receipt/execution/PID，不重发、不穿插Turing构建或计时任务。

按原四peer门复核：M实际停掉、两次实际请求、经N、新首内容≤1.5秒、≥30持续样本、≥10秒、双方Link/held/selected与实际business wire/stream/outer serial保持。新失败样本输出两端hop/via、selected_every/carrier_both；即使held首次false也取完30样本，但累计false仍失败。该采证改变失败时观察时长，不与旧v2冒充同测量版本。原阈值与旧红均保留。

物理轮次后仍须同源 Linux `net-node` 全12（control68cad），再推进以下完整范围。优先先做物理轮，是因为 Mac全12+Linux专项已绿，而物理结果可能直接要求继续修改候选；**不能用短8项替代Linux完整回归**。Mac独立槽可并行，当前两份Mac任务均已终态，无需重发。

## 本轮实现的准确含义

- Node 已安装 carrier 时，普通拨号保留原generic路，与一个立即显式 Custom 普通握手竞争；early 保留原fast0RTT尝试与一个父future借用的 Custom 普通握手。原caller/deadline/最多2活动保持，旧路失败不会抢先取消在途carrier，认证resting仍决定。无新增常驻map/task/timer/强Node；写入/0RTT准入不扩大。快速Direct负控通过，但额外握手CPU/字节/手机成本仍待测，不能称零成本。
- Iroh49：peer共享preferred不在本conn且本conn只有一条established/open path时，报告Noq实际fallback路径；空/多条不猜，仍Unknown。共享偏好/Noq状态/发送/其他连接保持。`rtt_of`/Patience会读到实际RTT，须物理与成本验证，不能称纯显示。主vendor现 **59 = Iroh1–41/45–49 + Noq1–13**，42–44隔离；上游等价性未验证。
- 两个旧incoming测试保留“无marker时仍保留活carried wire”前置与最终所有权断言。测试专用selector只对Custom返回none，独立真实Direct/TLS端点保留出站Direct/PathId0/前后echo；实际carried incoming仍allcarried且all!selected。没有删除marker条件或把出站改Unknown掩盖原错误。它是所有权集成合同，不是生产单endpoint拓扑或实际两条中继腿。
- 四个installed-carrier合同与旧573逐字相同：旧同身份IP连接前后真业务活、预装carrier、原1秒caller、唯一实际响应/正确路型、两快速Direct负控。新的生产拨号与166逐字相同、49生产观测与eb72逐字相同；SHA/不变量在 `effective-route-observation-audit-v1.json`。

## Turing 开发加速入口

控制树 `/Volumes/disk0/Khor/worktrees/current/khor-build-net` 本地HEAD **caea4234bc0df61964815ace051571815c07f065**，新增 SSH-only `net-node-routes` 固定6+2过滤。驱动核实际数量/单一summary/无ignored/原日志hash，任一失败或零项都失败；原完整net-node与资源门保持。控制代码Python stub单测10项绿、bash语法/diff检查通过，未在本机跑产品。记录 `control-route-contract-tests-v1.json`。**新control仅短回归使用**；Mac GH与现有full/physical继续精确68cad保持可比。无须为SSH通道推送新control；原2个untracked pycache保留。OPS只扩该固定任务allowlist。

## 必须保留的旧红与当前风险

旧573两端最终合同 **answered=false,path=None,requests=0**；候选166两端变成 **answered=true,path=Some(Unknown),requests=1**，same最终路型门仍红。eb72两端hop87pass/2fail来自旧fixture无marker前置不再成立，未到最终所有权；新fixture3de两端对应专项绿。412早先是测试清理None panic，不当作产品最终红；ca5两端仅format红，精确patch已应用3de。这些各自原件/分类/ledger终态都保留，不覆写为绿。

最新已完成的物理源b1a57d（Turing2f2a53e1）新f4r三项仍红：首内容 **1.2207/1.2515/1.2418秒**，前两持续首样本held=false；第三30样本/15.327秒/双方31业务流/同wire/outer不换，但唯一Custom第一响应selected=false。原三控制绿不消除早先源963/a653两轮first-leg-heard2059/2325ms、业务最长5.2/4.9秒的真实红。证据 `business-continuity-decomposition-v1.json` / `old-f4-business-owner-audit-v1.json`，完整6行见该run `physical-result-v1.json`。

候选风险假说（**未证实**）：接收端generic拨号可能早于incoming carrier附着，自己的prep已结束None后缺少新carrier事件唤醒；不能仅凭入站业务活推定该端出站Link已好。正在运行的逐样本两端hop/via采证用于分解，先看实际结果，不提前加轮询、timer、常驻状态或修改阈值。

更早严重6aa stall/quality_steps0/RTT不对称与carrier约21秒结束、约8秒恢复尾巴、Busy854、resume32timeout、quiet/f4s/coldstart、Noq13物理7.3/5.4秒、截断8f3均未闭环。Iroh47原4项的10秒/7.499秒红保留。旧源动态27/长稳4绿、底层Noq704样本/15bulk、原9项对照都不是当前源码完整验收或手机证据；3样本不称p99。

## 完整 N0–N5 仍需交付

- N0：冻结最终UX门、足量p99与同条件完整对照报告。
- N1：恢复长尾/严重stall/质量/跨连接测量、quiet与比较成本。
- N2：真实同conn独立及同时v4/v6迁移、LOST身份、完整取消/owner/迟到/撤销、carrier在调用开始后到达。
- N3：真实Node交互+批量、共享/分连接/背压/公平、1/3/5设备CPU/流量/唤醒/能耗；当前底层Noq证据不足。下一实现应先建立可追溯真实Node混合负载基线，目录RPC不冒充终端回显。
- N4：完整双腿Direct/Relay/SSH、唯一/多middle、prepared竞争/混合版本、独立v4/v6移动与故障域/运维设计。
- N5：59项vendor上游/退出与Noq1/5专项、最终Quality/native/proxy/多机/iPhone和一致报告。

MacLab与Turing同LAN；macserver为另一构建机、用户报告异网。localhost iPhone为历史0.2.0，曾反复关开App才恢复、蜂窝截图仅hinton直连29ms。**候选未安装、无phone sampler、旧测试窗口已过期**；可信GitHub/TestFlight具体构建就绪后再交接真机，不将旧安装版体验归成候选证据。

12:56 UTC向用户更新：还没完成，Mac全12/Linux专项8绿，本轮四设备验证启动；完整交付暂估仍3–5天、低置信。此前按小时估计过于乐观；不因compact重置目标或将单轮测试时长当完整ETA。

## 执行约束与下一动作

本机source-only：无本地Cargo/Rust/Node/Xcode/fmt/产品执行；Git/Python源码/元数据与control stub单测允许。Turing严格串行UID1110/flock/user-netns `/tmp/khor-netlab-feat-net-1110` / Rust1.95 / jobs16 / nice10 / 50GiB起40停 / 90min原门，无sudo或host网改动。Mac独立jobs2/shared cache/原资源门，可与Turing并行。SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。

完整实现及这些远端验证已获授权；无main合并、发布、采购、生产部署或已装App/config/store/resident修改授权。fixture只动namespace scratch。Iroh42–44隔离树detached2ef6bee5不动。图谱先行；本worktree无.codegraph不擅建，CRG部分/滞后，0flow不等于无影响。debug/explore/review技能已使用。

证据root `/Volumes/disk0/Khor/evidence/net`，ops同根；`source-refs.json`仅root单写，stage/register未终态前不并行ledger mutation。timeout/EOF不是终态，恢复同一receipt/PID/Actions，不重发。下一步先核唯一物理6970ad46；终态后收原件/identity/实际门，若红按两端真实business wire/stream/outer/Link分解，保留旧红；然后同源Linux全12及N3等完整范围。无外部阻塞，目标保持active。
