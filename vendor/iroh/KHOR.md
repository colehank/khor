# vendor/iroh —— 为什么这里有一份 iroh

## 来源

- crates.io 上的 `iroh 1.2.0`，逐字拷自 cargo registry 解开的那份
  （`.cargo_vcs_info.json`：上游提交 `17c0612f80f78f5288e97b818b1360ae6ea0a51a`，`path_in_vcs = "iroh"`）。
- 只去掉了 cargo 自己的 `.cargo-ok` 标记；另从上游仓库根目录（tag `v1.2.0`）补了
  `LICENSE-MIT` 与 `LICENSE-APACHE`，因为 crates.io 的包里没带（许可证：`MIT OR Apache-2.0`，不变）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] iroh = { path = "vendor/iroh" }`，
  以及 `[workspace] exclude = ["vendor"]`。依赖声明仍是 `iroh = "1.0"`，lock 里的版本仍是 1.2.0，
  只是来源从 registry 换成了这个目录。

## 补丁

十五处。

### 15. `src/socket/transports/relay/actor.rs`：一条中继连接不再卡住所有中继；慢的 pong 不算断（2026-10-09，NET-DIRECT，net-g）

MacLab 10-09 约 11:36 起，经中继什么也发不出去：iPhone 的 Initial 从家中继 aps1 进来（13:07 测得约 57 KB），
一个字节也没回；每分钟一次的拨号从没打开过北京或 pk 的连接；家也再没换过。原因是三处没有时限的等待：

1. 连接 actor 空闲 60 秒正常退出时 `client_sink.close().await`。tokio-websockets 的 close 要读到服务器关流为止，
   回程死了（代理 TUN 把 TCP 一直托着，euc1 那条 5233 进 / 50245 出冻结了 40 多分钟）就永远等下去；
   这期间它不再应答任何消息，收件箱却开着，不会被回收。
2. 中继 actor 给一台没有连接的中继发数据报前，挨个问现有连接「见过这台机器没有」（`HasEndpointRoute`），
   `rx.await` 没有时限：问到上面那条就永远等下去，整个中继 actor 跟着停，所有中继的发送、换家都停了。
3. 某条连接的队列满了，中继 actor 也一直等它腾出位子。

改为：close 最多等 1 秒；每条连接的回答最多等 250 毫秒，不答当作没见过；队列满了最多等 1 秒，之后丢掉这批
数据报（同 UDP 丢包）。

另一处（同一文件）：周期 ping 的 pong 时限从「上一次 pong 的 3 倍，至少 0.5 秒」改为固定 5 秒（上游的最大值）。
经 TUN 到 aps1 的 pong 在 124–976 毫秒之间跳，临时节点 6.5 分钟里被判断开、重拨了 3 次，每次约 4 秒，
期间中继告诉对方这台机器走了。网络变化后的那次快速检查仍按 RTT。

测试：`crates/net/tests/relay_stuck.rs`（一条吞掉 Close、此后两个方向都不再转发的路；之后 alpha 要能拨到第三台
中继上的 gamma，回环中继上的 delta 要能拨到 alpha 的家），打补丁前两项都超时，之后 7–14 毫秒；
`crates/net/tests/relay_jitter.rs`（隔一个 pong 压 700 毫秒），两分钟里打补丁前连了 8 次，之后 1 次。
上游 1.3.0 三处相同，升级时要重新打；中继服务器那一侧（iroh-relay 的 `server/client.rs`）对客户端也用 3 倍 RTT，
khor 自己的中继要改得 vendor iroh-relay，没做。

### 14. `src/net_report.rs`：完整报告要等不答 QAD 的中继量完（2026-10-09，NET-DIRECT，net-f）

上游的完整报告一拿到两个 IPv4 QAD 回答就停掉其余探测（`have_enough_reports`），不答 QAD 的中继（khor 自己的
三台都是 `quic = None`）的 HTTPS 探测还没回来就被丢下，这一轮报告里没有它，于是永远当不了家。MacLab 10-09：
代理改走 DIRECT 后 n0 的 aps1、euc1 都答 QAD，11:36:41 起家一直是 aps1（经 TUN 握 TLS 0.75–4.1 秒），
北京（18 毫秒）再没被量过；补丁 11 只管增量报告，管不到这里。补丁：完整报告里，只要还有不答 QAD 的中继没有
HTTPS 延迟，就不走两个 QAD 的捷径（仍可按「每台都有 HTTPS」收场）；它的探测都结束了这一轮自然收场。
代价：不答 QAD 的中继若是死的，完整报告要等它的三次探测超时才定家。改动约 15 行。
测试：`crates/net/tests/qad_home.rs`（net-g）。两台答 QAD 的替身（`KHOR_PUBLIC_RELAYS` 的 `url|端口`，
`localhost` 自签证书，每个方向加 100 毫秒）加一台不答 QAD 的明文中继；打补丁前家是替身（报告 210 毫秒就收场，
没有 HTTPS 一项），打补丁后家是明文中继（同样 210 毫秒，它的 HTTPS 1–2 毫秒已在报告里）。注意：上游的捷径看
netwatch 的 `have_v6`，一个 ULA（如 Tailscale 的 fd7a::）就算有 IPv6，这时捷径还要一个 v6 的 QAD 回答。
上游 1.3.0 没有对应改动，升级 iroh 时要重新打。

### 13. `src/socket/remote_map/remote_state.rs`、`src/socket.rs`、`src/endpoint.rs`：转连的路断了，当场关掉自定义路径（2026-10-09，NET-DIRECT）

转发（经第三台 khor 机器）的中间那台冻住时，khor 的转连链路结束了，iroh 却不知道：自定义路径还开着，`RoadOrder`
按它最后的往返（1–2 ms）继续选它，直到路径空闲超时（`PATH_MAX_IDLE_TIMEOUT` 15 s）把它放弃，才回到中继。
三台机器的回环测试（`crates/cli/tests/hop.rs`）量得冻住到回中继 19.9 s；把那条隧道线的静默上限从 15 s 降到 10 s
（`wires::HOP_SILENT`）单独不改这个数，因为线退了路径还在。补丁：`Endpoint::close_carried(EndpointId)` 让该远端的
actor 在每条活着的连接上关掉所有自定义路径（`RemoteStateMessage::CloseCustom`）；noq 当场发出 `Abandoned`，
iroh 随即重新选路。连接只剩这一条路径时 noq 拒绝关，照旧等空闲超时。khor 这边在转发链路结束、且没有新链路顶上时
调用（`carry::let_path_go`：`hop::open_hop` 的守望任务、`serve_hop` 的 `Ask::From`、ssh 转连的 `OneCarry::run`
与 `carry_hosted`）。量得冻住到回中继 12.9–13.2 s（三次）；ssh 会话断掉到回中继 1.8 s（三次，未打补丁时 15.9 s）。
路径关了以后，ssh 重连上来的链路由 `carry::offer_path`（补丁 12）当场在活着的连接上再开。
改动约 35 行。上游没有对应 PR，升级 iroh 时要重新打。

### 12. `src/socket/remote_map/remote_state.rs`、`src/endpoint.rs`：连接活着时交来的转连地址当场打开（2026-10-09，NET-DIRECT 第四步）

上游收到新地址（`ResolveRemote`，`connect` 时发）只记下，不在已有连接上开路；补丁 6 只在连接加入时补开自定义
地址。于是一条已经落在中继上的连接，永远用不上之后才搭起来的转发（经第三台机器）。补丁：`ResolveRemote` 里的
自定义地址在每条活着的客户端连接上 `open_path_ensure`（已开的不重复开）；`Endpoint::offer_addrs(EndpointAddr)`
不拨号、只交地址。khor 这边 `hop::compare_roads` 搭起转发后调用它，由 `RoadOrder` 按往返决定走不走。
改动约 25 行。上游没有对应 PR，升级 iroh 时要重新打。

### 11. `src/net_report.rs`：增量报告量不到家中继时不换家（2026-10-09，NET-DIRECT）

增量报告只量握着 QAD 连接的那台中继（`have_enough_reports` 一个 v4 回答就够），`add_report_history_and_set_preferred_relay`
只在这一轮量到的中继里挑，旧家不在这一轮里时也不走「新的要快三分之一才换」的滞回。MacLab 10-09 整夜：完整报告
（5 分钟一次）量到北京、住北京，25 秒后的增量报告只量到 n0 的 aps1、住 aps1（audit/net-d.md §1.4）。补丁：
**增量**报告没有量到当前的家，就留在家里；完整报告照旧可以换（家真死了，下一次完整报告或网络大变时离开）。
函数多一个参数 `full`，上游自己的表测试跟着传 `true`。改动约 15 行。上游没有对应 PR，升级 iroh 时要重新打。

**补一处（2026-10-09，net-h）**：`full` 原先只看 iroh 的 `do_full`，可不是 `do_full` 的报告也可能把每台中继都量一遍：
上一轮什么都没量到时，`ProbePlan::with_last_report` 退回初始计划。中继都不答 QAD 时报告就是这样一轮空、一轮全量交替。
家中继停了以后，全量那一轮只量到别的中继，补丁 11 却当它是增量报告，把死了的家留住，直到 5 分钟一次的完整报告。
`crates/cli/tests/relay_https.rs` 因此常红（TLS 中继停了 60 秒还住在它上面）。改为：`do_full`，或者计划是初始计划
且每个探测都跑完了（没被 QAD 捷径截断），就算量全了。临时打点的一次运行：停后三轮全量报告都只量到明文中继，
家都留在 TLS；改后第一轮全量报告就回到明文中继（35–48 秒）。

### 10. `src/socket/remote_map/remote_state.rs`、`src/socket.rs`：对端看到的地址（2026-10-09，NET-DIRECT）

QUIC 的 observed address 报告（QAD 在中继上用的就是它）iroh 默认两边都关，开了收到也不管
（`NoqPathEvent::ObservedAddr => {}`）。于是一台机器只能从答 QAD 的中继得知自己的公网地址；khor 自己的中继
都不答。补丁：直连路径（`to_transport_tuple(..).is_ip()`）上收到的对端观察地址记进一张共享表
（`PeerObserved`，十分钟有效，最多 8 个，回环和未指定地址不收），`update_direct_addresses` 在本机网卡地址之后
把它们作为 `DirectAddrType::PeerObserved` 加进直接地址——从此作为打洞候选给出去、也随行发布。中继和转连路径
上的观察地址是映射地址，不收。khor 这边 `endpoint::transport()` 打开收发两向的报告；旧版对端不协商、不发，
不受影响。改动约 50 行。验证：回环上看不到（对端看到的是回环，不收），要在机群上看连接记录「本机地址变为」
里多出的地址。上游没有对应 PR，升级 iroh 时要重新打。

### 9. `src/socket/remote_map/remote_state.rs`、`src/address_lookup.rs`：打洞只给发布过滤留下的地址（2026-10-09，NET-DIRECT）

上游 `local_candidates` 把所有直接地址都作为打洞候选（QNT ADD_ADDRESS）给对端，代理 TUN 的 198.18.0.1 也在
里面（MacLab 10-09 的连接记录）。补丁：先过端点的发布过滤（`AddressLookupServices::filtered`，就是 khor
`endpoint::unroutable` 那条），发布什么就给什么。改动约 25 行。测试：`crates/net/src/endpoint.rs` 的
`a_punching_round_offers_no_unroutable_address`（读补丁 8 的打洞轮；在有 198.18.0.1 的机器上对照：iroh 的
直接地址里有，打洞候选里没有）。上游没有对应 PR，升级 iroh 时要重新打。

### 8. `src/socket/remote_map/remote_state.rs`、`src/socket/remote_map.rs`、`src/socket.rs`、`src/endpoint.rs`：每一轮打洞（2026-10-09，NET-DIRECT 第一步）

`Endpoint::holepunch_rounds()`：iroh 每开始一轮打洞（`do_holepunching`，连接加入、任一方候选变化、
没有 10 ms 以内的直连时每 60 秒）就广播一个 `HolepunchRound`：对谁、本机给出的候选、对方给出的候选。
上游只有 tracing 事件 `iroh::_events::qnt::init`。khor 这边 `journal::watch_ears` 订阅，写成连接记录的
「打洞」一行（候选变了就写，不变时最多十分钟一行，带上这期间的轮数），`--report` 据此说「为什么不是直连」。
打没打通看随后有没有直连路径打开（`Connection::path_events`，`roads::follow`）。改动约 30 行：
`RemoteMap::new` 多一个参数（broadcast 发送端），`RemoteStateActor` 存一份。
测试：`crates/node/src/journal.rs` 的 `rounds_are_written_on_change_and_then_sparingly`（只测节流）。
上游没有对应 PR，升级 iroh 时要重新打。

### 7. `src/endpoint.rs`：带来源的直接地址（2026-10-08，NET-STABLE 第二步）

`Endpoint::watch_direct_addrs()`：把 `watch_addr` 里那组直接地址连同来源（`DirectAddr::typ`：
本机网卡 / QAD 学到的公网 / 端口映射）给出来。iroh 1.2 导出了 `DirectAddr` 和 `DirectAddrType`，
却没有公开的取法，`watch_addr` 只给不带来源的地址。khor 要的是「本机网卡地址变了」——iroh 自己的
网络监视看到换网时重绑、重探，khor 不知道，退避照等；QAD 学到的公网地址会自己变，不能算换网。
改动 8 行，只是把内部的 `ip_addrs()` 露出来。khor 这边 `journal::watch_ears` 订阅，稳定 1 秒后
告诉常驻（`Change::Addrs`）。测试：`crates/node/src/journal.rs` 的
`interface_addresses_are_told_once_they_hold_still`（只测去抖；回环上没法真的换网卡）。
上游没有对应 PR，升级 iroh 时要重新打。

### 6. `src/socket/remote_map/remote_state.rs`：中继先答时也打开自定义地址（2026-10-08，ROAD-3 轮）

`handle_msg_add_connection` 里，客户端在连接建好后把已知的中继地址补开成额外路径——上游只在
路径 0 不是中继时这么做，而且只补中继。khor 的自定义传输（`khor_net::carry`，ssh 里的 khor
和经第三台机器的转发）跟中继一起拨号：中继先答时路径 0 是中继，自定义地址从此不会被打开，
「先转发、后中继」（`carry::RoadOrder` 的开局三秒）就落空。补丁：不论路径 0 是什么，已知的
自定义地址都补开；中继仍只在路径 0 不是中继时补。改动约 10 行。
测试：`crates/net/tests/carry.rs` 的
`the_carried_road_is_opened_when_the_relay_answers_first_and_the_faster_carries`（回环中继加一条每字节慢
120 ms 的链路：中继先答，补丁前这条链路从没被打开，补丁后它被打开；2026-10-09 起选路按往返挑，
这条慢链路打开后由更快的中继接着走）。上游没有对应 PR，升级 iroh 时要重新打。

### 5. `src/net_report.rs`：中继的接班者（2026-10-04，NET-7 轮）

`NetReportConfig` 多一个 `successors: Vec<(RelayUrl, RelayUrl)>`（旧，新）；`Client` 存下它；
`add_report_history_and_set_preferred_relay` 在补丁 3 之后再加一段：家要是某个「旧」，而这一轮
报告量到了它的「新」，家就换成「新」。为的是北京那台换 HTTPS：同一台机器上两个中继进程（明文
3340、TLS 3341），延迟差不多，iroh 自己按延迟挑会随机落在其中一个；补丁让量得到 TLS 那台时
一律住它，量不到时照旧住明文那台。放在补丁 3 后面，所以首选了旧地址的人也跟着换到新的。改动
约 15 行。khor 这边 `endpoint::successors` 传进来（`KHOR_SUCCESSORS` 加 `KHOR_RELAY_SUCCESSOR`，
两台都在中继表里才算）。

**多久换过去**：iroh 只在「完整报告」里量中继——增量报告的探测计划是空的
（`probes.rs` 的 `with_last_report`，带着上游自己的 `// TODO: is this good?`），于是下一轮看到
上一轮一台都没量到，又从头量一遍。所以实际是每两轮周期报告（20–26 秒一轮）量一次：回环上量到
TLS 那台启动后 44–50 秒住过去、它停了 40–46 秒住回明文那台；绑新端点（启动、手机回前台）时
当场就量。测试：`crates/cli/tests/relay_https.rs` 的
`the_tls_relay_takes_over_renews_in_place_and_gives_way_when_it_stops`。上游没有对应 PR，
升级 iroh 时要重新打。

