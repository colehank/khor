# feat/net 当前状态与恢复入口

## 最新接续点

2026-11-11 04:58 UTC 用户恢复活动目标“完成计划”。上轮仅回复进度，归为无进展。本轮已核对工作树、控制版本、先前验证和执行器，开始下一项原始矩阵。目标面板曾被误作停止条件，现依 PLAN 的 U14 及用户继续指令推进。

当前唯一重型任务为 `turing-20261111T045728Z-3d36cba8`，PID 3326979，04:57:49 UTC 启动。执行原 `netlab-dynamic` 的 p2、p3、p7 三轮，源码 `8cd58f8ea707991f23b10ac05d6c8388a61dd519`，tree `4c4d7be8de2c615c35de9b5ca6f3db0cce47a78c`，snapshot `4e900db337c5c4356cb4b27b0de2880210bcf67f`，control `68cadb57a0ed72d40a4ca9a843e86b9c6797bf34`。独立 macserver 可穿插便携构建，Turing 本机计时保持独占，依据 ACCEPTANCE.md 的开发加速规则。完成后采集并审计实际结果，再执行已登记的原稳定性一轮。矩阵见 `confirmation-copy-identity-original-matrix-plan-v1.json`。

上述候选的协议、路径、选中继单元分别 443、58、11 项通过。同源 Turing `f1537e21` 和 macserver `42915496687` 各 12 组回归已验证。最近固定四轮中候选仍有 7.31 秒恢复长尾。更早的单向停顿、完整两腿 peer relay、IPv6、iPhone 换网和能耗待验。当前并行准备 NoRelay、ToNoRelay 缺失第二腿提示的真实转发回显合同，尚未改该生产分支。

下文为此前工作记录，运行状态以上述接续点及收据为准。

更新：2026-11-10 21:58 UTC（新加坡11月11日凌晨）。当前事实以本页为准。此前STATE逐字保存在[最近历史](HISTORY-20261111-before-carrier-stream-diagnosis.md)，其链含after-scope55及更早历史；失败/旧源/fixture不删。历史运行句柄不当当前。

## 授权与位置

完整N0–N5尚未完成，用户已要求持续实施；不能因分项绿、compact或状态问句停工。root串行，无子智能体。速度/体验优先、个人可信设备网，peer relay与v4/v6随时变址为硬范围。无main合并/发布/采购/生产部署/改已装App、config、store、resident授权。

- 产品 `/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net；先核实际HEAD/status/diff，不按摘要覆盖WIP。
- 控制 `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，feat/net，固定68cadb57a0ed72d40a4ca9a843e86b9c6797bf34；原两个untracked pycache保留。
- 原N1隔离 `/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines`，detached2ef6bee5a5aaf841af066f4edb5c894769d4539d，clean。Iroh42–44仍在隔离，未合主，不覆盖它。
- 证据根 `/Volumes/disk0/Khor/evidence/net`，ops在ops/，source-refs.json单写者；receipt/ref/tree/control优先。
- 完整范围：[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)、[NEXT-DESIGN](NEXT-DESIGN.md)、[REPORT](REPORT.md)、[VENDOR-AUDIT](VENDOR-AUDIT.md)、[审阅](REVIEW-20261111.md)。

## 当前实现候选与接受边界

当前产品候选 **2892a8c0c3cdc016de51203b60c5aaa10f946e50 / tree909ecc440a129f94d9b99cd816f500cc9651c2d0 / snapshotf3a3fc92acb03ce918b76078f00bec6d7c21e233**。含原1c847恢复候选、ef0b流计数诊断及Noq13显式path PING及时确认；Noq13协议scope在33bc9584已443/55/11绿，双OS全12仍在跑、物理未开始。原1c847/tree90f62/snapshot4c18的固定六次N1失败不由新单元绿覆盖。实际HEAD/status另核；证据按准确源，不按文档提交HEAD猜测。

**Iroh46候选仅单元/便携scope通过，物理恢复验收失败，不能当最终方案。** 主vendor55含候选46（Iroh1–41/45/46、Noq1–12），42–44仍隔离。46在已有有限恢复episode，真实post-episode样本重返实测pre-fault质量时仅首次刷新quality窗口；历史值不当新样本/可达/收益证据。原3probe（失败/合并也消耗）、250ms、2秒采样、Moving/gain、新packet epoch与重hold均保持。

