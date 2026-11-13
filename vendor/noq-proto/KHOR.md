# vendor/noq-proto —— 为什么这里有一份 noq-proto

## 1.4 升级候选（2026-11-12，尚未验收）

本隔离树已将发布源码从1.3.0三方迁移至1.4.0；下文为原补丁历史，不代表仍基于旧版或全部必须保留。当前迁移与退出账目见[IROH-UPGRADE](../../docs/net-optimization/IROH-UPGRADE.md)。所有旧专属回归保留；新源编译和行为证据尚待，原绿不能沿用。

## 来源

- crates.io 上的 `noq-proto 1.3.0`，逐字拷自 cargo registry 解开的那份（iroh 1.2 钉的版本）。
  许可证：`MIT OR Apache-2.0`，不变（`LICENSE-MIT`、`LICENSE-APACHE` 在包里）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] noq-proto = { path = "vendor/noq-proto" }`。
  lock 里的版本仍是 1.3.0，只是来源换成了这个目录；`noq`、`noq-udp`、iroh 都不动。

## 补丁

以下按本地补丁编号记录。


### 13. 显式路径PING在已协商的1-RTT上要求立即确认（2026-11-11，feat/net，候选）

受控20ms实际网络往返、对端已协商30ms延迟ACK时，原ping_path只发PING，包虽收到，
20ms时仍无新ACK样本。54c1/bfe61241真实Noq442项441绿/1红，普通connection ping延迟ACK、
Initial握手两项负控及原439通过；新增真实TLS resumption早数据控制另行执行，未提前声称通过。
这是测量及时性缺陷，不是此前6aa承载流单向停顿的定因或N1接受。

仅在Established、Data空间且对端min_ack_delay表明支持时，沿同一个pending path probe
附加IMMEDIATE_ACK。原PING数量/合并、普通ping、所有连接/路径保活、PTO/拥塞与RTT数学不变；
握手和0RTT不附加，不替未协商对端开启扩展。没有新timer/task/重试/应用重放，原3次/250ms/2秒预算
与收益25%/Moving2秒/四秒合同不改。更早ACK可能减少合并、增加实际ACK datagram，需计量成本，
不能以PING数量不变推导总包数/能耗不变。独立path_probe模块whole fixture与旧行为结果冻结，原multipath不改。

兼容依据是本地实现引用的[ack-frequency-08第3/5节](https://www.ietf.org/archive/id/draft-ietf-quic-ack-frequency-08.html#section-3)，
这是固定历史draft，不声明当前标准版本。候选尚待全vendor/双OS Node与物理重复/稳定性/quiet成本，
更不替代phone/energy。上游未提交、未核新的等价实现；上游提供显式path probe同等及时性后，
以延迟ACK/普通ping/握手/真实0RTT及原移动/成本矩阵复验退出，不凭函数同名删除。


### 12. 选路质量在ACK-delay校正越过已测下界时夹到下界（2026-11-09，feat/net）

264/265实际peer-relay准备失败，四端outer均Direct约41–42ms，而inner quality约109ms/public Relay约131ms，既有25%收益门不足，两look后回收。265全部8个Custom比较值按纳秒与同resident此前唯一Noq样本相等：如raw109.520837ms/delay25ms/min85.622430ms/sample109.520837ms。原样本的raw-delay略低于min，传输RFC估计器保留raw，选路也把25ms确认等待全记作网络延迟。此为数值关联；原Noq日志无conn/path ID，不冒称逐包绑定或唯一移动根因。

新增单元42722555246真实438绿/1红：实测85ms后raw109ms/ACK-delay25ms，选路原109ms而合同要求85ms。候选仅在已有实测且0<raw-delay<min时，将独立选路latest_sample夹至已测min；正常校正、首次无样本及delay>=raw仍保守。传输adjusted_rtt/SRTT/方差/PTO/拥塞/线上协议不变，25%收益门和确认不改；整个Noq paths tests模块字节不变。后续270（旧质量）/271（候选）采用同actual-latency fixture及相同paths tests模块，全部Rust diff只本段质量修正；270仍438绿/仅floor1红，271 fmt/439Noq/36iroh path/9home全绿。267原physical fixture 124/124ok、reported Carried再hold；单轮不能代替原稳定性/完整移动与手机能耗验收。

本地未提上游；上游具备独立、保守的ACK-delay路径质量语义后，以近下界/过量ACK-delay/首次样本、完整vendor与peer-relay/稳定性/黑洞场景复验后退出，不凭同名字段删除。

### 11. 已经移路之后失效的Backup也在原PTO上重检（2026-11-09，feat/net）

补丁9只在status改变时重排；如果先移路、旧路之后才达到不应答门，它的流字节仍等在旧路。
真实虚拟计时回归42529470019取得434pass/1fail，原故障后移路/健康同速/慢Backup测试均绿，
新增先移路用例失败在在飞数据未释放。实际p3此前也有15秒超时，但本回归不证明其全部原因。

沿现有Backup LossDetection timeout将重排检查标记交给下一次poll_transmit，仍使用同一
unanswered_for与速度判据、MTU排除、原丢包/拥塞计量及QUIC去重。无独立timer、无应用请求重放，
健康同速与未满300ms停顿不重排；新增用例还验证原PTO释放后健康路收到完整流字节。
候选42529931673单元435Noq/29iroh全绿；真实p3/f4/稳定性仍需新源证据。仅在status改变或Backup恢复事件检查，不在每次读stats/发送时扫描。
本地未提上游；上游具备等价后移路故障重排后，以本组及p3/f4/短停顿覆盖复验退出。

### 10. RTT样本携带发送与ACK处理时刻（2026-11-09，feat/net）

读取stats的actor可能晚于ACK处理，不能让读取时刻变成新的测量证据，也不能让故障前发出的包确认故障后的往返。
`RttEstimator::update_at`使用原来的包发送/ACK处理时刻，调用原`update`执行相同RTT/ACK-delay/PTO/拥塞数学，
另外保存`sample_times`，由PathStats发布`latest_rtt_sent`/`latest_rtt_received`；初始估计没有时刻，重复读取不刷新。
这里的received是协议处理ACK的时刻，不是内核接收时间，不能据此声称消除全部CPU影响。

Noq元数据/传输数学一致性回归在42518379039为434全绿；同run的iroh三项新鲜性回归22pass/3fail，
证明读取延迟、初始估计变化和旧包ACK仍被旧Reading误算新证据。iroh补丁31修复后42519038936为434Noq/26iroh全绿；实际网络仍需对应源码回归。
本地未提上游，本轮未检查最新上游实现；上游提供等价采样时刻后，以这些回归及p1/p2/p3/p7/f4复验退出，不能只看字段同名。

### 9. 同速但不应答的Backup也释放在飞数据（2026-11-09，feat/net）

黑洞路径不再取ACK样本，历史RTT仍可能是20ms。原补丁7只对“明显更慢”的Backup重排，
移向同速健康路径时在飞流数据仍留在旧路。新增真实虚拟计时多路径回归：永久移除第一条路、
保持第二条路可达，两条历史RTT都20ms；两次PTO及350ms后移路，旧版不能立即重排。
42512359677取得产品红（432pass/1fail），不是编译或准备失败。

`PathStats::unanswered_for`统一原iroh的两次PTO、max(300ms,3PTO)等待与最近收包保护。
`resend_off_backups`在路径状态改变后，对明显慢或已经不应答的Backup把在飞可重传帧重新排队；
保留MTU探测排除、原丢包/拥塞计量和重复包去重，健康同速路径仍不重排。不是重新发起应用请求。
iroh补丁30调用同一个判据，避免选路认为已坏而重发层仍按旧RTT认它快。

候选验证状态见STATE；该编号不表示已通过实际p3。上游尚未提案，本轮未核查其最新实现；
上游具备等价恢复行为后，以同速黑洞/健康同速/慢Backup及实际p3/f4复验决定退出。

### 8. 路径RTT公开最新ACK样本与计数（2026-11-09，feat/net）

`RttEstimator` 对每个合法RTT更新递增计数（相同数值也递增）；初始化估计不算样本。
`PathStats` 新增 `latest_rtt` / `rtt_samples`，iroh可以区分新测量和旧平均值，避免备用恢复后仍被旧慢值困住。
平滑RTT、PTO、拥塞数学与线上协议不变；后续校正候选：`latest_sample` 保存与平滑器同一次更新所用的ACK-delay校正样本，避免把对端延迟确认算作网络变慢；原raw `latest`、保守RTT、PTO和拥塞数学不变。新增 `khor_selection_samples_do_not_count_peer_ack_delay_as_network_latency` 覆盖可扣与不可扣的ACK-delay。此候选由p2/p3重复回归触发，尚待验证，不能直接称其为该回归定因。
回归 `khor_equal_rtt_samples_are_fresh_and_latest_is_not_the_smoothed_value`，全noq单元与iroh p7动态场景。
新候选未验证；上游未提交。上游具备同等观测接口后，在原恢复/防抖场景复验并移除字段补丁。

### 7. `src/connection/mod.rs`：离开慢得多的路时，它上面在飞的东西在新路上重发（2026-10-23，carry）

QUIC 的流按序交付：一段字节走了慢路，后面走快路的全要等它。netlab f4（手机到 turing 的直连黑 3 秒，中继发给手机的晚 1.7 秒）：
turing 到手机的连接在黑洞里改走中继约一秒，腿通后一个 tick 就回了直连，可 hop 那条流在这段上又 1.5 秒什么新东西也交不出来，
等的是 turing 走中继时发出、晚 1.7 秒才到的那些字节（黑洞里丢在直连上的包，判丢以后的重传也是在中继上发出去的）。
补丁：本端有路径的状态变了（`set_path_status`；iroh 每次改选都会设），下一次 `poll_transmit` 先跑 `resend_off_slow_backups`：
凡是 Backup、而往返不小于此刻 Available 里最快那条的两倍加 10 ms（`KHOR_FAR_BY`）的路径，把它上面在飞的、要回 ACK 的包当作
丢了（`handle_lost_packets`：流数据重新排队，从可用的那条发出去；不含在飞的 MTU 探测）。晚到的原包是重复，接收端丢掉。
代价：每次从慢路换到快得多的路，慢路上在飞的数据（至多一个拥塞窗口）多发一遍，慢路的拥塞窗口按一次丢包减；
新连接打洞成功、从中继升到直连时也会这样，那时中继上在飞的东西改从直连送到，本来就是要的。两条差不到两倍加 10 ms 的路之间
什么也不重发（直连之间不受影响）。改动约 70 行（含一个只在测试里用的 `khor_set_rtt`）。
测试：`src/tests/multipath.rs` 的 `what_is_in_flight_on_a_far_slower_backup_is_resent_on_the_new_path`（不推时间，改状态后
那一次发送就重发了；去掉补丁红）、`what_is_in_flight_on_a_backup_of_one_speed_is_not_resent`；noq-proto 自己的单元测试 430 个全过。
netlab f4 改前改后见 vendor/iroh/KHOR.md 补丁 17 的「10-23 修订（carry）」。上游可以提（要点：多路径下换到快路时，慢路上在飞的
数据会把按序的流堵住一个慢路往返；草案允许在任何路径上重传）。升级时要重打。

### 6. `src/connection/mod.rs`：连接级的保活只 ping 没有自己保活的路径（2026-10-17，path-fix 第三棒）

2026-11-09新增覆盖候选`a_path_keepalive_is_not_shortened_by_the_connection_timer`：真实虚拟连接，
连接5秒、路径15秒，6秒时不应多PING、16秒时路径自身仍保活；没有路径计时器的对照仍由连接5秒保活。
禁用MTUD排除探测PING混入，无生产修改；42543928044已SUCCESS 437Noq/35iroh路径/9选家与根格式。
未宣称旧源取红或证明整个人网空闲/手机能耗预算。

上游连接级保活（`ConnTimer::KeepAlive`，iroh 设的 5 秒）一到时就 `ping()` **每一条**路径（源码里的 TODO 自己也说太粗）。
iroh 给每条路径都设了路径级的保活（`default_path_keep_alive_interval`，备用直连、排在直连后面的中继是 15 秒，vendor/iroh
补丁 18），可连接级和选中那条路径的 5 秒是同一刻复位的，谁先到看运气：先到的那次把备用、中继也一起 ping 了，它们自己的
15 秒形同虚设（netlab p4 打点：备用直连和中继实际约 9 秒一个 PING，两边都发，每个再一个 ACK）。
补丁：连接级保活只给没有自己保活的路径置 `pending_ping`；有的由它自己的定时器管（`PathTimer::PathKeepAlive`）。
iroh 的路径都有，所以连接级保活在 khor 里等于不发包；连接本身的空闲超时由选中那条 5 秒的保活顶着。改动 10 行。
noq-proto 自己的 427 个单元测试打上后全过。上游可以提（要点：`ping()` 的 TODO；路径级保活应当盖过连接级的）。升级时要重打。

### 5. `src/connection/mod.rs`：最大确认号不是新确认的，不取 RTT 样本（2026-10-17，path-fix）

`on_ack_received` 在 ACK 抬高了最大确认号、又新确认了一个要回 ACK 的包时取一个 RTT 样本：现在 − `largest_acked_packet_send_time`。
这个发送时间只在最大确认号还在 `sent_packets` 里时才更新；不在（已经不在记录里了）时用的是上一个最大确认号的发送时间。
netlab 里测到 7 次，全是这种：1 ms 的路上 4.67 秒、17.6 秒的样本，平滑 RTT 一下变成几百 ms，选路器据此把连接从好好的路上
挪走（p1 的来回换路、p2 起步时 beta 先跑去局域网）。RFC 9002 §5.1 要求最大确认号是这次新确认的才取样本。
补丁：记下最大确认号是否这次新确认（`largest_newly_acked`），不是就不取样本。改动 5 行。上游可以提（要点：RFC 9002 §5.1 的
第一个条件没检查，旧的发送时间会被当成这次的）。升级时要重打。

### 4. `src/connection/mod.rs`：一条路径的 ACK 走它自己（2026-10-17，path-fix）

2026-11-09覆盖补齐候选：`a_backup_probe_is_acked_when_the_responders_data_road_is_blackholed`
建立两路后将响应端备用路设Backup，黑洞其主数据路，独立PING备用；断言ACK从健康备用本路回来。
这是此前path_acks只数总PATH_ACK没有覆盖的行为；42539256792已SUCCESS 436Noq/33路径/9选家。
不把新增用例当作已在旧源取红。
非对称单向故障、未验证/abandoned兜底及额外包数预算仍需另外覆盖。

上游把所有 PATH_ACK 都放在「最适合发数据」的那条路径上（`may_send_data`，即本端标 `Available` 的那条），备用路径的 ACK
从别的路回来。于是一条路径答没答、RTT 多少，量的其实是「这条路去 + 对方选的那条路回」：
一、对方选的路回程断了，这边每条路径都收不到 ACK、PTO 一起涨，连中继也像死了，选路器没有一条可换（netlab p3，10-17：
黑洞之后 alpha 的中继路径 PTO 涨到 8，连接在黑洞的路上停了 15 秒，直到路径空闲超时）；
二、两边各选了一条直连时，各自选中的那条只收得到对方 15 秒一次的备用保活，15 秒的路径空闲超时把它关掉（p1 的来回换路）。
补丁：能自己发的路径（多路径已协商、没放弃、有对方的 CID、已验证，`acks_on_own_path`）只带自己的 ACK，
不管它是不是 `Available`；不能自己发的（放弃了、没验证的）照旧由最适合发数据的那条代带（`acks_go_on`）。
三处：`space_can_send` 的 `acks` 只算这条路径会带的；`poll_transmit` 里自己的 ACK 到时（`pending_acks.can_send()`）
备用路径也发一个包；`populate_packet` 的 ACK 段按 `acks_go_on` 挑。握手中、没协商多路径时 `acks_on_own_path` 是假，
和上游一字不差。
代价（10-17 第三棒用 PATHDBG 量的，第二棒原来写的「包数不变」不对）：一条没被选中的路径收到 PING，回的 ACK 现在是这条
路径上单独一个包，原来它和别的路径的 ACK 挤在选中那条的同一个包里；在中继路径上它还走 TCP，中继那头回一个 TCP 的 ACK。
netlab p4（双栈，两条连接，各有一条中继、一条备用直连）300 秒：只关补丁 4 时 QUIC 包 3.05 个/秒，开着 5.74 个/秒
（同时还多了一条中继路径，两次不完全可比），线上（alpha 网卡）1713 对 3269 个包。
为什么不只对直连生效：noq-proto 看不出哪条是中继——iroh 把中继映射成 `fd15:…:12345` 这样的普通 UDP 地址，
要分就得经 `noq::Path` 加一个按路径的开关，而 noq 没有 vendor。改为从源头少发：iroh 补丁 18 让排在直连后面的中继
和备用直连一样 15 秒一个保活，补丁 6 让连接级保活不再顺带 ping 它们；之后 p4 是 3.64 个/秒（同一窗口），
线上 1816 个包对改前 1498 个（300 秒，两次不同批，多出来的主要是双栈下留着的备用直连）。同一批二十个场景并排跑的
600 秒（改前的树和改后的树同时在 turing 上）：p4 线上 2607 个包 635 KB 对改前 3383 个 791 KB，p1 2303 / 259 KB 对
2638 / 359 KB，p1j 3008 / 487 KB 对 3195 / 571 KB，都比改前少。
noq-proto 自己的 427 个单元测试（其中 44 个多路径）打上补丁后全过（拷到 disk0 上跑的，`cargo test --lib`）。
上游可以提（要点：多路径草案允许 PATH_ACK 走任何路径，但 RTT 和丢包判断都按路径算，ACK 走别的路会让一条路径的
存活和 RTT 掺进别的路径；建议能发的路径带自己的 ACK）。升级时要重打。

### 3. `src/connection/stats.rs`、`src/connection/paths.rs`、`src/connection/mod.rs`：`PathStats` 带上 `pto_count`、`last_received`（2026-10-16、10-17，path-fix）

一条路径黑洞了，它的 RTT 估计不会变（只有确认才更新），从 `PathStats` 上看不出它已经不答；只有 PTO 计数在涨。
iroh 的选路器要据此离开它（vendor/iroh 补丁 17，`PathSelectionData::answering`）。补丁：`PathStats` 加 `pto_count: u32`，
`Connection::path_stats` 读时填上（同 `rtt`、`cwnd`），两处把 `PathStats` 解构的地方补上这个字段。改动约 10 行。
`PathStats` 是 `#[non_exhaustive]`，加字段不破坏外面的用法。上游可以提。升级时要重新打。

