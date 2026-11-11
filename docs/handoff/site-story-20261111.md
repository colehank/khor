# 官网本地迭代 · 2026-11-11

用户授权：先实现讨论的官网，随后明确改为本地快速迭代 Web，可在本机运行网页构建、预览与浏览器验证。该授权只作用于本任务；没有合并、发布或生产部署授权。

- Worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`
- Branch：`feat/site-story`；基线 `97bbf2a4e036f1a334c9921961d3d0b283f46c59`
- 预览：`http://127.0.0.1:23820`，Astro dev daemon（启动报告 PID 4944）
- 用户 main 的 handoff 改动与未跟踪 lab-preview、23950/23961 的现有 Lab 和用户 App 均保留。

## 当前叙事定稿（五幕修订，替代下方历史顺序）

1. **You have many devices.** 四颗灰珠戴机器标记，顺时针为 Linux、macOS（Finder）、Windows、iOS（Apple），明暗略有不同。
2. **With many harnesses.** 同一批珠在原位将标记切换为 Claude、Codex、Cursor、Shell，继续保持灰色。
3. **But one mind.** 四个外节点缩到原图最终尺寸并变暗，harness 标记淡出；中心珠和内圆同时以原图最终的绿色、尺寸、形态与材质浮现。不缩放中心，不提前显现光晕。
4. **How about letting them work together?** 一次连续动效沿标准外圆的几何画一根简单灰线，连接四个灰节点。没有菱形、背景、玻璃或环境光；中心珠和内圆保持绿色。停止滚动也会完成这一幕。
5. **Meet Khor.** 一束光从左上方向右下推进，显出原图的菱形、背景和玻璃外圆，四个外珠随光恢复绿色。外圆已在上一幕连好，此处不再描一遍圆周。最后完整回到真实 SVG 图稿。
6. 同一个 SVG 缩入笔记本；屏内光标短距离移动并点击一次，打开真实前端。随后 **Keep going.** 笔记本合盖，手机接续同一个 App 和任务草稿。

## 本轮实现与验证

- 仍使用真实 `mandala-glass.svg` 的连续图层，保留原图的路径、渐变、阴影和绘制顺序。临时连线取自该 SVG 的外圆，不另画一套图标。
- 机器与 harness 标记由 App 的 `machineMark` / `sessionMark` 在构建时输出静态 SVG，取消原来的源码正则提取；网站不因此加载 React 客户端。Finder 遮罩 ID 保持唯一。
- Mind 与内圆不参加灰阶处理；灰珠保留原生 SVG `feColorMatrix` / `feComponentTransfer`，保持 Khor 内置 WebKit 的颜色兼容性。首帧隐藏门和减少动态效果按章节显示的修复保留。
- How about 只驱动连接进度。Meet 的材质用方向渐变遮罩照亮，连接线对应部分退场；四珠使用同一光照方向恢复原色。Meet 不改变圆周描线进度。
- 审查时发现 One mind 过渡中的机器标记可能短暂回闪，已让 harness 状态维持到标记完全退场。
- Chrome 与 WebKit 均已通过五幕、灰阶、内圆/中心几何、无提前光晕、Meet 连线不重描、倒滚、390px 无横向溢出、减少动态效果的定向检查；页面错误为空。
- 两个内核另通过浅色主题、harness 淡出不回闪机器标记、屏内单次点击、笔记本/手机同一 iframe 与草稿接续；网页动效 TypeScript、包含静态标记模块的 demo TypeScript 和 Astro 静态构建通过。
- 完成图对照标准 SVG：主要图形一致；截图差异集中在圆角裁切边缘（不足千像素，单通道最大 8/255），Chrome 另有少量 1/255 的栅格舍入，不宣称本轮截图逐像素零差异。
- 验证脚本：`/private/tmp/playwright-test-khor-five-scenes.js` 与 `playwright-test-khor-five-handoff.js`。截图和收据：`/Volumes/disk0/Khor/evidence/site-story-20261111/five-scenes`。

以下为此前实现与迭代记录；叙事冲突一律以本页当前定稿为准。

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

用户最后回复“解决了”，确认在 Khor 实际内置浏览器中网页的原生 SVG 颜色处理已生效。此轮仅修改网页，Khor 原生浏览器代码未改。四珠仍按最新约定在 How about 转绿，外圆在 Meet 出现。类型检查、静态构建、Chrome 深色实际像素检查通过；不把尚未执行的独立 WebKit 对照记为通过。临时 23821 对照服务已停止，测试内核下载进程已结束，未继续执行额外对照；主预览 23820 保留。无需为了本问题继续下载或扩大测试。
