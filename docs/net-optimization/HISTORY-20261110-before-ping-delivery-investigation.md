# feat/net 当前状态与恢复入口

更新：2026-11-10 04:51 UTC。用户已授权继续完整N0–N5，尚未完成。root串行，无子智能体。旧状态逐字保存在[HISTORY-20261110-before-window-and-process](HISTORY-20261110-before-window-and-process.md)及历史链；计划/硬要求/验收以[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)为准。速度/体验优先，可信个人设备网；无main合并、发布、中继采购/生产部署授权。

## 恢复先核实际句柄

- 主worktree `/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net，功能HEAD757e013f（此后仅计划文档），53 vendor。控制worktree `khor-build-net` feat/net HEADccd8576cabec82510019c786d79948b66d57bc74，已推。先核Git/receipt/GH/真实PID，不盲重发；本段优先旧历史追加，逐字旧状态在[HISTORY-20261110-before-quic-recovery-diagnostics](HISTORY-20261110-before-quic-recovery-diagnostics.md)。
- **Turing唯一重型任务：turing-20261110T044937Z-36296fd4**，PID1694050，04:49:58UTC启动，netlab-peer-mobility-samples/ef0b。隔离88c3c3b9/tree212b08e7/snapshot41149e3c，在c090诊断上再加line_to_inner等待锁/hop_ahead/实际dial开始结束的opt-in lifecycle，不改策略，未合主。原whole物理fixture保持。接下来先collect/分类cold-start，再关联recovery每copy，不盲刷绿。
- **当前canonical真跨网120/120已绿**：coordinator4cee410c97601afe/Mac42757732776/controlccd8576，同757e013f/tree2d218d48/snapshot3f0c7ea6；双向各30目录+30固定29B正文均ok，双端fresh配对ID相符，MAC11blobs/source/hash核。实际公共Relay101.200.80.77:3340，非Direct/peer/移动/手机/p99验收。两端joined/owned空/Linux原group空/锁释放，临时GH secret已删，Linuxfresh identity已清；Mac trusted成功清理路径已执行，直接目录观测与6+2 lifetime fixture GC尚待。详wan-current-directory-body-proof.json。
- **新cohort生命周期Mac3/3+Linux3/3真绿**：stop/EOF/kill均Node真正join/worker0/owned空/锁释放；wan-current-lifecycle-proof.json。最初两stop运行的post-observer脚本IndentationError保留，readonly恢复已核清理且不计pass，然后新六轮绿。guardian69e7346与currentccd脚本字节一致。ops/wan-runtime-contracts.py只修observer literal/dedent（3条remote snippet语法核），旧副本与恢复记录保留。新wan-binary-manifests已同3f0 cohort，旧ff608逐字备份；MacSHAcd9be1a8/LinuxSHA0e063201，binary不在本机执行。
- **N1当前未验收，一绿一红**：Iroh41同源f07db0c9 fullsamples ad62734d双端3816/3613ms绿，repeat2 34871ed6为3747/7238ms、一端红。后续诊断源c090b190/tree381b9052/snapshot7d23febd只增加关联conn/path的debug观测，未合主；Turing042825-2a7fe04f已实际初连10.032秒超时/error1、未replug、rc101/原group空，不能当恢复时序。保留cold-start独立缺陷，不盲刷第三绿或猜测调参。
- **N2新资源分项同源Mac/Linux全绿**：Mac42755640884与Turing042313-d964a40d，准确757e013f/tree2d218d48/snapshot3f0c7ea6/controlccd8576。实际fmt/portable/IPC18/本机32/process1/远端QUIC32/原canonical busy-wire整例均绿，两端process预算1006ms左右、真实join/reap；源/控制/整fixture/hash均核。前驱两轮formatter、一次test-build错误与真实QUIC红完整保留。
- macserver SSH最近03:54UTC成功，hostname z13deMac-mini.local；此前banner timeout历史保留，不当Khor数据面故障。A=macserver/B=Turing天然异网，P=iPhone，不再要求另找两电脑。当前源提交用本机GitHub SSH上传，worker保持原API bundle，无个人凭据转发。

## 新闭合与当前红项

### N1 恢复与成本

- Iroh41同源fullsamples第一轮ad62734d实际124/124、请求51/51、Go3815.789/3612.992ms及预算7请求/3episodes绿；repeat2 34871ed6仍124/124、请求51/51/预算6请求2episodes绿，却Y Go7237.649ms红、X3746.991ms绿。三次同源达标未成立。peer-recovery-decision-four-seconds-red-green.json保留全部。旧fullsamples040407-e3bf首次业务10.032秒超时未replug，亦保留。
- 慢Y已证986/1237/1488ms三probe，outer1249ms Go，1486ms quality reset220→129，5237ms才开始inner Moving、7238ms Go。期间ACK日志无conn ID，不能把98.8/41/130ms直接归某copy；新增c090只debug打印conn/path/actual stats/Reading/road quality，不改变策略。该诊断首轮042825再次初连超时，尚未取得replug后观测，须先诊断cold-start。对照Iroh40 040644-3d3ebe28原124/124可达、Go5385/5374ms均红，完整同profile而非精简日志。

- Iroh41首轮vendor `turing-20261110T033054Z-ada79942`，准确335b8196/tree0c3da38b/snapshot2b4c25c1/control96fedf90，实际fmt/Noq439绿、paths49绿1红，home未执行；rc101/原group空。新窗口仍按旧changed计数排名，导致新copy新鲜89输给旧85。
- 1514516e只将representative confidence改为当前recent样本数，**整个paths tests SHA5c4da4b4不变**；`turing-20261110T035447Z-0755e480`准确1514516e/tree94f45efd/snapshotf07db0c9/control5f310f43实际fmt/439/50/9全绿，45.799秒/rc0/原group空。recovery-quality-window-vendor-red-green.json。主分支cherry-pick为0c223ce4+724582b9。仅单元分项与一轮物理绿，另一轮物理红，整体未接受。
- Iroh41在有限故障恢复前2秒中，只有packet.sent>recovery开始且实际新样本较旧质量及上次reset值又低至少25%才重启质量窗口，最多3次；每copy继承packet-send epoch，旧ACK不能刷新新窗口。正常Moving2秒/两fresh/gain与3probe/250ms/2秒预算保持，不新增PING或永久timer。
- Iroh40物理 `turing-20261110T024006Z-fc435ccd`：124/124可达、rc0/原group空，但请求门48/51、双端Go5392/5630ms均红；实际3次预约、>=250ms与<2秒绿。两次probe在outer好路前，旧慢样本使quality median滞后。Iroh40有限timer与sticky copy单元/439/46/9已绿，非物理达标。
- Iroh39物理fa99原请求门51/51绿，却双端Go4060/5799ms；AB后才测BA的旧窗口不能证明双端同时恢复。新scripts/netlab-peer-decision.py在改40/41前对旧source取红并冻结；原scripts/netlab-peer-selection.py和whole物理fixture不动。peer-recovery-decision-four-seconds-red-green.json保留真实红。
- 现代匹配baseline53aa：5a5ed51a/treeace0f8ee/snapshotc3977924/同ef0b，124/124可达但请求41/51、Go7674/7913ms红。旧267/272四秒红继续保留。
- **Noq12 CPU稳定性未闭合**：现代baseline两轮零move/worst539.2/635.3ms；candidate两轮1/2次move、约1s/719ms，第二原at-most-one门红（Relay305s→LAN367s）。四轮均原600秒/20×300ms/22CPU markers逐线程核，不放宽旧门。ack-floor-controlled-stalls-comparison.json。
- Iroh40 quiet原stalls `turing-20261110T030246Z-71aa0502`准确c67365b9/control4eeeea9已采证1142.968s/rc0/原group空；原p1s/p5门绿，一move LAN@183s、zero retired、worst1.3s、10punch、3621packets/1397810B。**不能性能验收**，成本与原始CPU audit待并入比较。诊断021944原p1s/f4s绿但有日志开销、855.6ms/17punch、f4s最长按键10秒，不关闭quiet稳定性红。
- 诊断021944的一个Go实际为当前PTO3/无hint/last_received约2.4秒旧、备用新往返后failure fastleave；尚未证明与quiet同根因，不凭推测改PTO/gain/确认门。
- Noq12 floor真实单元红→绿266/270/271、whole paths tests SHA1d8da16d及multipath.rs SHAfff96ca1不变；新质量只改selection latest_sample九行，不改transport SRTT/PTO/拥塞。旧短stall准备几何纠错与失败均保留。

### N2 IPC与远端内容回复

- caller短期限42729827780实际5绿1红→42733228660 IPC6及便携全绿，整个原测试块SHA10838747；旧a40f抽取勘误保留。
- 原本机32回复42737166916实际barrier32/permit0、caller abort/join后5秒不归还，旧whole wire_tests.rs SHA945c22aa；lease接入首轮42738965522 IPC18绿但32 SIGABRT；00d88f3b只Box::pin大型operation future后42739923461准确76adf825/treeb3af/control4bf原整fixture全绿（0.33秒）。42745451776 Iroh40+IPC整源同分项绿。
- 新独立process层a2bbf866/e15651b7：42752637727仅formatter失败，42753148983实际1006ms预算回收/normal resident join/child reap、IPC18/原32/portable全绿。独立child私有root/空凭据环境/Unix控制socket/60秒绝对lease，caller TCP writer仍开放、barrier仍关闭；不依赖task-local。坏cookie/多余输入无Begin副作用、legacy半关闭兼容。ipc-independent-process-proof.json，详[合同](IPC-PROCESS-ACCEPTANCE.md)。新module使新whole wire_tests hash变化，旧945c原证据仍锚旧source。
- **远端QUIC32真实红→同源两OS绿**：Mac42754548807准确c5ba3d25/tree3e9dad9d/snapshot14aa8637/controlacaa，B实际编码barrier32、A/B permit0、所有caller abort/join后原5秒回收门红；其余portable/IPC18/local32/process绿。17d11fb3只对Resolve/Window/Record的dispatch/回复准备监听SendStream::stopped，输入FIN不取消、Begin/Cancel/写job/Watch保持旧语义；首轮42755165540 test-build E0425未执行行为。757e013f保留resp+frame至最终write后，Mac42755640884远端32为1.53秒、原canonical忙IO/显式ContentCancel整例2.35秒绿；Turing042313-d964同source分别2.37/4.34秒绿，全部便携与本机32/process也绿。
- quic-response-owner-32-red-green.json已逐字比较red与green两个完整fixtures（quic_cancel_tests.rs SHA bb4e8b69、wire_tests.rs SHA4f38e643），candidate_verified仅此层true。红controlacaa→绿ccd只在原新门之后追加已有canonical整例，至原失败门所有命令字节一致；MAC source/11blobs/artifact hash与Linux bundle/control/19artifact/原group空核。Mac/Linux均同snapshot3f0c7ea6，不混cohort。
- 资源回收与真实QUIC已证明，但**实际STOP事件/严格same-wire身份仍需独立观测**，不能单凭permit推导；accepted writes/jobs、跨两middle总预算、真实WAN/phone/energy仍待。本次两Node是同host test process，不扩称跨host取消。

### N4 真跨网

- 263 Mac42721024415/coordinator e0b0b491f8336536/controlebf7f0d，同ff608e91/tree7d0c7467/snapshotb13e4398：Mac↔Turing两端fresh配对身份相符，目录双向30+30全部ok；p50/p95/max分别61004/65659/84881us与59729/61452/71335us。真实公共Relay101.200.80.77:3340，非Direct/peer-relay/正文/冷建连/p99证明。两端joined/owned空/Linux原PGID空/锁释放、临时secret与fresh homes已清。
- 同ff源Mac262与Linux69e7346各stop/EOF/kill真实Node lifecycle3/3绿、无forced cleanup。
- 277 Mac42727910806准确60ce10cf/tree0e8ef193/snapshotb4c490cb/control4eeeea9只build-only；正文29B fixture的read_all加入，MacbinarySHA c377ce6482a71c358eb96c5324bb1104a39bc123b03e6769d548cf283b43edca。global wan-binary-manifests仍旧ff608 cohort，不可与已覆盖Mac cache混用。v2双向各30目录+30正文controller在9992254，尚未实际执行；先Linux同源guard build/完整cohort身份，再跑。
- iPhone可操作/配对方式的可选问题仍待答，服务器独立开发继续；无手机换网/能耗证据。

## 完整剩余范围，不能缩减

| 包 | 必须闭合 |
| --- | --- |
| N0 | 最终目标/容差/样本量/p99充分性、全outcomes、同量测版本baseline/candidate/固定好路/负向校准 |
| N1 | 同source至少3轮双端四秒物理、原CPU稳定性/黑洞/探测成本；冷暖多候选/多middle双向、201/208/244/242慢方向因果与业务；原hop/recovery/dynamic/stalls/full矩阵与162/182历史失败 |
| N2 | 独立process/远端32分项已绿；仍须真实STOP事件/严格same-wire、accepted写job所有权；长多阶段/两middle合计3秒、输者/晚候选、近期地址/重启/jitter合并/quiet epoch/close-vs-offer；写不重放/legacy halfclose |
| N3 | 真实Node/content32已有部分证明，仍须Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；Noq低层704请求不能替代 |
| N4 | U12完整IPv4/IPv6独立/同时/地址族切换与peer移动、Mac↔Turing↔iPhone、冷暖不对称、登记/出口/整机故障域；有数据的布局容量运维方案，非采购部署 |
| N5 | 原48＋Iroh38/39/40/41与Noq12共53补丁逐项覆盖/上游等价/退出，Noq1/5专门缺口；准确最终源可信Quality、Mac原生App/代理/多机/手机/能耗；证据前不升级删补丁 |

## 执行与证据规矩

- 本机source-only，禁Cargo/Rust/Node/Xcode产品编译、测试、formatter及下载可执行物。Git/Python stdlib、数据分析与纯stub控制合同允许。不改installed policy/ensure-budget/已装App/生产store/其他worktree。
- Turing owned root `/tmp/khor-netlab-feat-net-1110`、uid1110，单flock重型槽，Rust1.95/jobs16/nice10，50GiB准入/40GiB停/90min。仅fresh user/netns，无sudo/宿主网络更改，计时不夹编译或其他重负载。
- Mac仍existing run-build→build_guard.py单槽/jobs2，cache/build.lock、50/40GiB与100GiB target门，SSH也须走guard，不改/绕过，自有resident不得脱离guard生命周期。
- SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking=yes，不转个人key/token/agent/签名/模型凭据，不重设HOME/CODEX_HOME。GitHub本机SSH源上传不改变worker认证。
- 不确定timeout先查receipt/GH/proc/PGID，旧controller已结束与未到dispatch有实证后才恢复上传；已有bundle/run则先recover、不重发。Turing stage/launch显式完整control与source，前驱终态采证/原group空/锁与inputs核。
- collect返回101/2可能是采到产品红，读collection分类。verify-mac-record.py只核source/control/11blobs/hash，实际绿须读日志，fmt/编译/fixture/前置/执行器/行为分开。
- graph first；worktree无CodeGraph不自建，code-review-graph可能滞后，先图谱再定向读缺口。无子智能体授权。
- 旧baseline/control worktrees WIP保留不reset；仅终态、所有权/hash留证后清理自有临时资料，不删共享refs。

证据根 `/Volumes/disk0/Khor/evidence/net`。source-refs.json当前305项（后续逐条pin），全量审计最近287项/25ops；待稳定后刷新全量。ops/stage-launch-turing.py只stage一次/launch一次，不自动等待或重试；当前物理跑完先collect。下一Mac新process结果、Turing两个冻结四秒门、真实probe/quality epoch、再按实际失败定位。继续完整N0–N5，不因单项绿/goal服务paused停止。

最新验证：source-refs-audit-20261110-306.json已核306refs/tree/control与30ops hash，此后逐条pin；当前有新增Turing证据通用identity校验器（不分类行为）及probe数据checker。8个pure Python预算正负样例绿。fixture/测量门/旧失败不覆盖。

下一：collect两端当前canonical WAN build并核manifest；建立新cohort后同版本lifecycle stop/EOF/kill、再v2跨网30目录+30正文每方向。Mac不编译/运行本机下载binary，Turing独占heavy guard。N1先cold-start与copy观测，绝不把单次绿当最优策略。完整N0–N5继续。

N4新增事实（2026-11-10 04:51UTC）：正文/目录真实双向各30，worker elapsed_us最近秩p50/p95/max：Mac→Linux目录55382/58815/76133、正文61322/64630/66470；Linux→Mac目录59527/64622/65616、正文55671/58949/59727。caller/controller数据另存proof（约63–73ms p50/p95），全120outcomes无过滤。不是与旧ff608的匹配性能比较，也不是专门cold/p99。coordinator旧ledger registration误标directory_v1，实际两端records均directory_and_content_v2；原label保留并artifact_measurement_revision/erratum补正，ops未来label已修，旧脚本留证，不重跑已实证业务。
