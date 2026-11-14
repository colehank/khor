# khor.cloud

Astro 官网、文档和真实前端的交互演示。演示入口为
`apps/gui/dev/site`：复用生产 App 与浏览器内的示例数据，不连接实际设备。

## 本地迭代

```sh
npm ci --prefix apps/site
npm ci --prefix apps/gui
npm run demo:build --prefix apps/site
npm run dev --prefix apps/site -- --host 127.0.0.1 --port 23820
```

前端需要已有的 `apps/gui/src/gen` 生成契约。当前 worktree 复用 RC12 官方
frontend artifact 的 gen（Release run 42824017790，产品 GUI 源码与当前基线相同），
没有触发 Rust 编译。全新环境按仓库前端生成流程准备契约。

改官网文案、动效和 CSS 后由 Astro 自动刷新；仅修改演示数据或 App 组件时重跑
`demo:build`。`demo:check` 检查演示与生产前端类型；`npm run build` 先打包演示再输出静态站。

- `src/components/Mandala.astro` 读取完整的 `mandala-glass.svg` 和既有 session 字形；`icon-story.ts` 从原图提取静态节点、轮廓和完整材质，每个片段只带所需 defs，保留最终图稿的全部原始路径与几何。
- `src/components/ProductHero.astro` 首屏为左侧大字 Khor、一段小字介绍和下载 / 文档入口，右侧完整 Desktop GUI，共同落在 Fog 上；窄屏上下排列，下滑直接接 “You have many devices” 图标故事。`SiteNav.astro` 统一首页、下载和文档的原始图标 + Khor 导航；kapestack 公司链接保留在页脚。Desktop 红黄绿按钮与原有导航同一行；桌面首图在可见范围的空闲时段由真实 App 接管，在原位置操作同源 iframe；手机／粗指针设备只显示完整真实首图，不启动桌面 App，保留浏览器本身的两指缩放。没有放大入口或查看层。不连接 Lab 或真实设备，没有界面旁注或单独的体验页面。正文不重复绘制应用图标，两个按钮都不带箭头。首屏下方的平台图标复用 App 既有系统标记；macOS / Linux 前往各自安装选项，平台组标题为 Other ways to get Khor，Windows / iOS / Android 置灰，悬停、键盘聚焦或触屏点按时显示 Coming soon，支持 Escape 和点外侧退出，提示层位于 GUI 上方。macOS 标明 App · CLI，Linux 标明 CLI。可用性从既有 release 资产推导，不虚构平台安装包。安装组在宽屏横排为一行，窄屏分行；另有 or Khor CLI 与同一个 INSTALL 命令；悬停 / 聚焦显示复制图标，触屏直接显示。CopyButton 复用实际 App 的 Copy / Check 字形，复制后以勾短暂替换，不显示成功文字，保留屏幕阅读器状态；命令与图标不因反馈重建。
- Hero 复用生产 App 的 `data-window-transparent` 与 `--window-opacity:40%` 单层背景，不改变前景 opacity，不启用原生后端。首图由父窗口取样 Fog；交互界面将同一 Fog 放入 iframe 的背景，原有玻璃控件在窗口 40% 底面上正常取样，不重复叠加暗底。深浅首图保留 alpha（背景 102/255，文字可达 255/255），加载前后材质一致。窗口圆角和 GUI 共用缩放系数，避免窄屏时裁掉红黄绿按钮；外层没有白色描边。
- 首屏 `fog-options.ts` 完整保留用户指定的 Vanta Fog 参数。three.js r134 和 Vanta Fog 0.5.24 固定存于 `public/vendor`，许可证同目录，运行时不访问 CDN。效果离开视口后由 Vanta 跳过绘制。减少动态效果或无法加载时，显示从本页同参数画布截取的干净帧。首页品牌故事保持黑场，下载页、文档和 iframe 内的 GUI 继续跟随浏览器明暗；首页不强制 `color-scheme`，避免改变 iframe 的主题偏好。
- 首页依次为首屏、图标故事、网络基础设施／Work across the net／Khor Agent 三幕、下载收尾；依次说明连通设备、由你跨设备完成任务、把同一任务交给 Khor。开场四个设备节点与后三幕共用 Server、Workstation、Desktop、Mobile 图标，来自生产图标家族；下载选项继续使用系统标记。图标故事先留桌面 .5 屏、手机 .7 屏阅读首幕，再进入 4.8 屏的演化（段总高桌面 6.3 屏、手机 6.5 屏）；姿态地标为 `0 / .22 / .47 / .73 / 1`。首幕停留时只有底部细线反馈滚动，不提前换脸或淡出文字。首屏末端的渐变至少为 40svh。
- `ProductTour.astro` 第一幕 Net 为无控件的自动循环超图：LAN 包含 Workstation 与 Server，Wi-Fi 包含 Desktop 与同一个 Server，Mobile 从 Wi-Fi 开始，移动到 Cellular 再返回 Wi-Fi。Desktop 在左、Workstation 在右。KhorNet 一次成形后始终包围四台设备，随后以 10 秒周期漫游，不重复初次建网；原生网络成员随 Mobile 移动变化。平滑凸包保持大弧度，没有逐节点鼓包。各超边使用生产图标与沿弧线排列的名称；图标和标题固定位置，边界按实际字宽、前后各 8 个 SVG 单位留完整缺口；KhorNet 直连时显示 `P2P`，中继示意时显示 `relay` 和副标题 `hole punching`，移入网络后仍保持中继 1.3 秒，再切直连；仅 `hole punching` 轻闪，图标、名称与连接状态常亮。仅可见且前台时运行，减少动态效果时静止；没有真实网络访问。该幕按宽屏／平板／手机调整图形上限、字号、标题排布和留白；后续浏览器验证统一使用 headless Chrome，不打开桌面窗口。实现与验证见 [Net 漫游场景](../../docs/handoff/site-net-roaming-20261113.md)。
- 三幕文案现为 **Stay connected → Continue anywhere → Let Khor take over**，前两幕副文案以 with Khor 收尾。后两幕为固定关系图：Continue anywhere 将笔记本与 Mobile 圈为 You / Same task 访问组，常驻连接台式机的 Claude 图标 / File 和 Server 的 Codex 图标 / Web（不重复标注智能体名称，不单列 Terminal）；Agent 幕由网外 You 以固定 Task / Result 双向路径连接超边上的 Khor，再由 Khor 连接两台远端设备。设备、文字、session 与资源归属始终可见，不再循环播放阶段、传输载荷或模拟进度。连接采用与第一幕一致的 10 秒节奏：细光以 1.7 秒缓动沿固定路径经过，到达后活动环轻闪，再停留；Work 两条访问线交替提示，Agent 依次提示请求、调用、返回，离屏／后台暂停，减少动态效果时静止，无 JS 仍有完整构图；没有真实连接。沿用第一幕的设备几何与 KhorNet，第一幕漫游不变。当前实现与证据见 [固定关系图](../../docs/handoff/site-relations-20261114.md)，此前任务动画保留为历史记录。本轮仅本地，线上仍为 2026-11-13 快照。
- 已删除被替代的 12 张介绍截图、图片选择组件与专门截图场景；默认首屏 GUI 场景、虚拟世界和下载回复保持不变。首页的完整 App 仍只在桌面首屏加载，手机保留静态首图策略。
- 文字在每段图形起动时交接：旧句用 120ms 渐隐，新句随后用 120ms 渐现；这是可完成的短时过渡，停止滚动不会把两句固定在重叠状态。图形在前 86% 演化，段末 14% 保留完整姿态，`iconSegments` 的几何采样仍随滚动位置连续推进；分页跟随当前文案。首幕阅读时间不变。
- 示例数据包含 macOS、Windows、Linux、iOS、Android 五种系统的六台设备，Claude、Cursor、Codex、Khor、Shell 五种会话类型；全部是浏览器内的合成连接状态。会话字形沿用生产 App 的类型标记。
- 演示范围固定：机器、会话、额度、工作区列表仅第一项可点；会话、侧栏与工作区各显示三个标签，只保留当前标签可用。「…」和新增、编辑等复杂操作不开放。不可点击项保留正常颜色与不透明度，不做灰化。主会话是 `wide/all`，使用真实 `MachineChat` / `KhorAgentResult` 组件及本地生成的命令回执，展示 Khor 内置对话；Claude、Cursor、Codex、Shell 仅作其他会话列表项。
- `apps/gui/dev/site/world.ts` 是设备、任务、项目和文件的共同数据源；`fixture.ts` 补齐第一台机器的仪表、历史趋势、进程及额度读数。列表保留真实日常任务和六个普通账户名称，不加演示说明。右侧与工作区默认展示同一份包含表格、代码和清单的 Markdown 修复记录；网站专用 FileView alias 复用生产渲染器，仅调整 Markdown 的默认阅读姿态。
- `dev/site/telemetry.ts` 提供固定的合成遥测：额度主项展示六个模型、30 天堆叠趋势及按模型散点，日汇总由模型数据相加，保持图表与表格一致。机器主项含 44 个进程，CPU 与内存独立分布，全天记录有工作高峰、短时编译和回落；末个桶与当前仪表相同。`data/avatars.json` 为六台机器保存不同的配色，沿用原生头像几何及 `crates/core/src/avatar.rs` 出厂色板；同一机器在列表、会话、额度和工作区始终共用一张脸。
- `src/assets/product/desktop.webp` 与 `desktop-dark.webp` 来自本地 `/experience/app/index.html?scene=content&hero=1`，Chrome 1360×850、2× 像素密度、WebP quality 92，分别在浏览器浅色和深色偏好下截图。本轮窗口底色为 40%；使用浏览器透明截图，保留原生 alpha 与实际前景，父窗口提供 Fog 模糊；不把背景透明度施加到文字，也不后期伪造界面。App 或 `apps/gui/dev/site/scenario.ts` 更新后重新截图。首图与可操作 GUI 均跟随浏览器明暗偏好；GUI 使用生产 App 的 auto 主题监听，实时切换不重载会话。所有任务与文件均为浏览器内的演示数据；发送消息固定回复 “Let's try Khor, download it here.”，链接进入同一套下载推荐。固定回复中的下载按钮从演示交互限制中单独放行，其余复杂操作仍受限。
- `src/scripts/product.ts` 从 iframe 首次载入前就固定四倍 list-w 的桌面视口，桌面只用 transform 缩放并原地交互，手机在启动前移除 iframe、保留首图。用户已取消 GUI 放大功能，没有对话框、横条或手机覆盖按钮。WebKit 的布局 zoom 会锁存初始窄屏状态，因此不使用。预览圆角与 GUI 共用缩放系数。
- `src/scripts/product-background.ts` 为 iframe 提供同一份 Fog：同源画布流最多 15fps，不创建第二个 WebGL 渲染器，不逐帧读取像素；窗口不在视口或页面隐藏时停止轨道。减少动态效果、画布流不支持或失败时使用同参数首图。内层窗口仍只绘制一层 40% 的底色，控件沿用生产的 blur、tint、rim。
- `src/scripts/download.ts` 根据平台与可用架构信息推荐已有发行包，顶部与首屏入口共用结果。macOS 旧 UA 不用于猜芯片，信息不足时进入平台选择页；未提供安装包的平台展示选项和真实可用范围。下载页按 App / CLI 分组，Linux 优先 CLI；安装文档增加经真实 App 文案核对的 GUI 下载、会话和设备邀请路径。
- 演示在 App 挂载后交出首图，包括从设备、设置等非聊天页恢复的入口；不以聊天输入框是否存在作为可交互条件。顶部导航直接使用生产 `LandingStrip` 及其选中态，不在官网另画高亮。
- `src/scripts/story.ts` 用一条原生 WAAPI ScrollTimeline 推进图形；字幕在章节边界做可独立完成的短时渐变，关键帧仅在初始化或稳定几何改变时生成。范围按 section / sticky 的实际高度计算，不混用手机动态 innerHeight。旧浏览器使用可见范围内、缓存属性值的 RAF 后备，正倒滚共用 `iconState()`；没有触摸拦截或强制翻页。One mind 时外珠归位缩小，中心珠和内圆保持最终绿色与几何；How about 仅绘制三条简单轮廓。Meet 用旋转裁切和相反方向的 HTML 位移揭示完整原图，羽化形状固定，不逐帧改 SVG 遮罩、滤镜或祖先色彩。灰节点有不透明底面，轮廓不穿透；Meet 前不出现材质高光。章节书签向前取整，确保末端原图完整。实现与 Chrome 检查见 `docs/handoff/site-scroll-research-20261112.md`。
- 减少动态效果模式将五句故事和最终图标合为紧凑的静态章节，移除长距离 sticky 滚动；原始叙事文案保留。
- `src/releases.ts` 是下载页和 `/download.json` 的共同来源，仅列已验证的发行资产。
- 站点 token 从 `docs/design-system/tokens.json` 生成；不另存调色板与字体。
- 演示产物在 `public/experience/app`，由构建生成并忽略；没有重新实现一套 App 页面。