- 新whole五合同ae62ab385dae3e2f973895f438027e4f1bd7dfae3b0d61840edd5fc6da5bf170与原50测试尾冻结。旧c775/eec09ec1实际Noq439绿/path53pass2FAIL、原50及3负控过；41.799秒/13files/group空/identity，home未跑。
- 候选5c2e4ff2：45.781秒/14files/group空，Noq439/path55/home11全绿。同fixture/旧尾不变。
- 同canonical LinuxNode dc23e338：315.188秒/21files/group空/identity，全12组/hop55/wires16/budget30/resume1。
- Mac42892759864：同canonical全12组/hop55/wires16/budget26/resume1；exact artifact13799886372，zip30923c273dc616538f86b06811f8187bb9d2ae535bf59cfefe5f1d946c07a166，20files/11control blobs核。已终态success。
- freeze recovery-known-quality-fixture-freeze.json只记录相应scope，full_n1_verified=false/field=false。图谱不覆盖vendor，0结果/旧HEAD不是风险或覆盖证明。

此前923baaeb/treee8893d45/snapshot37d77b42已在Linux828ae604/Mac42883511676全12组/hop55/wires16验证：MISSED原network-wait epoch失效（Wires唯一Arc，操作/cache只Weak，旧完成不能盖新）；incoming实际Noq last_received年龄、当前IP/Relay与原Patience，unknown/custom不猜。后者是快照年龄模型，不是OS暂停/手机证明。详最近历史、两freeze。

更早153263c2 cold旧IP错误身份失败后原2秒内peer relay回退（45 idle actor撤旧source IP，新source/Relay/Custom/live保留）、801dff9e initial Drop/normal release pending owner、48947721 unused正常释放与incoming-only首腿、ordinary及cold/resumed并行、sole Recovery、actor条件关闭等仅各自实测scope，准确双OS源/红绿/fixtures见历史。没有升级成全入口/代际/phone结论。

## 固定物理对照：已完成，N1仍失败

**下列六次均已结束；新的流诊断Node检查见下一节。** 以下6次均终态/采集/identity/group空，source ledger pending=false。Mac重型也全部结束。无第七次盲重跑。

预先登记recovery-known-quality-physical-pair-plan.json，C-B,B-C,C-B共三组；C=1c847229，B=c7750419b272716381d2b650627d5992943d58c2/treef21b4a140903f3c1a1e6a7b8a800c3a52c53bc3e/snapshot7ca5901194a7d5c4823c87e140393deecbcd8949（有同五合同/同显式DEBUG，旧行为）。全部control68cad、原20ms腿、真实middle IPv4 replug、原10秒holds、各124业务、原两个四秒门不改。

| 序/角色/运行尾 | 双端Go（ms） | 原warm门 | 独立decision |
| --- | --- | --- | --- |
| 1 C 201812Z-9cb240b7 | 3855.026 / 4068.484 | 51/51绿 | Y超时红 |
| 2 B 202344Z-f79d5c08 | 3557.596 / 3732.647 | 51/51绿 | 绿 |
| 3 B 202919Z-3266eaea | 3751.098 / 5469.355 | 51/51绿 | Y超时红 |
| 4 C 203321Z-71eab46c | 3851.354 / 3820.801 | 51/51绿 | 绿 |
| 5 C 203817Z-6aa2fe4e | 双端均无Go | 52/52失败 | 红，最终未观察到carried |
| 6 B 204400Z-2135d831 | 3521.250 / 3484.977 | 51/51绿 | 绿 |

744/744业务成功不替代选路恢复。C1绿2红，B2绿1红；样本少且copy数4/6有差异，不作显著性/整体收益/p99归因。总结果known-quality-physical-pairs-through-6.json，partial2–5均保留。首组变慢、第二组Y改善1648.554ms、第三组候选严重红全保留，不能挑中一组报收益。预算全按原3次/250ms/2秒审，实际counter界区分全inner Custom与outer wire/能耗；run5至reads_finished enclosing缺边界保持unknown，不补0。

