# feat/net 当前状态与接续入口

更新于 2026-11-13T18:42:54.665924+00:00。完整目标 [GOAL](GOAL.md) / [PLAN](PLAN.md) N0–N5 **未完成**；逐项缺口见 [REQUIREMENTS](REQUIREMENTS.md)。root 串行，不派子智能体。用户已授权持续实施与远端验证，不因阶段完成停工。

E=`/Volumes/disk0/Khor/evidence/net`；工作树均在 `/Volumes/disk0/Khor/worktrees/current/`。此前 STATE 逐字存 [HISTORY-through-terminal-direction-launch](HISTORY-20261113-through-terminal-direction-launch.md)，SHA256 `b6712ae6174fb1be910c4f43f4b11424d6be545645358fc2a7d50c63c4eded47`；其中更早历史链接保留。历史 pending/PID 不能用来重新派发。

## 当前执行与紧接工作

**修复v2普通Release producer191019Z-979f0ccd已0/594.656134秒闭证**，22files/identity/strict native完整，Rust9分02秒。准确source81ae5c9812b06298a06549f1c16a3728baac2c8d/treeba3da3451c027904b657183bc26d990191d75c30；CLI ef9a3dd1061ab557c4aa1709b9a14901813efff2e91251bbdef5e415f8c49519、test2296f0f2b87910dee724f65b9b52957e7e255bf9e0d9a29ea24fbb63f1816f4b，实际fresh=false。原fixture逐字不变，旧selected5归档与ed1基线树重建已核。run/relay-silent-demand-repair-native-compile-verification-v2.json。原生compile不代业务/组件通过。

**修复v2原3场真实前台计时192107Z-941d6b67已0/166.445581秒闭证**，15files/identity/native/CPU/owned均完整；原2秒caller/250ms计划间隔、3固定重复、原fixture全部字节不变，无替代/运行编译，**126/126成功、零超时**。首个有效响应960623/958965/906570μs，旧源约20.04秒、30timeout均保留。control5ee17e95bb98278e1fd6f65e846b8896a788e96a、协议SHA2f07687a98b22247826a0ffd37f0632d6bf5de41dfb5e2655346821935c7ecdc，run/relay-silent-demand-repair-runtime-verification-v2.json。仅该窄几何3红→3绿，不代可靠率/同步A/B/完整N4/手机。详[RELAY-DEMAND](RELAY-DEMAND.md)。root唯一ledger writer。

**入口误选的标准Node回归192504Z-47660213已0/383.521577秒闭证**，21files/identity完整；实际12组 **278passed/0failed/原ignored1**，包含net70、whole hop92、wires16、journal30、dial-budget46、IPC18和6个真实租约/取消/恢复合同。原ignored名称保留，按实际标准debug Cargo回归接受这份有限证据，不冒充七组件/Release性能/冷target来源。run/relay-repair-standard-node-misdispatch-verification-v1.json；调度错误与原receipt不改。

**七组件第一次正确入口193344Z-033021a3因产物选择器错误失败，已1/57.967688秒闭证**，12files/identity完整。Cargo冷编译33.17秒成功、413artifact全部fresh=false，但控制器沿用Noq的kind=['lib']，iroh实际声明lib+cdylib，选中数量断言失败；组件0/7执行、无原生准入/业务结论，原日志全留，run/relay-health-component-preparation-failure-v1.json。

**修正后七组件193936Z-3817ecbf已0/67.994809秒闭证，Turing当前无pending。** 22files/identity/owned完整，独立空target冷编译33.52秒、413artifact全部fresh=false。原固定7项全部native通过，包括新增deadline/pong合同和5原actor合同；control4bc91f797c32fe7da3a98a21c55f430f14e5723c，run/relay-health-component-verification-v1.json。仅standalone iroh锁域＋同源noq-proto组件证据，不代Release业务、成本、手机或全目标。首次误调度Node278通过与第一次组件选择器失败分别完整保留。

