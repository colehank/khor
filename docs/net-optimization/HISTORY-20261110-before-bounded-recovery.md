# feat/net 当前状态与恢复入口

更新：2026-11-10 00:07 UTC（本地2026-11-10）。root串行，无子智能体。

完整目标仍是完成[PLAN](PLAN.md)的N0–N5，尚未完成。用户已授权实施/远端验证并多次要求继续；U14明确不能因goal服务paused或阶段通过自行停止。后台调度状态不是本轮停止/授权门，不另建目标，不伪造complete/blocked。速度和体验优先，个人可信设备网络；无合main、发布、中继采购/生产部署授权。

本页重写前逐字内容在[HISTORY-20261109-before-wan-access.md](HISTORY-20261109-before-wan-access.md)，再沿其历史链。所有旧真实红/失败保留；新绿不能回填旧因果。完整硬要求/协议见[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)，设计边界见[NEXT-DESIGN](NEXT-DESIGN.md)。

## 先核当前句柄，禁止盲重发

- **恢复等待接续只加观测，尚未新发probe**：scripts/netlab-peer-selection.py固定four-second物理合同已用267/272原数据取红（51个deadline后warm requests分别42/40选对，9/11仍Relay），checker/fixture hash在peer-recovery-four-seconds-red-green.json。新增gated full-samples诊断只读实际conn/path统计：last_received>judged但latest_rtt_sent<=judged时记PTO/unacked/sent/received，用于确认“旧ACK清PTO后等keepalive”的假设；默认无新probe/timer。274长回归仍沿旧准确source，不混当前WIP。

- **274 Turing唯一重型长回归在途**：turing-20261110T000645Z-a87d2f92，PID以receipt核，netlab-stalls；现代Noq12特定对照baseline：660483fb＋actual-latency测试fixture，tree d25e828d/snapshot9c8fa9d4/control4eeeea9。实际p1s 600秒/20次300ms/checked affinity原负载＋原p5全部150/300/600ms×4周期，不缩时/不改门/不夹别的Turing编译。其后候选必须沿相同control/模式准确a42 source执行，结果全部保留。该baseline只隔离Noq12，不冒充原6485整个feat/net baseline。
- **272/273均终态采证绿**：272 168.291s/rc0/原group空，124/124ok/reported Carried再hold10秒，原physical fixture；准确同271/273 fullsource0c6ab3ea/tree48b907da/snapshota42b5b64。按正确phase outer Go+1131/+1289ms，inner开始考虑Custom+5451/+5709ms、Go+7701/+7959ms，复现恢复等待而非全完成。273 Mac42724807967 fmt/net67+1ignored/hop12/wires11/journal30/budget20（49.92s）全绿，11control blobs/source/transport/hash核。