## GitHub Pages

可发布到 `https://colehank.github.io/khor/`：

```sh
KHOR_SITE_URL=https://colehank.github.io KHOR_SITE_BASE=/khor/ npm run build --prefix apps/site -- --force
```

输出仍是 `apps/site/dist`，将其内容作为 `gh-pages` 分支根目录发布。产物含 `.nojekyll` 和 `install.sh`，GitHub 托管静态产物；构建在本机完成。主域名部署使用 `KHOR_SITE_URL=https://khor.cloud KHOR_SITE_BASE=/`。基路径同时作用于页面、Markdown、资源、下载推荐及演示 GUI 的下载回复。

官网安装脚本使用仓库原始安装器，并将默认版本固定到 `src/releases.ts` 的已发布预览版；保留 `KHOR_VERSION` 覆盖能力。当前只有预发布版，不能通过 GitHub 的 `/latest` 入口选择它。安装脚本只作为静态文件输出，不在构建或发布时执行。

本次发布与 Cloudflare DNS 状态见 [Pages 发布记录](../../docs/handoff/site-pages-20261112.md)。

## 部署

用户本轮新增 GitHub Pages 发布授权；feature 分支不合并 main，网站静态产物发布到独立 gh-pages 分支。
2026-11-12 整页收整与定向 Chrome 验证见 [本轮记录](../../docs/handoff/site-refinement-20261112.md)。
现有 `deploy/push.sh` 继续负责静态站与下载镜像的分阶段同步；页面同步保护 `/dl`，
版本校验完成后才推进下载指针。详细约定见 [镜像维护](../../scripts/release-mirror.md)。
中继、寻址和官网分别部署。

2026-11-13 本地全页精修与 Safari 缩放验收见 [本轮记录](../../docs/handoff/site-zoom-20261113.md)。手机完整首图、桌面可操作 GUI，普通首页已获用户 Safari 真机放大／滚动通过反馈；已随 2026-11-13 网站更新发布，版本与公开验收见 [Pages 发布记录](../../docs/handoff/site-pages-20261112.md)。
