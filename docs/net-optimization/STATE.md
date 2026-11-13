# feat/net 当前状态与接续入口

更新于 2026-11-13T20:55:26.503130+00:00。完整[GOAL](GOAL.md)/[PLAN](PLAN.md) N0–N5 **未完成**；逐项缺口见[REQUIREMENTS](REQUIREMENTS.md)。用户已授权持续实施/远端验证，root串行、不派子智能体；不因局部通过停止。无main合并、正式发布、生产Relay采购部署或现装App替换授权。

E=`/Volumes/disk0/Khor/evidence/net`；全部worktree在`/Volumes/disk0/Khor/worktrees/current/`。此前STATE逐字归档[HISTORY-through-relay-health-v2](HISTORY-20261113-through-relay-health-v2.md)，SHA256 `04c4a8d83d8f48a963f0762581843009788d3f24cdadeebd89f29dc55ce744c5`；其中保留更早全部历史/失败/来源链接。历史pending/PID不用于重派。

## 当前唯一执行与接着工作

**F3十组件203529Z-34d82f2e已0/72.022379秒闭证**，25files/identity/owned完整，冷编译33.83秒、413artifact全部fresh=false；新增3与原7全部通过。run/relay-health-progress-component-verification-v1.json；不代Release或真机结论。

**F3普通Release producer203855Z-f070edef已0/592.435102秒闭证**，22files/identity/strict native完整，Rust9分01秒、实际CLI fresh=false。source7a79427414b40d96d18db6a867236560ac6d01bb/tree173ce959113a03b42b2c556a69140d8c63e9a5e9/snapshoteef032c13f37f66d049f58c12be0cc93ea1afd4b；CLI a2a655f73f168dc10ed580e1a578c5db5c3056a6d5207aacceebc82aebba450d，fulltest788230d99aec5e32ae4db895f55dcaf5b1caad9b08b295bfbede80a600e6f181。run/relay-health-progress-native-compile-verification-v1.json。普通Release/原features[]/无qlog与net-diagnostics保持，不代业务接受。

**F3两组原业务合同已全部闭证，共固定6场168/168真实读成功、0超时。** silent205454Z-a05d8a1c 0/166.430165秒、15files/identity/native/CPU/owned完整，三场首有效响应953959/872597/934411μs，原2秒caller/250ms计划不改。healthy205820Z-118a2b49 0/102.174631秒，同15files与来源/资源/清理闭，42/42读成功、三场原TCP均不变、changed=0，124→976ms原delay/filter/no-drop/5秒caller保持。两原fixture逐字不改，无运行编译/替代；E/relay-health-progress-combined-runtime-verification-v1.json及两run/relay-progress-*-runtime-verification-v1.json。仅窄修复合同接受，不代全N4/N5或总体可靠率。

**当前Turing无pending。独立I3已保留F3并无冲突集成main与harness，接着冻结准确源检查**：E/relay-health-progress-integration-intent-v1.json前置两组已满足。新khor-net-product-integration-v3 / integrate/net-product-v3从F3保留全部runtime，合入已观察main9a4219ff并应用独立harness87ff588a；main/现装App/旧Lab23961保持。补齐patch15扩展说明，冻结准确clean源后发可信完整Quality和Mac Lab，继续旧N3/N4/custom缺口；不得把这6绿写成完整目标完成。

**健康jitter真实3场已完整重现F2误断，F2不晋升。** runtime201418Z-2b427674 **1/112.210432秒**、15files/identity/native/CPU/owned闭：42/42实际读成功、无队列drop，但三场原TCP都在约1.11秒消失后被新tuple替换。100ms内核采样、原124→976ms delay/filter/无drop和原5秒caller均独立核验，protocol_verified=true；每场158/159/161次改变观测。run/relay-health-jitter-runtime-verification-v1.json。原silent3场改善依然保留，但不足以全面采用F2。

**下一候选F3已在独立khor-net-relay-health-progress-v1提交7a794274，十组件与Release已核，仅窄6场业务接受，完整目标未齐。** 从J2 a199ff48起仅改vendor/iroh/src/socket/transports/relay/actor.rs，原silent与healthy夹具全部字节保持。E/relay-health-progress-design-v1.json提前冻结：同一ping的full max_timeout仍从首次发送固定不延长；RTT早期检查仅在没有收到Relay消息时判坏，收到实际消息只让过早期判断，匹配pong才结束整次检查。connected/pending-send两循环共用状态，无新周期timer/额外ping/应用重放。新增3原生合同（无回包早期失败、有进展等pong、持续进展不延长硬期限），保留原7，先跑固定10组件再普通Release原6场。