- **272旧重复句柄已完成**：turing-20261109T235801Z-51970ada，PID1314016，netlab-peer-mobility-samples，base0c6ab3ea/tree48b907da7a1d47c6d00db106763260c6b36b12c9/snapshota42b5b642b3d143c13d42739802ae11af32f69e7/control ef0b1ab。与271 vendor准确同source，原physical fixture不变，不能因旧267绿而盲判本轮。
- **273旧Mac句柄已完成**：request124966a85c8742a28fcd0fb338234eec，同272/271完整source，control4eeeea9，root fmt/net/hop/wires/journal/budget；准确状态以GH/ledger核。
- **270旧质量红→271完整vendor绿**：Mac42724156712（660483fb＋唯一actual-latency fixture WIP，tree d25e828d/snapshot9c8fa9d4）438绿/1红且仅ACK-floor；Mac42724277499（0c6ab3ea/tree48b907da/snapshota42b5b64）439Noq/36iroh path/9home与fmt全绿。两者均control4eeeea9/11blobs/原artifact hash核，整个Noq paths tests模块SHA1d8da16d、整个multipath.rs SHAfff96ca1相同；全部Rust diff只selection sample九行（ack-floor-controlled-rust-diff.patch）。原268旧短停顿红保留；修正实际两腿10ms/20msRTT，原300ms零提前lost/350ms两PTO后释放/<1秒/完整流与新增准备条件保留，两源该用例均绿。隔离对照worktree khor-net-ack-floor-control detached660483fb仅本次fixture WIP，勿覆盖。
- **269严格持续NAT门首次远端实际绿**：turing-20261109T235237Z-bb957183，rc0/原group空，准确267完整source、control4eeeea9；124/124ok、warm remote30/30选新Direct、0 mismatch；mapping64ms，remote-first+1ms。新report/nat-selection-contract.json与原独立冻结checker逐requests/problems相同，no threshold changes。
- **267单轮peer移动绿，恢复等待仍未优化完**：226.642s/rc0/原group空，124/124ok/reported Carried再hold，原physical fixtureSHA保持。before AB/BA p50110123/109834us；after AB156074/BA110983us为先后窗口，不称持续方向性故障。按真正middle_replug_finished phase：outer Relay→Direct Go+1244/+1287ms，inner重新考虑Custom+5317/+5326ms、Go+7326/+7567ms；已核rehold=false/immediate=true；约4秒是在等待post-judgment新往返（不是SILENT_HOLD），之后约2秒才是确认，不能宣称已极致/全移动根因闭合。
- **时间参照勘误**：旧outer-copy-timeline.json中本轮曾把result.after.at_ms（reads完成后的kernel snapshot）误标为replug参照，旧字节保留；新outer-copy-timeline-replug-reference.json明确使用middle_replug_finished。260实际四端首快照+3407/+3428/+3511ms，均Direct，quality约41–51ms/age67–318ms/samples8；后续+8.4/+13.4秒亦保留。267上述延迟已按正确phase。全量原始logs/phase/身份未变，不用地址快照倒推移动时刻。

