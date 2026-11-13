# feat/net 当前状态与接续入口

更新于 2026-11-12 04:23 UTC。用户明确要求“停下吧 我先lab验收”，完整目标“完成计划”已 **paused、未完成**；范围为 [PLAN](PLAN.md) N0–N5，root 串行，不派发子智能体。
此页只列当前状态与关键证据；此前全部原状态逐字保存在 [历史状态](HISTORY-20261112-through-stream-preparation.md)，SHA256 `dbe4b053534a6483528f89656cd4c9e808450fcb95b6acd65182594c0f15136e`，并继续链接更早全部历史。实际 worktree、收据与原日志优先，不按摘要覆盖未提交内容。

## 当前状态与立即下一步

**用户暂停（优先于下列旧执行记录）：不继续修改、构建或自动调度。** 用户先做Lab验收，恢复需等用户明确继续。
本次暂停核对：Turing **turing-20261112T041552Z-e46a2819** 已实际completed/原0/345.480199秒，原runner PID已消失；Mac **43153603327** network-mac及共享guard已实际completed/success。两项都已自然结束，没有取消或新发任务。尚未采集/完整验证本组原件，不能把原0/CI成功当整体验收。
首block仍停在原位置，**没有运行advance 1或提交block 2**。E/pipeline-campaign-v1.json标记用户暂停；ledger pending代表证据待收口，不代表远端任务还活着。
被打断工具已创建 `/Volumes/disk0/Khor/worktrees/current/khor-net-pull-cancel-v1`，branch experiment/net-pull-cancel-v1，HEAD e5527f93；仅创建干净副本，未写取消合同或产品改动，保留现场。canonical仍无rolling集成或App部署。
恢复后先收集这两个已结束任务、核验原件，再由用户决定接续；不重发旧run，也不凭下文旧“在途”记录调度。

以下为暂停前执行记录：

**当前两项在途（已提交，不重复）：**

- **Turing：turing-20261112T041552Z-e46a2819**，PID1044381/child1044594，netlab-workload-pipeline / workload_pipeline_v1-block1-source8-batch；宽8A首block，实际running/network-driver，format已过，尚无场景已收件接受。source0c4e4e35/treeaf0c558c/snapshot45306634，controld2853824。
- **Mac：43153603327**，request36ef1faf8c2c4a7aa5c16cbc91e57e58，network-mac共享guard步骤in_progress；sourcee5527f93/tree78bb8acf/snapshota5e2b8c3，准确control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34** / feat/net-node-priority0-v1。固定12 net-node回归，尚未完成；不含Finder files库，不豁免旧Finder红项。E/mac-pipeline-node-43153603327已登记，artifact下载/行为验证待终态。

**宽8和宽2真实RPC合同四arm全部通过。** 隔离v2树 `/Volumes/disk0/Khor/worktrees/current/khor-net-pipeline-v2` / experiment/net-pipeline-v2，HEAD e5527f93；canonical未集成rolling。

- 原批量source **b020a80163240c5ed90df4fc8195b0d8e90d53bc** / tree86271ca6 / snapshot9ffaa90f；**turing-20261112T040425Z-36a4357d** 原0/108.097333秒、13files。
- rolling source **b774671bbc4e723d4b39c43b08b090dab881bfbd** / tree2cf168cc / snapshot17971742；**turing-20261112T040708Z-436c6664** 原0/90.080052秒、13files。
- 两轮固定control **916b2770a11e2fdefbe1a121b8fcdcee35741592**，各原四逻辑/三真实Node pull_into→QUIC FetchPath合同全过；准确冻结metadata、原人口与文件hash、identity/原group空及ledger终态，各pipeline-wire-verification-v1.json。
- 慢尾：batch已发9/客户端解码8/提交1片；rolling已发11/解码8/提交8片，尾片仍未释放，真实补齐不同。慢头两arm已发9/解码8/提交1片且未超开。取消两arm八条等待流都收到STOP_SENDING，同wire独立Sessions仍响应。
- 只证明控制responder与原future drop；不替代生产server/Job stop/host-child或普通性能/手机。
- 原 **turing-20261112T040138Z-5f783fc8** 原1/45.810095秒、10files，仅格式、零行为；远端补丁准确应用为b020a801，原失败/format-only-verification/identity与ledger终态保留。

