# 图标故事滚动与手机性能调研（2026-11-12）

用户要求：首屏 Fog 到页面底色的渐变拉长；对 You have… 后续不顺、手机卡顿、一次划过多段的问题先调研实现方式。本轮只修改渐变，以下动画方案尚未实施。前一轮的 GUI 桌面视口修复和删除介绍段落已完成。

## 已修改

`story.css` 中首屏渐变由 5 × gap-pane（120px）延长到 `max(10 × gap-pane, 40svh)`。390×844 采样为约 338px。Fog 参数完全不变。

## 当前实现与证据

- `icon-story.ts` 地标 `[0, .55, 1.15, 1.8, 2.6]`，四次演化共 2.6 屏。390×844 时有效行程约 2194px，各段约 464 / 506 / 549 / 675px；一次长惯性滚动可以经过多段。当前没有 wheel / touch 拦截、滚动吸附或自动翻页，跨段来自滚动距离映射。
- 图形按整段 move 演化，文字却使用 `from + ease(move, 0, .55)`，在该段前 55% 就到下一句；文字与图形不在同一语义节奏上。不能只增加动画时长来修这个错位。
- `animateIcon()` 每次更新所有层，包括当前没有变化的遮罩、滤镜值、材质透明度、节点位置。Chrome 采样 121 帧记录 5260 次 opening 子树属性变化，平均约 43.5 次 / 帧。
- Meet 使用移动渐变遮罩；外节点还逐帧更新 SVG `feColorMatrix` / `feComponentTransfer`。这些操作需要绘制，不能因加了 requestAnimationFrame 或 will-change 就当作已转交合成线程。
- `story.ts` 通过 `offsetHeight - innerHeight` 计算行程，而 sticky 的高度是 `100svh`。手机动态地址栏造成 innerHeight 变化时可能重映射进度；这是代码中的潜在跳动点，本轮没有真机证实其占比。下一版应从同一份稳定的 sticky 几何计算行程，避免混用。
- Fog 已由 Vanta 的 `isOnScreen()` 跳过离屏绘制，但仍有 RAF 循环与布局读取；不能据此认定它一直在图标故事后面绘制 WebGL。示例 GUI 的离屏活动也应单独检查。

诊断脚本 `/private/tmp/playwright-khor-scroll-research.js`，输出 `/Volumes/disk0/Khor/evidence/site-story-20261112/scroll-research`：`report.json`、两份 Chrome trace、`long-fade-mobile.png`。环境为桌面 Chrome、390×844、DPR3、CPU 4 倍限速，不能作为 iPhone 帧率结论。当前和临时禁用 SVG filter 的两组 RAF 间隔中位数均约 10ms，未在桌面模拟中复现明显掉帧；禁用 filter 没有证明它是单一瓶颈。临时隔离只作用于浏览器页面，没有改源文件或提交视觉降级。

## 建议的下一版

1. 保留原生页面滚动和 sticky 舞台，一条连续时间轴统管文字、位置、线条、颜色。不锁住触摸，不做强制逐页翻页。原生快速甩动仍允许跨段；目标是普通手势容易控制、跨段过程连续。
2. 先以桌面约 4.8 屏、手机约 6.2 屏作为可调起点，再实际手感验收。各段从开始滚动就有可见变化，把阅读时间安排在减速收尾，避免重新出现开头长死区。文字切换节点与图形阶段共同编排，取消目前提前半段切完的映射。
3. 保留原始 SVG 的路径、几何和最终图稿，把节点、轮廓、完整材质拆为独立呈现层。灰色节点直接使用预置灰色面，Meet 时与原始彩色材质交叉淡入，避免对整组逐帧做去饱和 / 亮度滤镜。复杂玻璃与高光预先绘制，位置和透明度优先交给合成层；How about 的三条简单轮廓单独处理，Meet 的扫光需要验证遮罩或揭示方案的实际绘制成本。
4. 支持时使用原生 CSS / WAAPI ScrollTimeline；只做一次关键帧和范围初始化。范围基于统一稳定的 section / sticky 几何。旧浏览器保留仅在可见、进度变化时工作的 RAF 后备，并缓存属性值，避免反复写同值。只把可合成的 transform / opacity 视作合成线程候选；描边与 SVG 滤镜仍须采样确认。
5. 减少动态效果仍给清楚的静态姿态。先用 Chrome 检查轨迹、倒滚和普通滚动，再用用户手机验收实际流畅度；不扩大为原生构建或浏览器矩阵。

这些是工程建议，不是对 Apple 营销站内部实现的断言，也不是已完成的性能修复。

## 一手资料

- [Chrome：滚动动画性能案例](https://developer.chrome.com/blog/scroll-animation-performance-case-study)：主线程 scroll 事件与滚动异步，ScrollTimeline 尽量降低主线程工作，CSS 与 WAAPI 都可用。
- [web.dev：高性能 CSS 动画](https://web.dev/articles/animations-guide)：优先 transform / opacity；blur 等会增加绘制成本；will-change 应在采样后谨慎使用。
- [WebKit：CSS 滚动动画指南](https://webkit.org/blog/17101/a-guide-to-scroll-driven-animations-with-just-css/)：单个滚动时间轴按滚动进度前进 / 倒退，停止滚动即停。
- [WebKit：Safari 26.4](https://webkit.org/blog/17862/webkit-features-for-safari-26-4/)：Safari 26.4 引入 threaded scroll-driven animations；不能把这一点外推到所有旧版 WKWebView 或所有可动画属性。
- [web.dev：small / large / dynamic viewport](https://web.dev/blog/viewport-units)：手机地址栏会改变动态视口；svh 稳定，动态尺寸更新频率与手势相关。
