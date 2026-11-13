# 三项亮点精修 · 2026-11-13

用户接受成熟度审阅方案并要求执行，随后要求「后面的动画改成循环的吧」。当前版本替代[连线整理](site-routing-20261113.md)的浅色整图容器、抽象标签与有限播放约定；这是既有官网内三张功能图的局部精修。

设计权威仍为 [设计系统](../design-system/README.md)及其索引，站点继续从既有 token 派生样式。沿用 Khor 黑场、字体、橄榄色强调和生产 App 标记，没有建立新视觉系统，也没有改全局 token、`DESIGN.md`、设计 sidecar 或产品定义。首屏真实 GUI 与手机静态首图策略、Fog 参数、原图标故事、导航及下载保持不变。

## 当前内容与布局

- **P2P**：MacBook Pro 与 iPhone 之间保留一条直接连接。Files 显示 Atlas Web 的 README、日期规则与 CSV 示例；Web apps 显示同一项目 `localhost:5173` 的订单表。选择会切换实际内容。文案明确资源留在 Mac、从手机访问，不暗示文件转移或已连接真实设备。
- **Khor Agent**：请求是审阅 Atlas Web 并总结变化。Codex 在 build-01 检查日期处理与导出测试，Claude 在 Mac mini 阅读变更并写项目说明。结果分别指出 CSV 保留本地日期、README 说明日期列与 CSV 格式；工作阶段显示待办／进行中内容，完成后才显示两条结果与 Ready。
- **Terminal**：固定展示 MacBook Pro 上 `~/atlas-web` 的 `npm run dev`、服务器地址和两条请求记录。Desktop／Phone 只改变访问端、连接强调与 Viewing from 标签，主机与缓冲内容保持不变。

浅色表面只承载可读文件、网页和汇集结果，连线在独立留白中衔接内容。窄屏上下排列，任务说明与结果仍完整保留；320px 时两项工作沿共享分支纵向排列。结果行与状态为阶段变化预留高度，避免循环时页面跳动。

## 循环与控制

P2P 连接信号使用 CSS 循环；Agent 每轮委派／工作／汇集共 3.8 秒，再保留结果 3.6 秒后重来；Terminal 每 4.6 秒切换一次访问端。每图都有独立 Pause／Play，资源与访问端也可手动选择。

只有进入可见范围的图才播放；离屏、页面隐藏或减少动态效果时停止计时和动效。暂停／离屏时 Agent 落在完整结果态，终端保留当前访问端；减少动态效果隐藏播放控件并保留可读静态内容，资源与访问端选择仍可用。自动阶段不使用 live announcement，不抢焦点。实现用有限计时器与 CSS，没有持续的 JS 帧循环、新依赖、canvas、iframe、视频、渲染器或真实 IO。

产品代码范围只有 `src/components/ProductTour.astro`、`src/components/concepts/{Peer,Agent,Terminal}Diagram.astro`、`src/styles/product-tour.css` 与 `src/scripts/product-tour.ts` 六个文件，路径均相对 `apps/site`；未修改生产 GUI 或后端。

## 本轮证据与边界

本地预览 <http://127.0.0.1:23820/#connect>，后两图为 `#khor-agent` 与 `#terminal`。证据目录为 `/Volumes/disk0/Khor/evidence/site-story-20261113/maturity-build/`：

- `direction.md` 记录已接受的局部方向与后来追加的循环要求。
- `verification.json` 最终记录 1440、900、390、320px 定向 Chrome 检查通过，容纳检查无溢出、`errors: []`；覆盖 Files／Web apps、Agent 循环与真实阶段标签、终端切换与缓冲不变、键盘、Pause／Play 及减少动态效果。实现验收同时确认工作／结果阶段高度稳定。
- `desktop-features.png`、`tablet-features.png`、`mobile-features.png`、`small-features.png` 保存四档三图；`desktop-agent-working.png` 保存委派／工作起始态，不代表每一动画帧。主三图截图为局部捕获临时隐藏固定导航，不是全页导航避让认证。
- `mobile-web.png` 已替换为实际 390 × 1100 完整视口、Web apps 选中且主动暂停的截图，包含导航及页面上下文；此前裁切的辅助 locator 图不再作为验收依据。`web-state-geometry.json` 确认该状态端点、资源及整体容器在界内且无自身横向溢出。
- 网站脚本 TypeScript 检查通过；最新 `site-build.log` 记录 Astro 静态构建成功。没有为这轮文档补记而重跑验证。
- `finish-review.md` 是新的只读收尾审阅，结论为批准局部范围内 **ship**，无实质视觉阻塞。它核对当前源码、上述截图及收据，未独立重跑浏览器或构建，也未独立测量所有中间帧和精确计算对比度。
- `detector.json` 有一次成功扫描的空结果 `[]`，早于最后的短标题、状态文案和高度预留细调，不能作为最终源码已完整扫描的证明。更早一次因工作目录错误而提示无法访问目标，不属于干净扫描。当前几何与交互以最终截图和验证记录为依据。
- 定向脚本已归档为证据目录内的 `playwright-test-khor-maturity.js` 与 `playwright-test-khor-web-state.js`，临时副本已删除；`documentation-review.md` 记录本次文档核对。

本轮仅有界的 Chrome、网站脚本 TypeScript 与 Astro 验证，没有新增原生、WebKit 或实际 iPhone Safari 验证，也不证明真实 P2P、智能体任务或 PTY。先前 Safari 真机反馈不扩展为本轮三图验收。本轮未推送、发布或合并；ship 是局部审阅结论，不是部署状态。