健康夹具J2原生producer200542Z-1bf520ed **0/238.758950秒**、24files/identity/strict native闭，Rust3分08秒。source a199ff480e7c817c6a4bcbb94e3f349e85a4e23f/tree3b4a982616787852a5a9561b799f5656dd74fd3a，CLI逐字F2 ef9a、fulltest3813e1a72e0295fb8eaf3865702053bb5ed757e9fa0a73bfa30e367d27ea65b7。run/relay-health-jitter-native-compile-verification-v2.json；该源只是测试桥，产品仍F2。

首个健康夹具预校准195501Z-d304e9a5 **1/25.804405秒**保留，14files/identity闭，tc filter JSON格式导致解析失败，Cargo/产品/场景0；run/relay-health-jitter-preflight-failure-v1.json。随后改为filter原文与qdisc JSON、实际u32/统计核验，先恢复旧准入F2 source并验selected5，再切J2，完整来源/恢复记录保留；不弱化SLO或记产品失败。

root唯一ledger writer，无后台协调器。恢复先查source-refs和实际worktree/source，不按历史PID/旧pending重派。下一组件入口和准确源码见下次实际receipt；旧F2控制器不能拿来冒充新F3。

## 已确认的本轮改进

**真实前台Relay TCP静默：固定3红→固定3绿。** 详[RELAY-DEMAND](RELAY-DEMAND.md)及[DESIGN](DESIGN.md)。原双strict NAT/单Relay只drop旧alpha TCP tuple、新flow可通。原idle c的30秒观察门曾通过，但没有前台业务；不能拿它代最佳体验。

- 旧源D `62b28e084a8cf72593a9ed1ba270327d281f9d49`仅加测试、runtime逐字I2/R。普通Release runtime `184914Z-5cfe53b4` **1/198.559489秒**，15files/identity/native/CPU/owned闭：126实际读**96ok/30timeout**；每场40故障后读10timeout/30ok，首有效响应20042371/20040904/20049739μs。真实nft/tuple/新marker及所有起止记录独立核验，原件保留。
- 修复F2 `81ae5c9812b06298a06549f1c16a3728baac2c8d` / tree`ba3da3451c027904b657183bc26d990191d75c30`，worktree khor-net-relay-demand-repair-v1。resident有活请求、开始后未收到任何对端datagram、选中Relay且达到max(250ms,3×路RTT)时，提示现有Relay健康检查；同TCP actor仅持一个未完成health ping，匹配pong/原timeout结束它，周期ping不能替换deadline。原wire退役/读写重试/总预算不改，无全endpoint rebind或新增周期timer。入口也供后台同步使用，不能宣称idle流量/能耗不变。
- F2普通Release producer `191019Z-979f0ccd` **0/594.656134秒**、22files/identity/strict native闭，Rust9分02秒，CLI **ef9a3dd1061ab557c4aa1709b9a14901813efff2e91251bbdef5e415f8c49519**，fulltest **2296f0f2b87910dee724f65b9b52957e7e255bf9e0d9a29ea24fbb63f1816f4b**，实际fresh=false。run/relay-silent-demand-repair-native-compile-verification-v2.json。
- 原fixture逐字不变的F2 runtime `192107Z-941d6b67` **0/166.445581秒**、15files/identity/native/CPU/owned闭：**126/126成功、0超时**，首有效响应**960623/958965/906570μs**，原2秒caller/250ms计划间隔未改，无替代/运行编译。control5ee17e95bb98278e1fd6f65e846b8896a788e96a，协议2f07687a98b22247826a0ffd37f0632d6bf5de41dfb5e2655346821935c7ecdc；run/relay-silent-demand-repair-runtime-verification-v2.json。
- 这不是同期交错A/B、总体可靠率/p99或手机现场。健康抖动、发送队列已堵、one-shot、长请求后半程、其他协议/peer relay以及成本仍待验，F2未晋升canonical。

F1 `33718341...`普通Release producer185850Z-ad21afa7已0/602.427023秒闭、**runtime0**。静态审查发现上游PingTracker每次new_ping替换deadline，仅500ms节流可能被并发提示推迟，故保留编译、不晋升；F2用实际TCP内未完成ping合并，原F1源/收据不改。

## 当前F2的实际回归与控制器失败

