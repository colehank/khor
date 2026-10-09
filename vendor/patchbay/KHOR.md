# vendor/patchbay —— 为什么这里有一份 patchbay

## 来源

- crates.io 上的 `patchbay 0.7.0`，逐字拷自解开的那份包
  （`.cargo_vcs_info.json`：上游提交 `5cc60db1eb190360dfcd64eaf2f829bce4072551`，`path_in_vcs = "patchbay"`）。
- 只去掉了 cargo 的 `.cargo-ok` 标记；`LICENSE-MIT` 与 `LICENSE-APACHE` 从上游仓库同一提交补来
  （包里没带；许可证 `MIT OR Apache-2.0`，不变）。
- 接进来的方式：根 `Cargo.toml` 的 `[patch.crates-io] patchbay = { path = "vendor/patchbay" }`。
  只有 `crates/cli` 的 Linux dev-dependency 用它（`tests/netlab.rs`，docs/NETLAB.md）；发行的二进制里没有它。

## 补丁

两处，都在 `src/nft.rs`，都为同一件事：让实验室跑在 turing 上（Ubuntu 20.04，Linux 5.4，nft 0.9.3）。
上游假定较新的内核；在 5.4 上原样的 patchbay 一个用到 NAT 的测试都起不来。

### 1. 全锥映射表只记内部地址，端口原样通过（2026-10-09，net-lab）

上游的 EIM（端点无关映射）用一张动态表 `外部端口 → 内部地址 . 内部端口`，入站时
`dnat to udp dport map @fullcone`。目标是「地址 . 端口」拼接的 NAT 要 Linux ≥ 5.6、nft ≥ 0.9.4；
5.4 上 nft 报 `unqualified mapping data type specified in map definition`，路由器建不起来。
补丁：表改为 `外部端口 → 内部地址`（`type inet_service : ipv4_addr`），更新时记 `ip saddr`，
端口不改写。EIM 在这里本来就保留端口（`snat to <wan>` 不带端口，内核只在冲突时才换），
所以外部端口就是内部端口，行为与上游一致；只有两台内部机器用同一个源端口出去、内核被迫换端口时才会分叉，
实验室里每台设备一个地址，碰不到。
验证：上游 iroh 的 patchbay 测试在 turing 上用这份 patchbay 跑，`holepunch_simple`、
`nat_moderate_x_moderate`、`nat_open_x_strict` 等全绿（62 绿、1 红、9 个上游自己标的忽略；
红的是 `switch_server_dual_to_v4`）。
升级 patchbay 时：若 turing 已换到 ≥ 5.6 的内核，这处可以丢掉。

### 2. 命名空间里没有 conntrack 的 sysctl 时，保留内核的超时（2026-10-09，net-lab）

上游在 NAT 规则之后写 `net/netfilter/nf_conntrack_udp_timeout` 等三项，失败即报错。
Linux 6.x 之前，属于非初始 user namespace 的网络命名空间根本不导出这些 sysctl
（`nf_conntrack_standalone.c`：「Don't export sysctls to unprivileged users」），文件不存在，
于是每个带 NAT 的路由器都失败（`sysctl write net/netfilter/nf_conntrack_udp_timeout`）。
补丁：文件不存在时记一条 warn，保留内核默认（5.4：UDP 30 s，UDP 流 120 s，TCP 已建立 5 天），照常继续。
代价：预设里的超时（比如 `IspCgnatSymmetric` 的 UDP 流超时）在 5.4 上不生效；测保活与映射超时的场景要记住这一点。
改动约 8 行。
