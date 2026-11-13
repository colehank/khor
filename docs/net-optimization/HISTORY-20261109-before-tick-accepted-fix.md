# feat/net 当前状态与恢复入口

更新：2026-11-09 11:53 UTC；root串行，无子智能体。活动目标“完成计划”仍active。

**当前Turing：157th turing-20261109T114555Z-10025913 net-node，PID365030/child365065。** 已实核/proc与source/control，编译/旧合同/新F9/F10运行中，生产仍25d0c803。156th Mac同准确snapshot e20f36dd/tree0552f630/base df6188c5/controlb998的19便携全绿；新增Linux-only两项未预记结果。154th旧生产同源hop三轮仍stage未launch；先157取得真正结果再定修复/准确源复验，不重发。157 refs=155终态+157在途+hop154 staged。完整N0–N5未完成。

CPU原版153与候选152均原门SUCCESS2/2、完整report已采核。候选0/0移动、最慢3.8秒、12打洞；原版1/0移动、最慢732.4ms、5打洞；均零退役。p5三档均零relay移动/零打洞：候选279.9/518.6/992.0ms，原版310.1/499.8/976.0ms。只是一对顺序运行的最大值/原门观察，不能宣称整体或单补丁提速。p1s原门允许每端最多一次移动、零退役，并不约束长尾/打洞；300ms尖峰实际是CPU争用下的道路延迟注入，原helper忽略taskset结果且未留有效负载证据，最终性能结论前须校验。完整对照见evidence/net/turing-shared-probe-stalls-comparison.json。

**F9/F10新增回归准备**：155th Mac42611015315仅format红、无编译/行为测试；原format.patch已应用。156th Mac42611507471同准确tree0552f630/snapshote20f36dd/base df6188c5/controlb998，fmt/编译/原有net66+1ignored/hop9/wires10/journal29/Mac19全绿，Linux新F9/F10排除。157th turing-20261109T114555Z-10025913已launch PID365030，完全同156准确snapshot/tree、连bundle SHA也同ef1b74f9；生产仍25d0c803，仅增fixture与合同。baseline153已终态采证，当前优先跑157验证真实活性反例，再定hop154/新准确源复验；此顺序调整为了先确认共享连接的巡检/Accepted所有权风险，没有新行为红/绿。
完整范围：[PLAN](PLAN.md)；约束：[REQUIREMENTS](REQUIREMENTS.md)；运行记录：[RUNS](RUNS.md)；原始结果：[REPORT](REPORT.md)。
此前STATE原样归档[Accepted轮历史](HISTORY-20261109-before-accepted-green.md)与[本轮历史](HISTORY-20261109-before-probe-green.md)及[Turing141前历史](HISTORY-20261109-before-turing-141.md)与[共享确认绿前历史](HISTORY-20261109-before-shared-confirmation-green.md)；历史在途/候选文字仅代表当时状态，恢复以本页和实际工作区为准。

## 授权与工作区

速度/体验优先，个人可信设备网；活动目标授权N0–N5实施验证。没有合main、发布、中继采购/部署授权。
本机source-only：不运行Cargo/Rust/Node/Xcode编译、测试、formatter或下载可执行物；Git/Python stdlib/脚本检查可用。重型Linux沿U9授权Turing、Mac原生沿macserver/GitHub，不碰现装App、生产store、个人会话或其他任务树。
图谱先用，覆盖不足才定向读；不新建CodeGraph索引。外盘布局/治理入口遵守AGENTS，本任务不接管代码治理。

| 用途 | 位置 / 当前基准 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；当前生产候选25d0c803d153cb3476125ec2a6a197460e558911；共享确认修复已有Linux21/Mac19同源绿；选路vendor未改，准确源全矩阵待验，恢复核HEAD/diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，b9984797f412f352f3c76f99c47ade3ee10f68e9已push；新增net-node SSH任务白名单，profile未改；新CPU两run用b998；历史hop/recovery仍control59dc223，不能回写历史控制版本 |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture，生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json157条不可变refs；155终态、Linux157在途、candidate hop154 staged。每次源变化前锚ref；SSH不是GitHub run |

## N2：期限/流/探针与共享活性确认

