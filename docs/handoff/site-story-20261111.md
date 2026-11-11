# 官网本地迭代 · 2026-11-11

用户授权：先实现讨论的官网，随后明确改为本地快速迭代 Web，可在本机运行网页构建、预览与浏览器验证。该授权只作用于本任务；没有合并、发布或生产部署授权。

- Worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`
- Branch：`feat/site-story`；基线 `97bbf2a4e036f1a334c9921961d3d0b283f46c59`
- 预览：`http://127.0.0.1:23820`，Astro dev daemon（启动报告 PID 4944）
- 用户 main 的 handoff 改动与未跟踪 lab-preview、23950/23961 的现有 Lab 和用户 App 均保留。

## 叙事定稿

1. You have many devices. 四颗明暗不同的珠。
2. With many agents. 珠上出现 session face。
3. But one mind. 外珠与 face 淡下，中心珠浮现。
4. How about letting them work together? 内环照出菱形，珠子变绿，外环及底板显现。
5. Meet khor. 同一个 SVG 缩入笔记本，鼠标点击并打开真实前端。
6. Keep going. 笔记本合盖，手机接续同一个任务。

## 最新用户修正

用户指出网站图标应为真实 khor icon，最后从简化图跳到完整图标生硬。
已删除单色几何到整图的换图方式，直接使用 `mandala-glass.svg` 原始图层：
不改原图，运行时为连续图层加动画容器与揭示遮罩，保留路径、渐变、阴影和绘制顺序。
外珠的明暗、饱和度与放大倍率逐步回到原值；中心、内环、菱形、外环和底板按顺序显现。
成形后的同一个 DOM SVG 持续缩入笔记本，直到打开真实 App。
favicon、导航和下载图标全部使用同一标准图稿。

## 实现与验证

- Astro 站点保留文档，新增下载页与 `/download.json`，共同列出 RC12 已存在的六项 App/CLI 资产。
- `apps/gui/dev/site` 是网站专用入口与示例任务数据；使用生产 App，不连接真实模型或设备。
- 两种设备外形内始终是同一个 iframe，调整视口而不重新挂载。展开/收起使用同一个 dialog；Escape 可从 iframe 内返回。
- Lab fixture 增加可选的叙事数据，原默认场景保留。生成 world 数据从 RC12 官方 frontend artifact 恢复 settings，`fixture_data.py --check` 通过；GUI 源码与 RC12 基线相同。
- 已通过演示/生产前端 TypeScript、站点动效 TypeScript、本地静态构建。
- 浏览器检查通过：跨尺寸任务与草稿保留、手机审批、展开与返回、390px 无横向溢出、减少动态效果、深色、键盘跳过与手动打开、下载页/JSON、工作区及 Web 场景；页面错误为空。
- 完成图与独立原始 SVG 以 780 × 780 同尺寸逐像素比较，差异为空。参照图独立重命名 SVG 资源 ID，避免双 SVG clipPath 冲突。

证据：`/Volumes/disk0/Khor/evidence/site-story-20261111`，包含截图、构建日志和 `verification.json`。
浏览器脚本保存在 `/private/tmp/playwright-test-khor-*.js`；检查使用独立 Chrome 实例，不接管用户已有浏览器。
后续继续本地调整观感与节奏；这不是实际 P2P 或跨硬件任务同步的验收。
