# feat/net 当前状态与恢复入口

更新于 2026-11-11 05:50 UTC。此前逐字状态在 [最近历史](HISTORY-20261111-through-original-dynamic.md)，SHA 9d789f1ac24504e3d56c813a9858272219b62e95326ffa1839a183717088113b。历史内继续链接更早状态和失败记录。当前句柄、收据和实际工作树优先。

## 当前目标与位置

完整 N0–N5 持续实施，活动目标为“完成计划”。用户 U14 明确要求持续推进。上次误将目标调度面板状态作为停止条件，本轮已纠正并推进原矩阵。root 串行，不派发子智能体。速度、体验和个人可信设备网为范围，peer relay、IPv4/IPv6 变址为必验项。

产品位于 `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch `feat/net`。最新生产候选为 `4b02c53ff928ed02ab50a1bb3cfa1b9f6f16d45a`，tree `d2c4cabb4bbe12bf614ae232e47a4da33fb54063`，snapshot `4d46d3009ffab2429f06f1df28258fdfa9eb9315`。17346为其格式修正前行为相同候选。此前NoRelay模式顺序修复为ff743。其后文档提交须另核 HEAD/diff。

控制位于 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，固定 `68cadb57a0ed72d40a4ca9a843e86b9c6797bf34`，原两个 untracked pycache 保留。原 N1 隔离位于 `/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines`，detached `2ef6bee5a5aaf841af066f4edb5c894769d4539d`，Iroh42–44仍隔离。

证据根为 `/Volumes/disk0/Khor/evidence/net`，ops 在其 `ops/`。`source-refs.json` 由 root 单写。完整范围与授权见 PLAN、REQUIREMENTS、ACCEPTANCE、NEXT-DESIGN、REPORT、VENDOR-AUDIT。main 合并、发布、采购、生产部署及已安装 App/config/store/resident 的修改仍需对应授权。

## 当前运行中的重型任务

**Turing `turing-20261111T052125Z-088e00f9` 正在执行 `netlab-stability`**。PID3381428，子进程组3381653。05:21:47 UTC 启动。05:41实核同一PID已运行19分41秒，execution仍running。原p1的600秒已完成、0切路0退役，p1j仍执行；这不是整个长稳通过。先查同一运行，避免重复提交。

此任务冻结源码为 Iroh47候选 `8cd58f8ea707991f23b10ac05d6c8388a61dd519`，tree `4c4d7be8de2c615c35de9b5ca6f3db0cce47a78c`，snapshot `4e900db337c5c4356cb4b27b0de2880210bcf67f`，control68cad。原 p1/p1j/p1s/p5 前缀、单线程、一轮，原持续时间和故障注入保留。计划 `confirmation-copy-identity-original-matrix-plan-v1.json` 已登记两项任务，动态已终态，此稳定性尚在跑。

Mac42964365563已结束并采集。最新候选Mac42968221420（net-node，source4b02/tree d2c4/snapshot4d46/control68cad）已实际提交，验证待核；先查同一run，不能重复提交。Mac42967869816已终态并采集，仅格式patch，未跑单元。旧行为Mac42966906265已采集并核身份，hop63/2红。独立 macserver 可穿插便携构建，依据 ACCEPTANCE 的开发加速规则，Turing 本机计时保持独占。此前过度保守的“所有Mac重型均等待”条件已在矩阵计划中纠正。

## 本轮完成的证据

### 原动态三轮

`turing-20261111T045728Z-3d36cba8` 已终态、采集、identity与空进程组验证。1313.385秒，15files。准确8cd/tree4c4d/snapshot4e900/control68cad。measurement-mode 为1线程、3轮、p2/p3/p7三个原前缀。每轮9项通过，合计27项及27条原 green case rows全核。

逐轮原时间 p2 慢路切换为990.5/937.1/946.7ms，p3最慢回显为532.3/481.3/546.6ms、各轮业务零失败。活跃和闲置p7原门均通过，原闲置门20秒仍需按后台成本评价。审计文件为该目录 `original-dynamic-verification-v1.json`。逐case表在task.log，三份scenarios.log保存各9/0原总结。首次分析错误地预期表在scenarios.log，已按实际保留位置核27条，并记录修正。没有产品重跑。

### NoRelay 缺第二腿提示

五个新回归位于 `crates/node/src/hop/no_relay_hint_tests.rs`，SHA `9629a261389cc6152b7938a6e6d02697121e031432f4c0c00a47accb769cb01b`。原sole_peer正文和Publication适配fixture保留。覆盖本端、对端、双端no_relay的缺提示情况，另有已知Direct正控和递归拒绝控制。真实hop_ahead、Tunnel、内层TLS、echo均执行，Publication中间节点直接适配目标transport。完整物理两腿在N4验证。

旧2cd67b9c/tree01cc/snapshot23930497在Mac42963573470实际hop57pass/3FAIL，原55和两控制过。exact artifact13830014031，zipSHA `2adeb885e80e68f15264ce1e9ab59e54c70b5f5ea1dffc9439de9dfee402b9cf`，10files/11control blobs/identity核，后续组未执行。

候选ff743只把目标缺wire的既有Recovery判断置于NoRelay/ToNoRelay标志前。Mac42964365563同一fixture已60/0，全12组通过，wires16/budget26/resume1。exact artifact13830134613，zipSHA `070ba81abece2159145779c84c8f3e57f0ee746808888fe8a18deecde1a16a6f`，20files/11controls核。Linux新源码回归待跑。源审计 `no-relay-missing-hint-source-audit-v1.json` 核生产只有模式guard顺序改变。

### Iroh47 与 Noq13

Iroh47使确认计数与质量、年龄来自同一代表连接。whole fixture `395341c56e7c3024505be5a838d8f1fa298e63066d33de1c237a0793b9a02b43`。旧41c2/b0146293实际Noq443过，path56/2红，代表自身0/1却借另副本报2。候选8cd/c350f32c已443/58/11过，原55及代表改变控制过。Linux f1537e21与Mac42915496687准确8cd源各全12组/hop55/wires16过，Mac exact13808888027，zipSHA `9c5f5122716eada434268b82cbad21f63ba453cbb82bea9500127a2fd532c0ca`。

Noq13使已建立、Data空间且支持扩展的显式path PING附IMMEDIATE_ACK。原ordinary PING/Initial/真实accepted0RTT控制过。whole e057 fixture旧efb2/8a4a7521为442/1红，候选2892/33bc9584为443/0、path55/home11过。2892还通过Linux ea5f7a11和Mac42909901676全12。Noq13的物理固定四次仍未满足稳定四秒，实际ACK成本须保留。

主vendor候选57项为Iroh1–41/45–47和Noq1–13。42–44仍隔离。逐项上游/退出与Noq1/5专项缺口继续按VENDOR-AUDIT处理。

## 保留的恢复失败

Iroh46五合同ae62旧c775/eec09ec1路径53/2红，候选1c847 vendor439/55/11及双OS全12通过。物理固定C-B,B-C,C-B六次均收齐，744业务成功。C1绿2红，其中9cb一方向4068ms，6aa双端无恢复Go；B2绿1红，3266反向5469ms。集合 `known-quality-physical-pairs-through-6.json`，各副本数差异和成本缺界已保留。

6aa没有postfault质量刷新，quality_steps0。X仅零星收到Custom包，Y无故障后新RTT，载体约21秒后结束。四端outer曾显示新IP约41–42ms，middle onward质量年龄3.526秒。真实流进度尚未重现此停顿，根因未闭环。原 outer-copy-timeline、failed-recovery-copy-observations 和 budget/cost原件保留。

流诊断v4 fd92在Linux3e49bad9全12过，但d111和0e2f两物理在原prepare150秒前置失败，未replug。原max3计划在两次同类前置失败后停止。v5 ef0b减少成功IO同步输出，保留累计字节和last_read/write_at，在pending/flush/end输出。单次883校准124/124、warm51/51、双端3435.866/3412.927ms通过，8方向字节持续推进，诊断因果未证明。

Noq13的固定四次C-B,B-C全部结束，共496业务成功。C f8为7307.560/3517.794ms且warm10/51失败，C e712为3703.047/3670.630ms双门过。B fe3为3386.396/3596.237ms过。B 8f3原availability前缀全部检查通过，但x末条IO记录在+31.279秒截断，原严格解析失败；独立Go5437.441/5401.818ms均晚，正式warm/cost未知。保留截断原件，无补跑。三次完整trace都见字节推进。集合 `path-probe-prompt-ack-physical-fixed-four-result.json`。

f8记录某probe副本回到90.353ms后，另副本100.025ms代表，原预算已用尽。质量代表选择与7.31秒的因果仍待独立验证。Iroh47修正确认归属，尚待peer移动实测。第二对reads_finished enclosing innerTX C353/B313 datagrams和IMMEDIATE_ACK C47/B16，选路阶段不同，不能全归因补丁。

更早923原固定三轮0aec/1cb/b158为2绿1红，b158最慢5514ms。Busy854、旧resume32 timeout、CPUquiet/f4s与原cold-start失败继续保留。旧元数据/编译失败、格式patch和全部源记录在最近历史及ledger。

## 下一步与全计划缺口

1. 先监控当前stability088e00f9，终态后collect、核identity/空进程组、原测量模式、每个原case及波动/探测/CPU结果。原矩阵源保持8cd。随后安排最新NoRelay源码的Linux回归。
2. N2真实候选变化合同已写并取得旧行为红；17346缓存候选经格式修正后在Mac42968221420验证中。先采集实际结果和格式/编译证据，保持原五fixture不变，随后同源Linux。候选事实仅IDs与实际首腿可达性；同ID地址变化、LOST namespace/address context仍未覆盖，不能由此宣告完整N2。
3. ASKING为计数而非互斥，不能据它假定open_hop串行。候选context须排除每ACK/RTT排序噪声、覆盖真实可达性与地址变化、保护late完成，保持有界和弱所有权。LOST还缺本机identity namespace及地址context。
4. 继续N1 peer-relay四秒/单向停顿、原quiet/stalls/成本矩阵，按证据决定下一策略，失败保留。N0最终SLO/样本量/p99仍欠。
5. N3保留交互与批量共享/分连接比较、背压公平、1/3/5设备、CPU/字节/唤醒/手机能耗。N4覆盖真实两腿Relay/唯一middle/断腿/多middle、独立IPv4/IPv6及同时变址、Wi-Fi↔蜂窝和自动恢复。N5完成全部vendor审计、最终Quality/native/proxy/多设备/phone。

MacLab与Turing同LAN。构建macserver为另一台，异网为用户报告。iPhone localhost旧现场需反复关开App后某次经Turing恢复，截图5GA下仅hinton Direct29ms。首腿可达性与第二腿实际可达性分别验。候选未安装，当前没有phone采样任务。iPhone按可信GitHub TestFlight边界接入最终验收。

## 执行与接续

本机source-only，重型构建、测试、formatter在远端。Git和Python标准库元数据/分析可本机执行。先核工作树，不覆盖WIP。图谱先行，本树无.codegraph则不建索引；现CRG部分覆盖且HEAD滞后，0flows/无测试边作为导航信息。

Turing仅用UID1110的 `/tmp/khor-netlab-feat-net-1110`、flock、user/netns、Rust1.95、jobs16/nice10、50GiB起40GiB停及90min资源门。网络计时保持Turing本机独占，独立Mac构建可穿插。Mac原单cache/jobs2/50–40GiB/100GiB guard保留。SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking，无凭据转发或host网络改动。

stage helper仅新增已被固定control允许的stability条目，SHA `ff09d4efb50656136d60f5ab58bc190d978923aab8519dc3b49d071208a14c1c`，审计 `stability-task-allowlist-audit-v1.json`。stage/launch只一次。观察超时后查询同一receipt和PID，不重发。先结束exec session再做依赖动作。

终态用控制树的 `scripts/collect-network-ssh.py RECEIPT`，再用ops `verify-turing-record.py RUN_DIR`。验证身份后读实际合同。Mac exactartifact下载、zipSHA、安全解包、11control blobs及所有组结果逐项核。ledger单写。动态逐case表在task.log。v5流分析用累计read/written和last进度，Ready bytes=0不能作无流量依据。缺失边界保留unknown。

原19:52 UTC的N1两至四小时窗口已经过去，不能在接续时重置为新的完成承诺。完整N0–N5原3–5工程日是量级估计，当前故障及真机/能耗验证仍影响结束时间。目标保持active，完整验收依据PLAN和ACCEPTANCE。

2026-11-11 05:36 UTC 已补五个候选变化合同，candidate_change_tests.rs，真实NO_ROUTE、新实际TLS middle、内层carried echo，且断言本机hop_epoch相同。覆盖拒绝后新增、旧请求pending时新增、无变化与仅cost变化的冷却控制、较新上下文失败先完成后旧拒绝晚回的控制。原miss_epoch测试正文逐字前缀保留，旧NoRelay fixture未改。尚未修改生产缓存，先用macserver验证旧行为。Turing088e稳定性仍在跑。

## 2026-11-11 05:46 UTC：候选变化缓存修复待验证

五fixture的test-only基线92c530/tree281337/snapshot8a568在Mac42966906265实际63pass/2FAIL。两红为真实NO_ROUTE后新增middle，以及旧请求pending时新增middle；60旧测试和3新控制全过。原fixture SHA510b70e6f0b7b6f91ff17ae1d549db99cfe172fb9e2953b5052dd82e20e4696b未改。exact artifact13830614574，zip SHA2afb0759faa8fe24a09ca8edadca443deb9d01479ea1f7f938ed6b6d9af42a9b，10files/11controls/identity核。format及net67+1ignored过，hop91.85秒；后续组未跑，不算全12。

生产17346只改hop.rs：MISSED在构建候选后按规范化middle ID集合及真实首腿可达bool登记context；RTT与排序不改变context。原MISSED表持有唯一candidate Arc，操作仅弱token，网络owner仍Wires弱token。替换context/成功移除使旧完成失效，读写均在原MISSED锁核代际；不增新表、常驻task、timer或Node/Connection强所有权。已死亡网络条目在登记时回收。原30秒起指数退避/10分钟上限、TRIES2、HOP_WITHIN3秒、caller期限及信任/暂停/单跳门均保留。compare_roads保持原近期miss旁路，只捕获明确context传入open_hop。没有把RTT噪声当新路。

此为候选，尚不能声称修复验收通过。源审计candidate-change-miss-source-audit-v1.json、fixture与旧证据在证据根。除晚完成之外，旧候选快照在多线程登记时的观察顺序还需独立审查；同ID地址变化/实际两腿/手机/成本矩阵仍未收口。用户问“做完了？”已明确答未完成，并说明原动态27绿、恢复四秒门仍有失败。没有新的精确完成时点承诺。

2026-11-11 05:50 UTC：Mac42967869816为格式门失败，9files/11controls/identity核，exact artifact13832540054，zip SHA5ce77a3d1c2530f2608352aa8da81fce209a566efce2961181a91a6f1be7c6de。没有执行单元，不能算行为红。已用git apply --check后原样应用远端format.patch，4b02只有hop.rs格式变化，原五fixture SHA不变。新Mac42968221420已登记ledger，源为4b02/tree d2c4cabb4bbe12bf614ae232e47a4da33fb54063/snapshot4d46d3009ffab2429f06f1df28258fdfa9eb9315。新run未核结果，旧收据均保留。

Turing088同一PID3381428在05:48实核running、27分02秒。已读原p1及p1j各600秒green、0切路/0退役，正在p1s原CPU停顿注入；p5未开始。未collect，不声称全长稳通过。结束后按原收据采集和逐case验证，不混入最新cache候选源码结论。


## 2026-11-11 05:56 UTC：编译修复与旧快照合同

上一goal turn有实质进展：五合同旧63/2红，缓存生产候选17346及格式4b02已提交并保留实际证据。当前仍未完成N0–N5，目标active。

Mac42968221420已终态并采集：format和net67+1ignored过；hop因三个原测试直接调用open_hop缺新增epoch参数而编译E0061，未执行hop用例。exact13831339312，zipSHA c56004ff32192e2c2636bc01d6bb632a26fbb9c755f31043bfac0c0d0c71c167，10files/11controls/identity核。已恢复open_hop原接口，明确上下文内部实现名为open_hop_in_epoch；原offer/prepare_budget/published测试正文不改。

源1d4484dcb3639bdf212dc9cd43a579100c0beb33/tree5d1016cc24e23dee4a9f9bdacae94dff1fc5a8f4/snapshot24c67bbb4a8b9cadaae9e15ccdbba6f3d5f29c11还加两个实际旧快照合同，Mac42968959558已实际提交，先查同一run。原candidate_change五测试文件逐字前缀保留，仅末尾引用新module。新candidate_snapshot_tests SHA726e1888b75a8fca33f2543ea0bfcc08830d29696413c8d96a8a08c6c6e27d20。cfg(test) gate让旧picked在MISSED登记前停住；新middle实际TLS可达、新请求真实NO_ROUTE完成后才释放旧picked，要求不重开旧路也不撤回新冷却。同候选晚登记为控制。尚未实现观察顺序保护或取得这组行为结果。

Turing088同一PID在05:55实核running、34分钟，p1/p1j/p1s各600秒原green、0切换0退役。p1s20次300msCPU停顿、最慢回显578.7ms。p5仍在执行原4次×150/300/600ms故障，每次30秒间隔，没有缩短。待整组结束采集，之后再安排最新候选Linux。


## 2026-11-11 06:12 UTC：长稳收齐、旧快照真实红与候选复验

原stability088已终态，2475.166秒、11files、准确8cd源/control68、原1线程/1轮/四前缀，identity及空进程组核。原四green均逐条核：p1/p1j/p1s各600秒、0切路0退役；p1s20次300msCPU停顿最慢578.7ms；p5各150/300/600ms×4、30秒间隔，均0Relay切换/0punch，最慢299.1/495.3/948.5ms。original-stability-verification-v1.json保存原行及SHA；原动态+稳定计划已终态。源早于NoRelay/MISSED，不外推到这些候选或全N1/手机。

Mac42968959558的1d448实际hop66/1红、87.80秒。原65全过（含候选变化五合同），同候选晚登记控制过；旧picked晚于新middle真实NO_ROUTE完成后登记，多发一次旧隧道请求，count(2,1)应为(1,1)。exactartifact13831834082，zipSHA4f07811a07ccb475c6d66ed4e2b54ca7868b5720835dcc17c6e17de95a1d2b2e，10files/11controls/identity核，后续组未跑。

新生产6e5edfec62c8d198eca9d14b47c4122fc595a8d1/tree3ac48d6834a0a432241d4e190315a367432a8566/snapshot12ef3f505525ba39a4e47f03b8527a2ce4c966f7，在读roster/roadbook前捕获单调observation顺序与弱网络token，原MISSED锁内拒绝旧登记/旧网快照；不因RTT/排序刷新，不增map/task/timer或Node强引用。两个生产入口都带其实际快照的观察；原四参数open_hop为cfg(test)适配，调用同一真实open_hop_in_epoch；原三测试正文未改，rustdoc指向真实生产函数。whole两fixture未改。原made成功删除context后的观察高水位寿命、同ID地址变化、LOST context仍待独立合同。Mac42969959757已实际success终态，artifact尚未采集，不据状态直接报全12过。

Iroh47物理对照固定四次C-B/B-C已登记confirmation-copy-identity-physical-plan-v1.json，源8cd对照2892，原20ms双腿、真实middlev4 replug、150秒prepare/10秒holds/124业务/4秒双门/3probes与v5诊断均保持，重试0。前置vendor443/58/11、双OS全12及baseline相应443/55/11/全12源与身份实核；独立Mac可同时构建，Turing计时独占。遇第一严重单向不进展/无恢复Go或连续两prepare失败按原stop停止，不加第5次。

第1候选turing-20261111T060457Z-62030f1b已收齐，178.369秒、16files/identity/空group，124/124业务，warm51/51，双向Go3569.426/3536.190ms，原预算3+3、4个innercopies界完整，8流方向单调且均有postreplug进展。原6aa未重现。一处本地分析把decision reader与selection writer误并行，读空JSON失败，空原件/stderr与analysis-dependency-race-v1.json保留；依赖顺序修正后只重做数据分析，未重跑网络产品或改4秒门。正式decision双绿。第2baseline已提交stage-launch，执行会话38375待收据，先查同一会话，不重发。