- **267旧句柄已完成**：turing-20261109T234540Z-3661a3e2，PID1298280，netlab-peer-mobility-samples；base3afd69c1/tree3386599f472cfd9f58f3166d7b23f268fc0179d2/snapshot62dda02e9ef539a29baa7bbd8eaa5a5889634510/control ef0b1ab。仅Noq12质量修正，原完整physical fixture/负载/断言/诊断模式与265不变。已真实终态采证/原group空，详最新272句柄。
- **268 Mac42723181146旧vendor候选已终态红**：request4507e610d5ea4088a3d0738b13ab1233/control4eeeea9，canonical source文件evidence/net/ack-floor-candidate-source.json；准确tree/snapshot与267相同，不把后续docs HEAD混作候选。
- **266单元实际红**：Mac42722555246，fmt通过，Noq438绿/1红（109ms≠85ms），source660483fb/treeefa289e0/snapshotdf0d095c/control4eeeea9，11control blobs/source/transport/hash核。whole paths tests模块SHA1d8da16d59f86c11595f10952b40139510f89b35344058db47a5c6a2c4ff7363红→候选不变。Noq12只将独立selection sample夹到已测floor；transport adjusted/SRTT/variance/PTO/拥塞/25%门不变。270/271同fixture真实红绿已取得，详最新项。
- **264/265均移动前准备失败，不是移动可用性红**：分别194.445/210.546s，rc101/原group空，只有2 setup logical requests ok，150秒未维持Carried，尚未移动。实际四个outer端在两次hop观察均Direct约41–42ms；inner Custom quality约109ms、Relay约131ms，两look因既有25%门不足Outperformed回收。265八个Custom比较值按纳秒唯一对应此前同resident ACK-floor样本，raw-delay正且略低于min而sample原raw。preparation-outer-timeline.json及ack-floor-quality-correlation.json留证，数值关联不是逐包ID绑定，不冒称唯一移动根因。
- **263真实Mac↔Turing首轮双向60/60ok**：Mac42721024415/coordinator e0b0b491f8336536/controlebf7f0d，ff608e91/tree7d0c7467/snapshotb13e4398完整source。Mac→Linux n30 p50/p95/max61004/65659/84881us；反向59729/61452/71335us。实际两端公共Relay101.200.80.77:3340，不称Direct/peer relay/最优。fresh身份逐ID相符、两端resident joined/owned空、Linux原group空/锁释放，成功后自有fresh身份/复制binary清理；临时Actions secret pair后删除。Mac11files/11control blobs/hash核。wan-first-paired-proof.json。仅paired-warm目录list（marker名/类型/大小），非冷建连/正文/移动/手机/能耗/p99，只有一轮。
- **两平台真实生命周期各3/3绿**：Mac262(42718502406/control7ed29ab)及Linux69e7346下stop9d6744346bfdb955/eofa5912e892695a08d/killf351939e640ec31a，均worker rc0/resident joined/owned空/lease socket无/锁释放；同ff608完整source。Mac10control blobs已核。Linux旧三套fresh homes/tmux/copy已在269终态后原flock内核owner/hash/真实原group空并清除，仅留sanitized owner/lifecycle/inputs；linux-lifetime-cleanup.json留证，未动cache/target。
- **260/261旧诊断绿边界**：260 226.606s/rc0/原group空、124/124可达、reported Carried恢复hold；四端准确replug phase时间已由上方勘误/新timeline修正，旧timeline字节保留。不是完整重复SLO。261 Mac42717041225同代码tree不同snapshot便携全绿67/12/11/30/20。其单轮绿没有抹掉264/265红。
- **严格NAT可持续门已实现**：scripts/netlab-nat-selection.py复放247/249真实数据，与冻结checker每个requests/problems相同，仍0/30+69 mismatch红→30/30+0绿。control4eeeea9仅lifecycle NAT可达绿后执行此门，Python控制合同含选择门失败退出绿；第一次远端实际新门仍待。原fixtureSHA ac97726906429a5dd63dd91b5c9ed99624c273d9a253f7facb916ffcf47e8a9b未改，无新延迟阈值，独立source/transport身份审计继续。
- **证据与执行器**：source-refs-audit-20261109-266.json逐核266refs/source trees/control commits及23ops脚本hash；后续逐条pin，终态后刷新。Mac SSH新试仍banner timeout，既有GH guard通道正常且已做实际WAN，不再当数据面阻塞/不推Khor故障。当前总50vendor（Iroh38+Noq12候选），原48仍全审。
- 更早句柄逐字内容见[HISTORY-20261109-ack-floor-candidate.md](HISTORY-20261109-ack-floor-candidate.md)及其归档链。产品/控制HEAD以Git、候选以receipt为准；无main合并/发布。

## 最新实证与未闭合问题

**247已终态，NAT错路因果取得真实反例**：138.17s/rc0/原group空、124/124可达绿；B实际Noq conn/path1已迁50002、iroh仍缓存50001，69条逐次不一致。独立合同nat-selection-contract.json实际红（warm remote 30次0通过、新tuple未selected）；原整个rebind.rs SHA ac97726906429a5dd63dd91b5c9ed99624c273d9a253f7facb916ffcf47e8a9b保留不动。下一候选只在既有reselect前reconcile真实tuple、移除变更path旧Reading、重新登记/强制apply；不改策略/计时/并发/发包。尚待同整fixture真实绿与vendor合同，不称已完成。248 Mac同代码tree便携全绿（67/12/11/30/20），不同snapshot元数据明确保留。


**246已终态**：132.16s/rc0/原group空，r0+r1可达2/2、124/124ok；after AB p5022166us、BA123616us，首BA660726us。nat-rebind-calibration.json与request-wire-correlation.json核141条非重复lifecycle；实际after-change每方向31个request streams，B业务wire已有新:50002但所有路径selected=false，另wire仍selected旧:50001。247只增加Noq真实tuple vs iroh缓存的gated mismatch诊断，保持旧行为查因，不盲改策略。


### Peer relay与最优连接（N1/U11）

