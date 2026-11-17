---
title: 中继
summary: 自动连接，或运行自己的中继
order: 5
---

设备会优先直连。中继帮助交换地址，并在无法直连时转发流量。中继内容采用端到端加密，中继只负责转发，不存储或读取内容。

## 默认配置

无需配置。Khor 优先使用自己的中继，以 n0 公共中继作为后备。每台设备会选择附近可达的中继。

## 运行自己的中继

使用已入网、具有公网地址和域名的设备：

```sh
khor relay --acme relay.example.com --only-mine
```

`--acme` 从 Let’s Encrypt 获取证书。`--only-mine` 只允许网络中可信、未被撤销的设备连接，无需共享令牌。

| 选项 | 作用 |
|---|---|
| `--acme <domain>` | 获取 Let’s Encrypt 证书，在 443 端口提供 HTTPS |
| `--tls-cert <cert> --tls-key <key>` | 使用已有证书 |
| `--dev` | 默认在 3340 端口提供纯 HTTP，仅供测试 |
| `--only-mine` | 只允许网络信任的设备 |
| `--token <secret>` | 要求共享令牌，不能与 `--only-mine` 同用 |
| `--rate <bytes-per-second>` | 限制每个客户端的上传速率 |
| `--metrics <address>` | 暴露监测指标 |
| `--stats` | 输出用量汇总，不启动中继 |

## 使用中继

中继设置保存在各台设备本地。一台设备不能修改另一台设备使用的中继。

- 应用中：**设置 → 网络 → 添加中继**。
- 通过 `KHOR_RELAY`：用逗号分隔 URL；需要时增加 `token` 参数。

```sh
KHOR_RELAY="https://relay.example.com?token=secret,https://relay2.example.com"
```

令牌只保存在本地。每台通过受令牌保护的中继访问你的设备，都需自行配置令牌。`--only-mine` 可免去这一要求。

开启 **仅使用自己的中继** 后，将停用默认的 Khor 与 n0 中继。

## 用量汇总

```sh
khor relay --stats
```

只保存汇总信息：每台设备的连接次数、在线时间，以及中继收发字节数。不记录设备间的连接关系。