首个rolling计时前已冻结四树 **E/pipeline-source-quad-v1.json**：宽8A **0c4e4e35**、B **e5527f93**；宽2A **ea414bfc**、B **2438172c**。四份E/pipeline-w{8,2}-{batch,rolling}-source-v1.json。A原生产batch函数逐字不动，B pull函数逐字来自57ff原型；同helper/逻辑/真实合同/长files_wire/所有vendor锁，pair只有该函数调度差、跨宽只有PULL常量8→2；priority0。
新增共同source-owned pipeline metadata只复用原recorder并准确标记本轮，原期限/失败/全部人口门不改。独立timing control **d28538241f29a1608a7fed606da25d2fde57470f** / feat/net-pipeline-campaign-v1，只加固定netlab-workload-pipeline分支；原prefetch分支逐字保留，C HEAD5d不动。宽2batch **turing-20261112T041034Z-0a61687c** 原0/90.094105秒、sourceea414bfc/tree9ffedafc/snapshot74fee44c；宽2rolling **turing-20261112T041334Z-19528761** 原0/94.184242秒、source2438172c/treeada3cd2a/snapshot204945db。各13files、四逻辑/三真实全过；准确metadata、独立原人口/hash、identity/原group空与ledger终态，各pipeline-wire-verification-v1.json。
四树已在首个timing前保存；宽8合同源相对计时源只有新增的共同Python wrapper，crates/vendor/Cargo逐字相同。完整campaign **E/pipeline-campaign-v1.json** 已冻结12blocks/24cases、先8后2、各ABBAAB、low/high独立，首block已启动，未完成任何timed block。
工具准入 **E/pipeline-tools-synthetic-check-v1.json**：新control固定分支相对原prefetch只改显式标签与wrapper，原守卫/low/high/失败/期限/诊断off调度逐字一致；六dispatch stub+九wrapper数据stub过，零产品/网络/普通计时样本。

立即接续：只读 `E/ops/poll-turing-pipeline-campaign-v1.py turing-20261112T041552Z-e46a2819`。等当前实际terminal后，CONTROL `scripts/collect-network-ssh.py E/RUN/receipt.json`，等collector实际session终态；PRODUCT `verify-turing-record.py E/RUN` → `verify-pipeline-block-v1.py RUN 1`。两profile各720完整人口/原hash及冻结metadata复算后，`advance-pipeline-campaign-v1.py 1` **只执行一次**，它先关闭ledger/campaign再唯一stage/launch下一block。任一缺失人口先调查，不补样本；业务超时保留，不改绿。
Mac以现request/run查GitHub jobs/artifact；下载、身份和实际固定12日志验证后才关闭ledger。所有stage/launch/register工具session已终态，两个远端job仍在途；无待恢复本地exec句柄。

最近已完成 **turing-20261112T033334Z-a09450d0** 已原0/339.545858秒、29files/identity/原group空/准确冻结metadata与独立原始计数时间窗/ledger终态。
source **5064fe2a435d9cf68389c97d7dbb95d5bdf2c715** / tree **9bf94eed4355b0eb9b96f6c99a0f9b284dd07923** / snapshot **05a7bab4a6b24a324aa2906d8bdc86c8a51c63d3**，control **73fa35f5954175e5e20373c858691315d4a3e6fe**。
原两guard+六读全过；十二准备见证、十二≥10秒静默见证与十二真实请求阳性对照全过。proof **E/turing-20261112T033334Z-a09450d0/stream-quiet-verification-v2.json**；完整表和接受边界见IDLE-ACTION末节。
这是带full日志的诊断准入，ordinary_latency_samples=0，不是普通性能/手机/完整N1接受；原v1静默失败和旧普通六观测的证据边界保持。

**下一步回到既定N3匹配batch/rolling**：先准备priority0、相同fixture/helper/长files_wire载荷的两arm与宽度8/2四树，保留原batch函数；补真实RPC已发/完成/待提交与补齐、取消所有权合同，再按PIPELINE-COMPARISON原8→2、各ABBAAB/low+high的24fresh顺序执行。当前rolling计时0，不拿旧prefetch当baseline。完整N0–N5剩余范围见下，未改为只完成这一组实验。

当前Turing e46a2819在途，不能跳过它另行launch；其前件19528761已收件验证终态。更早a09450d0等与四contract全部终态，不能重复。新任务用准确source/control与唯一measurement revision。
只读轮询工具 `E/ops/poll-turing-streams-v1.py RUN`；CONTROL `scripts/collect-network-ssh.py RECEIPT` 需等实际session终态后才验证。
采集原非零有完整报告时继续核原失败；PRODUCT用 `E/ops/verify-turing-record.py RUN_DIR`核身份，再做相应实际行为/人口/原SHA验证。
准确冻结STREAM metadata比较前需JSON roundtrip规范整数conn-id字典键。root为source-refs.json唯一串行原子writer；观察超时不是终态，按同receipt恢复，不重发。