### 4. `src/socket/transports/relay/actor.rs`、`src/socket.rs`、`src/endpoint.rs`：中继说谁走了（2026-10-04，NET-5 轮）

中继服务器在一个端点断开时，给之前收过它包的端点发 `EndpointGone`。iroh 1.2 收到只是从内部的
`endpoints_present` 里删掉，不告诉任何连接，也不告诉应用。补丁：`HomeRelayWatch` 多一个
`departed: broadcast::Sender<EndpointId>`，收到 `EndpointGone` 时发出去；`Socket` 留一份，
`Endpoint::relay_departures()` 给出订阅。改动约 30 行。khor 这边 `link::Node::set_ears` 订阅，
`wires::Wires::departed` 把走中继的那条线当场收回、记下「刚走」，此后拨它只给短时间
（3 秒或 6 个往返），直到又听到它。
测试：`crates/node/tests/departed.rs`（两台只能走回环中继，一台关耳朵，另一台的状态词立刻离开
「中继」，问它 3 秒内失败；不接这个补丁时要等 10 秒的拨号）；`crates/cli/tests/diagnose.rs`
（进程被杀：状态词 14 秒 → 4 毫秒，第一次问 14 秒 → 3 秒）。上游没有对应 PR，升级 iroh 时要重新打。

### 3. `src/net_report.rs`：首选中继（2026-10-03，NET-2 轮，负责人批）