最新150th Linux21与151st Mac19同源码均全绿：fmt/net66+1ignored/hop9/wires10/journal29；F8慢原生UDP路上的4.1秒到期保持同wire控制请求、F7真实黑洞确认后恰一次退役、外部thread取消、replacement与Weak owner生命周期均通过。Mac19排除Linux-only F7/F8；两集合不混算。以下保留先前真实红绿链，不代表完整N2已收口。

**42571393016 Mac net-node SUCCESS**：based4a2e115b898fc337f5091b51fa71e45740c667f clean、treef278b9330f51268182b142729db2c86687f4af4b、snapshot3459894c81cb994310a97bfcba4ca39ae97e569b、control0cff9671fb2f80945423132e0df46995f6ff3198。
mac-probe-owner-candidate-42571393016保存source/environment/raw/classification；根格式、net66pass+1ignored、hop9/wires10/journal28/dial_budget14全部绿（预算/流/探针9.93秒）。仅Mac便携子集，不是完整Quality或N2全包。

| 边界 | 实际红 | 实际绿 / 限制 |
| --- | --- | --- |
| normal/early拨号锁期限 | 42556768499 / 42557961726 | 42557583517 / 42558820303及最新；40ms到期不改共享结果/peer down/backoff，过期空锁不拨号 |
| 操作scope到真实ask | 42562932053两红：忽略40ms、300s被28s截断 | 42566727617首次绿，最新保持；只是持锁边界，不代替长工作/全Op |
| Accepted回答所有权/期限 | 42567527752两红 | 42567946141及最新；取消/到期STOP本流、同wire独立Sessions、相同stable_id/严格3请求/零dial_failed |
| 普通交换/一次重试/写失答 | 新增覆盖，未声称旧源行为红 | 42569954515四项首次绿，最新保持；1秒普通exchange到期不伤并发请求；2秒原by剩余700ms进入真实读重试并到期；失答Sessions严格2次、Mkdir严格1次 |
| probe直接取消、配对外层取消、首成功 | 42570772550实际11pass3fail | 42571393016及最新绿；真实HTTP GET和fast已204前置后取消，800ms内TCP EOF，第二URL不被slow first挡住 |
| 调用过期与全路probe宽限 | 143rd实际16pass1fail，4.1秒到期关闭共享wire | 150th Linux21绿，F8函数逐字未变；151st Mac19保持便携合同，F7真实黑洞按独立确认退役 |
| 确认任务生命周期 | 新增覆盖，不冒称旧源行为红 | 150/151实际replacement保持新wire并成功Sessions，pending任务不保留Node/Wires强引用 |

resident入口一次建立原Op档5/30/300/3600秒减.5秒的不可变Tokio by，任务局部只承载值；锁/hop/有界拨号/延迟候选/普通交换/一次读重试显式沿用较早截止点。CallerExpired本身不记peer失败/退避；之后独立全路确认确实无回应，仍可记Unanswered。
独立ask无Op时由Request工作预算加现有dial25s/hop3s/最大road120s合成外层上限，各阶段仍可先结束。更新dial原10秒自身上限已由Mac146真实UDP黑洞覆盖：30秒scope仍约10秒返回；200ms scope先到则约200ms返回。
Accepted交接保留Aborts，adopt后按原by等答；取消/到期停止本流、保留共享健康conn。Rejected和未adopt新conn清理、读写/0RTT准入/frame/IPC半关闭未改。
新普通wire控制包含真正同时等待的第二请求，首流STOP后才回答；零读重试/answer_late/peer failure/down/backoff/未答退役。真实retry仅一次closed退役，第二条仍live。Mkdir为代表写请求覆盖，未宣称穷尽所有动作的幂等专项。
probe内部JoinSet按完成取首成功并取消输者，pair_with用Aborts闭合外层；dial成功先drop probe再pair_over，所有早返也清理。原并发量/2.6秒单HTTP/proxy/空与全失败false/诊断fallback不变。图谱还确认ears_heard直接await该函数，返回值映射未改，不能据此声称其完整故障/回声场景通过。
RetireUnheard长静默取消只发起/加入Wires::Slot拥有的确认，normal exchange共用相同count/by/grace；每wire至多一个任务，任务仅持Weak。收包、adopt替换与清理wire均取消旧任务；捕获poll时runtime，std thread Drop仍能继续确认。失败在slot锁内提交、adopt发布新conn后清down，防旧确认晚写新状态。裸未持有wire的pairing连接保留原本地probe合同。F7观察v2保留真实黑洞，只将“同步Drop关闭”改为owner自身grace+500ms内确认；这是为了全路证据，不能一律去掉死wire退役。

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

