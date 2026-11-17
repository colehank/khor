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

- 导航的 Khor 标记使用 `BrandIcon variant="navigation"`：复用 `mandala-glyph.svg` 已通过的小尺寸几何，保留外圆、菱形、内环与五颗节点。结构使用导航当前文字色，内环与节点使用 `lq2` 和当前文字色混合出的品牌绿；透明底、无滤镜、无渐变，保持 32px。默认 glass 变体仍是完整玻璃图稿，用于文档品牌和下载收尾。
- `BrandIcon.astro` 默认 glass 变体直接使用完整 App SVG 的实体图标层，保留玻璃、菱形、节点、阴影与渐变定义，只去掉黑色底板和环境背景，透明区域是真 alpha；导航使用前述 navigation 变体。页脚为 Presented by NeuroAI Innovation Team / KapeStack，团队名链接已确认的 GitHub org `https://github.com/AI-innovation-team`，KapeStack 链接公司主页。
- `SiteNav.astro` 右侧依次为 Docs、Download、GitHub、明暗、语言图标，点击区域均为 44px。功能图标使用 Apple SF Symbols，按实际图形边界校准视觉大小；GitHub 保留官方品牌图稿。明暗点击直接切换，语言使用网页原生 select，没有 Khor 弹窗。Download 始终指向安装选项页；主要平台推荐交给正文共用按钮。来源与许可保存在 `src/assets/navigation`。
- 首屏与收尾共用 `ProductActions.astro`：Download / 下载按钮固定文字，右侧按访问平台切换字形，目标仍推荐已有 macOS App 或 Linux CLI；移动设备前往下载选项页。Docs 指向文档。收尾为 Build with Khor，完整玻璃图标与标题双栏，下方保留平台与 CLI 安装组。
- 官网默认跟随浏览器语言选择英文或简体中文、跟随系统选择浅色或深色；导航可手动切换并在本机保存选择。首页、导航、图中文字、下载、文档与错误页共用静态双语内容。偏好在首帧前解析，不增加服务端路由。GUI 演示接收同一语言与主题，任务、文件和 Git 记录仍来自统一 fixture；移动端只加载所选语言和主题的一张真实首图，保留 Safari 两指缩放。中文文档位于 `src/content/docs-zh`。
- 后两幕光流通过 `FlowTrace.astro` 的 halo / trail / head 三层原生 SVG 描边增强，亮芯在正向与反向传输时都位于前端。没有 SVG 模糊滤镜或逐帧 JS；保留 1.7 秒传输、Continue 同步 6 秒往返循环、Agent 12 秒往返与到达依存、离屏暂停和减少动态效果处理。
- `src/components/Mandala.astro` 读取完整的 `mandala-glass.svg` 和既有 session 字形；`icon-story.ts` 从原图提取静态节点、轮廓和完整材质，每个片段只带所需 defs，保留最终图稿的全部原始路径与几何。
- `ProductHero.astro` 首屏为左侧 Khor、简短介绍及下载／文档入口，右侧完整 Desktop GUI，窄屏上下排列。桌面首图在可见范围的空闲时段自动由真实 App 接管，无需进入或退出试用；可直接点击、发消息。手机／粗指针设备仅显示首图、不加载 iframe，保留 Safari 缩放。首屏下方常显 Other ways to get Khor、五个平台与 CLI 命令；未来平台置灰，悬停／聚焦／点按提示 Coming soon。复制后只短暂显示勾。
- Hero 复用生产 App 的 `data-window-transparent`，演示窗口底面为 `--window-opacity:40%`。文字保持完整不透明度，玻璃控件沿用生产 blur、tint、rim；没有重复黑底或白色包边。四张真实首图与交互 App 使用同一材质，背景来自同一份 Fog。
- 首屏 `fog-options.ts` 完整保留用户指定的 Vanta Fog 参数。three.js r134 和 Vanta Fog 0.5.24 固定存于 `public/vendor`，许可证同目录，运行时不访问 CDN。效果离开视口后由 Vanta 跳过绘制。减少动态效果或无法加载时，显示从本页同参数画布截取的干净帧。首页品牌故事、下载页、文档和 iframe 内的 GUI 均跟随浏览器明暗；Fog 参数及原始玻璃图稿保持不变，配色使用既有设计 token。
- 首页依次为首屏、五句图标故事、Stay connected／Continue anywhere／Let Khor take over 三幕和下载收尾。开场设备与三幕共用 Server、Workstation、Desktop、Mobile 图标；第二句为 With many agents。开场保持原生下滑驱动，包含完成态阅读距离与网络展开，当前开场容器高 6.25 屏，五句演化占 3.8 屏，每次变化约 .84–1.03 屏。五个可读书签可定位姿态；每步前 60% 演化，后 40% 保持，停滑即停、倒滑可逆。Meet Khor 与固定左上光源同步起动；首屏渐变至少为 40svh。
- `ProductTour.astro` 第一幕 Net 为无控件的自动循环超图：LAN 包含 Workstation 与 Server，Wi-Fi 包含 Desktop 与同一个 Server，Mobile 从 Wi-Fi 开始，移动到 Cellular 再返回 Wi-Fi。Desktop 在左、Workstation 在右。KhorNet 一次成形后始终包围四台设备，随后以 10 秒周期漫游，不重复初次建网；原生网络成员随 Mobile 移动变化。平滑凸包保持大弧度，没有逐节点鼓包。各超边使用生产图标与沿弧线排列的名称；图标和标题固定位置，边界按实际字宽、前后各 8 个 SVG 单位留完整缺口；KhorNet 直连时显示 `P2P`，中继示意时显示 `relay` 和副标题 `hole punching`，移入网络后仍保持中继 1.3 秒，再切直连；仅 `hole punching` 轻闪，图标、名称与连接状态常亮。仅可见且前台时运行，减少动态效果时静止；没有真实网络访问。该幕按宽屏／平板／手机调整图形上限、字号、标题排布和留白；后续浏览器验证统一使用 headless Chrome，不打开桌面窗口。实现与验证见 [Net 漫游场景](../../docs/handoff/site-net-roaming-20261113.md)。
- 三幕文案现为 **Stay connected → Continue anywhere → Let Khor take over**，前两幕副文案以 with Khor 收尾。后两幕为固定关系图：Continue anywhere 将笔记本与 Mobile 圈为 You / Same task 访问组，常驻连接台式机的 Claude 图标 / File 和 Server 的 Codex 图标 / Web（不重复标注智能体名称，不单列 Terminal）；Agent 幕复用同一个 You / Same task 笔记本与 Mobile 访问组，以 Task / Result 双向路径连接超边上的 Khor，再由 Khor 连接两台远端设备；没有独立的网外 You。设备、文字、session 与资源归属始终可见，不再循环播放阶段、传输载荷或模拟进度。You 访问组在发送任务与收到结果时整条超边一起亮起，含轻微面色；移除组外的接收小圆点。Continue 和 Agent 都在派发及返回交付时各亮一次，两条边界随同一份响应布局更新。连接共用第一幕的 1.7 秒缓动与 1.3 秒交接停留；Work 每 6 秒完成一次往返：You 同步发送给两台远端设备，1.7 秒到达后才点亮智能体，再等待 1.3 秒，两台机器一起返回 You；You 发送与收到时整条超边点亮。Agent 以 12 秒完整往返：You → Khor → 机器，调用到达后才点亮智能体，随后机器 → Khor → You；每段都等前段到达后再开始；Khor 从收到任务到结果送达用户持续轻闪，交付后恢复常亮，离屏／后台暂停，减少动态效果时静止，无 JS 仍有完整构图；没有真实连接。沿用第一幕的设备几何与 KhorNet，第一幕漫游不变。当前实现与证据见 [固定关系图](../../docs/handoff/site-relations-20261114.md)，此前任务动画保留为历史记录。这些更改已随 2026-11-16 网站快照发布。
- 已删除被替代的 12 张介绍截图、图片选择组件与专门截图场景；默认首屏 GUI 场景、虚拟世界和下载回复保持不变。首页的完整 App 仍只在桌面首屏加载，手机保留静态首图策略。
- 文字在每段图形起动时交接：旧句用 120ms 渐隐，新句随后用 120ms 渐现；这是可完成的短时过渡，不会把两句固定在重叠状态。每次演化在段内前 60% 完成，段末保留 40% 的滚动阅读段；Mind 外珠用 smoothstep 归位。字幕在滚动跨越章节起点时渐变交接，五个分页跟随当前文案；图形连续跟随同一份滚动进度，停止滚动不继续演化。
- 示例数据包含 macOS、Windows、Linux、iOS、Android 五种系统的六台设备，Claude、Cursor、Codex、Khor、Shell 五种会话类型；全部是浏览器内的合成连接状态。会话字形沿用生产 App 的类型标记。
- 演示范围固定：机器、会话、额度、工作区列表仅第一项可点；会话、侧栏与工作区各显示三个标签，只保留当前标签可用。「…」和新增、编辑等复杂操作不开放。不可点击项保留正常颜色与不透明度，不做灰化。主会话是 `wide/all`，使用真实 `MachineChat` / `KhorAgentResult` 组件及本地生成的命令回执，展示 Khor 内置对话；Claude、Cursor、Codex、Shell 仅作其他会话列表项。
- `apps/gui/dev/site/world.ts` 是设备、任务、项目和文件的共同数据源；`fixture.ts` 补齐第一台机器的仪表、历史趋势、进程及额度读数。列表保留真实日常任务和六个普通账户名称，不加演示说明。右侧与工作区默认展示同一份包含表格、代码和清单的 Markdown 修复记录；网站专用 FileView alias 复用生产渲染器，仅调整 Markdown 的默认阅读姿态。
- `dev/site/telemetry.ts` 提供固定的合成遥测：额度主项展示六个模型、30 天堆叠趋势及按模型散点，日汇总由模型数据相加，保持图表与表格一致。机器主项含 44 个进程，CPU 与内存独立分布，全天记录有工作高峰、短时编译和回落；末个桶与当前仪表相同。`data/avatars.json` 为六台机器保存不同的配色，沿用原生头像几何及 `crates/core/src/avatar.rs` 出厂色板；同一机器在列表、会话、额度和工作区始终共用一张脸。
- `src/assets/product/desktop{,-dark,-zh,-zh-dark}.webp` 为同一真实 App 的 EN / ZH、浅／深首图，Chrome 1360×850、2× 像素密度、WebP quality 92，窗口底面 40%。Continue anywhere 只保留标题、说明和概念图，不添加会话结果截图。App 或演示数据改变后需重新截图；不得用 CSS 透明度降低文字或后期重绘控件。
- `src/scripts/product.ts` 从 iframe 首次载入前就固定四倍 list-w 的桌面视口，桌面只用 transform 缩放并原地交互，手机在启动前移除 iframe、保留首图。用户已取消 GUI 放大功能，没有对话框、横条或手机覆盖按钮。WebKit 的布局 zoom 会锁存初始窄屏状态，因此不使用。预览圆角与 GUI 共用缩放系数。
- `product-background.ts` 为自动加载的 iframe 提供同一份 Fog：同源画布流最多 15fps，不创建第二个 WebGL 渲染器，不逐帧读取像素；窗口不可见或页面隐藏时停止轨道。减少动态效果、画布流不支持或失败时使用同参数首图。内层只绘制一层 40% 窗口底色。
- `download.ts` 根据平台与可用架构信息推荐已有发行包。macOS 旧 UA 不用于猜芯片，信息不足时进入平台选择页；Linux 优先 CLI。手机 Download 指向 `/download/`，恢复 App / CLI 安装选项；不改成网页访问设置入口。未发布的平台不虚构安装包。
- 演示在 App 挂载并完成一轮 DOM 稳定后发出就绪信号，包括设备、设置等非聊天页恢复入口；不以聊天输入框存在与否作为就绪条件。顶部导航使用生产 `LandingStrip` 及真实选中态。
- `story.ts` 使用原生 WAAPI ScrollTimeline；只有几何改变时重建关键帧。旧浏览器使用可见范围内、缓存属性值的 RAF 后备；没有触摸拦截或强制翻页。One mind 外珠归位缩小，中心珠与内圆保持最终绿色；How about 绘制简单轮廓，Meet 恢复原图光照和材质。两端交接的实际路径和材质形变由 `story-morph.ts` / `story-material.ts` 负责，详见下方当前动效说明。
- 减少动态效果模式将五句故事和最终图标合为紧凑的静态章节，移除长距离 sticky 滚动；原始叙事文案保留。
- `src/releases.ts` 是下载页和 `/download.json` 的共同来源，仅列已验证的发行资产。
- 共用 token 从 `docs/design-system/tokens.json` 生成；官网单独采用已确认的雾白 `--site-daylight-canvas`，玻璃色阶由既有品牌与中性色派生，GUI 和共享产品 token 保持独立。
- 演示产物在 `public/experience/app`，由构建生成并忽略；没有重新实现一套 App 页面。