**10-17 补 `last_received`**（path-fix 第三棒）：这条路径最后一次收到包的时刻（`PathData::last_received`，
`on_packet_authenticated` 里记下，`path_stats` 读时填上）。两次 PTO 在 1 ms 的路上只要约 75 ms，比机器卡一下还短：
netlab 二十个场景一起跑时（turing 负载 40–110），两条路都好好的，PTO 计数也到过 2、3。iroh 补丁 17 据此要求
「两次 PTO，且这条路至少 300 ms 什么也没收到」才算不答。改动 6 行。上游可以和 `pto_count` 一起提。

**10-21 补 `unacked_since`、`pto`**（net-flap）：这条路径上最早还在飞、要回 ACK 的那个包是什么时候发出的（各个包号空间里
`sent_packets` 中第一个 `ack_eliciting` 的 `time_sent`，取最早），以及它此刻的 PTO（`rtt.pto_base() + max_ack_delay_for_pto()`，
不带退避），都在 `path_stats` 读时算。「300 ms 什么也没收到」在闲着的路上恒真（5 秒才一个保活的 ACK），实际只剩两次 PTO，
2 ms 的路上 ACK 晚 0.1 秒就判死（真机一天两百多次「改走中继」又当场改回）；iroh 补丁 17 的 10-21 修订改为从这个包发出算起等
max(300 ms, 3·PTO)。改动约 15 行（读时扫一遍在飞的包，到第一个要回 ACK 的为止）。上游可以和 `pto_count` 一起提。
`src/tests/multipath.rs` 另加了一个测试 `max_path_id_reached_lasts_until_a_path_is_abandoned`（三条路径到顶后 `open_path` 报
`MaxPathIdReached`，关掉一条、对方回了 MAX_PATH_ID 之后第四条开得出来）：证 iroh 补丁 1 的 10-21 修订的前提，不改行为。
noq-proto 自己的单元测试 428 个（含这个）打上后全过。

