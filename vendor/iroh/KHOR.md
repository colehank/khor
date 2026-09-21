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

只改了一个文件：`src/socket/remote_map/remote_state.rs`。

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
