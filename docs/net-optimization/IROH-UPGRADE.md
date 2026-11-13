# iroh 稳定版升级与补丁退出

2026-11-12，按用户U18实施。完整目标仍为[PLAN](PLAN.md) N0–N5，当前执行入口见[STATE](STATE.md)。本页描述升级候选与已验证边界；旧版、全部原红及冻结性能源保留。

## 候选和迁移

隔离树 `khor-net-iroh-upgrade-v1`，分支 `experiment/net-iroh-upgrade-v1`，冻结源 **02807c79e04a92c46fb2e6c14d2ecf107b98336c** / tree **f87d31f97b89474fe52297d87e3827de33550493**。
使用已核对的最新稳定 **iroh1.4.1 + Noq1.4.0**，根及vendor锁一致。[官方latest release](https://github.com/n0-computer/iroh/releases/latest)在2026-11-12复核仍指向v1.4.1；跟踪稳定发布，不将主干或实验transport的更新等同于可发布版本。Canonical feat/net仍为旧运行时5064fe2a（iroh1.2.0/Noq-proto1.3.0），尚未合入本候选。

- 以旧发布包为共同基线三方迁移，7处冲突的逐文件记录为E/iroh-upgrade-source-merge-v1.json。保留本地速度选路、pending队列硬上限/路径ID释放重开、原专属回归，并保留上游并行Initial发送与blocked-Relay回归。
- 发布包均与registry SHA256一致。解包目录再次逐文件对照原包：iroh85文件、Noq-proto70文件，无不同（E/upstream-20261109/published-directory-audit-20261112-v1.json）。未将修改后的vendor当作上游源码。
- app net/node Rust和四份wire metadata相对旧整合源dc73837e逐字不变，E/iroh-upgrade-runtime-diff-v1.json。依赖变化仍改变运行时，旧性能不自动适用。
- Turing生成锁/formatter的准备轮不是原冻结源接受。首编译红与共享source保护拒绝均保留；原准备目录完整改名保存，未reset/删除。后续冻结源才执行下列回归。
- 本机source-only；重型验证Turing与macserver/private khor-build。无本任务main合并、发布、生产Relay采购部署或已装App替换。

## 已完成的候选验证

| 范围 | 精确run与结果 |
| --- | --- |
| Linux全vendor库门 | 084724Z-35d6dfe3，0/226.384657s；Noq467过，iroh197过/原flaky ignored1；node lib/tests编译、三锁、post-check源码diff均通过 |
| Mac同树库门 | 43186999295；与Linux相同命名人口及原ignored1，16原files/11controls核验 |
| 应用固定12 gate | 085339Z-ab57a92e，0/385.723121s；273过/原identity helper ignored1，与旧dc738人口相同 |
| 36真实流合同 | 090144Z-223b1901，0/31.781419s；4准确冻结解析器重算，独立74Fetch RPC/10peer；与273有重叠，不累加 |
| 实际CLI取消两合同 | 095017Z-a01289cd，0/186.335657s；2795源文件/CLI prior→node provenance/选中binary核对；两parent及两声明helper，6不同任务的Cancel IPC到达，4/24owner退出无遗留；控制Python sidecar，非真实agent bundle/WAN |
| netlab目标编译 | 100217Z-d286d5bb，0/120.570580s；locked compile及artifact/binary SHA/源码diff通过，runtime0，不称native_provenance接受 |
| 静默first-action诊断 | 100747Z-15afeb63，0/332.109970s；2 guards/6fresh读，12准备/12完整≥10s静默/12真实STREAM阳性；普通性能样本0 |
| 实际接口换网三场景 | 102059Z-2a7ba71e，0/186.422335s；dual/v4-NAT/v4→v6，372逻辑请求/744起止全ok、非移动端先读、内核地址独立核对；未证明实际QUIC端口重绑定/同连接/手机/完整N2 |

所有Turing run前缀为turing-20261112T；完整source/control/bundle/原文件hash/进程组空与行为proof分别保存在E/run目录，ledger终态。单次换网读取不构成p99/SLO或能耗结论。IPv6 QAD unit含地址不可用提前返回，普通ok不独立证明IPv6 WAN。

## peer relay 新红与确认

新源 **105138Z-c9de2969** 原101/150.987066s：第一笔setup读取10.039537s超时，**尚未注入middle换网**；16原files和准确attempts核验，原失败保留。
旧dc738同control69d23d4f的 **110236Z-baeba5f5** 原0/213.826096s：124请求全ok，middle实际IPv4变化，变前carried连续报告≥10s、变后再观察carried。
新红启动load90.18/128逻辑CPU，旧绿9.78；这只是差异，不能据一次对照归因上游或负载。新源原样确认 **110840Z-1e90b3dd** 已0/339.290546s，124请求全ok且变后再观测carried；源/控制/原hash/进程组空/行为已核验。其启动load144.61比首红更高，不能以单一host load解释。三轮及5份完整相同fixture绑定在E/iroh14-peer-mobility-comparison-v1.json。**首连间歇红未定位/未豁免**；不再无假设原样重跑挑绿。IPv6 overlap两合同111507Z-d7912aa7已0/165.659413s，248请求/496起止全ok、内核地址/隔离/remote-first核验；同前缀替换，不称手机/同连接或完整N2。实际QUIC NAT端口合同111859Z-de624e0b也已0/99.774935s：124请求全ok、30/30 warm远端stream选中新tuple、mismatch0，两个frozen metadata/原source/hash/进程组空核验。未证明同连接/逐packet/手机。同条件普通负载六block对照现已结束，8640交互全ok/12file完整；有文件收益/轻载交互代价和终端长尾，见IROH-WORKLOAD-RESULTS，未接受N3。唯一在途见STATE。

## 已补查的原四秒恢复门：新候选红

E/iroh14-peer-strict-gate-comparison-v1.json把原frozen selection/decision门用于上述两轮完整日志，零新产品执行。旧110236两门过，inner Go3472631/3462188us；新110840两门红，Go9301772/9349684us，52个eligible warm请求中15个AB仍在错误选择。原availability通过仅限业务可达，不代表最佳路及时恢复。新候选不能验收或合入canonical。

原行绑定与后续假设见E/iroh14-peer-recovery-sample-gap-v1.json：carried质量111.16ms在2.535–6.998s间未刷新（age增到4.835s），Relay约135ms，原25%gain门保留Relay；新carried约98ms后才于7.3s重启2s确认。这是request-open观测与源码/data推断，仍需定位采样代表copy、probe copy、取消后的有界刷新及外层/调度，不能当唯一上游原因。原四秒门/探测预算/新鲜性规则保持，冻结性能源028不改。

## 补丁退出与原失败

E/iroh-upgrade-patch-ledger-v4.json的60项历史账目为：**1项上游替代库验证、2项保护由删除对照支持保留、57项待逐项退出核查**。57不代表全部必要；各项原原因/覆盖/退出合同见[UPSTREAM](UPSTREAM.md)和[VENDOR-AUDIT](VENDOR-AUDIT.md)。

- iroh22手工IPv6字面地址trim已替换上游relay_tls_server_name；helper与两测试体忽略空白相同，新源库门通过。未扩称手机/WAN。
- Noq5删除源e69b137c：091835Z-cb319e57原101，4未变fixture三过一红，合法已分配/回收ACK-only最大号加低号需ACK包，误增RTT counter5→6。基线Noq1.4同四测试绿，保护保留；这是状态unit模型，非wire性能。
- Noq2删除源79d2ca64：093340Z-deb1f7de原101，双方reporting保持both、服务端握手响应前发STREAM时出现不允许的0-RTT ObservedAddr TX1；基线同fixture绿，保护保留。两删除源不晋升。
- 新/旧完整node诊断090450Z-bfc82a31 / 092140Z-5216816b均1387人口、1274过/同38失败/75原ignored，无新独有失败。20项直接观察Git2.25.1不支持initial-branch；CLI/agent/native等前置不完整。两CLI合同已另按正确前置通过，**整套诊断仍非Quality完整接受，也不豁免其他原红**。
- 首CLI诊断094221Z-91240bdf控制漏声明合法helper导致严格metadata拒绝；原失败保存，v2只补控制声明与attempt持久化，未改产品或断言。
- Mac退出43189070840为50GiB启动余量门拒绝、零产品测试。后续只读库存43205919189又被GitHub付款/额度在prepare前拒绝，storage skipped、零Mac操作；已请求恢复额度或常规SSH，Turing继续。不降门/清未知缓存。

## 仍需完成

peer relay前置红与四秒恢复红定位、其余补丁语义/退出核查、完整原生Quality与Lab/真机/能耗验收仍开放。实际QUIC NAT端口/IPv6 overlap已有上述限定场景证据，同条件普通负载对照已闭；它们不关闭完整N2/N3。官网BLE/Tor/Nym扩展未接入本轮；上游custom transport不替代khor的两腿选择、信任或自动接管。

## 下一项缩减候选：把补丁21的两种行为分开

静态审查E/iroh14-event-buffer-exit-audit-v1.json绑定候选028和准确上游1.4.1文件SHA：上游已有PathEvent::Lagged与Connection::paths快照恢复，容量8；本地容量64。roads::follow已在Lagged时记录漏事件并读取当前选择，但该分支没有对账其mine/Openings直接路径库存；它只用于日志去重，不据此归因选路故障。

先用真实连接让消费方落后超过8个事件，验证当前选择、库存对账、日志与关闭清理，再考虑退出64覆盖。优先使用上游现有快照API，不增加定时器。同一历史21的Path::ping是另一项能力，不能因容量恢复就把整条21记为删除。当前仅静态退出候选，零新增产品测试/实现；冻结性能源不改。

## iOS通知链仍须真机核对

E/iroh14-ios-notification-static-audit-v1.json核对旧/新三文件逐字相同：watch_the_path已从生命周期入口启动，callback→link::network_changed→resident Wake::Network(Path)→wires恢复/iroh network_change/有界round的链存在；resting期间Network交由Resume处理。
该callback去重键只有status和Wi-Fi/cellular/wired类型集合，同interface内仅地址/地址族变化不单独穿过这一去重。iroh自身另有监控，所以**不能由静态代码判为漏事件或现场故障原因**。必须在真机记录同Wi-Fi的IPv4/IPv6地址/前缀/地址族变化、callback/iroh事件归属、remote-first与休眠恢复顺序；Linux场景未执行此iOS回调。当前零新增产品实现或手机接受。

## 当前普通对照与独立诊断

六block源028普通对照已闭：[完整结果](IROH-WORKLOAD-RESULTS.md)，E/iroh14-workload-comparison-v1.json SHA940dda764a0f7787bf2439194b9bed6802ee3dbeed3736d60fe709358328732a，8640交互/12file完整，所有人口保留，吞吐/交互有取舍，非N3/最优接受。
独立源08ecab50仅补原DEBUG下pending取消/Rest事实，不改变算法，结构审计E/iroh14-peer-diagnostic-source-audit-v1.json。121515Z-e8d79c86与预先冻结的追加122705Z-1248c9ae/123405Z-8a26c7cf共三轮均已终态，各124请求全ok；首轮Y缺Go保留decision红，追加两轮原四秒两门绿（X/Y约3.51/3.51s、3.28/3.54s）。E/iroh14-pending-cancel-two-diagnostics-results-v1.json绑定全部proof，批次按两追加轮上限关闭。没有重现9.3s或捕获目标Relay→Custom取消，不证明根因/修复，也不晋升canonical。原失败X确认质量代表和实际probe的副本不同，下一步检查同PTO tie-break与测量所有权，不能据绿诊断豁免。

## 确认copy候选未通过物理门

[独立设计与全证据](IROH-CONFIRM-COPY.md)：原日志证明同PTO副本质量来源与probe不一致；等价抽取源9f3a6ee6新增行为回归真实红，其余原197/467过。修复源80aeeff9仅equal-PTO tie-break，准确全库125007Z-645fe88c绿（Noq467/iroh198，原ignored1）。但固定三轮物理批次125445/125919/130239为首连失败、恢复5.52/7.76s失败、3.38/3.61s通过，全部保留。E/iroh14-confirm-copy-physical-results-v1.json accepted=false；不以单位绿晋升，028原红保持。

下一步首连阶段诊断从028独立派生，复用已有opt-in observe_lifecycle，不增加依赖/恢复状态或改变预算。固定3fresh协议/准确当前run以STATE为准，不继续原样跑80挑绿。混版/Quality/真机与其余补丁退出仍开放。

## 官网连接模式与兼容政策复核（2026-11-12）

[官方Tor文档](https://docs.iroh.computer/transports/tor)与[BLE文档](https://docs.iroh.computer/transports/bluetooth)均明确custom transport API及相应crate仍为experimental。它们提供新的承载方式，不能由此推出khor的两腿选择、信任、单跳、deadline或自动接管已由上游实现。本轮不扩展到新承载，优先复用上游已有API、核查可删除的vendor覆盖，并守住个人网实际体验。

[官方release policy](https://docs.iroh.computer/about/release-policy)承诺同major内minor wire兼容。这支持开展1.2↔1.4混版验证，但不替代应用协议、本地vendor补丁、会话生命周期或具体已安装iPhone版本的互通证据。现有sync_mixed可复用；必须注入真实准确旧CLI且检查非skip标记。

首连阶段被动诊断794（由028派生）固定三轮132402/132802/133218已全部终态，每轮124请求全ok、各16原files；未捕获首连红，三轮原decision门仍红（Y约7.61/7.59/5.45s，另有X缺Go）。批次停止，E/iroh14-hop-preparation-diagnostic-results-v1.json完整留证，不晋升/不宣告修复。原首连105138/125445与恢复110840红保持；下一项设计提议见IROH-HYSTERESIS，尚未实现。
