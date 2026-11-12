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
- `src/components/ProductHero.astro` 首屏为左侧 Khor、介绍和双入口，右侧完整 Desktop GUI，共同落在 Fog 上；窄屏上下排列，下滑直接接 “You have many devices” 图标故事。左上角与页脚是链接到 `kapestack.top` 的 kapestack。Desktop 红黄绿按钮与原有导航同一行；首图由真实 App 接管为可操作的同源 iframe，不连接 Lab 或真实设备，没有界面旁注或「打开体验」入口。
- Hero 复用生产 App 的 `data-window-transparent` 与 `--window-opacity:50%` 单层背景，不改变前景 opacity，不启用原生后端。模糊在父窗口对真实 Fog 取样，iframe 内保留原有玻璃控件与正文；深浅首图保留 alpha（背景 127/255，文字可达 255/255），加载前后材质一致。窗口圆角和 GUI 共用缩放系数，避免窄屏时裁掉红黄绿按钮；外层没有白色描边。
- 首屏 `fog-options.ts` 完整保留用户指定的 Vanta Fog 参数。three.js r134 和 Vanta Fog 0.5.24 固定存于 `public/vendor`，许可证同目录，运行时不访问 CDN。效果离开视口后由 Vanta 跳过绘制；减少动态效果或无法加载时，显示从本页同参数画布截取的干净帧。首屏颜色不随主题重配，正文与 GUI 继续跟随浏览器。
- 首页只保留首屏、图标故事和下载收尾，系统 / harness 标识及三组图文已从页面移除。图标故事行程为桌面 4.8 屏、手机 6.2 屏，姿态地标为 `0 / .22 / .47 / .73 / 1`。首屏末端的渐变至少为 40svh。
- 示例数据包含 macOS、Windows、Linux、iOS、Android 五种系统的六台设备，Claude、Cursor、Codex、Khor、Shell 五种会话类型；全部是浏览器内的合成连接状态。会话字形沿用生产 App 的类型标记。
- 演示范围固定：机器、会话、额度、工作区列表仅第一项可点；会话、侧栏与工作区各显示三个标签，只保留当前标签可用。「…」和新增、编辑等复杂操作不开放。不可点击项保留正常颜色与不透明度，不做灰化。主会话是 `wide/all`，使用真实 `MachineChat` / `KhorAgentResult` 组件及本地生成的命令回执，展示 Khor 内置对话；Claude、Cursor、Codex、Shell 仅作其他会话列表项。
- `apps/gui/dev/site/world.ts` 是设备、任务、项目和文件的共同数据源；`fixture.ts` 补齐第一台机器的仪表、历史趋势、进程及额度读数。列表保留真实日常任务和六个普通账户名称，不加演示说明。右侧与工作区默认展示同一份包含表格、代码和清单的 Markdown 修复记录；网站专用 FileView alias 复用生产渲染器，仅调整 Markdown 的默认阅读姿态。
- `dev/site/telemetry.ts` 提供固定的合成遥测：额度主项展示六个模型、30 天堆叠趋势及按模型散点，日汇总由模型数据相加，保持图表与表格一致。机器主项含 44 个进程，CPU 与内存独立分布，全天记录有工作高峰、短时编译和回落；末个桶与当前仪表相同。`data/avatars.json` 为六台机器保存不同的配色，沿用原生头像几何及 `crates/core/src/avatar.rs` 出厂色板；同一机器在列表、会话、额度和工作区始终共用一张脸。
- `src/assets/product/desktop.webp` 与 `desktop-dark.webp` 来自本地 `/experience/app/index.html?scene=content&hero=1`，Chrome 1360×850、2× 像素密度、WebP quality 92，分别在浏览器浅色和深色偏好下截图。App 或 `apps/gui/dev/site/scenario.ts` 更新后重新截图。首图与可操作 GUI 均跟随浏览器明暗偏好；GUI 使用生产 App 的 auto 主题监听，实时切换不重载会话。所有任务与文件均为浏览器内的演示数据；发送消息固定回复 “Let's try Khor, download it here.”，链接进入同一套下载推荐。
- `src/scripts/product.ts` 从 iframe 首次载入前就固定四倍 list-w 的桌面视口，统一用 transform 缩放呈现；WebKit 的布局 zoom 会锁存初始窄屏状态，事后恢复视口也不能可靠纠正。圆角与 GUI 共用缩放系数。`src/scripts/download.ts` 根据平台与可用架构信息推荐已有的发行包。macOS 的旧 UA 不用于猜测芯片，信息不足时进入平台选择页；未提供安装包的平台展示所有选项。
- 演示在 App 挂载后交出首图，包括从设备、设置等非聊天页恢复的入口；不以聊天输入框是否存在作为可交互条件。顶部导航直接使用生产 `LandingStrip` 及其选中态，不在官网另画高亮。
- `src/scripts/story.ts` 用一条原生 WAAPI ScrollTimeline 同步文字与图形，关键帧仅在初始化或稳定几何改变时生成。范围按 section / sticky 的实际高度计算，不混用手机动态 innerHeight。旧浏览器使用可见范围内、缓存属性值的 RAF 后备，正倒滚共用 `iconState()`；没有触摸拦截或强制翻页。One mind 时外珠归位缩小，中心珠和内圆保持最终绿色与几何；How about 仅绘制三条简单轮廓。Meet 用旋转裁切和相反方向的 HTML 位移揭示完整原图，羽化形状固定，不逐帧改 SVG 遮罩、滤镜或祖先色彩。灰节点有不透明底面，轮廓不穿透；Meet 前不出现材质高光。章节书签向前取整，确保末端原图完整。实现与 Chrome 检查见 `docs/handoff/site-scroll-research-20261112.md`。
- 减少动态效果模式按当前章节直接展示静态姿态，图标故事仍从四颗灰珠开始。产品首屏、图标故事、资源体验与下载均可直接访问。
- `src/releases.ts` 是下载页和 `/download.json` 的共同来源，仅列已验证的发行资产。
- 站点 token 从 `docs/design-system/tokens.json` 生成；不另存调色板与字体。
- 演示产物在 `public/experience/app`，由构建生成并忽略；没有重新实现一套 App 页面。

## 部署

本轮只作本地预览，没有合并或线上部署。
现有 `deploy/push.sh` 继续负责静态站与下载镜像的分阶段同步；页面同步保护 `/dl`，
版本校验完成后才推进下载指针。详细约定见 [镜像维护](../../scripts/release-mirror.md)。
中继、寻址和官网分别部署。
