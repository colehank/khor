# feat/net 当前状态与恢复入口

更新：2026-11-10 03:58 UTC。用户已授权继续完整N0–N5，尚未完成。root串行，无子智能体。旧状态逐字保存在[HISTORY-20261110-before-window-and-process](HISTORY-20261110-before-window-and-process.md)及历史链；计划/硬要求/验收以[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)为准。速度/体验优先，可信个人设备网；无main合并、发布、中继采购/生产部署授权。

## 恢复先核实际句柄

- 主worktree `/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net，功能HEAD724582b9，已接入IPC v2、Iroh40/41，当前53 vendor（Iroh41与Noq12为新增候选）。控制worktree `khor-build-net` feat/net HEAD5f310f43，已推。先核Git/receipt/GH/真实PID，不盲重发。
- **Turing唯一重型任务：turing-20261110T035638Z-c20449ca**，PID1624931，03:56:59UTC启动，原netlab-peer-mobility/controlef0b1ab0；隔离source1514516e/tree94f45efd/snapshotf07db0c9，刻意在IPC v2前仅验证选路。下一collect，再calibration、两个冻结四秒门及实际probe预算；同源至少3独立物理轮尚未成立。
- **Mac独立process首轮42752637727**：requestd5e32279892e42ef92282a62a18d724f/source7d63db6d/treecf2f9662/basea2bbf866/control5f310f43，已dispatch并register，待collect/实际formatter或行为结果。首次HTTPS上传180秒超时且receipt无bundle/run，原controller已结束；核远端相同ref、无已派发run后用本机GitHub SSH恢复同request和同source，worker仍用原API bundle。证据mac-source-resume-d5e32279892e42ef92282a62a18d724f.json。主HEAD26346c86随后移除child HOME覆盖，尚未测试此差异。
- **macserver SSH最新03:54UTC成功**，hostname z13deMac-mini.local；此前banner timeout历史保留，不能当Khor数据面故障。A=macserver/B=Turing天然异网，P=iPhone，不再要求另找两电脑。

## 新闭合与当前红项

### N1 恢复与成本

- Iroh41首轮vendor `turing-20261110T033054Z-ada79942`，准确335b8196/tree0c3da38b/snapshot2b4c25c1/control96fedf90，实际fmt/Noq439绿、paths49绿1红，home未执行；rc101/原group空。新窗口仍按旧changed计数排名，导致新copy新鲜89输给旧85。
- 1514516e只将representative confidence改为当前recent样本数，**整个paths tests SHA5c4da4b4不变**；`turing-20261110T035447Z-0755e480`准确1514516e/tree94f45efd/snapshotf07db0c9/control5f310f43实际fmt/439/50/9全绿，45.799秒/rc0/原group空。recovery-quality-window-vendor-red-green.json。主分支cherry-pick为0c223ce4+724582b9。仅单元分项绿，物理尚待。
- Iroh41在有限故障恢复前2秒中，只有packet.sent>recovery开始且实际新样本较旧质量及上次reset值又低至少25%才重启质量窗口，最多3次；每copy继承packet-send epoch，旧ACK不能刷新新窗口。正常Moving2秒/两fresh/gain与3probe/250ms/2秒预算保持，不新增PING或永久timer。
- Iroh40物理 `turing-20261110T024006Z-fc435ccd`：124/124可达、rc0/原group空，但请求门48/51、双端Go5392/5630ms均红；实际3次预约、>=250ms与<2秒绿。两次probe在outer好路前，旧慢样本使quality median滞后。Iroh40有限timer与sticky copy单元/439/46/9已绿，非物理达标。
- Iroh39物理fa99原请求门51/51绿，却双端Go4060/5799ms；AB后才测BA的旧窗口不能证明双端同时恢复。新scripts/netlab-peer-decision.py在改40/41前对旧source取红并冻结；原scripts/netlab-peer-selection.py和whole物理fixture不动。peer-recovery-decision-four-seconds-red-green.json保留真实红。
- 现代匹配baseline53aa：5a5ed51a/treeace0f8ee/snapshotc3977924/同ef0b，124/124可达但请求41/51、Go7674/7913ms红。旧267/272四秒红继续保留。
- **Noq12 CPU稳定性未闭合**：现代baseline两轮零move/worst539.2/635.3ms；candidate两轮1/2次move、约1s/719ms，第二原at-most-one门红（Relay305s→LAN367s）。四轮均原600秒/20×300ms/22CPU markers逐线程核，不放宽旧门。ack-floor-controlled-stalls-comparison.json。
- Iroh40 quiet原stalls `turing-20261110T030246Z-71aa0502`准确c67365b9/control4eeeea9已采证1142.968s/rc0/原group空；原p1s/p5门绿，一move LAN@183s、zero retired、worst1.3s、10punch、3621packets/1397810B。**不能性能验收**，成本与原始CPU audit待并入比较。诊断021944原p1s/f4s绿但有日志开销、855.6ms/17punch、f4s最长按键10秒，不关闭quiet稳定性红。
- 诊断021944的一个Go实际为当前PTO3/无hint/last_received约2.4秒旧、备用新往返后failure fastleave；尚未证明与quiet同根因，不凭推测改PTO/gain/确认门。
- Noq12 floor真实单元红→绿266/270/271、whole paths tests SHA1d8da16d及multipath.rs SHAfff96ca1不变；新质量只改selection latest_sample九行，不改transport SRTT/PTO/拥塞。旧短stall准备几何纠错与失败均保留。

### N2 IPC

- caller短期限42729827780实际5绿1红→42733228660 IPC6及便携全绿，整个原测试块SHA10838747；旧a40f抽取法不可复现勘误保留。
- 真实32回复42737166916实际barrier32/permit0、caller abort/join后5秒不归还，整个旧wire_tests.rs SHA945c22aa；lease接入首轮42738965522 IPC18绿但32 SIGABRT；00d88f3b只Box::pin大型operation future后42739923461准确76adf825/treeb3af5eb0/control4bf fmt/IPC18/原32/便携全绿（32为0.33秒），旧whole fixture一致，ipc-response-owner-32-red-green.json verified。
- Iroh40+IPC整源42745451776准确7905f2b1/tree3f76fc00/snapshotdf20a618/control4bf已collect/11blobs与source/hash核：fmt/net67+1ignored/hop12/wires11/journal30/budget20/IPC18/原32一项全绿，32为0.32秒。尚无Iroh41最终整源证明。
- 新独立process合同已实现于a2bbf866，详[IPC-PROCESS-ACCEPTANCE](IPC-PROCESS-ACCEPTANCE.md)，首次Mac42752637727待实际执行。child私有root/空凭据环境/继承Unix控制socket/60秒绝对lease，真实Node服务；parent TCP writer保持开放，1秒remaining header应自动释放barrier阻塞的实际回复；还核坏cookie/额外输入无Begin副作用、legacy兼容、resident join与child reap。原32函数不改，新module使新整文件hash不同，不冒称旧整文件一致。只限Mac/Linux本机process，未证明QUIC远端执行停止/写job/远端32buffer。

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
| N2 | 独立process预算与取消、真实QUIC stopped与远端32回收；长多阶段/两middle合计3秒、输者/晚候选、近期地址/重启/jitter合并/quiet epoch/close-vs-offer；写不重放/legacy halfclose |
| N3 | 真实Node/content32、Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；Noq低层704请求不能替代 |
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

最新接续（2026-11-10 04:09UTC）：主功能HEAD70ebdad5，独立process合同已在Mac42753148983准确e15651b7/treea9ab1ab6/snapshot6b1503e3/control5f实绿：回收1006ms，resident joined/child reaped，无caller EOF/manual release，旧32及便携全绿，11blobs/hash核。新真实远端QUIC32合同70ebdad5尚未执行，控制acaa3c3已补同net-node入口，先取实际结果再改生产；详QUIC-CANCEL-ACCEPTANCE.md。

Turing035638-c204 lifecycle124/124/请求51/51绿，但我误选精简日志profile，没有完整Go/probe记录；保留为可达与请求事实，不计双端四秒或同profile比较。已补跑同源fullsamples040407-e3bf：实际首次setup业务10.032秒超时、error1/rc101/原group空，未进入replug，非恢复四秒红；完整失败保留并calibration。当前唯一重型任务是匹配Iroh40旧源fullsamples **turing-20261110T040644Z-3d3ebe28，PID1639567，04:07:05UTC启动**，准确5352a0fc/tree d530/snapshotc673/同ef0b；判断初连问题是否candidate特有后再继续。之后真正验四秒必须task netlab-peer-mobility-samples，不能用生命周期profile，两个checker有依赖：先将原request checker JSON保存peer-selection-contract.json，再运行decision。

quiet Iroh40原stalls已独立核全部report/control/bundle/source/hash、22CPU markers/20尖峰/全部resident线程及两contender均CPU64、32/32 outcomes ok；quiet-stalls-audit.json与ack-floor-controlled-stalls-comparison.json新增followup，仍不能用1move/1.3s对零move539/635ms性能验收。source-refs-audit-20261110-306.json已核306refs/tree/control与30ops hash；此后逐条pin。

最新接续（2026-11-10 04:24UTC，完整计划未完成）：主HEAD757e013f，控制ccd8576已推。Iroh41完整物理ad62734d（1514516e/tree94f45efd/snapshotf07db0c9/ef0b）124/124、原请求51/51、双端Go3815.789/3612.992ms与探测预算7请求/3episodes全绿；但同源repeat2 **34871ed6双端3746.991/7237.649ms，一端四秒红**，仍124/124/请求51/51/预算6请求2episodes绿。不盲刷第三轮、不验收N1；旧真实绿与红均保留。慢Y timeline已留slow-y-recovery-timeline.json：3probe986/1237/1488ms，outer Go1249ms，窗口reset1486ms（220→129），直到5237ms才开始inner Moving，7238ms Go。ACK日志无conn ID，不能把观察到98.8/41/130ms直接归某inner copy；需进一步关联后才改策略。探测budget独立audit以first reservation作RecoveryCheck.begin、fault epoch分开，8个纯Python数据checker正负样例绿，ops/audit-recovery-probes.py。

真实远端QUIC32新合同Mac42754548807准确c5ba3d25/tree3e9dad9d/snapshot14aa8637/controlacaa实红：两端permit0、B编码barrier32后abort/join所有caller，原5秒settled资源门超时，其他portable/IPC18/本机32/process1绿；source/11blobs/hash核。quic-response-owner-32-red-green.json冻结整个新quic_cancel_tests.rs SHA bb4e8b69、整个wire_tests.rs SHA4f38e643。17d11fb3生产候选仅Resolve/Window/Record执行/回复准备select SendStream::stopped；输入FIN不取消、Begin/Cancel/写job/Watch语义保持。Mac42755165540准确17d/tree95e59742/snapshot7ef454fc/controlacaa实际fmt/net67绿，test build E0425（reply_written需resp）未执行行为；757e013f让原response与frame一起存活到写后，fixture两文件原字节不变。

当前同一757e013f/tree2d218d48/snapshot3f0c7ea6 canonical源已同时提交Turing net-node与Mac net-node（待从当前receipt/GH补实际run/pid），控制ccd8576含旧IPC/本机32/process、新QUIC32及原canonical busy-content-wire整例，先实际日志分类与fixture hash。Turing唯一重型槽是这轮net-node，不并跑计时；34871ed6已collect/原group空。下次任何dependent snapshot/提交先等commit hook真正完成，不能把exec session返回当commit完毕。