1. **当前Turing net-node**：157th已launch PID365030/child365065，实核live；先收真正F9/F10与原Linux21结果/完整report，再决定生产修复与准确源hop三轮。CPU152/153已采证均2/2，保留3.8秒vs732.4ms、12vs5打洞差异；原profile的CPU亲和与busy loop效力需先补可断言证据，不把两次观察当因果提速。154th hop旧准确源仅stage，不重发；若生产更新则保留旧ref并给最终源另建run。之后recovery/dynamic/full矩阵和f4i连接身份定因仍需推进。
2. **N2剩余合同**：F8本批已红→绿，F7真黑洞/外部thread取消/owner与替换已实证；update10秒与200ms by已绿。长工作、多阶段、hop两middle合计3秒、dial输者和延迟候选资源/过期、独立后台预算仍缺。还要核resident tick独立silence/network门与较长probe grace的协调，以及Accepted长无应答取消，不能由本批直接Node合同替代。pair/doctor两层已修；代理listener/传输job/Visit/nudge有意独立，不继承结束的前台by。
3. **IPC边界**：client Op外层与serve接收后deadline起点不同，尚未跨进程传递剩余预算或取消。正常exchange本来就shutdown发送半边，服务端EOF不是取消；不得为贯通而重放已发送写请求。具体原源码hash见deadline-continuation-20261109。
4. **N1**：Turing已恢复产品验证；旧候选hop8/9、baseline7/9的红及约10秒/91.4秒体验缺口保留；本次owner修复后的准确源完整矩阵、连接身份RCA仍待。Mac单元不能替代namespace/netem/nft实测。
5. **N3**：真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平推进、共享vs分连接、1/3/5设备后台、真实手机能耗，均需独立负载数据；不增加总资源伪造调度胜出。
6. **N4**：默认北京HTTP/TLS同机、自家登记集中北京、香港默认无独立pkarr；复用已有多登记/选家/继任机制，尊重only_mine/n0配置。双端完整/不对称路径、冷暖发现、整机/登记/出口故障域及容量实测和拓扑方案未完成，不采购/部署。
7. **N5**：48初步补丁（Iroh37+Noq11）仍需覆盖收口；Noq6计时42543928044、Noq4备用ACK隔离42539256792、Noq2/37本轮前绿保留。Noq1/5专项、其余边界、固定1.4.0逐行为等价尚缺；UPSTREAM静态hash不是上游运行，不升级/删补丁。原生Mac/代理/多机/手机/能耗与最终可信Quality仍缺。

本轮用户询问进度/工期时，已给出当前N2这一批粗估1–2小时、全N0–N5粗估3–5开发日且以Linux与真机条件到位为前提；这是粗估，不是缩范围或固定完成承诺。当前Linux21/Mac19预算/流/探针子集已实际绿，但后续范围仍如上。

## 证据与恢复规则

先核worktree/branch/HEAD/diff、当前run与准确回执；以本页顶端当前任务为准，历史在途文字仅在归档中保留。157refs=155终态+Linux157在途+hop154 staged；恢复按run/path/PID实核，SSH观察失败须重查同句柄，不重启。
`git rev-parse refs/khor/net-evidence/<run>:source`必须等于回执tree；只git show读容器，不checkout覆盖/push本地证据refs。失败明确分format/compile/fixture/行为/执行器；tests_executed只按实际子集，历史红不删除。
Mac source/bundle分支只在终态、证据留存且所有权/SHA匹配后清理；复用源的请求不拥有原源分支。原attempt不覆盖。
外盘缺失只读`git show feat/net:docs/net-optimization/STATE.md`与PLAN；原外盘未提交内容不能按摘要覆盖。其他治理沿中央STATE/RESUME，不混授权。

## 当前准确源、运行与下一句柄

