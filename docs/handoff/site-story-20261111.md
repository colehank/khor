# 官网本地迭代 · 2026-11-11

用户授权：先实现讨论的官网，随后明确改为本地快速迭代 Web，可在本机运行网页构建、预览与浏览器验证。该授权只作用于本任务；没有合并、发布或生产部署授权。

- Worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`
- Branch：`feat/site-story`；基线 `97bbf2a4e036f1a334c9921961d3d0b283f46c59`
- 预览：`http://127.0.0.1:23820`，Astro dev daemon（启动报告 PID 4944）
- 用户 main 的 handoff 改动与未跟踪 lab-preview、23950/23961 的现有 Lab 和用户 App 均保留。

## 当前叙事定稿（连续滚动与观看停留区，替代下方历史顺序）

1. **You have many devices.** 四个普通灰色圆节点松散分布，大小与明暗各异，分别戴 Linux、macOS（Finder）、Windows、iOS（Apple）机器标记。前两幕不预先排成图标的四个固定位置。Meet 之前不出现珠光高亮、渐变、边缘反光或珠子的投影。
2. **With many harnesses.** 节点沿各自的短弧线移动，错开将标记切换为 Claude、Codex、Cursor、Shell，继续保持灰色；每个节点的移动起止与缩放不同。
3. **But one mind.** 四个外节点一起归位、缩到原图最终尺寸并变暗，harness 标记淡出；中心节点和内圆同时浮现。中心从第一帧起就是原图的最终绿色和尺寸，但先是纯色圆，光泽等到 Meet。内圆保持原图绿色与几何；不提前显现环境光晕。
4. **How about letting them work together?** 一次连续动效画出外环的内外两道细边，以及只有边缘的菱形；三条线统一细度、灰度，环带中间留空，保留最终玻璃环的占位宽度。没有菱形填充、玻璃或环境光；四个外节点灰色，中心节点和内圆绿色。绘制进度跟随滚动位置，停止滚动就停在当前形态。
5. **Meet Khor.** 原图的暗色环境铺满整个画面，左上方的光让菱形填充、玻璃外环和珠子的高光逐渐显现，四个外节点随光恢复绿色。外环和菱形不重新描线；此时标志没有方形底板边界。
6. Desktop / Mobile 设备体验段已按用户要求移除，包括笔记本、手机、鼠标点击与内嵌体验 iframe。开场之后直接接功能介绍与下载；独立资源体验链接仍保留。导航「体验」改为「功能」，跳过开场也指向功能段。

## 本轮实现与验证

- 最新观感修订：Devices 使用不等大的散布节点；Harnesses 以四条独立弧线和不同的标记切换区间打破同步；One mind 才统一归位。运动仍随滚动进度，停留区内不自行漂浮。最终 focus=1 时明确移除外围节点 transform，保持标准图稿几何。
- 本次只在 Chrome 查看首幕、错开切换、Harnesses、One mind 和 Meet；One mind / Meet 的四个外节点 transform 均为 none，TypeScript 与 diff 格式检查通过。截图：`/Volumes/disk0/Khor/evidence/site-story-20261111/scattered-nodes`。下面的滚轮节奏数据属于上一轮距离调整，本次未重跑。
- 用户反馈连续直线映射仍容易一次跳过几幕。经对照 Apple 官方 AirPods / Vision Pro 页，用户授权按「看清当前状态 → 演化 → 新状态停留」重排距离；保持原生滚动，不做滚轮拦截或整屏吸附。
- 开场实际滚动行程从 4.2 屏增至 9 屏，页面段总高度 10 屏，由时间线数据设置 CSS 变量，移动端共用同一相对视口行程。五个完整姿态的稳定区间依次是 `0–.8 / 1.55–2.5 / 3.35–4.65 / 5.55–6.45 / 7.5–9` 屏。
- 区间之间仍按滚动位置连续插值，停止即停止，倒滑可重现中间态。One mind 稳定停留 1.3 屏，Meet 停留 1.5 屏，其余约 .8–.95 屏。
- 转场内文字进度在前 55% 完成交接，图形在 18%–94% 之间演化，避免每个元素同时启动与结束。章节按钮定位到各自稳定区间中部。减少动态效果使用邻近完整姿态。
- Desktop / Mobile 体验段保持移除，开场直接接功能介绍与下载；纯色节点、细边外环、空心菱形、Meet 的整幕环境和左上光照不变。
- 本阶段按用户要求只在 Chrome 快速查看，不跑 WebKit / 多主题 / 手机矩阵。9000px 行程下，四次各 600px 的滚动停在 devices → 过渡 → harness → harness，没有快速穿过全部句子；3500px 与 4400px 都保持 One mind，4950 / 5100 / 5300px 的轮廓按滚动增长，倒滑 5100px 回到同一中间态。页面错误为空；动效 TypeScript 和 diff 格式检查通过。
- 截图与姿态记录：`/Volumes/disk0/Khor/evidence/site-story-20261111/pacing`。脚本：`/private/tmp/playwright-test-khor-pacing-preview.js`。

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