## Apple 网络图标

Wired / Wi-Fi / Cellular 使用已确认的 SF Symbols 原稿：`cable.connector` / `wifi` /
`cellularbars`。正式首页与原本地对照页保持相同形状、比例和颜色；图稿已保存到
`src/assets/network`，普通静态构建不依赖本机缓存或 Apple API。

需要重新导出时在 macOS 运行 `node apps/site/scripts/render-network-symbols.mjs`。
脚本离屏渲染 96px 透明原稿，SVG alpha mask 继承当前文字色，不打开 App 窗口。
来源与版权说明在 `src/assets/network/README.md`。本地旧对照地址仍可访问；正式站只生成首页。

2026-11-17 本地布局精修：三幕共用左文右图／窄屏上下排布，内容宽度与首屏统一；
三幕超过 760px 统一左文右图，760px 及以下统一上文下图，共用列宽、间距、画布高度和标题起点；所有尺寸完整显示一段统一字号、颜色并带正常标点的正文。三幕共用左侧笔记本／手机、
右侧 Server／Workstation 的两组设备位置，KhorNet 为固定的轻斜椭圆；原生网络与漫游只在
第一幕变化。Continue 两条分支分别连接两台远端设备；Agent 的派发与交付弧线连接顶部 Khor，
调用路径从另一侧连接远端设备，Web / File 只保留右下角附着的图标，与右上角的智能体图标一致。
图中字体和图标按 SVG 的实际显示宽度补偿缩放，节点半径、标记、标题缺口与连线端口一起更新；
只在容器尺寸变化时处理，不增加逐帧布局。超边图标与标题使用同一参考姿态的
切线方向，漫游时不转动、不移动；设备自身图标保持正向。实现与 Chrome 证据见
[宽屏布局](../../docs/handoff/site-wide-20261117.md)。此轮尚未发布。

