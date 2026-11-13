# feat/net 当前状态与恢复入口

更新：2026-11-09 09:47 UTC；root串行，无子智能体。活动目标“完成计划”仍active。

**当前无在途任务，140th Linux net-node已终态并采证。** 根格式/编译/net66+1ignored/hop9/wires10/journal28通过，dial_budget15pass1fail；唯一新F7在nft规则语法前置失败（未安装规则），不是产品退役行为红。已改多行nft规则，下一先固定新源重跑Linux net-node，再corrected baseline hop137th→candidate stalls130th→baseline stalls131th（均已stage未launch）。corrected candidate hop136th已采证8/9，独立f4i回退原断言红仍待定因，91.4秒回carried及约10秒按键卡顿仍是体验缺口。140 refs=137终态+3 staged；完整N0–N5未完成。
完整范围：[PLAN](PLAN.md)；约束：[REQUIREMENTS](REQUIREMENTS.md)；运行记录：[RUNS](RUNS.md)；原始结果：[REPORT](REPORT.md)。
此前STATE原样归档[Accepted轮历史](HISTORY-20261109-before-accepted-green.md)与[本轮历史](HISTORY-20261109-before-probe-green.md)；历史在途/候选文字仅代表当时状态，恢复以本页和实际工作区为准。

## 授权与工作区

速度/体验优先，个人可信设备网；活动目标授权N0–N5实施验证。没有合main、发布、中继采购/部署授权。
本机source-only：不运行Cargo/Rust/Node/Xcode编译、测试、formatter或下载可执行物；Git/Python stdlib/脚本检查可用。重型Linux沿U9授权Turing、Mac原生沿macserver/GitHub，不碰现装App、生产store、个人会话或其他任务树。
图谱先用，覆盖不足才定向读；不新建CodeGraph索引。外盘布局/治理入口遵守AGENTS，本任务不接管代码治理。

| 用途 | 位置 / 当前基准 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；最新已验证生产d4a2e115b898fc337f5091b51fa71e45740c667f；随后有文档和scripts/dev.py VZ专用调度；生产网络未改，恢复核HEAD/diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，b9984797f412f352f3c76f99c47ade3ee10f68e9已push；新增net-node SSH任务白名单，profile未改；当前hop/stalls仍control59dc223，不能回写历史控制版本 |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture，生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json140条不可变refs；137终态，其余3 staged；下一Linux net-node新源待stage。每次源变化前锚ref；SSH不是GitHub run |

## N2本轮：普通wire覆盖与配对探针三红→绿

**42571393016 Mac net-node SUCCESS**：based4a2e115b898fc337f5091b51fa71e45740c667f clean、treef278b9330f51268182b142729db2c86687f4af4b、snapshot3459894c81cb994310a97bfcba4ca39ae97e569b、control0cff9671fb2f80945423132e0df46995f6ff3198。
mac-probe-owner-candidate-42571393016保存source/environment/raw/classification；根格式、net66pass+1ignored、hop9/wires10/journal28/dial_budget14全部绿（预算/流/探针9.93秒）。仅Mac便携子集，不是完整Quality或N2全包。

| 边界 | 实际红 | 实际绿 / 限制 |
| --- | --- | --- |
| normal/early拨号锁期限 | 42556768499 / 42557961726 | 42557583517 / 42558820303及最新；40ms到期不改共享结果/peer down/backoff，过期空锁不拨号 |
| 操作scope到真实ask | 42562932053两红：忽略40ms、300s被28s截断 | 42566727617首次绿，最新保持；只是持锁边界，不代替长工作/全Op |
| Accepted回答所有权/期限 | 42567527752两红 | 42567946141及最新；取消/到期STOP本流、同wire独立Sessions、相同stable_id/严格3请求/零dial_failed |
| 普通交换/一次重试/写失答 | 新增覆盖，未声称旧源行为红 | 42569954515四项首次绿，最新保持；1秒普通exchange到期不伤并发请求；2秒原by剩余700ms进入真实读重试并到期；失答Sessions严格2次、Mkdir严格1次 |
| probe直接取消、配对外层取消、首成功 | 42570772550实际11pass3fail | 42571393016全部绿；真实HTTP GET和fast已204前置后取消，800ms内TCP EOF，第二URL不被slow first挡住 |

resident入口一次建立原Op档5/30/300/3600秒减.5秒的不可变Tokio by，任务局部只承载值；锁/hop/有界拨号/延迟候选/普通交换/一次读重试显式沿用较早截止点。CallerExpired不记peer失败/退避。
独立ask无Op时由Request工作预算加现有dial25s/hop3s/最大road120s合成外层上限，各阶段仍可先结束。更新dial保留原10秒自身上限；已编译但专门运行覆盖仍缺。
Accepted交接保留Aborts，adopt后按原by等答；取消/到期停止本流、保留共享健康conn。Rejected和未adopt新conn清理、读写/0RTT准入/frame/IPC半关闭未改。
新普通wire控制包含真正同时等待的第二请求，首流STOP后才回答；零读重试/answer_late/peer failure/down/backoff/未答退役。真实retry仅一次closed退役，第二条仍live。Mkdir为代表写请求覆盖，未宣称穷尽所有动作的幂等专项。
probe内部JoinSet按完成取首成功并取消输者，pair_with用Aborts闭合外层；dial成功先drop probe再pair_over，所有早返也清理。原并发量/2.6秒单HTTP/proxy/空与全失败false/诊断fallback不变。图谱还确认ears_heard直接await该函数，返回值映射未改，不能据此声称其完整故障/回声场景通过。
RetireUnheard仍要求armed、heard_count不变、elapsed>=Patience.ack；短取消不退役，真正长黑洞保护不能移除，Mac138长ACK阳性已通过，UDP代理黑洞前置失败；新Linux独立namespace/nft黑洞覆盖尚未运行，不以Mac或纯函数绿替代。

