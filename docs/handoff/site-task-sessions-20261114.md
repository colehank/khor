# Task、远端 session 与跨设备接续 · 2026-11-14

用户确认两幕沿用同一条任务流程，并要求做本地预览。任务只叫 **Task**，不再聚焦 vision model；本轮未发布，线上仍是 2026-11-13 的 `f5c457d7` 产物。

## 文案

- **Stay connected** — Build a resilient peer-to-peer network across devices with Khor
- **Continue anywhere** — Use Codex, Claude Code, or just a terminal from any device with Khor
- **Let Khor take over** — Delegate tasks across devices and agents to Khor Agent

## 动作与归属

两幕保留四设备坐标、KhorNet 超边及沿边 Khor 图标，均为 20 秒循环、四句阶段提示。

Continue anywhere：You 在翻盖笔记本发送对话到台式显示器对应的 Workstation；请求到达后，台式机创建 Claude Code session 并执行、生成本地 File，向笔记本返回完成回执。笔记本向 Server 发送下一条对话，File 直接由台式机传到 Server。Server 收齐请求与文件后才启动 Codex session，运行并启动本地 Web，向笔记本返回访问入口。You 最后移到 Mobile，继续同一个 Task，向 Server 请求页面，再接收页面内容。

Let Khor take over：You 固定在 KhorNet 外，将 Task 交给超边上的 Khor。Khor 发起台式机 Claude session、收回文件完成回执、向 Server 发起后续请求并安排文件传输，收回 Web 入口后交付给 You。四设备仍保留原位置，笔记本和 Mobile 不被强加执行步骤。

- 传输的是消息、文件、完成回执、链接或页面；Claude／Codex 图标只显示在执行设备上，session 不沿路径移动
- session 与输出使用独立标记：Claude Code 留在台式机、Codex 留在 Server，文件与 Web 有明确本地归属。文件交接后源文件仍保留，手机收到页面后 Server 的 Web 仍运行
- 图上每次只有一条当前操作路径。请求抵达前不创建 session，Server 请求与文件都到达后才执行，Web 产生后才可访问，访问请求到达后才返回页面
- `taskState()` 共用纯状态重放；每次操作起步 200ms，载荷移动 1.2 秒后抵达，session 只在抵达后执行。文件／Web 生成后保留 1 秒，再进入下一步；You 的位置切换耗时 1 秒。最终结果分别停留 1.8／2.6 秒。四句提示每句至少 2.4 秒，时序常量由状态与渲染共同读取
- You 从笔记本到手机的访问位置切换沿曲线绕过手机字形，KhorNet 始终保持。离屏／后台暂停，减少动态效果与无 JS 显示完成态；没有真实设备、session 或模型请求

## 实现与验证

用户反馈 10 秒版本太快、看不清。已将两幕调整为 20 秒，分别放慢载荷、执行、用户换设备，并延长关键输出停留，避免仅拉长循环却仍保留原先 700ms 的传输上限。

最新节奏证据：`/Volumes/disk0/Khor/evidence/site-story-20261114/task-pacing/`。`checks.json` 验证手机 Work 与宽屏 Agent 各跑完 20 秒循环，传输均 1.2 秒、两个输出各停 1 秒、四句提示均至少 2.4 秒，到达依赖及离屏暂停通过，路径无文字碰撞或页面错误。定向 TypeScript、diff 与十页静态构建通过；产物 `site-story-20261114-task-pacing`。仅 headless Chrome，本轮仍为本地预览。

实现入口沿用 `CapabilityScene.astro`、`capability-data.ts`、`capability-scene.ts`、`task-state.ts` 与 `capability-geometry.ts`。第一幕只改标题／副文案，原网络拓扑和漫游时序不变；首屏、Fog、GUI、下载与图标演化不变。链接图标复用 `icons.tsx` 的 `IconLink`。

证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261114/task-sessions/`。

- 初版 `checks.json`：390／1440px 两幕完整循环、四句提示、10 秒总长、节点／图高稳定、离屏暂停；对话到达才运行、文件完成回执、文件与请求双依赖、Web 创建与访问依赖、完成后来源保留均通过。缺失 Server 请求的反例不会运行 Codex 或交付结果。无路径文字碰撞、重复 ID、横向溢出或页面错误
- 首次手机查看发现 Server 的 File／Web 标注与路径重叠，移到节点左下方后上述检查通过；静态图标位置不随阶段改变
- `static.json`：320／768／1440px 三幕无溢出，任务两幕均为完成态；禁用 JS 时两个 session 及两幕完成态保留
- `390-work-page.png` 为普通播放的手机访问结果，`320-khor-agent-static.png` 为完整 Agent 构图；连续假时钟截图偶有静态文字栅格缺失，不用于证明文字缺失
- 定向 TypeScript、diff 检查与十页 Astro 静态构建通过。产物 `/Volumes/disk0/Khor/artifacts/site-story-20261114-task-sessions`，日志 `build.log`

全部浏览器验证使用 headless Chrome，没有桌面测试窗口；未扩大到 WebKit、原生 GUI 或实体手机。当前本地预览：<http://127.0.0.1:23820/#work>、<http://127.0.0.1:23820/#khor-agent>。