## 当前本地动效与偏好（2026-11-17）

中间的章节使用普通页面滚动，不将三幕固定在同一个舞台里按阈值换页。
只有 Meet Khor（试试 Khor）到 Stay connected，以及 Let Khor take over 到
Build with Khor 使用局部 sticky 交接。前者先将完整原图的玻璃、高光和光场收成同几何的线条，
再将完整标记收拢到 KhorNet 标题旁，同时展开网络轮廓和四台设备；后者反向合拢后恢复玻璃和光照。
中心内圆不再单独变成第二个完整图标；没有跨正文的全屏悬浮过渡层。

开场每步前 60% 演化、后 40% 保持完成态，初始保留 .25 屏阅读距离；网络用 .8 屏展开后额外保持 .4 屏。
Agent 完整进入后保留 1 屏阅读段，再用 .85 屏合拢；Build 完成后保留 .5 屏。
这些都是原生滚动距离，没有时间锁、滚轮拦截或强制翻页；Continue anywhere 随页面直接滚动。
`story-stops.ts` 在文档空间放置 9 个原生阅读停靠点，采用 proximity + scroll-snap-stop:always，不把移动 SVG 当作吸附目标。章节按钮也定位到已完成姿态。原生轻吸附不保证直接拖滚动条或指定巨量 wheel 位移只经过一幕；不增加强制逐页控制器。放大网页时退出停靠，恢复原倍率后重新启用。
局部光流和漫游仅在对应图可见时运行。减少动态效果时移除停靠，全部内容按普通文档流静态展示。