上一个 **turing-20261112T032645Z-37e89511** 原1/182.291534秒、10files/identity/原group空/ledger终态，仅wall_us链式格式差异、零行为；原patch精确映射成5064源码，format-only-verification-v1.json保存原失败。

## 工作位置、执行与授权

- PRODUCT `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch **feat/net**，创建基线6485cb83；当前已测代码提交5064fe2a，之后STATE/历史文档提交不冒充重新测试的源码。
- CONTROL `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，HEAD仍 **5d3691f252f9dee0297fa3c494374be6119bca29**。新控制只建Git对象/ref，不移动HEAD；现有两个未跟踪__pycache__保留。
- 隔离旧rolling原型 `/Volumes/disk0/Khor/worktrees/current/khor-net-pipeline-v1`，branch experiment/net-pipeline-v1，clean **57ffe973ed11d03dc76c5fa01904719b27651b36**；未集成、priority仍-1，不能直接与当前priority0比较。
- EVIDENCE `/Volumes/disk0/Khor/evidence/net`，ops在其下ops；canonical ledger `source-refs.json`仅root串行原子写。
- 本机source-only：不跑本机Rust/Cargo/rustfmt/Node/Xcode/产品。Git、Python标准库数据/合成检查、bash -n可用。重型验证Turing与macserver+私库colehank/khor-build；具体控制和收据不得混用。
- Turing原UID1110/flock/user-netns、Rust1.95/jobs16/nice10、50GiB启动/40停止、90分钟保持；不sudo/改宿主网络/在同Turing计时中插入编译。Mac共享guard/jobs2/90分钟保持，可独立重叠。
- 用户已授权“完成计划”实施，旧AGENTS“仅保存计划”不是永久暂停。无本任务main合并/发布、中继采购/生产部署、已装App变更授权；中央GUI任务的ship授权不套用。
- MacLab与Turing同LAN；macserver为另一构建机。localhost是旧0.2.0 iPhone；此前人工反复开关恢复不算候选自动接管通过，旧手机窗口已结束。
- 图谱partial/stale，不覆盖vendor时的0nodes/0flows不是覆盖证明；未建新的CodeGraph索引。不派发子智能体。

## N1首次操作与静默：已知证据及未闭合同

协议 [IDLE-ACTION](IDLE-ACTION.md)。原p7 active10秒/idle20秒门不改。原idle选路恢复16–17秒与15秒备用保活量级接近，只是假设，不证明唯一根因或极致体验达标。

**fixture所有权旧红→修复绿已闭**：

- 旧source e471a2f1、control e9a443c4，**turing-20261112T015902Z-e8569de4**：真实caller drop后device future继续，原1失败/1负控过，清理后才断言，原101/90.034秒/13files。
- 修复 **0a8b90d9986d3426f65b61ae9d994083f1fa3321** 只在LabNode.with由调用者持有AbortHandle Drop。**turing-20261112T020453Z-b9c5278d** 原0/53.827秒/两项全绿/13files。
- 两份idle-action-ownership-verification-v1.json：guard逐字相同、control相同、准确metadata/原marker/footer/hash、旧红绑定、原group空与ledger终态。只证明fixture caller→device future；不替代server RPC/host-child/产品恢复。
- 原workload filepull是父future直接借用tokio::join!，没走LabNode.with；不据此改写prefetch结果。

**普通六次首次读取已观测，静默合同未由它们证明**：

- 首个源码725e12f7的e6569ee7仅格式红/零行为，原件保存。
- source8f87adcb的 **turing-20261112T022049Z-7b65087d**：六次读取全ok，但metadata误期望两个netem都在alpha，缺router ingress读回，原1/226.568秒/17files失败保持。
- 修正source **5a5d7e84a557b2f2000aa0dcf513d61adc4538cb** / tree d1e6496d / snapshot67db6c6f，control2cfce93d；**turing-20261112T023151Z-8d7b0ea1** 原0/224.643秒/17files。六caller/十二起止/162条命令、真实双向150ms安装及恢复清除、veth ifindex/link_index双向绑定、逐接口计数独立复算、全部原SHA与ledger终态。
- alpha phase0/14/5首次读取297.685/327.577/321.867ms；beta230.688/265.406/250.937ms。实际phase与两端事后Relay路径、全部接口差值见first-action-verification-v2.json；每格只有一个fresh，无p95/p99/显著性/唯一因果或能耗结论。
- quiet-scope-audit-v1.json绑定上述proof：没有主动前台调用不证明无后台STREAM；完整N1/quiet未接受。普通source-owned输出显式continuous_stream_silence_verified=false/full_quiet_contract_accepted=false。

