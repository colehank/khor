# Net 漫游场景 · 2026-11-13

仅替代[网络、会话与资源三幕](site-net-sessions-resources-20261113.md)中的第一幕 Net，位于 `feat/site-story` 本地工作区。用户明确指定设备分布、iPhone 漫游，以及中继通信与尝试直连并行后由直连接管；局部最终审阅为 **ship for local user preview**，无实质修正。这不是用户审美验收、整页通过或部署授权。

第二、三幕未改，仍待用户方向；保留此前 critique，不标记整组问题已解决。设计权威仍为[设计系统](../design-system/README.md)与既有 token，全局设计文档及配置未改，未创建 `PRODUCT.md`。首屏 GUI／手机首图、Fog、图标故事、导航与下载保持不变。

## 场景与行为（当前）

- Network A 包含 Linux 与 Windows，Network B 包含同一个 Windows 与 Android；Windows 是两条超边共享的节点。iOS 在 A／蜂窝／B 之间漫游，不绘制其独立虚线圈或 Cellular 区域，固定节点错位排布。
- 20.4 秒自动循环：A 内连接、蜂窝中继与候选直连并行、超边收拢到直连设备、进入 B、返回蜂窝，再回到 A。中继是示意路径，不承诺蜂窝必需中继或直连必定成功。
- 全部网络和连接均为闭合超边。中继加入时活动超边包住 Windows／Khor／iOS；直连建立后收拢为设备集合，候选也用较淡闭合虚线表达。同网时只强调所属 Network A 或 B，避免重复轮廓；没有开放的点对点路径或信号流线。
- 圆形节点仅保留平台／Khor 标记与隐藏 SVG 名称，图内仅有 Network A／Network B 标签，通过 textPath 排在对应超边上。移除底部状态说明、位置选择与 Pause／Play。其他两幕控件不变，共用挂载逻辑允许该幕无按钮。
- 可见且前台时循环；离屏和页面隐藏停止，减少动态效果时静止，无 JS 时保留首态。几何变化用 1350 ms 有界 RAF，停留时没有持续 JS 绘制；无新依赖或真实网络连接。

入口：<http://127.0.0.1:23820/#connect>。源码位于 `apps/site/src`：`components/concepts/NetworkDiagram.astro`、`scripts/network-scene.ts`、`scripts/network-geometry.ts`、`styles/network-scene.css`；`scripts/product-tour.ts` 接入场景，`components/ProductTour.astro` 调整首幕布局，`styles/product-tour.css` 移除旧网络样式。

## 共享节点、沿边标题与曲率修订（当前版本）

按用户新拓扑，移除 macOS 标记，Windows 为两网共享节点，Android 位于 B。iOS 在外游走时没有单独区域。Network A／B 标题引用各自边界路径，绘制在所有网络轮廓上方，文字描边让边界从字下退让。曲线收敛为每段一条很轻的 Bézier 弧，节点处保留圆弧；移除额外鼓包与反弯。

证据 `/Volumes/disk0/Khor/evidence/site-story-20261113/net-shared/`：1440／390px Chrome 四阶段共八组成员检查通过，Windows 始终属于 A 和 B，iOS 仅在到达时加入对应集合；无独立 Cellular 区域、无无效路径或横向溢出，两个标题均绑定对应边界，`errors: []`。TypeScript 与静态 Astro 十页构建通过。构图以 `desktop-local.png`／`mobile-local.png` 为准，连续抓帧的后续图片有静态内容栅格缺失，仅用于变动几何核对。未跑原生、实体手机或 WebKit，未发布；其他两幕不变。

## 自循环与全超边修订（上一轮）

最新证据：`/Volumes/disk0/Khor/evidence/site-story-20261113/net-loop/`。1440／390px Chrome 的四个稳定阶段共八组检查通过：无按钮、无横向溢出、区域成员正确、所有连接路径闭合、中继进入／退出活动集合；两档均自行循环回 A、离屏停止，另一幕 Terminal 控件仍可用，`errors: []`。定向 TypeScript、diff 格式检查及十页静态 Astro 构建通过，产物为 `/Volumes/disk0/Khor/artifacts/site-story-20261113-net-loop/`。

