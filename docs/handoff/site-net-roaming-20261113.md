# Net 漫游场景 · 2026-11-13

仅替代[网络、会话与资源三幕](site-net-sessions-resources-20261113.md)中的第一幕 Net，位于 `feat/site-story` 本地工作区。用户持续调整设备分布、超边、漫游范围和边界标题；以下当前行为覆盖下方历史记录。旧独立审阅只覆盖其当时版本，不视为本次修改的审阅或用户审美验收。

本篇只记录 Net。用户随后确认第二幕 Work across the net、第三幕 Khor Agent，其实现与验证见 [Work 与 Khor Agent](site-work-agent-20261113.md)；此前 critique 与旧审阅结论不视为新两幕的验收。设计权威仍为[设计系统](../design-system/README.md)与既有 token，全局设计文档及配置未改，未创建 `PRODUCT.md`。首屏 GUI／手机首图、Fog、导航与下载保持不变；图标故事的四枚设备标记按最新要求与本幕统一。

## 场景与行为（当前）

- NetA / LAN = {Workstation, Server}，NetB / Wi-Fi = {Desktop, Server}，NetC / Cellular = {Mobile}。Server 为共享节点，Desktop 在左、Workstation 在右；Mobile 从 Wi-Fi 开始，移动到 Cellular 再回到 Wi-Fi，不进入 LAN。内部几何键保留原名，图形换成机架／紧凑主机／屏幕／手机四枚角色标记；开场 You have many devices 同步复用。
- 原生网络先出现，KhorNet 在 2 秒内一次成形，始终包围四台设备。初次建立后以 10 秒周期 Wi-Fi／Cellular 往返，每次移动 1.7 秒，到位后保持中继 1.3 秒再切直连，并停留 2 秒；不重复播放初次建网，也不拆掉 KhorNet。蜂窝超边淡出时 Mobile 加入 Wi-Fi。
- 大弧度闭合超边来自平滑支撑函数，保持凸曲率，没有节点鼓包或反折；外层与原生网络留出两行弧形标题的间距。没有点对点路径、额外 Khor 设备节点或底部控件。
- LAN、Wi-Fi、Cellular、KhorNet 使用图标 + 名称，文字以 `textPath` 沿参考边界弧线排字；每档宽度的图标位置、标题位置与曲率固定，漫游时只改变超边。副标题使用独立内侧路径按真实弧长排字，不用 `dy` 压入内侧。统一 SVG mask 只裁线条、保留淡底，在图标和标题处留完整线条缺口；所有标题按当前字宽前后各留 8 个 SVG 单位，P2P／relay 切换时只更新缺口长度，文字不再靠黑色粗描边盖住超边；生产图标保持正向。蜂窝图标为三格、前两格实心而最后一格空心。Wi-Fi／蜂窝字形补入现有 `icons.tsx` 并由网站静态渲染，品牌与系统标记继续共用真实 App 来源。
- 直连示意显示 `KhorNet P2P`；建立及换网时显示 `KhorNet relay`，副标题 `hole punching`。仅 `hole punching` 轻闪，Khor 图标、名称、`P2P` 与 `relay` 均不闪；恢复直连即隐藏副标题。中继承载与打洞尝试并行的语义依据 [iroh 网络迁移说明](https://www.iroh.computer/blog/healing-connections)，只是示例，不承诺每次换网都用中继或一定打洞成功。
- 可见且前台时运行；离屏和页面隐藏暂停，减少动态效果时显示完整静态 KhorNet，无 JS 时也保留完整关系。仅几何变化期间使用有界 RAF；无新依赖、渲染器或真实网络连接。

入口：<http://127.0.0.1:23820/#connect>。源码位于 `apps/site/src`：`components/concepts/NetworkDiagram.astro`、共用 `components/concepts/NetworkGraph.astro`／`scripts/network-map.ts`、`scripts/network-scene.ts`、`scripts/network-geometry.ts`、`styles/network-scene.css`；`scripts/product-tour.ts` 接入场景，`components/ProductTour.astro` 调整首幕布局，`styles/product-tour.css` 移除旧网络样式。

## 响应式与浏览器偏好

- 宽屏限制图形宽度并参考视口高度，避免网络图撑满长屏；短横屏保留最小阅读尺寸，不把图硬压进一屏。
- 平板将标题和说明上下排；手机保留竖向网络构图、裁掉 SVG 顶底冗余空白，并放大网络标签。窄屏 KhorNet 标签位于上侧，与 Wi-Fi 标签错开；图形漫游时仍保持位置不变。
- 用户明确要求后续使用 **headless Chrome**，不再在桌面打开测试窗口。只做网页定向验证，不扩大原生／WebKit 测试范围。

## 角色图标与共用图形（最新）

后两幕现直接复用同一网络图形，仅叠加任务载荷、进度与操作者。Net 原有拓扑、布局、漫游与连接切换时序保留。当前构图及新增图标的开场／三幕验证见 [Work 与 Khor Agent](site-work-agent-20261113.md)，证据位于 `network-task/`；下方保留先前网络几何与响应式验证。

## 网络几何定向验证

证据目录：`/Volumes/disk0/Khor/evidence/site-story-20261113/khornet-curved-labels/`。1440／390／320px Chrome 完整漫游采样确认：KhorNet 成形后不重复消失且包围四台设备；标题和图标位置在漫游中不变，文字路径足够长、无标签间字形包围盒重叠、无横向溢出；图标／P2P／relay 常亮，仅 hole punching 闪烁；到位后仍为 relay，随后切为 P2P。`desktop-checks.json`、`mobile-checks.json` 与 `small-checks.json` 为数值记录，`desktop-viewport.png`／`mobile-viewport.png` 为正常播放截图。蜂窝图标为当前三格版本。`responsive.json` 记录 320／390／768／844 横屏／1024／1440／1920 七档的尺寸、无横向溢出、标题字号与等距缺口；`responsive-*.png` 为各档完整视口。最后响应式与循环验证均为 headless Chrome。

定向 TypeScript、diff 格式检查及十页 Astro 静态构建通过，最终产物：`/Volumes/disk0/Khor/artifacts/site-story-20261113-khornet-curved-labels/`。图谱先查，但其记录陈旧，源文件与本轮定向验证优先。没有扩大到 GUI 原生构建、WebKit 或实体手机验证；本轮仅本地，未发布。

## 共享节点、沿边标题与曲率修订（历史版本）

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
