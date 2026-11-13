# feat/net 当前状态与恢复入口

更新：2026-11-09 10:51 UTC；root串行，无子智能体。活动目标“完成计划”仍active。

**共享确认候选仍待行为验证。** 147th仅format红，远端patch已应用；148th Turing根格式/net66+1ignored绿，node编译因两处Arc未全限定失败，无node合同运行。已仅补std::sync::Arc，下一新源重跑。149th Mac42604002430正在验证与148th相同snapshot ef83ab11/tree473e41ce（独立request/ref），待实际artifact，不能预记同因失败。149 refs=146终态+Mac149在途+2 staged（旧stalls）。完整N0–N5未完成。
完整范围：[PLAN](PLAN.md)；约束：[REQUIREMENTS](REQUIREMENTS.md)；运行记录：[RUNS](RUNS.md)；原始结果：[REPORT](REPORT.md)。
此前STATE原样归档[Accepted轮历史](HISTORY-20261109-before-accepted-green.md)与[本轮历史](HISTORY-20261109-before-probe-green.md)及[Turing141前历史](HISTORY-20261109-before-turing-141.md)；历史在途/候选文字仅代表当时状态，恢复以本页和实际工作区为准。

## 授权与工作区

速度/体验优先，个人可信设备网；活动目标授权N0–N5实施验证。没有合main、发布、中继采购/部署授权。
本机source-only：不运行Cargo/Rust/Node/Xcode编译、测试、formatter或下载可执行物；Git/Python stdlib/脚本检查可用。重型Linux沿U9授权Turing、Mac原生沿macserver/GitHub，不碰现装App、生产store、个人会话或其他任务树。
图谱先用，覆盖不足才定向读；不新建CodeGraph索引。外盘布局/治理入口遵守AGENTS，本任务不接管代码治理。

| 用途 | 位置 / 当前基准 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；最新已验证生产d4a2e115b898fc337f5091b51fa71e45740c667f；后续有fixture/调度与journal wire编号（Mac146便携子集已验）；选路/预算逻辑仍d4a，恢复核HEAD/diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，b9984797f412f352f3c76f99c47ade3ee10f68e9已push；新增net-node SSH任务白名单，profile未改；当前hop/stalls仍control59dc223，不能回写历史控制版本 |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture，生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json149条不可变refs；146终态、Mac149在途，其余2 staged。每次源变化前锚ref；SSH不是GitHub run |

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
独立ask无Op时由Request工作预算加现有dial25s/hop3s/最大road120s合成外层上限，各阶段仍可先结束。更新dial原10秒自身上限已由Mac146真实UDP黑洞覆盖：30秒scope仍约10秒返回；200ms scope先到则约200ms返回。
Accepted交接保留Aborts，adopt后按原by等答；取消/到期停止本流、保留共享健康conn。Rejected和未adopt新conn清理、读写/0RTT准入/frame/IPC半关闭未改。
新普通wire控制包含真正同时等待的第二请求，首流STOP后才回答；零读重试/answer_late/peer failure/down/backoff/未答退役。真实retry仅一次closed退役，第二条仍live。Mkdir为代表写请求覆盖，未宣称穷尽所有动作的幂等专项。
probe内部JoinSet按完成取首成功并取消输者，pair_with用Aborts闭合外层；dial成功先drop probe再pair_over，所有早返也清理。原并发量/2.6秒单HTTP/proxy/空与全失败false/诊断fallback不变。图谱还确认ears_heard直接await该函数，返回值映射未改，不能据此声称其完整故障/回声场景通过。
RetireUnheard仍要求armed、heard_count不变、elapsed>=Patience.ack；短取消不退役，真正长黑洞保护不能移除，Mac138与Turing141长ACK阳性已通过，Turing141真正独立namespace/nft长静默F7也已绿；早取消发生在all-path grace前的边界仍未验证。

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

