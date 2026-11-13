# feat/net 当前状态与接续入口

更新于2026-11-11 06:58 UTC。完整N0–N5未完成，目标“完成计划”active。用户U14要求持续推进，不以调度面板paused或阶段完成作为停止条件。root串行，不派发子智能体。此前逐字STATE保存在[最近历史](HISTORY-20261111-through-snapshot-and-stability.md)，SHA `974f95530ba857f2d2e9669b91728f7210d2efc7fb1c1aad11c251d108df4a7c`，它继续链接更早历史与失败。收据、进程和实际工作树优先。

## 位置、授权与当前源

产品 `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch `feat/net`。最新生产候选 **2874985e90c1523a6db1b4171a48de085718552d**，tree **2780ffe007956ec655bb4475b1a2c57b1853bb7e**，snapshot **8cdc87e0040c5a813e40c70197ea6d40dcf237c7**。最新已核双OS全12源仍6e5/tree3ac48/snapshot12ef；287仅made成功保留观察的修复，Mac42975178687准确同源全12/hop69已过，最新Linux待跑。后续文档提交另核HEAD/diff。

控制 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，固定 **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**，原两个untracked pycache保留。原N1隔离树 `/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines`，detached2ef6bee5a5aaf841af066f4edb5c894769d4539d，Iroh42–44仍隔离。

证据根 `/Volumes/disk0/Khor/evidence/net`，ops同根/ops，source-refs.json由root单写。完整范围与授权见PLAN、REQUIREMENTS、ACCEPTANCE、NEXT-DESIGN、REPORT、VENDOR-AUDIT。main合并、发布、采购、生产部署及已安装App/config/store/resident修改不由本目标自动授权。本机source-only；重型验证仅远端。

## 当前运行任务：先查同一收据

**Turing turing-20261111T064010Z-1501a97c**，`netlab-hop`，PID **3492772**，child **3492990**，06:40:32 UTC实际launch。准确6e5/tree3ac48/snapshot12ef/control68cad，原1线程/3轮/f4、f4s、f4i前缀。计划candidate-context-original-hop-plan-v1.json，retries0，不改原门/持续时间/拓扑。06:44同一PID存活，首轮进行中。观察超时不重发。

Mac**42975178687**准确287/tree2780/snapshot8cdc/control68cad，已success终态、采集并核实际全12组：hop69、wires16、budget26、resume1。exactartifact13835396432，zipSHA6b34b08ec3f76bfc92613e2018549c204547b68b17aef1718a7e51cef94ed5e0，20files/11controls核。首次identity verifier的GH只读请求EOF，重查同一run后成功；没有重跑产品。当前无Mac待核构建、无phone采样。

成功后原行为Mac42973738072已真实67/2红并收齐，修复287的whole成功fixture37aed942不改。Linux最新287需等Turing原三轮netlab-hop1501a97c终态、collect/空group后提交，使用已冻结candidate-success-miss-candidate-source-v1.json；不要把6e5的Linux绿外推给287。

6e5同源Linux **turing-20261111T063108Z-52af3091** 已327.133秒终态，21files/identity/空group，实际全12组通过：hop67、wires16、budget30、resume1。与Mac42969959757准确相同source/tree/snapshot/control，窄缓存合同现双OS通过，N2整体仍未结束。node-regression-verification-v1.json保存每组实际总结。

## 本轮取得的实际证据

### 候选变化冷却与旧快照

旧test-only92c530/tree281337/snapshot8a568在Mac42966906265实际hop63/2红：真实NO_ROUTE后新增middle、旧ask pending时新增middle均被旧冷却挡住；旧60及3新控制过。exactartifact13830614574，zipSHA2afb0759faa8fe24a09ca8edadca443deb9d01479ea1f7f938ed6b6d9af42a9b，10files/11controls/identity核。

17346把规范化候选ID集合与实际首腿可达bool合入同一MISSED表，表持唯一candidate Arc，操作持弱token；Wires网络token仍弱。真实候选改变使旧冷却/旧完成失效，RTT和排序不改变context。原指数退避30秒至10分钟、TRIES2、HOP_WITHIN3秒、caller期限、信任/暂停/单跳门保持，不增map/task/timer/Node或Connection强引用。42967869816只格式门失败，4b02原样应用远端format.patch；42968221420因三个旧测试直接调用open_hop缺epoch编译E0061，未执行hop。两次均采集保留，不算行为红。

1d448恢复原四参数测试入口并加两个真实旧快照合同，Mac42968959558实际hop66/1红、87.80秒：原65全过；同候选晚登记控制过；旧picked在新middle真实NO_ROUTE完成后才登记，撤回新context并多发一次旧隧道，count(2,1)应为(1,1)。exact13831834082，zipSHA4f07811a07ccb475c6d66ed4e2b54ca7868b5720835dcc17c6e17de95a1d2b2e，10files/11controls核。原五fixture逐字前缀保留，仅追加新module。新candidate_snapshot_tests SHA726e1888b75a8fca33f2543ea0bfcc08830d29696413c8d96a8a08c6c6e27d20。

**6e5修复**在读取roster/roadbook前捕获单调observation与弱网络token，在MISSED锁内拒绝旧登记/旧网快照；两个生产入口均绑定其实际观察。原open_hop为cfg(test)适配，调用真实open_hop_in_epoch，旧测试正文保留，rustdoc指向生产函数。旧快照与候选变化fixture相对1d448原样不变。

**Mac42969959757**准确6e5/tree3ac48/snapshot12ef/control68cad，exact artifact13832933221，zipSHA **54b210ac9fa7861e3fdbe7af9b2455db03053d81ac9152a53f4d202de59bf66e**，20files/11controls/identity核，实际全12组通过：hop67/0、wires16、budget26、resume1，其余原组均0failed。Linux52af3091同源全12已过。fixture/source-audit的candidate-change-miss与candidate-snapshot-miss JSON保留全部源与失败。6e5成功删除context的后续旧快照问题已取得67/2真实红，287修复后Mac69/0、全12通过（见顶部），最新Linux待；同ID换址/LOST context仍未解决，不能称完整N2。

### 原动态与长稳矩阵已收齐

冻结源 **8cd58f8ea707991f23b10ac05d6c8388a61dd519**，tree4c4d7be8de2c615c35de9b5ca6f3db0cce47a78c，snapshot4e900db337c5c4356cb4b27b0de2880210bcf67f，control68cad。它含Iroh47，早于NoRelay/MISSED，不跨源推论。

动态 turing-20261111T045728Z-3d36cba8：1313.385秒、15files/identity/空group，原1线程3轮p2/p3/p7，各9项、共27原green rows。p2切换990.5/937.1/946.7ms，p3最慢532.3/481.3/546.6ms、业务零错。p7闲置原20秒门通过但裕量/成本仍欠。original-dynamic-verification-v1.json已核原task.log逐case表，三scenarios.log各9/0。

长稳 turing-20261111T052125Z-088e00f9：2475.166秒、11files/identity/空group，原1线程1轮p1/p1j/p1s/p5 **4/0**。p1/p1j/p1s各600秒、0切路0退役；p1s20次300msCPU停顿，最慢回显578.7ms。p5原150/300/600ms×4、各30秒间隔，均0Relay切换/0punch，最慢299.1/495.3/948.5ms。original-stability-verification-v1.json保存原行和SHA。原matrix计划已completed；非phone/p99/peer-replug接受。

### Iroh47物理固定四项已收齐

C=8cd、B=2892。原20ms双腿、真实middle IPv4 replug、150秒prepare/10秒holds/124业务、warm与独立Go4秒、3probes/250ms/2秒/25%gain/Moving2秒及v5诊断均保持。两源vendor与双OS全12、8cd原动态/长稳前置已实核。Turing计时独占，独立Mac构建可穿插。

1. **C turing-20261111T060457Z-62030f1b**：178.369秒、16files/identity/空group，124业务全过，warm51/51，双向Go **3569.426/3536.190ms**，原探测3+3，4innercopies相邻成本界完整，8流方向单调且均有postreplug字节进展。不是6aa重现。一处本地分析误把decision reader与selection writer并行，读空JSON失败；原空文件/stderr及analysis-dependency-race-v1.json保留，依赖顺序纠正后只重做数据分析，未重跑产品/改门。正式decision双绿。
2. **B turing-20261111T061130Z-fe19c4ed**：103.987秒、16files/identity/空group，原test29.90秒初始业务连接y超时10.028585秒。尚未进入prepare holds/replug，attempt error1，warm/Go未验证，不能解释成换址恢复红或C胜B的性能证据。是本固定集第一准备失败。
3. **B turing-20261111T061509Z-a43e01ff**：170.363秒、16files/identity/空group，124业务全过；warm51中11失败，双向Go **7499.610/3473.251ms**，一向超过原4秒。8流方向单调并均有postreplug进展，无severe absence。既有analyze-prompt-ack-physical.py只作同合同数据分析，源身份2892原样保留；它的旧输出名prompt-ack也保留，计划统一summary指向confirmation-copy-identity-physical-observation.json。该项非prepare失败，不触发连续两prepare停止。
4. **C turing-20261111T062035Z-817adf28**：220.569秒、16files/identity/空group，124业务全过，warm51/51，双向Go **3820.207/3533.388ms** 均过4秒。8流方向单调并均有postreplug进展，仍非6aa重现。

每项summary为各目录confirmation-copy-identity-physical-observation.json。原whole日志不修补，缺少边界计unknown。原计划与ledger单写。分析依赖必须顺序：collect→identity→calibrate→selection→decision；budget/stream/cost在calibrate后可独立分析。已有ops/analyze-prompt-ack-physical.py按顺序组织，但它会重做calibrate，不覆盖已有分析。

## 必须保留的旧结果与未过项

- NoRelay缺第二腿提示：ff743仅将原Recovery缺wire判断置于NoRelay/ToNoRelay前；旧Mac42963573470为57/3红，候选42964365563同fixture60/0、全12绿。五fixture SHA9629a261389cc6152b7938a6e6d02697121e031432f4c0c00a47accb769cb01b，三no_relay场景和Direct/递归控制。真实内层TLS/echo，middle用Publication适配；实际完整第二腿仍N4，Linux最新同源待验。
- Iroh47代表copy确认：whole395341c56e7c3024505be5a838d8f1fa298e63066d33de1c237a0793b9a02b43。旧41c2/b0146293 path56/2红，候选8cd/c350443Noq/58path/11home绿；8cdLinux f1537e21/Mac42915496687全12绿。只让samples_after与quality/age取同一代表，原quality选择和预算未改。
- Noq13 explicit path PING IMMEDIATE_ACK：旧efb2/8a4a7521实际442/1红，2892/33bc9584为443/55/11及Linuxea5f/Mac42909901676全12绿。原ordinary PING/Initial/真实accepted0RTT控制保留。此前固定四物理C-B/B-C全收齐、496业务成功，却含7.308秒/5.4秒恢复与warm失败；8f3日志末条截断，正式warm/cost unknown。原结果path-probe-prompt-ack-physical-fixed-four-result.json，不由本轮C1覆盖。
- Iroh46 known-quality原五合同旧53/2红、候选1c847全vendor/双OS绿；固定六物理744业务，C1绿2红、B2绿1红。严重6aa双端无Go、quality_steps0、单向/后续无RTT刷新，约21秒carrier结束；outer约41–42ms/open不证明byteflow。根因仍未闭环。known-quality-physical-pairs-through-6.json及流/outer原件保留。
- v4两次物理d111/0e2f在prepare150秒前置失败，不再盲跑第三；v5原883单次124/warm51/Go3435.866/3412.927ms绿，8方向进展，不证明严重stall原因。更早923原三轮2绿1红（b158 5514ms）、Busy854、旧resume32 timeout、CPUquiet/f4s、原cold-start红继续保留。
- 主vendor候选仍57项：Iroh1–41/45–47及Noq1–13；42–44隔离。完整逐项上游/退出审计与Noq1/5专项缺口见VENDOR-AUDIT，不由分项绿替代。

## 下一步与完整范围

1. 固定四项已结束，完整结果confirmation-copy-identity-physical-fixed-four-result.json已核，原C-B/B-C/retries0。保留B1初始失败、B2恢复红，不追加第5次；后续按独立新问题/原矩阵推进。当前先采集Mac42972830373实际合同，再跟踪同源Linux52af3091。
2. 6e5双OS全12已经收齐；Turing1501a97c三轮计时结束并核空group后跑**最新287同源Linux net-node**，冻结source文件candidate-success-miss-candidate-source-v1.json，Mac42975178687已同源全12/hop69。不要在活计时期间穿插Turing构建。
3. N2成功后旧selection两真实合同已67/2红→287的Mac69/0绿，保持fixture不改并补最新Linux。接着覆盖同ID真实新Tunnel/IPv4/IPv6地址变化、LOST本机identity namespace及相关地址context。DeviceInfo.addrs是拨号hint Vec<String>（sync/devices.rs315）；net/endpoint.rs963的dial_addr统一解析IP/Relay并合并nearby，地址相同而来源/顺序不同不应取消冷却。实际live bool不覆盖同wire换地址。
4. N1继续peer-relay4秒稳定性/单向stall、quiet/stalls与成本原矩阵，按证据收敛机制；N0最终体验目标/足够样本p99与完整前后对照未收齐。
5. N3交互/批量共享与分连接、背压公平、1/3/5设备、CPU/字节/唤醒/手机能耗。N4实际两腿Relay、唯一/多middle、断腿，独立v4/v6及同时变址、Wi-Fi↔蜂窝、自动恢复，测完整两腿而非首腿。N5全vendor审计、最终Quality/native/proxy/多设备/phone。完整计划不缩减。

MacLab与Turing同LAN；构建macserver另一台、用户报告异网。iPhone localhost旧现场反复关开App后某次经Turing恢复；5GA截图仅hinton Direct29ms。候选未安装，当前无phone sampler。可信GitHub/TestFlight边界下最终真机验收，旧安装版事故不能冒充新候选通过。原几小时时间窗口已失准，不在接续时重置承诺；完整按天计但结束时点尚不可靠。

## 执行与证据规范

先核worktree/branch/HEAD/diff，不覆盖WIP。图谱先行；本树无.codegraph不建索引，CRG部分覆盖/滞后，0flow/无边不是无影响证据。debug-issue/explore-codebase/review-changes技能此前已读并告知，不必重复宣告。

Turing仅UID1110的/tmp/khor-netlab-feat-net-1110、flock、user/netns、Rust1.95、jobs16/nice10、50GiB起40GiB停及90min门；无sudo/host网络改动。Mac原jobs2/单cache/50–40GiB/100GiB guard，独立构建可与Turing计时并行。SSH BatchMode、ForwardAgent=no、StrictHostKeyChecking，不转发凭据。stage helperSHAff09d4efb50656136d60f5ab58bc190d978923aab8519dc3b49d071208a14c1c，只增已被固定control允许的stability allowlist，原guard不变。

stage-launch只一次，结束exec session后做依赖动作。记录exact收据后查同一PID，不凭旧running文件/观察超时重发。Turing终态控制树scripts/collect-network-ssh.py RECEIPT，再ops/verify-turing-record.py RUN_DIR，之后核原日志合同。Mac exactartifact下载、ZIP SHA、安全解包、11control blobs与每组实际总结核。source-refs单写，stage writer未完成不启动其他ledger写入。依赖分析按顺序，缺失数据不补零，失败原件不删除。Ready bytes=0不是无byteflow；v5累计read/written/last进度与outer绑定分别判断。


2026-11-11 06:32 UTC：固定四轮集合已收齐：两C均warm/Go绿，共248业务成功；B一次初始业务超时、一次124业务但warm11/51失败且一向Go7499.610ms。三个实际replug均8方向有进展、4innercopies、原3+3 probe，完整相邻成本界。reads_finished enclosing aggregate innerTX datagrams依次C328/B312/C337，字节23357/22403/23740，IMMEDIATE_ACK37/39/37；选路驻留阶段与相邻边界不同，不能全归为补丁额外开销/单probe/整网或手机能耗。完整result JSON含原边界和source。两C样本太少且较慢值3820ms贴门，不称可靠性/p99或全N1。

成功后旧快照新fixture candidate_success_tests SHA0b81c4384956ac4d240ec3a65959b4009fa209f2d25e584575979d3da0d87ea0，真实新middle、内层carried TLS/echo/adopt，分别保持consumer活跃或实际close+detach再放旧selection，要求没有旧middle额外请求、本机网络epoch不变。旧candidate_snapshot测试文件逐字前缀保留，仅追加module。candidate-success-miss-fixture-v1.json注册，尚未改made/生产缓存逻辑。元数据准备时源已冻结后因缺Path import停止，只完成后续metadata并匹配实际HEAD/tree，未重建/覆盖source或发生产品运行。


2026-11-11 06:49 UTC：成功后旧selection合同已取得真实旧行为红。Mac42973738072准确0a7/treef68ab/snapshot2980990/control68cad，format与net67+1ignored绿，hop67pass/2FAIL、89.02秒，原67全过；两新合同都先完成真实新carried TLS/echo，再放行旧selection，各额外向旧middle发1次（实际count1,1应0,1）。通路使用中和业务完毕close/detach后均红。exact13834782095、zipSHA1b8032d952913474e51e09e4aeeb923b041bdff43ff46b3e373cdb1e36093bf3，10files/11controls/identity核，后续组未跑。

生产2874985e只改made_in_epoch：成功count归零，保留最新observation/candidates，在原entry内替换Arc<()> token使旧完成失效；None纯backoff helper仍删除。原退避/预算/信任/所有权边界保留，不新建map/task/timer/Node或Connection强引用。零count路由元数据在活网络保留，network弱token过期时由下一次miss_context登记回收。whole成功fixture SHA37aed9428b2bafa5097f651ce4c1ca6124af111477b2a6e8435bc6b3fa9b8161不改。candidate-success-miss-source-audit-v1.json记录窄源差与寿命。Mac提交执行会话73036待实际收据，先查同一会话，不重发；此时尚未核候选结果，Linux最新287也待Turing计时矩阵结束后调度。

Turing1501a97c同一PID3492772在06:49实核已运行8分32秒，首轮f4原green（回carried3.8秒、middle leg听到后0.3秒、最慢echo3.4秒）；f4i原green但闲置后恢复17.7秒，仍是体验长尾，不能由现有断言绿称最佳设计。首轮f4s仍在跑。只监控同一任务，完整三轮与原失败都保留。

Mac archive采集新增ops/collect-mac-archive-v1.py，SHA7caaea2bb16610072d79e84dbf7129cc9fd6a17736b1b75e5cb1080c206f3d19，只读ledger/GH，核exactartifact/run/control与API.digest，拒绝覆盖、路径逃逸与symlink，安全解包；从不执行归档内容，也不分类产品行为。已用于42973738072精确归档，archive-verification.json保留脚本SHA。之后仍必须verify-mac-record与实际日志判合同。


2026-11-11 06:55 UTC 原f4i指标语义补核：crates/cli/tests/netlab/hop.rs799先让两端let_go、切断x→relay及x→middle75秒；先恢复relay、visit并等到Relay连接，之后才恢复middle并启动back=until(Forwarded)。因此报告17.7秒是middle重新可达后回到peer relay的路径收敛时间，不能当作首个业务响应/整个请求断连时长。先前“闲置重连17.7秒”的口径需按此限定。其assert主验first road started而非move语义，120秒回peer仅可达门，不是最终体验SLO。后续分别测首个有效响应与路径收敛，旧case门不改。


2026-11-11 06:58 UTC：287候选Mac42975178687已实际全12绿/hop69，旧67与成功使用中/正常释放后两合同全过。wholefixture37aed942保持。candidate-success-miss-fixture-v1.json状态mac_behavior_verified_linux_pending，source-refs已终态分类；最新287 Linux在Turing计时后跑。身份校验首次只读GH请求EOF，原artifact已精确hash保存，第二次同run校验成功，verification-transport-note.json说明，没有产品重跑。

原netlab-hop1501a97c同一PID3492772在06:57实核running、17分25秒。前两轮各3/0（479.50、491.36秒），第三轮f4刚开始。第二轮f4i回peer31.6秒（已先在公共Relay可用），路径收敛长尾保留；第二轮f4s Y没有记录切出/切回，case仍green、最长echo9.4秒，X见relay6.2s→carried9.2s。此原oracle只验证其特定路径/日志语义，不能把never/0.0s当作实际瞬时恢复。后续按N0分解真实故障持续、首响应、fallback/收敛和指标覆盖，而非由三个green概括极致体验。完整三轮源6e5，非287。
