# feat/net 当前状态与恢复入口

更新：2026-11-10 11:00 UTC。完整N0–N5仍未完成，用户授权继续。速度/体验优先，可信个人设备网；无main合并、发布、采购/生产部署授权。root串行、无子智能体。长期[PLAN](PLAN.md)/[REQUIREMENTS](REQUIREMENTS.md)/[ACCEPTANCE](ACCEPTANCE.md)不缩减。上一状态逐字在[历史](HISTORY-20261110-before-natural-recovery-epoch.md)，保留更早历史链。不要因旧goal服务paused停止。

## 先核真实句柄

- 主 `/Volumes/disk0/Khor/worktrees/current/khor-net`，feat/net。当前候选HEAD **f9ced4ad**：3a3同tick修复上再将Asleep旧退役提交移入持槽阶段；retirement gate仅cfg(test)，主53vendor/Iroh41未动；主53vendor/Iroh41不改，已双OS全net-node绿（Linux61aa、Mac42798075914 attempt2），不是已部署源。Mac42770379226+Linux4bcfad82同source全部net-node已绿，见下。不把doc-only HEAD误当已测试二进制源。
- 隔离 `/Volumes/disk0/Khor/worktrees/current/khor-net-recovery-deadlines` detached **2ef6bee5**，treef1da6579/snapshot48e9e24b（b4行为仅追加actual carrier与connect阶段诊断，e819已实际编译执行）；无IPC v2，Iroh42/43/44未合主。Iroh43实际vendor绿但物理f2b仍红。Iroh44基线d7/tree937/snapshot714在fe3实际54/2红；当前b4只保留有限稀疏测量，重新合格或新故障重启确认时钟，原whole50+epoch+sampling fixtures不变；待同源vendor与物理验证。
- 控制 `/Volumes/disk0/Khor/worktrees/current/khor-build-net` feat/net HEAD **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34**已推。net-node仅在原remoteQUIC32之后/canonical之前加resume合同；3个source-only stub控制测试实际绿。
- Turing当前无本轮heavy，**f9 source0666/snapshot764/control68 的61aa0e2b已全net-node绿**，wires15、全IPC/32STOP/remote32/resume/预算等绿，339.284秒/21artifacts/原group空核。09a原行为14/1实际红、whole原11+wake2+retirement2逐byte同。Mac同source42798075914 attempt1在bootstrap GitHub API截断JSON失败，未还原源/无compile/test，artifact-attempt1与失败证明保留；同一workflow attempt2已全12项绿/wires15，20artifacts/11controlblobs与exact f9 source/tree/snapshot已核。name下载首次选到attempt1旧产物，逐byte确认后另存，后按artifact13754344749精确下载attempt2，双方原件保留；不是产品失败或新调度。request a056239e8193417da93238233b10e77b，所有本地会话已结束。
- Mac前一42793316303同tick source3a3已terminal全绿；当前f9的42798075914 attempt2已同source全绿，无活动Mac job。macserver SSH已恢复；用户已澄清同LAN为MacLab本地电脑↔Turing；构建macserver为另一台。A=macserver/B=Turing沿用U10用户异网说明，C=MacLab同LAN参照，P=iPhone；公共Relay自身不证明NAT穿透。源提交经本机GitHub SSH，worker仍原API bundle/guard，无个人凭据给worker。
- 用户已确认：**iPhone在手边，可切Wi-Fi/蜂窝；已装khor并已与Macserver/Turing配对**。两次可用性/安装配对问题均已答，不重复问。真机采样已结束；Wi-Fi/蜂窝/回Wi-Fi全部outcomes保留。用户明确反复关开App后才某次经Turing peer relay恢复，自动接管失败，未部署候选或验收手机；不再要求保持前台。

## 当前关键因果：稀疏恢复质量中断补测

