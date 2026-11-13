# feat/net 当前状态与接续入口

更新于2026-11-11 13:38 UTC。完整N0–N5目标“完成计划”仍 **active、未完成、未阻塞**，root串行，不派发子智能体。上份STATE逐字保存在[完整历史](HISTORY-20261111-through-workload-startup-v2.md)，SHA `76e0dbfb6c8dd23f58ffad064ca745290818a0dac42723a2c6a35506e0755971`，它继续链接旧原件/所有失败记录。历史里的“当前/待验”只代表当时；原始收据/日志和实际worktree优先，不按摘要覆盖未提交内容。

## 14:15 UTC当前入口：kernel三轮原件/单位已核，动态选路f2f在途

独立kernel模型 **turing-20261111T140913Z-141b0ea2** 已 **94.220秒0**、17files/identity/原group空，原始自动contract三轮全绿；更独立的 `kernel-profile-verification-v1.json` 核完实际12份读回，kernel_model_profile_confirmed=true、**full_N3_accepted=false**。min RTT40199/40216/40277us，两方向实际payload约7.52/7.55Mbit/s；原4预热+64echo/两8MiB全部成功/全量字节一致。JSON rate1000000↔文本8Mbit、delay0.02↔20ms、lat40000↔40ms相互核实；burst4096B，x eth0/gateway v7前后同profile且两侧qdisc都经过≥8MiB实际数据。记录qdisc drops每轮18/21，不能把零randomloss写成零drop。校准的是共享condition的独立kernel模型，**不追填旧Node cohort已读回配置**；下一版w1也要保留自身前后TC读回。详细数值/边界已入WORKLOAD，proof/ledger终态。

准确源仍 **df5b3f6a** / treea315f0e280e0766b68b48f5c8b15b6f5c2e06aa6 / snapshot97386eb2d87102dc7d14619ef929447385ed677a，source-v2不变；本次control **ea44ab27af3011821d188e0a500ad91b20ed17b1**。前一bd7e5f6c原始总job1/contractfalse已保留：三项Rust门全绿，但harness把首manifest接在test名称同一行，旧startswith校验漏读。17files/identity/空group、validator-prefix-failure-v1 proof/ledger终态；没有追认旧report为绿。新control只修唯一marker后解析，原门/样本数不变，新增harness前缀回归；完整control **12项8.578秒绿**、bash/diff过。旧原件副本的metadata-only重放验证另存control-workload-calibration-prefix-replay-v1.json；新的三轮真实运行才是当前0收据。外部单位verifier的1正例/6单位错例已过，SHA在kernel-profile-verifier-audit-v1.json。无本地Rust/产品执行。

唯一Turing当前为 **turing-20261111T141304Z-f2f17750**，PID **4174858**，14:13:25 UTC launch；task `netlab-dynamic` / revision `effective-route-dynamic-v3` / candidate / 原control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34** / 同一生产候选 **3de372a0** 的冻结source-v3。stage exec57393终态，原固定p2/p3/p7三轮矩阵，不重发、不穿插Turing构建/计时；这是49改变有效RTT/Patience后对完整动态矩阵的最新源复验，前一单独p2/p3绿不代替p7等场景。下一步先核实际PID/execution/全部原门。

与此同时root可以本地source-only实现下一版Node观察器校准：同一实际condition/真实终端与文件、固定计划与每类一个在途/所有720人口保持，加入父future拥有的有界定时探针，比较逐条同步落盘与阶段末有界记录；明确模型版本/顺序对照，保留失败与取消/中断缺样本判断，不冒充产品优化或删计划积压。w1自身前后设备/gateway TC读回也要保存；`closed`仍只是API成功，独立host/child退出证明待补。这些**尚未实现**，待当前Turing时序完成再按固定新源/control运行；Mac独立格式/构建可以并行。完整N0–N5/原严重stall与quiet/长稳/真机/59vendor专项等范围不变，目标active，无外部阻塞。

## 14:05 UTC当前入口：恢复三轮已收齐，独立kernel校准bd7在途

生产候选3de的恢复 **turing-20261111T134417Z-ce3b1222** 已 **568.791秒0**，15files/identity/原group空，p2/p3各三轮 **全6绿**。p3黑洞后228/225/227次真实Node目录RPC，worst644.5/664.6/590.5ms，零失败/零retired；p2约2.0秒/947.8ms/962.2ms离开慢路（可用Relay或Direct均允许），原30post样本/无退役门保持。`recovery-result-v1.json`另核原始每个logical attempt恰有started/finished、list成功/无error、所有p3真实elapsed≤原1秒。它是目录RPC，不是PTY或完整N1稳定/严重stall/成本接受；proof/ledger已终态，不重发。