`NetReportConfig` 多一个 `preferred_relay: Option<RelayUrl>`；`Client` 存下它；
`add_report_history_and_set_preferred_relay` 最后加一段：这一轮报告量到了它（`relay_latency`
里有它，即答上了），就把 `preferred_relay` 定成它——家中继于是住在它上面。量不到时照 iroh 原来
的选法（最近 5 分钟延迟最低、新的要快过旧的三分之一才换）。iroh 1.2 没有选家中继的旋钮，只有
这个函数决定家，所以只能补在这里；改动 15 行。khor 这边经 `endpoint::bind_preferring` 传进来，
值来自 `.khor/relays.json` 的 `preferred`（设置 → 网络 → 中继 → 首选中继）。
测试：`crates/cli/tests/slow_relay.rs` 的 `a_preferred_relay_is_home_while_it_answers`（两台本机
回环中继，偏好远的那台就住远的，远的那台没了就回到近的）。上游没有对应 PR，升级 iroh 时要重新打。

### 2. `src/util.rs`：探测的 HTTP 客户端不读系统代理（2026-10-01，负责人定）

`reqwest_client_builder` 里加一行 `.no_proxy()`。iroh 的 net_report 探测（量各中继延迟、
强制门户）和 pkarr 发布用的是 reqwest；khor 工作区为额度页给 reqwest 开了 `system-proxy`，
cargo 的 feature 合并后这些探测也读 macOS 系统设置里的代理，而 hyper-util 读系统代理时不看
例外表。真正的中继连接只读环境变量（`proxy_from_env`），直连是 UDP 根本不经代理——三条路
三种规矩。`no_proxy()` 只关掉「自动读系统代理」，调用方在有 `proxy_url`（来自环境变量）时
仍 `.proxy(..)` 加上去，所以探测从此和连中继同一条规矩：只认环境变量。
账在 docs/NET.md「代理」一节；这条上游没有对应 PR，升级 iroh 时要重新打。

