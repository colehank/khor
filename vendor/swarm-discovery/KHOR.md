# vendor/swarm-discovery —— 为什么这里有一份

## 来源

- crates.io 上的 `swarm-discovery 0.6.3`，逐字拷自 cargo registry 解开的那份
  （`.cargo_vcs_info.json`：上游 `rkuhn/swarm-discovery` 提交 `d7486668c71c580631b9d7faadd263eaa620a648`）。
  许可证 `Apache-2.0`，包里带着 `LICENSE.Apache_2.0`。不是新依赖：iroh-mdns-address-lookup 本来就依赖它。
- 只去掉了 cargo 的 `.cargo-ok`。包里自带的 `.gitignore` 忽略 `/Cargo.lock`，所以那个文件在磁盘上、不进仓库。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io]`；`vendor` 已在 workspace 的 `exclude` 里。

## 补丁

一处，`src/updater.rs`，**整个文件换成上游已合并、还没发版的修复**：
`rkuhn/swarm-discovery` PR #27（"fix/gc-task-shutdown"，提交 `bbff547299ae`、`bad0fd5b2070`，
合并提交 `b41dbcc431f4`，2026-07-13）。0.6.3 之后上游这一处之外没有别的改动。

### 1. 停掉的服务，它的 GC 定时器也停（2026-10-05，TERM 38）

0.6.3 的 `updater::gc` 每个周期往 updater actor 发一次 `Input::GC`；发不进去就 10 毫秒后再发，
**不看 actor 还在不在**。服务一停（`DropGuard` 被丢掉，actor 跟着被 abort），`send` 永远返回 false，
这个任务就每秒醒 100 次，直到进程退出。khor 每次关掉一个端点都会停掉它的 mDNS 服务
（`crates/net/src/endpoint.rs` 的 `follow` 修好之后才真的停），所以不打这个补丁，每关一次端点就多一个
100 Hz 的空转任务：回环上关掉 40 个端点后，进程稳定在 2% CPU（`audit/term-mdns.md`）。

上游的修法：重试前先看 `ActoRef::is_gone()`，actor 没了就不再发。

上游发了 0.6.4 或更新的版本、带上了这个修复，就删掉这个目录和 `[patch.crates-io]` 那一行。
