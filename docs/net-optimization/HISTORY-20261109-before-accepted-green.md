# feat/net 当前状态与恢复入口

更新：2026-11-09 05:12 UTC；root串行，无子智能体。活动目标“完成计划”仍active。
完整范围：[PLAN](PLAN.md)；约束：[REQUIREMENTS](REQUIREMENTS.md)；准确源/历史结果：[RUNS](RUNS.md)。
此前STATE原样保存在[本轮历史](HISTORY-20261109-before-duplex-candidate.md)，其中pending/未执行均为当时记录；当前以本页为准。

## 授权与工作区

速度/体验优先，个人可信设备网；活动目标授权N0–N5实施验证，取代最初仅写计划。没有合main、发布、中继采购/部署授权。
本机source-only：不运行Cargo/Rust/Node/Xcode编译、测试、formatter或下载可执行物；小型Git/Python/脚本检查可用。重型沿macserver/GitHub，不碰现装App、生产store、个人会话或其他任务树。
图谱先用，覆盖不足才定向读；不新建CodeGraph索引。外盘布局和治理入口遵守仓库AGENTS，本网络任务不接管治理。

| 用途 | 位置 / 分支 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；恢复核实际HEAD/diff，N3模型实现1478bf97，N2生产802e13a8+远端格式修正；随后HEAD为证据文档提交 |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`；0cff9671fb2f80945423132e0df46995f6ff3198已push；历史run各绑定原control |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture；生产原样，现场已复核 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json现110条不可变refs；全部终态 |

## 当前工作：N3校准与三轮完成，下一步贯通N2总预算

**42559476637实际模型红**：base6880d251edd38218007da99c9d070df2dd1957a5 clean、tree751e95115a7fb3a17faca1a5a44129e00a27655c、snapshote851d256586088d589971c2c1cb60f1461233a0c、controle7258e57155da1618f0f3b00fd1e395f481b5f19。
根格式与congestion目标编译通过，真实UDP校准0pass/1fail：双向各1000B/s、RTT20ms、0损失、队列1秒，正向两个500B，第一包到后反向1B被队头未来包挡住，超过250ms容差。两ignored过滤，零计时轮；该红源的v2记录器完成编译但未进入计时。证据mac-road-model-typed-42559476637已核source/control并下载。
前42559155803仅新fixture误用u8编译失败，修b"e"后才实际取红，不能混算。

ACCEPTANCE在修改模型前冻结校准协议。本轮修复仅改测试道路：每方向一个FIFO派发器，保留每lane序列化/损失/队列/seed，移除跨方向未来due阻塞；标识`independent_fifo_v2`。记录schema仍v2，case/summary/sample均标road_model。
**当前独立方向模型已实际校准绿**（旧源42559476637红→本源绿）。产品CC/优先级没有调整。control同步该model字段；先要求唯一校准1pass/0fail，再跑三轮ignored完整采样。driver本机桩检查通过：校准红即止，首计时轮红仍保留三轮及最终exit8；不是Rust实测。
**42560015454 Mac net-congestion SUCCESS**：base1478bf9712ad50c00b502c78613cd932f20586b3 clean、tree33d5ef6d47cde04620db1baa470bd7fefdf0c99d、snapshot9a6f9717470e0a741aedbdd9dd1ecf0a32d18fc8、control0cff9671fb2f80945423132e0df46995f6ff3198。
mac-duplex-congestion-42560015454及106th ref已锚，API请求e7e4871ed8c5478ebcc9022972cb3a0c；根格式/编译/校准1pass、三轮各2pass完成（167.64/167.09/167.29秒）；704原始请求/51摘要/15bulk case逐项核计数、索引和分位数，零失败/超时，sample-audit.json已保存。此任务已终态，不重跑。旧shared_fifo_v1结果不能与新模型差值算产品改进。

## 已取得的本轮真实回归证据

| 边界 | 实际红 | 实际绿 / 范围 |
| --- | --- | --- |
| N1 Iroh37：无当前copy近期接收提示时允许故障后备用ACK快速Go | 42551075037：根格式437Noq绿，路径35pass1fail仅新快速Go用例 | 42555870179：根格式438Noq/36路径/9选家全绿；同时Noq2 reporting始终启用恢复测试通过；Mac26.6.2 arm64、Rust1.95/jobs2 |
| N2 normal拨号锁调用期限 | 42556768499：根格式/net66+1ignored/hop9/wires10/journal28绿，dial_budget1pass1fail；40ms调用被锁挡至500ms防挂 | 42557583517：同既有子集及dial_budget2全绿；CallerExpired、owner结果不变、零peer down/backoff/失败，过期空锁不拨号为新增绿覆盖 |
| N2 early-data拨号锁调用期限 | 42557961726：normal控制绿，仅early持锁40ms不返回至500ms防挂；dial_budget2pass1fail | 42558820303：根格式/net66+1ignored/hop9/wires10/journal28/dial_budget3全绿；含两入口与过期空锁；treef3b250de87c64b452ccf2dac6543eb63d677938a、snapshot27bfd24c3233e98ab8998f8056bbba3f8650e5c4、controle7258e5 |

N2新增CallerExpired与peer TimedOut区分；by入门过期、取锁等待和锁就绪后都检查，None保持原来等待。到期不写共享失败缓存/peer失败/退避。fixture为scratch hosts=false Node及无relay/discovery的Minimal Endpoint，绑定在计时前，不碰个人网络。
42556474188和42558490443分别只有新测试assert的格式门失败；均原样应用远端format.patch后复验，不是行为红。本机未运行formatter。

37生产最初72b1d891：当前tuple最近原250ms tick内且晚于故障began的live copy收包仅作为复测提示，不证明正向健康；有提示保留36的决定后备用应答/当前坏copy探测，无提示接受began后的已有备用应答。旧包/4秒兜底/正常两样本/速度优先/原节奏保留，无应用请求重放。
以上都是Mac单元，不能收Linux SLO或完成整计划。

## Linux原红与已有稳定性证据

下列同basea087 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644，普通日志关闭：

| Run | 结果 |
| --- | --- |
| 42542824071 netlab-stalls | SUCCESS2/2；p1s600秒/20次300ms忙CPU，alpha/beta0/0移动、零退役/打洞，最慢865.4ms、3019包312293字节；p5原150/300ms门绿、600ms仅观察 |
| 42542984111 netlab-dynamic | FAILURE25/27；p3三轮1.1/1.2秒/818.4ms，前两轮超原1秒门；p2及21相位绿，零错/退役 |
| 42543254686 netlab-hop | FAILURE8/9；f4s首轮0.7秒回carried、10.9秒后退Relay、28.8秒才再回，最终LinkRelay；另两轮0.4/0.6秒无回退，f4/f4i均绿 |

f4s中继2.5秒双向注入在断腿恢复1.5秒后清除，后续整段不能当5秒Relay RTT。回退独立定因，不宣称37已修复。
42542228149诊断2/2不是SLO；旧Watch较快但CPU3/1与hop1513ms红仍留RUNS，不混源/机器算百分比。恢复Linux后原p3三轮<1秒、CPU0/0、hop无回退，再准确源完整矩阵，门不放宽。

## 执行通道与限制

托管Linux prepare因GitHub账单/额度拒绝：42545673450两attempt及42545712302无步骤/无产品测试/source artifact；原始拒绝已保存。已异步询问额度或已有Linux通道，无回复，不改账单/采购。
直接SSH macserver经原iroh-ssh路由banner超时，未改路由/信任。Mac自托管worker-smoke42549824680仅工具链探针；实际产品测试现已恢复。
42552723913两attempt在Mac→github.com:443的source/control checkout超时，无编译/测试/源artifact；执行器传输失败不是产品红。

当前API Git bundle通道已多次实用：本机上传私库dev-bundles/<request_id>，仅无父快照对象、64MiB上限；worker经API读bundle和GITHUB_SHA的可信控制文件，校Git blob/SHA256/snapshot/parentless/tree，复用原prepare和已安装build guard。Git只读本地bundle；临时repo-read token不继承到Git/prepare/构建。
沿development.yml、mac-mini-worker单槽，仅net-unit/net-node/net-vendor/net-congestion；默认standard不发新input、旧checkout保留。资源/空间/清理策略不改，无个人key/token/签名/模型凭据/生产store。产品12项、bootstrap3项、prepare1项、driver桩检查及YAML/16shell语法先前通过，后续driver改动亦已重验。

host-capabilities42555870179实际：VZ=1、16GiB/10CPU、约76.2GB空闲，仅brew已装，无Lima/QEMU/容器工具。Linux VM具体候选已写NEXT-DESIGN：项目专属Lima2.3.0/Ubuntu24.04固定checksum、2CPU/6GiB/30GiB、无home/key/agent挂载、前台hostagent归guard生命周期。**没有安装/启动VM**，不能说Linux已恢复；新后端需同配置基线重新测。

## N0–N5未完成范围与接续顺序

N0部分、N1仍红、N2部分、N3部分、N4未完成、N5部分。不能用本轮单元或测量器修复结束目标。

1. N3独立方向模型与三轮已绿，现需用Noq基线定位真实应用排队；后续失败/超时仍须保留，成功与全尝试结束分布分开、空分布null，不用少量screen样本报p99；它只是Noq模拟层。
2. N2接续另核IPC正常半关闭不等于取消；RetireUnheard已有完全无收包+Patience.ack门，具体hash/摘录见deadline-continuation-20261109与NEXT-DESIGN，未新增运行。N2除已绿normal/early锁准入外：hop准备、拨号自身裁剪/CallerExpired分类、普通交换/一次读重试总期限、Accepted后Aborts回答取消、长操作档位和并发/输者释放仍缺。按Op原30/300/3600真实deadline贯通，不能把长CodeServe/transfer压成28秒。hop3秒为两个middle合计；取消不计peer故障，真实RetireUnheard仍保留。七类合同在ACCEPTANCE，具体调用链NEXT-DESIGN。
3. N1先恢复Linux能力，独立定因f4s再原门复验与最终完整矩阵。Linux新VM尚是设计，不跳过能力预检/基线。
4. N3真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop排队/公平、共享vs分连接、1/3/5设备后台与手机能耗仍缺。不能把增加总资源当调度胜出或Noq模拟当Node SLO。
5. N4默认北京HTTP/TLS同机、自家登记集中北京，香港默认无独立pkarr；已有多登记/选家/继任机制须复用，尊重only_mine和用户n0配置。两端全路/不对称/冷暖发现/故障域/容量实测与拓扑设计仍缺，不采购/部署。
6. N5共48初步补丁（Iroh37+Noq11）：Noq6计时覆盖42543928044、Noq4备用ACK隔离42539256792和本轮Noq2/37绿保留。Noq1/5专项、其余边界、固定1.4.0逐行为等价仍缺；UPSTREAM静态hash证据不当上游运行，不升级/删补丁。原生Mac/代理/真实手机/多机/能耗及最终可信Quality仍缺。

## 证据与恢复

先核worktree/branch/HEAD/diff、当前run与准确回执，不按摘要覆盖未提交内容，不重发活动run。source-refs.json现110项，均终态；42562932053已实际预期红；42562380977为格式门失败、未编译/测试；前106项终态且ref/tree已核。
git rev-parse refs/khor/net-evidence/<run>:source必须等于回执tree；只git show读容器，不checkout覆盖或push本地证据refs。账单/checkout失败均tests_executed=false，校验成功但format/compile失败与真实行为红分开。
Mac源/bundle分支只在终态、证据已留、所有权/SHA匹配后清理；复用源的请求不拥有原源分支。原attempt记录不覆盖。
外盘缺失只读git show feat/net:docs/net-optimization/STATE.md和PLAN；原外盘未提交内容不能按摘要覆盖。其他治理继续中央STATE/RESUME，不混授权。

## 本轮收尾与下次具体入口

42560015454的v2 Noq样本仅此固定300ms/2%/20Mbit/200ms、三轮同seed，非产品SLO。合并bulk-echo p95：Cubic普通890.3ms/优先389.9ms，BBR3普通449.6ms/优先612.4ms；不能据此宣称优先级总能改善或修改产品CC。吞吐范围及样本量见REPORT；每CC screen仅9样本，不报p99/置信尾部。
此前applicability核对时link.rs与42558820303相同；现在N2候选已改变link.rs，该Node证据只适用历史版本。两个vendor目录仍与42555870179相同，applicability.json保存适用范围；没有把这些局部绿当最终完整Quality。
下一步先按NEXT-DESIGN/ACCEPTANCE推进N2真实Op截止点贯通：保持IPC正常半关闭和长档位，覆盖hop准备/拨号截断分类、普通交换/一次读重试、Accepted回答取消与其他并发请求独立。并行的逻辑设计可继续N3真实Node/SSH/hop排队；Linux/VZ仍未实施、N1原红须恢复后复验、N4/N5仍未完成。goal保持active，不标complete或因额度问题整体blocked。

## 新接续：N2操作总预算回归待执行

新增deadline任务局部载体仅传不可变截止点，尚未被Op入口或网络消费者使用，生产28秒行为原样。新增两个真实ask_on/持锁回归（40ms必须结束、300秒不能被28秒截断），预期红尚未执行；暂停时钟仅dev test-util，绑定在计时前，本机只检查TOML/diff。下一准确源Mac net-node先取红，再贯通。前106 refs全终态，不复跑旧已绿Noq测量。

**42562380977 Mac net-node操作期限载体回归在途**：base4b7773007e2d3a4fd61960ce6f82b305167ac111 clean、treed3cc715bcac2173cdaa5905ad173ce295f931906、snapshot2daaec31a984488b7cfbf6f5ef8549448d1546c7、control0cff9671fb2f80945423132e0df46995f6ff3198。载体未被生产消费，真实ask_on两个scope测试预期红未实际取得；mac-operation-budget-red-42562380977/107th ref已锚，恢复只查此ID。

42562380977只因新测试调用的布局被根格式门拦下，无编译/行为红；远端patch已原样应用。本机未跑formatter，107 refs终态；生产仍未消费新scope，下一准确源继续取红。

**42562932053 Mac net-node操作期限回归格式修正源在途**：base28d9fe6ebb02475a6054d87bc0ab089f21290a02 clean、tree6c3de5d38a2d302813f6c66401436d3ddf6454ad、snapshot79e97ce75b77d17889ac0a2bcb008fbff47df146、control0cff9671fb2f80945423132e0df46995f6ff3198；载体仍未被生产消费。mac-operation-budget-formatted-42562932053/108th ref已锚，恢复查询该ID，不重发。

## N2总预算实际红与贯通候选

42562932053已实际红：根格式/net66pass+1ignored/hop9/wires10/journal28绿，dial_budget3pass2fail，仅真实ask_on忽略40ms scope、并以固定28秒截断300秒scope；测试均清理任务/endpoint并检查ready返回为正常错误，无peer失败。准确tree6c3de5d3/snapshot79e97ce7/control0cff967已核，mac-operation-budget-formatted-42562932053保存raw/log/source/classification；108 refs终态。

当前候选在run_handoff_within一次建立Op原档-.5s的Tokio绝对截止点，任务局部只承载不可变值，普通/early锁、hop准备、有限拨号候选/1500ms延迟、普通交换与一次读重试均向下沿用。显式短by与操作by取早；短操作到期不记peer失败/退避，拨号自己的上限仍保留；Miss::CallerExpired独立，RetireUnheard的原独立活性证据继续生效。
独立ask无Op上下文时，由Request工作预算加既有拨号25秒、hop3秒与最大路程120秒合成外层上限；各阶段仍受自身上限裁剪。这次复核纠正先前套IPC档位会挤掉工作加路程预算的问题；实际前台仍用原Op截止点。新阶段不重建时长，原请求预算/读写/0RTT白名单不改。IPC正常半关闭不动。**候选尚未远端验证**，跨前台spawn的显式继承审计、后台多阶段入口、0RTT Accepted后回答所有权/取消的真实回归与其余七类合同仍需完成，不能以本次两锁测试覆盖全链路。
Accepted后原裸JoinHandle await本批尚未修正；握手到期若刚得到未adopt的conn则中止该回答并关闭该新conn，避免新代码引入额外孤儿。下一步先候选net-node格式/编译/五测试，再单独构造真实0RTT恢复取消/健康共享wire对照。

**42564150236 Mac net-node总期限贯通候选在途**：base1950943bdfff6f2944f5dd9a5124328ae7fa50dc clean、tree9f57f8181925f12763dd597fd336ead82741abd9、snapshot787c72e71038003da290e0b3009c47244eddc9cf、control0cff9671fb2f80945423132e0df46995f6ff3198。mac-operation-budget-candidate-42564150236/109th ref已锚，未预记五测试绿；Accepted回答所有权尚未改。

42564150236仅根格式门失败（函数签名、array和链式await布局），source/control已核、无编译/测试；远端patch已原样应用。109 refs终态，贯通候选仍待新源验证，不记行为红或五测试绿。

**42564749932 Mac net-node贯通格式修正源在途**：basea4ca8637cad3fa13554a54c7ef95e7e481041b75 clean、tree1f1677176b810668ef61103fafaffd9e52d1eb66、snapshotc39f8cc6f3793d2ef828f9d7b29ff83bdc479d91、control0cff9671fb2f80945423132e0df46995f6ff3198；mac-operation-budget-propagation-42564749932/110th ref已锚，恢复查询此ID。

42564749932终态：根格式/net66pass+1ignored绿，Node因dial_update旧std时钟参数编译失败，Node测试未运行，不算行为红。准确source/control/ref已核，raw与classification保留。现将更新拨号自身10秒上限与调用期限分开；无Op的ask外层上限由现有阶段预算合成，避免IPC档位错误压缩工作。此修正候选待新的Mac net-node。


**42565998212 Mac net-node更新拨号上限修正候选在途**：baseeb4f8c1b70d118394b596ff21bd31a280ad4a412 clean、treeab8e723fbb02aecb43f7c86b0577563f6c32052b、snapshot8964af950889ceec388af1af9eabf900db9206eb、control0cff9671fb2f80945423132e0df46995f6ff3198；111th ref和mac-operation-budget-limits-42565998212已锚。更新保留原10秒自身上限；独立ask外层由既有阶段预算合成。待格式/编译/五预算回归，不预记绿。


42565998212终态仅**格式门失败**：提取RTT_BUDGET_MOST后budget_for链式表达缩为一行，远端patch原样应用；未编译/测试，不记行为红。source/control/ref已核，111 refs终态。新增真实0RTT Accepted→调用取消回归正在编写，生产回答所有权尚未改。

已新增Accepted回答真实取消回归及fixture_for真实peer身份，生产Accepted裸JoinHandle未修。协议执行前写入ACCEPTANCE；前置要求真实0RTT与adopt后仍等待，随后取消500ms内只停止回答流、同一健康wire仍服务独立Sessions、严格三请求且无peer失败。下一Mac net-node先验证五预算回归并为该所有权问题取红；尚无该用例远端结果。


**42566472411 Mac net-node Accepted回答取消回归在途**：base371d92b018bfd0732edda959504a0af887e6db99 clean、tree3efb5c3ef62d366356d9db978a55464b5ce4c732、snapshot9c417d4632b2040881d8898205ec5c506dd79a08、control0cff9671fb2f80945423132e0df46995f6ff3198；112th ref/mac-accepted-answer-red-42566472411已锚。生产裸回答所有权未改；五预算回归也待实际验证，不能预认新用例行为红。


42566472411终态仅**格式门失败**，新fixture的matches/链式调用/async块排版，远端patch原样应用；准确source/control/ref已核，112 refs终态，未编译或执行六测试。所有权问题尚未取得行为红，生产仍原样。


**42566727617 Mac net-node Accepted回归格式修正源在途**：basef1ef9049559cbb26b93eee361f293929842e53c3 clean、treec21e025c0cbd66cdb4aca4a5ab523d3be1be7434、snapshot9ca1c0fab8980b93903aee579ee14bf01f6b7f33、control0cff9671fb2f80945423132e0df46995f6ff3198；113th ref/mac-accepted-answer-formatted-42566727617已锚。六测试仍待真实编译/执行，生产回答所有权不变。


42566727617终态**fixture前置失败**：根格式/编译/net66+1ignored/hop9/wires10/journal28绿；dial_budget5pass1fail。两个scope40ms/300s实际红由42562932053转绿，原三预算回归均绿。取消测试因server recv.is_0rtt=false停在前置，尚未取消，不算所有权行为红。准确treec21e025c/snapshot9ca1c0fa/control0cff967已核，113 refs终态。下一fixture将server第二次accept后首TLS响应延迟100ms，仍必须is_0rtt真实前置；不删除该门、不修改生产准入。另复用fixture新增Accepted后1秒绝对by必须结束的第二回归，预期红尚未执行。


**42567204795 Mac net-node延迟服务端握手fixture在途**：based3e9a6a8772211cc603c67be4c7d55f0babf4487 clean、tree68329fdd70be7901a43a3194c79b154002d7a29a、snapshot225a9db2aa9df1ca769618ea0d4d374e9b0028f8、control0cff9671fb2f80945423132e0df46995f6ff3198；114th ref/mac-accepted-answer-delayed-42567204795已锚。取消与1秒by两Accepted回归生产未修，待格式/编译及真实前置判定。


42567204795终态仅**格式门失败**：deadline变体matches缩为一行，远端patch原样应用；准确source/control/ref已核，114 refs终态、未编译或执行七测试。随后官方Noq1.3 poll_accept源码明确is_0rtt在应用接纳当时取is_handshaking，不能当此前包加密级别证明；notes在结果取得前已保存。修正fixture以真实第二请求+Node adopt+调用继续pending20ms证明Accepted分支；撤回is_0rtt硬前置和100ms握手延迟，标志只观察。所有权生产、STOP/独立请求/计数/peer条件不改。下一准确源先取真实所有权红。


**42567527752 Mac net-node Accepted所有权双回归在途**：baseb097a2dec5a61e26032b8f4336d1caf4fe56e17a clean、tree54b47ae52986e9d1a1663a25805c212e30d94aee、snapshot6c7e63589e1a968650dcc3021dc337a471718a22、control0cff9671fb2f80945423132e0df46995f6ff3198；115th ref/mac-accepted-owner-red-42567527752已锚。使用实际adopt+pending的Accepted前置；生产裸回答所有权未改，取消和1秒by预期红未取得。


42567527752终态**Accepted回答所有权双实际红**：根格式/编译/net66+1ignored/hop9/wires10/journal28绿；dial_budget5pass2fail，只1秒by被回答忽略、调用取消后回答流未STOP。两fixture均经过第二请求/adopt/pending20ms实际Accepted前置，且最后独立Sessions成功、stable_id相同、请求数3/零dial_failed均已过，随后才在目标断言失败；不是前置失败或靠关闭conn的伪红。准确tree54b47ae5/snapshot6c7e6358/control0cff967已核，115 refs终态。下一最小修复把Aborts随Accepted回答交接，等待显式沿原by；不改白名单/请求frame/共享wire。

当前最小候选已改：Accepted交出的是Aborts所有者而非裸JoinHandle；adopt后借用owner内的handle，用deadline::wait(by, …)等待。调用取消/到期会drop owner并abort回答任务，只停止回答流；已adopt健康wire仍留Node。到期为CallerExpired，不转NotTaken重试或记peer失败；Rejected与未adopt连接的清理仍保留。该候选未验证，下一准确源Mac net-node需七测试全绿。


**42567946141 Mac net-node Accepted回答所有权修复候选在途**：base2fc650b30a3ab16792adb7babe566b8cec1bfeb5 clean、tree314894b33ed72a5ff2aeed47504d6c9e7f575bd9、snapshot78c6fb8eabf86ff755800188d9f92e64f8d4e3cf、control0cff9671fb2f80945423132e0df46995f6ff3198；116th ref/mac-accepted-owner-candidate-42567946141已锚。等待七预算/取消回归与既有子集，不预记绿。
