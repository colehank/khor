# vendor/iroh-mdns-address-lookup —— 为什么这里有一份

## 来源

- crates.io 上的 `iroh-mdns-address-lookup 0.6.1`，逐字拷自 cargo registry 解开的那份
  （`.cargo_vcs_info.json`：上游 `n0-computer/iroh-address-lookups` 提交
  `0f5f547cc65514e4537d4e66f776319ea8e30a2e`，`path_in_vcs = "iroh-mdns-address-lookup"`）。
- 只去掉了 cargo 的 `.cargo-ok`；`LICENSE-MIT`、`LICENSE-APACHE` 从上游仓库同一提交的根目录补来
  （crates.io 的包里没带；许可证 `MIT OR Apache-2.0`，不变）。不是新依赖：khor-net 本来就依赖它。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io]`，和 iroh 一样；`vendor` 已在 workspace 的
  `exclude` 里。依赖声明仍是 `iroh-mdns-address-lookup = "0.6"`。

## 补丁

一处，`src/lib.rs`，标着 `KHOR PATCH`。

### 1. 先拿原始 id 字符串比，再解码（2026-10-05，TERM 38）

上游 `MdnsAddressLookup::new` 的 actor 循环里，每收到一条 mDNS 应答（`Message::Discovered`）都先
`PublicKey::from_str`：base32 解码，再做一次 ed25519 点解压（`EdwardsPoint` decompress，
`sqrt_ratio_i`），然后才看这个 id 是不是自己、是不是已经知道且地址没变。同一网段上每个端点都回答
每个查询，所以同样几个 id 一遍遍到来，每一遍都解压一次。负责人跑了 14 小时的 app 里，这一段
是最大的一块（`audit/term-cpu.md`）。0.6.1 会更新缓存地址并抑制重复事件，公钥解析仍在缓存比较之前。

补丁：actor 留一张「原始字符串 → 解析结果」的表（`parsed`，解析失败的也记下，最多 1024 条，满了
清空重来），先和自己的 id 字符串比（`own_id`），表里有的直接取，没有的才解析一次。继续采用 0.6.1 的过期、地址替换和重复事件处理。底层 `swarm-discovery` 已从 crates.io 使用，
0.6.6 包含停止服务后退出 GC 重试的修复。

可以提给上游：行为不变，只是少做重复的解码。

升级这个 crate 时要重新打；上游修了就删掉这个目录和 `[patch.crates-io]` 那一行。