`narrative-tour.ts` 管理两端交接与图的可见播放，`story-morph.ts` 管理路径形变，
`story-material.ts` 在同一份原始图稿上连续改变描边厚度、曲面留白、高光大小、渐变与节点半径。
不再叠加玻璃图、线稿、小图标并交叉淡化；主体在交接中始终保持 opacity 1。
完整图标以同一组几何缩至已接受的小尺寸字形，外围连线沿路径展开，原图的四颗外珠沿路径移动，连续改变大小和材质成为设备节点；不再从零画新的圆。
完整记录、偏好覆盖和无界面 Chrome 证据见 [本轮记录](../../docs/handoff/site-preferences-20261117.md)。
本轮尚未提交或发布。

## 当前本地 UI/UX 精修（2026-11-17）

开场保留五句，当前滚动节奏与阅读停靠见上一节；Continue anywhere 保留概念图，不增加示例任务段落。
用户随后要求恢复原先的可交互和下载：桌面 GUI 自动载入，主要按钮为 Download，
手机前往下载页，平台与 CLI 常显。撤销进入／退出试用、网页访问主按钮及折叠安装区。
导航和正文在超宽屏共用对齐轴；减少动态效果时，网络图从首帧即可见。
完整改动、验收范围和证据见 [UI/UX 精修记录](../../docs/handoff/site-apple-ux-20261117.md)。