Accepted测试证明应用的真实Accepted后等待路径；Noq is_0rtt只是应用accept时握手状态的观察，不是包级重放证明。旧前置42566727617失败与格式/编译红均独立分类，不能混算。117th run42569493787、119th run42570381896仅格式门，patch原样应用后继续；本机未运行Rust工具。

## N1仍有Linux红，不能用Mac单元收口

Iroh37/Noq2已由42555870179实际绿：base d2849d4732c7e9b6dd2e953616ce583f0e520391、tree4a9c4d89cc01e33a480375d50cc9f77b6d2d7a83、snapshotb6a4e9240786b1b178aee1e11284e6164aecc377、controldd57be6c57090a811726983e700d0b39a40fd1e8；根格式438Noq/36路径/9home。37当前tuple近期copy接收只作复测提示，无提示允许故障后备用ACK快速Go；有提示保留决定后验证，原250ms节奏/4s兜底/正常两样本保留，无应用重放。

以下原Linux同basea087 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644，普通日志关闭：

| Run | 结果 |
| --- | --- |
| 42542824071 stalls | SUCCESS2/2；600秒/20次300ms CPU，alpha/beta0/0移动、零退役/打洞，最慢865.4ms、3019包312293B；p5原150/300ms门绿，600ms仅观察 |
| 42542984111 dynamic | FAILURE25/27；p3三轮1.1/1.2秒/818.4ms，前两轮超原1秒门；p2及21相位绿，零错/退役 |
| 42543254686 hop | FAILURE8/9；f4s首轮0.7秒回carried、10.9秒后退Relay、28.8秒再回，最终LinkRelay；另两轮0.4/0.6秒无回退，f4/f4i绿 |

f4s的Relay2.5秒双向注入在断腿恢复1.5秒后清除，后续整段不能当5秒Relay RTT。该回退未独立定因，不宣称37已解决。
42542228149诊断2/2不是SLO；旧Watch较快但CPU3/1和hop1513ms红仍留，不能跨源/机器算改进。恢复Linux后原p3三轮<1秒、CPU0/0、hop无回退，再最终准确源完整矩阵，不放宽门。

## N3测量模型已校准，真实应用验收未完成

42559476637实际UDP道路校准红：旧单FIFO把反向1B挡在未来正向包后；此前42559155803只新fixture字节类型编译失败。
仅测试模型改每方向独立FIFO，保留lane/几何/seed，road_model=independent_fifo_v2、schema v2，产品CC/优先级未改。
**42560015454 SUCCESS**：base1478bf9712ad50c00b502c78613cd932f20586b3 clean、tree33d5ef6d47cde04620db1baa470bd7fefdf0c99d、snapshot9a6f9717470e0a741aedbdd9dd1ecf0a32d18fc8、control0cff9671fb2f80945423132e0df46995f6ff3198。
root fmt/编译/校准1pass、三轮各2pass（167.64/167.09/167.29秒）；704完整samples/51摘要/15bulk、零失败/超时，每轮235/234/235，计数/索引/分位数逐项重算一致，sample-audit.json存hash/方法。
固定300msRTT/每向2%/20Mbit/200ms队列/20s bulk/2固定seed；只是Noq模型，不是Node/SSH/hop/手机SLO。bulk-echo合并p95：Cubic普通890.3ms/优先389.9ms，BBR3普通449.6ms/优先612.4ms，效果不一致，不据此改产品CC；screen每CC仅9样本，不报p99。旧shared_fifo_v1不能和新模型差值算产品改进。

## 执行通道与Linux限制

托管Linux prepare因GitHub账单/额度拒绝（42545673450两attempt、42545712302，无步骤/产品测试/source artifact）；已异步询问额度或已有Linux通道，未有回复，不改账单/采购。直接SSH macserver原iroh-ssh路由banner超时，未改路由/信任。
Mac自托管worker已实际跑产品，现API Git bundle通道多次验证：私库无父source/bundle快照、64MiB上限；worker按精确GITHUB_SHA取可信bootstrap/driver，校blob/SHA256/snapshot/tree后仅本地Git clone。临时repo-read token不继承Git/prepare/构建。
沿development.yml、mac-mini-worker单槽，仅net-unit/net-node/net-vendor/net-congestion，Rust1.95/jobs2；原安装guard/内存/空间/缓存策略未改，无个人key/token/签名/模型凭据或生产store。标准executor旧checkout/default inputs保持兼容。
host实际VZ=1、16GiB/10CPU，installed guard实际50/40GiB、Mac target100GiB（42573745492采集）；未改policy。项目目录中固定Lima2.3.0/Ubuntu24.04已下载校验并在42579282147实际完成native raw转换、Linux启动与正常lease停机，无helper残留。VM为2CPU/6GiB/30GiB、空host mounts、无个人key/agent/proxy/自动应用端口。42579909366已实际验证guest namespace/veth/netem/nft/conntrack及正常/父死清理（同guard PGID/继承lock fd、Killed controller=-9、双次不同当前boot IID、最终Stopped/无helper）。VZ能力绿保留备用；U9已切Turing并实际完成源码编译和recovery，产品矩阵正在恢复，不再开发VZ产品入口。

## 完整未完成范围与下一步

N0部分、N1仍红、N2部分、N3部分、N4未完成、N5部分；完整PLAN范围保留，不以本批单元绿结束目标。

