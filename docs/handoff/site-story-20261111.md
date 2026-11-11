# 官网本地迭代 · 2026-11-11

用户授权：先实现讨论的官网，随后明确改为本地快速迭代 Web，可在本机运行网页构建、预览与浏览器验证。该授权只作用于本任务；没有合并、发布或生产部署授权。

- Worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`
- Branch：`feat/site-story`；基线 `97bbf2a4e036f1a334c9921961d3d0b283f46c59`
- 预览：`http://127.0.0.1:23820`，Astro dev daemon（启动报告 PID 4944）
- 用户 main 的 handoff 改动与未跟踪 lab-preview、23950/23961 的现有 Lab 和用户 App 均保留。

## 叙事定稿

1. You have many devices. 四颗明暗不同的珠。
2. With many agents. 珠上出现 session face。
3. But one mind. 四个外节点缩到最终尺寸并变暗，face 淡出；中心珠以原图最终的绿色、尺寸和材质浮现。
4. How about letting them work together? 一次连续动效形成内环与菱形等灰阶结构，Mind 始终绿色，四个外节点在此转绿，外圆不出现；停在这一句也会自动完成，不拆成多个滚动步骤。
5. Meet khor. 显现外圆并恢复外围原图颜色与光泽，形成完整 icon，随后同一个 SVG 缩入笔记本；屏内光标短距离移动并只点击一次，打开真实前端。
6. Keep going. 笔记本合盖，手机接续同一个任务。

## 最新用户修正

用户指出网站图标应为真实 khor icon，最后从简化图跳到完整图标生硬。随后明确四个外节点在 Meet 前保持灰暗，Meet 后再上色；How about 是一步演化到最终形态。
已删除单色几何到整图的换图方式，直接使用 `mandala-glass.svg` 原始图层：
不改原图，运行时为连续图层加动画容器与揭示遮罩，保留路径、渐变、阴影和绘制顺序。
每句文案对应一个目标姿态和一次连续动效；How about 的所有图层共用同一演化进度并重叠展开。整枚叙事图标在 Meet 前保持灰阶，Meet 时才恢复原色。外珠初始放大幅度已降低，位置保持不变。
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

## 第二轮观感修订

- 场景入口触发完整的动效，停下滚动也会完成当前句子的演化；How about 1600ms，Meet 1100ms。
- 单一动画循环处理姿态插值，快速滚动重新定向当前姿态；直接跳 Meet 会先完成灰阶形态再上色。减少动态效果直接显示完成图并保留手动入口。
- 光标已移入笔记本屏幕的裁切区域，完成短移动与一次点击后退出。点击过程不会抢焦点或重复执行。
- 验证通过：How about 停止滚动后完整成形且无彩色；Meet 上色；倒滚回灰；屏内点击一次；App/草稿/手机接续与展开返回；390px 无横向溢出；减少动态效果/键盘入口。
- 第二轮完成图仍与标准 SVG 的 780×780 参照逐像素相同。灰阶图中部区域的 RGB 通道差为 0。
- 新证据在 `/Volumes/disk0/Khor/evidence/site-story-20261111/revision-2`；脚本 `/private/tmp/playwright-test-khor-revision-2.js`。

## Mind 与外节点修订

用户明确 Mind 从浮现开始就应保持最终绿色与形态；四个外节点在 One mind 时就缩到最终大小。
已将灰阶处理移到外围结构和外珠，中心珠始终使用原图材质且不再缩放。外珠缩放由 focus（One mind）驱动，How about 和 Meet 阶段不再改变它们的大小与位置。
浏览器确认 One mind、How about、Meet 的全部五珠几何矩阵相同；中心材质保持原始绿色，四个外珠在 Meet 前仍为灰阶。类型检查与定向浏览器检查通过。截图与收据在 evidence 的 `mind-final` 子目录。

## 外圆推迟到 Meet

用户明确 How about 时除 Mind 外都为灰阶、没有圆周，Meet 后再显示完整。按外圆理解并已在 commentary 说明：外圆的显现随 Meet 的 colour 进度，How about 仅完成内部灰阶结构。Mind 的最终绿色、外珠在 One mind 达到最终尺寸的约定继续保留。

## 首屏全绿修复

用户报告刚进入时所有节点都是绿色。通过阻断页面脚本及模拟 prefers-reduced-motion 的浏览器入口得到明确失败：脚本就绪前原始全彩 SVG 可见，减少动态效果在 scroll=0 时直接进入 Meet/color=1。普通模式稳定后仍为 devices/color=0，因此此前只验稳定态漏掉了这两项。
修复：图稿在首个正确姿态画好前保持不可见，避免原图闪现；减少动态效果只跳过动画，不再跳过章节，仍从 devices/color=0 开始。无 JavaScript 时用静态灰阶图稿兜底。
同一个原始入口检查修前失败、修后通过，包含阻断脚本、减少动态效果、普通模式。命令 `PHASE=after node .../playwright-skill/run.js /private/tmp/playwright-test-khor-first-paint.js`；证据 `evidence/site-story-20261111/first-paint/before.json` 与 `after.json`（绝对前缀 /Volumes/disk0/Khor）。

## Khor 内置浏览器颜色问题（用户已确认解决）

用户截图显示 You have many devices 的四个节点仍为绿色，随后说明是在 Khor 内部浏览器观看。最新明确顺序：四珠首屏灰，One mind 缩到最终尺寸，How about 再变绿；Meet 才显现外圆。该指令替代此前 How about 四珠仍灰的约定。
Mac Khor 网页由 WKWebView/WebKit 直接加载，入口 apps/gui/src-tauri/src/lib.rs:2104。已有 WebKit issue 246106 记录 SVG 图层上的 CSS filter 函数不生效；尚待同内核对照复现，不先归因 Khor 自己的业务代码。
当前网页未提交修改已改为 feColorMatrix/feComponentTransfer 原生 SVG 颜色处理，nodeColor 在 How about 恢复。Chrome 深色首屏截图像素为 (155,155,155)，类型检查通过。
保留的旧网页产物：/Volumes/disk0/Khor/artifacts/site-story/css-filter-baseline-58026204，Python 23821（exec session 21958）只作旧实现对照。主预览仍 23820。WebKit 测试内核正下载到 /Volumes/disk0/Khor/caches/playwright/site-story，exec session 94778；不是应用/系统升级。基线/新版本脚本 /private/tmp/playwright-test-khor-webkit-colour.js，环境 BASE_URL/PHASE/ONLY；证据 webkit-colour 子目录。

用户最后回复“解决了”，确认在 Khor 实际内置浏览器中网页的原生 SVG 颜色处理已生效。此轮仅修改网页，Khor 原生浏览器代码未改。四珠仍按最新约定在 How about 转绿，外圆在 Meet 出现。类型检查、静态构建、Chrome 深色实际像素检查通过；不把尚未执行的独立 WebKit 对照记为通过。临时 23821 对照服务及未完成的测试内核下载已停止，主预览 23820 保留。无需为了本问题继续下载或扩大测试。
