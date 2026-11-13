# feat/net 当前状态与恢复入口

更新：2026-11-09 21:52 UTC（本地2026-11-10）。root串行，无子智能体。完整目标仍是“完成计划”N0–N5，未完成；用户明确“继续”并补充U11–U13，当前工作已恢复。用户再次指出不应停止；goal服务的paused只是后台调度状态，不构成本轮停止依据。必须持续推进完整计划，不能在阶段通过后自行停下，也不得伪造complete/blocked或另建目标。速度/体验优先、个人可信设备网；没有合main、发布、中继采购/生产部署授权。

## 当前源码与已收齐证据

- **243 Mac最新outer-wire准确源便携预检全绿**：42705855560，与242/244完整source相同d1a7a401/tree0bb908a7/snapshot6833a423/controle4c58dc；fmt/net67+1ignored/hop12/wires11/journal30/budget20（49.88s）全部绿，完整source/transport/9 control blobs/hash核。不代表CLI物理病例原生可跑。
- **新独立NAT lifecycle控制profile已通过8项stub合同并提交**：control fc204ac（全SHA以Git/receipt为准），普通netlab-nat-rebind保留；新netlab-nat-rebind-lifecycle明确samples off，r0失败/无成功证明就不启r1，r1或parser失败保留原错误。Python cargo/rustc stubs＋小Git验证，本机没有产品编译/测试。244仍pin旧e4，不会受控制更新影响。下一Mac先验证新增request_stream_opened与整个便携合同，再Turing独立诊断NAT慢方向。

- **242 NAT-only真实校准+业务2/2可达绿，最好路不绿**：turing-20261109T214614Z-f70be96f，base d1a7a401/tree0bb908a7/snapshot6833a423/controle4c58dc；138.23s/rc0/原group空。r0同local tuple/同WAN:50001→:50002、kernel两族不变实际1/1；r1全部124/124ok，mapping82ms、remote-first开始紧接+0ms、首BA616119us/其后AB121397us；after AB p50 18186us、BA121073us（before18573/18048us）。nat-rebind-calibration.json核准确source/control/refs/hash/raw/summary/物理；新Direct :50002在B journal +610ms确实opened，但未见selected，另一wire +1501ms仍selected旧:50001。字段new_quic_path_selected=false；不能把它写成完全未发现新端口，不能把业务绿当最优绿。
- **244 Peer移动实际wire关联在途**：turing-20261109T215036Z-46419d95，与242/243完全同源，含2654 owner停止修复和新outer-wire绑定；PID1170834，按同句柄核终态。原peer_relay_replug_v1全部业务/hold/资源界不变；这轮改源多于Unused单项，不冒称单补丁因果对照。
- **后续NAT诊断补真实请求wire**：exchange_inner在实际open_bi成功后仅以gated lifecycle记wire/stream ID、reading布尔与path facts，不含请求正文/个人路径/token，不额外probe/持有连接/改变期限重试。它关联使用的Connection，不冒充逐包最终路径。244已pin旧准确源，不混此新增观测。

- **237→239/240 blocked-owner红绿闭合**：240 Turing turing-20261109T214055Z-1eb59ad7实际204.43s/rc0/原group空，与239 Mac完全同source。fmt/net67+1ignored/hop12/wires11/journal30/Linux预算24（91.94s）全部绿；原整个carry tests SHA f6a0e6324d3c2b27763cb80b8e1cb90533727f444dcc521c6742259e20534365与237红相同。blocked-carrier-owner-red-green.json保留准确身份/哈希/双平台结果；只证明blocked stream的owner回收，不替代最优选路验收。
- **241只在新诊断格式前置失败**：42705246391，完整source/transport/9 control blobs/hash核；尚未编译/执行。远端format.patch原样应用，不计行为回归。接下来NAT r0→r1实际执行与Mac native同源预检可在不同主机并行，Turing仍单重型槽。