旧main55/923原固定三轮0aec、1cb、b158为2绿1红，最慢X5514.252ms，372业务均成功；main55-peer-mobility-repeatability-result.json保留。de03/55d1e669是另一个机制诊断（124/124、3432.844/3391.236ms绿），不并成第四轮验收、不归因46。

## 下一步：定位run5真实承载流单向停顿

先用实际绑定carrier的收发进度/外层Noq stats补诊断，源与门冻结后再有限运行，不能第七次原源盲重跑。**流诊断v4通过LinuxNode全12；两次物理诊断均在换址前准备失败，旧计划已停止；v5降低输出后的单次校准通过，策略未改**。以下先列原失败证据，准确当前句柄见本节末。

run5 6aa：quality_since=None、quality_steps0，没有postfault质量刷新或answered，不能把此前质量窗口问题当此轮根因。X两Custom copy的RTT计数48/53不增长但last_received偶尔更新；Y两copy计数48/49及收包时刻停在故障前、PTO增长。两个方向无内层Go，载体约21秒后结束；4预约2episode预算绿。这表明单向数据面仍有缺口，尚不能定因Noq/转发/46。

实际carrier outer快照+6秒四端选IP约41–42ms、连接未关闭，但middle onward质量年龄约3.526秒；0条tuple mismatch。低RTT/未关闭不证明流字节推进，也不证明每包路径。证据run5 failed-recovery-copy-observations.json、recovery-mechanism-events.json、outer-copy-timeline-replug-reference.json及原日志。

已知代码：Node hop.rs::splice用两次tokio::io::copy+select，任一结束结束整hop；正常分支不可因诊断改半关闭/取消/finish语义。Links attach→read_frames/write_frames，后者256KiB BufWriter、队列drain后flush。拟仅显式KHOR_NET_DIAGNOSTICS下观察真实carrier serial、流方向/累计字节、read/write/flush pending与完成；沿原5秒look输出，无新timer/task/探针/正文。需要同时分清X写入、middle前后两流与Y读帧/交给iroh，逐段定位。正常日志关闭时保持原热路径。此诊断已实现/编译；v4与v5及其准确验证边界见本页末，不扩大budget或缩确认时间。

原netlab-dynamic（三轮p2/p3/p7）、netlab-stability（一轮p1/p1j/p1s/p5）仍待；当前先处理N1物理硬红。trusted runner/network.sh都允许stability，ops submit-turing.py本地stage白名单尚漏该现有task，后续若用须准确补本地allowlist，不改control/guard。计时任务与Mac/其他Turing重型不重叠。

N2后续context设计只提议：hop_ahead在picked前被MISSED挡住，新live middle且自身net未变仍可能旧退避。需要先实际NO_ROUTE→新候选可达的真实TLS/echo红；排除RTT/排序/每ACK噪声，旧完成不能覆盖新context。不得在每adopt/正向RPC调用forgive_all来制造重试风暴。LOST还缺本机identity namespace及地址context；尚未新加map/token，详NEXT-DESIGN。

## 全计划仍欠的完成条件

| 包 | 未闭合 |
| --- | --- |
| N0 | 最终体验SLO/容差/足量样本与前瞻校准、p99；失败不删 |
| N1 | 上述固定物理红/单向流停顿、原dynamic/stability/quiet成本及完整矩阵；隔离42–44仍未接受 |
| N2 | 全部admission/地址context、incoming live胜两cached、NoRelay/ToNoRelay未知第二腿、静默黑洞新carrier接管、accepted jobs/late losers、LOST归属 |
| N3 | Custom公平/背压与SSH/hop混合，shared/separate、1/3/5设备，交互/CPU/字节/唤醒与手机能耗 |
| N4 | 真实两腿Relay/唯一middle/断腿/多middle、独立v4/v6/同时变址、Wi-Fi↔蜂窝、自动恢复与拓扑验真 |
| N5 | 全部vendor逐项审计/上游退出，Noq1/5专属缺口，最终Quality/native/proxy/多设备/phone |

独立旧Busy854aa6ce core transfer、原resume32 timeout、CPU quiet/f4s及N1 cold-start红不由后来的分项绿解释或抹掉。

