# 单机自建网络服务

同一台服务器运行 `iroh-relay` 1.5.0、`iroh-dns-server` 1.5.0 和 Caddy。客户端导入一个 HTTPS 配置地址。部署可保留现有的 Khor 配对和设备身份。

1. 从 [iroh 发行页](https://github.com/n0-computer/iroh/releases/tag/v1.5.0) 安装对应架构的两个服务。为实际域名设置 A/AAAA，按各自确实可达的地址发布。替换模板中的 `network.example.org` 和 `devices.example.org`。为 Relay 安装该域名的证书和密钥，服务账户需要读权限。QAD 使用同一证书。
2. 复制 [relay.toml](relay.toml)，将已配对设备的 Endpoint ID 填入 `access.allowlist` 后运行 `iroh-relay --config-path /etc/khor-network/relay.toml`。也可改用 `access.shared_token`，将密钥保存到仅服务账户可读的配置，并在每台客户端分别为该 URL 填写。模板的空名单拒绝所有转发请求。
3. 复制 [lookup.toml](lookup.toml)，创建服务账户可写的 `data_dir`，运行 `iroh-dns-server --config /etc/khor-network/lookup.toml`。按 [Caddyfile](Caddyfile) 提供 HTTPS Pkarr 和配置文件。两个后台程序以独立的服务账户运行，由服务管理器负责重启。
4. 对客户端开放 TCP 443、TCP 3341 和 UDP 7842。先从 IPv4 和 IPv6 各自可用的客户端验证 `https://<域名>:3341/ping`。再用两台 Khor 设备验证仅凭设备身份寻址、实际通过 Relay 无直连传输，以及 QAD 发现的地址。这个部署用 Pkarr 完成发布与查询。若另外运行权威 DNS，按部署的区配置发布和委派，并将区填入配置的 `dns_origins`。
5. 把修改过的 [network.json](network.json) 放到 `/srv/khor-profile/network.json`。每台设备在「设置 → 网络 → 导入配置」输入 HTTPS 地址，或运行 `khor net import-url https://<域名>/network.json`。重启 Khor 或 `khor serve`，确认设置页显示已生效。

为可更新配置单独生成签名密钥，并通过可信渠道分发输出的公钥。

```sh
khor net keygen ./network-signing.key
khor net sign ./network.json 1 ./network-signing.key > ./network-signed.json
```

发布签名文件作为 `/srv/khor-profile/network.json`。客户端导入时追加公钥，后续使用「检查配置更新」或 `khor net refresh`。每次变更递增 revision，先生成，再原子替换发行文件。下载失败、签名错误、回滚都不会覆盖上次成功保存的配置。签名私钥和中继访问密钥分别保管。

多个站点可在一个配置中放多个 Relay URL 与 Pkarr URL。替换某个 Relay 时，可先用新版配置将新旧地址并列，下次所有设备应用后再去掉旧地址。配置 URL 自身变更时，用固定公钥导入新地址。Pkarr 服务默认处理设备签名记录，并受配置的写入限流约束，有额外访问要求时可放到私有网络。

可先在命令行运行 `khor net self-hosted` 生成空白的自管配置，或用 `khor net export` 导出当前设备的地址与选项。设备 Relay 密钥只留在本机。
