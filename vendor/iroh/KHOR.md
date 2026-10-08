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

七处。

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
`the_carried_road_is_opened_and_kept_when_the_relay_answers_first`（回环中继加一条每字节慢
120 ms 的链路：中继先答，补丁前路径一直是中继，补丁后开局内换到链路上、过了开局不回中继、
链路断了中继接着用同一条连接）。上游没有对应 PR，升级 iroh 时要重新打。

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