## 真机、执行与证据边界

MacLab/Turing同LAN；构建macserver另一台，异网为用户报告、非独立NAT证据。iPhone localhost旧现场自动Wi-Fi→蜂窝→Wi-Fi恢复失败，反复关开App某次才经Turing恢复；未独立证明OS force-quit。截图5GA下hinton Direct29ms、多peer不通仅证首腿可见，不证第二腿/当时源码/精确故障时长。

旧手机采样已结束、候选未安装；当前不做phone RPC/sampler或要求亮屏/切网。旧79样本47ok/22busy/9paused/1error不作候选SLO。Mac便携网络executor不是native检查，iPhone沿可信GitHub TestFlight；未修改信任/发行边界。当前没有安装、合并或发布。

本机source-only：不跑Cargo/Rust/Node/Xcode构建/测试/formatter/下载执行。Git/Python标准库元数据允许。Turing只走UID1110自有/tmp/khor-netlab-feat-net-1110、flock、user/netns、Rust1.95/jobs16/nice10、50GiB起40停/90min；无sudo/host网络变更。Mac原单cache/jobs2/50–40GiB/100GiB guard保持。普通unit可两端并行，计时netlab不与Mac重型重叠。SSH不转agent/凭据，无子智能体/CUA。

图谱先行，旧HEAD/0flow/0test不是安全证明。避免宽读crates/net/src/endpoint.rs顶部编译Relay bearer。collector只终态/group空，verify仅核identity，须看真实日志与原合同。Mac exactartifact ID、11control blobs核。source ledger单写者；exec session先poll再依赖；stage/launch不在不确定时重发。

refs最近只读审计191453Z为457refs/64control/363explicit receipts+94legacy缺字段，当时828 pending；不是全artifact/runtime审计。新增各run逐一identity实际核；未把旧审计当最新全账。源码cache已保留mtime，同源CLI编译0.53/0.54秒，换源约49秒；完整网络观察窗保留，不擅改jobs/资源门。

旧scope55相对路径freeze错误、collect命令初次从错误cwd查无脚本等metadata错误都未改变源码/门、未重复物理提交；原日志保留。临时worktree非force清理和pins见历史，不重建/删原件。

## 估时与沟通

16:20 UTC原下一候选2–4小时锚18:20–20:20已产出48与55/16双OS窄scope，不等于全计划。19:52说明N1当前修正/复验另需2–4工程小时，完整N0–N5仍3–5工程日量级。20:41用户再问，明确同一窗口已用约50分钟，剩余该批对照5–10分钟；20:50已完成固定六次且如实报告候选失败。不得每次compact无声重置窗口，不许承诺到点就关硬红。完整计划含真机/能耗未完成，持续推进。

21:01 UTC：新增stream_trace诊断94d3241a，只在KHOR_NET_DIAGNOSTICS开启时对三位置actual Noq Send/Recv加Tap，按poll真实read/write/flush/shutdown状态及累计偏移记录（pending重复抑制、无payload）；绑定stream/wire/attachment身份。中间仍原双copy/select/finish，关闭时调用逐字未改的normal splice和原Links.attach/Noq IO类型，无hot-path Tap。原5秒outer look另读实际conn/path stats。写入/flush ready不是ACK/远端交付，read ready0不自动当EOF。新诊断尚未类型/行为验证；c801首check仅fmt失败43.803秒/10files/group空/identity，已apply经check的远端4920byte格式patch，无本机formatter。旧源/失败保留，准备格式后重提Node。

21:04 UTC：db9cb1c3 stream诊断beb35913已终态81.964秒/11files/group空/identity；fmt及net67（原1ignored）绿，Node编译两处E0308：incoming_wire为Option<usize>。已修诊断身份为Option，X真实outer用Some、中间incoming/receiver允许None，不unwrap/不伪造0；缺ID不能作跨端精确关联。尚无Node/物理通过结论。无额外heavy，准备v3准确源。

