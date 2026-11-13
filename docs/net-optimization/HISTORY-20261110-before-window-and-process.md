# feat/net 当前状态与恢复入口

更新：2026-11-10 02:04 UTC。root串行，无子智能体。完整N0–N5尚未完成，用户已授权继续；不因goal服务paused或阶段通过停止。速度/体验优先，可信个人设备网；无main合并、发布、中继采购/生产部署授权。

重写前逐字状态在[HISTORY-20261110-before-ipc-lease-integration](HISTORY-20261110-before-ipc-lease-integration.md)及其历史链；旧失败与时间参照勘误均保留。计划/硬要求/验收以[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)为准。

## 当前句柄与接续

- 先核Git/receipt/GH/真实PID再动作，禁止盲重发。产品功能HEAD00d88f3b：Iroh39已实现，IPC v2双协议/EndpointFile能力/Target原子刷新/服务端租约也已接入开发源，仍待实际32资源绿；未部署生产。控制HEAD4bf7e6eb（已推），net-node含IPC所有18项及真实32回复用例。共51 vendor（Iroh39新候选）。
- **Turing唯一重型任务是第一轮Iroh39真实移动候选turing-20261110T015757Z-fa99fcb7，PID1487754，02:00:43UTC已启动**，准确15f45e6b/tree144025c2/snapshotba7bcefb/controlef0b；它刻意隔离在新IPC v2接入前，不冒充最终分支整源。先采证、原group空、独立calibration+冻结四秒checker与实际probe预算再判定；须同源至少3轮，当前尚无候选物理绿。
- **现代恢复匹配baseline已可达绿/四秒红**：turing-20261110T015157Z-53aa567e准确5a5ed51a/treeace0f8ee/snapshotc3977924/同ef0b，124/124/最终hold10秒与独立calibration绿、终态rc0/原group空；四秒后51请求41通过/10仍Relay，inner Go+7674/+7913ms。与ba7候选全部Rust diff只有Iroh39实现和6个单元，物理fixture不改。隔离控制worktree khor-net-recovery-control detached5a5ed51a干净，source JSON/patch已留证。
- **Noq12现代CPU对照四轮已全采证，存在未解稳定性红**：两baseline 000645-a87d2f92/010548-d234b36c原p1s/p5绿、零move，worst539.2/635.3ms、punch13/9；两candidate 003612-b78c04a7/012832-5e8674ab分别1/2次move，worst约1s/719.0ms。第二candidate原at-most-one门实际红（Relay305s→LAN367s），p5绿、32/32请求ok/零retired/零network changes，1191.007s/rc101/原group空。四轮均原600秒/20×300ms与22 CPU markers逐线程亲和核，全部Rust diff仅Noq12质量9行。ack-floor-controlled-stalls-comparison.json保留全结果，不能性能/稳定性验收，也不放宽旧门。下一用现成netlab-diagnostic保留原p1s窗口+f4s取完整统计，仅诊断不作quiet SLO；local stage allowlist已补现有后端支持的diagnostic两项，旧ops副本保存。
- **IPC caller40ms实际红→绿分项不变**：42729827780旧IPC5+1红→42733228660准确3ad3851a/controlce02 IPC6与便携全绿，整个原测试块原始SHA10838747一致。先前a40fbc31抽取法未复现，旧hash/勘误保留。只证明client；新lease详[IPC-LEASE-DESIGN](IPC-LEASE-DESIGN.md)。
- **真实32回复新红/候选栈溢出均保留**：42737166916准确e80fa2c1/treeb6d853dc/snapshotf466d990/control4bf7e6eb，fmt/IPC14/portable全绿，实际32编码回复barrier全进入/permit0、所有caller abort/join后5秒内仍未归还，真实行为红。whole wire_tests.rs SHA945c22aa冻结。85403344接入lease，494e5241追加四项真实TCP兼容合同和前次formatter；42738965522准确1b22a3b0/tree8afa1dc2/control4bf，fmt/IPC18/net67/hop12/wires11/journal30/budget20全绿，但原32用例worker stack overflow/SIGABRT，非资源绿。00d88f3b只Box::pin大型run_handoff_within future，保持所有权/取消和whole fixture；Mac42739923461/requestc98e79a4b9f54691a88bd6f2b1c8dc43/source76adf825/treeb3af5eb0/control4bf在途，先采证分类。
- **此前执行器与格式失败单列**：42736367611 attempt1源blob JSON截断（尚无源码导入/driver），确认终态后重跑同run attempt2仅格式失败；两份partial/trace分开保留，attempt2 source/11blobs/hash核。42738040751也仅格式失败，精确patch后续494已应用；原资料未覆盖、不当业务红。
- **Iroh39 vendor/portable已真绿，物理仍待**：42734017374准确ba7bcefb/tree144025c2/controlce02 fmt/Noq439/paths42/home9（含6个新预算单元）；42734610668同完整source/control9992254 portable67/12/11/30/20/IPC6全绿。source/control/11blobs/所有artifact hash已核。下一仍须原peer物理至少3轮/冻结4秒门/实际新增probe计数、原stalls与黑洞成本，不能把单元绿当四秒达标。
- **恢复缺新往返已实证**：turing-20261110T002954Z-cabd9725准确3013038a/tree4e9a79a3/snapshot8f483819/controlef0b，124/124、终态rc0/原group空，70条实际conn/path快照均PTO0/unacked None、received>judged而sample.sent<=judged。outer Go+1187/+1253ms，inner考虑+5302/+5313ms、Go+7552/+7562ms。不是SILENT_HOLD，不能弱化sent>judged。Mac275(42726616050)同source fmt/Noq439/iroh36/home9绿。候选最多3个PING申请的策略与限制已在ACCEPTANCE实施前修订；四秒checker/原物理fixture不动。
- **四秒门已有真实红**：267/272原数据，warm deadline后51次分别42/40选对、9/11仍Relay。scripts/netlab-peer-selection.py与whole physical hash冻结在peer-recovery-four-seconds-red-green.json；至少3独立physical重复。旧可达/rehold10秒绿不能替代此门。
- **Noq12单元/匹配fixture已真红绿**：266新floor单元438绿1红；268新floor绿但旧shortstall准备几何不一致红，原失败保留。270旧质量+真正20ms RTT fixture（snapshot9c8fa9d4）仅floor红；271新质量+同fixture（a42b5b64）fmt/439/36/9全绿；whole paths tests与whole multipath.rs分别SHA1d8da16d/fff96ca1相同，全部Rust diff仅selection样本9行。267/272 physical124/124但恢复仍慢。273同a42 portable67/12/11/30/20绿。Noq12不改transport SRTT/PTO/拥塞/25%收益门。
- **严格NAT门持续化已实际绿**：269 turing-20261109T235237Z-bb957183准确3afd69c1/tree3386599f/snapshot62dda02e/control4eeeea9，124/124，warm30/30实际新Direct、零mismatch、原group空；新report与冻结独立checker每request/problems一致。Iroh38的247红→249绿与whole rebind.rs SHAac977269不变。
- **N4正文只编译，尚未真实跨网**：277 Mac42727910806准确60ce10cf/tree0e8ef193/snapshotb4c490cb/control4eeeea9 build-only绿；新Mac worker SHA c377ce6482a71c358eb96c5324bb1104a39bc123b03e6769d548cf283b43edca、182600176B，read_content逐字核29B fixture。276仅格式失败。旧global wan-binary-manifests仍ff608 source，不能与已覆盖Mac cache混用；待Linux同b4c source受guard构建，再更新完整cohort/两端controller并真实正文验收。

