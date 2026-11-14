# 后两幕固定关系图 · 2026-11-14

用户反馈任务动画需要等待、放快又看不清，最新要求后两幕不再变化，可以保留局部闪烁。本轮用始终可读的关系图取代阶段叙事，仅本地预览，未部署。

## 构图

- **Continue anywhere**：保留第一幕四设备位置。笔记本与 Mobile 以轻薄胶囊边界组成 You / Same task 访问组，组边界连接台式机与 Server。Claude Code 和 File 固定属于台式机；Codex、Terminal、Web 固定属于 Server。用户可以从多个设备访问同一任务，图中不再传输智能体或轮换用户位置。
- **Let Khor take over**：同样的四设备与远端能力。You 固定在 KhorNet 外，以 Task 和 Result 两条有方向的弧线连接超边上的 Khor 图标，Khor 的两条固定连接通往远端设备。笔记本和 Mobile 保留在网内，不被强加运行任务的步骤。
- 两幕标题／副文案保持用户确认版本。第一幕 Stay connected 的网络漫游、P2P／relay／hole punching、开场图标演化、首屏 GUI、Fog 与下载均不变。

## 局部动态与实现

`CapabilityScene.astro` 直接输出完整 SVG。`capability-geometry.ts` 复用网络节点位置，分别给宽屏与手机安排静态曲线、访问组和注释；`capability-scene.ts` 只在初始化和 760px 断点切换时更新布局，没有逐帧 JS、阶段计时、载荷或执行进度。

`capability-scene.css` 仅为连接覆盖线、session 小活动环和 Khor 活动环设置 opacity 脉动，周期为既有 dur-240 token 的 16 倍（约 3.84 秒），透明度在 .25–.65 之间。文字、字形、灰色底线与图形几何保持不变，不会整图消失。原 ProductTour 可见性／页面后台处理负责暂停；减少动态效果时完全静止，无 JS 仍保留全部关系和文字。

删除已无使用者的 `capability-data.ts` 与 `task-state.ts`，此前任务播放记录见 [历史版本](site-task-sessions-20261114.md)。没有新增字体、颜色 token、依赖或实际网络调用。

## 验证

证据：`/Volumes/disk0/Khor/evidence/site-story-20261114/relations/`

- `checks.json`：320／390／768／1440px 两幕均无文字与关系线碰撞、横向溢出、重复 SVG ID 或页面错误；全部 session、资源和任务文字常驻，旧阶段节点为零
- 手机 Work／宽屏 Agent 正常动态：仅 opacity 关键帧，脉动过程中 DOM 与图高不变，离屏暂停；减少动态效果时动画数为零，无 JS 页面保留两张完整关系图
- 目视检查手机与宽屏截图；修正手机 Result 与返回线间距、Server Web／Terminal 与访问组的间距，以及文字遮罩对外层超边造成的多余缺口
- 定向 TypeScript、git diff 格式检查、十页 Astro 静态构建通过；产物 `/Volumes/disk0/Khor/artifacts/site-story-20261114-relations`，构建日志 `build.log`
- 图谱审查识别六个代码文件，图谱停留在较早的 `c48dab88`，实际 diff 与定向浏览器检查为本轮依据

所有浏览器验证使用 headless Chrome，没有桌面测试窗口；未扩展到 WebKit、原生 App 或实体手机验证。

本地预览：<http://127.0.0.1:23820/#work>、<http://127.0.0.1:23820/#khor-agent>。线上仍为 2026-11-13 的 `f5c457d7` 源码构建产物，本轮未发布。

## 同日视觉精修

用户认可固定构图后要求进一步提升美观。保留设备、关系、文字与脉动节奏，只精修后两幕：

- 设备圆面、角标与网外 You 共用克制的中性表面渐变、上亮下暗的细边；访问组使用相同方向的半透明层次，均由现有 ink／canvas token 生成，没有滤镜或阴影
- 连接细线与节点轮廓使用不随 SVG 缩放的描边，窄屏维持清晰；连接末端由悬空改为靠近设备边缘的精确接点，访问组出口增加单一汇合点
- Khor 的调用路径从设备上方接入，避开 Claude／Codex／Web 角标；图形位置不动
- Claude Code／Codex 名称提高到正文主层级，资源说明保留次级墨；补充平板字号，You／Task／Result 统一字重