- **冷候选排序223真实红→224 Linux/225 Mac同整fixture绿**：功能b05b5e81把已符合资格的候选按完整两腿成本排序，Direct只同成本tie-break。原Dial慢Direct120ms/未知65536ms曾压过快Relay+SSH5ms；现在5/120/65536。whole fixture SHA a0a106ebe2fac9795aa49e4ff1cab49c5b7bbd43bc822d14b3d68a295252bfa2，proof cold-hop-cost-red-green.json。资格限制/预算/收益门仍待完整设计，不冒称全部cold竞速完成。
- **Unused误退避232真实红→233 Linux/234 Mac同整hop tests模块绿**：功能58cb6475只在Outperformed且road=Relay时lost；Unused仍回收但不惩罚middle。所有Main/Tunnel/served live wire单快照/去重，unknown保守，旧owner guard保留。whole module SHA5f2cecfd427d3397c5a7147f1946248f99068e9317a263736b9510a96ff07ccc；proof unused-hop-release-red-green.json。原25%收益/两look/LOST十分钟未放宽。
- **228/235真实middle IPv4换网均124/124可达绿、最好路未恢复**。235仅含Unused修复，310.96s，before p50约108ms/after约155ms，60秒carried_observed_again=false；Y在换网后约5.6/10.6秒作Outperformed判定（Carried质量约263ms、age约4秒、samples0；Relay约131ms）并释放。228另为所有Custom消失后的Unused旧误退避。不能用同一解释覆盖两run。proof各peer-mobility-calibration.json、235/measured-release-trace.json，汇总peer-mobility-progression.json。
- **244新outer-wire关联：124/124可达绿、reported carried再次hold10秒**，turing-20261109T215036Z-46419d95，204.45s/rc0/原group空；与242/243同完整source d1a7a401/tree0bb908a7/snapshot6833a423/controle4c58dc。replug273ms、首AB636590us/BA617343us；before p50 110260/108046us，after155669/154371us，reported恢复hold从+21633至+32111ms。**测量reads在+21602ms已全结束，没有恢复后业务样本，不能称体验已达标**。当前serial2的实际outer绑定已记；原serial1在移动前约56秒已测量回收，不能把它算移动故障。outer-wire-correlation.json保留事实与限制。不同源包含owner停止修复和诊断，不能宣称单补丁因果。
- 新诊断仅在开关内记录实际initiator/receiver attachment→Proto::Tunnel wire、middle incoming/onward wire/splice起止；不额外持有连接或probe。新增request_stream_opened记录真实open_bi的wire/stream、reading布尔、当时path facts，不记正文/token/个人路径。它是使用连接与开流快照，不是逐包最终承载。
- **继续项**：先完成246实际业务wire关联，定位新路已打开却慢方向仍滞留的原因；244要核实际两腿wire，再补恢复后真实业务/多middle/稀疏新鲜确认与原f4/完整矩阵。201慢轮、208不足收益回收、162/182旧失败因果仍未关闭。

### 移动地址（U12）

- **215裸IPv6物理替换**1/1＋7正1负ping；**217真实Node IPv6替换124/124**；**218原双向与remote-first两例248/248**。真实同prefix旧址删/新址加、IPv4不变、两端eth0全部外部IPv4被nft阻断，不能借IPv4/唯一HTTP Relay过关。恢复可早于Node接口通知，不把业务恢复归功于210修复。不是静默后台/同wire/长流/手机证明。
- **226三病例3/3、372/372ok**：IPv4 Home→Strict NAT、双栈IPv4/NAT+IPv6 prefix同时变、IPv4-only→IPv6-only。真实kernel/12router rules/独立固定local UDP映射及Khor远端path记录均核；remote-first首分别675704/874294/827742us。旧nft0.9.3 fullcone map会在stdout前置warning，raw JSON/错误/成功text规则均保留，不冒称空规则；node-roam-calibration.json。
- **238 NAT-only r0实际物理1/1绿，但缺标准netlab-result导致driver拒绝，r1未执行**；只补成功assert后的结果行，shell guard不放宽。nat-rebind-calibration.json明确business_executed=false。
- **242 NAT-only r0+r1可达2/2绿，最优不绿**：turing-20261109T214614Z-f70be96f，138.23s，原group空。固定同local tuple/同WAN:50001→:50002；两族kernel相同、定向conntrack删旧/新QUIC映射实际留证。r1全部124/124ok，mapping82ms、remote-first紧接+0ms，首BA616119us/其后AB121397us；after AB p50 18186us、BA121073us（before18573/18048us）。B新Direct :50002在+610ms确实opened，另wire+1501ms仍selected旧:50001，new_quic_path_selected=false。不得写成完全没发现新端口，不能把可用绿当最优绿。完整nat-rebind-calibration.json。
- **246独立lifecycle模式**保留242全部物理/业务/hold/预算，只补实际request wire日志。控制fc204ac有新独立task，普通profile保留，8项Python cargo/rustc stub/小Git控制合同绿；没有本机产品执行。
- 仍需仅外部IP变化/更长映射缺口、双端/中继IPv6/前缀移动、连续变更旧结果晚到、休眠/长流/真实手机；部分可达绿不代表U12完成。