| 编号 / run | source / control | 状态与范围 |
| --- | --- | --- |
| 143 / turing-20261109T100821Z-08748d90 | base5aa96708/tree42050b56/snapshoted287397/controlb998 | 真实F8红，16pass1fail；全部隔离/两请求/时序前置先过，原report保留 |
| 150 / turing-20261109T105401Z-df35bc80 | base25d0c803d153cb3476125ec2a6a197460e558911/tree6805b7b71472b9a872721b00acd744081f23507e/snapshote0a42573bba599f022e82cb6c3b039f2897c34fd/controlb9984797f412f352f3c76f99c47ade3ee10f68e9 | SUCCESS Linux21；fmt/net66+1ignored/hop9/wires10/journal29，原group/全部report已采核 |
| 151 / Mac42605391960 | 与150完全同snapshot/tree/base/control，独立request50938337986345e1ad6f1d794af852fa | SUCCESS Mac19；同既有子集/29journal绿，Linux F7/F8排除；完整report/transport已采核 |
| 152 / turing-20261109T110456Z-dee800e0 | 与150同source、controlb998 | SUCCESS2/2，完整report已采；p1s0/0切路/零退役，但3.8秒按键/12次打洞，p5三档全绿；不是完整体验绿 |
| 153 / turing-20261109T110456Z-42e6ec16 | base6485cb83d3b1f0bb93d4633825dc956623d78af9/tree97040f31c547e90bf21703a396d14bda455c17c2/snapshot9012628386be6fc6b9bdc5ca15f4558d79d2f16c/controlb998 | SUCCESS2/2，1190.62秒；原p1s1/0移动、732.4ms/5打洞，p5三档全绿；完整report采核，不能当整体提速 |
| 154 / turing-20261109T112647Z-c5e5e53d | 与150完全同source/controlb998 | candidate netlab-hop仅stage未launch，hop_occupancy_started_v2、原单线程三轮，先153再157新回归再本项 |
| 155 / Mac42611015315 | basefdaf20f8/treee58c548b/snapshot8f52df34/controlb998 | 仅format红，原patch已应用，未编译/运行 |
| 156 / Mac42611507471 | basedf6188c5/tree0552f630/snapshote20f36dd/controlb998 | SUCCESS：原19便携与基础子集绿，新增Linux-only F9/F10排除 |
| 157 / turing-20261109T114555Z-10025913 | 与156完全同准确source/control，bundle SHA ef1b74f9 | net-node已launch PID365030/child365065，新F9/F10与原Linux21待结果，baseline153已终态采证 |

F8固定函数与红源逐字相同，SHA与两个snapshot核见evidence下f8-source-identity-red143-green150.json。F7的v2观察改变已先写ACCEPTANCE，仍同namespace端口丢弃/零新收包/远端open/一次unanswered/零其他故障。150下21合同46.37秒、151下19合同32.31秒均是子集运行时，不是产品SLO。

147th只是format红；148th Turing与149th Mac同旧snapshot ef83ab11，仅node编译Arc未限定红（net66绿，没执行node合同）；源码已仅补std::sync::Arc后得到150/151绿。原始失败与精确source/control/report均保留，不混为行为红。原旧stalls130/131未launch已标superseded_not_launched，没有测试红/绿。

新CPU两角色五fixture SHA在turing-stalls-shared-probe-fixture-identity.json，Bash network.sh与原59dc版本逐字相同；不是改实验门。队列evidence/net/turing-next-matrix.json已替换为152→153，原队列归档before-shared-confirmation-stalls。root=/tmp/khor-netlab-feat-net-1110；每run runs/<id>。恢复先核execution.json与/proc/PID/cmdline匹配input/network-ssh-runner.py；SSH观察失败重查同句柄，不重发。终态用control scripts/collect-network-ssh.py对应receipt采证；baseline153原group已采核，157th net-node当前独占；其终态采证后再决定candidate hop154/新源复验，当前无其他Turing重型任务。

旧corrected hop136th候选8/9（f4s3/3、f4i2/3）与137th基线7/9（f4s1/3、f4i3/3）都已采证，whole-source观测不证明单patch因果；新owner源尚未跑这些场景。新journal Path/Retired wire身份兼容已过29测试，真实多连接关联定因仍待；不能再用旧注释“一peer一份路径选择”推断当前Iroh，它按conn分配path_state。

用户设备组合iPhone＋两台不同网络电脑已确认，型号/iOS/电脑OS异步问题仍待回复，不重复询问或阻断独立工作。最近工期粗估本批修复/自动化对照1–2小时、完整计划3–5开发日，受实际失败与真机可用影响；不改变范围或构成完成承诺。当前本轮为progress，完整goal保持active，无合main/发布/部署。
