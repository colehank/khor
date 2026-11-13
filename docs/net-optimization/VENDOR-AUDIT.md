# Vendor 补丁覆盖与退出审计

2026-11-11 15:25 UTC：同生产候选3de新增动态三轮27/27、恢复三轮6/6原门均绿，原件与实际业务人口已收齐（f2f17750、ce3b1222）。最新同源完整长稳de04bcc1在途，p1原600秒已绿，其余未完成。当前工作负载六轮4320功能绿是观察器实验，不是vendor吞吐/能耗或严重stall关闭；新业务流诊断仅扩原opt-in字段，未改任何vendor源码。以下Noq1/5专项、严重6aa、quiet/成本与全部59上游/退出缺口保持，详STATE。

2026-11-11 13:22 UTC：Iroh48/49与已有carrier拨号候选 **3de372a0** 已取得同源 Mac43022962905 / Linux5c2a0339 全12绿、Mac43023967463 的443Noq/58path/11home绿，Turing6970ad46四peer三轮全6绿（每轮31匹配实际响应流/同business wire/单outer无detach；首内容1.2630/1.2532/1.2230秒、各30持续样本超过15秒）。准确原件/identity/逐门proof/ledger已收齐，见STATE。只接受上述已测scope，不将筛选回归称59补丁全部专项或完整上游/退出接受，不回填旧3.6–5.2秒业务尾巴、严重stall、Noq1/5、移动/quiet成本或N5。当前主仍59，42–44隔离；新N3测量代码无生产vendor diff，正在单独baseline验证。下面各时点“尚待”按历史理解。

2026-11-11 12:26 UTC：新增Iroh49候选，仅修每connection effective路观测：共享preferred不在本conn且只有一条established路径时报告Noq实际Backup fallback，零/多条未匹配仍Unknown，不改实际shared选路/PathStatus。源166双OS实际两项已有carrier内容已成功且各1响应，但same final route断言Unknown仍红，准确原件/部分证明见STATE。新49尚待同四合同转绿、全12、vendor/四peer/恢复成本；主现在59=Iroh1–41/45–49+Noq1–13，42–44隔离。之前58/57计数按各源时点保留。

2026-11-11 11:36 UTC：源9634737a的Mac节点全12与vendor443/58path/11home均已identity/准确产物验证（Actions43010224456、43012074442）；Iroh48 scoped握手合同含错误目标身份明确拒绝已过。Turing四peer首业务三轮1.22–1.25秒，但连续性缺证仍三红，原停顿控制还两红；同源Linux全12在3fc797b0运行。不能从筛选单元绿接受48或消除历史严重stall/Noq1、5专项缺口。主仍58、42–44隔离；新业务完成收据只增强观测，不改vendor行为。

2026-11-11 11:02 UTC：主候选现 **58项**（Iroh1–41/45–48、Noq1–13，42–44仍隔离）。新增48是每次握手显式指定初始Custom载体及Node有限竞争，准确实现/合同在STATE与vendor/iroh/KHOR.md；尚待同源双OS Node、完整vendor与冻结四peer协议/成本，不作为已接受项。先前只解除“无活连接”selected_path的d521在Mac43005910530通过443/58/11过滤器，但Turing9d655e7f四peer三轮仍红，live=true/registered=1保护未触发，后继已撤回该行为；单元绿不补产品恢复证据。以下旧计数和“当前”段落均按其原时点保留，不覆盖旧严重stall/Noq1、5等缺口。

2026-11-11 06:35 UTC：当前主候选仍57项（Iroh1–41/45–47、Noq1–13，42–44隔离）。Iroh47实际旧41c2/b0146293为Noq443绿/path56绿2红；候选8cd/c350为443/58/11绿，8cd准确双OS全12，原动态27/0与长稳4/0均已核。固定物理C-B/B-C四项收齐：两C分别Go3569.426/3536.190ms与3820.207/3533.388ms、各124业务与warm51全过；B一次初始业务超时未replug、一次Go7499.610/3473.251ms且warm11/51失败。三个实际replug均4innercopies、原3+3探测预算及8方向byte进展，成本完整边界和selected-road驻留差异保留。详见证据根confirmation-copy-identity-physical-fixed-four-result.json和STATE。两C样本少且贴4秒门，不作p99/可靠性/全N1接受，6aa严重stall未重现/闭环，quiet/stalls/手机/能耗和57项专项审计仍欠。Iroh47只统一samples_after与quality/age的代表copy，原quality选择/预算不改；上游未核等价，退出仍需对应原合同/成本/实际场景。下文带旧时点的“尚未验证”等为历史，当前以本段和STATE为准。