### 期限、取消、资源归属（N2）

- **237 Mac blocked writer实际双分支红→239 Mac/240 Linux同整fixture绿**：功能2654cbdf每个Link持有独有oneshot，detach/replacement drop后停止该旧read/write，即使flush阻塞；serial guard不变。真实一字节duplex证明writer在阻塞，远端不关闭前old Attached必须结束，旧源false/false，新源通过；替换新owner仍在。whole carry tests SHA f6a0e6324d3c2b27763cb80b8e1cb90533727f444dcc521c6742259e20534365，blocked-carrier-owner-red-green.json。双平台fmt/net67+1ignored/hop12/wires11/journal30、Linux预算24/Mac20全绿。不是N1慢路根因证明。
- 189真实stale-owner红→193 Linux/194 Mac同fixture绿：detach_attached同锁核serial，旧watcher不得撤新路/记lost。
- 178真实共享消费者红→179 Linux/180 Mac同fixture绿：所有live Main/Tunnel/served、stable ID去重；iroh selected FourTuple按peer共享，但各conn open paths不同，不能按每conn独立selector解释。
- F8/F9/F10期限、Accepted后取消已有真实红绿；245Mac20/240Linux24重核。跨进程IPC预算/取消尚未贯通；请求正常write shutdown后read回复，服务端EOF不能误当取消。写不重放、读≤1重试、0RTT信任边界/健康共享wire保持。
- 仍需输者/晚候选回收、两middle合计3秒长多阶段、jitter/事件合并、quiet epoch、旧地址/重启与close-vs-offer竞态；不能以本项资源绿缩小N2。

## 设备与执行器（用户硬要求）

- **A=macserver（Mac），B=Turing（Linux），两台天然异网，P=iPhone**。用户已提供设备组合，不再等待别找电脑。
- 用户2026-11-09约21:54UTC答“再试试 现在可能通了”；随后实际SSH已成功，host z13deMac-mini.local/Darwin arm64/uid501。Mac owned builder根/现有guard只读核，free约65.3GB；SSH别名ProxyCommand是/usr/local/bin/iroh-ssh，**不是Khor**。此前banner timeout只是当时通道观测，现已恢复，不能继续当阻塞。真Mac↔Turing数据面尚未执行。
- N4真跨网下一按NEXT-DESIGN：fresh私有owned root/identity/store/fixture、两端准确binary/source；Node::serve/open_as/invite/pair/devices/ls_of与同进程stop_serving已审计，具体lease EOF/绝对期限/身份核/真正join/清理合同及harness尚未实现。临时ticket仅可信controller私传，不进源码/普通日志/通用artifact，不传个人凭据/生产store。不能设KHOR_NETLAB；保持KHOR_NO_MDNS与独立home/socket。首次配对后读不能称冷网络建连。独立stage准备可以推进，计时不与246争用Turing。
- Turing128CPU/~504GiB用于缓存/独立Mac并行验证，不默认派子智能体。稳定路径缓存206/207同快照17.05→0.41s构建只是一组，不承诺通用倍数；fmt失败补丁在一次性副本生成，原cached tree干净/mtime保护已远端实核。

