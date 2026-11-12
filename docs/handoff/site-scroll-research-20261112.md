# 图标故事滚动与手机性能调研（2026-11-12）

用户要求：首屏 Fog 到页面底色的渐变拉长；对 You have… 后续不顺、手机卡顿、一次划过多段的问题先调研实现方式。调研后用户确认「做」，已按下述方案实现。前一轮的 GUI 桌面视口修复和删除介绍段落已完成。

## 已修改

`story.css` 中首屏渐变由 5 × gap-pane（120px）延长到 `max(10 × gap-pane, 40svh)`。390×844 采样为约 338px。Fog 参数完全不变。

## 改动前的实现与观察

- `icon-story.ts` 地标 `[0, .55, 1.15, 1.8, 2.6]`，四次演化共 2.6 屏。390×844 时有效行程约 2194px，各段约 464 / 506 / 549 / 675px；一次长惯性滚动可以经过多段。当前没有 wheel / touch 拦截、滚动吸附或自动翻页，跨段来自滚动距离映射。
- 图形按整段 move 演化，文字却使用 `from + ease(move, 0, .55)`，在该段前 55% 就到下一句；文字与图形不在同一语义节奏上。不能只增加动画时长来修这个错位。
- `animateIcon()` 每次更新所有层，包括当前没有变化的遮罩、滤镜值、材质透明度、节点位置。Chrome 采样 121 帧记录 5260 次 opening 子树属性变化，平均约 43.5 次 / 帧。
- Meet 使用移动渐变遮罩；外节点还逐帧更新 SVG `feColorMatrix` / `feComponentTransfer`。这些操作需要绘制，不能因加了 requestAnimationFrame 或 will-change 就当作已转交合成线程。
- `story.ts` 通过 `offsetHeight - innerHeight` 计算行程，而 sticky 的高度是 `100svh`。手机动态地址栏造成 innerHeight 变化时可能重映射进度；这是代码中的潜在跳动点，本轮没有真机证实其占比。下一版应从同一份稳定的 sticky 几何计算行程，避免混用。
- Fog 已由 Vanta 的 `isOnScreen()` 跳过离屏绘制，但仍有 RAF 循环与布局读取；不能据此认定它一直在图标故事后面绘制 WebGL。示例 GUI 的离屏活动也应单独检查。

早期诊断数值保留在本节，脚本为 `/private/tmp/playwright-khor-scroll-research.js`。环境为桌面 Chrome、390×844、DPR3、CPU 4 倍限速，不能作为 iPhone 帧率结论。旧版和临时禁用 SVG filter 的两组 RAF 间隔中位数均约 10ms，未在桌面模拟中复现明显掉帧；禁用 filter 没有证明它是单一瓶颈。临时隔离只作用于浏览器页面，没有改源文件或提交视觉降级。新版的原始 trace 与诊断报告保存为下一节列出的 `after-*`。

## 实现方向

1. 保留原生页面滚动和 sticky 舞台，一条连续时间轴统管文字、位置、线条、颜色。不锁住触摸，不做强制逐页翻页。原生快速甩动仍允许跨段；目标是普通手势容易控制、跨段过程连续。
2. 先以桌面约 4.8 屏、手机约 6.2 屏作为可调起点，再实际手感验收。各段从开始滚动就有可见变化，把阅读时间安排在减速收尾，避免重新出现开头长死区。文字切换节点与图形阶段共同编排，取消目前提前半段切完的映射。
3. 保留原始 SVG 的路径、几何和最终图稿，把节点、轮廓、完整材质拆为独立呈现层。灰色节点直接使用预置灰色面，Meet 时与原始彩色材质交叉淡入，避免对整组逐帧做去饱和 / 亮度滤镜。复杂玻璃与高光预先绘制，位置和透明度优先交给合成层；How about 的三条简单轮廓单独处理，Meet 的扫光需要验证遮罩或揭示方案的实际绘制成本。
4. 支持时使用原生 CSS / WAAPI ScrollTimeline；只做一次关键帧和范围初始化。范围基于统一稳定的 section / sticky 几何。旧浏览器保留仅在可见、进度变化时工作的 RAF 后备，并缓存属性值，避免反复写同值。只把可合成的 transform / opacity 视作合成线程候选；描边与 SVG 滤镜仍须采样确认。
5. 减少动态效果仍给清楚的静态姿态。先用 Chrome 检查轨迹、倒滚和普通滚动，再用用户手机验收实际流畅度；不扩大为原生构建或浏览器矩阵。

