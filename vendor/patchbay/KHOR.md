# Khor 使用的 patchbay

源码来自 crates.io 的 `patchbay 0.8.0`（n0-computer/patchbay 提交
`03245af2422e3d833200f21b99fe69936fb3fdca`）。保留 MIT / Apache-2.0 许可证。
只有 `khor-cli` 的 Linux Netlab 测试依赖它。根 `Cargo.toml` 使用本地补丁。

- `src/nft.rs` 使用「外部端口 → 内部 IPv4 地址」的 DNAT 映射。Turing 的
  Linux 5.15 可以使用命名空间内的 conntrack 超时设置，但实验环境的 nft 0.9.3
  无法解析上游的「IPv4 地址 . 端口」映射。现有场景使用保留端口的 NAT，
  实验工具更新并能加载上游规则后即可移除这处补丁。
- `src/router.rs`、`src/lab.rs` 和 `src/core.rs` 提供
  `RouterBuilder::public_v6()`。校园网和移动网场景需要私有 IPv4 经 NAT、
  同时具有可路由的全球 IPv6。上游 0.8.0 仍共用一个地址池选项。支持按地址族
  选择下游地址池后，可以改用上游接口。