1. **下一动作：F8已实红，实施共享活性确认所有权修复**，按NEXT-DESIGN末节；不把Drop阈值简单加一秒或取消所有退役来追测试。正常exchange与取消共享全路probe/count/grace证据；取消只结束本流，replacement/新收包/退出必须取消旧确认。先真实F8红→绿且保持F7/长ACK/旧合同，再继续已stage的candidate stalls（c56d74b3）→baseline stalls（f8a5a7e2），不重复创建/并跑。旧stalls源不含后续修复，最终准确源全矩阵仍需重跑。
2. **N2剩余合同**：F7真实长黑洞已绿；F8早取消/全路grace已实红待修，长工作运行覆盖仍缺；update10秒已绿；hop两middle合计3秒、dial输者及延迟候选的资源/过期验证；前台多阶段及独立后台整体预算。pair/doctor探针两层已实际修复，不重复排成未修。代理listener/传输job/Visit/nudge有意独立，不能继承已结束前台by。普通wire四项与Accepted覆盖不能证明完整N2。
3. **IPC边界**：client Op外层与serve接收后deadline起点不同，尚未跨进程传递剩余预算或取消。正常exchange本来就shutdown发送半边，服务端EOF不是取消；不得为贯通而重放已发送写请求。具体原源码hash见deadline-continuation-20261109。
4. **N1**：恢复Linux，独立定位f4s回退、原门复验及准确源完整矩阵。Mac单元不能替代namespace/netem/nft实测。
5. **N3**：真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平推进、共享vs分连接、1/3/5设备后台、真实手机能耗，均需独立负载数据；不增加总资源伪造调度胜出。
6. **N4**：默认北京HTTP/TLS同机、自家登记集中北京、香港默认无独立pkarr；复用已有多登记/选家/继任机制，尊重only_mine/n0配置。双端完整/不对称路径、冷暖发现、整机/登记/出口故障域及容量实测和拓扑方案未完成，不采购/部署。
7. **N5**：48初步补丁（Iroh37+Noq11）仍需覆盖收口；Noq6计时42543928044、Noq4备用ACK隔离42539256792、Noq2/37本轮前绿保留。Noq1/5专项、其余边界、固定1.4.0逐行为等价尚缺；UPSTREAM静态hash不是上游运行，不升级/删补丁。原生Mac/代理/多机/手机/能耗与最终可信Quality仍缺。

本轮用户询问进度/工期时，已给出当前N2这一批粗估1–2小时、全N0–N5粗估3–5开发日且以Linux与真机条件到位为前提；这是粗估，不是缩范围或固定完成承诺。当前十六预算/流/探针测试现已实际绿，但后续范围仍如上。

## 证据与恢复规则

先核worktree/branch/HEAD/diff、当前run与准确回执；以本页顶端当前任务为准，历史在途文字仅在归档中保留。149refs=146终态+1在途+2 staged；恢复按run/path/PID实核，SSH观察失败须重查同句柄，不重启。
`git rev-parse refs/khor/net-evidence/<run>:source`必须等于回执tree；只git show读容器，不checkout覆盖/push本地证据refs。失败明确分format/compile/fixture/行为/执行器；tests_executed只按实际子集，历史红不删除。
Mac source/bundle分支只在终态、证据留存且所有权/SHA匹配后清理；复用源的请求不拥有原源分支。原attempt不覆盖。
外盘缺失只读`git show feat/net:docs/net-optimization/STATE.md`与PLAN；原外盘未提交内容不能按摘要覆盖。其他治理沿中央STATE/RESUME，不混授权。

## 本次实际结果与准确恢复句柄

| 编号 / run | 准确source / control | 当前结论 |
| --- | --- | --- |
| 136 / turing-20261109T091323Z-60e73cac | basefd1f30ee/tree78d063d9/snapshota828463e/control59dc223 | corrected hop FAILURE8/9；三轮2/3、3/3、3/3；完整report已采 |
| 137 / turing-20261109T091325Z-2c64555c | base6485/tree97040f31/snapshot90126283/control59dc223 | baseline hop已采证FAILURE7/9，三轮2/3、2/3、3/3 |
| 140 / turing-20261109T094205Z-595cfb65 | base6fff96a3/tree6099df33/snapshot67950708/controlb998479 | 15pass1fail，nft语法前置红，未安装规则；原report保留 |
| 141 / turing-20261109T095017Z-93c2c1a2 | base9eafba741772138e9dfbe59ca9a010cb843b1ef5/treef93c32b1aa8ef9b7c36d9e3e2437c2beb26851b4/snapshot6708accbbb016be75c10befdbb0d14f1ce002875/controlb9984797f412f352f3c76f99c47ade3ee10f68e9 | SUCCESS；fmt/编译/net66+1ignored/hop9/wires10/journal28/dial_budget16绿，已采证 |

136th三轮f4/f4s绿，f4s腿恢复后0.2/0.0/0.0秒回carried、无后续fallback；最大按键8.6/10.2/10.1秒仍待优化。f4i首轮to_relay=1而原门0，首行started条件通过；X carried→relay1697ms、relay→carried5803ms。后两轮f4i绿但第三回carried91.4秒。没有connection stable_id，未定因，不能归咎started计量修正。原20秒/1.5秒/30秒与零回退门未改。baseline五实际fixture/control身份核在evidence/net/turing-corrected-hop-fixture-identity.json，不能由fixture一致推出性能因果。

