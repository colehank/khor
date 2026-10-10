# vendor/loro-internal —— 为什么这里有一份 loro-internal

## 来源

- crates.io 上的 `loro-internal 1.16.2`，逐字拷自 cargo registry 解开的那份
  （`.cargo_vcs_info.json`：上游提交 `af130d97d8b110add13283fc5bfc81d97a9a609f`，`path_in_vcs = "crates/loro-internal"`）。
- 只去掉了 cargo 自己的 `.cargo-ok` 标记；另从上游仓库根目录（同一提交）补了 `LICENSE`，
  因为 crates.io 的包里没带（许可证：MIT，不变）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] loro-internal = { path = "vendor/loro-internal" }`，
  `[workspace] exclude = ["vendor"]` 已有。依赖声明仍是 `loro = "1.16"`，lock 里的版本仍是 1.16.2，
  只是 loro-internal 的来源从 registry 换成了这个目录。
- 包里的单元测试不能单独跑：`src` 里引用的 `dev_utils` 没有随 crates.io 包发布。

## 补丁

一处。

### 1. `src/version.rs`：合并版本向量时不再整份深拷（2026-10-10，PEERS-BRIEF，peers-a）

`ImVersionVector` 是 `im::HashMap`（持久化 HAMT，克隆共享节点）。`extend_to_include_vv` 对另一份向量里的
每个 peer 调 `get_mut`；`im` 的 `get_mut` 不管改不改都把通往该键的路径复制一遍，于是对每个 peer 都调一次
就等于把整棵树复制了一遍。`AppDag::ensure_vv_for` 给每个有两个以上依赖的变更（并发后的合并）算向量时走这条路，
算出的向量缓存在该变更的 `OnceCell` 里，永不释放。所以每个合并变更都留下一份完整、不共享的向量：
32 路节点约 1 KB，peer 数上千时约 1.1 KB × peer。

改为：先 `get`，只有计数确实变大时才 `insert`。语义不变（原来那支 `get_mut` 只在计数变大时才写；
负计数归零后必小于正的 `counter`，同样走 `insert`）。

量（`crates/sync/examples/docstat.rs`，活堆字节，自带计数分配器）：

| | 补丁前 | 补丁后 |
|---|---|---|
| 合成 1500 peer，两副本每轮各写一笔再交换，每轮留下 | 1660 KB | 17.6 KB |
| 合成 3000 peer，同上 | 3272 KB | 17.6 KB |
| 合成 1500 peer（多为合并），快照载入后第一次并发导入 | +148 MB，57 ms | +7.6 MB，27 ms |
| 真 devices 堆（1515 peer，只读字节），载入加第一轮后的活堆 | 320 MB | 37 MB |
| 同上，之后每轮 | +1564 KB | +18 KB |
| 同上，第一次并发导入 | 54 ms | 24 ms |

测：`cargo test -p khor-sync --test peers_memory`（不打补丁两项都红：1660 KB/轮、148035 KB）。

上游议题草稿：`audit/loro-issue-peers.md`（负责人发）。上游修了就删掉这个目录和 patch 那一行。