21:07 UTC当前唯一重型：turing-20261110T210521Z-3e49bad9（net-node诊断v4），PID2895702，21:05:42 UTC启动；fd92b2ac177a4054aad1e96f3ca30fe16d1e75ea/tree45ddec136ba973a32fd0dbae93d3ed590e8b6559/snapshotbaa0800f51609169c6a0d59258ca8002272df31b/control68cad。尚在Node编译/回归，未提交新的物理。v3 5d99仅修Option后未提交，v4只手工换行105字符构造签名，保留独立source JSON。normal splice与1c逐字hash核，diagnostic source audit已存；没有行为通过声明。Mac本轮尚无新任务。

21:15 UTC：流诊断v4 fd92b2ac/tree45ddec13/snapshotbaa0800f在Linux3e49bad9已终态315.140秒/21files/group空/identity核，fmt/net67（原1ignored）及全12组/hop55/wires16/budget30/resume1绿、STOP32/32，正常分支回归通过。Mac流诊断版未单独执行；不作跨OS/性能声明。当前唯一重型 **turing-20261110T211143Z-d1113762**，PID2902304，21:12:05 UTC启动，原physical profile+stream Tap，精确v4/control68cad。新plan carrier-stream-progress-diagnostic-plan-v1.json在首物理前登记，最多3次、0重试，首次重现且定位或3次后停止；全部结果保留，不并入旧六次验收。解析器记录每端wire/stream/方向的真实偏移及last OBSERVED state，pending不当持续阻塞/活任务证据，必须关联结束与对端read。21:15仍running，未采集，不重发。Mac/其他Turing重型无并发。

21:31 UTC：流诊断attempt2 0e2f0e2d已终态194.445秒/16files/group空/identity，和attempt1一样prepare exceeded150s、仅2setup业务ok，无replug。载体正常转发后两look按Outperformed释放，Custom约100–105ms/PublicRelay131–132ms未满足原25%收益；不能定因观察器、Noq或配置。两个失败保留，停止该旧观察器计划于2/最多3次；不继续第三次同源等待准备。准备降低观察器每successfulIO序列化：所有真实字节仍累计并记last-progress时间，只在wait/flush/end等边界输出、重复empty flush抑制。未改Noq/选路/准备期限/门，观察器开销假设仍需新source校准。当前无重型，正在改诊断源码。


21:44 UTC接续：当前没有运行中的重型任务；source ledger全体pending=false（本次逐项核883为completed_verified）。v4物理d1113762与0e2f0e2d分别244.746/194.445秒、各16files/group空/identity，均原prepare超过150秒，仅2次setup业务成功、未执行replug；两次流进度均保留，不能算移动恢复失败重现。两份comparison表明Custom约100–105ms、Relay约130–132ms，未满足原25%收益而释放，已转发对应全部累计字节；不能由此证明日志为因。旧max3计划在两次同类准备失败后显式停止，不盲跑第三次。

v5 ef0bfa7e/tree d635e42951140ed36ca0e41f586bfc9569940cb1/snapshot31e10e49c71361cfe7c87153bf4f3b3bc7668489仅减少成功read/write的同步序列化，累计计数及真实last_read/write_at保留，在pending/flush/error/end输出。关闭诊断原IO类型/正常splice不变。单次校准883d38e1已终态216.656秒/16files/group空/identity：原124/124业务、两个10秒hold、warm51/51及独立decision X3435.866/Y3412.927ms全绿，预约6次/2episodes预算绿。one-run计划已complete，无额外retry。此结果不证明此前准备失败由输出引起、不重现6aa单向停顿、不接受N1；全12Node仍只对v4准确源成立。v5流分析须使用累计read/written与last_read_at/last_write_at，不能用正bytes Ready事件数量（成功IO已延后输出）判断是否推进。

下一步先完成883流偏移及身份/end关联分析，然后用独立协议合同核对显式path PING的采样及时性；此时Noq13只是提议，未写测试/行为、未计数。6aa真实单向停顿仍未闭环，fixed6红全部保留。

21:48 UTC：新增vendor/noq-proto/src/tests/path_probe.rs三项真实虚拟网络合同（whole SHA348a5fc8571e136d7144f18668fe6b779ea3a110b1db7bf7189b6398f79bb1bd），母文件仅新增module声明且可逐字还原，原multipath SHAfff96ca12a2c3ed53ea85e6a4241998cd56573cc873fefed49644c2f8fb81077不变。现先在旧ping_path行为上执行，不称Noq13/6aa根因或策略修复。counter分析883共有8组双向累计偏移单调、全部故障后有推进，最终42752/43072匹配；v2 helper/metadata错误与hash manifest保留，未重跑profile。

