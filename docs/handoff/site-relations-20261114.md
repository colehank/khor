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