### 1. `src/socket/remote_map/remote_state.rs`

- 用的是上游 PR **#4522**「fix(iroh): bound and deduplicate pending_open_paths」的改法
  （PR 头提交 `eaca51d4c003f8a9f3764bd7ff7eca1a8dbdc309`，基于 main；截至 2026-09-21 仍是 OPEN，未合并）。
  它修的是 issue **#4390** 和 **#4509**（两条都还 OPEN）。
- PR 的十个 hunk 里，九个原样打上；第一个（新增 `OPEN_PATH_RETRY_DELAY` 与 `MAX_PENDING_OPEN_PATHS`
  两个常量）因为 1.2.0 在那里少了 `DATAGRAM_SEND_TIMEOUT` 几行上下文而没对上，内容一字不改地手工放在
  `ACTOR_MAX_IDLE_TIMEOUT` 之后。最后那个 hunk（测试模块）打在了文件末尾。
- 没有用 issue #4509 里那段更短的 hunk（只去重+限长）：#4522 除了去重和限长，还让报了
  `MaxPathIdReached` 的连接不再重试（path id 只增不减，那条连接再也开不了新 path），这正是我们遇到的那种情况。
- 想看补丁本身：`git show` 那个只动了 `vendor/iroh/src/socket/remote_map/remote_state.rs` 的提交，
  它的上一个提交就是原样拷贝。