21:49 UTC：old54c1/tree4ba70049/snapshotaf88e149在bfe61241终态40.004秒/12files/group空/identity，fmt绿、Noq442项441pass/1FAIL；新explicit path one-RTT合同确实对端收到PING，客户端RTT计数2未增至3，普通delayed ACK与Initial握手控制绿、原439绿；Iroh/home未执行。不是6aa归因。另加真实TLS resumption早期STREAM与path ping兼容控制，whole v2 SHAe0571cbca5d64554d58e9ea960259e8a82a47167df6816d5d7a23e78968ca65b；准备先跑旧行为v2，不改源码策略。IETF ack-frequency-08第3节仅作本地实现所引draft兼容核，不把旧draft当最新标准；支持按min_ack_delay，0RTT禁止这两扩展frame。

21:52 UTC：v2旧efb2/treeb1e95/snapshotef151在8a4a7521终态35.709秒/12files/group空/identity，fmt/原439与三负控（含真实accepted0RTT+实际early STREAM逐字交付）绿，仅explicit probe一RTT时无新样本失败，442pass/1fail。已准备Noq13候选：Established+Data+peer支持时同一次pending PING附IMMEDIATE_ACK，普通ping/保活不动，无新任务/计时器/重试；ACK可能更早额外单包须计量。主vendor候选56，隔离42–44未动；whole v2e057与原multipath/parent还原SHA再核相同。当前未编译，准备原net-vendor执行；不把此独立缺陷当6aa定因。

21:54 UTC：候选4e0/673dd20b终态27.678秒/12files/group空/identity，fmt绿但Noq编译E0308，highest_space实际SpaceKind而非SpaceId；未执行任何行为测试，原源/结果留存。已按声明及既有1079检查修成SpaceKind::Data，测试whole v2e057不变后重提；没有扩大功能/门。

21:57 UTC：Noq13候选2892a8c0c3cdc016de51203b60c5aaa10f946e50/tree909ecc440a129f94d9b99cd816f500cc9651c2d0/snapshotf3a3fc92acb03ce918b76078f00bec6d7c21e233在33bc9584已终态59.799秒/14files/group空/identity，fmt/Noq443/irohpath55/home11全绿；wholetest v2e057旧442/1红→候选443绿。仅协议及时性scope，原6aa/N1/phone仍红或未验。source audit核connection/mod.rs只ping_path变、普通ping/保活与其余全文件逐字一致，原测试SHA不变，图谱不覆盖vendor。

当前唯一Turing重型 **turing-20261110T215546Z-ea5f7a11**（net-node），PID2952660，21:56:07 UTC启动，准确2892/tree909e/f3a/control68cad。Mac同源net-node正经既有dev提交，request f66c4cbd4c4e41a4bdfd8eb61a9ebd96，尚未收回最终run ID，不重提；exec session98379须先poll。待双OS原全12通过且Macheavy结束再做timedphysical。path-probe-prompt-ack-physical-plan-v1.json已前瞻登记C-B,B-C最多4、0重试，原门/hold/预算/observer保持；首真实换址后单向非推进/无Go或连续两次准备失败即停作定位，固定集合不完整仍如实保留。ACKcost v2另存旧helper，加入ACK/PATH_ACK/IMMEDIATE_ACK实际counter，缺界unknown。尚未开始物理。

21:58 UTC：Mac提交已收回run **42909901676**，request f66c、准确2892/tree909e/f3a、control68cad，已注册ledger（484条）；workflow in_progress。先前session98379已终态退出，不再poll/重提。当前仅Turing ea5f Node +Mac429099 Node在跑，无物理/phone任务。