## 已有真跨网与设备事实

A=macserver、B=Turing天然异网，P=iPhone，不再询问另找两电脑。用户“再试试”后的SSH确曾成功，随后多次banner timeout；最近01:03UTC重试仍timeout。这是iroh-ssh控制通道事实，不是Khor数据面故障。GH受guard通道可用。

- 263 Mac42721024415/coordinator e0b0b491f8336536/controlebf7f0d，同ff608e91/tree7d0c7467/snapshotb13e4398：实际两端fresh配对身份相符，双向目录30+30全部ok；Mac→Turing p50/p95/max61004/65659/84881us，反向59729/61452/71335us。真实公共Relay101.200.80.77:3340，非Direct/peer-relay/正文/冷建连/p99证明。两端joined/owned空/Linux原PGID空/锁释放，临时secret删除与fresh身份复制binary已清理。wan-first-paired-proof.json。
- Mac262与Linux69e7346各stop/EOF/kill三种真实Node lifecycle3/3，resident真正join、原group空、无forced cleanup，准确ff608源。Linux旧3套fresh homes在原flock下核owner/hash后已清除，不动cache/target。
- iPhone可操作/配对方式的可选async问题待答，服务器开发继续。未做iPhone换网/能耗，不冒称完成。

## N2进度边界

237blocked writer真实红→239Mac/240Linux同整fixture绿；189 stale owner红→193/194绿；178共享消费者红→179/180绿；F8/F9/F10已有分项真红绿。IPC caller短期限实际红绿见上；lease framing/within/Target与server实际接入已写，源码声明max2向下兼容能力，真实32资源尚待绿。正常legacy半关闭不能作取消，不因client结束宣称server/remote停止。

## 完整剩余范围，不得缩减