唯一Turing现在为 **turing-20261111T140243Z-bd7e5f6c**，PID **4165925** / child4166154，14:03:05 UTC launch；`netlab-workload-calibrate` / revision `workload-kernel-calibration-v2` / baseline / control **d77c38c0975b6d23505df1c954b36863ede9b32f**。stage exec55237终态；14:04:17实际ps活、编译/首轮已执行绿，第二轮执行中。首轮minRTT40250us、upload/download7.5199/7.5528Mbit/s；实际x eth0及gateway v7均有netem.delay.delay0.02、tbf.rate1000000/burst4096/lat40000。只是暂时原始读回，**尚未全三轮或正式单位复核**，不称已校准。只查同receipt/execution/PID，不另起Turing工作。

准确源 **df5b3f6a0ac26aa02dc92a5593735fef35048adf** / tree **a315f0e280e0766b68b48f5c8b15b6f5c2e06aa6** / snapshot **97386eb2d87102dc7d14619ef929447385ed677a**，`candidate-workload-calibration-source-v2.json`，冻结clean/0excluded。新代码仅ignored校准和共享测试condition，无Node/net/vendor/Cargo生产diff。w0独立kernelTCP无resident/Noq，父调用拥有一个server/三个socket，4预热+64echo+两8MiB方向；前后x和gateway各留TC JSON/文本及接口计数，**连接/echo2秒、传输120秒**，模型minRTT35–80ms与双向6.4–9.6Mbit/s门已在首次运行前冻结，详见WORKLOAD；不是用户SLO。原自动report保持calibrated:false/units_review_required:true，须原始JSON与文本单位相互核对另存proof才接受kernel模型，不能直接接受N3性能。

其首源 **e7939f3d** / tree8aebed0b5b04493540e9f5a64efc7e8744cd994d 在 **turing-20261111T140042Z-e6e3d424** 48.105秒1、10files/identity/空group，仅root格式/0样本；准确patch已check/apply到df5，format-only proof/ledger终态。控制新task仅固定允许列表+独立分支/原始数据验证，full/physical旧68cad不改；控制stub **12项8.550秒绿**（新增1项11个缺失/伪造/提前接受反例），bash语法/diff过，零本地Rust/产品执行；`control-workload-calibration-tests-v1.json`及helper前后SHA收据已保存。原两pycache保留。

上一节所有“唯一ce3/2aff”均被本节终态替代。下一步收bd7准确三轮/identity/原始TC单位与物理门；然后实现独立观察器定时探针和记录开销对照，关联真实wire/许可/队列后再选策略。全N0–N5、最新严重stall/quiet与长稳/手机等缺项不变，目标active，无外部阻塞。

## 13:46 UTC当前入口：混合负载已收三轮功能绿，唯一在途为恢复ce3b

前述N3 **turing-20261111T133630Z-2aff8ae2** 已216.845秒0、17files/identity/原group空，三轮各720、共 **2160成功/0错误/0超时/0blocked**；三份16MiB逐字节一致，每类每轮120样本全在实际bulk区间，close_API accepted。完整功能proof/独立latency分解/ledger终态，**不是完整N3性能接受**，不重发。原始表与解释现见[WORKLOAD](WORKLOAD.md)：bulk目录服务p95约107–112ms（空载62–66）、终端81–84ms（空载43–44）；计划→完成目录289–320ms包含串行观察器积压。第三轮空载自身有85–93ms计划迟到，不猜CPU/I/O因果。实际payload率7.120/7.127/6.735Mbit/s不是独立容量校准。仍须校准/观察器成本/真实wire与排队因果，再选调度方案。

唯一新Turing **turing-20261111T134417Z-ce3b1222**，PID **4138770**，13:44:39 UTC launch；task `netlab-recovery` / revision `effective-route-recovery-v3` / role candidate / 原control **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34** / 同一生产候选 **3de372a0** 冻结source-v3（准确tree/snapshot见下文）。stage exec52496终态，下一步核同receipt/execution/实际PID和原p2/p3三轮门，**不穿插Turing构建或计时**。先前“唯一在途2aff”已由本节实际终态替代。N3校准/定时探针的具体下一协议已记录，尚未实现，不擅自把待做写成绿。全N0–N5范围与授权不变，目标active。

## 唯一在途：真实Node混合负载基线