证据 `site-story-20261114/relations-polish/`：320／390／768／1440px 文字与连线避让、无溢出、唯一 SVG ID、静态几何、仅 opacity 动效与离屏暂停通过；最终接点调整另检查 390／1440px Agent。定向 TypeScript、diff 检查和十页 Astro 静态构建通过，产物 `site-story-20261114-relations-polish`。仅 headless Chrome、本地预览，没有部署。


## 同日文字精简（当前）

用户要求更简洁：两幕移除 Codex／Claude Code 的重复文字，保留原始厂商图标和无障碍说明；删除含义不清的独立 Terminal 图标／文字。File／Web 和 You／Same task／Task／Result 保留。同步移除旧文字遮罩、定位与 CSS，避免无文字处还留超边缺口。手机／宽屏 headless Chrome 查看无页面错误、无横向溢出，定向 TypeScript 通过。证据 `site-story-20261114/clean-labels/`；仅本地。


## 同日动效统一（当前）

用户要求后两幕动效与第一幕一致。保留固定构图和精简文字，取消整条线持续呼吸，改为路径上的短细光与到达反馈：

- 三幕共用 `tour-motion.ts` 的 1.7 秒 smoothstep 过渡、1.3 秒停留和 2 秒稳定段，循环为 10 秒。第一幕原数值与缓动函数仅等价抽取，建网／漫游行为不变
- Work 在每圈 0 秒与 5 秒分别提示台式机和 Server 的访问连接；Agent 在 0 秒提示 Task 进入，3 秒提示 Khor 的两条调用连接，6 秒提示 Result 返回
- 底层连线始终可见；流光是额外的短线段，不代表图标、文件或 session 在移动。到达后只让活动环做一次约 960ms 的轻闪，字形本身不闪
- 全部新增运动由 CSS 完成，没有逐帧 JS、模拟执行进度、阶段文案或新增业务状态；图形／文字／图高保持固定。离屏与后台暂停，回到视口续播；减少动态效果时隐藏流光、保留稳定活动环

证据 `site-story-20261114/shared-motion/`：headless Chrome 手机 Work／宽屏 Agent 各取 13 个时间点，核对流光顺序、到达后活动环才增强、DOM／图高不变；另检查实际播放、离屏暂停与恢复、减少动态效果、无页面错误。定向 TypeScript、diff 和十页静态构建通过；产物 `site-story-20261114-shared-motion`。本轮仅本地，没有部署。


## 同日流向与到达依赖修正（当前）

用户指出 Continue 的流向应为远端设备给 You；Agent 必须完整往返，且调用抵达前不能提前点亮智能体。

- Continue：两条路径反向播放，台式机／Server → You。智能体在发出时点亮，访问组的接收环在抵达后才响应；仍为 10 秒循环
- Agent：0–1.7 秒 You → Khor；3–4.7 秒 Khor → 两台机器；4.7 秒后 Claude／Codex 才从中性灰变亮、活动环响应；6–7.7 秒两台机器 → Khor；9–10.7 秒 Khor → You。Khor 分别在收到任务与收到回传后响应，You 的接收环只在最终到达后响应。每次交接保留 1.3 秒，总周期 12 秒
- `capability-motion.ts` 以共享 travel／settle 推导后续时间点，生成浏览器原生 Web Animations 时间线，取代手写 CSS 百分比；新增回传光线复用原路径，底线、节点、文字和资源归属均固定。只在初始化建动画，没有逐帧 JS 或运行中的 DOM 改写
- 智能体未收到调用时保持灰色；接收后逐渐点亮，完成后恢复。Khor 图标本身保持品牌色，不闪烁；反馈由外环表达。离屏暂停／续播，减少动态效果取消时间线并保持静态图，恢复偏好后正常启动

修前 headless Chrome 已复现 Work dashoffset 从 .08864 降到 -.96864（错误正向），Agent 没有机器回传路径。修后升序为 -.96864 → .08864，且两条回传存在。`roundtrip/checks.json` 覆盖手机／宽屏两幕：方向、调用到达前不点亮、点亮后再回传、回传到达后才交付、最终接收反馈、图形／DOM 不变、离屏与减少动态效果恢复全部通过。另保存手机正常播放截图。定向 TypeScript、diff 和十页静态构建通过，证据 `site-story-20261114/roundtrip/`，产物 `site-story-20261114-roundtrip`。本轮仅本地。


## 同日同步访问与持续忙碌反馈（当前）