1. **当前Turing串行队列**：corrected candidate hop已终态8/9并采证；当前Linux net-node验证新F7，再corrected baseline hop（2c64555c）→candidate stalls（c56d74b3）→baseline stalls（f8a5a7e2）。Linux net-node140th已launch/control b998；其余三项已stage不得重复创建。f4i当前真实回退红待定因，原门不变。随后dynamic/full准确源矩阵；VZ只备用。
2. **N2剩余合同**：原F7真实长黑洞、长工作/更新10秒运行覆盖；hop两middle合计3秒、dial输者及延迟候选的资源/过期验证；前台多阶段及独立后台整体预算。pair/doctor探针两层已实际修复，不重复排成未修。代理listener/传输job/Visit/nudge有意独立，不能继承已结束前台by。普通wire四项与Accepted覆盖不能证明完整N2。
3. **IPC边界**：client Op外层与serve接收后deadline起点不同，尚未跨进程传递剩余预算或取消。正常exchange本来就shutdown发送半边，服务端EOF不是取消；不得为贯通而重放已发送写请求。具体原源码hash见deadline-continuation-20261109。
4. **N1**：恢复Linux，独立定位f4s回退、原门复验及准确源完整矩阵。Mac单元不能替代namespace/netem/nft实测。
5. **N3**：真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平推进、共享vs分连接、1/3/5设备后台、真实手机能耗，均需独立负载数据；不增加总资源伪造调度胜出。
6. **N4**：默认北京HTTP/TLS同机、自家登记集中北京、香港默认无独立pkarr；复用已有多登记/选家/继任机制，尊重only_mine/n0配置。双端完整/不对称路径、冷暖发现、整机/登记/出口故障域及容量实测和拓扑方案未完成，不采购/部署。
7. **N5**：48初步补丁（Iroh37+Noq11）仍需覆盖收口；Noq6计时42543928044、Noq4备用ACK隔离42539256792、Noq2/37本轮前绿保留。Noq1/5专项、其余边界、固定1.4.0逐行为等价尚缺；UPSTREAM静态hash不是上游运行，不升级/删补丁。原生Mac/代理/多机/手机/能耗与最终可信Quality仍缺。

本轮用户询问进度/工期时，已给出当前N2这一批粗估1–2小时、全N0–N5粗估3–5开发日且以Linux与真机条件到位为前提；这是粗估，不是缩范围或固定完成承诺。当前十四预算/流/探针测试现已实际绿，但后续范围仍如上。

## 证据与恢复规则

先核worktree/branch/HEAD/diff、当前run与准确回执；以本页顶端当前任务为准，以下带时间追加为历史。140refs=137终态+3 staged；恢复按run/path/PID实核，SSH观察失败须重查同句柄，不重启。
`git rev-parse refs/khor/net-evidence/<run>:source`必须等于回执tree；只git show读容器，不checkout覆盖/push本地证据refs。失败明确分format/compile/fixture/行为/执行器；tests_executed只按实际子集，历史红不删除。
Mac source/bundle分支只在终态、证据留存且所有权/SHA匹配后清理；复用源的请求不拥有原源分支。原attempt不覆盖。
外盘缺失只读`git show feat/net:docs/net-optimization/STATE.md`与PLAN；原外盘未提交内容不能按摘要覆盖。其他治理沿中央STATE/RESUME，不混授权。


## 本轮边界与下一入口

产品已提交、当前无未验证生产候选；control仍0cff967 clean且未改。没有执行安装/启动VM或改宿主网络/现装App/guard/空间策略。对控制仓库scripts/network.sh/bootstrap-network.py已只读复核：当前Mac只有四portable task，重用Linux profile前需显式executor/任务入口与准确source/control receipt；不能把Linux命令直接跑在Mac。
Lima v2.3.0树800a96b14e96b5d2ceb20ab2602147c1e79224ea的9份源码逐blob校验，具体Linux VZ CreateDisk→driverutil.EnsureDisk→proxyimgutil.Convert在qemu-img不存在时走native；此前仅看到Mac guest native分支不能代替该Linux链证明，现已补齐。默认ssh/hostLoopback配置已读，guest配置必须显式关闭个人key/agent/home及自动端口暴露，不把hostLoopback:false误当阻断全部宿主LAN地址。foreground与真实guard进程清理尚需验证。
用户已回复确认真机组合为“iPhone＋两台处于不同网络的电脑”（PLAN U8）；已异步补问iPhone型号/iOS与两电脑OS，尚待版本细节。此前Linux额度/已有通道也无明确答复，不能仅由设备可用推定有Linux执行器。继续独立工作，整体goal active。

下一步具体入口已进一步核实：控制仓库没有AGENTS.md，图谱对scripts/network.sh未覆盖后已定向只读；worker-storage.yml的inventory虽不改policy，却也依赖Ubuntu prepare，因此不能作为当前账单条件下的Mac直达采集。可在既有network-mac只读能力记录中增加有界guard文本/hash采集，核已安装run-build/build_guard.py的进程/信号清理；不得调用ensure-budget或改installed guard。此workflow补充尚未实现/调度，control仍0cff967 clean。读取的控制文件包括development.yml、network.sh、bootstrap-network.py、worker-storage.yml及inventory分支；当前hostcap没有guard逻辑正文。用户OS细节仍待答复，Linux能力工作不等该回复。

## 新接续：Mac只读guard采集

上一goal轮为progress（普通wire四项新增绿、probe三红→绿、真机设备组合确认）。控制60821accc131e837da476706cb6b3ad93812d30e已push feat/net，只在既有network-mac hostcap增加固定两份admission脚本的有界只读文本/SHA256记录；不调用ensure-budget、不执行被读取脚本、不改installed guard/资源/缓存策略。YAML、16 Bash块和2内嵌Python语法已本机轻量检查通过；实际采集待新的Mac net-unit验证。产品生产仍d4a2e115，既有14绿的控制版本仍0cff967，不能覆盖历史回执。下一run先锚准确source/control再读artifact；VM尚未安装/启动。