- f0a实际7bf72701物理原124/124、请求门绿，但Y7404ms。首post-judge合法慢RTT恢复可达，旧mark_silent先删Silence，后recovery_probes漏建epoch；当时无pending/inflight/cwnd阻塞，等待keepalive。Iroh43仅交换联合bookkeeping顺序；新同whole回归9bf实际52/1红→9cf实际Noq441/Iroh53/home9绿，原whole50不改。
- Iroh43 first physical f2b832dc仍红：原124/124但请求11bad，GoX7728.243/Y3469.794ms。X实际PING/ACK/RTT工作；85→112ms两点上中位数撤销收益、清Moving、停止补测，等待keepalive后才重新确认。预算6预约/2episode绿，不是确认总包/能耗证明。旧2c X缺RTT仍独立未证。
- Iroh44设计先以cdf8018d冻结，再d7保持旧clear-on-stay helper建立独立状态合同，fe3cf29e实际fmt/Noq441绿、Iroh54pass/2新正向fail（旧53+新负向绿）、home未执行；56.055秒/13artifacts/sourcecontrolbundle/原group空核。完整whole50 SHA5c4da4b4、epoch3c547c63、samplingefed1fbc冻结。
- b4e4d232最小候选：只允许既有pending候选在原两秒内继续稀疏恢复测量；要求RecoveryCheck存活、open非silent且质量不足三点，仍走原confirmation probe入口。采样不算连续合格时间；再次符合收益或新故障须重新开始正常确认。不改sent>judged/25%gain/两fresh/2秒确认/4秒门/原3预约250ms2秒、不增加强owner/永久timer。先同whole实际vendor绿，再原物理≥3次、quiet/黑洞/实际总包与CPU/能耗；不能预记已修复7.7或旧5.8。

## N1已有证据边界

- Iroh42 8d91e1ec只让candidate confirmation复用recovery_copy_rank（低PTO优先，同PTO恢复copy粘附），current最高PTO规则不动。旧5a诊断Y1549ms取84ms后confirmation切另一copy、101ms代表使Moving撤销，Go7549；另一copy同刻PTO未知，不称同PTO事实。42尚未合主。
- 42 source894：4f2物理124/124、请求绿、X3262ms，Y始终同wire Custom/首stream14ms/首成功762ms；原decision缺Go仍红，补充continuity仅回溯探索绿。repeat2 **2c9d**为124/124但请求门及两端Go5825/5870ms红；X三API Ok后RTT49/1.1646秒不前进，未有实际tx计数；另一copyPTO2/unacked None亦近期收包，不能假设同PTO或直接猜cwnd。原失败完整保留。
- fullcounter v1 source1a/tree80a/snapshotd2，两轮6d83与ff132双端3585/3578与3285/3518ms绿，原124/124/两门/state/6预算2episodes绿；只得正常counter参照，不能倒推旧红已解，不盲刷同一诊断。原wholeNoq multipath、Iroh50tests、物理fixture保持。carried-ping-source-audit/fixture-identity/counters/reference留证。
- sourcef0a仅Noq3 PathStats扩bytes_in_flight/ping_pending/pending_other_frames（global pending不等于此path承载业务），两模型补API排队/tx前后状态；selector不读新fields、CC/PTO/发包/RTT数学不改。Turing505de602实际fmt/441/50/9绿，61.836秒/14artifacts/原group空核。提取器v2要求send_state测量三字段存在，旧版备份保留。上节7bf据此定位新的Y分支。
- Noq两个真实模型：健康backup确实PING发出/本路ACK/新RTT，黑洞API/tx不假刷新RTT；Mac427650+Linuxf1d19b07同1a完整441/50/9绿、13/14artifacts核。普通ping不等于PTO到期，不能为四秒绕过RFC9002拥塞边界；primary links在NEXT-DESIGN。
- **小补丁quiet匹配一对**：42候选da82e2b8 source894与41基线f9d50e90 source829，实际仅一Rust tie规则+KHOR记录不同，同control4eeee/原p1s600秒20×300ms+p5。两run全部32ok/22CPU/all live threads64/2hogs/source/control/bundle/11artifact/原group空核。候选0moves/671.7ms/6punch3362pkt1325754B；基线1relay@245s/881.6ms/18punch4632pkt1706984B。p5均绿。confirmation-copy-matched-quiet-comparison.json accepted=false：单对无重复统计/p99/手机能耗接受；初始diff checker漏KHOR文档的assert已修并留erratum，非产品红。
- Iroh41首物理ad627双端3816/3613绿、repeat34871为3747/7238红；Iroh40约5.4秒双红、Iroh39约4060/5799红、现代pre39 baseline约7674/7913红。原Noq12 quiet两candidate1/2move、旧第二at-most-one红；现代旧baseline零move/worst539/635ms。Iroh40 quiet71aa一move/1.3秒不算性能成功。ack-floor-controlled-stalls-comparison.json保留所有负面证据。
- 040407/042825等初连10.032秒超时未replug保留cold-start独立红，不混入恢复时序。N1仍未验收。

