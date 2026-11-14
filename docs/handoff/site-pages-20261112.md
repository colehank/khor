# GitHub Pages 发布 · 2026-11-12

用户在网站本地迭代完成后要求部署 GitHub Pages，随后询问通过既有 Cloudflare / cloudflared 设置解析域名。这是网站静态产物与目标域名的发布授权；不合并 main，不启动原生构建。

## 2026-11-14 更新已发布（当前）

用户明确要求「部署吧」，已沿用既有本地构建 → gh-pages 静态快照 → GitHub Pages 流程发布，没有合并 main。

- 网站：<https://khor.cloud/>；UI 源码 `bdb29937a03b0efd2e624af53e75a49483e92f69`，静态发布提交 `70c6b2e87b74d57c45d64c5cd95dee99f377851b`
- [Pages run 43593164300](https://github.com/colehank/khor/actions/runs/43593164300) success；公开 build.json 已返回该源码版本
- 这轮发布包含固定关系图及完整往返光流、全英文网站／GUI 及真实浅深首图、首尾共用安装区、带图标的下载／文档按钮、三项图标导航、32px 彩色简化 Khor 品牌、透明完整收尾图，以及 Presented by NeuroAI Innovation Team / KapeStack 的 org／官网链接。App／CLI 安装版本仍为既有 `v0.2.0-rc.12`
- 按根域参数完整构建（包含真实 GUI 演示），10 页完成；145 个本地 HTML 资源与站内链接无缺失。严格静态服务器下核对英文网站、生产演示与彩色标记，无资源／页面错误
- 普通 https://khor.cloud/ 访问返回最新页面；390／1440px headless Chrome 检查三项导航、彩色品牌、团队 org 链接、下载页、两幕原生动画和增强光流正常，无溢出、页面或 HTTP 资源错误。手机仅首图、桌面 GUI 正常启动。线上 build.json、GUI HTML、install.sh 与发布产物逐字节一致，未执行安装器
- 普通快进推送 gh-pages；CNAME、Cloudflare DNS 和 HTTPS 配置沿用。Pages source 仍为 gh-pages /，https_enforced=true。临时 23822 静态检查服务器已停止，23820 开发预览保留
- 产物：`/Volumes/disk0/Khor/artifacts/site-pages-20261114/dist`；证据：`/Volumes/disk0/Khor/evidence/site-pages-20261114/`，含构建、链接审计、Pages 收据、公开版本／文件校验、两档 Chrome 检查和截图。发布 checkout 仍为 `site-pages-20261112/publish`

## 2026-11-13 更新已发布

用户明确要求「部署出去」，已将当前网站发布到既有 `gh-pages` 分支，源码分支未合并 main。

- 网站：<https://khor.cloud/>；源码 `f5c457d796b9ddff0b63c27f6182f2aeb1b710cf`，静态发布提交 `cebdd1d120ec98f825be579426483c9b9df94ce8`
- [Pages run 43444719571](https://github.com/colehank/khor/actions/runs/43444719571) 成功；线上 `build.json` 已返回上述源码版本
- 内容包含手机完整 GUI 首图与缩放策略、导航精修、网络超图及约 10 秒的 Work／Khor Agent 模型训练故事。交互与示例仍为静态前端；App／CLI 安装版本仍为 `v0.2.0-rc.12`
- 按根域参数运行完整网站构建（包括 GUI 演示），171 个本地 HTML 资源链接无缺失；线上 390／1440px headless Chrome 检查两幕完成态、网外用户、无溢出／页面错误／资源 HTTP 错误。线上 GUI HTML 与 install.sh 和发布产物逐字节一致，未执行安装器
- 域名、CNAME、DNS 与 HTTPS 配置沿用；普通快进推送，无强推。此次仍为本地构建静态快照，再由 Pages 自动发布，没有新增远端源码构建流程
- 产物：`/Volumes/disk0/Khor/artifacts/site-pages-20261113/dist`；独立发布 checkout 仍为 `site-pages-20261112/publish`。证据：`/Volumes/disk0/Khor/evidence/site-pages-20261113/`，含构建、链接检查、Pages 日志、公开版本／资源检查和截图

## 首次发布与域名配置（2026-11-12 至 13）

- 源码 worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`，branch `feat/site-story`。
- 目标仓库：公开仓库 `colehank/khor`；仅发布静态快照到独立 `gh-pages` 分支，Pages source 为该分支根目录。
- 自定义域名：`khor.cloud`；`www.khor.cloud` 作为同站备用入口。原始项目地址 `https://colehank.github.io/khor/` 将跟随 Pages 配置跳转。
- 首次发布：根路径快照为 `gh-pages` commit `233cfe2`，最终发布 HEAD `5afd72e`（GitHub 重存 CNAME 后自动提交），源码 commit `f196459e1ba84acb33bd1ffb24becf46e48dde95`。已按 `KHOR_SITE_URL=https://khor.cloud KHOR_SITE_BASE=/` 构建并发布根路径产物，含 `CNAME=khor.cloud`。Pages 状态 built，source 为 gh-pages /，custom domain 为 khor.cloud。
- 2026-11-13 DNS 与 HTTPS 均完成。GitHub Pages health 对根域和 www 均确认 DNS 有效、由 Pages 提供服务、可签发 HTTPS，CAA 无错误。证书状态 approved，覆盖 khor.cloud 和 www.khor.cloud，到期 2027-02-11；https_enforced=true。
- 实际验证：`https://khor.cloud/` 返回 HTTP 200 且正常通过 TLS 验证，`https://www.khor.cloud/` 与 `http://khor.cloud/` 均 301 到主域 HTTPS。使用公共 DoH 的正常域名请求取得正确 build.json；没有跳过证书验证。当前 Mac 曾缓存根域不存在的旧 DNS 结果，因此另用公共 DoH 与定向 GitHub Pages IP 的保留 SNI / TLS 校验请求交叉确认。
- 起初证书排队未完成，按 GitHub 官方 HTTPS 排障说明在 DNS 有效后移除再保存一次自定义域名，随即获批并成功开启 HTTPS。没有反复重新绑定或改动 Cloudflare SSL 级别。
- 首次项目路径快照为 `d8fc1c5945dc8c55fabbaf9ad23e576dff7ad2fd`，GitHub 随绑定域名生成 CNAME commit `c651712`，发布根路径快照前已 fetch 并快进接收，没有强推。
- 首次推送 gh-pages 后 GitHub 自动启用 Pages，因此后续创建 API 返回 Already enabled（409）；已重新 GET 验证正确的发布源，未覆盖其他部署。
- Cloudflare：用户实际登录自己的 Chrome。CUA 原生 / 浏览器 host 均失败；用户明确同意改用 AppleScript 后，已通过 System Events 读取并操作该已登录窗口。没有开启 Chrome 的 Apple Events JavaScript 设置，没有读取密码、Cookie 或令牌。
- 在既有 8 条记录之外导入 5 条：根域 A 为 `185.199.108.153` / `185.199.109.153` / `185.199.110.153` / `185.199.111.153`，www CNAME 为 `colehank.github.io`，均仅 DNS、TTL 300。界面确认成功并显示 13 条；既有 relay、LAN 与 tunnel 记录完整保留。Google / Cloudflare DoH 均已核对新记录。没有修改 tunnel 或其他服务的 SSL 设置。

## 实现与校验

- `deployment.mjs` 统一 `KHOR_SITE_URL` / `KHOR_SITE_BASE`；`sitePath` 用于页面与运行时 URL，原片段锚点与外链不变。Sätteri 原生插件调整 Markdown 的站内链接与安装命令，使用 visitor context 写入属性；没有引入 remark 管线。
- GUI 从自身 URL 推导所属网站根路径，固定下载回复、允许点击的按钮和测试网页链接均跟随该根；虚拟 `/api` 仍在原网站 fixture 内拦截，不需要服务器 API。
- 输出包含原安装器的静态副本，将默认版本 pin 为网站已有的 `v0.2.0-rc.12`，保留 `KHOR_VERSION` 覆盖；未执行安装器。GitHub 当前 release 为 prerelease，`/releases/latest` 返回 404，六项已有下载资产已由 release API 重新核对。
- `khor.cloud` 的 NS 为 Cloudflare 的 `vicky` / `vida`。仓库中指向本机 8787 的 cloudflared 文件仅为模板，本次 GitHub Pages 自定义域名直接使用 DNS，无需本机 tunnel 常驻。
- 本地完整 Node 网站构建、网站运行时脚本 TypeScript、demo TypeScript 及 shell 语法校验通过。163 条生成的本站链接均存在且带 `/khor/` 前缀；严格静态服务器下 Chrome 检查首页、Fog、真实 GUI、固定回复下载按钮、CLI 复制、文档与下载页，无页面错误或本机资源 404。

## 产物与证据

- 产物：`/Volumes/disk0/Khor/artifacts/site-pages-20261112/dist`。
- 临时验证预览 `http://127.0.0.1:23822/khor/` 已停止，仅用于本任务静态检查。
- 证据：`/Volumes/disk0/Khor/evidence/site-pages-20261112`，含 `build.log`、`link-audit.json`、`staged-checks.json`、截图和 release 资产清单。
- 原开发预览 `http://127.0.0.1:23820/` 仍按根路径工作。
- 自定义域名产物 `/Volumes/disk0/Khor/artifacts/site-pages-20261112/domain-dist` 已发布，根路径链接审计通过。独立发布 checkout 为同父目录 `publish`。
- DNS 导入文件、最终 13 条记录截图、公共 DNS 查询与 Pages health 收据在证据的 `cloudflare` 子目录；HTTPS 最终状态以该目录 `deployment-state.json` 和本页最新记录为准。
- 公开 iframe 文档 HTTP 200，无阻止同源嵌入的响应头；公开 install.sh SHA256 为 `cbeb3a4264a9e5991799b391c13928069d3c1c8d4cfa6e5fbcdc5c5eb9248207`，与发布产物相同。
- 自定义域名的 HTTPS iframe 返回 200；install.sh 经 HTTPS 下载后与根路径产物逐字节相同，未执行。已在用户自己的 Chrome 新标签页实际打开官网，首屏、Fog 和完整 GUI 均可见；截图为 `cloudflare/public-final.png`，DNS 最终截图为 `cloudflare/dns-final.png`。

参考：[Astro GitHub Pages 部署](https://docs.astro.build/en/guides/deploy/github/)、[GitHub Pages 发布来源](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)。