**42573745492 Mac net-unit只读guard采集在途**：base03df5a47f37d24a0473b9693320e48aed78b52bb clean、tree919008b695a05cd3dbbb0f90b0d5db94d193818e、snapshot2d69c1a1a9c5d13c95a70bbee90d4df83258682f、control60821accc131e837da476706cb6b3ad93812d30e；122nd ref/mac-guard-capture-42573745492已锚。产品生产未变，读取admission文本/hash及net-unit通道验证待结果；未执行VM。


42573745492终态 **SUCCESS**：root fmt/net66+1ignored绿，guard两文件大小/SHA256已核，准确tree919008b6/snapshot2d69c1a1/control60821acc已核，122 refs终态。实际installed guard SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb：准入50GiB、运行中止40GiB、Mac target100GiB，非此前文档35/20；当前free72254976000B，VZ=1/16GiB/10CPU。guard对子命令start_new_session/pass lock fd，TERM后10秒KILL整个PGID，但leader已退出则不杀；据此VM需foreground同组及lease supervisor闭合父死清理，不能只依赖正常finally。未启动VM或Node/完整测试。


## 隔离 VZ 能力候选（实际启动前）

control 97334be 添加单独 netlab-vz-probe，沿既有 Mac worker/admission/token 剥离通道；本机只做 Python stdlib 7 生命周期/隔离测试、bootstrap 3、client 14 和 Bash/YAML/Python 语法检查。固定 Lima 2.3.0 / Ubuntu 24.04 20261017 校验和，2 CPU / 6 GiB / 30 GiB；项目 owner/run 目录、空 mounts、关闭个人 key/agent/proxy/containerd/全部自动应用端口转发，保留仅 VM 管理 SSH。本地镜像下载由 owner 缓存校验，未用不受支持的 LIMA_CACHE_HOME；Lima 固定版本本地镜像分支不写全局下载缓存。工具只在远端运行。

验收先固定为：准确 source/control；installed guard SHA/fd；foreground 同 PGID；guest 实际 Linux/userns/veth/netem/nft/conntrack；正常停机；杀死测试 controller 后 pipe EOF 触发 supervisor 停机；无本次 helper 残留才清理本次临时状态。supervisor 持 admission fd，避免 controller 先死释放单槽；不改 installed guard/空间策略，不调用 ensure-budget。不挂载宿主目录、不编译产品，product_tests_executed=false。能力或生命周期红独立归执行环境，不计产品红/绿。通过后仍须同新后端重跑基线和候选，原门不改。

产品 scripts/dev.py 只增加该 task 的 Mac 专用调度与回执检查；生产网络仍 d4a2e115 的已验证实现。待推控制后提交此次客户端/计划，再 dispatch；恢复以实际 run/receipt 为准，不重复提交。


**42578713267 VZ 能力探针在途**：baseb56576ce52c56b4d8c31d84e6618abec662d0cd9 clean、tree2b5bfce2e95c0ef4169a6cb3bd719ba9914d414b、snapshot6a0465886f46b8058eb8cb149d3f4a54584a7c93、control97334bed6074a4b4208887d705ae0621e3b929be；123rd immutable ref 已锚。远端已通过可信源码准备，正在 guard 内执行；尚未读取 VM 结论。


42578713267终态 FAILURE，已核准确 source/control/bundle。仅执行器路径红：Lima 的 ssh.sock.1234567890123456 路径107字节，要求<104，未启动VM/未执行产品测试。固定两资产实际下载校验通过，limactl2.3.0，foreground 与 guard 同 PGID；无本次残留且临时状态已清理，owner 下载缓存保留。接续缩短确定性实例名并冻结完整临时socket长度检查；不放宽Linux/产品门。123 refs现全部终态。


**42579282147 短实例名重跑在途**：复用42578713267准确 source（baseb56576ce/tree2b5bfce2/snapshot6a046588），control8d3908095aab7484ebfee8b0510edbb22ed5362e；124th ref已锚。完整临时socket路径轻量回归绿，controller8项；不拥有原请求的源码/bundle分支，终态前不得清理共享源。


42579282147终态 FAILURE，准确 source/control已核。实际VZ/Linux启动与SSH uname前置成功，native qcow2→raw无QEMU成功；正常 lease EOF→force stop，Stopped、无本次helper残留、临时状态清理通过。guest能力命令非零但stdout/stderr空且旧驱动未留具体退出码，原因尚未确定；hostagent当时仍初始化step2，不能只凭SSH可用判完整就绪。未跑父死场景/产品。接续按固定Lima boot.sh的当前IID与/run/lima-boot-done一致才就绪，增加输出marker/退出码/trace；controller9项轻量绿。124refs终态，下一run继续复用同准确源，不算产品红绿。


**42579909366 当前boot就绪修正探针在途**：复用source6a046588/tree2b5bfce2/baseb56576ce，controlbedd8960d8a11ebb3263eda0f24bec9e5d9a774d，第125个ref已锚；不拥有共享源分支，结果待实际能力/父死清理。


## 42579909366 实际能力/清理绿与下一步

source baseb56576ce52c56b4d8c31d84e6618abec662d0cd9、tree2b5bfce2e95c0ef4169a6cb3bd719ba9914d414b、snapshot6a0465886f46b8058eb8cb149d3f4a54584a7c93，controlbedd8960d8a11ebb3263eda0f24bec9e5d9a774d；mac-vz-ready-42579909366保存原始日志/source/config/lifecycle/readiness/command及分类和逐文件SHA256。Linux aarch64 6.8.0-147-generic，非root UID501；unshare -Urn内veth双端UP、netem1ms、nft建表/list、conntrack-L均实际成功。2CPU/6GiB/30GiB，空host mounts，无个人key/agent/proxy/自动应用端口。

