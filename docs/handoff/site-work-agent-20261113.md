# Work across the net → Khor Agent · 2026-11-13

> 2026-11-14 当前实现已改为通用 Task、台式机 Claude → 文件 → Server Codex／Web → 手机访问，见 [最新接续入口](site-task-sessions-20261114.md)。下文为 2026-11-13 已发布的模型训练版本。

当前使用同一个 **Train a vision model** 任务，替代早期 Atlas Web 构建故事。用户要求快速、直觉地看懂任务，因此两幕只保留四句提示，不展示脚本准备、文件名、训练轮次或逐步技术解释。继续复用第一幕 Net 的四节点与超边布局。

## 当前叙事

- [Work across the net](http://127.0.0.1:23820/#work)：You 在 Workstation 与 agent 发起任务 → Server 终端训练 → You 到翻盖笔记本查看结果、修订并交回 Server → You 在 Mobile 试用 Web。共 10.4 秒；末段传输到达后保留约 2.1 秒的完成态
- [Khor Agent](http://127.0.0.1:23820/#khor-agent)：You 始终在 KhorNet **外**，交出目标；**超边上的既有 Khor 图标**接单并路由 Workstation agent、Server 训练与笔记本修订，再收集模型并把结果与 Web 交回 You。共 11 秒、四句提示；相邻路由步骤共用同一句，最后约 1.3 秒保留交付态
- 两幕只显示 KhorNet 外层超边及名称，不显示内部 LAN／Wi-Fi 或 P2P／relay 后缀。一次只显示一条当前任务路径和一个载荷，进度环显示 Server 工作。没有额外 Khor 机器节点，也不把 Khor 放在 Workstation 上
- 设备位置、外层超边来自第一幕的稳定几何；Agent 视框向上扩展容纳网外 You，设备本身不移动。标题缺口按真实字宽计算，Agent 的 Khor 图标环与文字间距单独留足
- 所有状态遵守到达依赖：训练命令到达后才出现终端与进度；训练完成才可审阅，修订结束后才可开始第二次训练；Web、模型收集与结果返还依次到达后生效
- 自动循环，可见且前台时运行，离屏／后台暂停并续播。减少动态效果和无 JS 显示完整完成态；RAF 只用于正在传输或推进的任务，不在结果停留期间空转

## 图标与实现

四设备沿用真实 App 图标体系：Workstation = 带底座的台式显示器，Server = 机架，Desktop = 翻盖笔记本，Mobile = 手机。开场 You have many devices 已复用同一套图稿；随后仍换成 Claude／Codex／Cursor／Shell。安装平台和真实 GUI 各自系统标记不改。

- `NetworkGraph.astro`／`network-map.ts`：三幕共用图形与标签缺口，每个 SVG 实例独立 ID；可选外部用户视框不改变 Net 场景
- `capability-data.ts`：精简阶段、四句提示、操作者、路由与到达效果
- `task-state.ts`：载荷位移和目标状态共用到达时刻；依次重放已完成阶段，保证缩短时序后仍满足依赖。到达为阶段的 52%，上限 900ms，剩余时间用于执行和查看
- `capability-geometry.ts`：调用原设备几何，只计算任务曲线、Khor 端点和网外用户；手机 Web 路径绕开 Server 状态文字
- `CapabilityScene.astro`／`capability-scene.ts`：静态完成态、任务徽标、进度、用户和播放生命周期；Agent 最后一步同时包含结果交付，减少动态效果必须取该步末尾状态

首屏 GUI／手机首图、Fog、导航、下载和开场图标演化保留。全程静态合成示意，不连接真实设备、Lab 或模型。本轮仅本地，未推送、部署或合并。

## 定向验证

证据：`/Volumes/disk0/Khor/evidence/site-story-20261113/model-training/`。

- `checks.json`：390／1440px 两幕各跑完一轮并回到起点；到达前后状态、两次训练、修订、预览、结果返还依赖通过。每幕恰好四句提示，总时长不超过 11 秒。节点与幕高稳定、路径不穿文字或其他节点、You 在超边外、无重复 ID／横向溢出／页面错误；离屏后停止
- `layout.json`：320／390／1440px 减少动态效果均显示完成态，无横向溢出。`390-agent-refine.png`、`1440-work-preview.png` 为正常播放截图；`*-final.png` 为静态完成态
- 定向 TypeScript、diff 检查和十页 Astro 静态构建通过；构建日志 `build.log`，产物 `/Volumes/disk0/Khor/artifacts/site-story-20261113-model-training/`
- 只用 headless Chrome，没有桌面测试窗口、WebKit、实体手机或原生构建。截图不证明实体手机帧率。图谱记录落后于 HEAD，以实际源码与本轮浏览器证据为准

早期 Atlas Web／到达修复的历史证据保存在同日 `network-task/`、`task-arrival/`、`web-to-mobile/`。它们不代表当前叙事与时长。