**turing-20261111T133630Z-2aff8ae2**，PID **4128164** / child **4128397**，13:36:52 UTC launch；task `netlab-workload` / revision `resident-mixed-workload-v5` / role **baseline**，control **ebe8c8243ee639aa63aaa08a3202cbc9d1c0c8d8**。stage exec85816已终态；13:37:43实际ps活、execution running/network-driver，格式/编译已过（9.81秒编译）；首轮随后实际manifest1/396样本全ok、进入bulk，见准确scenarios原件，**尚未全轮终态或验收**。只查同receipt/execution/实际PID，不重发，不另起Turing构建或计时。

准确源：**3c1e852002e0ade43215c248e45caaf8a780b791** / tree **66c5744b067f0a594da54d04485868938af55f07** / snapshot **5d999cad79c3f2ffe7f9cff62d0ea7af6dcf8e96**，冻结时clean/0excluded，根 `candidate-resident-workload-source-v5.json`。产品HEAD另可能有本状态docs-only提交。相对已验证3de的Node/net/iroh/noq生产文件与Cargo锁无diff；新增ignored测量代码/文档，不把旧收据称新fixture已验。

协议 **resident_mixed_workload_v3**：[WORKLOAD](WORKLOAD.md)，代码 `crates/cli/tests/netlab/workload.rs`。真实配对resident；目录与FetchPath文件请求走Main，终端reach/实际PTY走现有独立Tunnel。OpenOnce argv显式/bin/sh -c启动read/printf（沿既有term_echo测试入口），关闭输入回显、唯一请求/响应前缀，Ready/实际程序就绪/预热共用原10秒绝对期限。无额外身份endpoint/脱离父future的测量任务，无用户会话参与。名义40ms/每方向8Mbit/40ms队列/零随机loss；**尚未独立校准**，diagnostic/non-SLO。

每轮 idle_before/bulk/idle_after，各目录+PTY120样本、100ms计划、2秒调用边界，总720；16MiB固定seed完整字节比较，两类至少各30个样本完全落在bulk实际在途区间；三轮串行。错误/超时/blocked都保留。每类最多1在途，固定计划的积压记录scheduled/start/end；分析同时报开始→结束与计划→结束/调度迟到，不只看成功服务时间，不称开放并发模型。`closed`目前只证明close_on API成功，**没有独立host/child退出证明**，不可称泄漏/完整取消验收。三轮功能绿也不接受N3性能、CPU、能耗、校准、p99或队列设计。

## 已验证的连接候选：同源3de

源 **3de372a026488a64a794935600e47db29f2f6faf** / tree **ae76cd646749ca624d5e3e98af01f96099ebd0e0** / snapshot **a9f46eaa909b8cf45663395912798effb31b7261**，`candidate-effective-route-source-v3.json`。

| 验证 | 精确句柄 | 已核原件与范围 |
| --- | --- | --- |
| Mac完整Node | **43022962905**，requestd951483105d54f8bbfc8d39f8811c854 | 全12绿，budget32/hop89/net68+1原ignored/wires16/journal30/IPC18+六单项；六initialCustom/两incoming实际执行。20files/11controls/identity，exact13855483361，ZIP SHA64ddd87f3f21ce255ae06f06ccf8a305eb435ff799f2a7020838ce8ed879c50c。 |
| Linux完整Node | **turing-20261111T131248Z-5c2a0339** | 391.748秒终态0，21files/identity/原group空，全12绿（budget36，其余同Mac范围）。 |
| Linux路由短回归 | **turing-20261111T124757Z-678b0ebd** | 118.671秒0，6+2=8项绿；13files/identity/空group。短8不代全12。 |
| Mac vendor筛选 | **43023967463**，request571e47203a21465283c6c9292719d51e | 443Noq/58path/11home绿，13files/11controls/identity，exact13855853516，ZIP SHAc4c6f98d72a3fffabd974399bcd34d6cd3b23df1d675ca489b9bf12202ded5be。不是59补丁全专项或完整vendor。 |
| 四peer三轮 | **turing-20261111T125643Z-6970ad46** | 699.061秒0，37files/identity/空group，全6绿；新f4r首内容 **1.263034349/1.253216299/1.222981525秒**，各30持续样本、约15.3–15.5秒；held/continuous/selected_every/carrier_both true。 |

三轮四peer已另核原始两端：每轮31实际完成响应/31匹配stream/每端单一business wire/每端一个outer commit/previous_serial null、无detach/I/O-ended，首次响应在M恢复前。三原f4控制也绿，但其业务最长仍3.6/3.7/3.6秒；不是旧所有长尾关闭、p99、手机或完整N1/N4接受。独立 `physical-result-v1.json` 保留原6行与log SHA。上述全部准确原件、identity/逐组behavior proof和source-refs ledger已终态，不重发。

## 当前实现含义与完整性边界

