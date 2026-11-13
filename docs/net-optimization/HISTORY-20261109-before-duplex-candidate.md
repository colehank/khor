# feat/net 当前状态与恢复入口

更新：2026-11-09 03:19 UTC · owner=root，串行，无子智能体。
完整范围：[PLAN](PLAN.md)；要求：[REQUIREMENTS](REQUIREMENTS.md)；历史准确源/结果：[RUNS](RUNS.md)。
此前STATE原样保存在[历史检查点](HISTORY-20261109-before-api-transport.md)，其中pending/未改leave均为当时记录，不代表当前状态。

## 授权与位置

活动目标“完成计划”授权N0–N5实施/验证，取代最初仅规划；速度/体验优先，个人可信设备网。
没有合main、发布、中继采购/部署授权。本机source-only：不运行Cargo/Rust/Node/Xcode构建、测试、格式器或下载可执行物；小型Python/Git/脚本检查可用。
重型沿macserver/GitHub；不碰其他任务树、现装App或用户会话。图谱先用，缺覆盖再定向读；不建CodeGraph索引。

| 用途 | 位置 / 分支 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；恢复核实际HEAD/diff，本轮API入口已验证；37仍为72b1d891，N2锁等待候选随后提交，恢复核实际diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`；2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16已push；历史run绑定各自control |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；只允许paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture；生产基线原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json现105条不可变source refs；仅42559476637在途 |

## 执行通道现状

托管Linux prepare仍因GitHub账单/额度拒绝：42545673450两attempt及42545712302没有进入步骤，没有产品测试/source artifact；原始拒绝证据已保留。
已异步询问恢复额度或已有Linux通道，无回复，不改账单/采购。直接SSH macserver经原iroh-ssh路由banner超时，未改路由或信任。
Mac自托管runner可执行：worker-smoke42549824680 SUCCESS仅工具链/guard探针；42551075037已实际执行产品回归，见下文。
既有Mac入口跳过托管prepare/retire，沿development.yml、mac-mini-worker共享队列和已安装run-build；只接受net-unit/net-node/net-vendor/net-congestion。资源/空间/清理策略未改。

**42552723913两attempt均终态执行器传输失败，不是产品红。** 同clean72b1d891、treef6d0e84996723b8e9ad4659b6a2be8b625cb74b5、snapshotbafedc8a33390550c2bd0b10de0103e305500d31、controlb9b27116ad286fb51edc1e211949127681c5aa27。
attempt1源checkout、attempt2控制checkout连续github.com:443超时；未到源码校验/格式/编译/测试/host-capabilities；无source artifact。
两attempt日志、raw run/jobs与未执行分类分开保存在mac-fault-candidate-checkout-42552723913/及attempt-2/。不得再把该run记pending或438/36/9绿。

**API Git bundle已实际远端传输成功，产品单元已通过。** 控制dd57be6沿GitHub API取准确bundle、在GITHUB_SHA取可信控制文件；校验Git blob/SHA-256、快照与parentless/tree后，复用原prepare和build guard。Git只读本地bundle。
本机客户端额外上传私库dev-bundles/<request_id>，仅含无父快照的Git对象；64MiB上限，无开发历史/未跟踪文件。原source分支保留。
短期只读job token只在可信bootstrap步骤；Git/prepare和构建步骤不继承。默认standard不发送新inputs，保留旧main兼容；省略bundle双输入可走旧checkout。
本机remote_development12项、bootstrap3项（准确还原/错误checksum/错误snapshot）、prepare1项、network-driver1项通过；Ruby YAML及16个bash语法块、diff-check通过。没有本机Rust/Node构建。
**42555870179 net-vendor SUCCESS**：base d2849d4732c7e9b6dd2e953616ce583f0e520391 clean、tree4a9c4d89cc01e33a480375d50cc9f77b6d2d7a83、snapshotb6a4e9240786b1b178aee1e11284e6164aecc377、control dd57be6c57090a811726983e700d0b39a40fd1e8。
API bootstrap步骤实际success、两checkout均跳过，已进入现有共享guard；根格式、438Noq/36路径/9选家全绿；Noq2新恢复测试与37快速Go实际通过。bundle17935476字节，SHA-256 284904bec454a22f964fbb5c4f5e0307e3f76342815d95d641dd9f18c261cf0b，blob91937c9b7d1330f2b1d8352dbe6d309c278e38b5。
refs/khor/net-evidence/42555870179已create-only锚定，回执及raw run在mac-api-vendor-42555870179；恢复只查该ID，不重发。

## N1当前候选与真实红

37生产条件分支72b1d891c879cab63aaed032e201aee7baf1734c已由42555870179实际单元验证；Linux SLO仍缺：
当前tuple有故障后且最近原250ms tick接收copy提示时，保留36的决定后备用应答与当前坏copy复测；无提示时接受began之后已有备用应答，避免真黑洞多等tick。
接收提示不能证明当前正向健康；故障前旧样本仍无效、4秒兜底/正常两样本/速度优先/原节奏不变，不增加应用请求重放。

实际预期红 **42551075037**，旧7b61b7edc12ed5ef384b7054240e7b5202f1638b / snapshota21c1a978549979a8956639d35fdbd722cc6c8df / basea087 dirty / control6b1616c8b816bdb72a8a031d0ff28cc46a3fb4c7：
根格式/437Noq绿，Iroh35pass1fail仅无接收copy快速Go用例，实际Hold双PING、预期Go。home因失败未执行；证据mac-current-copy-hint-red-42551075037，Mac26.6.2 arm64/Rust1.95/jobs2。
37新增false旧包/4秒为实际绿覆盖，不宣称它们也已有旧源红。Noq2 reporting启用的新0-RTT测试实际通过；N3记录器仅根格式通过，尚未编译/执行。

36普通Linux结果仍有效且不能被Mac单元覆盖。同basea087 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644，日志关闭：

| Run | 结果 / 未解决事项 |
| --- | --- |
| 42542824071 netlab-stalls | SUCCESS2/2；p1s600秒/20次300ms忙CPU，alpha/beta0/0移动、零退役/打洞、最慢865.4ms、3019包312293字节；p5原150/300ms绿，600ms仅观察 |
| 42542984111 netlab-dynamic | FAILURE25/27；p3 1.1/1.2秒/818.4ms，前两轮超原1秒门；p2与21相位绿，零错误/退役 |
| 42543254686 netlab-hop | FAILURE8/9；f4s第一轮0.7秒回carried，10.9秒后又退Relay、28.8秒才再回，最终LinkRelay；另两轮0.4/0.6秒无回退，f4/f4i均绿 |

f4s中继2.5秒双向注入在断腿恢复1.5秒后清除，不能把后续整段观察当作5秒Relay RTT。回退红独立保留，不能声称37已解决。
42542228149诊断2/2仅定因证据，不是SLO。旧Watch p3较快但CPU3/1移动及hop1513ms红保留，不混不同源/机器算改善百分比。

## N2–N5与顺序

N0部分、**N1仍红**、N2–N4未完成、N5部分。不能以单元绿或执行通道建立完成整计划。

1. Mac vendor根格式/438/36/9绿。N2锁期限回归42556768499已实际红；最小CallerExpired/normal有界锁42557583517已实际绿，下一步early入口独立回归。
2. 恢复Linux能力后，原p3三轮<1秒、CPU0/0、hop不回退，再准确源完整矩阵；不能放宽门。host-capabilities实际：VZ支持1、16GiB/10CPU、可用约76.2GB，仅brew已装，Lima/QEMU/Docker/Podman/Colima均无；未启动VM/安装/改空间策略，Linux VM尚为提议。
3. 独立定因f4s回退，必要时保存默认关闭诊断三端trace，保持原断腿与1500ms门。
4. N2七类预算/取消/0-RTT/写协议已冻结ACCEPTANCE，真实源码设计见NEXT-DESIGN。锁/hop/首拨尚未贯通by、早数据回答取消所有权缺口仍未实施。按Op原30/300/3600档传deadline，不能把长操作压成28秒；hop3秒为两个中间设备合计，取消不计peer故障，真实RetireUnheard仍要保留。
5. N3记录v2候选f17b8a31：全部echo/typing/screen结果、成功/全部结束时间分布分开、bulk真实字节/时长/结束原因；默认几何/seed/工作量不变。net-congestion三轮顺序、独立log、slo=false；尚未远端运行。实际Node32席位/content预算/Custom外层队列、SSH/hop公平与能耗仍缺。
6. N4默认北京HTTP/TLS同机、自家登记集中北京；香港存在中继但无默认独立pkarr。已有多登记/选家机制需沿用，不能重复造。两端全路径/不对称/冷暖发现/故障域/容量实测与最优拓扑设计仍缺；不采购/部署。
7. N5补丁48项（Iroh37+Noq11）初步清单见VENDOR-AUDIT。Noq6覆盖42543928044根格式437/35/9绿、Noq4 ACK隔离42539256792绿、Noq2 reporting恢复与37条件分支42555870179根格式438/36/9绿；Noq1/5专项、其余边界与上游行为等价仍缺。固定1.4.0调用链/hash证据见UPSTREAM，不升级/删补丁。原生代理/Mac/手机/多机/能耗与最终可信Quality仍缺。

## 证据与接续

恢复先核绝对worktree/branch/HEAD/diff、当前run终态和回执，不从摘要覆盖未提交内容，不重复调度活动run。
每run锚refs/khor/net-evidence/<run>；git rev-parse REF:source必须等于回执tree；只用git show读容器，不checkout覆盖、不push本地证据refs。
source-refs.json现105条；425527传输失败与托管账单拒绝均tests_executed=false、remote_source_verified=false。旧Mac实际红已验证source/control。重要artifact下载到规范证据目录。
Mac源/bundle分支仅在终态、证据保留、所有权和SHA匹配后清理；复用源的请求不拥有原源分支。重试不改旧源/control/attempt证据。
外盘缺失只读git show feat/net:docs/net-optimization/STATE.md和PLAN；原外盘未提交内容不能按摘要覆盖。治理另沿中央code-quality STATE/RESUME，不与本任务混合。

## N2锁期限实际红与当前候选

42556474188第一轮仅新assert根格式失败，未编译/测试；远端format.patch原样应用，本机未运行格式器。
**42556768499实际预期红**：base2018ff956744f5dc33f5aa8b16a5caa93a36dad8 clean、treec60e145a72b88dd311fd90c9ff6a193983644210、snapshot654d409551e512462901a237e8bd47c7bc2c42da、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。
根格式、net66pass/1ignored、hop9、wires10、journal28全绿；dial_budget旧用例pass、新用例fail：真实line_to_by持同peer/proto锁，40ms调用期限不返回、外层500ms防挂触发。scratch hosts=false Node、Minimal无relay/discovery Endpoint，绑定在计时前，失败后关ep清理；不碰个人会话。
mac-dial-budget-formatted-42556768499已核source/control、保存原log及classification；99 refs均终态。

当前最小生产候选：line_to_inner到期入门拒绝；有by时timeout_at等待锁，到期/锁同时ready再核时钟；无by仍原锁等待。CallerExpired独立于peer TimedOut，不写共享失败结果/peer down/backoff；各错误消费点有显式处理。
原测试加强验证CallerExpired及释放锁后已过期调用也不拨号（后者为额外绿覆盖，不宣称已有旧源红）。已由42557583517远端格式/编译/单元通过。
**此批只处理normal line锁等待。** ask_early锁、hop准备、拨号自身裁剪与CallerExpired分类、operation总期限和普通交换/一次读重试、Accepted回答取消、长操作30/300/3600档仍须后续实施/实测，N2不能收口。

**42557583517 normal锁期限Mac net-node SUCCESS**：base19f0e1900e9344461c2b7cb2cb85f9e4a7921361 clean、tree2fd3cf7ea6ba50276be1bb7e9243c784c50830d8、snapshotd76915fc0c91ffde65ceb63d5fde560fb5506162、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。mac-dial-lock-candidate-42557583517及100th ref已保存；根格式、net66pass/1ignored、hop9/wires10/journal28/dial_budget2全绿；普通锁边界由42556768499实际红转绿。

新增独立early入口测试an_early_call_deadline_bounds_waiting_for_another_dial：同真实Node/Endpoint/持锁fixture、Request::Sessions、40ms by与500ms防挂；early生产锁仍原样，待实际取红。fixture只整理测试设置，正常已绿行为仍需随新源复验。首次证据下载API EOF后同run只读重取成功，无重跑。

**42557961726 early锁期限Mac net-node实际预期红**：base91d8777ce7bbe6ad55234482361f399f3f2372d7 clean、tree11262ccc042a1bfcd4628a5b00f669d7cafc151a、snapshot4eb0b8da9b37a2d3554c6179f479e347fd3ac5e7、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16；当时early生产锁原样，根格式/net66+1ignored/hop9/wires10/journal28绿、dial_budget2pass1fail仅early持锁40ms不返回至500ms防挂；normal控制绿。mac-early-admission-red-42557961726及101th ref已保存；恢复查该ID。

early准入现按实际红最小写候选：by到期入门与取锁后都拒绝、等待锁timeout_at受同by约束，None原样。到期返回CallerExpired且不写owner共享结果/peer失败；额外过期空锁不拨号覆盖待绿。只有两个准入边界，未补hop/拨号截断分类/普通交换/Accepted回答取消/总期限和长操作档，N2仍不完整。101 refs终态，下一准确源net-node待绿。
N3已发现旧共享FIFO延迟派发器可能跨方向阻塞，模型校准协议先冻结ACCEPTANCE；此时未改N3派发器/写校准测试或实际测量，不能据toy模型选产品CC。

**42558490443 early锁期限候选Mac net-node终态格式门失败**：base802e13a86ba738d54a8b096445b95435ab6b7712 clean、treef2afc1c9a48599f6091675105bc66075c73fe830、snapshotbfb637abe9d3d8a56683a5d7c6b5fcefa0999273、control2e70c39fe6a70bbbd09b7434f90e6c2d62e84b16。mac-early-admission-candidate-42558490443保存准确source及format.patch；仅新增过期空锁assert内await换行，未编译/测试，现已原样应用。生产early候选待新源绿，不记行为红。

N3因果校准用例已按ACCEPTANCE冻结协议写入congestion.rs，尚未编译/执行；模型本身仍旧单FIFO。control e7258e57155da1618f0f3b00fd1e395f481b5f19已push：net-congestion先运行唯一校准用例并要求1pass/0fail，再执行三轮ignored测量；失败时不产后续计时log。本机driver桩检查1项通过（校准红即止、计时首轮红仍三轮全留），无本机Cargo/Rust。下一准确源先Mac net-node检查early候选与新增源码格式，再N3校准预期红；不可在校准前据模拟结果选择CC。

**42558820303 early格式修正源Mac net-node SUCCESS**：base783619dd2bf1d034828f717bfb90f1f2b635a6a7 clean、treef3b250de87c64b452ccf2dac6543eb63d677938a、snapshot27bfd24c3233e98ab8998f8056bbba3f8650e5c4、controle7258e57155da1618f0f3b00fd1e395f481b5f19。含early修复/格式patch及N3模型校准测试（本profile仅格式/Node，不执行N3测试）。mac-early-admission-formatted-42558820303已核source/control并保存；根格式、net66pass/1ignored、hop9/wires10/journal28/dial_budget3全绿。normal/early持锁期限两项分别实际红转绿，N3仅格式通过、未编译/校准。

下一步以e7258e5控制profile执行net-congestion：先实际UDP双向道路校准预期红，模型仍旧shared_fifo_v1；取红后修独立方向派发并显式版本，再校准绿/三轮完整v2采样。不要把尚未运行的模型/测量记为N3绿。N2除锁准入之外的冻结边界及N1 Linux原红、N4/N5均继续。

**42559155803 N3模型校准Mac net-congestion测试fixture编译失败**：base599c57d0be6abee38488b7814daa9ca598370402 clean、treeb84a3bc4ea9a028fa8e2c147264623dfcb5fa971、snapshot54082af8662510e0548c5d4b584d06531724c8c0、controle7258e57155da1618f0f3b00fd1e395f481b5f19；模型仍shared_fifo_v1，校准先实际取红，不预记编译或测量绿。mac-road-model-red-42559155803及104th ref已锚，恢复只查该ID。

42559155803准确source/control已核、根格式通过，校准fixture的send_to错误传u8而非字节切片，未执行测试；现已修b"e"，模型仍未改，不能记模型行为红或N3记录器已编译完成。104 refs终态，下一准确源重取原模型校准红。

**42559476637 N3校准类型修正源Mac net-congestion在途**：base6880d251edd38218007da99c9d070df2dd1957a5 clean、tree751e95115a7fb3a17faca1a5a44129e00a27655c、snapshote851d256586088d589971c2c1cb60f1461233a0c、controle7258e57155da1618f0f3b00fd1e395f481b5f19。模型仍旧shared_fifo_v1，待实际行为红；mac-road-model-typed-42559476637和105th ref已锚，恢复查该ID。baseline实际仍仅原六fixture，生产未改。