**Iroh50被动见证、v1真实红与v2准入绿**：

- 只在原250ms reselect已有stats读取上加DEBUG快照：所有受跟踪连接STREAM tx/rx、actor序号、增删代数及单调/墙钟；连接在采样间新建关闭也会改代数。原合计/ActiveRefresh/策略/生产计时器不改，无新任务/定时器。
- source **09374004906a9a400004874fa957726e857bef1f** / tree b23ff21f / snapshot8ee7e55c，control **1bdb2272f23bfd59961af79cbebf04242791ce33**；**turing-20261112T030608Z-15ebb293** 原1/305.229247秒/29files/identity/原group空。
- Rust原两guard+六读全过、十二真实请求阳性对照全过；十二静默见证全红。stream-quiet-verification-v1.json准确冻结复算、独立原人口/全部原SHA与ledger终态。
- 十三次STREAM变化被见证于候选窗起始后307642–645086us，actor/registry不变。这是观察tick，不是精确发包时刻或唯一业务来源；原失败保留，不把后半段选出来改绿。

**当前v2修复的是准备阶段**：

- 新revision first_foreground_stream_witness_v2，控制73fa35f5只改同task MODE literal；runner/其他分支/原UID/flock/netns/资源/期限逐字不变。
- 仅诊断模式、候选窗之前只读双方serve.log，实际counter/actor/registry签名连续≥1秒相同、快照年龄≤1秒、总准备期限10秒。无请求、无Node状态查询、无关闭后台任务。
- 记录准备起止和各端实际稳定快照；外部逐条绑定原日志。随后原12秒候选窗、固定500ms边界内≥10秒完整见证、整窗恒定、最大1秒采样间隔/100ms时钟增量差、原phase0/14/5与30秒caller、六fresh顺序及阳性门全部不改；候选失败不重试挑窗。
- 普通模式仍10秒/off且不等待此诊断准备，诊断不作为普通计时/CPU/能耗样本。
- 11种见证+6种准备+19种first-action+9种guard合成数据，4 unittest已过；6种调度stub保持guard off→action full与首原73/79失败。全部只是工具合成准入，真实v2已按顶层run/proof通过该诊断准入；完整N1/ordinary性能仍未接受。

## Mac默认vendor与其他回归

**新Iroh50准确源码默认库已绿**：Mac **43146243502** / request **ff36367cf58e4a1ebd0e5d6f5d1c460c**，source09374004/treeb23ff21f/snapshot **901d85407023827999147998c3318192f8be302b**，control **4e34fb7dce174345410a33e892b0eb76a5cc5c23**。
artifact **13911657397** / ZIP **4d8db782bfd2767c88855d5a14f3b6249aa06e338617ed1ba7e715e5a4904ce3**；E/mac-stream-vendor-43146243502，实际文件在artifact/（不是report/）。
16files/11controls/source/bundle/两vendor锁逐字一致/各原footer和SHA核验、vendor-full-verification-v1.json与ledger终态。
Noq完整默认库447过/0忽略/0筛除；Iroh完整默认库184过/原1忽略/0筛除。唯一原ignored flaky为endpoint::tests::connect_via_relay_becomes_direct_and_sends_direct，未运行/删除/称通过；58路径/11home专项与184重叠，不相加。
相对当前5064完整vendor差异只有KHOR.md文档，运行时代码与锁逐字相同；该证明适用于vendor字节，不证明CLI新准备逻辑或最终Node/native/手机。

原e471准确源码Mac **43134579036** 固定12项net-node全部通过：net68+1原worker ignored/hop89/wires16/journal30/budget32/IPC18+六单项。该门不是全部1378 Node，也不能声称新Iroh50之后最终Node已全跑。

隔离rolling原型57ffe：4窗口逻辑、跨八片的files_wire和handoff真实scratch合同已两次过；files库32过/唯一Finder trash AppleEvent -1712失败，Mac **43124311515 / 43126157862**，整轮失败保存。一次确认已用尽，不盲目同源重跑、不豁免该门。只读主机观察43128627304为零产品测试，未证权限/响应唯一根因；后续仍需收口。

## N3已闭对照与接下来rolling