- `192504Z-47660213`原想跑组件但root误选net-node，实际标准Node回归 **0/383.521577秒**、21files/identity闭；12组 **278passed/0failed/原ignored1**：net70、whole hop92、wires16、journal30、dial-budget46、IPC18及6租约/取消/恢复合同。run/relay-repair-standard-node-misdispatch-verification-v1.json记录真实suite、原ignored名称和误调度，不代组件/Release性能或冷target来源。
- 首次正确net-vendor `193344Z-033021a3` **1/57.967688秒**、12files/identity闭：Cargo冷编译33.17秒、413artifact全部fresh=false，但控制器误用Noq kind=['lib']，iroh声明lib+cdylib，选择器失败；**组件0/7执行、未准入native**。run/relay-health-component-preparation-failure-v1.json，原件保留。
- 修正选择器后的net-vendor `193936Z-3817ecbf` **0/67.994809秒**、22files/identity/owned闭；冷编译33.52秒、413artifact全部fresh=false，**固定7项原生全过**。新增重复提示不延后deadline/匹配pong才解锁；原PingTracker、cancelled task关闭、reconnect、inactive、backoff优先inbox共5项。control4bc91f797c32fe7da3a98a21c55f430f14e5723c；run/relay-health-component-verification-v1.json，来源为standalone iroh锁域＋同源noq-proto，不代Release业务/手机/全目标。

## Quality、独立GUI修正与Mac Lab

**I2完整Quality43474829073原两轮都12/13**，准确3093f86662b9b9ff139f29f85692e84ec2e47358。仅attachment-ui/local-folder-preview目录setInputFiles12秒超时；原12jobs及其他交互均通过，原log/ZIP/attempt1、固定唯一同源attempt2全留。禁止再原样自动重跑；I2未获完整Quality接受。

隔离harness修正 `87ff588a63fa5c1b5b2eec2aee1c3527c9c77e28` / tree99b8b8ba54e687e66749589d185ee683e5901fdf，在khor-net-attachment-harness-v1、基于最新观察main9a4219ffe5e3ceba1f09cd3288cb0e27f91613d7。仅脚本遵循CI已有PLAYWRIGHT_CHROMIUM_CHANNEL空值选bundled Chromium、本地默认Chrome，并记录实际版本；产品UI/断言/12000ms不改。

- 独立Quality **43490751091首轮success**，verify/net-attachment-channel-v1，suite=gui/interaction_scope=all：frontend/gui/developer-lab三所选jobs成功，**27/27原GUI交互组全过**、**9/9附件case全过**；actual bundled Chromium151.0.7922.34，目录真实字节/选择保留/临时资源退役通过。
- E/attachment-channel-quality-completed-v1.json、attachment-channel-artifact-verification-v1.json及精确ZIP/log；ZIP SHA7d694b2c55e78de4111373400e662098568a062a9d124cde49421ddb1fc2230c/CRC已核。
- 该独立harness树并非F2网络准确源、suite也非all；不覆盖原I2失败或算最终N5。最终集成需带上这份修正，并保留当时main已有成果；当前main含独立暗色图标恢复，不能覆盖别的任务。

**I2原生Mac Lab43475243450全部完成并验源**：可信GH macos-26，request56f540fd11ca4a978c52b7ff474487c4、port23965，I2 treebf4d38d7/snapshot385078c7b1c42444cfbdb44d5e2dda927a03ee23。4jobs首轮成功，77浏览器case/热更5项通过、116源码＋10生成绑定核验，362145952B arm64 host SHAe948c3173555ff15392c6020d8ec923c602e67bfc3b648b857d5ed783987b16b，远端probe real_backend=true。E/public-native-lab-net-product-v2/lab-acquisition-verification-v2.json及outer/inner ZIP保留。本机只收集，不执行产品；E/Open-Network-Lab.command未运行，仍要求最终Quality通过，Lab23961不动。这份包不是后续F2/J2或最终候选；SSH macserver最近仍banner timeout。

## 其他仍适用的有限证据