## N2已闭合的分项：真实暂停恢复、STOP、回复所有权

- **暂停恢复真实因果红绿**：Linux旧c2ec在原canonical主动pause→resume后731行立即Cancel被paused拒绝，先前主I/O/cancel已完成；最初误称first/precondition已在ledger/proof记erratum，保留真红。随后独立真实Node held-next-poll合同，Mac427683仅fmt失败（不计行为），精确patch4b7后 **Mac427690实际paused拒绝红**、wholecase6.25秒。原四whole fixtures冻结。
- **ba28候选同source双OS绿**：treecf0b29d4/snapshotf9efe7c2/control68cadb，Mac42770379226 +Turing4bcfad82全部12项net-node实际绿；新resume1.74/1.83秒、原canonical含pause/resume6.87/4.24秒、actual32STOP1.70/2.16秒、portable/IPC18/local32/process/remote32/budget20Mac24Linux均绿。分别11controlblobs/20与21artifacts、Linux原group空核；四whole fixtures与实际红逐byte同。resume-admission-red-green.json candidate_verified=true，scope限此恢复bug。
- 修复仅serve初始pause::poll初始化共享cache、incoming准入读node.paused该cache；accept热路径无磁盘读，不改半秒poll/持久job/预算/重试。之后f381仅纠正Node.paused及link注释。跨process文件通知仍原半秒poll，旧IO读覆盖新本地cache潜在generation竞态未另证明，不能偷带广重构。
- **actualSTOP同source双OS已闭合**：32Window实际encoded barrier/permit0、caller abort/join、32个Ok(Some(STOP))、零connection-close替代、同B stable wire、barrier仍持有时同wire Begin/Cancel成功。quic-explicit-stop-wire-proof.json更新ba28同cohort，首次Mac427613/a00证据逐字备份quic-explicit-stop-wire-before-cross-platform.json；旧c2ec没跑到STOP保留。不是WAN取消、accepted写jobs或手机能耗证明。
- 原remoteQUIC32 Mac427545真实5秒回收红→757保resp/frame至finalwrite、仅Resolve/Window/Record可取消，Mac427556+Linuxd964同source3f0全部net-node绿；whole两个原fixtures/red-green命令前缀核。quic-response-owner-32-red-green.json。Begin/Cancel/写job/Watch语义保持。
- caller短期427298真实红→427332原whole块绿；本机32旧427371真实红/whole945c→首次lease427389 SIGABRT保留→Box::pin后427399旧fixture绿。独立process427531约1006ms预算/真实Node join/child reap、坏cookie/额外字节无副作用、legacy halfclose绿。相关proof锚原sources，不拿新增module后whole hash倒填。

## N4真实双机与手机（异网尚未验收）

- 当前已执行双机 cohort（保留历史WAN任务名）仍 **757/tree2d218/snapshot3f0/controlccd**：coordinator4cee410c97601afe/Mac42757732776，Mac↔Turing各30目录+30固定29B正文逐byte验证，120/120全部ok，两端fresh配对ID相符。MacbinarySHAcd9be1a8/Linux0e063201。实际公共Relay101.200.80.77:3340；120/120只证明双机业务及该Relay可用，不证明跨LAN、公网NAT穿透。最新设备映射已确认：同LAN为MacLab↔Turing，cohort使用另一台构建macserver；A/B异网依用户报告，独立网络/NAT测量未完成。不是Direct/peer/v6/mobile/p99/吞吐/手机能耗；也非与旧ff608匹配性能比较。
- worker_us最近秩p50/p95/max：Mac→Linux目录55382/58815/76133、正文61322/64630/66470；Linux→Mac目录59527/64622/65616、正文55671/58949/59727。caller另存proof。wan-current-directory-body-proof.json。
- 同cohort Mac3/3+Linux3/3 stop/EOF/kill真实Node join/worker0/owned空/锁释放；最初两observer IndentationError留原并readonly恢复不计pass，然后新六轮绿。两个business fresh identities与8lifetime copies直接验证清理、一次GH secret已删。manifest current3f0/旧ff608备份，误directory_v1 ledger label保留并actual_v2+erratum补正。
- iPhone可手动Wi-Fi/蜂窝且已装/已配A/B。准备实际版本/cohort与现有可观测API后，给短、明确采样窗口，记录全outcomes/首成功/选路/实际v4v6。本日已有混合已装版本手机采样与手动关开恢复事实，见下方09:23记录；未执行当前candidate或能耗验收。