### 2. `src/connection/mod.rs`：OBSERVED_ADDRESS 不进 0-RTT（2026-10-09，NET-DIRECT，net-g）

khor 从 f6d6d7fd 起双向打开地址观察（`endpoint::transport()`）。客户端带着记住的传输参数恢复会话时，
`set_peer_params` 把 0 号路径的 `pending.observed_address` 置上，`populate_packet` 只看 `SpaceId::Data`，
就把这一帧写进了第一个 0-RTT 包。这一帧只许在 1-RTT 里（`Frame::is_1rtt`），对方以 PROTOCOL_VIOLATION
关掉连接，客户端听不到，重传的 Initial 落进对方正在排空的连接，约 6 秒后才重新握手，0-RTT 也丢了。
测得：回环中继上每一次「握手时带着请求」的拨号 6012–6022 毫秒（打补丁后 8–14 毫秒）；
`crates/cli/tests/journal.rs` 的 `a_relay_outage…` 因此时红时绿。

改两处：写 OBSERVED_ADDR 的条件加 `!is_0rtt`；`can_send_1rtt` 在 0-RTT 时不把这一帧算作有东西可发，
否则 debug 构建在「只写了 ACK」的断言上崩（测到过：常驻进程因此重启）。上游 1.3.0 同样如此，升级时要重新看。