正常和parent-death两次当前IID分别iid-1794209554 / iid-1794209580；controller真实SIGKILL=-9，supervisor读lease EOF后stop返回0、foreground同原guard PGID26538、两个实例状态均Stopped，无error/stop_error/cleanup_error。最终无本次helper，清理临时state/tools，固定镜像/工具owner下载缓存保留。installed guard/policy未改；这证明已实测生命周期，不仅是静态flag。**product_tests_executed=false**，不能因此收N1/N2/完整Quality。此前空输出红原因没有单独A/B定因；就绪修正后成功也不把原红改称产品问题。

具体接续（尚未实现，不重复cap探针）：
1. 沿已验证Mac API bundle/单槽/原guard，显式允许所需netlab task进入VZ，portable task保留原入口。guest复用可信scripts/network.sh的原filter/repeat/gate，不重写产品阈值。source.json/tree/build_commit/control/host与guest环境均留证。
2. 首次Rust1.95 Linux aarch64构建用固定四个官方component与SHA256；清单已静态取得并核channel manifest，evidence/net/linux-vm-source-audit/linux-rust195-components.json（manifest SHA821ff14e4c4a1cbe1e8915f35aff0a3fbbdf8d293ad48ab8f31e3b0440c581f9、2026-04-16）。尚未下载/执行Linux工具。本机继续source-only。
3. 源码与可信driver经本次专用输入包传guest，不转个人凭据；产物只取本次report。首构建/冒烟先不设计全局缓存或改installed资源策略；从真实磁盘/耗时证据决定项目专属缓存。保留2CPU/6GiB/30GiB和50/40GiB原guard。
4. 同新后端基线6485（原六个fixture以外生产不改）与候选严格串行、同配置重跑p3三轮<1秒/CPU0移动/hop无回退原门；旧x86托管Linux结果不能与aarch64 VZ直接算速度收益。f4s回退仍独立定因。然后最终准确源完整矩阵与N2–N5剩余范围，真机组合已确认，OS/型号待回复。

当前control clean bedd896，产品仅此轮证据/计划文档WIP待提交，生产网络仍d4a2e115。125 refs全终态并逐一核对。无活动VM/远端run；继续实际Linux产品通道，完整goal active。


本轮为progress：新取得真实隔离Linux网络能力与正常/父死清理证据；未收完整目标、未设blocked。三次cap run全部终态/下载核验/immutable refs保留后，原owner以精确force-with-lease原子退役共享source/bundle两远端临时分支，ls-remote复核为空；复用请求未单独删除它们。记录vz-probe-source-retirement.json。下一步无需重新索取设备组合或重复能力探针，继续上述产品Linux通道。


## U9：Turing为Linux主通道

用户明确授权Turing，覆盖此前macserver-only的Linux执行器选择；本机仍source-only，Mac原生/可信发行边界不变。Turing SSH既有alias/known host，ForwardAgent=no；不转SSH/GH/token/签名/模型凭据。当前Linux5.15.0-139 x86_64，UID1110，128逻辑CPU，503GiB总/470GiB可用内存、约490GiB本地盘空闲，初始load2.63/5.82/8.44。userns/veth/netem/nft建表与conntrack-L均实际通过（只在新namespace，不改宿主网络）。已有nft0.9.3/conntrack1.4.5只读复用；默认Rust1.97.1不作为本计划工具链，将用官方固定1.95四component+x86 SHA。

控制新增network-ssh-runner.py，使用/tmp/khor-netlab-feat-net-1110的owner/每run目录、单槽flock，本地盘专属CARGO_HOME/target/tools，不在NFS home写缓存/源码，不改默认Rust/系统包。编译jobs16/nice10，测试filter/线程/重复次数/门由原network.sh保持；比较时严格串行，单场景线程1。source用无父snapshot bundle/SHA256/tree核验，本地immutable ref/远端准确source/control/report；不是release attestation。prepare/compile/fixture/产品红分别归类。VZ未提交guest产品原型已保留到evidence/net/vz-product-prototype-before-turing，控制树不再接入该原型。下一步实际调度Turing，既有Linux原红和N2–N5未结。


**turing-20261109T075531Z-b6277f48 Turing netlab-recovery在途**：PID4083030，remote/tmp/khor-netlab-feat-net-1110/runs/turing-20261109T075531Z-b6277f48；basec55d74ddaf697f7a389c2bc377d1e0016c47adcf、treeef392bb746c998a10c0b6cf8b14f40ffcfe2a2aa、snapshot8ff966dbf71bb0c027bbeac2277ccce6e315dbcf、control59dc223ad263056bc0fa5c073c69a9c5d66b592a；第126个immutable ref已锚。源码/driver经SSH传准确bundle/hash，不走Mac或托管Linux。先固定工具链/编译，产品结果待实际log，禁止重复调度。receipt在/Volumes/disk0/Khor/evidence/net/turing-20261109T075531Z-b6277f48/receipt.json。


基线turing-20261109T075906Z-5464f2db已stage但**未launch**，等待candidate turing-20261109T075531Z-b6277f48终态后串行启动。base6485cb83d3b1f0bb93d4633825dc956623d78af9、tree52a442603df49fb0f3e9821f5c9c9fb9ce4da7e5、snapshot9e64c63c8a81011e8aad1e09c0d6b2aeedb9516b、同control59dc223/同netlab-recovery三轮；127th ref已锚。baseline仅原六fixture变化，五个实际netlab fixture/script与candidate逐字一致；生产6485不改。receipt=/Volumes/disk0/Khor/evidence/net/turing-20261109T075906Z-5464f2db/receipt.json。


### Turing恢复命令与启动顺序