用户要求 Continue 两台机器同时给 You，Khor Agent 持续闪烁直到交付结束。Work 改为两条设备 → You 路径同步出发、同时到达，5 秒一起循环；去掉原来的交替与多余接收反馈。Agent 的往返顺序不变，Khor 图标与外环从 1.7 秒收到任务起持续柔和脉动，到 10.7 秒结果抵达 You 时准确收束；名称常亮，第一幕图标不变。忙碌区间平均分为约 1 秒的完整呼吸，避免尾端半次闪烁或提前停止。此项替代上一节第三幕 Khor 图标自身不闪的规定。

定向 headless Chrome 检查同步循环、设备 → You 流向、每个忙碌波峰、交付前仍闪／交付后停止和减少动态效果下恢复常亮通过；定向 TypeScript 与十页静态构建通过。证据 `site-story-20261114/busy-delivery/`，产物 `site-story-20261114-busy-delivery`。仅本地，未部署。


## 同日统一用户入口（当前）

用户确认第三幕沿用第二幕的 You＋笔记本＋Mobile 访问组，移除网外独立 You。两幕设备坐标、访问组轮廓、You / Same task 位置及 SVG 画幅／图高相同。Task／Result 从访问组上缘接入 Khor，结果回到同一组边缘后接收环响应；完整往返依存与 Khor 持续忙碌至交付的时序不变。第三幕 Server 的 Web 角标移到右下，给交接曲线留出空间；字形与资源归属不变。

Task／Result 标签采用按实际字宽留缺口的路径遮罩，窗口变化时更新，避免线条穿字；活动 Khor 标记的避让与外部用户画幅解耦。320／390／768／1440px headless Chrome 核对两幕几何一致、无独立用户、文字连线无碰撞、请求和结果确实落在访问组边缘、最终接收反馈通过。TypeScript、diff 和十页静态构建通过，证据 `site-story-20261114/access-group/`，产物 `site-story-20261114-access-group`。仅本地，未部署。

## 同日全英文与光流增强（当前）

用户要求光流更明显、更好看，整个 Web 改为英文。

- `FlowTrace.astro` 统一输出 halo / trail / head 三层 SVG 描边，绿色尾迹外增加低透明宽边，前端为较短的亮芯；反向传输时亮芯仍在前端。复用同一浏览器动画时钟、1.7 秒传输与既定依存，不使用 SVG blur、不增加逐帧 JS。两幕布局、5 秒同步访问、12 秒委派往返和交付后停止脉动均保留
- 首页导航与按钮、下载与设备推荐、五篇文档、404、示例网页、屏幕阅读器标签全部英文，`html lang=en`。320px 导航减小横向内距，为英文标签保留间隔。复制安装命令仍只短暂变勾，英文反馈仅供读屏
- 演示入口 `site-v7-en` 固定英文 catalog；任务、对话、README、文件路径、测试与 Git 提交保持一致地翻译。生产 GUI 原有硬编码的输入提示、复制按钮、完整内容、工具状态与会话配置无障碍标签改接既有 catalog，中文 App 仍按自身语言偏好显示，无新词库和运行时替换
- 用真实 GUI 在 Chrome 1360×850、2×、quality 92 重新生成浅／深 WebP 首图，背景 40% alpha 不变；中文浏览器环境下也保持英文，主题仍跟随浏览器。修正 site identity-marks 原有 `sessionMark('shell')` 缺少 category 参数的类型错误
- 本地文档缓存已刷新，预览 daemon PID 83220，地址仍为 127.0.0.1:23820。未发布远端

验证：demo 类型检查与构建通过，定向 motion/download TypeScript 通过；10 页静态构建通过。headless Chrome 检查 10 个站点路由、机器／额度／工作区／会话的可达页面，正文和 title/aria-label/placeholder 均无中文残留；390px 与 1440px 两幕增强光流以及调用抵达前不点亮通过。320／390／1440 首屏无横向溢出，320px 品牌与导航间有约 14.5px 空隙。手机仅使用透明首图策略不变；未声称本轮做过真 iPhone 性能验证。

证据：`/Volumes/disk0/Khor/evidence/site-story-20261114/english-flow/`；产物：`/Volumes/disk0/Khor/artifacts/site-story-20261114-english-flow`。


## 同日统一收尾下载区（当前）

