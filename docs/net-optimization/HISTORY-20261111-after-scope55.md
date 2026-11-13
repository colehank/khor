# feat/net 当前状态与恢复入口

更新：2026-11-10 18:23 UTC（新加坡 11 月 11 日凌晨）。本页只记录当前状态；全部此前段落逐字保存在 [历史](HISTORY-20261111-before-current-state-compaction.md)，历史内部还有更早链。不得删除历史失败、用新绿覆盖旧红，或将历史中的运行句柄误当当前。

## 授权、范围与工作位置

完整 **N0–N5 尚未完成，用户已要求持续实施**；不能因分项绿、compact、状态问句或旧 goal 服务 paused 停工。root 串行，无子智能体。速度和实际体验优先，个人可信设备网；peer relay 与 IPv4/IPv6 随时变址是硬范围。没有 main 合并、发布、采购、生产部署或改已装 App/config/store/resident 的授权。

- 产品：`/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`。先核实际 HEAD/status/diff，不按文档覆盖 WIP。
- 控制：`/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，当前固定 `68cadb57a0ed72d40a4ca9a843e86b9c6797bf34`；两处既有 untracked `__pycache__` 保留。
- N1 隔离：`/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines`，detached `2ef6bee5a5aaf841af066f4edb5c894769d4539d`，clean，未接受。Iroh42–44 在该树预留，未合主。
- 证据根：`/Volumes/disk0/Khor/evidence/net`，ops 在其 `ops/`。`source-refs.json` 单写者，实际 receipt/ref/tree/control 优先于段落。
- 规划及验收：[PLAN](PLAN.md)、[REQUIREMENTS](REQUIREMENTS.md)、[ACCEPTANCE](ACCEPTANCE.md)、[NEXT-DESIGN](NEXT-DESIGN.md)、[REPORT](REPORT.md)、[VENDOR-AUDIT](VENDOR-AUDIT.md)、[本批审阅](REVIEW-20261111.md)。

## 最新已验证分项

最新分项 **48947721c339c4ac75a15c4f2dd609bff2f38773**，tree **5b55b4c550c3bd1836e39d5a98588d63c3eb3d98**，snapshot **f78b389a051c5f7091815bddb6738279d97648b8**。

- Linux95f8da6b全12/hop52/wires15/budget30/resume1，313.106秒/21files/group空/identity与实际summaries核；Mac42878663761同canonical全12/hop52/wires15/budget26/resume1，exact13792758066/20files/11control blobs核。
- 只闭合unused prepared carrier在原两5秒观察后释放、有限pending期间保留/结束后释放，以及真实incoming-only第一腿→实际新Tunnel→carried TLS/echo。closed incoming负控/此前48全过。unused与incoming两个freeze仅scope true/field false，旧8196d388 48/2、6da35a6a 51/1红及fmt尝试保留。
- **新鲜度仍有限制**：incoming helper接纳原Patience内或自上次tick字节数变化，后者不提供准确收包时间，进程停顿后不能等同刚收包。尚未单独验证入站live对两条低RTT缓存的排序；网络epochs/NoRelay未知第二腿/所有入站登记竞争/N1–N5未闭合，不称整包接受。

下列为之前已验证分项及完整旧证据，不替代上面最新source：


**`801dff9e71b610a205fb6253a8502ae8182e0b15`**，tree `9ba7af025b8856d19a537bcd54696ea97c08192b`，snapshot `7bbffc722697f90feb4f8756979520cae9e2846d`。

- Linux `turing-20261110T181408Z-aa81601a`：全12组，hop48/wires15/budget30/resume1；278.840秒、21files、group空、identity及实际summaries核。
- Mac `42873590614`：同canonical全12组，hop48/wires15/budget26/resume1；exact artifact `13790762470`、20files、11control blobs核。目录 `mac-42873590614-hop-withdrawal-two-owners-v1`。
- 只闭合两个有限pending消费者合同：initial PendingHop::drop撤回与新pending准入同一决定；normal watcher在同admission锁内检查新pending/live使用并复核当前road/all-live comparison，一致才撤自己exact serial，否则原两LOOK从0重看。无新生产task/timer/延长预算。无消费者仍释放的两个控制也过。
- `pending-drop-fixture-freeze.json` / `normal-release-fixture-freeze.json` candidate true / field false；更广无pending入站登记、unused carrier生命周期和网络epochs未闭合。原9bb7adf7 45/1、74d8633c 47/1红保留。c188/61511a4c与Mac42872979659仅格式，后者exact13789109409/9files/11control blobs核，均无行为结果。
- vendor代码与下列153分项相同，仅账本文档更新；没有为无代码变化重复跑vendor。图谱+定向diff审阅仍标剩余边界，不能称整包完成。

### 上一cold stale-IP分项

**`153263c2df33d73f652a4d42691fba0307f873d4`**，tree `ee002acdb61286aec395dae2a54cef2418e18566`，snapshot `f4c5f39034e9493cba57bc942f134fc7415819f5`。

- Linux `turing-20261110T175147Z-8adb26d3`：全12组，hop44/wires15/budget30/resume1；256.669秒、21files、group空、identity与实际summaries核。
- Mac `42869844599`：同canonical，全12组，hop44/wires15/budget26/resume1；exact artifact `13790105312`、20files、11control blobs核。证据目录 `mac-42869844599-cold-ip-hint-recovery-v1`。
- 同canonical vendor Linux `turing-20261110T174726Z-1e1264ef`：Noq439/path50/home11绿；43.738秒、14files、group空、identity核。
- 实测范围：原地址指向错误身份，首次真实 client Refused 之后 trusted middle 才许可；peer relay 在原2秒 caller 内读 Sessions 一次。NO_ROUTE 时无业务请求；真实已有 authenticated wire 拒绝 hint 撤回且继续收发。较新 IP 来源保留，Relay/Custom不清。
- 实现：Iroh45 actor执行时只有无live连接才撤回所有来源时间均<=首次失败边界的IP hint，必要时清旧selected IP；Node只有未可能发early业务数据、无既有carrier且首次Refused时，在原绝对dial window内等现有hop并补至多一次普通握手。不放松身份验证/写重试/Asleep/总期限。
- `early-fallback-fixture-freeze.json` 与 `cold-ip-hint-fixture-freeze.json` 仅该scope true、field false。**不等于逻辑地址/network epoch闭合、手机问题解决、整包或完整计划接受。** 主vendor总54（Iroh1–41/45、Noq1–12）；45已有该分项验证，42–44隔离未接受。

此前已闭合的窄范围仍保留：sole-peer Recovery准入、慢middle不挡快普通拨号、有限pending lease、cancelled watcher successor、live middle先于两条旧低RTT缓存、cold/resumed读取并行；以及取消/响应owner/STOP/peer contact/actor conditional carried close。具体红绿源、双OS回执与限制见历史及对应freeze，不能升级成全部入口/所有代际已安全。

## 当前任务与下一步

19:17 UTC当前：923baaeb/treee8893d45/snapshot37d77b42，Turing828ae604（PID2760606，19:13:45启动）实际hop55/wires16已全绿，全12其余组收尾；Mac42883511676同canonical已提交，request aa8d071e32c144fd8ec3fff23233986e，尚未接受。bd43f055仅fmt43.791秒/10files/group空/identity核，已用远端patch修正。当前无source WIP，除下方文档更新。

最新只读source refs审计source-refs-audit-20261110T191453Z.json：457 refs/snapshot source trees、64control labels、363显式receipt存在，94legacy缺字段原样保留；ledger SHA前后不变，当时仅828 pending。不是runtime/artifact全审。

下方旧任务均已完成，不重发；以上当前828与Mac句柄优先。


19:10 UTC：7bb0e664已终态采核，hop55全绿（3项epoch均过）但wires16中15/1，只有过期packet仍被计数当新包失败；fresh无需tick、原15、实际wire仍开与cleanup过。196.500秒/12files/group空/identity核，packet-age freeze baseline true；epoch候选只有Linux分组绿，全套仍红，不接受。

当前freshness产品候选改为：原Patience内当前selected IP/Relay路径Noq last_received的实际年龄，Unknown/Custom/缺timestamp不猜；同一显式now快照，不依赖节点tick新鲜度或计数变化。无新vendor/API/探针/task/timer，未验证。forgive_all额外调用者egress.saw只在本机出口状态变化时调用，确认这是routing-wait context更新，不是每个普通ACK失效。

下方7bb已完成；新候选待登记，恢复看ledger。


当前唯一运行baseline `turing-20261110T190521Z-7bb0e664`，PID2753086，19:05:43 UTC启动；17333cb3/tree04a5e998/snapshotc8cebad0，JSON epoch-candidate-packet-age-formatted-baseline-source.json。兼测MISSED弱epoch候选与incoming计数旧年龄逻辑；已过fmt，尚在编译/测试，不先接受。713f5795仅fmt43.754秒/10files/group空/identity核，远端patch已应用。下方旧run均已完成，不重发。


19:03 UTC WIP：31ca9aed仅fmt45.842秒/10files/group空/identity核，已应用远端epoch锁格式。新增incoming packet-age真实握手/echo + 显式now/future快照年龄合同；仅回拨节点cached heard，真实Noq last_received未改，不称真实OS暂停。incoming生产谓词仍旧，只抽显式clock边界；原Wires tests whole tail逐byte保留。下一baseline兼测epoch候选与packet-age旧行为，两个scope尚未接受；查ledger新句柄，不重发旧任务。


最新覆盖（18:57 UTC）：98b997de已终态采核55项53/2，网络标记在真实NO_ROUTE前/后两个正向均失败；无变化原冷却和此前52过，200.482秒/11files/group空/identity核，hop-miss freeze baseline true。当前候选用Wires持有Arc<()> epoch、操作/cache只存Weak；forgive_all原入口替换token，open_hop首await前捕获，MISSED同锁校验epoch才写/清、读取只用当前epoch。仍每(me,to)一项、不按换网增加key，不留Node活；原纯退避测试通过tokenless入口调用同生产核心，不改原测试body。未测不接受。只本机all-waits-forgiven/MISSED，LOST/peer与middle地址context仍未完成。

下方98句柄已完成，新的candidate回执待登记，查ledger。


**当前活动：hop-miss旧行为baseline** `turing-20261110T185034Z-98b997de`，PID2740089，18:50:56 UTC启动；6f611cf2/tree4356aa33/snapshot2bae21a5（hop-miss-epoch-baseline-source.json），新增3项预期2正向红/无变化控制绿，未测不猜。MISSED/LOST产品不变。除该run外，之前Mac/Turing任务均终态采核。先取实际结果再设计epoch，不重发旧52 candidate。

此前运行段落为历史更新，以上唯一活动句柄与ledger优先：


18:48 UTC：48947721/tree5b55b4c5/snapshotf78b389a，Linux95f8da6b实际hop52已全绿（含incoming准入/closed负控与unused两项），全12其余组仍在收尾；Mac42878663761同canonical已提交，尚未接受这两个scope。新WIP只追加miss_epoch三项真实合同，MISSED/LOST产品不变，尚未编译/跑。实际NO_ROUTE在网络标记前/后完成的新网重试正向 + 无变化原冷却/实际ask计数负控；freeze保留sole_peer旧whole为prefix。勿把新合同混为已测。


18:40 UTC：056dea初候选f301d401仅格式43.816秒/10files/group空/identity核，无行为结论。已应用远端Road构造格式；入站排序在去重后固定一次RTT key，避免排序时读变化中的统计。原fixture不改，新candidate待登记；不要重发已完成6da/f301。


最新覆盖（18:38 UTC）：6da35a6a已终态采核52项51/1；unused两候选与closed-incoming负控、原48均过，只有真实入站first-leg未获准入。196.406秒/11files/group空/identity核，incoming freeze baseline true、candidate false。当前新候选：只读选最近收到过包/原Patience内的live incoming，按RTT遍历，只接纳实际selected Direct/Relay（Unknown/Carried不猜）；仍建立原协议Tunnel而非反向复用。Recovery live分组复用同事实。无UI/暂停信任/单hop/TRIES/原budget或后台task/timer改变；尚未验证。

下方6da句柄已完成，不再运行；新candidate回执待登记，恢复查ledger。


**当前唯一重型任务** `turing-20261110T183321Z-6da35a6a`，PID2723734，18:33:43 UTC启动，net-node；source fd8a6dad/tree4d22178c/snapshotd849a325，JSON `unused-candidate-incoming-middle-formatted-baseline-source.json`。同时测unused候选与incoming-only旧行为，预期后者独立红，尚未接受两个scope。前a16f92b9仅格式43.755秒/10files/group空/identity已核，远端patch已应用，无行为结论。不要重发run。


最新覆盖（18:30 UTC）：unused旧行为8c10db13/tree690fdf65/snapshot016103d4，Linux8196d388已真实50项48/2红，pending期间保留控制过，但无owner和owner结束后都仍留carrier；原48全过，190.355秒/11files/group空/identity核。当前候选仅将normal comparison对所有live连接执行，不再把无outgoing road一律视作Needed；原pending admission/unknown保守/两5秒观察/exact serial仍保留，未测不接受。

另新增incoming-middle旧行为两项合同：真实middle→本机Tunnel及往返字节、只serving而无outgoing slot/无middle公告IP；节点须用已知入站路建立实际新Tunnel，再carried TLS/echo；closed incoming负控。first-leg准入/rank产品仍旧。下一run兼测unused候选与incoming旧行为，不能将预期混合红称整组绿。两个freeze独立留证。


最新WIP（18:26 UTC）：新增unused prepared carrier两项真实合同与cfg(test)只读post-look observer；生产comparison仍旧。先正常publication，再让实际主IP连接关闭；原两次5秒观察后，无owner应释放，新pending应保留且结束后再按原两次观察释放。freeze保留normal-release原whole为prefix，尚未运行/接受。下一baseline回执以ledger为准。


18:23 UTC里程碑时此前任务均已终态采核；之后新任务以上方唯一句柄/ledger为准，不按这条历史重发。核git实际WIP。

下一合同：正常publication完成后，原主连接消失且无人使用的prepared carrier必须释放；有新有限pending或实际incoming carried-only使用者时保留，owner结束后重新判断。不缩LOOK=5秒/两次观察，不加常驻task/timer，先真实旧红再修，仍保留exact serial与原所有连接比较。

随后继续：incoming-only首腿、NoRelay/ToNoRelay缺失提示；MISSED/LOST冷却和IPv4/IPv6/本机与peer/middle网络代际、旧完成事件不能污染新网；accepted写job/晚到loser/N1物理恢复/N3成本/N4/N5。下一合同的具体方案在NEXT-DESIGN新增章节，不是已实施承诺。

新合同旧源/候选/fixture/hash分别留证，不能拿fmt/compile失败当行为红，不能拿分组绿当全量绿。原始initial Drop与normal旧行为回执均已采核，不重复收集覆盖。

## 完整范围的未完成项

| 包 | 当前仍欠的完成条件 |
| --- | --- |
| N0 | 最终体验SLO/容差/采样量冻结，前瞻paired校准与足量p99；失败/超时不得从分布删除 |
| N1 | 隔离42–44物理恢复/quiet成本及原矩阵仍红，不能拿主Node绿覆盖。2ef bf6105为124/124但5bad，X5505.782/Y3496.330ms；quality确认窗重启及cold start仍待解决 |
| N2 | 上述初始Drop、正常release、epochs、各入口与ownership全合同，当前只接受各自已实测scope |
| N3 | Custom1024frames/256KiB公平性，SSH/hop混合、shared/separate 1/3/5的交互/CPU/字节/唤醒与手机能耗 |
| N4 | 实际两腿Relay、唯一middle/断腿/多middle失效；独立IPv4/IPv6变址及Wi-Fi↔蜂窝、peer/middle移动；真实手机自动恢复和拓扑证据。macserver↔Turing已有cohort不独立证明NAT穿透 |
| N5 | 原53+新增45及隔离42–44逐项审计/上游退出证据、Noq1/5缺口、最终Quality/native/proxy/多设备和手机能耗 |

原始独立失败保留：Linux pair setup core transfer Busy `854aa6ce`、此前原resume32 timeout、N1物理时限与cold-start红；后来的同scope绿不自动解释或关闭这些失败。全部详情/回执在历史及ledger。

## 真机事实与当前限制

- MacLab本地电脑与Turing同LAN；构建macserver是另一台，异网来自用户报告，尚非独立NAT验真。
- iPhone localhost 已安装且配对；旧现场经历Wi-Fi→蜂窝→Wi-Fi自动恢复失败，反复关开App后某次经Turing恢复。未独立证明OS force-quit，更不是自动收敛。
- IMG_5608显示蜂窝5GA、hinton Direct29ms而多设备不通。单可达peer在第二腿可达/策略允许时应有界尝试转发，但截图不证第二腿、当时源码或精确故障持续时间。
- **旧手机采样都已结束，候选没有安装。** 现在不启动phone RPC/sampler、不要求用户保持亮屏/切网。旧79样本47ok/22busy/9paused/1error不是新候选SLO。

## 执行与证据规则

本机source-only，不跑Cargo/Rust/Node/Xcode构建、测试、formatter或下载执行。Git/Python标准库与只读元数据核可用。Turing只经既有UID1110/flock/隔离netns/build admission，Rust1.95/jobs16/nice10、50GiB起40停/90min；无sudo/host网络变更。Mac用原单cache/jobs2/50–40GiB/100GiB guard。无重叠重型Turing profile；普通unit可Mac+Linux并行，计时物理netlab不得与Mac重型重叠。SSH不转发agent/credentials。

图谱先行，只作导航；build落后、flows0、tests_for0均不是安全/覆盖证明。避免宽读 `crates/net/src/endpoint.rs` 顶部编译relay bearer token。无需CUA。没有子智能体。

`ops/stage-launch-turing.py` 用已存canonical source JSON、固定control和前一terminal receipt；exec session必须poll完成再依赖其结果。collector只收终态/group空，已有report不重复覆盖；verify只核identity，实际行为必须读原日志。Mac登记source ref后按**精确artifact ID**采集并核11 control blobs。ledger单写者，stage期间不并发写。

最新全source-ref只读审计 `source-refs-audit-20261110T165639Z.json`：429 refs/tree、64 control labels、45ops SHA、335显式receipts存在；94 legacy row缺submitted_receipt原样保留，短9135b6d唯一解析不改旧label。不是runtime/artifact全审。

临时baseline worktrees已clean非force清理：early serial 9b680476保留pin `refs/khor/net-baselines/early-serial-calibrated-v2`；queued817与peer543同样已清理。不要按历史重建/重复删除。所有失败/原fixture/hash/refs保留。

## 对用户的估时口径

16:20 UTC已告知：下一份可评审候选约2–4工程小时（原锚点18:20–20:20 UTC），完整N0–N5含真机/换网/能耗仍3–5工程日量级。原13:00 UTC的4–6小时只指下一批，已向用户解释此前口径不清及新增缺陷造成延后。不得每次compact或状态询问无声重置起点；这些是工作量估计，不是完成保证。18:23 UTC本批可评审候选已双OS全12/hop48闭合其scope，在原预计区间内；完整计划仍未完成，继续执行。