**接着做健康Relay延迟突增回归**，E/relay-health-jitter-design-v1.json已提前冻结：同strict NAT/单Relay、只在私有Relay出口对alpha把delay124ms改976ms，无丢包/reset/换址；3固定重复、每场2setup+12新marker实读，post caller5秒，100ms实际TCPtuple采样＋2秒尾观察，实际tc/filter/qdisc/stat readback与无drop。42预期实际调用；原c-demand/2秒不改。独立fixture工作树khor-net-relay-health-jitter-v1 / experiment/net-relay-health-jitter-v1已clean7fe8b4770bdb148a182ba1543deeb10bea31d8bc/tree6479ec7ead27b28b4b45261c1bd405ceaad743a3、snapshot6d30628b49a1d310337101a8adf119bc2bebdaee，仅2测试文件297行，产品逐字F2/v2。**普通Release compile已唯一发起**，revision relay_healthy_jitter_release_compile_v1/control348f5e971383eb96b11be6ea973d517ceace8080；恢复先核source-refs/receipt，不重派。独立私有ns先校准tc序列化/过滤器/drops字段，再编译，产品执行0/fixture预校准1；CLI预期保持ef9a。E/relay-health-jitter-source-v1.json/native-plan-v1.json，完成后collect→identity→ops/verify-relay-health-jitter-native-compile-v1.py；尚无实际healthy-jitter runtime，不能说已重现误断。该问题来自既有patch15记载的健康TUN RTT波动，需实证再定探测参数；窄20s→约1s不代表一般安全、手机或N0–N5完成。

修复v1 producer185850Z-ad21afa7 **0/602.427023秒**，22files/identity/strict native闭，Rust9分11秒，CLI8743022736512a69f9cb5aeb46abbdfef76c05177da0356aaccc64856c1b9081、test2bc7af0ced4badb2f44e4c935853d91ef9a87fbc33581597436be0fedfad251a。保留为未运行候选：审查发现iroh-relay1.4.1 PingTracker每次new_ping替换deadline，v1仅500ms节流在并发提示下可能反复延长等待，所以**v1 runtime未派发，不准入**。v2改TCP连接内单个未完成健康ping，原生产源/记录不重写。

**真实前台Relay静默故障已3/3重现产品红。** 184914Z-5cfe53b4 **1/198.559489秒**，15files/identity/native/CPU/owned闭，126次实际读 **96ok/30timeout**；每场40故障后读10timeout/30ok，首个有效响应20042371/20040904/20049739μs。原2秒caller/250ms计划间隔、3固定重复/全部新marker/真实nft旧tuple双向drop/原无Direct或peer替代均核；不是执行器或清理失败。run/relay-silent-demand-runtime-verification-v1.json protocol_verified=true。下一步隔离候选修复前台无ACK时的Relay健康验证，复用原3场与原门，再核慢健康路/Accepted/取消/后台成本。原15秒常规ping+最多5秒pong等待与20秒相符，但修复因果仍需同夹具对照证明。详[RELAY-DEMAND](RELAY-DEMAND.md)。

原核心路径9项已分6+3补齐：174412Z-b676ad99前6原native通过、423/423实际读成功；第7因控制器owned不足中断、后2未执行，原非零与清理记录保留，不重写成原9项全绿。独立175639Z-b739a547剩余3项 **0/813.552456秒**、15files/identity/native/CPU/owned均闭。p4保留600秒观察、30秒settle，仅外层owned1200秒；两端重复地址没有诱发network change，alpha0punch/0selection、2797包/365783B。原Relay静默c fresh TCP约20.0s、再次heard22.9s，原30s门通过但没有前台业务，不能推成23s用户中断；uplink约919.0ms恢复、wire未退役。E/run/current-core-remainder-block-verification-v1.json；详[CORE-PATHS-RELEASE](CORE-PATHS-RELEASE.md)。