## 全范围仍要完成

| 包 | 余项 |
| --- | --- |
| N0 | 最终SLO/容差/样本量/p99充分性、同测量baseline/candidate/固定好路/全失败/负向校准；补充state checker冻hash及18数据正负控制保留，旧selection/decision/物理门不改，回溯绿不充prospective |
| N1 | 自然epoch实际红绿→原两门同source≥3次、旧2c X/独立cold-start、原quiet/黑洞/成本；冷暖多候选/多middle双向、201/208/244/242慢方向、原hop/recovery/dynamic/stalls/full矩阵与162/182红 |
| N2 | accepted写job所有权、两middle总3秒、输者/晚候选、地址epoch/重启/jitter/quiet epoch/close-vs-offer、写不重放/legacy；真实WAN取消 |
| N3 | Custom1024帧/256KiB外层队列、SSH/hop混合公平/共享vs分连接、1/3/5设备CPU/字节/唤醒/手机能耗；Noq704请求模型不能替代 |
| N4 | U12 v4/v6独立/同时/切族、peer移动、Mac/Turing/iPhone冷暖不对称、登记/出口/整机故障域；布局容量运维数据，不采购部署 |
| N5 | 主53/隔离42/43/44逐项vendor coverage/上游等价/退出、Noq1/5缺口、最终源可信Quality/nativeApp/代理/多机/手机能耗；证据前不升级删补丁 |

## 执行硬约束

本机source-only，禁Cargo/Rust/Node/Xcode产品build/test/formatter或下载执行物；Git/Python stdlib元数据/数据checker/纯stub控制合同允许。Turing owned `/tmp/khor-netlab-feat-net-1110` UID1110、单flock槽/Rust1.95/jobs16/nice10、50GiB准入40停/90min，仅fresh user/netns，无sudo/宿主网络改动，计时不夹编译/其他heavy。Mac走既有run-build/build_guard单槽/jobs2/50–40GiB/100GiB target，resident不脱离guard，不改installed policy/ensure-budget/已装App/生产store。

SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking=yes；无个人key/PAT/agent/签名/模型凭据给workers，不重设HOME/CODEX_HOME。WAN能力只私管道/一次secret，禁止普通artifact/日志/聊天票据。超时先查receipt/GH/proc/PGID，已有bundle/run先recover不重发。stage/launch显式完整source/control、前驱terminal采证/原group空/锁核；exec session不是结束，commit hook完成后才snapshot；ledger禁止并发writer。

图谱先，缺vendor覆盖才定向读；无.codegraph不自建，无子智能体授权。旧baseline/control WIP保留不reset；终态+owner/hash核后只清本次资源。verify工具只核identity，必须分fmt/compile/fixture/executor/product行为。证据根`/Volumes/disk0/Khor/evidence/net`，source-refs每次pin、最近全审358refs/45ops，后续新增逐条pin，稳定后刷新全量。恢复先核上方句柄，持续完整目标，不因分项绿或旧goal paused停下。

07:56新事实：43排序修复已在同whole新fixture真实52/1红→53绿，Noq441/home9均绿（proof vendor_candidate_verified=true），但first physical f2b832dc失败，candidate_verified仍false。X85→112稀疏质量造成Moving清除、停止补测，new44只分开有限采样/连续迁移确认，原radio总成本仍要验。phone-readonly-preflight-20261110.json保留Turing0.1.15混合cohort：首次devices本地document transfer busy，重试读取成功；对已确认手机localhost只读journal0minute一次360ms/valid空JSON成功，CLI总耗时非网络RTT；未开始受控换网/能耗。macserver只找到KhorDeveloperLab模拟器、标准路径无已装CLI/App，不能拿MacLab生产设备条目当macserver同身份。用户手机测试暂缓，不影响服务器目标。