候选remote `/tmp/khor-netlab-feat-net-1110/runs/turing-20261109T075531Z-b6277f48`；基线remote同runs下`turing-20261109T075906Z-5464f2db`。本地分别`/Volumes/disk0/Khor/evidence/net/<run_id>/receipt.json`。只用既有SSH alias turing，`-o BatchMode=yes -o ForwardAgent=no -o StrictHostKeyChecking=yes -o ConnectTimeout=10`。恢复先读取remote `report/execution.json`并核`/proc/<pid>/cmdline`包含该run的input/network-ssh-runner.py；此处已实核PID4083030 live。只凭status文件不得认verified wait，SSH暂时失败也不重发。完成后保留report/launcher原日志及准确source/control。

基线未启动。candidate终态并下载留证后，使用`/usr/bin/python3`通过subprocess.Popen启动该基线input/network-ssh-runner.py，stdin=DEVNULL、stdout/stderr=该run/launcher.log、start_new_session=True、最小HOME/USER/LOGNAME/LANG及系统PATH环境；记录真实PID到该baseline receipt。不要在锁忙时并行启动/把admission红算产品红。runner内部flock保持单槽，编译jobs16，原network.sh recovery是test_threads1/repeats3。

对照必须承认两套机器不同：旧托管Linux与Turing x86_64相同ISA仍非同硬件，不能直接取历史差值；先Turing同配置baseline/candidate。五个paths/hop/lab/nat/netlab.sh已逐字一致，baseline第六改动仅dev.py；其余生产6485。最新candidate网络生产仍d4a2e115，后续仅工具/文档，无重定义SLO。


最新观察2026-11-09T08:05:25.056040+00:00：candidate /proc PID4083030命令行匹配仍live，phase=network-driver，child4090915；toolchain rustc1.95.0/cargo1.95.0/rustfmt1.9.0与四SHA已实核，namespace前置通过，remote source/snapshot/control与receipt一致。CARGO冷依赖先出现多次30秒超时，随后真实Downloaded条目持续推进，未因此重启任务。已有NFS cache 997锁包中815存在（含非Linux包，不能称完整）；当前运行保持，后续baseline复用项目本地cache，未改正在使用的cache或NFS。详细live观察存/Volumes/disk0/Khor/evidence/net/turing-20261109T075531Z-b6277f48/status-observed-20261109T080506Z.json。当前无产品测试结论，下一步继续poll同run；baseline仍staged未launch。


Turing首次产品编译实际通过（6m19s，Noq/Iroh均项目vendor准确路径），现phase=network-driver；2026-11-09T08:09:41.852879+00:00实核进程live=True。部分场景结果已出现（见/Volumes/disk0/Khor/evidence/net/turing-20261109T075531Z-b6277f48/status-observed-20261109T080923Z.json），仅部分、未终态，不把它们算完整三轮通过。基线仍staged；收candidate完整报告再launch。当前完整目标仍active，上一goal轮VZ能力/本轮Turing实际源码编译与运行均为progress，非blocked。


候选turing-20261109T075531Z-b6277f48终态 **SUCCESS6/6（3轮各2）**，完整report已取回并核source/control/driver/bundle/hash，原PGID无live剩余。p3最慢[518.5, 597.9, 706.9]ms、各<原1000ms，零失败/退役；p2三绿。只收recovery，不是CPU/hop/full或N2–N5全绿，未跑根fmt。126ref终态。

**基线turing-20261109T075906Z-5464f2db已launch，PID4135341**，remote/tmp/khor-netlab-feat-net-1110/runs/turing-20261109T075906Z-5464f2db；base6485/tree52a44260/snapshot9e64c63c，同control59dc223与原三轮recovery。准确receipt更新launched。当前127refs=126终态+baseline在途；不可重发基线。收藏器已提交control576c12769beef5f7ca08340f1df18ee87b019d8f，仅新增本地采证工具/2测试；执行中的两run仍绑定59dc223，不挪写controlSHA。


本轮progress：候选完整3轮recovery实际6/6绿，已取回原始report并核source/控制文件字节/hash/原PGID无live进程；新增安全采证工具并实际使用；baseline准确启动并实核/proc命令live，编译55.57秒完成后跑场景。对照尚未完成，不由候选绿收整体。当前唯一下一动作是收baseline同句柄，不能再launch它。

采证命令（终态后）：在control worktree运行 `PYTHONDONTWRITEBYTECODE=1 python3 scripts/collect-network-ssh.py /Volumes/disk0/Khor/evidence/net/turing-20261109T075906Z-5464f2db/receipt.json`。它先核terminal和原group无活进程，取tar不跟链接，核source三元组/任务/control driver实际字节/bundle/environment，保留每文件SHA。失败报告仍保留；原断言红与executor/compile红分开。collector source576c127不改变两个job执行control59dc223。

之后按原门Turing同机串行跑candidate与baseline的hop（f4/f4s/f4i三轮）、stalls（600秒/20×300ms CPU的0/0移动等），再dynamic/full准确源矩阵；f4s回退未定因。需要新任务时复用immutable源码可省上传，但每run新receipt/ref/job目录/任务，不能把老run改task重跑。旧托管机器与Turing的差值不算产品加速；本机编译与Rust依赖仍禁。手机/两电脑OS细节未答不阻断独立N1–N5工作。


下一批四任务只stage，**均未launch**；仍等待正在跑的baseline recovery终态/采证。按candidate hop→baseline hop→candidate stalls→baseline stalls串行，准确队列在evidence/net/turing-next-matrix.json：
- turing-20261109T082539Z-a6f5aaf4 candidate netlab-hop
- turing-20261109T082540Z-58971383 baseline netlab-hop
- turing-20261109T082540Z-c56d74b3 candidate netlab-stalls
- turing-20261109T082541Z-f8a5a7e2 baseline netlab-stalls
同control59dc223原profile，复用各role准确source bundle；每run独立job/receipt/immutable ref，131refs=126终态+当前baseline live+4 staged。不修改旧run任务、不并行计时。