- I2/R当前混合负载171546Z-dfd9502c **0/713.189051秒**，4320/4320实际请求、6完整文件，原low/high/high/low/low/high与2秒门保持；**high仍有超过1秒scheduled尾延迟**，旧超时未定因/豁免。详[CURRENT-MIXED-RELEASE](CURRENT-MIXED-RELEASE.md)。不是F2最新完整N3接受。
- 当前I2/R IPv6三项163223Z-b6d7f5c7 0/108.146424秒、248实际读成功，含7正1负包校准、同prefix换址/IPv4不变/Node真实阻断IPv4、Both与Remote-first；首有效1.22/1.73秒。四原换网170134Z-6a92a593 0/268.968832秒、496/496读成功，实际IPv4/NAT、双栈prefix、v4→v6-only和middle自身换址/再次carried10秒均核。详[MOBILITY-RELEASE](MOBILITY-RELEASE.md)，不代双端同时变化/手机/包级最优性。
- 核心9原合同分6+3覆盖。174412Z-b676ad99前6原通过、423实际读成功；p4因root外层owned不足取消、后2未执行，原非零/僵尸记录保留，controller退出后87 observed PID全不存在。175639Z-b739a547剩余3原合同0/813.552456秒补齐，p4原600秒＋30秒settle保持；原idle Relay fresh TCP20s/heard22.9s、uplink919ms。详[CORE-PATHS-RELEASE](CORE-PATHS-RELEASE.md)，不能写原9批全绿。
- 窄live-middle R9081b94d原sp7 I1三红→R三绿0.78–1.01秒，Linux/Mac whole hop各92/92；Mac43432815139精确同源/guard证据保留。详[LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)。F2标准Node已再过92，Release物理/真机仍需最终源对齐。
- 旧I1/A Relay可用性132935Z-b055e94a可用1/3、2095ok/2timeout/63blocked；不可用3/3、2160ok，原2秒TTY失败保持。选择器隔离、终端方向6场与持续PTY两场均未重现，不能称根因/修复。local tick正常只排除那两秒全本地计时器停止，不证明远端PTY健康。完整历史与[RELAY-AVAILABILITY](RELAY-AVAILABILITY.md)、[TERMINAL-DIRECTION](TERMINAL-DIRECTION.md)保留。
- W/CUBIC消融不晋升：W固定18场中2timeout/118blocked，C虽功能绿尾延迟不达；Noq CC矩阵19场/1101请求全成功但2%丢包CUBIC/BBR中位0.326/10.067Mbit/s，否决全局CUBIC。Noq backup-burst四组件181344Z-1de568b8冷编译19.06秒、全4过，只组件，不解旧Node超时或补丁1退出。
- tuple patch38退出R/M固定6块12场/744读均ok，但强oracle R3/3、移除0/3并mismatch215，保留于已测NAT合同。详[TUPLE-REFRESH-EXIT](TUPLE-REFRESH-EXIT.md)。验证工具typed/root-dev scope修正旧37＋新9共46绿；原失败保留，native_provenance.py SHA7dbd2c0f4048f8cc96a3c9303565b6298c4999daccdd4fe360825d21d225ac7b，不弱化来源或拷fingerprint。

## 源边界与完整剩余

canonical khor-net/feat/net runtime仍5064fe2a、iroh1.2/Noq1.3，后续文档提交不代表集成。I1=117c39e5，R=9081b94d，I2=3093f866/treebf4d38d7（runtime逐字R、两来源工具修正、apps/.github逐字旧mainc3b098bd）。F2=81ae5c98；J2=a199ff48仅加fixture，产品逐字F2。builder khor-build-net HEAD5d3691f252f9dee0297fa3c494374be6119bca29保持，只新增控制对象/refs。旧A/S/终端/IPv6桥树与完整hash见历史和各证据，不按摘要覆盖原未提交内容。

完整N1路径质量/置信/成本；N2全期限、取消/资源、全部写和0-RTT/NET16；N3真实同步/预览/SSH/hop公平、共享/分连接、背压、1/3/5设备CPU/线程/字节/实际唤醒/手机能耗；N4双端独立与同时IPv4/NAT/IPv6 IID/prefix、锁屏/休眠/Wi-Fi蜂窝、发布/发现/新连接、中继故障域/容量/布局；N5逐项custom、最终同源Quality/Mac/手机/设计报告都未齐。旧idle包+33.970%/字节+37.908%保留。ledger v5的60项=1上游替代已验＋2Noq退出红保留＋1iroh38退出红保留＋**56待审**，不是56均必要。

最新官方registry再次核验2026-11-13T20:47:08Z仍为iroh1.4.1/Noq+proto1.4.0；E/upstream-registry-recheck-20261113T2046Z/verification.json及三个完整原API JSON/SHA保留，最终交付若跨时再核。F2/J2基于这些版本。iPhone仍旧0.2，旧ready窗口不沿用；有可用准确候选后再约真机/能耗。MacLab/Turing同LAN、macserver另机。不能把新Relay两节点case当作现场peer relay/手机修复接受。

## 必须保持的开发约束

本机source-only：Git、stdlib Python源/数据/metadata/ZIP/hash与SSH/GH编排；无本机Cargo/Rust/rustfmt/Node/Bun/Xcode/native产品执行。重型只Turing/macserver/可信GH，khor-build用户已明确授权public；无main合并/发行/安装替换授权。

Turing UID1110、排他flock、jobs16/nice10/Rust1.95、50GiB准入/40GiB停止/90min outer、私有user/net namespace，无sudo/宿主网络修改；计时与编译分离。Mac持久worker保留50/40GiB、target100GiB、jobs2 guard（SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb），不得降门。可信GH托管原生Lab/Quality独立允许。

保护其他任务Lab23961，不能依据历史PID杀进程；先看中央治理STATE再碰共享资源。Code-review-graph先行，未覆盖才定向rg/read；没有.codegraph不自行建。runtime/writes/trust/只用自家Relay/暂停/单跳与原预算不放宽。恢复先核实际worktree/branch/diff和pending，不重复调度；root唯一ledger writer。
