# 网络、会话与资源三幕 · 2026-11-13

用户确认设备 → 网络、任务 → 跨网会话、资源 → 会话内使用，并要求「做出来看看」；此前追加的循环要求继续有效。本版替代[三项亮点精修](site-maturity-20261113.md)，仅在 `feat/site-story` 本地实现，未推送、部署或合并。最终局部设计审阅为 **ship**，无实质修正；这不是部署授权或全页验收。

设计权威仍为[设计系统](../design-system/README.md)与既有 token。沿用黑场、系统字体、橄榄色强调和生产 App 标记；未改全局设计文档、token、`DESIGN.md` 或 Impeccable 设计配置，也未补造缺失的 `PRODUCT.md`。首屏 GUI、手机静态首图、Fog 参数、图标故事、导航和下载保持不变。

## 当前内容与行为

三幕共用有示例标注的虚构 Atlas Web 项目：

- **Your devices, your net**：MacBook Pro、build-01 和 iPhone 构成私人点对点设备网，切换 Direct／Relay，信号沿当前路径运行。只说明直连和中继，不声称正在连接真实设备。
- **Your tasks, across your net**：Terminal、Codex、Claude 都是会话。访问端可在 MacBook Pro／iPhone 间切换，会话主机与内容保留；循环交替切换访问端和会话。
- **Your resources, in every session**：延续上一幕实际选中的会话标题、类型标记与主机，切换网页订单表、文件差异和工作区变更。资源来自 MacBook Pro；Codex 会话运行在 build-01。两者有意分开，表达跨设备访问。

各图支持手动选择和独立 Pause／Play。网络每 5.2 秒换路径，会话每 3.6 秒推进一次，资源每 4.8 秒切换；仅可见且前台时循环，离屏、后台和减少动态效果时停止，手动选择仍可用。窄屏上下排列，各选择状态预留稳定图高。

源码均在 `apps/site/src`：`components/ProductTour.astro`、`components/concepts/{NetworkDiagram,SessionDiagram,ResourceDiagram}.astro`、`components/concepts/tour-data.ts`、`scripts/product-tour.ts`、`styles/product-tour.css`。旧 Peer／Agent／Terminal 图已删除。入口为 <http://127.0.0.1:23820/#connect>、`#sessions`、`#resources`；旧 `#khor-agent`／`#terminal` 锚点保留并指向会话幕。没有增加真实 P2P、agent、PTY、Git 或后端操作。

## 证据与边界

规范证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/`。

- [方向](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/direction.md)与[最终审阅](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/finish-review.md)记录范围和 ship 结论。没有获批视觉稿，不作稿件还原度声明。
- [1440px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/desktop-three-acts.png)、[900px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/tablet-three-acts.png)、[390px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/mobile-three-acts.png)、[320px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/small-three-acts.png)保存完整三幕；捕获时启用减少动态效果，并临时隐藏导航／跳过链接，属于局部截图。辅助[手机文件](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/mobile-file.png)、[工作区](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/mobile-workspace.png)、[中继](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/mobile-relay.png)保留导航。临时 `.impeccable/review` 副本已移除，以上原件为继续入口。
- [verification.json](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/verification.json)记录四档 Chrome 无横向溢出、选择时图高稳定、单个会话标记可见及 `errors: []`，九项行为检查通过，覆盖循环、暂停、访问端切换保留终端缓冲、会话传入资源、离屏、键盘与减少动态效果。[验证说明](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/validation-notes.md)说明两次早期资源循环超时来自测试滚动定位；改为立即居中后单独重跑行为检查通过，其间未改产品源码。尺寸与截图来自同源码的前一捕获批次。
- `product-tour.ts` 定向 TypeScript 检查通过；[site-build.log](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/site-build.log)记录 Astro 静态构建成功、十页输出到 `/Volumes/disk0/Khor/artifacts/site-story-20261113-net-sessions-resources/`。[detector.json](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/detector.json)是最终针对 `ProductTour.astro` 与 `components/concepts` 的一次成功空结果，不扩展为单独样式表扫描保证。

审阅与本次文档核对没有重跑浏览器、构建或产品测试。截图不独立证明实时动画流畅度；本轮不覆盖全页导航、全部悬停／焦点状态、精确对比度、Safari／实机、生产 GUI 或真实网络与后台功能。此前用户的 Safari 放大／滚动通过反馈不扩展为新三幕验收。[documentation-review.md](/Volumes/disk0/Khor/evidence/site-story-20261113/net-sessions-resources/documentation-review.md)记录本次文档核对。