当前候选盘点（21:52 UTC）：主56项=Iroh1–41/45/46、Noq1–13，42–44仍隔离；46物理固定六次未接受。Noq13仅解决独立显式路径探测ACK及时性，实际旧efb2/8a4a7521原443项442绿/1红，普通延迟ACK/Initial与真实accepted0RTT三负控及原439绿，whole fixture e057冻结。候选仅Established+Data+peer supports时附同一次PING的IMMEDIATE_ACK，无新PING/任务/计时器/重试；更早ACK可能增加独立datagram，实际成本/物理稳定性未证。当前候选尚未编译/验证，不作为6aa根因、手机或N1接受。上游未提交/未新核等价行为；以全四合同及移动/quiet成本复验退出，不能由接口同名删除。下文53/54/55和旧范围均为历史记录。

2026-11-11，进行中。**主53外新增Iroh45（合计54），42–44仍隔离预留未合主。45已在153263c2实现idle actor按失败时间撤回旧IP；Linux1e1264ef Noq439/path50/home11绿，Linux8adb26d3/Mac42869844599同canonical Node全12/hop44及实际live guard/错误身份回退绿。仅该实测scope接受；完整地址epoch、所有iroh测试与N5最终矩阵仍未完成。** 旧stub cf2352f7 home10/1红及所有初始失败保留，详见STATE及两freeze。

历史盘点：2026-11-10，进行中。当前53项：iroh 1–41、Noq 1–12；原48全部保留。新增Iroh38实际NAT tuple红→绿，Noq12 floor单元红→绿；Iroh39/40/41的单元已通过，但物理双端四秒、CPU稳定性/成本仍未收口，不等于完成验收。
当前Iroh41准确1514516e/tree94f45efd/snapshotf07db0c9在Turing035447实际fmt/439Noq/50路径/9选家绿；主分支cherry-pick为0c223ce4+724582b9。整源Mac42753148983准确e15651b7实际便携/IPC18/原32/独立process绿；不是vendor全套或最终Quality。Iroh40 fullsamples物理两次约5.4秒均四秒红，Iroh41完整物理仍待。CPU quiet旧at-most-one gate实际红及新1.3秒worst不能由单元/诊断绿关闭，准确版本见STATE。
覆盖盘点不等于行为验收或上游等价性验证，缺口保持未完成。下文旧run与旧计数是历史证据，不当当前状态。

上游只读对照为已校验源码包noq-proto1.4.0、commit
`34f75c49f93de5e2ff943483cabc5b6d0c5df918`；包/校验和/原版本源见[UPSTREAM](UPSTREAM.md)。
没有执行上游候选，也没有升级或删除补丁。下面“未证明等价”包括尚未完成调查，不能读成上游必然仍有同一缺陷。

## Noq 1–12

Noq4另补真实虚拟连接专项a_backup_probe_is_acked_when_the_responders_data_road_is_blackholed，
42539256792已SUCCESS 436/33/9；不是只检查计数器方法本身，要求备用路独立回ACK。
未对旧源取红，单向故障/兜底/成本仍需覆盖，不能提前把下表该项记完整。