构图以 `desktop-live-viewport.png`／`mobile-live-viewport.png` 为准：正常 Chrome 播放、没有中途切换系统减少动态效果，节点与文字完整。阶段检查会切换媒体偏好以固定几何，须等偏好事件处理完成；相关分阶段截图部分有栅格缺失，不作为完整构图证明。未跑原生 GUI、WebKit、实体手机或独立设计审阅，未推送或部署。

## 超图修订（上一轮）

用户要求只保留节点，并以超边表示网络。图中可见文字仅保留网络名称；节点保留平台／Khor 标记与隐藏的 SVG 名称。用圆角凸包包围同网节点，A 内为三节点轮廓，漫游后收拢为双节点轮廓；B 从单节点环伸展为双节点轮廓；蜂窝网在中间显现。只保留当前直连、中继和候选路径，动画时序与手动控制不变。`network-geometry.ts` 共用于 Astro 静态首态和浏览器动画，无新增依赖、滤镜或持续 RAF。

定向验证：1440／390px Chrome 的 A、Cellular、B 六种组合均正确包住成员节点、无节点外部文字、无溢出和无效路径；中继承载与候选直连并行、蜂窝内直连接管、暂停通过。TypeScript 与十页静态 Astro 构建通过，未跑原生 GUI、WebKit 或实体手机。证据位于 `/Volumes/disk0/Khor/evidence/site-story-20261113/net-hypergraph/`；`desktop-a.png`、`mobile-b.png` 是完整构图，Cellular 元素截图有浏览器栅格文字缺失，仅核对图形，成员与行为读数见 `verification.json`。产物为 `/Volumes/disk0/Khor/artifacts/site-story-20261113-net-hypergraph/`。本轮只做本地预览，未发布或执行独立设计审阅。

## 节点修订（上一轮）

用户要求用节点抽象表达设备后，四种硬件外形已替换为等大圆形节点，系统标记居中；连接端点按节点半径重新计算，手机布局也保持相同节点尺寸。网络分组、漫游顺序与中继／直连时序不变。仅做桌面 1440px、窄屏 390px Chrome 定向查看：A、蜂窝、B 代表状态无横向溢出或页面异常，定向 TypeScript 与 diff 格式检查通过，未重跑完整测试、构建或独立设计审阅。最新截图及 `verification.json` 位于 `/Volumes/disk0/Khor/evidence/site-story-20261113/net-nodes/`；下方 `net-roaming` 截图属于修订前的硬件外形。

## 验证与局限

规范证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/`。

- [方向](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/direction.md)与[最终审阅](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/finish-review.md)记录用户指定的场景和局部 ship 结论；没有获批视觉稿，不作稿件还原声明。
- 四档完整场景：[1440px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/desktop-cellular.png)、[900px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/tablet-a.png)、[390px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/mobile-cellular.png)、[320px](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/small-cellular.png)。这些截图启用减少动态效果并临时隐藏导航／跳过链接；临时审阅副本已移除，以上原件为继续入口。
- [运行中的中继](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/desktop-relay-playing.png)保留信号标记与 Pause；[手机视口](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/mobile-viewport.png)保留导航及普通动态模式下暂停的控件。B 状态截图有局部栅格缺失，只支持路径／位置核对，构图以四档完整截图为准。320px 候选虚线较淡，旁边状态说明清楚；审阅认为不阻碍本地预览。
- [verification.json](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/verification.json)：四档宽度、三种位置均无页面或图内横向溢出，同档图高稳定，四种设备标签齐全，`errors: []`。八项检查覆盖中继与候选并行、蜂窝直连接管、进入 B、暂停、暂停时键盘选择、离屏停止、既有会话选择、减少动态效果。
- [验证说明](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/validation-notes.md)记录 `product-tour.ts`（含 `network-scene.ts`）定向 TypeScript 通过；[构建日志](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/site-build.log)记录 Astro 十页构建成功，产物为 `/Volumes/disk0/Khor/artifacts/site-story-20261113-net-roaming/`。[检测结果](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/detector.json)对改动 Astro 标记成功返回 `[]`，不扩展为独立样式表扫描保证。

截图不证明实际动画帧率或实体 iPhone 流畅度；本轮未验证 Safari／WebKit、真实网络、后台或原生 GUI。旧 Safari 反馈不覆盖本场景。图谱已先查询但陈旧，当前源码与定向证据优先。审阅和[文档核对](/Volumes/disk0/Khor/evidence/site-story-20261113/net-roaming/documentation-review.md)未重跑测试或构建；本轮未推送、部署或合并。