**下一网络步骤：真实前台Relay静默故障。** 独立khor-net-relay-demand-v1，clean62b28e084a8cf72593a9ed1ba270327d281f9d49/treeb9d5ac12e755ddd7696a4da8ab3bc70fa6d08dd6、snapshot29868ae901e11182c7d16e7e097faa9d58a31d7f。仅新增测试2文件162行，产品runtime逐字I2、原idle c不改。E/relay-silent-demand-design-v1.json提前冻结3独立重复、每次2setup+40真实新marker目录读、caller2秒/250ms计划间隔，首次有效响应≤2秒，全部126实际结果保留。先排他guard复用库缓存编译普通Release并严格核来源，再独立计时，不把空闲journal恢复代业务体验。原生producer184400Z-8e2c478a已0/234.817081秒闭22files/identity/strict native；Rust编译3分03秒。CLI实际fresh=false重新编译但字节精确同7e8；test SHA5f607b385e9e6dfd96af183bd33adb265247dca699b6f7b8ba7ea3b9f6c80634。run/relay-silent-demand-native-compile-verification-v1.json；保留旧selected5、重建90a1基线树，产品逐字不变。上述真实计时已闭，3/3产品红见顶部；不再重派旧源。

原四项换网170134Z-6a92a593 **0/268.968832秒、496/496读成功**，16files闭；实际IPv4/NAT、双栈前缀、IPv4→IPv6-only及middle自身IPv4变化已核，middle再次carried持续10秒。详[MOBILITY-RELEASE](MOBILITY-RELEASE.md)，不代包级最优性证明。

**当前R/I2原混合负载 `171546Z-dfd9502c` 0/713.189051秒**已闭18files、identity/native/CPU/owned。固定low/high/high/low/low/high、**4320/4320 ok、6完整文件**，保留原2秒请求、原profile与原场景，运行编译/替代0；control b320fcf93b435f47717a9ed49e8ed1dde1333e1c，协议SHA8124e47150e4ce62a56d60d05c6635b182cef7bc7714165c98be24e82c0f209e。E/current-r-mixed-summary-v1.json和run下current-r-mixed-block-verification-v1.json，完整数据见[CURRENT-MIXED-RELEASE](CURRENT-MIXED-RELEASE.md)。high仍有超过1秒的scheduled尾延迟；不以功能全绿认定体验完成，不覆盖原I1/A超时或声称已定位修复。

**同源I2 Quality43474829073首轮12/13通过，GUI一项目录上传超时。** 源3093f86662b9b9ff139f29f85692e84ec2e47358/treebf4d38d73fbb8eda558d08446deff2d930b65304、verify/net-product-v2。GUI仅attachment-ui/local-folder-preview的setInputFiles目录操作12秒超时，其他交互组及native等12jobs通过；E/net-product-quality-v2-attempt1.json、quality-v2-gui-attempt1.log/.zip保留。静态看到attachment harness硬编码Chrome而CI指定bundled Chromium，尚非定因。**固定唯一同源复验也在相同目录setInputFiles处12秒超时**，attempt2 job132091475836、12其他jobs通过；E/net-product-quality-v2-attempt2.json、quality-v2-gui-attempt2.log/.zip，ZIP SHA b3ba8981141b40f1209369ddfb64c61276dd76e6f45f52da2cf82078e155dfcb已独立核CRC。两轮原始失败均保留，禁止进一步原样自动重跑。需诊断硬编码Chrome与CI bundled Chromium不一致等原因，不能仅凭静态差异定因；最终Quality未接受，Lab入口仍未运行。