08:13更正：用户最新同LAN说明优先于旧U10异网表述；所有历史WAN文件名/cohort保留，跨网/NAT验收撤回。手机已确认localhost，用户“是，稍后再测”，不启动换网/能耗采样；待用户恢复。Iroh44基线d7d4557e/tree937/snapshot714：Turing fe3cf29e实际fmt/Noq441绿，Iroh54pass/2新正向fail（旧53及新负向绿），home未执行，56.055秒/13artifacts/原group空核；接下来同wholefixture修复。

08:19更新：Iroh44 samewhole实际vendor 47bd0154为fmt/Noq441/Iroh56/home9绿，63.296秒/14artifacts/原group空；baseline54/2红与green56的实际tree中原whole50/epoch/sampling逐byte核验，proof vendor_candidate_verified=true、candidate_verified=false。首物理a4ed770d/PID1964828正在原ef0b控制执行，具体source b4/tree948/snapshot932。346refs/64controls/41ops元数据全审；首断言误要求历史短control全40位，9135b6d唯一解析后通过，原标签保留，非产品红。

08:27因果方向：44 first physical两端从未进入Moving，三次恢复RTT约130ms时实际outer legs仍在变化；需订阅实际hop绑定的四个outer wire事件并关联内层测量，先诊断、再决定跨层epoch，不能把其他copy已Direct当actualcarrier已恢复。见NEXT-DESIGN 08:27；暂无新行为修复或已启动任务。

08:29实际承载事件诊断77433e5d仅改node/hop.rs：4个actual weak carrier事件stream，原hop生命周期select内拥有，关闭stream后pending直至原owner结束；仅诊断开关时订阅，新增无spawn/timer/路由/探测。源审计actual-carrier-events-source-audit.json；graph覆盖缺3函数测试映射，不声称图谱证明安全，下一原物理将执行三角色全部真实入口。原物理fixture不变，vendor回归/根格式先核。

08:38：e819实际Node helper已由e0完整编译/三角色执行，actual-carrier-path-events.json四流覆盖绿；此前b66仅vendor任务未编译helper的边界保留。auditor 76f4361d在首数据分析前冻结，原三checker SHA bf7883/7075a4/bfd8b7未变。现同源repeat2 4dc正在运行，最多先此轮，遇红立即分析，不盲刷。

08:43：当前优先复查cold-start，4dc与旧040407/042825同10秒首次连接超时类。源e819不再改行为；从dial_with→Endpoint.connect的实际入口追踪address resolution/known-hint连接与延迟identity候选，已知Relay hint不等于已发包/握手，当前日志没有目标y的selector观测，不能先猜网络不可达或CC阻塞。未新增重试、超时或放宽fixture。

08:46：2ef6bee5只对vendor/iroh/endpoint.rs加start/connecting-ready/handshake-finished bool阶段诊断，attempt仅DEBUG enabled时取本地单调时间；无新await/timer/task/probe、不打印请求/票据/完整error。connect-stages-source-audit.json冻e819后唯一diff，net-vendor f89fe221/PID2010457 source48e9/treef1da/control68已launch，查source-refs/receipt再恢复，禁重提交。netlab resident stderr直写serve.log文件（lab.rs272），不是未消费pipe；仍不以此排除诊断CPU/IO扰动。

08:49当前connect stage source2ef已vendor库实际编译、原fmt/441/56/9绿；bf6105dd原物理正在运行，成功和cold红都保留，不因缺replug失败创建假四秒结果。connect-stages-source-audit.json保留同e819后单文件诊断diff。

09:04用户最新两项：同LAN的Mac明确为MacLab本地电脑，构建macserver为另一台；旧U10 macserver/Turing用户异网说明仍适用，独立拓扑测量未做且公共Relay不等于NAT穿透。用户已恢复iPhone测试：首次session085800 Wi-Fi30calls26ok/4busy但用户报告曾锁屏，不能作纯前台基线，旧cellular_transition尚未发切换指令已自然结束（PID2037262确已退出，无任何kill信号）。新session090200用户再次确认已解锁/Wi-Fi/khor前台，现一分钟baseline，之后须先启动连续transition sampler，再发切蜂窝指令并等用户确认，再回Wi-Fi。v1将busy笼统标local，归属未证，原数据保留并erratum按state_busy解释；v2不再假定本地归属。collector ops/phone-rpc-sample.py，旧v1备份、protocol json与每phase receipt冻hash；30s外部CLI timeout即停止而不累积旧版本未确认回收的RPC。当前installed Turing0.1.15/phone0.2.0，非candidate或能耗/逐包验收。

