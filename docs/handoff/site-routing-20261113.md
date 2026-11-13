# 三项亮点的连线整理 · 2026-11-13

> 已被同日的[三项亮点精修](site-maturity-20261113.md)替代。本页保留当时的连线布局、有限动效与验证记录；当前三图的内容、黑场布局、循环及 Pause／Play 行为以新记录为准。

用户认可继续优化图形布局，指出线条交叠不好看。本轮只改三张概念图的拓扑、节点位置和随布局缩短的结果标签，保留桌面图文并排／手机上下排列、局部浅色画布、生产 App 标记、Khor 色板、资源选择、Replay、访问端选择与有限动效，不改首屏、故事、导航或下载。

这是已接受页面内的连线与构图调整。设计权威仍为 `docs/design-system/README.md` 及其索引，站点继续读取 `docs/design-system/tokens.json`；没有改全局 token、设计权威或品牌身份。方向记录为证据目录中的 `direction.md`，前版构图见 [iroh 参考重做](site-iroh-20261113.md)。

- **P2P**：移除辅助 Linux 节点、背景网线与区域虚线框，只保留 Mac 与手机的一条直接连接。文件／网页共用同一路径，切换资源标签；位置、资源名、端点名称各有自己的区域。更少元素使用较浅画布。
- **Khor Agent**：左侧 Khor 接收示例任务，中间上下两台示例设备上的 Codex／Claude 分别执行，右侧节点汇集。四条路径各占一个象限，只在节点内部衔接；移除外围回绕、重合的收集段和悬浮请求文字。节点说明在圆形节点下方。结果节点复用 App 的 Check 字形，委派与工作中为空心中性状态、标为 Results，收集中显示 Collecting 并强调圆边，结束后才显示绿色 Check 与 Ready。
- **Terminal**：进程便签下面是一根共享主干，在明确的分支处接 Desktop／Phone，两侧对称。说明文字位于主干旁，不再在两条连接上叠白底盖线。

节点文字不画遮线底条。SVG 路径几何和 HTML 节点坐标同步调整；宽窄屏保持同一逻辑，生产 Khor mandala、系统与 harness 标记不重画。字形通过既有 `identity-marks.ts` 在构建时输出，Check 来自生产 `IconCheck`。三图从第一帧可读，减少动态效果下完整静态呈现；三图仍只用 HTML／SVG 和有限 CSS 动效，没有新增依赖、运行时渲染器或后端访问。首屏沿用既有桌面交互 GUI 与手机静态首图策略。

本地预览 <http://127.0.0.1:23820/#connect>，后两图为 `#khor-agent` 与 `#terminal`。证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261113/routing`：

- `desktop-*.png`、`mobile-*.png`、`small-*.png` 保存 1440、390、320 宽的三图、资源切换与局部上下文；`agent-working.png` 保存工作中结果状态
- `verification.json` 记录 36 项定向 Chrome 检查通过、`errors: []`；覆盖图形／标签容纳、采样路径避让、资源切换、访问端键盘选择与 Agent 工作／完成状态
- 几何检查按实际 SVG CTM 采样可见基础路径的中心线，并将可见 HTML 标签框向外扩 3 CSS px 后检测相交；这是一项采样检查，不保证描边外缘在所有字体、尺寸与浏览器中都有绝对 3 px 间距
- 网站脚本 TypeScript 检查通过；`site-build.log` 记录 Astro 静态构建通过，输出为 `/Volumes/disk0/Khor/artifacts/site-story-20261113-routing/`
- `finish-review.md` 记录本轮新鲜的定向布局审阅，`disposition: ship`，范围内没有实质修复项；范围是已保存的三图及状态、宽窄屏局部构图，不是整站或跨浏览器认证，审阅本身未新增浏览器、测试或构建
- `detector.json` 为零项收据，早于最后的 P2P 画布高度调整；最终几何以当前截图和路径采样记录为依据。`documentation-review.md` 记录文档与源码／证据的核对

本轮只做定向 Chrome、网站脚本 TypeScript 与 Astro 静态构建，不运行 GUI／原生／WebKit 矩阵。没有新的 Safari 真机或真实设备／智能体／PTY 证明，未发布、推送或合并 main。