用户要求最后一页复用首屏下载，并将标题改为 **Build with Khor**。首尾共用 `ProductActions.astro`（Download / Docs）与 `ProductInstallations.astro`（五个平台、可用性、Coming soon、CLI 安装命令及复制反馈）。移除收尾独立的下载文案、版本注释和旧安装行。收尾保留黑底和原有图标／说明，按钮只按背景反转配色。首屏 GUI 与 Fog 不重复加载。

组件以 hero / download 前缀隔离 DOM 标签、tooltip 与复制目标；平台 SVG 通过既有 ConceptMark 隔离 mask ID，避免复用后互相引用。平台提示脚本由共享安装组件挂载，Astro 去重，只安装一次。

验证：headless Chrome 在 320／390／768／1440px 核对首尾操作链接、按钮尺寸、平台与命令一致，无重复 ID 和横向溢出。手机两处复制均写入正确命令，提示可打开／关闭，截图已审视；十页静态构建及 diff 检查通过。证据 `site-story-20261114/shared-downloads/`，产物 `site-story-20261114-shared-downloads`。仅本地，未发布。


## 同日操作按钮图标（当前）

用户要求 Download 旁显示平台图标、Docs 旁显示文档图标。共享 ProductActions 增加同一套 App 字形，首屏与收尾同步；16px 图标继承按钮文字色。下载推荐仍由原来的 detect / recommend 流程决定，新增图标字段；移动平台细分 iOS / Android（包括桌面 UA 的触屏 iPad），未知平台使用通用笔记本。已请求的平台参数优先显示相应字形。没有新增下载包或改变可用性；不支持平台仍进入现有下载选项。

headless Chrome 在 macOS／Linux／Windows／iOS／Android／未知平台核对两个下载图标一致、Docs 图标存在、下载链接正确；320／390／1440px 无横向溢出和重复 SVG ID。定向 TypeScript、十页静态构建、diff 检查通过，手机截图已审视。证据 `site-story-20261114/action-icons/`，产物 `site-story-20261114-action-icons`。仅本地。


## 同日图标导航与唯一 GitHub 入口（当前）

连续三条用户指令合并实施：GitHub 放导航栏最右侧，其他位置不重复 GitHub 入口；导航下载不单独高亮；最后确定右侧导航全部图标。SiteNav 统一 Explore（眼睛）、Docs（文档）、Download（下箭头）、GitHub，前三个复用现有 Khor 字形，GitHub 为 primer/octicons MIT 图稿并附许可证。所有项共用 44px 热区，无下载底色；隐藏文字保留可访问名称，悬停／键盘聚焦出现简短提示。左侧品牌继续 icon + Khor。移除首页页脚、文档 R0 的独立仓库链接；安装文档里的实际下载命令与发布链接保留。

headless Chrome 在 320／390／1440px 的首页、文档、下载页检查：唯一仓库入口为导航最后一个图标；四项都有 SVG、可访问名称、透明背景、44px 热区；提示可通过键盘显示且不超出屏幕。320px 品牌与右侧导航间仍有约 23.7px，无横向溢出。手机截图已审视，demo 类型检查、十页静态构建及 diff 通过。证据 `site-story-20261114/nav-github/`，产物 `site-story-20261114-nav-github`。仅本地，未发布。


## 同日导航图标重新选型（当前）

用户认为上一版不美观，要求寻找好看的图标，以 Apple 为参考。查阅 Apple SF Symbols 与 HIG 的轮廓、重量和尺度说明，以及 Phosphor 官方资产／许可后，导航前三项改成 Phosphor Regular 的 compass、book-open、download-simple。图稿为同一字重与 256 画布，按 22px 显示；GitHub 保留官方 Octicons 实心标记，20px 平衡视觉重量。44px 点击范围、提示、ARIA 和链接不变。SF Symbols 只作为视觉参考，不复制 Apple 图稿。

原始 SVG 与 MIT 许可静态收于 `apps/site/src/assets/navigation/`，上游固定 revision `2b75f3ad12b420c9504ef05df8d2564a28f8500e`，未改路径、未增加 npm 包或运行时请求。移除 GUI identity-marks 中不再使用的导航导出；真实 GUI 和概念图的图标不变。

headless Chrome 320／1440px 核对四个入口、尺寸、可访问名称、透明底与无横向溢出，并审视浅色首屏／黑色毛玻璃截图。十页静态构建和 diff 检查通过。证据 `site-story-20261114/nav-refined/`，产物 `site-story-20261114-nav-refined`。仅本地。