## 图标过渡与中文会话修复（2026-11-17）

菱形过渡只使用原图最外层的清晰描边；面内模糊边与玻璃体积可收退，轮廓始终保留。
四台机器复用原图四颗外珠的实体与高光，持续移动、变形和换材质，主体不淡入淡出。
KhorNet 生长的第一帧就带标题缺口；反向合拢先收完关系线和徽标，再移动设备。
第一幕从漫游返回形变时重置成员位置，并重新测量连接标题，避免残留中继状态的缺口。
首屏 GUI 的中文会话示例来自 `scenarioForLocale('zh')`，完整首图从真实 App 生成；后续章节不使用局部消息截图。
本轮按桌面／手机、展开／合拢各采样八个姿态，查看至少三个中间帧并检查反向滚动。
证据与根因见 [过渡修复记录](../../docs/handoff/site-morph-fix-20261117.md)，仍仅本地。

## 当前外环复制动效（2026-11-17）

完整玻璃外环复制后直接形变为 KhorNet 超边，始终是闭合轮廓，不再描出一条新弧线。
形变中的图标核心不预置四颗外珠；同一组外珠直接变为设备，回程由机器节点形成图标外珠。
仅外环可以复制。标题缺口到位后整句淡入。
高光边缘跟随带宽收薄，避免游离的细碎圆环；反向收尾使用同一套几何与材质。
本节覆盖前文旧版过渡细节。过程帧与检查见 [外环复制记录](../../docs/handoff/site-ring-branch-20261117.md)。
仍仅本地，GUI、下载入口和窗口底色保持此前恢复后的状态。