**附件浏览器脚本隔离验证43490751091已唯一发起**，repo colehank/khor、verify/net-attachment-channel-v1、准确87ff588a63fa5c1b5b2eec2aee1c3527c9c77e28/tree99b8b8ba54e687e66749589d185ee683e5901fdf，suite=gui/interaction_scope=all。独立树khor-net-attachment-harness-v1基于实际最新main9a4219ffe5e3ceba1f09cd3288cb0e27f91613d7，仅3行增1行删，遵循相邻gates已有PLAYWRIGHT_CHROMIUM_CHANNEL空值选bundled Chromium、本地默认Chrome，并记录browser.version；原产品UI/断言/12000ms不改。E/attachment-browser-channel-diagnosis-v1.json、intent与submission-v1.json。**首轮成功，27/27原GUI交互组全过**，frontend/gui/developer-lab三个所选jobs均success、其余按suite跳过；actual bundled Chromium151.0.7922.34，9/9附件case全过，目录真实字节/选择保留/临时资源退役通过。E/attachment-channel-quality-completed-v1.json与attachment-channel-artifact-verification-v1.json、原ZIP/log，ZIP SHA7d694b2c55e78de4111373400e662098568a062a9d124cde49421ddb1fc2230c已核CRC。该树基于新main9a42，只验证独立harness，不能把它当当前网络修复准确源或full Quality；两轮I2红不覆盖。新main已包含独立暗色图标恢复，后续最终产品集成需按当时main复核，不能覆盖别的任务成果。

**同源Mac Lab43475243450全部完成并独立验源。** 可信GitHub托管macos-26、request56f540fd11ca4a978c52b7ff474487c4、port23965；I2 treebf4d38d7/snapshot385078c7b1c42444cfbdb44d5e2dda927a03ee23。4jobs首轮成功，77浏览器case通过、source hot5/5，116源码与10生成绑定核验；362145952B arm64原生host SHAe948c3173555ff15392c6020d8ec923c602e67bfc3b648b857d5ed783987b16b、远端probe real_backend=true。E/public-native-lab-net-product-v2/lab-acquisition-verification-v2.json及精确outer/inner ZIP保留。仅源/数据收集，本机未启动产品，收集session95523已退出。E/Open-Network-Lab.command已准备但未运行，要求最终Quality通过后才放行，Lab23961不动；不是正式发行或安装替换。macserver SSH只读重试仍握手超时。

**Noq backup-burst四组件测试已完成。** Q clean a954fd8531ebe5eb0ec9daae118cef6b1bcc3c48/treeb32e6dcdb4ac62ee98172779eb5f1b578dd6a991，仅2test文件177行，生产源不改。181344Z-1de568b8 **0/49.929059秒**，19files/identity/owned闭，空独立target原生冷编译19.06秒、146artifact均fresh=false。CUBIC/BBR3×quiet/probed四项均通过，warmup及实际backup STREAM双向burst完整drain均650ms；probed含drain实际13秒、quiet10秒，不作纯探测因果比较。run/noq-backup-burst-verification-v1.json。control v2 fa3812fa6c66327a0d6aa0751a7d6b50e8ca7b02实际派发；v1未stage，v2显式使用已核CondaPython支持tomllib。仅组件诊断，原Node TTY超时未定因、Noq补丁1未退出。

**同源完整netlab producer 165200Z-a2a62249 0/234.868713s**已闭21files/identity/native来源，CLI实际fresh=true、精确SHA **7e8c52bc4d21a3471e3c0592ad1a874147a7d024c067bda23a7f4d08213b34df**；完整test SHA **e4f2f52c7c1ff7f17b5e970c20c1aab5d40d5620dd118fc8491631ee8fa84913**。E/run/current-full-netlab-native-compile-verification-v1.json。未改source/fixture，仅编译已有--test netlab；同test-root dev依赖scope重用旧CLI证明，未独立apply bin准备、未touch main，严格validator不变。这次完整fixture重用核心库与CLI，不需每种场景改source再重建。E/ops/fix-same-source-full-netlab-prepare-v2.py记录发起前修正；v1草案未stage。闭证协调器PID51522已完成退出，state E/full-netlab-native-close-coordinator-v1.json phase=completed、sole_ledger_writer=false；无运行中的本地协调器/另一个远端任务。