22:03 UTC：准确2892/tree909e/f3a/control68cad在Linux ea5f7a11终态329.181秒/21files/group空/identity，全12/hop55/wires16/budget30/resume1及fmt/net67+原1ignored绿；Mac42909901676终态SUCCESS、exact artifact13806019480、zipSHAee8957540905be46f47000b495ecfaaef64a73b7b54174e40a0fc50ee26a796e、20files/11control blobs核，全12/hop55/wires16/budget26/resume1绿。两个ledger pendingfalse；非native/phone/物理结论。当前唯一重型 **turing-20261110T220247Z-f8d3ff73**，PID2959961，22:03:09 UTC启动，固定C-B,B-C计划首候选、准确2892源/control68cad，物理未结束，无额外Mac重型。

22:16 UTC物理fixed4已完成前两次：候选f8d3ff73终态226.663秒/16files/group空/identity，124/124业务但warm10/51失败，独立X7307.560ms/Y3517.794ms；全8流方向故障后持续推进且单调，无NoGo/oneway stoptrigger。X收到选定probe copy90.352797ms后，另一copy100.024814ms作为代表；原3预约全用尽，跨copy原始观测12条与hash另存。只是原因候选，尚未改代表策略。

第二baseline8f3bfdc7终态224.742秒/16files/group空/identity，原可用性校准脚本在lifecycle前逐字prefix全部检查通过：124/124、10秒hold、replug274ms。随后严格JSON解析失败，x诊断末条hop_stream_io在+31.279秒被截断至null、无结尾；raw不修、不删，不作完整原checker通过。availability-pre-tail-audit与helper/prefixSHA留存；独立完整Go行X5437.441/Y5401.818ms均晚，warm/cost/flow正式合同未知。该次计入原固定4，不补跑。

当前唯一重型第三baseline **turing-20261110T221432Z-fe3f8fcb**，PID2977592，22:14:53 UTC启动，准确old efb2/treeb1e95/snapshotef151/control68cad，前两运行ledger已收口；Mac无重型。第四candidate仍预定，无额外次数。成本v2解析在既有883元数据校准发现首field ACK前导空格，旧helper bytes+修正SHA288630d440eaa8aa1a4097f3f83e0353aa8f238e78e9f5119c22956591fa509f与理由更新plan，883两窗口各4copy完整；该校准不计本固定对照、不改任何物理/门。

22:19 UTC：第三baseline fe3f8fcb已终态168.293秒/16files/group空/identity，原124/124、warm51/51、独立X3386.396/Y3596.237ms全绿，probe/ACKcost/stream各原审计保留。固定集合未完成，不当总体改善。第四candidate提交中（exec session97585须先poll），仍固定2892/tree909e/f3a；当前新加测试不进入该已冻结snapshot。

新source-only独立确认身份合同：vendor/iroh/src/socket/remote_map/remote_state/confirmation_identity_tests.rs三项whole SHA395341c56e7c3024505be5a838d8f1fa298e63066d33de1c237a0793b9a02b43。quality/age选一个代表，但samples_after当前取任意copy的最大新观察数，可能借另一copy确认旧质量；两负控分别0/1代表新样本不应借其他copy的2、正控代表改变后使用新代表自己的4。仅通过真实Reading::observe的有效PathStats输入，不直接写recent/changed，不改生产；原baseline whole ae62与原测试字节不变，无Iroh47。尚未提交远端，第四物理结束后先取旧行为红；不是f8/6aa定因，仍欠实际performance/稳定性验证。

22:19 UTC第四物理已明确启动：**turing-20261110T221818Z-e712a6f2**，PID2984471，22:18:40 UTC，固定最后candidate2892/tree909e/f3a/control68cad；无第五次。当前唯一重型，Mac无heavy。新增确认身份三合同已提交source-only41c2ca987e5ddfbfe4278da88dceb0e047a51bf7/tree6b0262dae35fabd834bac6a317bceb016c1dac90/snapshot6b307dce542ea16b8f804df78a848e6e1f8bfffc，未改变行为、未远端验证；后续old-source-v1 JSON已存，必须等e712结束后再原net-vendor取旧红，不并发计时实验。