## 导航、安装与浅色材质（2026-11-18）

Build with Khor 的图标、正文、下载按钮及完整平台／CLI 安装组共用同一布局。
短屏需要的额外高度保留在普通文档流中；CLI 命令框按内容宽度收缩，窄屏居中、超出时换行。
Other ways to get Khor 与 or Khor CLI 共用字体、字号、字重和颜色。
三幕 KhorNet 的图标与标题始终使用同一品牌绿。

`brand-artwork.ts` 保留原始图标的几何与深色绘制，为浅色页面设计透明玻璃带、
薄片菱形、折射边缘与日光环境；官网雾白为 `#f6f7f5`，玻璃色阶来自既有 token。Meet、Build 与两端形变共用这一份材质，
没有附加底板或另一套图形。第一幕有线网标题按实际字形及设备／边界留白选择固定锚点，
不在漫游中移动；KhorNet 外圈只使用自己的标题遮罩，避免原生网络标签擦掉外圈。
实现与无界面 Chrome 证据见 [本轮记录](../../docs/handoff/site-navigation-installations-20261118.md)。
本轮仅本地，尚未提交或发布。

浅色材质已按方向稿落地为日光玻璃，替代之前的烟色菱形：面内通透、外环以明暗边缘表现厚度，
绿色保留在原始内环和节点，环境照明改为宽日光与少量透射色。Fog 参数不变，末端渐变自然接入雾白。
详细实现与过程帧见 [浅色玻璃记录](../../docs/handoff/site-daylight-glass-20261118.md)。

## GitHub Pages

可发布到 `https://colehank.github.io/khor/`：

```sh
KHOR_SITE_URL=https://colehank.github.io KHOR_SITE_BASE=/khor/ npm run build --prefix apps/site -- --force
```

输出仍是 `apps/site/dist`，将其内容作为 `gh-pages` 分支根目录发布。产物含 `.nojekyll` 和 `install.sh`，GitHub 托管静态产物；构建在本机完成。主域名部署使用 `KHOR_SITE_URL=https://khor.cloud KHOR_SITE_BASE=/`。基路径同时作用于页面、Markdown、资源、下载推荐及演示 GUI 的下载回复。

官网安装脚本使用仓库原始安装器，并将默认版本固定到 `src/releases.ts` 的已发布预览版；保留 `KHOR_VERSION` 覆盖能力。当前只有预发布版，不能通过 GitHub 的 `/latest` 入口选择它。安装脚本只作为静态文件输出，不在构建或发布时执行。

本次发布与 Cloudflare DNS 状态见 [Pages 发布记录](../../docs/handoff/site-pages-20261112.md)。

## 部署

2026-11-16 当前已发布：UI 源码 `ed0623a9` → gh-pages `b18d1bd8`，<https://khor.cloud/> 公开版本与手机／桌面 Chrome 验证通过，Pages run `43851024328` success。最新细节见 [发布记录](../../docs/handoff/site-pages-20261112.md)。


用户本轮新增 GitHub Pages 发布授权；feature 分支不合并 main，网站静态产物发布到独立 gh-pages 分支。
2026-11-12 整页收整与定向 Chrome 验证见 [本轮记录](../../docs/handoff/site-refinement-20261112.md)。
现有 `deploy/push.sh` 继续负责静态站与下载镜像的分阶段同步；页面同步保护 `/dl`，
版本校验完成后才推进下载指针。详细约定见 [镜像维护](../../scripts/release-mirror.md)。
中继、寻址和官网分别部署。

2026-11-13 本地全页精修与 Safari 缩放验收见 [本轮记录](../../docs/handoff/site-zoom-20261113.md)。手机完整首图、桌面可操作 GUI，普通首页已获用户 Safari 真机放大／滚动通过反馈；已随 2026-11-13 网站更新发布，版本与公开验收见 [Pages 发布记录](../../docs/handoff/site-pages-20261112.md)。