141th在新user/netns中先核网络namespace与parent不同和单uid映射，才up lo/写nft；真实UDP端口双向丢弃，核drop计数>0、heard_count全程不变。真实request已poll入Pending，经过ACK+all-path grace后abandon，恰一次unanswered退役、remote仍open、零closed/readretry/dialfailed/late。长ACK阳性和旧14合同也绿。21.24秒为该单元子集运行时，不是体验SLO；新增覆盖绿不冒称旧源行为红。Mac139只format红、Linux140只nft fixture红均独立保留，前者patch原样应用，后者改多行规则后新namespace check实际通过。

额外静态风险未修：RetireUnheard::drop只看ack floor与无收包，exchange_inner却先ask_every_path再等grace；取消发生在4.1秒、备用路径4.8秒才回包时，两处判死边界可能不一致。真实反例与统一活性所有权待验证，不以长ACK阳性或长静默F7推定边界安全。NEXT-DESIGN新增后续反例设计，未预记红。

当前remote root=/tmp/khor-netlab-feat-net-1110；每run在runs/<run_id>，receipt和report位于/Volumes/disk0/Khor/evidence/net/<run_id>。先核remote report/execution.json及/proc/PID/cmdline匹配input/network-ssh-runner.py。F8已终态采证，禁止重发原run；修复源必须新receipt/ref。两个既有stalls尚未launch。SSH观察失败只重查，不重发；status文件不能单独证明进程live。

原用户设备组合已确认iPhone＋两台不同网络电脑；型号/iOS/电脑OS的异步问题仍待回复，不重复询问或阻断独立工作。更新本页/REQUIREMENTS不能缩小PLAN；本轮为progress，完整goal保持active，无合并/发布/部署。


N2下一反例F8已写入wire.rs并冻结ACCEPTANCE：新私有netns的lo netem单向2400ms，两个真实同wire请求；首个4.1秒到期、另一个12秒预算等待约4.8秒UDP往返。核server收到严格两请求/响应时序/到期前零回包后才判共享wire保留。仅新增测试与提取原F7隔离helper；生产仍d4a，F8尚未实际编译/运行或取得行为红。当前baseline hop PID161772 live，不能与新Linux编译/测试并跑；先由独立Mac通道检格式/15便携合同，再排Turing单槽。


142nd Mac42598044232（base2a3b7030/treed6f74af1/snapshotef91edf8/controlb998）已终态并采证，仅format门红，未编译或测试；准确source/control/transport已核。原format.patch已原样应用，F8仍无实际行为红。下一Linux net-node安排当前baseline hop终态后、两个stalls之前；这是短正确性反例，仍保持Turing单槽不与计时并跑。

基线137th首轮2/3：f4/f4i绿，f4s红；Link在腿恢复后约9.5秒已Forwarded，约35.6秒又Relay、37.4秒再Forwarded，原after-leg门失败。完整首轮待独立原始事件核对，保留原断言红，不宣称与candidate136th独立f4i红同因。第二轮继续、PID161772实核live，不能重启。


143rd turing-20261109T100821Z-08748d90 net-node已stage未launch：base5aa96708df80d3bb0ff53b396f32a830151b3b87/tree42050b56a97ce6c37883186e47e36de1fb2f21eb/snapshoted2873974c881ed3ad779fb86c7665c2eaa7c216/controlb998479。含Linux F8预期反例与原16合同，不含之后的两个update覆盖。排在当前baseline137th终态/采证后、两个stalls前，三者不可重复创建/并跑。

新增两个portable update_dial运行覆盖：真正loopback UDP黑洞收到数据后，200ms原操作by先到则返回；30秒by仍保留10秒自身cap。仅测试，尚未编译/运行，不冒称旧源红。与F8合并送独立Mac格式/便携验证，不用Mac绿替代Linux F8。


144th Mac42598977810（base53f5a2fe/treed8239c31/snapshot55325d5c/controlb998）终态并采证，仅formatter一处assert红，未编译/执行两个update覆盖；原format.patch已应用。143rd Linux F8准确stage源不含update覆盖或以下日志候选，不能挪改它的source/control。

为下一轮f4i诊断，journal::What::Path/Retired增加可选process-local wire编号；roads::follow在订阅处捕获同conn的stable_id，Wires::drop_noted记录实际被退役conn的相同编号。老日志无此字段仍可读、人类显示句子不变，新增兼容回归；不改变选路/退役政策或全局去重。此日志生产候选尚未格式/编译/运行验证，已验证选路/预算逻辑仍d4a。Iroh源码确认path_state按conn分配，不能再用roads注释“一台机器仅一份选择”当当前事实；是否误合并日志仍需连接编号的实证，不据此否定已有原门红。


145th Mac42599671755（basec899f0aa/treeddcf8256/snapshotd04f3749/controlb998）终态仅format门红，新的journal兼容回归与两个update_dial尚未编译/运行。准确source/control/transport已采核，原format.patch应用；下一Mac新源继续格式/编译/29journal及17便携合同，Linux F8仍由已stage143源独立验证。