- **239 Mac同整blocked-owner合同全绿**：[42704634733](https://github.com/colehank/khor-build/actions/runs/42704634733)，base2654cbdf/tree34c8e822/snapshot8c3205d4/controle4c58dc。fmt/net67+1ignored/hop12/wires11/journal30/budget20（49.93s）绿；完整source/transport/9 control blobs/hash核。整个carry tests模块与237真实红逐字相同，blocked-carrier-owner-red-green.json已记Mac绿，240 Linux同源待核。不是N1最优恢复已通过。
- **241新诊断与NAT结果标记Mac预检**：[42705246391](https://github.com/colehank/khor-build/actions/runs/42705246391)，requestcc1d544275f64e54814634a87c643184，basef2c40777/tree6c6cd566/snapshot28f49ee6/controle4c58dc，实际状态/结果按句柄核，禁止盲重发。
- **238物理校准独立数据审计已核**：nat-rebind-calibration.json对准确输入、refs、bundle/control/report哈希、原group空、旧新端口同WAN、固定local tuple/conntrack、两族kernel逐项核；business_executed=false明确保留。下一源只补netlab-result成功行，shell guard保持。

- **N1下一轮外层承载关联诊断**：hop_outer_bound记录initiator/receiver、attachment与实际Proto::Tunnel stable wire；middle记录incoming/onward wire与splice开始/结束。不持有额外Connection、不增加probe、不改选路/计时/回收策略，全部详细值仍在KHOR_NET_DIAGNOSTICS开关内构造。235中的Outperformed与228中的Unused不同：235 Y在换网后约5.6/10.6秒用到Carried质量约263ms（samples0、age约4秒）与Relay约131ms，经两looks释放；实际外层对应关系尚缺，不能断言一律是Unused或阻塞I/O。下一真实移动按新准确源关联，原peer_relay_replug_v1业务/hold/预算不改。

- **238 NAT端口物理校准实际1/1绿，业务未执行**：turing-20261109T213804Z-e0f84b61，31.77s/rc2/原group空；完整source/control/bundle/report哈希采证。固定同一local UDP端口，外部198.18.0.10:50001→:50002、定向conntrack删1条/新1条，设备两族kernel完全相同。r0 Rust成功但没有netlab.sh必需的netlab-result标记，driver正确拒绝无scenario结果并停止；不是产品失败。只补成功assert后的标准结果行，不放宽shell guard/物理协议；继续r0→r1。
- **239 Mac与240 Turing验证阻塞旧owner修复**：Mac42704634733/requestae61d279a8544af686af7193fdeaf7c0与Turing turing-20261109T214055Z-1eb59ad7，完整source相同base2654cbdf/tree34c8e822/snapshot8c3205d4/controle4c58dc。net-node含原整个carry/hop/wires/journal/budget合同；实际终态待核。当前只有240为Turing重型槽，后续fixture/诊断不混入。

- **237 Mac blocked-owner实际双分支红**：[42703661504](https://github.com/colehank/khor-build/actions/runs/42703661504)，base5b8f2964/tree5350c4f2/snapshot68c9b768/controle4c58dc。root fmt通过、net66绿1红1ignored；真实writer阻塞后，detach/replacement均未在远端关闭前结束（false/false）。完整source/transport/9 control blobs/hash核。候选只给每个Link加入owner独有oneshot，移除/替换自动drop并停止读写；serial guard保留，整个carry tests SHA f6a0e6324d3c2b27763cb80b8e1cb90533727f444dcc521c6742259e20534365逐字不动。尚待同fixture远端绿，不能把它当235慢路根因已定。
- **235真实Peer relay换网复验：可达绿，最优仍红**：turing-20261109T212616Z-b48bf385，310.96s/rc0/原group空；与233/234完整source/control一致。124/124ok、注入前两次hold10405/10397ms、实际IPv4换网232ms；首AB699996us/BA687733us，before p50 107689/107823us，after154840/154175us，60秒carried_observed_again=false。peer-mobility-calibration.json准确身份/全outcomes/13条非重复lifecycle核；Unused修复尚未解决底层Carried丢失和慢路滞留，继续逐真实外层wire追踪。
- **238 NAT-only物理校准与业务提交**：turing-20261109T213804Z-e0f84b61，复用235/233准确源，不混本地N2修复；controle4c58dc。沿原r0先物理成功才r1协议，单owned重型槽。实际启动/终态按receipt和进程核。

- **236 Mac新blocked-owner合同仅format前置失败**：[42702996220](https://github.com/colehank/khor-build/actions/runs/42702996220)，request5b88b67935934432834ad6f669e69aa5，baseb35df58e/tree59d96993/snapshotfcfa00e6/controle4c58dc。完整source/transport/9 control blobs/hash核，尚未编译/执行；远端format.patch已原样应用，继续同原行为取实际红。Turing235仍是233准确源，不混N2测试。

- **233 Linux/234 Mac Unused误退避红绿闭合**：Turing `turing-20261109T212044Z-2cc026f8` 206.42s/原group空；Mac [42701979487](https://github.com/colehank/khor-build/actions/runs/42701979487)，requesta16ca0476b294d2a97f900250ec61af6。base58cb6475/treea11a73fa/snapshot22997314/controle4c58dc准确源相同，双方fmt/net66+1ignored/hop12/wires11/journal30、Linux预算24（91.96s）/Mac20（49.94s）绿；234完整source/transport/9 control blobs/hash核。整个hop tests模块SHA5f2cecfd427d3397c5a7147f1946248f99068e9317a263736b9510a96ff07ccc与232逐字相同，proof unused-hop-release-red-green.json。仅Unused不再lost，Measured原margin/十分钟、两looks/owner guards保留。
- **235 Peer relay真实移动复验在途（只含Unused修复）**：`turing-20261109T212616Z-b48bf385`，与233/234完整source/control相同，PID1143345；后续本地N2测试/修复不会混入，按同句柄核终态与全部outcomes/最好路独立字段。
- **下一N2 blocked-owner合同已写，未远端format/编译**：真实1字节duplex阻塞writer、观察detach/replacement旧Attached在远端尚未关闭前是否结束；两分支都记录、失败也显式清理，替换新owner必须仍在。先保持原I/O实现取红。拟由Mac net-unit先跑，Turing只运行235，不污染计时。


- **232真实行为红→最小条件修正待验证**：`turing-20261109T211634Z-78586410`，base8a886552/tree11788251/snapshot4b8bc522/controle4c58dc，112.05s、rc101、原group空。fmt/net66+1ignored过；hop11绿1红，真实Links/current owner回收完成而suppressed=true。旧测量Relay/Direct、旧owner与冷排序正反控均绿。候选只在comparison=Outperformed且road=Relay时lost，Unused仍回收但记hop_unused_release；原LOST时长/收益/两look/owner guard不变。整个hop测试模块字节未改，proof unused-hop-release-red-green.json；完整实际移动仍待复验。


- **231仅根format前置失败，Unused合同未执行**：`turing-20261109T211319Z-3e206189`，base9333af64/tree20b1c7fa/snapshot8efac588/controle4c58dc；43.73s、原group空。4278字节远端format.patch已原样应用，原退避行为仍保留；下一准确源取真实红，不把格式失败记成行为红。


- **230 NAT物理前置失败并正确阻止Node**：`turing-20261109T211033Z-83dbb115`，baseffcf7a53/tree2c23700c/snapshot6b05809a/controle4c58dc，31.68s、rc101、原group空。root format/编译过，r0第一条nft table内链closing brace缺换行，0.45s明确syntax error；r1业务未执行，完整采证。只修owned规则header换行；不计网络/Node行为红。下一先N1新unused回归旧行为实际红，之后继续物理校准。

- **下一N1 unused-release新合同已写，旧抑制行为保留取红**：Wires同快照返回Needed/Unused/Outperformed，原bool语义折回仍相同；hop release增加证据参数但暂不改变旧Relay→lost实现。新增真实Links当前owner回收后不得压制middle的合同，原测量失败/Direct/旧owner正反控保留；尚未远端format/编译/红。只有准确源验证后再改退避条件，不能把接口分类存在记成修复完成。

- **229仅根format前置失败，NAT校准/业务均未执行**：`turing-20261109T210331Z-bba623f2`，base54e48129/treec4982e18/snapshotfd93cc4e/controle4c58dc，43.74s、原group空。远端rebind fixture format.patch已原样应用，下一准确源执行r0再r1。
- **228原始慢路追踪新增事实**：X/Y在M换网后都先退Relay；M多份外层Direct约1秒恢复，但inner carried样本开始持续变旧，后来custom paths消失。X+约26秒两次shared_comparison=true时所有live wire都已无Custom，仍调用hop_measured_release并按Relay记lost十分钟。没有新owner替换，不是189的stale-owner反例。先拆分Unused（无Custom消费者的回收）与Outperformed（实测不足收益），不能把前者计成middle输路；底层为何先不再刷新inner path仍待因果核查，不能以拆分类别宣称已定全因。

- **228 Peer relay移动：可用性1/1绿，但最好路未恢复**：`turing-20261109T205517Z-dee53ece`，base08b89cc6/tree8ab90a33/snapshot9dc90f9c/controlf4fa74c，230.56s、rc0、原group空。全部124/124ok，before p50 109187/110073us，首585544/538639us，after157580/156624us；M实际198.18.2.2→198.18.3.2，replug250ms，移动前两次10秒hold都达到。其后60秒未见双向carried再稳定10秒，carried_observed_again=false；不能把case可达绿当最优绿。当前正在核对外层早恢复、inner Carried不再选中的raw path/lifecycle；N1慢路滞留待解决，不能停工。
- **下一NAT-only端口重绑病例/控制已写，8项Python stub控制检查通过、未远端format/编译**：nat_port_rebind_calibration_v1先真实fixed本地UDP tuple、同WAN:50001→:50002、局部conntrack删除与设备两族kernel完全不变正控，失败不启Node；node_nat_port_rebind_v1再真实remote-first目录124请求、原预算/hold/完整outcomes，公共Relay保持。只动owned namespace，完整规则/conntrack原文与新QUIC path事实保留；不会用“本机IP变化”掩盖仅NAT变化。


- **224 Linux/225 Mac同源全绿，冷排序实际红绿闭合**：Turing `turing-20261109T203955Z-d097aa02` 208.58s/原group空；Mac [42695350029](https://github.com/colehank/khor-build/actions/runs/42695350029)。baseb05b5e81b2eafaacbe6d1bcd6f8dbb543973830d/tree8308907bcc420ddd520f34cf441055537e4e0ce8/snapshota6d24403ae447b04b8b4ec08389d55455a56ac9e/control05c8d7c0b792ebc9921f8e6bb907a7b11f0fc8fd。双方fmt/net66+1ignored/hop11/wires11/journal30、Linux预算24（91.94s）/Mac20（49.93s）绿。225完整source/transport/9 control blobs/artifact hashes核；whole新增fixture SHA a0a106ebe2fac9795aa49e4ff1cab49c5b7bbd43bc822d14b3d68a295252bfa2从223逐字未改，proof cold-hop-cost-red-green.json。排序只是N1一项，额外资格/新鲜度/真正全路竞速与实测完整矩阵未完成。
- **226换网三病例3/3绿、完整372/372ok**：`turing-20261109T204430Z-f7a1f53c`，同224/225准确source/control，234.69s、rc0、原group空。v4 NAT首B→A675704us，dual874294us，v4-only→v6-only827742us；之后本端23143/24386/22084us。raw/summary/kernel/12个router前后规则/映射与所有输入身份已核，node-roam-calibration.json。实际Khor远端path地址记录v4 NAT 198.18.0.11:50422→…12:11704，dual IPv6前缀变且v4 path…11:41625→…12:29219，family新selected IPv6 …:2::2:60768；不能单靠不同wire id断言重拨或同wire连续。旧nft 0.9.3在Home fullcone map前向stdout输出“warning: stmt ops map have no json callback”，不是空规则；raw/解析错误与独立成功文本均保留，SNAT/fullcone/APDF及Strict masquerade random已核。首样本不是p99/新SLO，NAT-only端口重绑（本机IP不变）仍需独立测试。
- **227只在根format前置失败，未执行Peer relay病例**：`turing-20261109T205046Z-57cb39ad`，base0cc6ae51/tree8eb26cf3/snapshot8d6a9c3d/controlf4fa74c；43.76s、原group空。5675字节远端format.patch已原样应用，下一准确源继续编译/执行，不算业务红。

- **下一Peer relay移动病例已实现，尚未远端format/编译**：peer_relay_replug_v1，复用f4完整几何/原20ms腿；X/Y经M reported carried连续10s、双向真实marker reads各30、注入前重新连续10s，再M真实换Public IPv4网络，立即双向首业务与各30。最后诊断最多60s是否carried再次连续10s，单独记录，不造新SLO；原1500ms/30s门未改。所有outcomes/kernel/carrier serial/lifecycle留证，profile单病例单线程、360s资源界。共享reads/marker helpers只改测试可见性，诊断传实际case标签防止未来输出混名；旧body/协议不变。

- **Mac控制通道事实**：再次只读SSH握手探针仍banner timeout，macserver别名实际ProxyCommand为/usr/local/bin/iroh-ssh（不是Khor命令），只有该Mac别名；GH guard通道正常，225正在执行。已异步询问可用常规SSH入口/别名，等待期间继续独立网络工作；不是再次等两台设备。

- **223冷候选排序真实红，最小排序候选待验证**：`turing-20261109T203435Z-b255a0a8`，baseff7ed42c/treec637e03b/snapshote468c3a6/control05c8d7c；112.13s、rc101、原group空。root fmt/net66+1ignored绿；hop10绿1红，Measured正控过，Dial实际120/65536/5ms，原合同要求5/120/65536。只把排序tuple从(type,cost,id)改为(cost,type,id)，Direct仅同成本tie-break；原资格/预算/收益/回收不动。整fixture字节不变SHA见cold-hop-cost-red-green.json；现224/225同fixture绿，完整N1仍未完成。

- **222仅根格式前置失败，N1合同尚未执行**：`turing-20261109T203228Z-a8e2de42`，base1487b061/treeae6defe1/snapshotb4f87a6e/control05c8d7c；43.75s、原group空。只是roam诊断新增assert需换行，远端format.patch已原样应用；不计候选排序行为红，下一同整N1fixture执行。

- **221终态采证：dual再次124/124绿、另两例nft JSON仍不合法**：`turing-20261109T202849Z-513b3d42`，101.96s、rc101、原group空；base21457c7e/treece58c098/snapshot336600b5/control05c8d7c。dual首B→A900943us/后A→B19109us，物理与完整raw核。先前把问题理解为纯空stdout不充分：增加空值判断后，另两例仍在非空JSON解析失败，原输出在unwrap前未保留，不能确定它是空规则。现改为先保留raw JSON stdout/stderr/解析错误，并独立保存/核成功的text ruleset；业务/物理协议不改。接下来先N1冷排序真实红，再按准确源继续三病例。
- **下一N1冷候选排序反例已加入、未运行**：原本已经合格的Relay2ms+SSH3ms与Direct100ms+SSH20ms/未知Direct；Measured正控后Dial必须按两腿5/120/65536ms排序。旧生产排序保留用于取得实际红，资格/两middle合计3s/收益门不动。该合同只是尝试顺序，不称完整最优连接完成。

- **220已完整采证：双栈业务绿，另两例fixture前置错误**：`turing-20261109T202504Z-0960af0c`，baseb689aa30/tree821781de/snapshot37baf115/control05c8d7c，104.06s、rc101、原group空。dual真实IPv4 10.0.1.2→10.0.2.2、IPv6 /64前缀…:2→…:3、独立UDP外部映射198.18.0.11:28666→…12:18027；replug256ms，remote-first紧接+1ms开始772543us成功，其后本端23413us；124/124ok。proof node-roam-calibration.json准确source/control/ref/hash与raw核。v4 NAT与v4→v6两个病例在业务前router_state解析空stdout panic，不计产品失败；修正为明确空rules并保留raw stdout/stderr/code后继续原协议。

- **219换网三病例首次仅格式前置失败**：`turing-20261109T202300Z-a9fe0a06`，43.85s、rc1、原group空；准确base38f92150/tree304d4a06/snapshot3750cf89/control05c8d7c0b792ebc9921f8e6bb907a7b11f0fc8fd完整采证，未编译或执行病例。8670字节远端format.patch已原样应用；下一准确源继续执行，旧case门未放宽。

- **218 Node双向/Remote-first两病例2/2绿**：`turing-20261109T201549Z-d2c3d29c`，140.30s、rc0、原group空；basecb1a9089/tree9cc2dfd5/snapshot4e577020/control7277b8ee27df630090deeb497c02a5b813cb77fe准确身份、refs/hash核。248/248全ok，raw与summary/物理IPv6/IPv4不变/nft/最后ping正负控均核。Both首AB584118us/BA1404254us；Remote-first B→A1435327us，A→B21459us且其start确在B finished后（删旧+1638ms，含reads原200ms间隔）。proof node-mobility-calibration.json；不冒称静默网络/新SLO/最优体验。
- **换网三病例原协议保留，220实际编译/执行后修fixture记录边界**：node_replug_triad_v1复用原eth0.replug，IPv4 NAT→另一NAT、双栈同时换prefix/IPv4、IPv4-only→IPv6-only。before/after完整双向n30+remote-first，实际kernel/NAT rules和独立UDP映射正控；不先用probe帮助首业务恢复。原IPv6 pair协议保留。

- **217实际Node迁移1/1绿，已完整收齐**：`turing-20261109T200546Z-6622ff6c`，81.97s、rc0、原group空；base06b67c8c/tree7aa3c7fa/snapshot86c4fafa/control5bc2368准确身份、bundle/control/report hashes与refs核。两方向before/after各n30，加setup2/first2，共124/124全部ok；首AB509570us/BA1703454us，after p50 21362/20830us。实际kernel新旧切换、IPv4完全相同、两端隔离nft前后保留、新IPv6 ping正/旧负均核。raw与summary逐项一致，proof node-mobility-calibration.json。
- **下一新增Remote-first病例已冻结**：沿用ipv6_node_overlap_v1全部物理/负载/预算，新增ipv6_node_overlap_remote_first_v1，删旧后只先发B→A，finished后才A→B；两病例fresh环境，pair profile单线程。前台静默不等于无后台包，不冒称手机/长流/全矩阵通过。217新路径opened/selected在删旧+264/+1595ms；A addrs/network在+3260/+4261ms，晚于首业务，不能把业务恢复归功于Node接口通知。当前无在途，继续新增实际测试，不停在阶段通过。

- **216只在格式前置失败，已完整收齐**：`turing-20261109T195955Z-5d3a0f91`，43.76s、rc1、原group空，没有编译或实际病例。远端导出12572字节format.patch并在一次性副本生成；cached source tree仍是输入b2c6d97b且tracked干净，真实远端证明f583的cache保护。补丁原样应用并提交06b67c8c，再冻结217准确源；不把格式失败当产品行为红。
- **U14业务迁移协议已冻结**：ignored paths::mobile::mv6_overlap_（ipv6_node_overlap_v1），两fresh Node及各自IPC/唯一fixture；mesh后两端eth0阻断全部外部IPv4，loopback保留，IPv4直连/HTTP Relay无备路。setup正控+Direct连续10秒、双向各30基线；A同prefix加新IPv6/ready后删旧，立即双向首请求，再双向各30，所有outcomes保留，最后旧址ping负控。双向同时发送有助于地址学习，首轮不能代表移动端静默的远端首请求；后续须另测。只作业务可达/记录校准，未新造恢复SLO或承诺同wire。215裸包fixture仅三个helper可见性变化、主体不动，旧profile不变。


- **215物理IPv6地址替换已实际校准1/1**：`turing-20261109T192606Z-22c24322`，75.90s、原group空。base897251615ed0932be2e15416721274f57ae7259d/treeb1ed82c5eeea6497fcc272deb3e7bcf4086362cf/snapshotba0e5d7f43527496f778c2ab76b1bdde2d5d5276，control f5838d7d5398da99786108face83bd8f204d8c35。新ignored paths::mobility::在fresh双栈Public namespace实际将IPv6 `2001:db8:1:1::2`换`…::42`、同/64；三阶段kernel JSON与8packet全核（7正、旧IPv6负控1），IPv4地址JSON完全相同且仍可达。地址ready状态已核，不独立证明DAD流程/时延。root fmt/CLI编译/控制与source/bundle/report hashes核，事后cached tree正确且tracked干净。`ipv6-renumber-calibration.json`留全部原值；只是物理注入，不是Node/QUIC业务迁移、前缀变化、IPv4/NAT或手机验收，单包无SLO/分位数。
- **214 Mac42682546955同215完整source预检全绿**：control ea97dd4a1d7d10b92b8201c4b49adba518ad9d66，root fmt/net66+1ignored，完整source/transport/9 control blobs/hash核；不执行CLI物理校准。213 Mac42681901453前版仅新fixture format红，原patch已应用、两Router显式DualStack，source ref/input全保留，不计产品/物理失败。
- **f5838d7控制已push**：Turing新mobility profile编译前先格式检查；若format失败，补丁在一次性副本生成，cached source不改。先用同一小Git＋Python cargo/rustc stub合同取得真实控制红（cached lib.rs被改），再原整合同转绿（bytes/mtime不变且patch导出）；driver2/runner6共8控制检查绿，proof在mobility-calibration/format-cache-control-{red,green}.json。本机没有实际Rust/产品执行。旧timed profiles/网络窗口/jobs16不变，Mac原唯一任务/guard保持。

- **最新功能10ae1ca0：稳定接口通知不被无关候选更新推迟**。Interfaces::saw在filtered set等于seen时直接返回，仅真正新集合重置原1秒settle。生产文件只增加四行guard，210整个fixture逐字不变；空网/同址恢复、回到已通知集合、Local↔QAD来源翻转原合同保留。
- **210真实红→211 Linux/212 Mac同完整source绿**：210 `turing-20261109T185254Z-88fdeca5`，base16a0bafc/treee6aebe0c/snapshotcbbbac6f，fmt/net66+1ignored/hop10/wires11过，journal29过1红、118.09s、原group空；first IPv6变化/IPv4保持iteration失败，reverse未到。211 `turing-20261109T190201Z-d70459fa` 与212 Mac42679667230完整source相同：base10ae1ca0b0935f8b72e78aab89a65c1932a6333b/tree00ca0b6f31322d551402610b7b38d5dd8a7e48da/snapshotaa259f3d1376aa9a2e96a0cb257d336eb20a57e6，control9135b6d4311ce74ee3a90dac188f4b094f780e35。
- 211 root fmt/net66+1ignored/hop10/wires11/**journal30/Linux24**（91.86s）全过，206.47s、原group空；212同源root fmt/net66+1ignored/hop10/wires11/**journal30/Mac20**（50.06s）全过，完整source/transport/9 control blobs/artifact hashes核。整fixture SHA `d66be0e62f349d2298d666c8470e09403f4bee6b53832d9e2fbf9f839315a822`；整生产文件除四行guard外逐字不变，proof `stable-interface-red210-green211-212.json`。green两族iteration均到；不是物理重编号、真跨网或手机能耗证明。
- **旧任务截至215全部终态，217已终态**：211/212终态完整采证；`source-refs-audit-20261109-ipv6-calibration-215.json`已逐run核全部215 snapshot/tree refs。任何后续恢复先查实际句柄/branch/diff，不按摘要重发。

## Peer relay与速度仍待收口

- **201 hop_reference_v2校准9/9**：`turing-20261109T175508Z-912bcc0c`，base9c2e7348/treedcad2092/snapshotd862e1ae/control7598180，1228.41s、原group空。576/576全ok（稳态540、bootstrap/setup36），三case每轮双向n30，raw与summary的p50/p95/max一致；实际两端nft relay input/output drop、peer-hop=off、60秒准备和最终10秒hold核。第一/三轮automatic约147/149ms、fixed-carried约145/146ms；第二轮automatic189.106/173.863ms，同轮fixed-carried145.221/145.701、fixed-relay190.427/193.093ms。可用性绿不代表最快路稳定胜出，v1/v2不混。
- **205/208 lifecycle各3/3**，每run192/192全ok、9诊断文件samples off、原group空；同完整source d90bf666/tree9a7bd3d0/snapshot1a501da8。205 `…183013Z-fdf5424c`/control32fa23c，416.55s，无release，未复现201；208 `…184729Z-59b8eed7`/control9135b6d，435.58s，有双向不对称起点。208第三轮当前serial1（非旧owner）经两次shared_comparison释放并close，测量前31秒，约16秒后serial2重建。原值Carried110.692/109.433、Relay132.754/130.884ms，samples1/0，age约2/4秒；`measured-release-trace.json`保留。符合现行quarter margin的“不足收益”逻辑，不等于替代路明显更快；不能盲降gain或称201因果已定。
- `middles_with_cost`的Measured按两腿成本排序，但冷Legs::Dial等仍有direct首腿优先和候选限制；单middle稳态不能代替全候选冷/暖设计。新候选有界验证、足够新鲜确认、无消费者/试用到期回收应分清，LOST十分钟与“尚未证实收益”不能未经合同混为一类。下一步反例/正控与速度、字节/探测/能耗一起收敛，详见NEXT-DESIGN。

## 用户补充与设备映射（硬要求）

U10：A=macserver（Mac），B=Turing（Linux），两服务器天然异网；P=iPhone。不得再等另找两台电脑，实际两服务器跨网尚未运行。Mac SSH单次banner timeout不证明离线；原GH guard构建通道可用，209/212已实际通过。

U11：Peer relay是可信设备carried候选，与Direct IPv4/IPv6、公共Relay、既有SSH承载共同按实际完整路径/双向体验竞争。覆盖多middle不同速度、middle自己换网/离线；尊重nonrecursive/trust/paused/only_mine/n0。

U12：IPv4/NAT外部映射或端口与IPv6隐私地址/前缀分别变化、同时变化、Wi-Fi↔移动网/地址族切换、双方与peer-relay移动、连续变化后旧结果晚到、休眠/恢复与旧缓存都是硬验收。通知合同只是前置一项；物理注入/发现发布/首个业务/最好路/长流/全部outcomes/成本与iPhone实测未完成。旧地址不能吃完整预算，旧epoch不能撤新路，写请求不隐式重放，健康共享连接不随任意hint全清。

U13：利用Turing128逻辑CPU、约504GiB RAM加速开发，先减少重复工作；不污染计时或缩短发现/hold/故障/稳定门，不默认派发子智能体。当前jobs16保持。

## Turing缓存加速已实证

控制9135b6d引入缓存、现f5838d7已push：原owned root/flock内复用cache/source，先核owner、非软链、tracked clean、准确source tree，未知/脏目录保留拒绝；不reset/clean/删除已有目录。稳定路径与未变文件mtime可复用Cargo，web占位内容相同不重写。Mac保持原隔离checkout。

206 `…184220Z-81707871`与207 `…184314Z-9e2b17cb`同205完整source，两次fmt/net66+1ignored都过，构建17.05→0.41秒，job51.52→35.84秒，实际tests11.79/11.93秒；仅一组同快照初始/复用对照，不承诺通用倍数。`turing-source-cache-pair-206-207.json`留证。driver2/runner6共8 Python控制检查含小Git fixture、mtime与拒绝脏/未知/软链目录，不是本机产品执行。206原输入短control SHA在启动前已锚完整SHA/ref/逐blob，原输入保持，后续stage自动40位。

## 必须保留的先前实证

- 189旧watcher真实反例→193 Linux/194 Mac原整fixture转绿：D077（功能62cf）`Links::detach_attached(peer,carrier,serial)`同锁核当前owner；只有真移除当前owner才lost。整fixture SHA f620fc97…，不回填182/162因果。211/212继承hop10绿。
- 178共享消费者真红→179 Linux/180 Mac整fixture绿：3948872d核Main/Tunnel/served全部live消费者、stable ID去重，unknown保守。iroh peer共享selected FourTuple但每conn open paths不同，不能说每conn独立selector；211/212 wires11仍绿。
- 195 held_precondition_v2三轮3/3：实际hold10444/10460/10455ms后故障，原1500ms恢复/30秒稳定门通过，73/73全ok。负向timeout/reset未冒称覆盖。190原诊断0/3、182原hop8/9、162/168历史均保留，不被合同绿抹去。
- N0 recorder v4真实183校准：5请求ok2/error1/timeout1/cancelled1，SIGSTOP/CONT与原期限实核；caller取消不证明远端停止。paths/hop全outcome保留，旧v3遗漏error不能补零。
- F8真红→150/151；F9 tick/F10长Accepted取消真红→160/161；当前211 Linux24/212 Mac20重新覆盖原合同，四Linux namespace合同不靠Mac替代。
- 176/181固定核stalls均原门2/2，但candidate p1s18 punch/728ms/更多bytes，对照3 punch/537ms，host load不同、单顺序pair，不定单补丁因果或总体提升；真实resident约275–280 threads，尚无能耗结论。

## 下一步与完整范围

1. 继续N1：以208原始quality/样本/年龄与逐wire时序补“稀疏/静默/真正更差”正反合同，分清验证/确认/成本回收；冷/暖Direct IPv4/IPv6、Relay、SSH、peer relay含多middle的有界竞速/持续优化，不盲改gain/时间门。再按准确源跑hop/recovery/dynamic/full、同版本baseline和固定好路。
2. U12业务迁移与扩展物理校准：215同prefix IPv6替换几何已成立，下一步把真实Node/QUIC目录/长流接上，验证旧/新地址发布发现与首个业务/最好路；再补IPv4/NAT映射、IPv6前缀、独立/同时与旧结果晚到、peer relay移动。所有phase/outcomes/资源回收保留，现p4只作无误报控制。先冻结基线/目标再改通知之外的策略。
3. N4实际macserver↔Turing：Mac主动配对fresh Turing身份的私有临时能力方案，Node::serve/open_as/invite/pair/devices/ls_of与同进程stop_serving API已审计，尚未实现。需要lease/绝对期限/身份/角色同步/真正join/owned-root清理合同，ticket不进源码、普通日志或通用artifact。完成服务器后接iPhone；不碰已安装App/个人网络store。

| 工作包 | 仍必须完成 |
| --- | --- |
| N0 | 最终目标/容差/样本量及p99充分性，全负载所有outcomes、v4/strict hold/reference v2同版本baseline/candidate/固定路与负向校准；不混旧量测版本 |
| N1 | 201慢轮/208当前owner确认，162/182未定因，冷暖多候选/双向与多middle、稀疏/噪声/CPU/防抖/故障/恢复、探测成本与完整矩阵 |
| N2 | IPC剩余预算/取消贯通（正常half-close≠取消、写不重放、读≤1重试、0RTT信任保持），长多阶段原by、两middle合计3秒、输者/晚候选回收、近期地址/重启、jitter/事件合并、quiet epoch与close-vs-offer竞态 |
| N3 | 真实Node32席位/content32、Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备后台、CPU/字节/唤醒/手机能耗；Noq FIFO_v2低层数据不能替代 |
| N4 | U12完整移动矩阵与Mac↔Turing↔iPhone真跨网，冷暖/双端不对称、登记/出口/整机故障域、布局/容量/运维实测；不是采购部署 |
| N5 | 48 vendor补丁（Iroh37+Noq11）逐项覆盖/上游等价/退出，Noq1/5等缺口，准确最终源可信Quality、Mac原生App/代理/多机/手机/能耗；证据前不升级/删补丁 |

## 工作区与执行规矩

- 产品：`/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net；control：`…/khor-build-net`，feat/net，f4fa74c已push。先核实际HEAD/diff；docs提交不改变上面的211/212准确源适用性。
- 旧baseline `khor-net-baseline` detached6485六fixture WIP、CPU baseline `khor-net-baseline-cpu-v3` detached6485七staged fixture/helper均未触动，原refs/ledger保留。
- 本机source-only，禁Cargo/Rust/Node/Xcode编译、产品测试、formatter、下载可执行物。只用Git/Python stdlib、脚本stub与数据核；不得改installed policy、ensure-budget、已装App/生产store/其他worktree。
- Turing root `/tmp/khor-netlab-feat-net-1110`，owned单flock重型槽，Rust1.95/jobs16/nice10，50GiB准入/40GiB stop/90min，计时不并跑编译/其他重负载，无sudo/宿主网络改动，仅user/netns；SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking=yes，不传个人密钥/token/agent/签名/模型凭据。
- Mac走macserver/khor-build现有guard单槽/jobs2；临时ticket未来只能给可信controller，不复制个人能力。202两次pre-job GitHub内部错（jobs=[]/无artifact）留为执行器事件，209/212后来成功；203错误cwd快照仅stage未启动、source预检已修，均不得算产品结果。
- `evidence/net/source-refs.json`、`turing-next-matrix.json`、各receipt/collection/report hashes为句柄事实。源码/控制/输入先锚，再改动；实际/proc cmdline＋terminal group，不信状态文件代替存活，观察超时不等于停止，不盲重发。
- ops/submit-turing.py stage显式task/revision/control/role，snapshot固定产品cwd并核预读HEAD，复用旧source用--source pure metadata或Turing receipt；launch需前驱完整终态采证并核group/flock/inputs。collector返回101可能是成功采到产品红，先读collection，不重复覆盖。
- ops/verify-mac-record.py只核GH/Git/artifact身份，分类要读日志；Mac临时source/bundle分支只有终态、留证/所有权/SHA一致才清理，复用者不删原请求分支。当前未清理，不擅删。

完整计划与硬要求见PLAN/REQUIREMENTS/ACCEPTANCE；决策边界见NEXT-DESIGN，原始演进见RUNS/REPORT。本页重写前逐字状态保存在HISTORY-20261109-before-mobile-interface-verification.md，再沿历史链；历史在途只代表当时。外盘缺失遵守恢复入口，不按摘要覆盖WIP。最近给用户完整计划粗估3–5开发日，受新回归、真机与能耗影响，非完成承诺。