09:08手机新session090200：纯前台Wi-Fi基线30calls18ok/12state_busy（归属未证）；随后用户确认已关Wi-Fi使用蜂窝，cellular_transition90秒10calls8ok/2busy，前3个成功调用耗时27916/29285/19093ms，随后成功约687–1844ms。不能把这些CLI总耗时称RTT或准确换网恢复时刻，但真长尾保留。当前wifi_return_transition已启动90秒，本地exec91517/remotePID2051242，remote09:08:06UTC；已发回Wi-Fi确认题，待用户回复。新sampler不并发、无原始journal正文输出，不启动heavy。

09:23新核心事实：用户强调relay/peer relay未及时接管，后明确恢复依赖反复关开App。manual-app-restart-erratum.json撤回“正常自动接管”解释。MacLab手机journal可见08:46/08:51 pump10s失败/300s退避、数次carried↔relay切换与retire、09:08 retire后至09:15新inbound存在空窗，09:06 hop曾via hinton而非Turing；需核完整原因/实际绑定，不能先猜根因或假定整个窗口backup可用。MacLab/Turing clock bracket：Turing快约18.603–18.644秒，跨主机时间线要校正。当前pmset断言防idle sleep15h、当前MacLab进程存在，只是当前状态，不冒称证明故障时无休眠。生产App/所有resident未重启或改配置。

09:38：wake fixture首55d sourcebb21/treec112在ccf95c98实际E0308 helper tuple编译失败，100.176秒/11artifacts/原group空，不计行为红；85d只补discard分号，原测试断言全不改，新whole hash已修订留初版历史。source85d/treeda15/snapshot80cc的新net-node baseline已launch，实际句柄查source-refs/receipt（本地stageexec4050已返回后更新）；需要真实tick断言红才改生产顺序。旧wholewires SHA18acdf83冻结。SDK已只读help核0.1.15支持journal --on，并真实missing-target非busy拒绝，先前busy control为不确定；phone command controls已存，未再采手机。

09:41：85d/7da54d78实际runtime wires12pass/1fresh-newwire FAIL，原11+新all-suspended负向绿，hop12/net/fmt绿，144.211秒/12artifacts/原group空核；其余net-node未执行。首ccf仅helper返回tuple compile错已留，85d只补分号，测试whole冻结。最小候选仅tick末尾先旧resting再新heard，使同批live新收包最后清gone；after_retirement并发与现场根因仍未关闭，原wires whole18ac与新whole不改。接下来同whole完整net-node双OS；不部署生产或重启用户App。

09:44：phone session090200 summary已归档WiFi30(18ok12busy)/to-cell10(8ok2busy)/return39(21ok8busy9paused1error)，三slowcell成功27916/29285/19093ms，return成功最大8311ms；全79 outcomes保留。用户已明确manual反复App重开后才Turingpeer relay成功，自动handover不接受。MacLabpublicid449/PID87295只读证实；Turing被动网络journal186条、MacLab220条已归档（Mac v1漏departed字段需补快照，不能臆造完整性），跨机时钟Turing快18.603–18.644秒。command negative busy一次不确定、随后help与missing-target真实非busy拒绝已核，--on非静默忽略。所有生产App/resident/配置未改，勿自行清理_host/_ghost等子进程。

09:53分项闭合：wake same-tick修复双OS实际13/0，原背景全suspended负向绿，准确source3a3；原canonical2.23s/Macresume1.69s/STOP1.50s等完整profile绿。after_retirement现仍在slots锁释放后写asleep，是相邻未测并发窗口；下一单独cfg(test) barrier在真实retire API停于该窗口，新adopt+真实echo后放行，先红再移asleep到槽所有权内提交。原wholewires18ac与wake fixture不改，原暂停/重试/既有node合同不缩减。