| 编号 | 保留目标 | 当前覆盖及缺口 | 上游静态观察与退出条件 |
| --- | --- | --- | --- |
| 1 | BBRv3及时知道应用闲置，避免闲后输出慢 | KHOR记录旧congestion对照，但移植时没有带专属测试；435通用绿不能补这项缺口。需闲置→突发、loss/inflight一致性及与Cubic对照 | 1.4完整链已复核：poll_transmit更新path标志，BBR在on_end_acks才接收；下一发送先读自身idle状态。loss路径也未即时扣自身inflight，见UPSTREAM。已证实事件时序差异，最终体验/等价仍须隔离同seed突发与丢包验证 |
| 2 | OBSERVED_ADDR不能进入0-RTT，且可发送性判断一致 | 原accepted在恢复时禁用客户端reporting，未盖住始终both；新增reporting_waits_for_one_rtt真实握手测试，42555870179 Mac实际438Noq全绿。原帧准入不变，最终源实际早数据仍待 | 1.4缓存角色→pending→可发0-RTT STREAM→populate分支及接收端1-RTT门已逐段确认，见UPSTREAM；缺少本地等价保护，但未执行上游。始终启用reporting的恢复覆盖及两处准入等价后退出 |
| 3 | 发布不应答所需PTO、最近收到包、最早未确认包及无退避PTO | 黑洞Backup两用例实际读取这些stats，iroh有不应答判据覆盖；max_path_id_reached_lasts_until_a_path_is_abandoned只证明ID释放前提，不是stats专项测试 | 1.4公开PathStats和构造函数已核；4项接口无对应，qlog内部私有metrics不是公共替代。需要等价接口/调用者整体迁移。覆盖闲置、短暂停顿、ACK/PTO重置后退出 |
| 4 | 可自行发包的多路径ACK沿自己的路返回，避免共享坏回程污染全部路径 | path_acks仅断言出现PATH_ACK，不验证走哪条路；历史p3/p4及现有黑洞场景有集成覆盖。需补非对称单向故障与ACK额外开销专项证据 | 1.4 populate_packet仍在may_send_data路上枚举各包号空间的pending ACK，未具本地按本路/兜底路分发。升级候选须验证备用路可独立收到ACK、未验证/abandoned仍有兜底，兼顾包数预算 |
| 5 | 最大ACK号不是本次新确认时，不用旧发送时间取RTT | KHOR有7次旧网络异常采样证据；当前没有据此确认一条独立、构造该ACK序列的回归。泛化435绿不够退出 | 1.4完整ACK链已复核：最大号不在记录时仍抬new_largest，较低号新ACK非空可进入旧时间采样表达式。见UPSTREAM/RFC9002；需合法已发但回收记录的ACK序列及正常采样对照，尚未取得行为红绿 |
| 6 | 路径已有自身保活时，不被连接级保活额外ping | 新增真实虚拟连接保活计时回归42543928044绿：连接5秒不抢路径15秒，路径自身与无路径计时器对照均仍保活；无生产改动/旧源红宣称。当前源空闲包/字节与手机预算仍缺 | 1.4 ConnTimer::KeepAlive调用ping，ping遍历所有包号空间置pending_ping；未见本地跳过规则。等价分层计时且实测成本达标后退出 |
| 7 | 从显著慢Backup离开时及时重排在飞流字节 | what_is_in_flight_on_a_far_slower_backup_is_resent_on_the_new_path与健康同速不重排对照，435套件绿；实际f4/f4s仍有1.5秒超限，不声称整体验收完成 | 1.4状态改变/原PTO/丢失与discard入口已复核，见UPSTREAM；普通重传存在，但未证明主动重排时机/完整行为等价。须原流完整交付、MTU排除、正常loss/CC记账及晚包去重后退出 |
| 8 | 选路读最新ACK校正样本及计数，不用慢EWMA或把相同值当旧值 | khor_equal_rtt_samples_are_fresh_and_latest_is_not_the_smoothed_value、khor_selection_samples_do_not_count_peer_ack_delay_as_network_latency均在435绿；CPU调度偏差尚未闭合 | 1.4公开PathStats/构造函数已核，qlog RecoveryMetrics为私有且feature限定，没有公共latest样本/ACK计数替代。等值样本、ACK-delay可扣/不可扣、初始估计不计数及调用者覆盖后退出 |
| 9 | 历史同速但当前黑洞的Backup也释放在飞字节 | a_blackholed_backup_resends_in_flight_even_if_its_last_rtt_was_fast已432/1红→433绿；真实虚拟PTO与健康同速对照。不是应用请求重试 | 尚未证明1.4有等价策略；在相同两个可用路径/一条黑洞场景，正常CC/去重/短停顿保护和完整流交付均成立后退出 |
| 10 | 每份样本携带包发送/ACK处理时刻，读取stats不刷新年龄 | khor_sample_times_belong_to_the_ack_not_a_later_stats_read比较原传输数学，435绿；iroh积压旧包/延迟读取/初始估计回归也有红绿记录 | 1.4公开PathStats/构造函数均无对应采样时刻；内部qlog也不能替代，当前driver同样不提供内核收包时刻。等价接口须保持旧包不能确认新路况，不用读stats时刻冒充采样时刻 |
| 11 | 先移路、后满足不应答时，在既有Backup PTO重新检查 | a_backup_that_fails_after_the_path_move_releases_its_in_flight_data已434/1红→435绿；额外验证<300ms不重排、虚拟<1秒释放及新路收完整流。最新p3三轮<656ms，但不证明旧15秒全部定因 | 1.4恢复入口已静态核对，普通PTO/重传存在但未证明与本地主动重检行为等价；必须同时保留故障前/后移路两时序、健康同速与短停顿对照，不能只证明set_path_status时重排 |

