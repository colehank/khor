# vendor/noq-proto —— 为什么这里有一份 noq-proto

## 来源

- crates.io 上的 `noq-proto 1.3.0`，逐字拷自 cargo registry 解开的那份（iroh 1.2 钉的版本）。
  许可证：`MIT OR Apache-2.0`，不变（`LICENSE-MIT`、`LICENSE-APACHE` 在包里）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] noq-proto = { path = "vendor/noq-proto" }`。
  lock 里的版本仍是 1.3.0，只是来源换成了这个目录；`noq`、`noq-udp`、iroh 都不动。

## 补丁

一处：BBRv3 不知道连接闲下来了（2026-10-04，NET-5 轮）。

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