| 工作包 | 仍必须完成 |
| --- | --- |
| N0 | 最终目标/容差/样本量与p99充分性、全部outcomes、同版本baseline/candidate/固定好路和负向校准；不混旧量测版本 |
| N1 | 冷暖所有候选/多middle/双向、201/208/244/242慢方向因果与恢复后真实业务，新鲜/稀疏确认、探测成本、原hop/recovery/dynamic/stalls/full矩阵与162/182历史失败 |
| N2 | IPC剩余预算/取消贯通、长多阶段/两middle合计3秒、输者与晚候选、近期地址/重启/jitter合并/quiet epoch/close-vs-offer；写不重放/正常half-close不取消 |
| N3 | 真实Node32席位/content32、Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；Noq FIFO_v2低层704请求不能替代 |
| N4 | U12完整移动矩阵、Mac↔Turing↔iPhone真跨网、冷暖/双端不对称、登记/出口/整机故障域、布局容量运维；不是采购部署 |
| N5 | 原48＋新增Iroh38/39和Noq12，共51 vendor补丁（Iroh39候选）逐项覆盖/上游等价/退出、Noq1/5缺口、准确最终源可信Quality、Mac原生App/代理/多机/手机/能耗；证据前不升级/删补丁 |

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


继续完整N0–N5。最终源可信Quality、Mac App/多机/iPhone/能耗以及上表缺口仍须闭合。

证据新增：source-refs-audit-20261110-287.json逐核287源refs/tree/control与25个ops hash；后续逐条pin，待终态再全量更新。register-mac-run.py仅锚GH实际控制HEAD/源码/refs，不分类行为。当前CPU重复比较在ack-floor-controlled-stalls-comparison.json。继续N0–N5，不在阶段绿后停工。

N2下一仍需独立process/真实server收到的预算、caller EOF/额外输入/坏cookie无副作用、IO未完成时owner留存、QUIC远端执行停止及32远端回复回收。当前Mac18项只含独立任务的TCP预算，不是独立process证明。N1下一保留4秒及原stalls两门，原日志定位后再改策略，不凭推测调高收益门或缩短确认。


最新接续（2026-11-10 03:20UTC，未完成）：主feat/net已cherry-pick Iroh40为7905f2b1，52 vendor（Iroh40+Noq12）；isolated worktree khor-net-recovery-deadlines detached335b8196另有Iroh41候选，尚未带回主分支。Iroh40 Mac42743081799准确c67365b9/4bf实际fmt/439/46/9绿并核11blobs；Primary整体便携Mac42745451776（request0c70556d20c34759830a2fa75e332dd1/source df20a618/tree3f76fc00）已提交，当前GH TLS失败只影响读状态，禁止重发该run。Iroh41 vendor提交在snapshot上传git push180s超时，requesteb6486e71642447eac4d58e6879f8d4a/snapshot2b4c25c1/tree0c3da38b，调用栈未到workflow dispatch，无实际run ID；先核远端ref/部分receipt再按同canonical source恢复，不能伪记测试红。

Iroh40物理fc435ccd已终态rc0/原group空、124/124可达，但原请求门48/51、双端决策5392/5630ms均红；3次预约、>=250ms与<2秒预算真实绿。旧Iroh39 fa99原请求51/51绿却双端Go4060/5799ms，已补新的独立双方决策四秒门（scripts/netlab-peer-decision.py），原checker/物理文件不改，决策checker在改40/41前已对旧source取红，peer-recovery-decision-four-seconds-red-green.json。不能把顺序AB/BA窗口当双端同时恢复证明。下一41只对有限故障恢复中的真实新packet显著下降重启质量窗口，每次需较前质量/前重启值再低25%、最多3次/2秒，携带packet-send epoch防旧ACK刷新新窗口；原Moving2秒/两fresh/收益与probe预算保持。4新状态测试已写，尚未远端通过。

**Turing唯一重型任务当前为turing-20261110T030246Z-71aa0502，PID1562855，03:09:54UTC启动**：netlab-stalls，准确c67365b9/Iroh40隔离源、control4eeeea9，原quiet600秒/20尖峰+p5，不夹其他重负载。前驱021944-2f23786e fullsamples诊断已终态rc0/原group空：p1s一move/855.6ms/17punch，f4s原门绿但最慢按键10秒；它有日志开销，不能关闭quiet012832两move稳定性红。诊断Go展示无当前hint、PTO3、当前last_received约2.4秒旧、备用新往返后快速离开；尚未证明与quiet同一根因，不凭推测改PTO/gain或旧门。

**IPC真实本机资源分项已闭合红→绿**：42739923461准确76adf825/treeb3af5eb0/control4bf实际fmt/IPC18/原32回归1/portable全绿，32回归0.33s，source/11blobs/hash核；整个wire_tests.rs仍SHA945c22aa，与427371原红相同。此前427389的worker stack overflow保留；00d88f3b Box::pin大型operation future后绿。独立process层只新增已提交IPC-PROCESS-ACCEPTANCE.md合同，尚未实现，不能把独立任务预算当跨process/远端QUIC取消。下一仍须原N0–N5全部未闭合项，不能停在这些分项。