**当前I2 IPv6原3项 163223Z-b6d7f5c7 0/108.146424s**已闭15files/identity/native来源/CPU/owned，原7正＋1负包校准、Node Both/Remote-first各124/124，合248读（含4setup）全部成功。实际IPv6同prefix替换/IPv4快照不变、Node两侧eth0 IPv4真实nft阻断、旧新包检查、远端先发顺序及原30/120/240/300s均核。换址首次Both a→b/b→a **258230/1217614us**；Remote-first b→a **1732189us**，随后a→b13665us。各只有1次，不作SLO/分位数/完整N4接受。E/run/current-ipv6-mobility-block-verification-v1.json、[MOBILITY-RELEASE](MOBILITY-RELEASE.md)。原native producer161712Z-1ed9c2c3 0/550.152129s/Rust8m20s/23files闭，IPv6小test fc36e3fe、CLI同7e8。测试树clean a5de56858c7ad727fd6d5f6bfe4646e1bf3295d0，两个bridge35行，原mobile/mobility/lab/attempts逐字未改。



持续会话 **154840Z-ed908e42 0/1000.028494s**已闭14files/identity/native来源/CPU/全部owned清理，**4800/4800 ok**，原shell/capture各同一真实PTY持续10窗口，各2400ok，无bulk；capture2404条连续无丢前缀、输入/回复各1201、epoch bracket120ns，替代/运行编译0。E/terminal-persistence-comparison-verification-v1.json与run下terminal-persistence-block-verification-v1.json。TTY scheduled p95原/capture **303409/303297us**；目录 **305919/305500us**；仅描述，不重现旧超时、不证明修复或bulk唯一因果。测试树clean **04c14e88f599ea522efabe84fd23c035104f7fd1** / tree3f6cb2c3bedfd350f06315cc7cb8a70d742c008f；producer154317Z-37b038ce 0/234.732497s、22files闭，testSHAbb4447a85abe251bac4fb9ce8e269c6f3b381c60a114897548f1e6df8fdc4fe0、CLI a7原样。7纯元数据检查绿、helper原4远端检查重用。

IPv6构建前两个控制器失败已分别闭证并保留：**160635Z-a6060fe0 1/25.780486s**、12files，目标名全局替换误改了预期删除的旧fixture路径，实际source未变；**161147Z-6ff4e63d 1/25.806716s**、14files，脚本误查parentless包中不携带的开发基线commit。两次都**Cargo编译0/业务0，无新原生准入**。第二次cache已checkout新tree90a1，但旧producer selected5字节仍精确不变，E/run/source-native-checkpoint-v1.json。当前v3在排他guard内先恢复原可信producer源、严格验证其原生bytes，再重新切换新源；从SOURCE_TREE的私有Git index仅删两个Release bridge，重建基线tree **bf4d38d73fbb8eda558d08446deff2d930b65304**，必须精确同hash，已本地Git-only验证。原native validator/所有产品源/门不变，不能再次引用未导出的开发历史。失败JSON及E/ops/fix-ipv6-parentless-baseline-control-v3.py保留准确接续。

 终端方向固定6场 **151349Z-c16a9107 0/1125.157539s** 已闭18files/identity/native来源/CPU/全部owned清理。原shell/capture各3/3、各2160ok、六完整192MiB文件；三个capture各724条完整前缀、输入/回复各361、epoch bracket130ns、无丢记录。替代0、运行编译0。原/capture文件中位22.720202/22.759749Mbit/s只作描述。**未重现原超时，未定位根因，不是修复**。E/terminal-direction-comparison-verification-v1.json、run下terminal-direction-block-verification-v1.json。