[优先级](SCHEDULING.md)：low/high各完整18fresh，分别12960交互全ok、文件全完整、原population/hash/准确metadata独立复核。低profile目录p95九对-1更快，但high文件/目录/终端服务九个对应位置六次-1更慢、三次更快；不把位置差当逐请求因果。canonical c3287169恢复中性priority0，PULL_AHEAD=8；不是全局最优已证。

[批量prefetch](PREFETCH.md)：原8,1,2,1,2,8,2,8,1九组18fresh完整终态，12功能通过/六high文件原240秒超时；12960交互全ok、153原report文件SHA与原始人口复核。low九份16MiB全完整；ahead8三份192MiB完成82.239568–98.920231秒，1/2各三high超时。超时是右截断attempt，无verified payload rate，不称文件完成时间。
prefetch-comparison-v1.json SHA **7dede5f2c580d46aff0e6138b968da2550db2e095d6de351652a56d8a7e537b0**；全部advance1–9与旧high1–6终态不能重复。

[rolling协议](PIPELINE-COMPARISON.md)及prefetch-to-pipeline-decision-v1.json在任何rolling计时前冻结：**先宽8，再独立宽2**，priority0，每宽A B B A A B、每block原w3两项再low/high各1，共12/宽、总24fresh。宽2在low的完成/交互取舍值得保留但high原超时，不能直接设全网默认。
四源码与campaign已冻结且首block在Turing执行，0 timed block已收件接受。匹配baseline/candidate包含同一未调用helper/四逻辑/大于八片的files_wire载荷；A原batch函数不动、B仅rolling循环及必需短读重启；四树审全部Git/runtime/fixture/参数，只在相同宽度比较。不拿旧prefetch当新baseline，原失败/完整人口/期限门不改。
后续还需真实RPC的已发/完成/待提交上限与补齐时刻，服务端/用户取消/host-child等所有权，而非只看逻辑future被poll；全前台预览、公平/背压/连接架构/规模成本仍必需。

## 完整剩余范围（不以已有绿项缩减）

- N0：最终分场景UX门、充分尾部样本与p99、相同条件完整前后对照、观测成本与成本前沿；对载荷/调度/模型作实际记录。
- N1：严重/quiet/recovery尾部、跨连接独立质量/旧样本、恢复后固定好路径匹配参考、启动/配对旧失败根因、动态模型延迟突变问题、真实静默及后台成本。旧16–17秒/单轮绿不算极致接受。
- N2：同一实际连接独立/同时IPv4与IPv6地址/前缀/NAT映射变化、换网/睡眠/黑洞/LOST，owner/cancel/late/revoke与carrier在调用开始后失效；身份、总期限、写入/0RTT不重放。
- N3：上述24rolling探索；完整前台文件/预览、持续公平/饥饿/背压、共享与分连接实测、真实server和独立host-child取消/退出、1/3/5设备CPU/字节/唤醒/能耗。文件快不代替其他行为。
- N4：Direct/公共Relay/peer relay/SSH双腿，唯一可达middle自动救通（第二腿可Relay或旧信息）、多个/移动middle、独立双栈变化、北京/香港整机/单出口/寻址故障域，自家relay约束、双家与地域/容量/运维设计。部署采购另授权。
- N5：主60项（Iroh1–41/45–50+Noq1–13；42–44仍隔离）的逐项上游等价/保留退出，其他feature/integration/physical/Noq1+physical5；最终准确源码Quality/完整Node/native/proxy、Finder依赖、多设备/手机自动换网/peer relay与真实能耗及一致报告。

历史动态f2f1775027、恢复ce3b12226、长稳de04bcc1、四peer6970ad46、模型容量c50ab6ef和cold high5d27f45e等既有证据在历史入口完整保留；warm v2路径失败、启动/配对超时及旧严重红项也保留，不能拿新窄场景覆盖。没有用户iPhone候选安装/最终接受。

进度估计：此前2–4天为低置信度估计，不因compact/session重新起算；目前没有可信完成日期。完成必须设计、状态职责、持续矩阵与同条件报告均有准确版本证据。未获准的合并/发布/采购不自动执行。

### 2026-11-12 04:03 UTC 状态回复与估计

用户再次询问；明确尚未完成，普通性能与iPhone自动换网/peer relay/能耗未接受。下一批真实RPC合同加24匹配场景暂按4–8小时预留（含排队与构建、低置信度、不是承诺），完整计划仍数天且无可信日期；此前估计偏乐观，不从本次回复重新起算。全部N0–N5范围保持。