这些是工程选择，不是对 Apple 营销站内部实现的断言。

## 已实施与 Chrome 验收

- `icon-story.ts` 提供纯 `iconState()` 和规范化地标 `[0, .22, .47, .73, 1]`。同一组采样生成原生关键帧，旧浏览器用同一个姿态函数；没有自动播放时钟、触摸拦截或滚动吸附。CSS 指定桌面 4.8 屏、手机 6.2 屏行程。
- 原始 SVG 的平面节点、Mind 与内圆、三条轮廓、完整材质分别呈现于 HTML 层。片段只包含其引用的 defs，ID 唯一，光学偏移 / 比例按原图读取。灰节点有不透明底面，背后的轮廓不会透出。完整原图的滤镜与路径保持静态；Meet 的左上揭示由旋转的 HTML 裁切与相反位移完成，羽化形状不变。不再逐帧改 SVG 饱和度、亮度、遮罩或渐变位置。
- `story.ts` 建立一条根滚动 ScrollTimeline，用像素范围连接 section / sticky 的稳定尺寸；不以 innerHeight 或动态地址栏尺寸作为行程减数。文字、提示、背景与图形共用时间轴；章节点仅在跨界时更新 aria-current。无原生能力时 RAF 只在故事可见、进度变化时工作，属性缓存避免写相同值；减少动态效果显示静态章节。
- 临时 Chrome 脚本 `/private/tmp/playwright-khor-scroll-timeline.js` 已核对桌面 / 手机尺寸、强制移除 ScrollTimeline 的后备、减少动态效果，共四种条件的五个姿态；检查了倒滚可逆、无页面错误、How about 灰节点 / 简单轮廓、Meet 原图完整。完整图稿对照截图的平均 RGB 通道差均低于 1/255（旋转合成抗锯齿略有差异）。
- 121 帧普通采样为 15 次属性变化；相同 CPU4 诊断为 12 次，旧版为 5260 次。后一次诊断总 Paint 约 25.9ms、FireAnimationFrame 约 14.1ms；旧观察分别为约 97.0ms / 115.2ms。该采样是桌面模拟中的工作量对照，不是 iPhone FPS 或所有浏览器性能保证。
- 仅运行本地网站脚本类型检查和 Astro 静态构建，没有 GUI / Rust / 原生构建或远端操作。证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261112/scroll-timeline`：`checks.json`、`after-report.json`、`after-current-trace.json`、各姿态截图、`final-actual.png` 与 `final-canonical.png`、`static-build.log`。Fog 参数、GUI、下载和页面顺序沿用上一轮结果。

## 一手资料

- [Apple AirPods Pro 产品页](https://www.apple.com/airpods-pro/)：本轮核对的 Audio Performance 段有独立 sticky 容器与视频播放区间；HTML 的播放终点为容器底部减 168vh，移动断点另外指定播放起点。对应 [CSS](https://www.apple.com/v/airpods-pro/t/built/styles/overview.built.css) 将 sticky 舞台与额外滚动高度独立安排。它支持将画面停留与播放进度分别编排，不能据此推断所有 Apple 页面都用同一个比例。
- [GSAP ScrollTrigger FAQ](https://gsap.com/docs/v3/Plugins/ScrollTrigger/#how-does-duration-work-with-scrub-true)：scrub 时间轴里的时长代表各段占用滚动距离的比例；总滚动距离与各段比例分开控制。这里借用的是分段编排思路，未增加 GSAP 依赖。
- [Chrome：滚动动画性能案例](https://developer.chrome.com/blog/scroll-animation-performance-case-study)：主线程 scroll 事件与滚动异步，ScrollTimeline 尽量降低主线程工作，CSS 与 WAAPI 都可用。
- [web.dev：高性能 CSS 动画](https://web.dev/articles/animations-guide)：优先 transform / opacity；blur 等会增加绘制成本；will-change 应在采样后谨慎使用。
- [WebKit：CSS 滚动动画指南](https://webkit.org/blog/17101/a-guide-to-scroll-driven-animations-with-just-css/)：单个滚动时间轴按滚动进度前进 / 倒退，停止滚动即停。
- [WebKit：Safari 26.4](https://webkit.org/blog/17862/webkit-features-for-safari-26-4/)：Safari 26.4 引入 threaded scroll-driven animations；不能把这一点外推到所有旧版 WKWebView 或所有可动画属性。
- [web.dev：small / large / dynamic viewport](https://web.dev/blog/viewport-units)：手机地址栏会改变动态视口；svh 稳定，动态尺寸更新频率与手势相关。

## 首幕阅读段（后续迭代）

用户要求首幕再停久一点。当前增加 `--story-intro-hold`：桌面 .5 屏、手机 .7 屏。它是额外滚动距离，演化阶段仍分别占 4.8 / 6.2 屏，所有原始形态与轨迹保留。

原生时间轴起点仍是故事进入 sticky 的位置，采样进度从 `-introProgress` 开始；负进度的 `iconState()` 始终解析为设备首姿态，底部细线从 0 填到 1，然后才进入原有 0…1 的演化。后备使用同一映射，无等待计时器、自动播放或手势拦截。首章节书签返回阅读起点，其余书签计入阅读距离。

临时脚本 `/private/tmp/playwright-khor-scroll-hold.js` 在 Chrome 桌面、手机尺寸、强制后备下确认：75% 阅读距离处节点位置与首帧相同、标题完整、细线已反馈；跨过阅读距离后节点开始移动；后续四个地标完整，倒滚后恢复首姿态。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/scroll-hold/checks.json` 及同目录截图 / 构建日志。类型检查与 Astro 静态构建通过，没有扩大原生或浏览器验证矩阵。