- Node已有carrier时，普通原generic路和一个立即显式Custom普通握手竞争；early保留原fast0RTT和一个父future借用的Custom普通握手。原caller/deadline/最多2活动，旧路失败不提前取消在途carrier，认证resting仍决定。无新增常驻map/task/timer/强Node，写/0RTT准入不扩大。快速Direct负控绿，但额外握手CPU/字节/手机成本待测。
- Iroh49只在peer共享preferred不在本conn、且仅一条established/open路径时报告Noq实际fallback；零/多条不猜、仍Unknown。保留共享偏好/Noq状态/发送/其他连接；rtt_of/Patience会读到实际RTT，不能称纯显示无影响。主vendor **59=Iroh1–41/45–49+Noq1–13**；42–44隔离；上游等价未新核。
- 两个incoming所有权合同继续保留实际carried-only且无marker条件，真实另一个Direct/TLS端点保持出站Direct/PathId0/前后echo，未删原最终所有权断言。四个installed-carrier合同与旧573逐字相同：旧同身份IP前后实际业务、预装carrier、原1秒caller、正确路型/唯一响应/两快速Direct负控。它们不是完整物理中继两腿或全部移动覆盖。
- carrier在调用开始后到达/own prep已结束的事件唤醒仍是待覆盖假说，不能仅凭入站业务活推定该端出站Link好；本轮四peer绿不证明所有此竞态关闭。若复现再按真实证据修改，不提前加轮询或放宽阈值。

## 新N3入口的失败原件与诊断

1. 初始源bf2ea621在Mac **43027741141**（request84f8206545c1475d9fa9d3e8950f7390，net-unit）仅root格式红，exact **13860120002** / ZIP SHA **b613d7c564bfdfb971f2c59defa35c76b608af0909ef5806caedd03b2d01d745**，9files/11controls/identity、零行为。精确patch仅workload.rs，已check/apply到b0ee2d1c；独立format-only proof/ledger终态。
2. 源b0ee2d1c / tree67fc3c61688a04634a44cb1b68387cd7f5fc302a，Turing **turing-20261111T132111Z-6f4c9c05**：92.245秒1、14files/identity/空group；三build各两处RoadsUnread无Display，零scenarios/零样本。仅fixture编译错误，0b6119a5改用Debug，不当产品网络红。
3. 源0b6119a5 / treeca6e8065970bf881eb5e291e28dcd7686662e4e2，**turing-20261111T132456Z-666f4160**：112.542秒1、17files/identity/空group，已编译但三次10秒PTY预热超时，零计时样本；无原始PTY，原因当时未定。独立premeasurement-startup-v1 proof/ledger终态。
4. 源2fdab521 / tree71f79090deb3833d87e06a88ab27c090565bb366，**turing-20261111T133113Z-960e717e**：112.402秒1、17files/identity/空group，测量协议v2增加程序就绪标记/CR及有限启动原始输出，仍三次10秒超时、零样本；原始只为shell提示符，没有脚本标记。premeasurement-startup-v2 proof/ledger终态。

静态+运行定位：spawn_shell_at设置KHOR_HOST_STARTUP；CLI `commands::host`调用host_main，后者传startup=None；Node自己的host_args才读取环境变量，CLI入口没走它。这个宿主启动入口缺口已在WORKLOAD/证据记录，**没有在网络任务中顺带改命令引擎**。当前v3沿既有 `crates/cli/tests/term_echo.rs` 的真实open_on argv入口，保留PTY/终端协议/传输/所有时间与样本门；首轮实际已收到程序就绪及warmup唯一回复并进入采样，不能回填此前失败绿。

## 工具与开发加速

控制树 `/Volumes/disk0/Khor/worktrees/current/khor-build-net` 本地HEAD **ebe8c8243ee639aa63aaa08a3202cbc9d1c0c8d8**。SSH-only net-node-routes最初controlcaea，固定6+2，已真运行绿；netlab-workload最初control1fac8560dc25fac3a64028ff25ecdb5e786e057e、v2用70ad99b62c1a8ecbbdadfdf6038f9ba738803910、当前v3用ebe8。新control只用于这些独立任务；原Mac/full/physical继续 **68cadb57a0ed72d40a4ca9a843e86b9c6797bf34** 保持可比。没有推送control或改变GH dispatch/Mac admission；Turing stage直接使用准确本地Git对象。

