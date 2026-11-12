# GitHub Pages 发布 · 2026-11-12

用户在网站本地迭代完成后要求部署 GitHub Pages，随后询问通过既有 Cloudflare / cloudflared 设置解析域名。这是网站静态产物与目标域名的发布授权；不合并 main，不启动原生构建。

## 目标与当前状态

- 源码 worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`，branch `feat/site-story`。
- 目标仓库：公开仓库 `colehank/khor`；仅发布静态快照到独立 `gh-pages` 分支，Pages source 为该分支根目录。
- 首次地址：`https://colehank.github.io/khor/`。Cloudflare 自定义域名接入完成后另用根路径配置构建。
- 当前：已完成产物准备与本地子路径验证，等待执行首次发布。

## 实现与校验

- `deployment.mjs` 统一 `KHOR_SITE_URL` / `KHOR_SITE_BASE`；`sitePath` 用于页面与运行时 URL，原片段锚点与外链不变。Sätteri 原生插件调整 Markdown 的站内链接与安装命令，使用 visitor context 写入属性；没有引入 remark 管线。
- GUI 从自身 URL 推导所属网站根路径，固定下载回复、允许点击的按钮和测试网页链接均跟随该根；虚拟 `/api` 仍在原网站 fixture 内拦截，不需要服务器 API。
- 输出包含原安装器的静态副本，将默认版本 pin 为网站已有的 `v0.2.0-rc.12`，保留 `KHOR_VERSION` 覆盖；未执行安装器。GitHub 当前 release 为 prerelease，`/releases/latest` 返回 404，六项已有下载资产已由 release API 重新核对。
- 当前 `khor.cloud` 的 NS 为 Cloudflare 的 `vicky` / `vida`，根域 A 查询无记录。仓库中只有指向本机 8787 的 cloudflared 模板；当前 Mac 未找到 Cloudflare 凭据或 cloudflared 命令。CUA 浏览器与原生管道均不可用，已告知用户自行打开 Cloudflare 登录，尚未更改任何 DNS 或 tunnel。
- 本地完整 Node 网站构建、网站运行时脚本 TypeScript、demo TypeScript 及 shell 语法校验通过。163 条生成的本站链接均存在且带 `/khor/` 前缀；严格静态服务器下 Chrome 检查首页、Fog、真实 GUI、固定回复下载按钮、CLI 复制、文档与下载页，无页面错误或本机资源 404。

## 产物与证据

- 产物：`/Volumes/disk0/Khor/artifacts/site-pages-20261112/dist`。
- 验证预览：`http://127.0.0.1:23822/khor/`，仅本任务静态检查。
- 证据：`/Volumes/disk0/Khor/evidence/site-pages-20261112`，含 `build.log`、`link-audit.json`、`staged-checks.json`、截图和 release 资产清单。
- 原开发预览 `http://127.0.0.1:23820/` 仍按根路径工作。

参考：[Astro GitHub Pages 部署](https://docs.astro.build/en/guides/deploy/github/)、[GitHub Pages 发布来源](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)。