已提交测试树 khor-net-terminal-direction-v1 / experiment/net-terminal-direction-v1，clean **ac261817d4d4e897acf2614a286f6e4ead78621c**、tree **776ecc6b2bf358bb9421dbbd5c988cb4f1a52a7b**，snapshotf2131be0；产品CLI精确原a7e3565f。producer145811Z-6f3704a7 0/544.370115s，22files闭、testSHA499340a0655fc827e5e792af7c364ade14188560901a1a8aa332975c05462954。原descriptor提交前basef6b9/dirty=true不重写。control0f50c7bcaa76a37897431f4dc0832ce271579f5a，协议E/terminal-direction-runtime-plan-v1.json SHAa6893aaa724d3f121c3c9f316ccdfbaf7b1808b9e99a5ea37f25f5475c7330ef。

新增纯数据复核 **E/red-terminal-locality-verification-v1.json**：旧Relay可用组case4/5两次TTY超时，2秒窗口内各有100个20ms本地probe；最大tick lag1489/1711us、邻近最大gap20573/20792us。同时Main目录请求仍约303–305ms成功。只排除“这两秒整个本地fixture计时器停了/设备对全部业务都断”的解释，不证明远端PTY或Tunnel健康。scheduled lag包括等待前面串行业务，不是OS调度延迟。

[TERMINAL-DIRECTION](TERMINAL-DIRECTION.md)保留解释边界：at_ns在I/O后采，是完成上界；只有早于观测超时下界的reply_written证明程序已写出；晚stamp不能证明晚到/晚写。summary只验证第一summary前连续无丢前缀，保留其后原行；缺input最多证明程序截至summary未处理完整行，不推成包未到内核。要求同time namespace、epoch bracket≤1ms、无读错/溢出。helper远端4绿、元数据合成检查6绿，不代业务修复。

当前IPv6任务设计：E/current-ipv6-mobility-design-v2.json，复用原packet校准1项与Node IPv6 overlap两项，固定3场/248预期真实目录读（含每个Node场景2个setup读）。准确I2 runtime不变（含R窄修复），独立树khor-net-current-ipv6-mobility-v1 / experiment/net-current-ipv6-mobility-v1，clean a5de5685；只加Release入口/bridge，原mobile.rs、mobility.rs及helpers不改。tree90a1d7813377d611ea93b3276a36d3876e28e09c / snapshotb6cb41a3adcc51abb175d8c92fcc008d7341eedc；当前准确control/plan/verifier/run见本页顶部，先核实际source-refs，不重派；计时与编译分离。原30秒读/120秒setup/240秒scenario/300秒owned保持，不代UX SLO、双栈fallback、前缀变化/双端/peer-relay/真机验收。新R Release CLI实际SHA须由producer核出，不沿用A a7；该版验证器已明确绑定I2 hop源而非要求旧CLI。

## 产品源与当前接受边界

| 源 | 当前事实 |
| --- | --- |
| canonical `khor-net` / feat/net | runtime 仍5064fe2a、iroh1.2/Noq1.3；本树后续文档提交不代表产品集成 |
| I1 `khor-net-product-integration-v1` | clean117c39e5432741198c924177fae1775ff292eada，iroh1.4.1/Noq1.4.0；N3原红，未晋升 |
| R `khor-net-live-middle-retry-v1` | clean9081b94dce827cbd3fb9b9e7bb13633d762e3cd9 / treecf9d172bb92778ccfd97899ddea27bdf0e6d2f6e；窄 live-middle 修复，不含W |
| I2 `khor-net-product-integration-v2` | clean3093f86662b9b9ff139f29f85692e84ec2e47358 / treebf4d38d73fbb8eda558d08446deff2d930b65304；runtime逐字R，另两份验证工具 scope 修正；apps/.github逐字冻结main c3b098bd，未最终准入 |
| A `khor-net-relay-availability-v1` | cleanf6b9a75633485b8ab29f187479506f186ba88618 / tree953fd88f5f39fa99bc726296e5a84559996ce5cc；runtime逐字I1，用于当前诊断 |
| S `khor-net-builtin-selector-v1` | clean5e712671e53620c14f6ac8931da64f677b557fc1 / treececa08abc132f807e74fe0b463a231b26d942645；只删RoadOrder注册，只诊断 |
| builder `khor-build-net` | HEAD5d3691f252f9dee0297fa3c494374be6119bca29未移动，只新增控制对象/refs |

