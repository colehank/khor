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