22:24 UTC固定四次全部完成，未额外重跑：末候选e712a6f2终态226.594秒/16files/group空/identity，原124/124、warm51/51、独立X3703.047/Y3670.630ms均绿。全集C1绿1红（7.307秒），B1绿、1次独立双向约5.4秒红且尾诊断缺口，496/496业务全成功不替代两四秒门。全部identity、终态与source ledger核，完整集合path-probe-prompt-ack-physical-fixed-four-result.json，Noq13仅协议及时性scope、不接受为N1物理方案、不重现或解决6aa。首对独立原时间差C相对B为X+1870.119/Y-1884.024ms（B正式warm/cost未知）；次对X+316.651/Y+74.393ms，描述值不作显著性/p99。3份完整实际计数各4copies，第二对reads_finished enclosing C353/B313 innerTx datagrams、C47/B16 IMMEDIATE_ACK，负载/切换时序/ACK合并不同，不把差额全归因补丁，不推outer/手机能耗。

当前唯一重型 **turing-20261110T222346Z-b0146293**，PID2992930，22:24:08 UTC启动，原net-vendor旧行为测试，source-only41c2/tree6b0262/snapshot6b307d/control68cad，包含三确认身份合同、原策略未改。待终态取实际红绿；无Mac重型、无phone。19:52 UTC锚原N1 2–4小时窗口仍到23:52，目前已用约2小时半，不保证到点关闭硬红；完整N0–N5原3–5工程日量级保留非承诺，手机/IPv6/能耗欠验。

22:26 UTC旧确认身份 b0146293终态59.867秒/13files/group空/identity、fmt/Noq443绿，iroh56pass/2FAIL：两个旧行为确实以其他copy的2覆盖代表自身0/1，原55及代表改变控制绿，home未跑。修正候选Iroh47只samples_after取同一代表，4处调用（Moving/首恢复/退化/自然停止）统一证据，原window/时限/门/3次预算保留，实际可能使用更多原预算，须量测。全57patch候选、42–44隔离，whole395341与原ae62不变。未编译/物理未验，不作f8或6aa定因。

22:28 UTC当前唯一重型 **turing-20261110T222723Z-c350f32c**，PID2996638，22:27:45 UTC启动，Iroh47候选8cd58f8ea707991f23b10ac05d6c8388a61dd519/tree4c4d7be8de2c615c35de9b5ca6f3db0cce47a78c/snapshot4e900db337c5c4356cb4b27b0de2880210bcf67f/control68cad。尚未终态，当前vendor范围57候选，无额外physical/Mac/phone。

2026-11-11 05:06 UTC 已准备 NoRelay/ToNoRelay 三个缺第二腿提示回归，加已知 Direct 可达及递归拒绝两控制，共五项。真实 hop_ahead、Tunnel、内层 TLS/echo，使用原 Publication 适配中间节点。原 sole_peer 文件仅末尾新增模块，原正文逐字保留。fixture SHA9629a261389cc6152b7938a6e6d02697121e031432f4c0c00a47accb769cb01b，证据 no-relay-missing-hint-fixture-v1.json。生产行为未改，先在独立 macserver 取旧行为结果。Turing 动态 3d36cba8 的 PID/进程组与 netlab 子进程已实核存活，仍在首轮 p7。

2026-11-11 05:08 UTC 原动态首轮9项日志已报通过，p3最慢回显532.3ms/零失败，活跃p7恢复约2.6–2.8秒；第二轮仍在跑，完整终态及三轮审计待采集。同一控制版本的独立Mac run42963573470正在验证NoRelay旧行为，source2cd67b9c/tree01cc7ec7/snapshot23930497，request9153492ce97547bf8a66b2a8a3739d18，ledger已登记。矩阵允许独立Mac构建的说明已纠正，Turing仍独占。

2026-11-11 05:12 UTC NoRelay旧Mac42963573470已终态failure，exact13830014031/zip2adeb885、10files/11control blobs/identity核。fmt/net67（原1ignored）绿；hop60项57pass/3FAIL，三个缺提示组合都未实际offer，原55和已知Direct/recursive控制过。fixture SHA9629...原样保持。候选仅将无目标wire的Recovery判断置于NoRelay/ToNoRelay标志前，复用已有live排序和有限第二腿验证。未新增谓词/任务/预算，等待Mac同整回归。Turing继续跑冻结8cd动态，尚未采终态。