## 文案先行（手机截图反馈后的修正）

本节为已被后续替代的中间方案。最新时序见下一节「文字与图形并行」。

用户指出动画接近完成才出现对应文案，截图中 One mind 几乎消失而连接线已开始绘制。原先把文本透明度绑定于 `state.phase` 的整数端点，中间会同时淡出：Chrome 复现连接段 45% 位置时，轮廓已约完成 70%，How about 的 opacity 为 0，One mind 仅约 .048。

当前 `iconSegments` 分别指定文案完成点（该段 12%）与图形开始点（14%）。`copyPhase` 与图形进度由同一个滚动位置解析，标题先交代新概念、随后图形演化、标题保持到段尾。标题采用互补交叉淡入，任一时刻至少有一句保持一半透明度，不再形成两句都消失的区间；原生采样显式包含分界点，避免插值让图形提前启动。分页与 aria-current 跟随文案阶段。

Meet 的暗色背景与白色标题一同进入，图标扫光仍等文字交代后开始。首幕额外 .5 / .7 屏阅读、总演化距离、原始图稿和原生滚动方式保留。

临时脚本 `/private/tmp/playwright-khor-caption-timing.js` 先红后绿，在 Chrome 手机深色、桌面浅色、强制后备下核对：文案先完整到达且图形尚未开始；各段 20% / 45% / 80% 文字保持完整；换字区间不全空、分页匹配、倒滚可逆，浅色 Meet 标题有暗底。类型检查与 Astro 静态构建通过。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/caption-timing`，包含 before / after 截图与 JSON。

## 文字与图形并行（当前实现）

用户进一步指出每次先出现文字再动画，拉长了每幕体感。现保留首幕独立阅读与正常总行程，只改段内编排：字幕与图形从段首同时起动，前 8% 完成字幕交接，图形在前 86% 完成，最后 14% 维持完整姿态。`iconSegments` 明确 `copyEnd / motionStart / motionEnd`，原生关键帧与 RAF 后备共用同一份计算。

字幕最初使用独立裁切带内的上下位移；用户随后要求渐变切换，现改为固定位置的互补渐隐渐现，交接仍只占每段前 8%。首页品牌黑场固定，避免浅色偏好导致故事文字落在错误底色上；没有改变 iframe 的系统主题。减少动态效果改为五句文案和最终图标的一段静态排版，手机测得 760px 内容高 / 844px 视口，无需经过原来的长 sticky 区间。

Chrome 定向检查确认前两次转场在换字区间就有节点位移，各段中部标题完整，裁切带不重影，最终 Meet 原图仍保留。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/holistic-implementation/checks.json` 及同目录截图。脚本 `/private/tmp/playwright-khor-holistic.js`；未扩大到原生构建或其他浏览器。更多整页实现见 [本轮记录](site-refinement-20261112.md)。
