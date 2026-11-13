# Work across the net → Khor Agent · 2026-11-13

用户否定能力树与任务星形图，要求**继续使用第一幕 Net 的节点、超边和布局**，展示沿路径传文件、监控任务的完整过程。以下替代旧 `task-ownership/` 与 `work-agent-graph/` 版本；旧审阅不代表当前审美验收。

## 当前内容

同一个虚构任务 **Build and check Atlas Web**，四节点为 Workstation、Server、Desktop、Mobile。任务幕沿用第一幕的稳定设备位置，但只保留包围四台设备的 KhorNet 超边，移除内部 LAN／Wi-Fi 分组；标题只写 KhorNet，不加 P2P／relay 后缀。第一幕网络基础设施仍展示原生分组与连接状态。

- [Work across the net](http://127.0.0.1:23820/#work)：You 从 Workstation 开始，与 Codex 准备项目；把 `atlas-web.tar` 传到 Server，再启动 `npm run build && npm test`。用户切到 Mobile，Server 继续执行；随后查看同一终端、打开服务器的网页预览、将报告送回 Workstation，结果为构建完成与 8 项测试通过
- [Khor Agent](http://127.0.0.1:23820/#khor-agent)：You 始终位于 Mobile。先向 Workstation 上的 Khor 交任务，Khor 调用 Codex 准备项目，传到 Server 执行、监控、检查网页和收集报告，最后把结果交回 Mobile。Khor 是 Workstation 上的会话徽标，不是第五台设备；用户始终在图上
- 每步只有一条临时操作路径，一个文件／终端／网页／消息／结果符号沿路径移动。Server 上的进度环和状态独立呈现，节点上只显示当前必要的徽标。Desktop 保持可用，不虚构其参与任务
- Work 24.2 秒、Agent 27 秒自动循环，结果均停留 4.6 秒；离屏／后台暂停并续播。减少动态效果与无 JS 均保留完成态。仅移动载荷和进度变化时使用有界 RAF

## 设备图标与开场

新增 `IconServer`、`IconWorkstation`、`IconDesktop`、`IconMobile`，沿用 `icons.tsx` 的 RailGlyph、光学校准、线宽与 currentColor。双层机架、紧凑主机、显示器与独立塔式主机、圆角手机通过轮廓区分，细节只保留接口和状态灯。

`identity-marks.ts` 在构建时导出同一套图稿。**You have many devices** 的四颗灰节点也使用它们，随后仍切成 Claude／Codex／Cursor／Shell。开场的几何演化和最终 Khor 图标未改；安装平台与真实 GUI 仍使用各自系统标记。

## 实现入口

- `NetworkGraph.astro` 与 `network-map.ts`：提取第一幕现有图形和渲染，三幕共用；每个实例的 SVG ID 有独立前缀，沿边标题保留真实字宽与两端 8 单位缺口
- `capability-data.ts`：同一任务的手动操作／Khor 受托执行阶段、操作者、进度、载荷
- `CapabilityScene.astro`：任务标题、共用网络、Server 进度、会话徽标、You 与阶段说明
- `capability-geometry.ts`：调用原 `networkGeometry`，只额外计算当前临时操作曲线与圆外端点
- `task-state.ts`：传输到达时刻与依赖状态，共用于载荷位置和目标机器状态。先收到项目才能启动终端，终端指令到达才推进构建，收到网页／报告／结果才显示对应标记；每个阶段继承已经完成的动作，不在切阶段时提前显示目标状态
- `capability-scene.ts`：阶段循环、有界载荷与进度动画、暂停续播；`product-tour.ts` 保留统一可见性与浏览器偏好生命周期

首屏 GUI／手机首图、Fog 参数、导航、下载保持既有行为。全程静态前端，不接真实设备、Lab 或模型。本轮仅本地，未推送、部署或合并。

## 定向验证

证据：`/Volumes/disk0/Khor/evidence/site-story-20261113/network-task/`。

- `checks.json`：320／768／1440px 无横向溢出、重复 ID 或断裂 textPath；三幕节点、viewBox 与稳定外轮廓一致。Work／Agent 在 390 与 1440px 各完成全部阶段并循环，节点与幕高不跳动，移动载荷持续推进，离屏后停止。当前任务路径不穿文字字形（外扩 4 CSS px 检查），无页面错误
- `opening-checks.json`：390／1440px 的四枚开场图稿与 Net 图稿一致、完整显示；切换到 harness 阶段正常。`device-icons.png` 对照 64／36／24px，`desktop-opening-devices.png` 与 `mobile-opening-devices.png` 为开场截图
- `390-work-monitor.png`、`390-khor-agent-transfer.png`、`1440-khor-agent-deliver.png` 为正常播放、内容完整的当前构图。初次预览的 `mobile-khor-agent.png` 有 headless 连续截图的静态栅格缺失，不作为完整视觉证据
- 定向 TypeScript、diff 格式检查、十页 Astro 静态构建通过。产物 `/Volumes/disk0/Khor/artifacts/site-story-20261113-network-task/`，日志 `build.log`
- 图谱先查，变更审阅图谱仍落后于 HEAD，因此以实际源码与浏览器证据为准。仅 headless Chrome，没有桌面测试窗口、原生 GUI 构建、WebKit 或实体手机验证，没有独立设计审阅

## 到达顺序与 KhorNet 精简（本轮修订）

用户指出终端尚在传输、Server 已经显示终端并执行。修改前 headless Chrome 的 `before.json` 记录进入 run 时目标进度已为 4.36%，回归断言先失败。现在载荷与接收状态共用到达时刻（阶段的 62%），剩余时间用于执行与阅读，指令在途时 Server 只保留已收到的文件标记；到达后才切为终端并开始进度。Mobile 的终端／网页、Workstation 的报告、Agent 的接单和返还结果同样遵守到达依赖。Desktop 改为显示器＋独立主机，并通过共享图稿同步开场与三幕。

最新证据：`/Volumes/disk0/Khor/evidence/site-story-20261113/task-arrival/`。`checks.json` 核对两条流程每种载荷到达前后、命令在途时进度为零、到达后启动、最终结果及离屏暂停；1440px Work 与 390px Agent 的实际 DOM／截图同步核对。`static.json` 确认 320px 减少动态效果下两幕为完成态、无溢出、开场 Desktop 图稿一致。后两幕仅 KhorNet，网络基础设施仍保留原分组与 P2P／relay。定向 TypeScript 与十页静态构建通过，产物 `site-story-20261113-task-arrival/`。仅 headless Chrome，未发布。
