# Net 漫游场景 · 2026-11-13

仅替代[网络、会话与资源三幕](site-net-sessions-resources-20261113.md)中的第一幕 Net，位于 `feat/site-story` 本地工作区。用户明确指定设备分布、iPhone 漫游，以及中继通信与尝试直连并行后由直连接管；局部最终审阅为 **ship for local user preview**，无实质修正。这不是用户审美验收、整页通过或部署授权。

第二、三幕未改，仍待用户方向；保留此前 critique，不标记整组问题已解决。设计权威仍为[设计系统](../design-system/README.md)与既有 token，全局设计文档及配置未改，未创建 `PRODUCT.md`。首屏 GUI／手机首图、Fog、图标故事、导航与下载保持不变。

## 场景与行为

- Linux 与 macOS 固定在 Wi-Fi A，Windows 固定在 Wi-Fi B；同一部 iPhone 带 Apple 标记与 iOS 标签，在 A／蜂窝／B 之间移动，通信目标始终是 macOS。
- 19 秒循环依次展示 A 内直连、移至蜂窝后中继承载流量并同时尝试直连、成功后直连接管、进入 B 后仍访问 macOS，再经蜂窝回到 A。这是示意路径，不承诺蜂窝必需中继或直连必定成功。
- 按用户最新要求，四台设备统一使用圆形节点与生产 App 平台标记，不使用硬件外形；两片网络区域和独立中继节点保持清晰。活动路径用绿色，候选直连用较淡虚线；路径避开设备标签。宽屏为展开场景，窄屏切换几何布局。
- Wi-Fi A／Cellular／Wi-Fi B 位置选择、状态说明与 Pause／Play 保留。自动循环独立于滚动进度，仅可见且前台时运行；离屏／页面隐藏停止，减少动态效果时不循环，仍可手动选择。无 JS 时保留静态首态。
- 使用 SVG／DOM、移动阶段的有界 RAF 与 CSS 信号运动；没有新增渲染器、依赖或真实网络连接。

入口：<http://127.0.0.1:23820/#connect>。源码位于 `apps/site/src`：`components/concepts/NetworkDiagram.astro`、`scripts/network-scene.ts`、`styles/network-scene.css`；`scripts/product-tour.ts` 接入场景，`components/ProductTour.astro` 调整首幕布局，`styles/product-tour.css` 移除旧网络样式。

## 节点修订

用户要求用节点抽象表达设备后，四种硬件外形已替换为等大圆形节点，系统标记居中；连接端点按节点半径重新计算，手机布局也保持相同节点尺寸。网络分组、漫游顺序与中继／直连时序不变。仅做桌面 1440px、窄屏 390px Chrome 定向查看：A、蜂窝、B 代表状态无横向溢出或页面异常，定向 TypeScript 与 diff 格式检查通过，未重跑完整测试、构建或独立设计审阅。最新截图及 `verification.json` 位于 `/Volumes/disk0/Khor/evidence/site-story-20261113/net-nodes/`；下方 `net-roaming` 截图属于修订前的硬件外形。

## 验证与局限

规范证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/`。

- [方向](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/direction.md)与[最终审阅](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/finish-review.md)记录用户指定的场景和局部 ship 结论；没有获批视觉稿，不作稿件还原声明。
- 四档完整场景：[1440px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/desktop-cellular.png)、[900px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/tablet-a.png)、[390px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/mobile-cellular.png)、[320px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/small-cellular.png)。这些截图启用减少动态效果并临时隐藏导航／跳过链接；临时审阅副本已移除，以上原件为继续入口。
- [运行中的中继](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/desktop-relay-playing.png)保留信号标记与 Pause；[手机视口](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/mobile-viewport.png)保留导航及普通动态模式下暂停的控件。B 状态截图有局部栅格缺失，只支持路径／位置核对，构图以四档完整截图为准。320px 候选虚线较淡，旁边状态说明清楚；审阅认为不阻碍本地预览。
- [verification.json](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/verification.json)：四档宽度、三种位置均无页面或图内横向溢出，同档图高稳定，四种设备标签齐全，`errors: []`。八项检查覆盖中继与候选并行、蜂窝直连接管、进入 B、暂停、暂停时键盘选择、离屏停止、既有会话选择、减少动态效果。
- [验证说明](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/validation-notes.md)记录 `product-tour.ts`（含 `network-scene.ts`）定向 TypeScript 通过；[构建日志](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/site-build.log)记录 Astro 十页构建成功，产物为 `/Volumes/disk0/Khor/artifacts/site-story-20261113-net-roaming/`。[检测结果](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/detector.json)对改动 Astro 标记成功返回 `[]`，不扩展为独立样式表扫描保证。

截图不证明实际动画帧率或实体 iPhone 流畅度；本轮未验证 Safari／WebKit、真实网络、后台或原生 GUI。旧 Safari 反馈不覆盖本场景。图谱已先查询但陈旧，当前源码与定向证据优先。审阅和[文档核对](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/documentation-review.md)未重跑测试或构建；本轮未推送、部署或合并。