## 为什么要 vendor

bug：同一个 remote 有 ≥2 条连接、都开不出新 path 时（`RemoteCidsExhausted` / `MaxPathIdReached`），
每条连接都把同一个地址重新塞回 `pending_open_paths`，每 333ms 一轮，队列按「连接数的 n 次方」增长。
RemoteStateActor 从此在 `open_path_on_all_conns` 里空转，排在它后面的消息（包括 `Endpoint::remote_info`）
一直等不到回复，内存一路翻倍。

在 khor 里的样子（docs/handoff/001.md 里 smoke §25 那一节有全部读数）：beta 对 alpha 同时有同步连接和
隧道连接，两边在同一台机器上，互相公布的候选地址里只有回环走得通（本机发往自己局域网地址、tailnet 地址的
UDP 到不了，连系统自带的 `/usr/bin/nc` 也一样），开往其余候选的 path 一直失败。从这里怎么一步步走到
`MaxPathIdReached` 没有逐帧追，读数只有结果：`sample` 看到 actor 所在的 worker 线程 263/266、389/393、
308/310 个样本都在 `open_path_on_all_conns` 里；bridge 的 `devices` 请求（→ serve 的 `Op::Hops` →
`remote_info`）从几十毫秒涨到 52950ms，然后再也不回来。

上游没有带修复的版本可升，所以只能在本地打补丁。

## 怎么验

补丁自带三条单元测试（`pending_open_paths_*`）。这个目录不是 workspace 成员，`cargo test --workspace`
不会跑它们；要跑：

```sh
cargo test --manifest-path vendor/iroh/Cargo.toml --lib pending_open_paths
```

它会用上游自己的 `Cargo.lock` 和 dev-dependencies（第一次要联网拉）。
**在 git worktree 里跑会报「believes it's in a workspace」**：worktree 嵌在主仓目录下，cargo
往上找到了主仓的 workspace。那就把这个目录拷到仓外再跑。

打补丁前（1.2.0 原样 + 同样的测试，排队逻辑按原样抽成一个函数）红句：

```
queue grew to 256 entries after 4 ticks
```

## 什么时候删

上游发出包含 #4522（或等价修复）的 iroh 版本后：

1. 把依赖升到那个版本；
2. 删掉整个 `vendor/` 目录、根 `Cargo.toml` 的 `[patch.crates-io]` 段和 `exclude = ["vendor"]`；
3. 跑全套门，再跑 smoke。

在那之前升级 iroh，要把新版本重新拷进来、重新打这个补丁，并更新本文件的「来源」。