## 最近已闭证据

- **Relay 可用性** 132935Z-b055e94a 1/1161.211608s，18files/identity/native/CPU/owned闭。原同配置固定6场：可用1/3、2095ok/**2timeout/63blocked**；不可用3/3、2160ok，六文件完整。两次原2秒TTY超时在 idle_after/index82 与 idle_before/index93，不仅发生于 bulk。bulk TTY scheduled p95 5.874877/.336874秒；完整 [RELAY-AVAILABILITY](RELAY-AVAILABILITY.md)、E/relay-availability-comparison-verification-v1.json。不能据此建议产品禁用Relay。
- **选择器隔离** 固定 RoadOrder/内建/内建/RoadOrder/RoadOrder/内建六块已闭，E/selector-comparison-verification-v1.json。各3/3、2160ok、共6完整文件，运行编译/替代0；同期 RoadOrder **未重现**原超时，预冻 diagnostic screen=false。bulk目录 p95 584911/409883us、TTY441411/316267us；文件中位23.548850/23.542792Mbit/s。内建仍含vendor补丁且默认Relay=backup，违背速度优先全合同，不晋升。producer135643Z-73d37a5b 0/552.550232s；六runtime run见原验证JSON，不重派。
- **窄 live-middle 恢复** 原I1 sp7真实3/3红保留；R070832Z-6b87e77c固定3在原5秒内恢复，1013894/784788/878995us，真实Direct首腿+Relay第二腿、held hint、同PID/marker/owner均核。有限Ask/Forced按原3秒复查，TRIES2/trust/单跳/写不重放保持。[LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)。Linux92/92；同源Mac **43432815139 92/0/0、105.52s**，14files/CRC/source/control/raw/50-40GiB guard已闭，E/public-mac-live-middle-whole-hop-v1/verification.json。只接受该窄范围。
- **Tuple patch38 退出** R M M R R M六块/12场、744读均ok、180暖开流。强属性R3/3且mismatch0；删补丁M0/3且mismatch215，保留patch38用于已测当前NAT合同。[TUPLE-REFRESH-EXIT](TUPLE-REFRESH-EXIT.md)。不覆盖全部IPv6/手机。
- **N3/W 与 CUBIC** [COPY-WAKE-RELEASE](COPY-WAKE-RELEASE.md)固定18场/12960记录：B6/6，C6/6但目录尾延迟越界，W5/6含2timeout/118blocked；均不晋升。CUBIC真实Node固定3功能2160ok但原门2/3（首轮direct_after=false）；当前Noq固定19场/1101请求均ok，2%丢包CUBIC/BBR吞吐中位.325742/10.067335Mbit/s，否决全局CUBIC。E/cc-matrix-comparison-verification-v1.json。模型绿不是原Node超时重现。
- **验证工具** Cargo dev依赖不传播，typed/legacy ID一致性检查保留；旧37+新增9回归原3fail/2error→修正后46/0/0，真实Release亦准入。native_provenance.py SHA7dbd2c0f4048f8cc96a3c9303565b6298c4999daccdd4fe360825d21d225ac7b。明确是修正后的工具；fixture变更仍须显式 prepare CLI，不能拷贝fingerprint或弱化来源。原未准入/格式/前置失败保留，细节见历史。

E/relay-selector-static-audit-v2.json、E/terminal-direction-flow-audit-v1.json记录静态事实：host Welcome后本来是raw PTY；p.rtt已有传输估计回退；Main等FIN而Tunnel双工，不能只改Proto共享；低multipath配置会被忽略；W实际发path.ping补探测，不是通用驱动wake；程序stdout写出不等于网络收到。避免重复错误假设。

## 全目标剩余与约束

N1完整路径/置信/成本；N2全期限、取消/资源、写与0RTT不重放、全部NET16；N3真实同步/预览/SSH/hop公平、背压取消、共享/分连接比较、1/3/5设备CPU/线程/字节/实际唤醒与手机能耗；N4独立及同时IPv4/NAT/IPv6 IID/前缀、双端/middle、WiFi↔蜂窝/锁屏/休眠/发布发现、中继故障域/容量/布局；N5逐项custom、最终同源Quality/Mac/手机、设计与报告均未齐。旧p1h包+33.970%/字节+37.908%保留。ledger v5的60项=1上游替代已验、2Noq移除红保留、1iroh38退出红保留、**56待审**，不是56均证明必要。

最新官方registry核验2026-11-13T10:15:56Z仍iroh1.4.1/Noq+proto1.4.0，E/upstream-registry-recheck-20261113T101556Z/verification.json；最终交付前再核。[MOBILE-ADDRESS-AUDIT](MOBILE-ADDRESS-AUDIT.md)确认iOS AF_ROUTE独立通知确在，App表面去重不证明漏地址。手机仍旧0.2，新候选真机/能耗未接受；有可用候选再约新窗口。MacLab/Turing同LAN，macserver另机；旧I1 Quality13/13和MacLab同源包不代最终I2验收。

本机source-only：仅Git、stdlib Python源/数据/metadata/ZIP/hash与SSH/GH编排；无本机Cargo/Rust/rustfmt/Node/Bun/Xcode/native产品执行。重型只Turing/macserver/可信GH；khor-build已用户授权public。Turing UID1110、排他flock、jobs16/nice10/Rust1.95、50GiB准入/40GiB停止/90min outer、私有user/net namespace，无sudo/宿主网络修改。Mac guard50/40GiB、target100GiB，SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb，jobs2，不能降门槛。保护其他任务Lab23961；需操作时先读中央治理STATE核实际源/PID，不信历史映射。

无main合并、正式发布/标签/TestFlight、已安装App替换或生产Relay采购/部署授权。使用CRG先探索；没有.codegraph就不建。保持全部原红、取消、无效前置及原意图，不能无变化重复直到选绿。所有原始源码、receipt、log、selected原生归档保持；snapshot envelope的 `source` subtree 才是实际源，不把根envelope当代码。


## 健康jitter夹具预检接续（2026-11-14T04:05）

195501Z-d304e9a5 **1/25.804405秒**已闭14files/identity：tc filter的JSON输出在当前工具上无效，内核预校准解析失败，**Cargo0/产品0/healthy场景0**；原日志与run/relay-health-jitter-preflight-failure-v1.json保留。stable R source已checkout旧fixture tree6479，但selected5原生字节未动。夹具a199ff480e7c817c6a4bcbb94e3f349e85a4e23f/tree3b4a982616787852a5a9561b799f5656dd74fd3a仅将filter readback改原始文本，qdisc JSON/实际统计保留；产品逐字F2，原SLO/样本不改。

**唯一已发起修正producer** revision relay_healthy_jitter_release_compile_v2、controla8b51e24bd03e557932850332f7a3c1f873e1c25、source-v2 snapshotcc72a2d363f4a26f68b7e25a96504b3bbb2b921d；查source-refs最新receipt，禁止重复。先在排他guard核前次Cargo未开始/selected5未动，恢复已准入F2 source后严格复查旧native，再切新源；父less树删jitter child＋恢复relay父文件必须重建F2 treeba3da345。kernel预校准读filter原文/flowid、qdisc/drops字段后再编译；CLI预期仍ef9a，产品执行0。完成collect→identity→ops/verify-relay-health-jitter-native-compile-v2.py，再冻结原3场42调用runtime。当前还没有healthy-jitter产品失败或通过证据。
