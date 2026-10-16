# vendor/noq-proto —— 为什么这里有一份 noq-proto

## 来源

- crates.io 上的 `noq-proto 1.3.0`，逐字拷自 cargo registry 解开的那份（iroh 1.2 钉的版本）。
  许可证：`MIT OR Apache-2.0`，不变（`LICENSE-MIT`、`LICENSE-APACHE` 在包里）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] noq-proto = { path = "vendor/noq-proto" }`。
  lock 里的版本仍是 1.3.0，只是来源换成了这个目录；`noq`、`noq-udp`、iroh 都不动。

## 补丁

五处。

### 5. `src/connection/mod.rs`：最大确认号不是新确认的，不取 RTT 样本（2026-10-17，path-fix）

`on_ack_received` 在 ACK 抬高了最大确认号、又新确认了一个要回 ACK 的包时取一个 RTT 样本：现在 − `largest_acked_packet_send_time`。
这个发送时间只在最大确认号还在 `sent_packets` 里时才更新；不在（已经不在记录里了）时用的是上一个最大确认号的发送时间。
netlab 里测到 7 次，全是这种：1 ms 的路上 4.67 秒、17.6 秒的样本，平滑 RTT 一下变成几百 ms，选路器据此把连接从好好的路上
挪走（p1 的来回换路、p2 起步时 beta 先跑去局域网）。RFC 9002 §5.1 要求最大确认号是这次新确认的才取样本。
补丁：记下最大确认号是否这次新确认（`largest_newly_acked`），不是就不取样本。改动 5 行。上游可以提（要点：RFC 9002 §5.1 的
第一个条件没检查，旧的发送时间会被当成这次的）。升级时要重打。

### 4. `src/connection/mod.rs`：一条路径的 ACK 走它自己（2026-10-17，path-fix）

上游把所有 PATH_ACK 都放在「最适合发数据」的那条路径上（`may_send_data`，即本端标 `Available` 的那条），备用路径的 ACK
从别的路回来。于是一条路径答没答、RTT 多少，量的其实是「这条路去 + 对方选的那条路回」：
一、对方选的路回程断了，这边每条路径都收不到 ACK、PTO 一起涨，连中继也像死了，选路器没有一条可换（netlab p3，10-17：
黑洞之后 alpha 的中继路径 PTO 涨到 8，连接在黑洞的路上停了 15 秒，直到路径空闲超时）；
二、两边各选了一条直连时，各自选中的那条只收得到对方 15 秒一次的备用保活，15 秒的路径空闲超时把它关掉（p1 的来回换路）。
补丁：能自己发的路径（多路径已协商、没放弃、有对方的 CID、已验证，`acks_on_own_path`）只带自己的 ACK，
不管它是不是 `Available`；不能自己发的（放弃了、没验证的）照旧由最适合发数据的那条代带（`acks_go_on`）。
三处：`space_can_send` 的 `acks` 只算这条路径会带的；`poll_transmit` 里自己的 ACK 到时（`pending_acks.can_send()`）
备用路径也发一个包；`populate_packet` 的 ACK 段按 `acks_go_on` 挑。握手中、没协商多路径时 `acks_on_own_path` 是假，
和上游一字不差。代价：备用路径保活 PING 的 ACK 在它自己上回，原来也多半是单独一个只带 ACK 的包，包数不变。
noq-proto 自己的 427 个单元测试（其中 44 个多路径）打上补丁后全过（拷到 disk0 上跑的，`cargo test --lib`）。
上游可以提（要点：多路径草案允许 PATH_ACK 走任何路径，但 RTT 和丢包判断都按路径算，ACK 走别的路会让一条路径的
存活和 RTT 掺进别的路径；建议能发的路径带自己的 ACK）。升级时要重打。

### 3. `src/connection/stats.rs`、`src/connection/mod.rs`：`PathStats` 带上 `pto_count`（2026-10-16，path-fix）

一条路径黑洞了，它的 RTT 估计不会变（只有确认才更新），从 `PathStats` 上看不出它已经不答；只有 PTO 计数在涨。
iroh 的选路器要据此离开它（vendor/iroh 补丁 17，`PathSelectionData::answering`）。补丁：`PathStats` 加 `pto_count: u32`，
`Connection::path_stats` 读时填上（同 `rtt`、`cwnd`），两处把 `PathStats` 解构的地方补上这个字段。改动约 10 行。
`PathStats` 是 `#[non_exhaustive]`，加字段不破坏外面的用法。上游可以提。升级时要重新打。

### 2. `src/connection/mod.rs`：OBSERVED_ADDRESS 不进 0-RTT（2026-10-09，NET-DIRECT，net-g）

khor 从 f6d6d7fd 起双向打开地址观察（`endpoint::transport()`）。客户端带着记住的传输参数恢复会话时，
`set_peer_params` 把 0 号路径的 `pending.observed_address` 置上，`populate_packet` 只看 `SpaceId::Data`，
就把这一帧写进了第一个 0-RTT 包。这一帧只许在 1-RTT 里（`Frame::is_1rtt`），对方以 PROTOCOL_VIOLATION
关掉连接，客户端听不到，重传的 Initial 落进对方正在排空的连接，约 6 秒后才重新握手，0-RTT 也丢了。
测得：回环中继上每一次「握手时带着请求」的拨号 6012–6022 毫秒（打补丁后 8–14 毫秒）；
`crates/cli/tests/journal.rs` 的 `a_relay_outage…` 因此时红时绿。

改两处：写 OBSERVED_ADDR 的条件加 `!is_0rtt`；`can_send_1rtt` 在 0-RTT 时不把这一帧算作有东西可发，
否则 debug 构建在「只写了 ACK」的断言上崩（测到过：常驻进程因此重启）。上游 1.3.0 同样如此，升级时要重新看。

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

上游发版带上这个修复后，这个目录和那一行 patch 就可以去掉。