137th corrected baseline hop终态FAILURE7/9（2/3、2/3、3/3），完整report/source/control/bundle/原PGID/逐文件SHA已采核。两红均f4s：第一实际Link曾恢复后约35.6秒又Relay，按原after-leg门红；第二carried后26.2秒又选Relay、原30秒无回退门红。f4/f4i三轮绿。whole-source观测对照在evidence/net/turing-corrected-hop-comparison.json，不给单patch或稳定性因果结论；candidate与baseline两边所有红保留。

143rd F8已launch PID236331，base5aa96708/tree42050b56/snapshoted287397/controlb998；原17 Linux合同包括F7/F8，不含updates两个。146th Mac42600205050准确base83d620d22ed06362ae0f6e2a578881128d869d3a/treebbed65034d97e015f122a81cc242c64d63f967ad/snapshotc8035ce2faa57ed51db8b8fb258b0a6957d74cfa/controlb9984797f412f352f3c76f99c47ade3ee10f68e9，含wire编号与两个update覆盖，Mac17=15旧便携+2update，Linux-only F7/F8被排除。两个17不可混算；均待格式/编译/runtime实际结论。

用户再次询问进度/工期，已明确仍在核心机制阶段、当前排队实验约30–45分钟，完整计划粗估还需3–5开发日、真机/能耗取决于设备可用，回退原因未定有不确定性。这不改变范围、也不是完成承诺。


143rd turing-20261109T100821Z-08748d90已终态FAILURE，完整report已采/source/control/bundle/group/SHA已核。root fmt/编译/net66+1ignored/hop9/wires10/journal28绿，dial_budget16pass1fail，F7保持绿；F8实际行为红。子harness前置均通过，received_after_ms2401/expired_after_ms4101/heard_before16/heard_at_expiry16/requests2；目标断言live_at_expiry None而应Some同stable_id，control_ok=false，retired_unanswered=1。RetireUnheard Drop只按ACK floor判死，与正常exchange全路probe宽限不一致已实证；修复前不launch CPU，下一按NEXT-DESIGN所有权约束实现。

146th Mac42600205050 SUCCESS已完整采证核source/control/bundle transport/hash：base83d620d2/treebbed6503/snapshotc8035ce2/controlb998。fmt/编译/net66+1ignored/hop9/wires10/journal29/dial_budget17绿（30.57s为子集运行时间）。两个真实update_dial均收到UDP后由本方法返回（200ms早by、30秒scope下10秒cap），journal兼容/roundtrip/显示句子控制绿。此Mac17不含Linux F7/F8，绝不能覆盖143rd F8真实红或证明真实多连接路径关联已验。


F8共享确认候选已实施、待验证：Slot最多一份QuietProbe（count/by/owned JoinHandle），任务只持Weak Wires/Conn；normal exchange与RetireUnheard共用，取消不越过全路grace。新包/替换/drop清除并abort；捕获runtime支持在外部thread Drop；adopt发布新wire后才清down，Unanswered失败在slot锁内提交，避免旧确认晚写peer down。裸未持有wire的exchange保留原本地probe合同。

F8函数与143rd红源逐字相同，已存SHA；F7观察升级f7_owned_confirmation_v2，保留同真实黑洞，仅改为外部thread取消后等待wire owner自身确认（grace+500ms），立即不能关闭。新增真实replacement和Weak owner两portable用例。仍未取绿，下一Turing net-node先校格式/编译/21 Linux合同，再Mac19便携；不能把候选记成已修复。


147th turing-20261109T104331Z-90266679已终态，仅FORMAT红：base45465a6adc5b9d341c9e8d820642033d7c2196a5/tree434de4e4189c4ef02c9e557063d57fd1793cc25e/snapshot366fb9ec00860c6db3203455e1958b76522a6ddf/controlb998479。44.81秒，未编译/执行；完整report已采/身份/group/hash核。原format.patch覆盖三个Rust文件，原样应用；下一同准确源码分别Turing21 Linux与Mac19便携（含two resource tests），不能预记通过。


148th turing-20261109T104749Z-3c7d804a终态编译红：basef28d04ae420ab51465576022de783600a11c0a1d/tree473e41cec25ed75e70325a896056181069ea3b71/snapshotef83ab1177ef1f12c75efd64ef7ca832d787cdbf/controlb998479。fmt及net66+1ignored通过，Node E0425两处Arc未限定，未执行21合同；report/group/source/bytes/hash已采核。类型已仅改std::sync::Arc，下一新源继续。

149th Mac42604002430与148th准确source完全相同（ef83ab11/tree473e41ce/basef28d），controlb998，新的独立request fd528bf6aeca4ee1b496d1b73da68110及owned source/bundle branches；复用了immutable snapshot，未重新创建另一tree。仍在途；21 Linux/19 Mac未预报绿。
