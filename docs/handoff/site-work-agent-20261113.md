# Work across the net → Khor Agent · 2026-11-13

当前方向以用户最新纠正为准：**Work across 是用户在各台设备便携地执行具体任务；Khor Agent 是用户把任务交给 Khor**。保留 graph 表达，但不再把第二幕画成能力目录，也不再通过删除 You、替换成 Khor 图标来表达第三幕。

## 两幕讲同一件事

固定任务为 **Fix the export dates**，虚构项目 Atlas Web。工作文件、终端进程、预览与 Git 均在 Mac；Linux 上的 Codex 接收代码审阅请求，不暗示代码自动同步到 Linux。

- [Work across the net](http://127.0.0.1:23820/#work)：You 直接连接每一步任务。先从 Mac 找 Linux 的 Codex 检查日期逻辑，再从 Windows 编辑同一个 Mac 工作区，随后用手机运行 Mac 上的测试、访问其网页预览、检查 Git 变更。访问端图标与来源说明切换，远端动作和文件归属不变。图下文案使用用户动作，最终交付 3 个变更文件、8 项通过的测试
- [Khor Agent](http://127.0.0.1:23820/#khor-agent)：You 和 Khor Agent 始终是两个节点。起步只有 **You → Khor** 的路径亮起，表示交出任务；随后 Khor 依次调用相同五个动作节点；最后把路径方向反转为 **Khor → You**，You 标记为 Your result。用户没有从图里消失
- 五个动作节点为 Review logic、Edit files、Run tests、Check preview、Review changes，标明 Linux／Codex 或 macOS 来源。节点复用生产图标，文字与连接线分开，任务标题在两幕保持一致
- 手机的 Agent 幕把 You 放在图的上方，先看到交付起点。桌面保持分支构图；节点和标签不随阶段移动。Work 22 秒循环，Agent 20.6 秒循环，结果分别停留 4.5／5.2 秒；离屏／后台暂停并续播，减少动态效果显示两幕完成态

## 实现入口

- `apps/site/src/components/ProductTour.astro`：副标题分别为 Keep your task moving, from any device／Give Khor the task, get the result
- `components/concepts/capability-data.ts`：任务步骤、访问端变化、用户直接操作与 Agent 受托执行两套阶段数据
- `components/concepts/CapabilityScene.astro`：始终保留用户身份的 graph 与任务说明；SVG 描述完整说明两种方式
- `scripts/capability-geometry.ts`：桌面／手机固定布局、圆外端点、交付路径正反向
- `scripts/capability-scene.ts`：操作者／访问端、活动任务路径、任务交出／结果返回及循环生命周期
- `styles/capability-scene.css`：任务节点、主机附注、访问端标记短时渐隐渐现

Net、首屏 GUI／手机首图、Fog、图标故事、下载不变。纯静态前端、无真实设备或模型请求。本轮只在 `feat/site-story` 本地实现，未发布、推送或合并。

## 验证与证据

当前证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261113/task-ownership/`；先前 `work-agent/` 与 `work-agent-graph/` 属于已被用户纠正的版本，不代表当前内容。

`checks.json` 记录 320／768／1440px 无横向溢出、文字无重叠或裁切、路径避开标签；桌面 Work／手机 Agent 完整循环。检查区分直接操作与受托执行：Work 访问端 Mac → Windows → 手机；Agent 始终保留 You，交任务时仅 handoff 路径活动，执行时每步只有相应任务路径活动，结束时返回结果。阶段高度与节点几何保持不变，离屏暂停，无页面错误。

`work-edit-live.png`／`work-test-live.png` 为 Windows／手机直接操作阶段，`khor-agent-request-live.png`／`khor-agent-done-live.png` 为用户交付与收到结果。四张 `desktop-*`／`mobile-*` 是减少动态效果下的完成态截图；正常播放截图优先用于判断实际呈现。

定向 TypeScript、diff 格式检查及十页 Astro 静态构建通过。构建产物 `/Volumes/disk0/Khor/artifacts/site-story-20261113-task-ownership/`，日志 `build.log`。仅使用 headless Chrome，没有打开桌面测试窗口，未扩大到原生 GUI、WebKit 或实体手机验证。无独立设计审阅，当前版本交用户本地查看。
