# feat/net 当前状态与接续入口

更新于 2026-11-13T07:12:38.894382+00:00。活动目标是 [GOAL](GOAL.md)、[PLAN](PLAN.md)完整N0–N5，**未完成**；117c39当前因Release高BDP交互退化被拒绝，不得晋升为最终候选。root串行，不派子智能体。

此前STATE逐字保存在[本次完整历史](HISTORY-20261113-through-qlog-cache.md)，SHA256 `e5fdf9aed5ef256161015e24ec96c51c8a3fbc1f77db07600b09a63f6b63ce94`；它继续链接全部更早历史。历史“在途”只指其原时点，不重启旧批次。逐项未完范围见[REQUIREMENTS](REQUIREMENTS.md)。

## 现场与边界

- canonical `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch feat/net；运行时仍5064fe2a、iroh1.2/Noq1.3。升级只在独立候选，文档/control提交不等于已合入canonical运行时。
- 产品I `khor-net-product-integration-v1`，integrate/net-product-v1，HEAD **117c39e5432741198c924177fae1775ff292eada**、tree **ace4fd8223ed7fcd8d2b29477d9e651a2df53ff2**；parent是冻结对照当时的公开main **c3b098bdbf52a9b3b38e887ae74b90155292b544**，apps/与.github/逐字一致。iroh1.4.1/Noq1.4.0。不是对持续变化main的自动重基/合并。
- 所有下述树同在 `/Volumes/disk0/Khor/worktrees/current/`。S `khor-net-product-sole-peer-v1` clean ade7decb4aa2d8616fb028b0b0221873d9d6f494，原Debug唯一middle固定3已闭。
- B/C `khor-net-sole-release-{baseline,candidate}-v1`，两experiment分支、各基c3b098/117c39；共同测试overlay staged，闭合首业务及共存证据，不按摘要覆盖。
- T `khor-net-sole-cache-v1`，experiment/net-sole-cache-v1，HEAD117c39；三个新测试文件staged。闭合v1源tree **14e9f060422b3cfbd28976364d781e80daedeef7**/snapshot **05325c56df1ab25d53878bcc16537187a9abff7a**。其后追加新sp6，当前三新文件staged：v2冻结tree **5625255d023a1f3f82fb305a64aa9cc45e2b2a15**/snapshot **4e2b2c66839b8401d0ed2c1e249c6f53286415ea**；format/compile/runtime都已闭，但sp6三次前置失败，不是产品hop恢复接受。旧sp5原函数保持。
- Q `khor-net-workload-qlog-v1`，experiment/net-workload-qlog-v1，HEAD117c39；staged诊断overlay，冻结tree **d372d293f69f4980bc0926797bf99f99ffa60f3a**/snapshot **7e328dc5d000873f1cca809683fa41c0fa472e99**，三qlog病例已闭。
- A（新ACK诊断） `khor-net-workload-ack-v1`，experiment/net-workload-ack-v1，HEAD117c39；staged冻结tree **f92abcfffe5084335feb0c50007a3327d54fae07**/snapshot **9c5a4829badfa1c351fddbe38c3def3fc446937c**。仅Q上测试环境/manifest变化，qlog optional代码默认关保留；actual CLI额外编译已有net-diagnostics feature。
- control `khor-build-net` HEAD保持 **5d3691f252f9dee0297fa3c494374be6119bca29**；控制仅新建Git对象与refs/khor/net-controls。E=`/Volumes/disk0/Khor/evidence/net`。

本机source-only：Git、stdlib Python源码/元数据/数据/ZIP处理及SSH编排；不运行本机Cargo/Rust/rustfmt/Node/Bun/Xcode或下载的原生产物。重型工作只Turing/macserver/可信GitHub。用户main、已安装App、其他任务Lab23961（源98285、PID4769/exec18781）不动；无main合并、发布/标签/TestFlight、生产Relay采购部署或已安装App替换授权。用户已明确公开colehank/khor-build，Actions通道已恢复，不再按旧私库额度阻塞。

## ACK串行coordinator已闭：root恢复唯一ledger writer

**E/ops/run-workload-native-ack-three-v1.py，exec76657，localPID64594已终态0**，progress/results completed，ledger无pending；root恢复唯一writer，不重启旧批次。详[WORKLOAD-NATIVE-ACK](WORKLOAD-NATIVE-ACK.md)。fixed3共2160ok、三file全，但未复现原2秒超时、未定因；原普通Release候选拒绝保持。

- protocol **E/workload-native-ack-diagnostic-plan-v2.json**；fixed3 fresh high病例、替代0，修订在runtime前冻结。source A如上；原192MiB/三phase各目录+真实TTY各120样本/2秒caller/240秒file/600秒owned不改。
- compile **054642Z-d37db21c** 已 **0/300.285793s**，19files及identity/source/hash/group空、原native verifier的Release opt3/20first-party unit/实际CLI与test核。18项目依赖复用，仅CLI/test fresh；old Q两ELF只读保全。actual CLI/test features均net-diagnostics，khor_net实际qlog。E/run/native-ack-release-compile-verification-v1.json。原compile.json继承upstream_qlog标签未覆写，专属audit明确标注这个元数据限制；实际feature/CLI及runtime合同另核。
- runtime控制 **ab2ac0803d207a3d98271a38f7ce2ccfa08134da**，E/workload-native-ack-runtime-control-v2.json。v1仅修suite标签、未执行；不可误启动。旧qlog控制不能在已切A的stable candidate source运行。
- 首runtime **turing-20261113T055445Z-f9df1c24**，receipt E/run/receipt.json，remotePID3488460；exec28549仅提交已终态0。后两轮055802Z-ad8c39fd、060113Z-5af34dfa已由coordinator串行执行完并闭。**E/workload-native-ack-diagnostic-progress-v1.json / results-v1.json**是执行索引。
- QLOGDIR不传且显式unset，无qlog采集；NETLAB_DIAGNOSTICS=1启用已有native ACK/选路日志，保留原两resident日志及wall/monotonic锚点。300MiB日志保护只向实际Popen-owned test group TERM，原512MiB压缩/1GiB展开collector门不放宽。触cap归采集资源中断、非产品根因，停后续。
- 专属 E/ops/verify-workload-native-ack-case-v1.py 核原720总体、原Rust出口/bytes/geometry/overlap、全部日志SHA/字节及实际ACK/stream snapshot、无qlog、native owner；Rust红保留。三次均未复现，停止同样诊断，不当修复或普通性能接受。原unlogged两红仍在。

Turing原UID1110/flock/jobs16/nice10/Rust1.95/50GiB启动/40GiB停止/90分钟outer守护保持。所有老coordinator（72887、45051、68965及旧61277）均终态，不恢复。

## 当前关键失败与定位

[WORKLOAD-RELEASE](WORKLOAD-RELEASE.md)、E/workload-release-comparison-verification-v2.json：公开冻结main对I117c39，同12文件测试overlay、生产overlay0、双方原Release opt3，fixed **A B B A A B**、low40ms/8Mbit与high300ms/50Mbit各fresh；12业务case、8640原记录、12完整file，额外0。原业务caller2秒不改。两preflight沿已定MODEL-DECISION分层准入，原high cold TCP门红仍保留，独立UDP容量及真实payload/RTT/几何/owned取消守护支持Node测量；不伪造TCP校准绿。

baseline low/high各3/3；candidate low3/3、high **1/3**。candidate high共1925ok/3timeout/232blocked；blocked为未发送的意图，不当成功或传输请求。首次041753Z-362039e0为bulk TTY index1与目录index2超2秒、其后118TTY blocked；二次043619Z-e3822070为idle_after TTY index5超2秒、其后114blocked。全部file均完整；high median file19.3299→22.4087Mbit/s不抵销交互失败，idle_before TTY计划至完成经验p95约0.303→8.229秒，失败总体不能造完整成功p99。原client调度probe无同量级暂停，只限其客户端，不证明resident/host。两arm都是BBRv3/PULL_AHEAD8/response priority0，不能误称旧Cubic。

[WORKLOAD-QLOG](WORKLOAD-QLOG.md)：独立optional上游qlog诊断固定3（051025Z-5907dd5c、052027Z-1edb3353、052344Z-509f2cd6）全2160ok，三file全，8trace/case约327/334/330MB；每case分别6/6/5个EOF末条不完整，原尾字节/hash保留，不造事件缺失结论。所有owner/source/hash/原collector核；不复现原超时，停止不加同源试验，不能覆盖原两红。

已从通过的qlog case2进一步匹配同CID/stream0/方向/offset/长度的727个完整frame：TTY bulk index38耗1459798us，Y收到输入后约0.543ms就发送回复，Noq path1回复的两个传输观察点相隔1308.782ms。path1经双方tuple_assigned及iroh fd15:70a:510b:1::/64定义核为Relay；X送出364frame全path0直连，Y350frame直连、13frame Relay；后续Y回直连。file名同kernel壁钟有毫秒量化；n0-qlog0.2.0事件单位ms已核。packet_sent不是NIC时间，未细分TCP队列/物理链路，且不是原超时捕获。证据 E/workload-qlog-tty-delivery-correlation-v1.json。**新诊断问题是双向选路/共享样本及旧路径在途恢复，尚未改参数或宣称根因。**

## peer relay当前已过范围和新准备

- [SOLE-PEER-RELEASE](SOLE-PEER-RELEASE.md)：相同Release唯一M的Direct首腿/真实Relay第二腿首目录ABBAAB已闭。基线3次5秒右删失；I3次345236/366587/366320us，真实marker/唯一middle/no_relay=false/cold/实际drop与owner全核。右删失不能变成功5秒延迟或有限加速倍数。S Debug先前固定3也通过，其适用范围独立。
- [SOLE-PEER-CACHE](SOLE-PEER-CACHE.md)：T v1 compile053516Z-5aa15758 0/81.366352s；runtime **053846Z-4b1bebf2 原1/365.502303s**、36files/identity/source/hash/owner与ledger已闭。不可用第二腿hint（JSON null，非删map）、过时Direct第二腿hint、X历史Direct目标三类各3/3，hint前后保持、实际唯一M的Direct/Relay两腿和目标marker/5秒门通过。该9case不重复。
- v1 sp5三次失败原件保持，middle_down前置全false、前两次first_leg_up=false、末次middle_resumed=false。**前置未成立，不能据此将根因归给30秒miss缓存。**已核set_machine_paused接受设备名，不误诊其名字参数。
- 新 **E/sole-peer-cache-resume-plan-v2.json** 与T追加sp6：用实际M→Y暂停拒绝、persisted policy bit和X→M真实内容见证，原X失败读期间5次有界只读keepalive维持首腿；从unpause开始含实际M恢复目录及一次新X目标读共享原总5秒，same caller PID、hint held、实际两腿。固定3、仅新case，不替换旧sp5分母。**v2已执行并闭**：format060655Z-eab274c7 0/45.943910s；compile061039Z-5f6b2be8 0/77.773270s、原verifier20unit/CLI/test/identity/hash/owner核；runtime061644Z-077fb72d原1/100.191668s、18files/owner/source/identity/ledger闭。三次均真实M→Y和X→M前控制成功，但外部pause后立即M读取仍成功，前置断言失败；零X负向/恢复测量，不能作产品根因。pause.rs resident缓存和link.rs PAUSE_POLL500ms静态已核；须有界等实际拒绝再测。v3只提议E/sole-peer-cache-resume-v3-proposal.json，恢复的实际M可用等待仍必须包含在原总5秒。无新v3源码/编译/运行，不重开旧v1/v2分母。

## 可复用的其他已闭证据

- 产品I完整Quality **43348869616** FIRSTATTEMPT13/13原门成功，8原artifact SHA/字节/CRC保留 E/public-quality-net-product-v1；scope为准确I源，不能盖过N3红。
- 最新I Mac Lab **43355810513 attempt2 success**（port23963，request ad847c8fb3874591bf786afd5c347a24，native_contract71e64768d0b001dcf6f8c45deaab341e845b226ae3d58443264bdb3e083ac4d7）。原WebKit settings15s失败保留，仅失败job同源重跑1次、77browsercase全过。继承native/frontend未重建，原artifact ID/digest相同。inner native.zip108065996字节/SHA2ca9754e5831d6d1dbfaa259abe1d80da8e211484a19bb3ab6adf8e3daa0b031与ZIP CRC/MachO arm64/Info.plist独立Lab/source/tree/snapshot/contract/probe核；无下载native本地执行或安装替换。详 E/public-native-lab-net-product-v1/lab-acquisition-verification-v1.json。物理iPhone仍旧0.2，不是候选真机/能耗接受。
- 新旧CLI真实双向row/pin及两常驻host/child见证 **011900Z-19b49119** 0/160.457763s；[IROH-MIXED](IROH-MIXED.md)。只证明两懂Versions版本的窄互通。
- 策略长稳A155449Z-56a8ac18/B180542Z-c09f6a3c各5case/600秒原窗口/32逻辑目录全成功；p1h包+33.970%/字节+37.908%，p5有轻退化，非闲时或能耗最优，原用户取消164807保持。
- 3b恢复first-ACK红→Noq467、iroh198+原ignored1、12app276+ignored1及固定3次物理124/两4秒门过；[IROH-RECOVERY-ORDER](IROH-RECOVERY-ORDER.md)。旧严重红与后续未复现均保留，不追加同源挑绿。
- 上游2026-11-13T05:25UTC registry最新稳定仍iroh1.4.1/Noq与noq-proto1.4.0，E/upstream-latest-recheck-20261113T0526-v1.json。Noq GitHub latest UI旧tag不代registry。custom ledger v4共60历史项：1上游替代已证、2退出对照红保留、57仍待退出审查，绝不是57都必要。iroh21 Lagged/Openings及iOS同interface地址-only dedup只有静态观察，不冒充当前根因或已测。

## 接下来与完整交付

1. ACK三个固定病例已闭，不重复已dispatch任务。根据实际ACK/选路与连接身份，设计最小修复或明确消融；普通unlogged同源Release复核必须另立固定协议，原失败保留。
2. 修正前置的新sp6三次及其余NET-16策略/缺字段/多middle/移动范围，不能把9窄合同代全部peer relay。
3. N3补同步/预览/SSH/hop并行负载、背压/公平/取消真实资源与共享/分连接比较；1/3/5设备闲时CPU/线程/字节/唤醒/手机能耗，现有成本回退需解释/改进。
4. N4双方与peer relay IPv4/IPv6/NAT独立或同时变址、prefix/Wi-Fi↔蜂窝/锁屏休眠、寻址与Relay登记/出口故障域/容量/布局；MacLab与Turing同LAN，macserver另机。新候选物理iPhone窗口未执行。
5. 所有custom逐项退出依据、状态/职责及持续短回归/完整矩阵/前后对照/中继运维设计，最终收敛到同一准确source；与届时main隔离集成并跑准确源完整Quality/Mac原生/手机接受。无main合并或正式发布授权。

所有已闭结果只复用于其准确源码/范围。失败、控制前置失败、用户取消、采集资源中断及未验证分别记；源码计划准备不冒充实现/接受。最终完整目标、[DESIGN](DESIGN.md)、需求审计的未完项不因局部绿缩减。

首ACK病例的1.664秒TTY尾与单Direct copy2PTO/另一copy较新收包和全局silence重合，move仍Hold；精确hash-bound行证据见E/workload-native-ack-first-tail-observation-v1.json。静态off_direct guard限制旧copy wake仅为待检假设，不冒充已证根因。接下来先修正前置的新sp6，并以明确受控问题收敛路径/连接停顿，不再追加相同日志试验。


## 下一独立消融已准备（尚未格式/编译/执行）

`khor-net-copy-wake-v1` / experiment/net-copy-wake-v1，HEAD117c39。新的专属worktree以闭合unlogged C共同fixture treeadbc7d6a5032cdc24db19cdbac4b9d1f7ec6415c为测试overlay，仅vendor/iroh/src/socket/remote_map/remote_state.rs有runtime差异：取消off_direct传输种类guard，调用已有wake(&backed_off,&heard)；原to_wake等待/同road收包证据、所有timer/质量/滞回/Noq/期限/重放/信任不改。测试overlay staged，runtime源为unstaged；不要按摘要覆盖。E/copy-wake-ablation-design-v1.json记录具体问题、依据、范围和旧p1s历史风险（旧守门前3/6健康抖动超move门），不是已证修复或最终默认设计。

后续先remote格式/库合同与原健康spike/黑洞/恢复负向，性能另冻双方准确source/producers/control/ABBAAB/完整人口与成本/吞吐/长尾判据。可考虑复用准确原Release产物减少重复编译，但须保全**全部selected文件（ELF与dep-info）**、源/context/path/字节哈希并过原native verifier；不能只保ELF或改proof。若不能严格准入则正常重编。还没有该消融Turing提交/运行，不能把准备等于改进。

最新现场：所有本轮原生任务/ACK coordinator均终态、E/source-refs无pending，root唯一ledger writer。canonical/I/main运行时均未应用此消融。完整目标仍未完成，当前117c39仍因原N3 high失败被拒绝。

### 新轮实证（2026-11-13T06:36:35.502708+00:00）

copy-wake消融source已冻结 f5faf2a2fef7fd64f09908fc53c73b24bb2eae4d / 8780bf503d753b820a7236931357b2dcebfa42a6，W所有更改已staged。library控制39ffdc8079f5bbe4bf7f023963e51ca2ce41e545，run063044Z-bf939c7b已0/154.431210s、完整iroh lib **198/0/原ignored1**、0filter，15files/metadata实际vendoredNoq-proto1.4路径与iroh1.4.1/noq1.4/source/hash/identity/owner核、ledger闭。Graph旧a3e07且不覆盖vendor，定向diff/源码人工核补，不擅建CodeGraph。编译约含下载跨平台metadata的开销；未来metadata应filter host。只是库行为，不是实际spike/黑洞/Release性能或N3接受；Noq算法本轮无diff，不重复其原已闭whole suite。

T新增sp7源码和 E/sole-peer-cache-resume-plan-v3.json 已预先固定3、替代0：10s setup有界等实际M错误包含peer_paused_refusal，成功的旧缓存响应/无关超时不算策略拒绝；恢复时M实际可用观察也含在原总5s，然后一次新X业务读。原sp5/sp6及所有分母保持。当前仅新增sp7源码（staged）待remote格式，**无v3编译/runtime**。预格式source9a2b273289c8d6df030d250f5732fc274fb759d1 / 057b0bbcbdca90f0621091ed53c413205342e6b0，controlad92cab61aed98696ca776f4db0b80cb3757739d。prep已经提交exec31550（仅提交）；真实run句柄以最新receipt为准。root唯一ledger writer，先核当前prep终态再发新任务。


### NET-16应用策略后恢复：有效三红已闭（2026-11-13T06:55:00.619059+00:00）

T3/sp7格式063455Z-c208e81b、编译063838Z-cdb6f871均成功，tree8ff93012943b0b6c6b598142559b74e682e58829 / snapshot2c66f25350ecc698cc91b4dc180717261a3e5d4b；T三个测试文件staged，产品117c39未改。原native verifier/20unit与同producer CLI/test/source/identity/hash/owner全核。CLI仍f129a57c，证明fixture修正未改产品。

runtime064222Z-57c213de，controlf4f202bf62ab9bfed86ddfec69d063bba8cc19a9，原1/114.475143s、18files/source/hash/identity/原group空与三个case-owned干净，ledger已闭。三个case前置**全部为真**：实际M暂停拒绝、X负向读、五次X→M真实读、首腿live、same caller PID、hint held；unpause后M在441358/337646/554797us读到真实Y内容，但一次新X请求仍耗尽原总5s（5008873/5026542/5008990us），实际两腿未形成。每case仅暂停期间有一次hop(tried1)，恢复后无新hop。proof E/turing-20261113T064222Z-57c213de/sole-cache-resume-physical-verification-v3.json。**这是有效产品恢复红，非sp5/6前置失败。**原red全保留，不追加不变病例。

当前优先查明/修复NET-16失败缓存对救通的压制。已核hop_ahead全走每candidate cooling过滤，MISSED_FIRST30s；journal::By区分Pump/Forced/Ask，有限deadline单独不能代前台来源。尚未宣称唯一根因或已修复。需要隔离新candidate，对活首腿的有界需求恢复放行，同时保留背景指数退避、并发/epoch/期限/策略/信任/单跳及写不重放。W消融库198过仍非物理/性能接受，暂后于确定的NET-16红。root唯一ledger writer，无pending。完整N0–N5未完成，I原N3高BDP拒绝不变。


### NET-16独立修复准备中

新R `/Volumes/disk0/Khor/worktrees/current/khor-net-live-middle-retry-v1` / experiment/net-live-middle-retry-v1，HEAD117c39，准确继承T3原fixture tree8ff930；仅hop.rs有runtime差异，两原node测试文件新增需求/后台/短caller/并发32控制。stage包括6文件，预格式tree9570105a4545452d66b928786e23c8b089ec4b78 / snapshot2f0ce65ba360077cd396146a65e39c1e2a6cd285。有限Ask/Forced且无target road、实际活首腿可在原总期限/原hop3s内重查；活首腿失败重查最短间隔复用HOP_WITHIN3s，原后台/比较指数退避、tries2、owner/epoch/策略/信任/写重放不变。不是已验证修复。协议E/live-middle-retry-plan-v1.json。

remote格式准备065900Z-8123d068、control6d09e1f39126b50c278c750cb4f388609cade323已提交；exec80771仅提交终态0，remotePID3627645真实任务须先核终态与收集/identity/patch后再发compile。root唯一writer，待闭这一个prep。W/I/canonical运行时不改，原red均保留。

R格式065900Z-8123d068已0/46.101216s、12files/source/hash/identity/owner及准确patch核并应用，ledger闭。冻结R treecf9d172bb92778ccfd97899ddea27bdf0e6d2f6e / snapshota3cad264bc6772d68289b5f954c0966dc87627cd，6文件staged、原T3三个fixture逐字相同（diff只三个node文件）。compile+whole hop模块合同070105Z-8e82e611 / control27e55eedd7dbb92c2c907d732ca73d8de18468d1已提交，exec74748仅提交终态0，remotePID3631399真实任务在途。先闭compile，不提前发physical。

070105Z-8e82e611在途更新：原whole hop::模块92/92、0failed/ignored，测试113.32s；新实际NO_ROUTE→需求恢复/背景退避/短caller/并发32均绿，原guard测试也绿。真实CLI/test producer准入尚未终态，不能提前发physical。原测试fixture与physical固定3协议E/live-middle-retry-physical-plan-v1.json已冻结。

R compile070105Z-8e82e611已0/281.033342s、17files/identity/source/hash/owner核，原native verifier20unit、实际CLI/test核，whole hop92/0/0（113.32s），ledger闭。CLI856ecd02bf638c2b08908a625e5579aac732d4369866df36eca5a6da0d101121；test72a8b6536c60eefd4adede0aa48aa549b07b93bcde6d64ecd64ec8d4b8b5afd3。原CLI/test只读保留closed-sole-cache-resume-v3。专属E/run/live-middle-retry-compile-verification-v1.json明确legacy compile-only元数据不涵盖已执行92库测试。

固定原sp7×3 physical控制8b36679b18dce3758baf0b9a995fe35f38a9b61f已提交，最新pending索引[{"run_id": "turing-20261113T070832Z-6b87e77c", "pid": 3643900, "submission_state": "launched"}]；先核receipt真实任务，root单writer，不重复dispatch。原fixture/原5s/策略前置/owned核不变。


### R唯一middle恢复已闭；下一步回到N3

详[LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)。R已commit9081b94dce827cbd3fb9b9e7bb13633d762e3cd9、clean，treecf9d172bb92778ccfd97899ddea27bdf0e6d2f6e准确等同冻结被测snapshot。runtime070832Z-6b87e77c / control8b36679b18dce3758baf0b9a995fe35f38a9b61f **0/98.267049s**，18files/hash/source/identity/owner核、固定原sp7 **3/3**，总恢复1013894/784788/878995us、原5秒不变、目标marker/真Direct-Relay两腿/hint held/same PID/drop counters全核，ledger闭。原有效3红保留；92hop tests与actual native producer也闭。只接受这项NET-16恢复，不是full NET16/N3/phone/energy/N0–N5。canonical/I运行时不动。

当前无pending任务，root唯一ledger writer。下一步W copy-wake消融的原健康spike/短停顿/黑洞控制（原600s/原4drops不缩短），再基于结果决定是否继续普通Release性能复核；不再重复原sp7挑绿。多middle排序/完整策略与移动等仍待。

W原physical控制已预先冻结E/copy-wake-original-controls-plan-v1.json：p1s原600s固定6fresh→p3原黑洞1→p5原4drops各stall1，串行、替代0；首产品红或owner/资源不安全停剩余并显式记not-executed，不补绿。原历史p1s3/6风险仍保持。仅owned watchdog1200s容纳原600s场景，产品/网络/test门不放宽，outer90min保持。
W actual netlab/CLI compile071457Z-c140a37b / control1a01b55207e0da74fe47561d833f5dd19909d702已提交，exec9298仅提交终态0，remotePID3656679真实任务；须先核终态/原native verifier再发physical。已命令保全R所有selected文件含ELF+dep-info至closed-live-middle-retry-v1，保全bytes不豁免未来原verifier。R修复不混入W源。当前root唯一writer，最新ledger为准。

W native compile071457Z-c140a37b已0/116.466771s、17files/identity/source/hash/owner/原native verifier20unit/test实际netlab与CLI同producer核、ledger闭；CLIbc47a17b2d3a424690478c42d8d556f66e7e21eee7ea02f1fdf48d24e07ff375，test8116cf8e939cf3a795266b2c6bfccb1e5209267a474794b8c3a0b5da0f72809f，R所有selected五文件含depinfo已只读保全、原bytes核。
W原控制runtime control94849c5a1748121af392ec68c00d153b4aba4482已提交，exec57768仅提交句柄；最新真实pending [{"run_id": "turing-20261113T071957Z-261fdeee", "pid": 3663224, "submission_state": "launched"}]。每case原native verifier复核，owned supervisor1200s（仅watchdog），原600s/p5每stall4drops/Rust原门不变。planned8：p1s×6→p3→p5，串行fixed0replacement，首红/不安全停后续并显式保留未执行项。状态看report/copy-wake-controls-progress.json，最终copy-wake-original-controls-contract-v1.json。root唯一writer，不重启或另发同源批次。此在途段不等于性能接受。


### W原控制v1为执行器中断，v2已修正监督容量

071957Z-261fdeee原1/594.071116s、15files/source/hash/identity核；只执行首p1s至supervisor524.866815s，historical owner inventory513触原512cap，native-15/forced cleanup，未到原600s/Rust verdict，无netlab-result/summary，14个已完成业务观察均ok。**不是W产品回归或接受**。其余7未执行，原结果完整保持，ledger闭。原case出口group_absent=false/8zombies保持；controller终态后独立只读PGID3664371与八个/proc项全消失，专属postmortem proof保留，不覆写原失败清理字段。

v2协议E/copy-wake-original-controls-plan-v2.json：仍p1s×6→p3→p5、原600s/原4drops/原门；保持512同时匹配owner上限，历史birth identity记录另有8192有界cap并记录peak live/history；以纯数据校准2000历史+1活、513活应拒、8193历史应拒。不改产品源/原native verifier，复用同actual producer无需重编。controlc2c2d706799a26585febf3d135d43ca039a7e327，提交exec23870；最新pending [{"run_id": "turing-20261113T073647Z-7e574645", "pid": 3708886, "submission_state": "launched"}]。root唯一writer，先核当前任务，不重开v1。最终report/copy-wake-original-controls-contract-v2.json、在途copy-wake-controls-progress-v2.json。

只读移动链路审计见[MOBILE-ADDRESS-AUDIT](MOBILE-ADDRESS-AUDIT.md)：准确netwatch0.19.3原crate checksum核，iOS有AF_ROUTE地址通知、minor也进iroh重新寻址/通知QUIC。App同interface种类去重不单独证明漏地址事件，须实际移动/发布/epoch/phone测试。无产品修改或N4接受；持续路由事件下250ms debounce只列待测风险，不作为已证根因。


2026-11-13T07:42Z现场核：当前唯一pending为 **073647Z-7e574645**（v2，remotePID3708886/child3709408，exec23870仅提交已终态0），准确W/source/control同上。report确认监督纯数据三边界校准全过、首p1s实际执行中；尚无该case verdict。继续先核原task，不重复dispatch、不覆盖未完分母。v1中断已闭；R恢复3/3与92测试仍成立，I原N3拒绝保持。

等长稳窗口时仅做只读退出审查：原E/upstream-20261109/iroh-1.4.1.crate的e3689cab508af9bcef7816c1201b403595ec6aa20e7d15e1f9683bb4be4a5b17与原registry元数据checksum核，从包内读取源码到E/upstream-iroh-141-address-review-v1。patch38当前source审查E/iroh-patch38-source-exit-review-v1.json：upstream Established时登记tuple，RemoteStatus/ObservedAddr忽略；未见同等refresh scan，但不能据此认定当前Noq1.4.0必须保留或删除。需同准确新源的真正NAT重绑退出对照与成本/tuple归属；ledger57 pending不擅改。没有新产品源码或本机native运行。

2026-11-13T07:50Z核073647Z-7e574645仍活、active_case2；原p1s case1已通过原600s，整体684.680064s、owner exit0/group空/无强制，peak_live10、peak_history697。原immutable case1 outcome的只读SHA快照E/live-copy-wake-controls-v2-case1.json，最终collector/hash/identity须等本批终态，不当8/8或N3接受。
