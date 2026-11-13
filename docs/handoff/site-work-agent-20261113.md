# Work across the net → Khor Agent · 2026-11-13

用户明确第一幕为网络基础设施，第二幕为跨网访问资源、终端与智能体，第三幕为 Khor Agent 调用这些能力完成任务，并授权「做出来看看」，随后要求后两幕也使用 **graph** 表达。本轮在 `feat/site-story` 本地实现，替换旧会话选择／资源切换两幕；Net、首屏 GUI 与手机首图、Fog、图标故事、导航、下载保留。未推送、部署或合并。

## 当前画面

- [Work across the net](http://127.0.0.1:23820/#work)：You 连接 macOS 与 Linux 两个设备节点。macOS 分支为 Files／Web／Git／Terminal，Linux 分支为 Codex／Claude／Cursor。路径依次亮起，展示访问网页、编辑文件与查看变更、连接终端、接续 Agent 会话
- [Khor Agent](http://127.0.0.1:23820/#khor-agent)：相同 graph 与节点布局，操作端换为 Khor Agent。“Fix the export dates” 触发读取文件 → 委托 Codex 审阅 → 应用修复 → 测试 → 检查 Web → 返回 3 个变更文件和 8 项通过的测试。完成态只亮起该任务实际用过的路径，不暗示调用了 Claude／Cursor
- 示例统一为 Atlas Web 的订单日期问题。文件、Web 预览和测试始终在 Mac，Linux 上的 Codex 接收代码审阅任务；不暗示代码自动跨机器同步。全部为静态演示，无真实设备或模型请求
- 生产系统／harness／文件／浏览器／Git 标记放在不同层级大小的圆节点中。细曲线连接主机与能力，图下集中展示当前动作与来源。没有文件面板、终端窗口或完整 GUI 截图
- 桌面横向展开，手机纵向错位；分别计算固定节点、标签和路径几何。路径在圆节点外留间距，标签放到支路另一侧，阶段切换不移动节点或文字。沿用站点 token，无新字体、图标包或渲染器
- Work 15 秒循环，Agent 20.8 秒循环（结果停留 5.2 秒）。离屏／后台暂停，回来续播；减少动态效果静态阅读，Agent 显示完成态。阶段调度只用定时器，几何只在断点变化时更新，无持续 RAF／ResizeObserver

## 实现入口

- `apps/site/src/components/ProductTour.astro`：顺序、文案、锚点；保留旧 `#sessions`、`#terminal`、`#resources` 链接别名
- `components/concepts/CapabilityScene.astro`、`capability-data.ts`：共用 graph 和两套阶段数据
- `scripts/capability-geometry.ts`：桌面／手机节点、圆外端点、弯曲路径、标签位置；构建和浏览器共用
- `scripts/capability-scene.ts`：阶段、活动路径、暂停／恢复和断点切换
- `scripts/product-tour.ts`：Net 与新两幕的可见性／前后台生命周期
- `styles/capability-scene.css`、`apps/gui/dev/site/identity-marks.ts`：图形呈现与已有生产标记的构建时导出，生产 App 未改

旧 `SessionDiagram.astro`、`ResourceDiagram.astro`、`tour-data.ts` 及其选择事件／样式已移除。

## 定向验证

最终 graph 证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261113/work-agent-graph/`。此前 `work-agent/` 是被用户要求 graph 后替换的内容片段版，不作为当前截图或验证依据。

`checks.json` 记录 320／768／1440px 的无横向溢出、标签无裁切／重叠、连线不穿过标签；桌面 Work 和 390px Agent 完整循环，图形高度与路径保持不变，离屏暂停及减少动态效果静态阅读，无页面错误。`desktop-work.png`、`desktop-khor-agent.png` 为桌面静态姿态，`mobile-work.png` 为手机整体构图，`390-khor-agent-done-live.png` 为正常循环交付态。

脚本定向 TypeScript、diff 格式检查及十页 Astro 静态构建通过。产物 `/Volumes/disk0/Khor/artifacts/site-story-20261113-work-agent-graph/`，日志 `build.log`。图谱先查但快照陈旧，新增文件尚未入图，结论以当前源文件和定向验证为准。

验证仅用 **headless Chrome**，未打开桌面测试窗口。未扩展为原生 GUI、WebKit 或实体手机验收；既有 Safari 反馈不覆盖本次 graph。本轮没有独立设计审阅，当前版本交用户本地查看。
