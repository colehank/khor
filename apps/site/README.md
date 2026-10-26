# khor.cloud

官网和文档，一个静态站。中继和寻址不在这里：它们是常驻服务，各有自己的机器和
`relay-*.khor.cloud` 记录（见 `deploy/cloudflared.yml` 头上那段）。

- 值从 `docs/design-system/tokens.json` 生成（`scripts/tokens.mjs`），图标读
  `apps/gui/src-tauri/icons/src/`，这里不留第二份。
- 文档一篇一个文件：`src/content/docs/*.md`，`order` 定先后，`summary` 是总览里那一行。
- `public/install.sh` 是 `curl -fsSL https://khor.cloud/install.sh | sh` 的入口，
  只把 `scripts/install.sh` 指到本站的镜像 `/dl`。

```sh
npm ci
npm run dev        # 本地看
npm run build      # 出 dist/
```

## 部署

源站在境内一台机器上，只听回环口；外面经 Cloudflare Tunnel 进来，机器不开公网端口。

那台机器上，一次：

```sh
cloudflared tunnel login
cloudflared tunnel create khor-site        # 凭据放到 /etc/cloudflared/khor-site.json
cloudflared tunnel route dns khor-site khor.cloud
cloudflared tunnel route dns khor-site www.khor.cloud
cp deploy/cloudflared.yml /etc/cloudflared/config.yml && cloudflared service install
caddy run --config deploy/Caddyfile         # 或者挂成服务
```

之后每次发布，在有这个仓库的机器上：

```sh
deploy/push.sh <ssh主机>
```

它构建页面，把一个已选定发行版同步进 `.mirror/releases/<tag>/`，再上传到远端独占暂存目录。
远端重新校验压缩归档摘要、gzip 和程序头；完整目录落位后才原子切换 `/dl/VERSION`。
既有版本保留，已选旧版本的客户端可以继续下载。页面同步保护 `/dl`，不删除它的历史版本。
部署端需要支持 `--protect-args` 的 rsync，远端需要 sh、gzip、SHA256 工具与基础文件命令。
出了新发行版也跑这一条，镜像才跟上。布局、失败恢复和旧镜像迁移见
[镜像维护](../../scripts/release-mirror.md)。