## 完整剩余范围，不得缩减

| 工作包 | 仍必须完成 |
| --- | --- |
| N0 | 最终目标/容差/样本量与p99充分性、全部outcomes、同版本baseline/candidate/固定好路和负向校准；不混旧量测版本 |
| N1 | 冷暖所有候选/多middle/双向、201/208/244/242慢方向因果与恢复后真实业务，新鲜/稀疏确认、探测成本、原hop/recovery/dynamic/stalls/full矩阵与162/182历史失败 |
| N2 | IPC剩余预算/取消贯通、长多阶段/两middle合计3秒、输者与晚候选、近期地址/重启/jitter合并/quiet epoch/close-vs-offer；写不重放/正常half-close不取消 |
| N3 | 真实Node32席位/content32、Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；Noq FIFO_v2低层704请求不能替代 |
| N4 | U12完整移动矩阵、Mac↔Turing↔iPhone真跨网、冷暖/双端不对称、登记/出口/整机故障域、布局容量运维；不是采购部署 |
| N5 | 原48＋新增38候选，共50 vendor补丁（Iroh38+Noq12候选）逐项覆盖/上游等价/退出、Noq1/5缺口、准确最终源可信Quality、Mac原生App/代理/多机/手机/能耗；证据前不升级/删补丁 |

## 执行与证据规矩

- 本机source-only，禁Cargo/Rust/Node/Xcode产品编译/测试/formatter/下载可执行物。Git/Python stdlib、数据分析与纯stub控制合同允许。不改installed policy/ensure-budget/已装App/生产store/其他worktree。
- Turing owned root `/tmp/khor-netlab-feat-net-1110`、uid1110，单flock重型槽，Rust1.95/jobs16/nice10，50GiB准入/40GiB停/90min。仅fresh user/netns、不sudo/改宿主网络；计时不并跑编译/其他重负载。
- Mac沿macserver/khor-build既有guard单槽/jobs2。installed run-build→build_guard.py持有cache/build.lock，50/40GiB与100GiB target门，不改/绕过。SSH开发也须走该guard，先核实际任务占用；不得让自有resident脱离guard生命周期。
- SSH BatchMode=yes/ForwardAgent=no/StrictHostKeyChecking=yes，不转个人key/token/agent/签名/模型凭据。源/控制/输入先锚refs/hash，变更后新快照；观察timeout≠停止，核/proc cmdline、终态原PGID空、collection，不盲重发。
- ops/submit-turing.py stage显式task/revision/完整control/role，当前HEAD或--source准确元数据；launch要求前驱完整终态采证、group空/flock/inputs正确。collect-network-ssh返回101/2可能是采到产品/fixture失败，先读collection不覆盖重采。
- ops/verify-mac-record.py只核GH/Git/artifact身份，分类须读log。源/transport/9control blobs/hash须核；别把fmt/编译/fixture/前置/行为/执行器失败混算。
- 旧baseline worktrees（detached6485与CPU snapshot）未动，现代fixture matched baseline仍待做；不能checkout/reset覆盖其WIP。Mac snapshot/bundle临时分支尚未清理，只有终态、所有权/哈希留证后才可清；不擅删复用者分支。
- graph first；本worktree无CodeGraph不自建，code-review-graph可能滞后HEAD，先图谱再定向读缺口。无子智能体授权，root串行。
- 183 recorder v4真实2ok/error/timeout/cancel全部outcomes、195 held_precondition_v2三轮原1500ms/30s门、176/181固定核stalls成本增加等旧证据详见历史；不因新校准抹去局限。

最近给用户完整计划粗估3–5开发日，受新回归/真机/能耗影响，非完成承诺。下一步继续246实际wire因果与新快路/业务验证，同时把恢复的Mac SSH接入后续N4；不要在阶段绿后停工。