Baseline recovery turing-20261109T075906Z-5464f2db终态FAILURE5/6：p3三轮[787.0, 787.1, 704.3]ms均过1秒门，p2第三轮p95247.204224ms超过原200ms门、其余route/保持/零错/零退役条件过；原始断言失败已核、完整report已采/source/control/字节/hash/group已核。主机load从candidate结束9.12升至baseline结束46.12，资源条件变动；保留此观测红，不据顺序前后直接称源代码加速或patch37定因。对照JSON保存在turing-recovery-comparison.json；同机配置相同不等于背景资源完全相同，仍需CPU受控/复验。127ref终态。

**Candidate hop turing-20261109T082539Z-a6f5aaf4已launch，PID4163640**，control59dc223、source同candidate c55/treeef392bb7/snapshot8ff966db；原f4/f4s/f4i单线程各3轮，没放宽门。当前131refs=127终态+candidate hop live+其余3 staged。后续按队列baseline hop→candidate stalls→baseline stalls严格串行；不得重启当前任务。


本轮progress：完整采证baseline recovery 5/6并核原200ms断言、三轮p3值与host负载变化；候选6/6没有被扩大为因果/全矩阵胜出。四后续任务准确stage且remote JSON/control字节已核，没有运行staged jobs。candidate hop已实际启动，剩baseline hop/candidate stalls/baseline stalls见turing-next-matrix.json；131个immutable REF:source逐项已核。下一步先收candidate hop；如果f4s红，保留原profile结果后走诊断，不改门或重跑覆盖旧红。继续完整N0–N5，goal active。


本轮并行独立机器验证（无子agent）：Turing仍candidate hop PID4163640，第一轮f4s红，all3继续；完整首轮scenario已只读复制first-round-observation。静态/原始事件证实该轮Y→X旧wire retired unanswered，之后started carried新wire，Link约8.9s已Forwarded，但f4_stall只计selected、忽略started，因此45.3s relay统计需独立修正/复验；不是据此否定所有旧f4s红，旧回归仍保留。待全3轮与baseline。

**Mac 42587980042 net-node在途**：base54d58ddca5842b3f0860136952f4f22cba9a9bca、tree6ca60d934b5a9f3262a819d39f62ef4da2df8805、snapshot21c5105122b9bab8e606aa6d8efc8f6e3041450f、control576c127；132nd ref已锚。仅wire.rs测试新增UDP gate真静默与长ACK-positive控制，生产未改；新覆盖不能预记绿或冒称旧源行为红。独立Mac不与Turing计时争资源。黑洞test当前边界4.1秒位于ACK floor后/all-path grace前，还须审查不要把这个过早退役边界冻结成最优设计；可改为真实长静默后abandon已poll一次的真实request future，验证Drop guard而不钉提前退役政策。


42587980042终态仅FORMAT红（无新测试执行），原始format.patch/task/source已存，132ref终态。已基于远端patch手工排版；黑洞覆盖调整为实际request future先poll入response wait，等真实ACK+all-path grace后abandon，避免固定过早4.1秒retire政策，生产仍未改。

**Mac 42589161986 net-node在途**：base9368c5372b99225c058360aa15e5ff323c74b882、dirty（STATE WIP）、tree877fb49295297ab7d19ac336eb56ab6119c234f2、snapshot9e04578396e61384ce5c4c0c0dfbf51dd8449225、control576c127；133ref已锚。测试尚未验证，后续勿把formatter门算行为红。Turingcandidate hop第二轮3/3绿，首轮f4s仍保留红且静态/原始事件支持selected-only误计新wire started carried，第三轮继续。


Turing candidate原hop128th已终态 **FAILURE8/9**（2/3、3/3、3/3），完整report已取回。唯一第一轮f4s旧统计45.3s未回，事件明确Y→X retired unanswered→started carried新wire，第一轮link约8.9s回Forwarded；旧selected-only reducer漏掉started。原始红永久保留。

独立同源reducer已原样抽取到Turing用固定Rust1.95运行（无Cargo/产品/全netlab，单槽无其他计时）：evidence/hop-timeline-original-audit，134th ref，compile0、4tests中2pass2fail。实际两红：漏掉新started carried的恢复、漏算新started relay的占用；opened候选不算胜出、真实10.9s后回退仍失败两控制通过。未改原20秒/1.5秒/30秒门。最小修复只将started纳入占用状态，selected move计数/独立f4i不变，日志新增wire starts；待绿审计与同fixture两角色复验，不能据纯函数即收hop。

Mac133th 42589161986仍仅formatter门失败（一处链式排版），没有行为测试。已从Turing同Rust1.95/rustfmt1.9配置取得两份格式化副本并逐SHA核旧源后应用，本机未执行Rust。wire.rs长abandon/ACK阳性覆盖仍待实际runtime绿；生产未改。


134→135独立reducer实际2红→4绿已完成，固定Rust1.95、源码片段/控制SHA/四测试日志均在hop-timeline-original-audit与hop-timeline-fixed-audit，format副本也有逐源hash。修复只纳入started作为占用状态，opened不算胜出、真实后续fallback仍红、20秒长占用仍红，move计数/独立f4i未改。尚未称全hop绿。两份远端格式化副本按源SHA核后应用，本机没跑Rust。

旧candidate hop128th终态8/9、report/classification留存。旧baseline hop129th只staged未launch，因已知计量缺陷标superseded_not_launched；保留其ref/job/receipt且不运行无效指标。马上新建两角色修正fixture hop任务，原20s/1.5s/30s、拓扑/注入/重复次数不变；已核baseline仍仅原六fixture变化且五实际fixture与candidate逐字相同，生产6485原样。旧stalls130/131未launch且不使用此reducer，保留串行计划，最终准确源矩阵还需复验。wire.rs两新覆盖runtime仍待Mac，133仍fmt-only红，135纯reducer不替代它。