2026-11-09追加覆盖候选`address_discovery_reporting_waits_for_one_rtt_on_resumption`：双方地址报告始终both，
恢复连接实际发早数据但暂不驱动服务端，要求OBSERVED_ADDR尚未发送；之后握手接受、流字节完整且1-RTT报告仍发送。
原accepted测试在恢复时禁用客户端报告，不能替代该配置。新测试未编译/执行，无生产改动、无旧源红宣称；
1.4缓存参数/发送/接收调用链差异见[UPSTREAM](../../docs/net-optimization/UPSTREAM.md)。

### 1. BBRv3 不知道连接闲下来了（2026-10-04，NET-5 轮）

上游 `n0-computer/noq` 还没有对应的修复（截至 2026-10-04 最新 1.3.0；它开着的 #802 修的是另一件事：
初始 pacing 太快）。修复在 MoQ 的分支 `moq-dev/noq` #5（合于 2026-09-25，进了 `moq-noq 1.3.1`），
它的主干和 1.3.0 已经分叉，补丁打不上去，所以按它的内容手抄了三处源码（测试没抄，它们依赖那边新加的
测试设施）：

- `src/congestion.rs`：`Controller::on_app_limited(in_flight)`，默认什么也不做。
- `src/congestion/bbr3/mod.rs`：BBRv3 实现它（窗口没满时把 `C.app_limited` 标到
  `delivered + in_flight`，草案 §4.1.2.4 MarkConnectionAppLimited）；丢包时把 `inflight` 减掉丢掉的字节。
- `src/connection/mod.rs`：一次发送轮询什么也没发、也没被谁拦着时，马上告诉拥塞控制器。

为什么要它：一条只有打字的连接，BBRv3 把空闲后的那一阵输出按它上回见过的带宽去配速，
所以一屏输出比 Cubic 慢两到三倍（`crates/net/tests/congestion.rs`，audit/net-2.md）。

上游含等价修复后，仅评估退出本条补丁；整个vendor及patch行必须等全部本地补丁都有
等价覆盖、准确源回归与退出证据后才可删除，不能用BBR一项退出替代其余补丁审计。

### 诊断说明：ACK采样分量（2026-11-09，feat/net）

`RttEstimator::update_at`用`khor_net_samples` target记录采样包发送/ACK处理时刻、raw RTT、
传入ACK-delay、min RTT、既有校正值和实际样本计数。只观察已有计算，不改任何数学/时钟。
默认CLI无subscriber；可选net-diagnostics构建显式开启后用于识别CPU停顿与ACK校正偏差，
日志模式不作为SLO成绩，完成定因或有等价低开销诊断后可移除。