工作负载parser核三日志/每个manifest、bulk、complete各1/单一成功summary/6组各120唯一index/时序/原始重算overlap/完整失败计数。缺样本、重复、超时、伪造overlap、缺轮、重复summary均不能绿。control11项stub 8.381秒（v2再11项8.291秒）绿；v3只改协议literal，聚焦1项7subcases 0.319秒绿、bash语法/diff检查过。全部仅Python模拟/元数据，零本地产品执行，原两pycache保留。准确SHA和检查在 `control-workload-tests-v1/v2/v3.json` / `control-route-contract-tests-v1.json`。OPS只扩固定任务allowlist，原guard/launch/ledger不改。

外部helper `ops/analyze-resident-workload-v1.py` 会在终态identity/原件后独立保存service与scheduled-to-end分解/真实bulk overlap子群/全部错误分母，永不覆盖原件或称N3完成。合成数值检查已过，SHA/检查在 `workload-analysis-helper-audit-v1.json`，不可在未收原件前出正式结论。

## 必须保留的历史红与完整待交付

旧573最终answeredfalse/pathNone/requests0；166变answeredtrue/Unknown/requests1但same最终路型门仍红；eb72旧fixture无marker前置红、412清理panic、ca5格式红各自原件分类保留。旧物理b1a57d/2f2a53e1 f4r三红（前二heldfalse、第三31真业务但首次selectedfalse），源963/a653原两控制first-leg-heard2059/2325ms、业务最长5.2/4.9秒红，不能覆写。business-continuity-decomposition/old-f4-business-owner-audit均在证据根。

严重6aa stall/quality_steps0/RTT不对称与carrier约21秒终止、约8秒恢复尾巴、Busy854、resume32timeout、quiet/f4s/coldstart、Noq13物理7.3/5.4秒、截断8f3仍未闭环。Iroh47四项的10秒/7.499秒红保留。旧动态27/长稳4绿、Noq704样本/15bulk与旧9项对照不当当前完整验收或手机证据；三样本不称p99。

完整范围保持 [PLAN](PLAN.md)：
- N0最终UX门/足量p99/同条件完整对照报告。
- N1恢复长尾/严重stall/质量/跨连接测量、quiet与比较成本；49改变RTT观测后还须同源恢复/长稳适当复验。
- N2真实同conn独立/同时v4/v6迁移、LOST身份、完整取消/owner/迟到/撤销、carrier在开始后到达。
- N3真实交互+批量的校准/真实wire/stream/排队与背压事实、共享/分连接/公平对照、1/3/5设备CPU/流量/唤醒/能耗。当前入口不代替完成。
- N4完整双腿Direct/Relay/SSH、唯一/多middle、prepared/混合版本、独立v4/v6移动、故障域/运维设计。
- N5全部59vendor上游/退出与Noq1/5专项、最终Quality/native/proxy/多机/iPhone和一致报告。

MacLab和Turing同LAN，macserver另一构建机、用户报告异网。localhost iPhone历史0.2.0、曾反复关开App才恢复，蜂窝截图仅hinton直连29ms。**候选未安装、无phone sampler、旧测试窗口已过期**；可信GitHub/TestFlight具体构建就绪后再交接真机。最近向用户完整交付估计仍3–5天、低置信；先前按小时过乐观，不把阶段绿/单轮耗时当完整ETA，也不因compact重置范围。

## 执行约束与下一步

产品 `/Volumes/disk0/Khor/worktrees/current/khor-net` branch feat/net。本机source-only，无Cargo/Rust/Node/Xcode/fmt/产品执行；Git/Python源码元数据与control stub允许。Turing严格串行UID1110/flock/user-netns `/tmp/khor-netlab-feat-net-1110` /Rust1.95/jobs16/nice10/50GiB起40停/90min原门，无sudo/host网改动。Mac独立jobs2/shared cache/原门可并行；SSH BatchMode/ForwardAgent=no/StrictHostKeyChecking。全部实现和上述验证已授权；无main合并/发布/采购/生产部署、已装App/config/store/resident修改授权，fixture仅namespace scratch。隔离Iroh42–44树detached2ef6bee5不动。

图谱先行，本树无.codegraph不擅建，CRG部分/滞后，0flow不是无影响。explore/debug/review技能已使用。证据root `/Volumes/disk0/Khor/evidence/net`，ops同根；source-refs.json仅root单写，stage/register未终态前不并行ledger mutation；timeout/EOF不是终态，查同receipt/PID/Actions不重发。

下一步只核2aff8ae2实际执行；终态后收原件/identity、重算3轮完整720/实际重叠/16MiB对照与退出API，保留所有失败，运行独立latency分解。若还红分清fixture/产品事实，禁止靠放宽原时限或删除样本过门。随后独立校准与成本/真实wire、最新生产恢复/长稳、N2/N4/N5全范围。无外部阻塞，目标保持active。