| 12 | 近min RTT的有效ACK-delay不污染独立选路质量，夹至实测floor | 266实际438绿/1红；whole paths tests模块hash冻结。264/265物理准备失败及265八个质量值纳秒关联留证，271完整439/36/9绿与267单轮物理绿；稳定性/重复待核。原过量delay/首次采样语义保留 | 本轮尚未核新的上游等价行为；不动传输RFC数学，以近floor/无样本/过量delay和peer relay/稳定性/黑洞完整合同复验后退出 |

源码入口：[本地补丁账本](../../vendor/noq-proto/KHOR.md)、
[1.4固定connection/mod.rs](https://github.com/n0-computer/noq/blob/34f75c49f93de5e2ff943483cabc5b6d0c5df918/noq-proto/src/connection/mod.rs)、
[1.4固定stats.rs](https://github.com/n0-computer/noq/blob/34f75c49f93de5e2ff943483cabc5b6d0c5df918/noq-proto/src/connection/stats.rs)、
[1.4固定BBR](https://github.com/n0-computer/noq/blob/34f75c49f93de5e2ff943483cabc5b6d0c5df918/noq-proto/src/congestion/bbr3/mod.rs)。

## iroh 1–38：第一轮覆盖盘点与新增修正

目标来自[补丁账本](../../vendor/iroh/KHOR.md)，不把其中“上游没有”“尚待验证”等历史描述当作当前事实。
42536711138的435/33中的33仅选中remote_state::tests，遗漏net_report/socket的khor_测试。
随后control322835b补执行，**42537392595已SUCCESS 435/33/9**；9项包含下表23–27的命名测试，
下面“当前33未执行”是原过滤器边界，现已补齐这9项实际执行，仍不代表对应集成/真机要求完成。
下表“现有覆盖”是已定位的测试/场景，不自动标记其余新源通过，历史全量单元数不补未跑项目。
除1号去重外，尚未逐函数完成1.4.1等价实现核查；共同退出规则是隔离候选通过对应行为与调用者验证。

| 编号 | 保留目标 | 现有覆盖、仍需验证及逐项退出要求 |
| --- | --- | --- |
| 1 | 开路重试去重、有界；path ID释放后能继续 | path_ids_run_out_until_a_path_is_abandoned与Noq对应前提用例；1.4.1已有contains去重，64上限/释放事件恢复尚未证明等价，不能整块删 |
| 2 | 探测不自动读系统代理，显式代理仍有效 | 需系统代理/NO_PROXY/显式环境代理对照，原生Mac不能由Linux替代；与真实Relay传输策略一致才退出 |
| 3 | 尊重用户明确首选家中继 | slow_relay历史集成；需首选HTTPS可达/不可达及全断恢复，不能让低RTT绕过用户配置；等价配置接口后迁移调用者 |
| 4 | EndpointGone通知应用，及时处理走中继的离线线 | node/tests/departed、CLI diagnose；仍需最终源重启/异常断开/有Direct并行的对照，不能让登记事件误杀健康直连 |
| 5 | 明文→HTTPS等successor按配置迁移 | relay_https、relay_qad历史集成；需单端只能到旧地址、继任失败/恢复与地址发布。等价受控迁移接口后退出 |
| 6 | Relay先赢也打开已知Custom路径 | net/tests/carry::the_carried_road_is_opened_when_the_relay_answers_first；开启路径与最终是否胜出分开，按现速度策略复验，不恢复旧类型优先 |
| 7 | 直接地址watch保留来源，应用区分换网与观察变化 | journal接口变化/稳定化覆盖；需来源变但地址未变不唤醒重拨，真实换网能唤醒；等价公开API后退出 |
| 8 | 广播每轮打洞及实际候选供现有journal记录 | 端点候选过滤测试与p1/p5打洞计数依赖；需有/无订阅者、不因观察触发打洞，Lagged后事实可恢复 |
| 9 | 发布过滤与打洞候选过滤一致 | khor-net::a_punching_round_offers_no_unroutable_address；需受限出口/TUN/IPv6及用户过滤，不能只测试192/198网段字面量 |
| 10 | PeerObserved提供非QAD公网地址，限制来源/数量/有效期 | 需直连才接受、Relay/Custom不接受、回环/未指定地址拒绝、8条/10分钟淘汰及网络变化；当前没有明确专属通过证据 |
| 11 | 增量报告没量到当前家时不无故换家 | relay_https历史；需增量保留/完整报告失效/全断后重连，避免为防抖长期留死家；和24/25一起核对 |
| 12 | 活连接收到后来的Custom地址能当场开路 | f4i及hop比较集成；需重复offer不重复开路、actor/路径预算、取消/关闭释放，不能把最终未选Custom等同未开路 |
| 13 | 承载断开及时关闭对应Custom路径 | 原node hop9及CLI hop历史保留。2026-11-10真实late-close旧行为761c4291为hop24/1红，replacement已offer仍Closed；候选新增actual-actor同步条件guard，原无条件close保持。47aaa/4a4055/a61643同source node双OS全12/hop27及Linuxvendor439/50/9绿，真实actor已入队后replacement与无replacement合同也闭合；证据见两carried-close freeze。pending Noq/ears仍独立待验，不将该合同升级成整个代际矩阵通过。 |
| 14 | 完整net report等待HTTP-only中继的必要测量 | relay_https与混合QAD报告；需慢/不答QAD、IPv4/IPv6部分完成及总期限，不能无限等探测 |
| 15 | 每个Relay actor独立，发送/close不堵全局；慢Pong不误断 | relay_qad/relay_https历史集成；需TCP接通却不转数据、close不返回、一端不通及慢而可用路线；N4仍缺当前源完整故障证据 |
| 16 | 应用前台恢复可明确触发major network change | N2真实睡眠/回前台场景待齐；需重绑/重探合并、有界并发、不重复应用写；不能只测方法调用 |
| 17 | 周期测量、可达性、质量、确认与静默恢复 | remote_state 33、p2/p3/p7、f4/f4s覆盖；当前p1s和f4s仍红。250ms tick的设备规模/能耗、type-dependent quick与两层hold仍待诊断 |
| 18 | 选择器可留一条备用，闲备用保活减慢 | p1/p3/p4集成、Noq保活依赖；需client最多两Direct、不双端竞相关最后路、闲置包数/手机预算，当前源正式成本证据未齐 |
| 19 | 当前路退化时有界探测/打洞，缩短周期探索盲区 | Watch及p5/p7回归；由29/34统一所有承载和新证据，旧“首慢即打洞”已替代。退化恢复/额度/原故障通道均须覆盖 |
| 20 | 直接地址身份不随来源变化；相同候选不反复触发 | journal::an_address_that_changes_only_its_source_is_no_change及p4；需QAD↔Local标签切换的真实打洞/同步计数，来源信息仍可观察 |
| 21 | 路径事件缓冲64、公开单路径ping | 现路径诊断和故障前探测使用；需超过缓冲后的Lagged补记、慢消费者不阻塞网络、ping只触达指定路径的证据 |
| 22 | IPv6字面中继正确生成QAD服务器名 | p4双栈QAD替身；需实际v6回答而非只看测试ok，并核DNS/IPv4无回归；1.4.1等价性未核完 |
| 23 | 配置的公共fallback仅在其他中继无HTTPS回答时当家 | khor_the_fallback_tier_is_home_only_when_no_other_relay_answered已定位，未被当前33执行。需用户自定义同URL/首选例外/全断回退集成 |
| 24 | 家中继两边采用同类历史样本，按配置最短驻留 | khor_the_home_is_weighed_by_its_best_recent_latency_too、khor_a_home_that_answers_is_kept_for_the_dwell、hinton轨迹测试；当前33未执行，仍需故障立即离开而健康不抖动 |
| 25 | 接班只认HTTPS可达，频繁断开的接班者暂时冷却 | khor_reached_is_an_https_answer、khor_a_successor_that_keeps_dropping_is_left_for_an_hour；当前33未执行。需major change复位、无其他候选兜底、个人配置边界 |
| 26 | QAD连接观察值初始化正确，失败的单族探测不提前收场 | khor_the_qad_connection_is_reused、khor_a_failed_qad_probe_does_not_end_the_wait_for_an_address；当前33未执行。需部分IPv6失效的最多3秒预算及无反复握手 |
| 27 | 不发布容器/虚机网桥，但保留真实私网接口 | socket::tests::khor_container_and_vm_bridges_are_not_offered；当前33未执行，需名称反例/真实172网段及可用候选不被误删 |
| 28 | 所有承载按速度与新鲜证据迁移，不给Direct无条件Go | 首批66单元与p6对照，当前动态27绿；还需恢复与稳定性同时达标，退出必须保留用户速度优先 |
| 29 | 真实ACK计数、近期中位质量、活跃时有界轮转探测与共享缓存 | equal_rtt_samples…、active_refresh_is_bounded…、shared_quality_and_age…及net cleanup；需冷/活副本偏差、闲置开销与读取无副作用，p1s仍红 |
| 30 | iroh选路与Noq在飞重排共用不应答事实 | 原unanswered/静默测试与Noq黑洞两时序；需慢备份、同速健康/坏备份、稀疏收包，不能只证明接口能编译 |
| 31 | 使用采样包发送/ACK处理时刻判断新鲜性 | 初始估计/延迟读stats/旧包ACK已22/3红→26绿；静默旧包额外覆盖。当前不拥有内核收包时刻，CPU污染仍须实测 |
| 32 | 缩小窗口同步重算质量，空窗不假装新观测 | 26/1红→27绿，过期不改ACK计数/时刻；动态/闲置相位绿不替代最终成本与CPU覆盖 |
| 33 | 快分支也要求决定后往返，等待时复测当前路 | 27/2红→29绿；p1s仍3/1移动，不能以红绿断言共同调度停顿已消除；按同条件防抖与恢复一起退出 |
| 34 | 普通慢质量先在原探测预算内验证后打洞 | 29/1红→33绿，Watch版p5原门绿、p1s零打洞但选路仍红；实际故障快速通道/30秒节流及日志关闭的最终源都须复验 |
| 35 | 首次新往返恢复可用性，质量/类型不再门住；重复故障仍hold | 42539860644已436Noq绿/iroh33pass1fail，候选移除quick回调、不加timer/探测；原旧包/重复故障/闭路覆盖与普通防抖/恢复待新源验证。上游等价性尚未核完 |
| 36 | 故障离开也需决定后备用包，当前探测选最高PTO副本 | 42541459177已436Noq绿/iroh34pass1fail；候选双向PING、备用用最低PTO副本、4秒兜底不变；p3<1秒/防抖/额外包数及上游等价待验证 |

| 37 | 无当前copy接收提示时可用故障后已有备用应答快速离开 | 42551075037 Mac实际437Noq绿/路径35pass1fail，候选条件分支42555870179 Mac根格式438Noq/36路径/9选家绿；有提示保留36、旧包/4秒/正常确认不变，原p3/CPU/转发未收口 |

1.4.1固定源码与PR证据见UPSTREAM；本轮未把账本中的旧OPEN状态照搬成事实。
2/3/5/9/23含用户配置/策略，不能仅因上游有不同默认策略便删除；需明确等价配置能力和调用者迁移。
其余缺少专属覆盖的条目保持缺口，不以45行表格宣告N5达成。

## 审计后的动作顺序

1. N1先取得当前诊断证据并解决实际稳定性/恢复红；本页不自动批准升级。
2. 补Noq 1/4/5明确缺失的行为覆盖及Noq6实际空闲成本，与现有实际网络成本/恢复对照一起保留；不是为凑测试数重写实现。
3. 对照iroh 1–36账目补当前源覆盖，继续逐函数核上游，再在隔离升级候选核对公共API、锁文件和行为。满足一条退一条。
4. 所有补丁和调用者均不依赖本地实现、准确最终源验证齐全后，才可删整个vendor/patch行。当前N5仍部分完成。

诊断日志调用未作为第12/35条生产行为补丁计数；其默认关闭、用途和可删除条件已分别记入两份KHOR.md。


## 新增38：被动迁移tuple同步（候选，未闭合）

247真实NAT-only端口重绑、Noq实际tuple与iroh缓存69次不一致，124/124可达但warm remote新路选择0/30；整个fixture与严格数据合同/哈希已冻结在ACCEPTANCE和nat-tuple-red-green.json。候选在既有reselect前按实际路径身份重登记/清变更Reading/重apply，不改其他策略。待同整fixture转绿、vendor/便携与完整移动/转发/成本矩阵；上游等价事件/同步API及调用者行为未验证，不声称上游必然有缺陷，也不自动批准升级。原48项覆盖/退出缺口继续保留。


## 新增Noq补丁12候选（2026-11-09）

当前总数50（Iroh38+Noq12），原48仍全部在审计范围内。Noq12是近RTT下界ACK-delay选路偏差的独立修正，不改传输RFC数学/收益门。原42722555246单元438绿1红，264/265实物理准备失败及纳秒数值关联留证；whole paths tests模块hash与后续绿必须相等，真实vendor/peer relay/稳定性证据尚待。准确理由/退出条件见vendor/noq-proto/KHOR.md，不能把候选记作已验收。


| 39 | 无PTO但有判定后回包提示的已知故障路，有界取得新往返 | 002954原stats70条实证，267/272冻结四秒门红；最多3次/2秒/250ms预算、无hint/初始/健康不探测、自然样本/关闭/独立epoch单元已写，尚待远端 | 上游是否存在等价事件/预算尚未调查；需同四秒物理至少3轮、原stalls/黑洞及实际探测字节、调用者全覆盖后才退出。不能用可用性或单元绿代替性能验证 |


更新2026-11-10：主分支现52项（Iroh40+Noq12），Iroh40 vendor439/46/9绿，物理两四秒门仍红；隔离Iroh41为第53项候选未验证。40原因/退出见KHOR，41未合主分支。旧“51项”是历史编号状态，不能读成当前总数。

## Iroh40/41新增覆盖（2026-11-10）

| 编号 | 原因与实现边界 | 实际证据与退出条件 |
| --- | --- | --- |
| 40 | 有限确认timer避免250ms tick抖动跨过预约；同PTO优先同copy，低PTO/关闭仍可换，绝不保有连接强引用/永久timer | Mac42743081799 fmt/439/46/9；物理fc435ccd原3probe/250ms/2秒预算绿，但Go5392/5630ms红；匹配重复3d3ebe28为5385/5374ms红。须四秒/原stalls/探测实际成本与全部调用者验证，上游等价未查 |
| 41 | 故障恢复2秒内新packet显著下降才重启quality窗口，25%再次改善/最多3阶；packet-send epoch阻旧ACK，confidence按当前recent数 | Turing033054真实49绿1红→035447真实50全绿，整个tests SHA5c4da4b4不变；新策略物理与CPU/成本未接受，首次完整样本e3bf仅初连超时、无replug，保留不冒充恢复失败。须同源至少3轮双端四秒与原稳定性/黑洞/字节预算后退出；上游等价未查 |

Noq1/5缺少专门行为覆盖的状态不变。没有升级、删补丁或发布。

## Iroh45 cold IP hint候选

新的公共API只送既有actor，执行时重读live连接；拟只撤掉失败前的IP来源，不撤Relay/Custom和更新的地址。当前scaffold不改地址簿。证据锚3a7424b7（prepare完成1985ms仍UnknownIssuer），新state/live guard合同先红再实现，原439/50/9保持。首次失败边界不是完整network/address epoch，不替代N4/手机；42–44编号仍由隔离试验占用。


## Iroh46候选：已知故障前水平的首次恢复刷新

主vendor55（Iroh1–41/45/46、Noq1–12），42–44仍隔离；新增46未全面验收。原c775/eec09ec1 Noq439/path53pass2FAIL（旧50及3负控过）→1c847229/5c2e4ff2 Noq439/path55/home11绿，同whole ae62和旧测试尾。首个pre-fault真实Reading作为恢复期触发提示，真实新样本回到其水平只准首次重建质量窗口；不能把旧值当新样本/可达/收益证据。原3probe/250ms/2秒、Moving与gain/新包epoch/重hold保持，无新task/map/timer/强conn。详KHOR46及fixture freeze。双OSNode在跑，物理三组前瞻对照/quiet成本/动态/手机待，不升格unit结论。上游等价有限epoch/实测历史确认尚未核，不删除其他补丁或依赖。


## Iroh48：pending-handshake与其他连接的选择隔离（候选未验收）

| 保留目标 | 当前证据与缺口 | 退出条件 |
| --- | --- | --- |
| 新准备的carrier能作为一次握手的初始路，另一活连接不遮住；保留TLS、正常读取/写边界和总期限 | 874eb4c4与d521/9d655e7f冻结四peer各3个真实业务红，后者明确旧Relay的live注册保护。4eff候选新增显式地址API，未提供该地址拒绝；Node合同拟验证活IP旁新Custom Path0/TLS/echo、错误身份拒绝与旧IP存续，当前未执行。普通拨号<=2活动/先join再复用slot，早发替代不发early业务，失败不领取另一carrier预算；实际额外握手字节/CPU尚欠 | 上游同语义实现尚未核，不据“支持自定义transport”或方法同名认定等价。必须验证每次初始目的地址、有活连接负控、身份错误/暂停/取消/原deadline/零新增写重放、真实唯一/多个middle两腿、移动与N3成本。全部调用者可迁移后退出；不能用本项窄绿删除其他57项 |


## Iroh49：每connection单一已建立路径的有效传输观测（候选）

来源是源166的真实Node普通/early最终断言：内容成功/唯一响应，但路型Unknown；同源两端均重现，fastDirect/旧IP保持负控绿。Noq无Available时使用Backup；仅一个已建立路径且共享preferred未匹配时报告该路径，不将零/多路猜成某类型，不改Noq状态/共享质量/探测预算。原四合同与所有时限保持，同源Node、vendor、四peer/动态/成本还待。上游是否等价未核，不凭同名API退出；退出需真实多连接/数据/空多路边界和原预算、恢复/手机成本矩阵。

## Noq5专属ACK状态合同（2026-11-11，已写，未远端执行）

新增四个真实连接上的unit合同：先完成TLS并实际发出一个PING，不驱动对端应答，保留真实在飞账目；随后单位夹具沿原包号分配器及sent retention构造合法ACK-only尾部，达到原有回收门而不手工删除最大号。第一份ACK同时新确认仍保留的较低PING及已回收的ACK-only最大号，必须不增加RTT样本、不刷新原采样时刻。正常保留最大号需正确采样一次；仅新确认ACK-only不能采样；重复合法ACK不能再采样；从未分配的最大号必须拒绝，以排除只测到PROTOCOL_VIOLATION的假回归。

helper是connection子模块、仅cfg(test)，使用真实已发PING的metadata与实际retention/ACK处理器，未修改生产采样代码。新增尾部是单位账目夹具，**不是1000份真实线上ACK发包，也不证明上游网络重现或完整Noq5退出**；后续应以同一完整测试夹具在隔离的旧采样门取得预期红、候选绿，再补网络/恢复与全部vendor回归。原Noq1 idle→burst/丢包/Cubic对照和其余59审计范围不变。当前未编译/运行，不记绿。

2026-11-11 15:52 UTC追加：上述Noq5四合同在准确source-b717的Mac43050978778真实全部执行绿，整个Noq447/Iroh路径58/选家11绿（原件/身份/proof见STATE）。仅减少四个专属unit行为缺口，不接受Noq5完整退出/上游等价、Noq1或N5。下一步隔离Git对象仅去掉largest_newly_acked采样门，三个完整测试源相同，验证reclaimed断言确能红；尚未收运行结果。最新生产3de长稳de04的全部原4门已绿/32目录RPC全ok、CPU约束逐原件核实；严重f4s/quiet与手机/成本范围保留。

2026-11-11 16:00 UTC追加：同一完整fixture的单门去除Git对象在Mac43052478191得到预期 **446pass/1fail**，其余三个新控制全绿；唯一reclaimed测试在“不得产生RTT样本”断言实际6/期望5失败。准确11files/11controls/identity、exact13870301323、ZIP SHA3cfa138fc7db0f7ce70c47ccb63b516035dfb771dd3dd7ba56daa92cbea45115；相对candidate-b717只有1行guard去除、测试源SHA完全相同、feat/net保护未变。`ack-ablation-verification-v1.json`与candidate447全绿构成这项unit行为的可辨识对照；不是官方上游/历史整版对照，也不替代Noq1/5物理性能或完整59退出。