09:59：第二回归6ba仅新增cfg(test)Wires实例gate+独立retirement2tests，production无gate/新锁/等待；新fixturewhole freeze见retirement-epoch-fixture-freeze.json，原whole11与wake2不改。先真实retire released-slot→新adopt+echo→旧finish的旧Asleep提交红，之后才移提交；Turing任务可能先有fmt/setup失败，须据实际日志分类，不伪称运行时红。

10:06当前源码LEFT_ALONE_FOR600秒，只抑制后台pump；前台请求不因backoff被直接拒绝，但仍须核gone-short-dial与cached dial failure/epoch等入口。第二退役gate 13d仅fmt红，2e修assert_eq排版后新whole已重冻，业务断言不变。当前root元数据358refs/64controls/45ops全核，09a唯一pending。不要将首次fmt当runtime红；收集/verify完成后据wires-unit实际assert判定。

10:10：09a/2e46实际runtime wires14/1红（replacement真实echo后，旧retire结束再次left_alone），原11+wake2+current-suspended负向绿，116.311秒/12artifacts/原group空；其余node未执行。候选只将Asleep标记移至retire_slot持槽锁阶段，after_retirement不再迟写；现有slot→gone顺序已审，无持gone再取slot路径，adopt发布后back清除。所有原/new whole不改，cfgtest gate不进生产；新serving并发/直接DialFailureAsleep/地址epoch仍未覆盖，不能称现场全修。

10:31：Macf9失败明确为GitHub API blob JSONDecodeError before materialization，64MiB raw/128MiB+JSON cap足够18.347MB bundle，非大小门产品失败，先同run retry一次。第一失败artifact未含source.json，verify identity不可能完成，已保留未误记通过。phone历史journal单次抓取因state busy失败、零手机侧历史，不做更多主动采样；Turing/MacLab被动日志留。用户“关开App”没有独立证明force-quit/OS进程PID变化，仍记manual reopen，不能写成受控进程重启。下一需核stale dial result与fresh positive proof：先清晰建模正向证据版本；不把它等同IPv4v6 network epoch或重置所有LOST/MISSED，不加重放/竞速写入。

10:44更新：f9退役竞态已同source双OS全net-node绿，retirement-epoch-fixture-freeze.json仅该scope candidate_verified=true、field_phone_issue_resolved=false。当前无heavy/phone sampler。下一按NEXT-DESIGN新鲜peer接触版本合同，先真实Asleep旧拨号/排队共享红，不先改生产或假称手机根因。

10:50实际句柄：第三合同only-test 653a源在82adb5aa仅单assert格式红、54.097秒/10artifact/原group空核，未compile/不计行为红；dce413de只应用远端format.patch，runnable red sourcec1cf/snapshot1b44。Turing97c96cff/PID2187034已launch原net-node/control68，收据/source-refs已pin；先核终态再collect，不重复提交。新增peer_contact whole SHA见peer-contact-fixture-freeze.json；旧wires whole+旧dial_budget whole（只去新mod声明）与wake/retirement/probe/wire逐byte核验不变。无Mac活动任务。

10:53 N1 bf6105实际carrier auditor已补齐：四绑定outer最晚1274ms选IP，X质量窗口在103.17ms重开，后89.55ms不立即改变reading quality/无pending采样，+3752才Moving、原5505.782ms红；新correlation留raw/SHA，不丢4秒失败、不改参数。手机相关97c96cff正在原dial_budget30项，先等真实红。

11:00第三竞态因果：旧97c96cff原24+4负向绿、2fresh-contact实际红，265.423秒/14artifacts/group空；初版scratch accept在关闭端点时先醒None造成清理task panic，原错误和断言均保留。f475只改先abort/join并验证不panic，clean baseline0e62a55f同whole实际28/2红、无清理panic，96.48秒budget，sourcebdae/snapshotfd88/control68核。之后才生产候选0e0b83e5：gone与正向contact版本同锁、旧Asleep只按开始版本提交/复用；非Asleep、计时/退避/请求语义不改，旧所有whole fixture不变。candidate source4f6a/snapshot1c3f已save，Turing提交本地exec83085需先取终态/receipt再恢复，禁盲重发；Mac未发。现场phone仍未部署/未验。