修正reducer后两角色新任务均只stage、未launch：
- turing-20261109T091323Z-60e73cac candidate
- turing-20261109T091325Z-2c64555c baseline
均control59dc223、hop_occupancy_started_v2，原f4/f4s/f4i三轮；新队列candidate hop→baseline hop→candidate stalls→baseline stalls在turing-next-matrix.json。旧129未launch已superseded，原candidate8/9保留。candidate复用Macro f45a5a...的同一snapshot a828463e/tree78d063d9，独立bundle；baseline新snapshot=9012628386be6fc6b9bdc5ca15f4558d79d2f16c tree=97040f31c547e90bf21703a396d14bda455c17c2（仅原六fixture变化，生产6485原样）。


**修正fixture candidate hop turing-20261109T091323Z-60e73cac已launch，PID62854**，tree78d063d915f033d9bffa58516488b65de9f0f14b/snapshota828463e8bc10dc9038e38e3e4777acac21cb443/control59dc223，measurement_revision=hop_occupancy_started_v2。baseline修正hop turing-20261109T091325Z-2c64555c仅stage、未launch。后接两个旧stalls。

**Mac 42591501822 net-node已dispatch/138ref锚**，同candidate hop source(tree78d063d9/snapshota828463e/basefd1f30ee clean)，执行controla6a00a5fea4ee4e5ec9a5be3f8f07b8e767386c4，格式化来源固定Turing1.95/rustfmt1.9。待16预算/流/探针专项和既有子集实际结果，不预报绿。两个独立机器可同时运行，Turing只有一个计时job。


Mac138th 42591501822终态：root fmt/编译/net66+1ignored/hop9/wires10/journal28绿，dial_budget15pass1fail（14.56s）。新长ACK阳性通过；唯一黑洞测试在静默前置就失败（heard_count12→15，不能当真实黑洞），仅fixture红，不是产品退役行为红，原因尚未区分尾包/绕行。当前15过含旧14合同，新F7未验证。

已移除UDP proxy fixture，恢复原native loopback Fixture；真正长静默改为Linux-only子测试，unshare -Urn在启动子harness前建私有user/netns，核网络namespace与parent不同且uid_map单映射后才up lo/写nft，精确drop两端server UDP端口并核计数/全程heard不变。主机网络不动，Mac不运行该Linux测试，但长ACK阳性/旧合同仍继续。Linux test必须排到Turing单槽后实际运行，不能以Mac15绿代替F7。新增server task白名单net-node以复用原可信profile；未改profile/资源门/生产协议。


**Mac 42594133373 net-node在途/139ref已锚**：baseef90f8bf02e024bcb8d51c1a223822e05b307ec2/tree3f3c4c52e0e8857448a921b567b5469ca7c70b74/snapshot800a54f8e5ad6d67fc33db6ed15228ccef86de65、controlb998479（仅Turing增加net-node allowlist，既有profile不改）。true F7为Linux-only新namespace，不在Mac假报通过；Mac验证其余15合同与格式。下一真正Linux net-node需Turing单槽排队并准确source，不得与hop计时并跑。当前corrected candidate hop首轮f4s绿、独立f4i红（未被此次reducer改变），第二轮继续；新红待原始断言/事件定因，不能据f4s计量修正把所有hop收绿。


Mac139th 42594133373已终态并采证，准确source/tree/snapshot/control及bundle-transport核一致，headSha=b9984797；仅根format门失败，没有编译或行为运行。format.patch原样应用，未在本机执行formatter。Linux-only F7需下一Turing net-node实际证明；Mac排除它，不能报全16绿。

corrected candidate hop第一轮f4i原始事件已取回：X selected carried@96.8→relay@98.5→carried@104.3；断言to_relay=1而原门0，Y first started relay@88.1满足首路径条件。回退时X measurements只剩Relay，先前两路径存在；日志无connection stable_id，不能仅凭这些行区分旧retired watcher与实际持有wire，仍保留行为观测红并等待基线/进一步诊断。未放宽f4i、未修改生产。第二轮3/3，第三轮继续。


136th corrected candidate hop已终态FAILURE8/9，完整report/source/control/bundle/字节/原PGID与SHA已核。f4s三轮全部通过原门，独立f4i首轮仍有1次selected relay，第二/三轮绿；原始报告不覆盖。首轮X carried→relay间隔1697ms、relay→carried5803ms，精确event见first-round-observation/f4i-analysis.json。第三轮f4i回carried91.4秒虽低于旧120秒宽上限，仍是体验缺口，非极致完成。

**140th turing-20261109T094205Z-595cfb65 Linux net-node已launch PID143315**：base6fff96a389d74aa0524257a4d4c250654bae5b90 clean/tree6099df332c86ff7c4224cbcec0d22ec1fb4ba28f/snapshot679507082f2343b1ba4ab36d475c8ea7ee328b22/controlb9984797f412f352f3c76f99c47ade3ee10f68e9。根格式通过、正在编译，Linux F7与15便携合同仍待实际结果。receipt在evidence/net同run目录；当前只有这一个Turingjob。完成采证后launch既有baseline hop137th，不能重复创建/并跑计时。

额外静态风险仍未修：RetireUnheard::drop只看ack floor与无收包，exchange_inner却先ask_every_path再等grace；取消发生在4.1秒、备用路径4.8秒才回包时，两处判死边界可能不一致。真实早取消/活备用反例与统一活性所有权待验证，不以长ACK阳性或新长静默F7推定该边界安全。


140th Linux net-node终态FAILURE：root fmt/编译及其余子集绿，15合同通过，新F7在nft规则解析时失败（unexpected closing brace，尚未生效），夹具红；完整report已采/source/control/group/hash已核。仅将规则改为多行分隔，先新namespace独立check语法，再新source重跑，生产未改。
