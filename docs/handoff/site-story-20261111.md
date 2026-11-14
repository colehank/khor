# 官网本地迭代 · 2026-11-11

用户授权：先本地快速迭代 Web，可在本机运行网页构建、预览与浏览器验证；后续已明确授权网站静态产物发布 GitHub Pages，并配置 Cloudflare 的 khor.cloud 域名。该授权只作用于网站任务，不包含合并 main 或原生发布。

- Worktree：`/Volumes/disk0/Khor/worktrees/current/khor-site-story`
- Branch：`feat/site-story`；基线 `97bbf2a4e036f1a334c9921961d3d0b283f46c59`
- 预览：`http://127.0.0.1:23820`，Astro dev daemon（本轮刷新英文内容缓存后启动 PID 83220）
- 用户 main 的 handoff 改动与未跟踪 lab-preview、23950/23961 的现有 Lab 和用户 App 均保留。

## 最新增量 · 统一收尾（2026-11-14）

收尾标题改为 Build with Khor，Download / Docs、平台与 CLI 安装区直接复用首屏组件；首尾链接、复制、平台可用性保持一致，黑底适配亮色按钮。手机／宽屏验证与十页构建通过，详情见 `site-relations-20261114.md` 最后一节，仅本地。

## 最新增量 · 全英文与光流增强（2026-11-14）

用户要求光流更明显、更好看，整个 Web 改成英文。后两幕新增三层轻量描边，保持已确认方向与交接依存；导航、下载、文档、示例任务、文件与 Git 记录全部英文。GUI 入口固定英文 catalog，手机浅／深色真实首图重新截图，40% 背景透明度与设备自适应策略不变。详情见 `site-relations-20261114.md` 的最后一节；本轮仅本地，未发布。

## 当前叙事定稿（Codex 参考改版，替代下方历史顺序）

0. **Khor.** 左上是原始图标 + Khor，首屏左侧大字产品名、小字标语与下载 / 文档，右侧完整 GUI，共同落在原参数 Vanta Fog 上；GUI 为 40% 背景毛玻璃，窄屏上下排列。手机／粗指针设备展示同一真实 GUI 的完整深浅首图，桌面保留原位交互。下方五个平台按钮中 macOS / Linux 可进入安装，分别标明 App · CLI／CLI；Windows / iOS / Android 灰显，悬停、聚焦或触屏点按时提示 Coming soon；公司 kapestack 链接保留在页脚。继续下滑直接接下列图标故事，不显示额外图文介绍及 GUI 旁注。有限合成数据不变，桌面发送仍固定回复下载链接；主下载按平台与可靠架构推荐。全程不接真实 Lab、模型或设备。

1. **You have many devices.** 四个普通灰色圆节点松散分布，大小与明暗各异，分别使用 Server、Workstation（台式显示器）、Desktop（翻盖笔记本）、Mobile 图标。前两幕不预先排成图标的四个固定位置。Meet 之前不出现珠光高亮、渐变、边缘反光或珠子的投影。
2. **With many harnesses.** 节点沿各自的短弧线移动，错开将标记切换为 Claude、Codex、Cursor、Shell，继续保持灰色；每个节点的移动起止与缩放不同。
3. **But one mind.** 四个外节点先开始收拢，临近最终位置时减速、缩小并变暗，harness 标记淡出；中心节点和内圆稍后同时以固定尺寸浮现。中心从第一帧起就是原图的最终绿色和尺寸，但先是纯色圆，光泽等到 Meet。内圆保持原图绿色与几何，Meet 前也使用纯色环，不提前显现渐变、阴影或环境光晕。
4. **How about letting them work together?** 一次连续动效画出外环的内外两道细边，以及只有边缘的菱形；三条线统一细度、灰度，环带中间留空，保留最终玻璃环的占位宽度。没有菱形填充、玻璃或环境光；四个外节点灰色，中心节点和内圆绿色。绘制进度跟随滚动位置，停止滚动就停在当前形态。
5. **Meet Khor.** 原图的暗色环境铺满整个画面，左上方的光让菱形填充、玻璃外环和珠子的高光逐渐显现，四个外节点随光恢复绿色。外环和菱形不重新描线；此时标志没有方形底板边界。
6. 原来的笔记本合盖、手机接续与鼠标点击段保持移除。图标故事位于首屏之后，先给首幕桌面 .5 屏、手机 .7 屏阅读停留，再以 4.8 屏演化；段总高桌面 6.3 屏、手机 6.5 屏。字幕在图形开始演化时短时渐隐／渐现，停手后不会保留叠字；几何继续逐帧随滚动正反推进。Meet 后按用户最新方向依次说明设备组成私人 P2P 网、由你跨网访问资源／终端／智能体、由 Khor Agent 调用同一组能力完成任务，再接下载；此顺序替代下方历史三幕。导航「了解」指向图标故事，键盘跳过入口直接到 GUI。

## 本轮实现与验证

- 2026-11-14 第三幕改为与第二幕相同的 You＋笔记本＋Mobile 入口组，Task／Result 从组上方接入 Khor，结果回到组内，移除网外独立 You。两幕设备坐标与图高对齐；任务时序不变。四档 headless Chrome 几何、文字避让与返回落点检查及静态构建通过，证据 `site-story-20261114/access-group/`；仅本地。

- 2026-11-14 Continue 两台机器改为同步给 You、5 秒一起循环。Agent 的 Khor 从收到任务到结果送达用户持续轻闪，交付时恢复常亮，往返依存不变。headless Chrome 定向检查、TypeScript 与静态构建通过，证据 `site-story-20261114/busy-delivery/`；本轮仅本地。

- 2026-11-14 修正动效流向与依存：Continue 为远端设备 → You；Agent 为 You → Khor → 机器，调用到达后才点亮智能体，再机器 → Khor → You，回传到达前不交付。Agent 12 秒完整往返，保持原有 1.7 秒缓动和 1.3 秒交接，Work 仍 10 秒。手机／宽屏 headless Chrome 方向与到达依赖、暂停恢复及静态构建通过，证据 `site-story-20261114/roundtrip/`，详见 [固定关系图](site-relations-20261114.md)。本轮仅本地。

- 2026-11-14 三幕动效节奏统一：后两幕改为固定路径上的细光经过、到达后活动环轻闪，再停留；Work 交替访问、Agent 提示请求／调用／返回。设备与文字保持不动，底层连接常驻。与第一幕共用 1.7 秒缓动、10 秒周期，第一幕行为不变。手机／宽屏 headless Chrome、定向 TypeScript 与静态构建通过，证据 `site-story-20261114/shared-motion/`，详见 [固定关系图](site-relations-20261114.md)。本轮仅本地。

- 2026-11-14 两幕文字精简：Claude／Codex 只留图标，去掉重复名称及独立 Terminal 标注，保留 File／Web 与任务关系文字；原定位、遮罩、样式同步删除。手机／宽屏 headless Chrome 与定向 TypeScript 通过，证据 `site-story-20261114/clean-labels/`，本轮仅本地。

- 2026-11-14 固定关系图视觉精修：后两幕设备／角标统一中性表面层次、细边与连接接点，会话名称和资源说明拉开主次，平板补充字号；Khor 路径从设备上方接入以避开角标。固定构图和局部 opacity 脉动保留。定向 headless Chrome 与十页静态构建通过，证据 `site-story-20261114/relations-polish/`，详见 [固定关系图](site-relations-20261114.md)。本轮仅本地。

- 2026-11-14 后两幕改为固定关系图：笔记本／Mobile 是同一任务的访问组，远端常驻 Claude Code／File 和 Codex／Terminal／Web；Agent 幕的 You 在网外，通过 Task／Result 与超边上的 Khor 连接。仅细连接与活动环缓慢脉动，设备、文字与资源位置固定；旧阶段播放、移动载荷与 task-state 已删除。320／390／768／1440px headless Chrome、仅 opacity 动效、离屏暂停、减少动态效果、无 JS 与十页静态构建通过。最新继续入口为 [固定关系图](site-relations-20261114.md)，证据 `site-story-20261114/relations/`。本轮仅本地。

- 2026-11-14 用户反馈两幕太快：当前改为各 20 秒，载荷移动 1.2 秒，文件／Web 生成后停 1 秒，四句提示均至少 2.4 秒；到达后执行的依赖与图形保持。手机 Work／宽屏 Agent 完整循环及静态构建通过，证据 `site-story-20261114/task-pacing/`，本轮仅本地。

- 2026-11-14 本地两幕改为通用 Task：笔记本发对话、台式机运行 Claude 并产出 File、File 直传 Server、Codex 运行并提供 Web、手机接续访问；Khor 幕替用户发起同一流程。session 固定在执行设备，输入到达后才运行，文件／服务来源持续保留。标题及副文案同步用户确认版本，两幕各 10 秒。当前继续入口见 [Task 与远端 session](site-task-sessions-20261114.md)，证据 `site-story-20261114/task-sessions/`，本轮未发布。

- 2026-11-13 用户明确要求部署：源码 `f5c457d7` 的完整静态产物已发布为 `gh-pages` 提交 `cebdd1d`，Pages run `43444719571` 成功，<https://khor.cloud/> 版本与两档 headless Chrome 检查通过。详情见 [Pages 发布记录](site-pages-20261112.md)。下方各次「仅本地」为对应迭代时的历史状态。

- 2026-11-13 最新任务改为 **Train a vision model**，精简为四句提示：Work 中用户从 Workstation 发起 Server 训练，到笔记本修订、手机试用 Web，10.4 秒；Agent 中用户留在 KhorNet 外，超边上的 Khor 图标路由同一任务并返回模型与 Web，11 秒。保留到达依赖，去掉脚本、文件名及训练轮次的额外解释。两档完整循环、三档减少动态效果、定向 TypeScript、十页静态构建通过；证据 `model-training/`。详见 [Work 与 Khor Agent](site-work-agent-20261113.md)。以下 Atlas Web 及 Workstation 上的 Khor 等描述均为历史版本，本轮仅本地。

- 2026-11-13 到达顺序修复：终端指令在途时 Server 保留文件状态，到达后再出现终端和进度；接单、网页、报告与结果也依赖到达。后两幕只显示 KhorNet 超边与名称，去掉内部网络及后缀；Desktop 改成显示器＋主机并同步开场。定向 headless Chrome 回归先红后绿，证据 `task-arrival/`，仅本地。

- 2026-11-13 用户要求后两幕回到原 Net 布局：当前以同一个 Atlas Web 构建任务，演示 Workstation → Server 传输／执行、Mobile 接续与监控；Agent 幕由 Mobile 的 You 交任务给 Workstation 上的 Khor，再执行并交回结果。三幕共用网络组件与几何，节点不再组成能力树。Server／Workstation／Desktop／Mobile 图标统一到开场 You have many devices，harness 与最终图标演化保留。手机／宽屏完整循环、路径文字间距、暂停、开场换脸、定向 TypeScript 与十页构建通过；仅 headless Chrome，本地未发布。最新实现与证据见 [Work 与 Khor Agent](site-work-agent-20261113.md)，替代下方任务星形图。

- 2026-11-13 用户再次澄清叙事：Work across 是用户在各个机器上便携地推进具体任务，Khor Agent 是用户把任务交给 Khor。现用同一项导出日期修复作对照：Work 的 You 在 Mac／Windows／手机之间切换，亲自调用远端审阅、编辑、测试与预览；Agent 幕始终保留 You，增加明确的任务交出和结果返回路径，中间由 Khor 推进相同动作。此条替代下一条的能力目录表达。当前文档 [Work 与 Khor Agent](site-work-agent-20261113.md)，本轮仅本地。

- 2026-11-13 用户明确第二幕为 Work across the net、第三幕为 Khor Agent，并授权实现。用户进一步要求也用 graph 表达，现用共享设备—能力图替换旧会话选择与资源切换：macOS 连 Files／Web／Git／Terminal，Linux 连 Codex／Claude／Cursor；第二幕由 You 接入，第三幕换成 Khor Agent 执行订单日期修复，沿路径调用并交付变更及测试结果。桌面／手机有独立固定构图，标签避开线条；两幕自动循环、离屏／后台暂停、减少动态效果静止。前一幕 Net 与首屏、Fog、图标故事、下载保留。当前继续入口为 [Work 与 Khor Agent](site-work-agent-20261113.md)，下方旧会话／资源场景为历史版本；本轮本地预览，未发布。

- 2026-11-13 KhorNet 当前修订：LAN 为 macOS／Linux，Wi-Fi 为 Windows／Linux；iOS 从 Wi-Fi 到蜂窝再返回，不进 LAN。KhorNet 一次成形后包围全部设备；换网到位后中继再保留 1.3 秒再切直连。沿边曲线标题为图标 + 名称，直连显示 P2P，中继显示 relay／hole punching；仅 hole punching 闪烁。副标题独立弧线路径修复字母挤压，外层留白修复标签间重叠；图标与标题位置固定，边界为文字留完整缺口；蜂窝三格前二实后一空。桌面／手机宽度 Chrome 循环检查、定向 TypeScript 与静态构建通过，证据 `khornet-curved-labels/`。详见 [Net 漫游场景](site-net-roaming-20261113.md)，下方旧 Net 拓扑均为历史版本，本轮未发布。

- 2026-11-13 Net 成员与曲率补充：A 为 Linux／Windows，B 为共享 Windows／Android；iOS 取消独立圈。标题沿超边排，曲率收敛为轻微单段弧，去掉额外鼓包。Chrome 两档八组归属检查、TypeScript 与静态构建通过，证据 `net-shared/`；详见 [Net 漫游场景](site-net-roaming-20261113.md)。
- 2026-11-13 Net 自循环补充：移除底部四个控件与说明，Wi-Fi 改为 Network，错位布局与弧线漫游；中继／直连统一为闭合超边，20.4 秒自动循环。桌面／窄屏检查、正常播放构图、定向 TypeScript 与静态构建通过；证据 `net-loop/`，详见 [Net 漫游场景](site-net-roaming-20261113.md)。
- 2026-11-13 Net 超图补充：移除节点外部标题，闭合超边表示网络成员并随漫游伸缩；移除常驻两两连线。桌面／窄屏六种成员组合、直连接管和暂停检查、定向 TypeScript 与静态构建通过，证据见 `net-hypergraph/`；详见 [Net 漫游场景](site-net-roaming-20261113.md)。
- 2026-11-13 Net 外形补充：按用户要求改为等大的系统标记圆形节点，重新对齐连线路径；桌面／窄屏 Chrome 查看与定向 TypeScript 通过，证据见 `net-nodes/`，详见 [Net 漫游场景](site-net-roaming-20261113.md)。
- 2026-11-13 按用户指定方案仅重做第一幕 Net：Linux／macOS 在 Wi-Fi A，Windows 在 Wi-Fi B，iPhone 在 A／蜂窝／B 移动；中继承载流量与尝试直连并行，成功后直连接管，19 秒示意循环。1440／900／390／320 Chrome 无溢出或错误，八项行为检查、定向 TypeScript 与十页 Astro 构建通过；局部审阅为可交用户本地预览的 ship，无实质修正。入口见 [Net 漫游场景](site-net-roaming-20261113.md)，替代下一条的第一幕；第二、三幕未改，仍待用户方向，此前整组 critique 不作已解决。B 状态截图仅用于位置／路径，构图以四档完整截图为准。本轮未推送、发布或合并。

- 2026-11-13 用户提出设备 → 网络、任务 → 跨网会话、资源 → 会话内使用，并要求「做出来看看」。现用同一个虚构 Atlas Web 项目串起三幕：直连／中继设备网、保留主机与内容的会话访问端切换、延续当前选中会话的网页／文件差异／工作区。会话主机与资源来源分别标明，Codex 在 build-01、资源来自 MacBook Pro。保留手动选择与独立 Pause／Play 循环，离屏／后台／减少动态效果停止；首屏、Fog、图标故事、导航与下载保持不变。最终 1440／900／390／320 Chrome 记录无溢出或错误，九项行为检查、脚本 TypeScript 与十页 Astro 静态构建通过；最终有界检测为空，局部审阅结论为 ship、无实质修正。当前入口为 [网络、会话与资源三幕](site-net-sessions-resources-20261113.md)，下方三项亮点精修为历史版本；旧 Safari 反馈不覆盖新三幕，本轮未推送、发布或合并。

- 2026-11-13 用户接受成熟度审阅方案并要求后续动画循环。三图现延续黑场，以 Atlas Web 的可读文件／网页、明确 Agent 请求／双任务／双结果及不变的终端输出建立内容；浅色只承载文档与结果，手机保留任务，320px 堆叠工作节点。P2P 信号循环、Agent 3.8 秒流程加 3.6 秒结果停留、Terminal 每 4.6 秒切换访问端，各图可 Pause／Play，离屏／页面隐藏／减少动态效果停止。1440／900／390／320 定向 Chrome、阶段高度稳定、控制与循环、终端缓冲不变、键盘及减少动态效果检查通过，无错误；网站 TypeScript 与 Astro 静态构建通过。新鲜审阅结论为范围内 ship；检测器空结果早于最后的标题／状态／高度预留细调，不作最终全量扫描证明。当前继续入口见 [三项亮点精修](site-maturity-20261113.md)，替代下方连线版与有限播放约定；本轮未发布、推送或合并。

- 2026-11-13 用户要求优化布局，尤其是连线交叠。P2P 收成两端点／单通道；Agent 改为左到右的双分支分流与结果汇集；Terminal 使用共享主干及两条分支。标签与路径分开，不再用文字底条盖线。设备／资源选择及有限演示保留，首屏与原故事不动。继续入口见 [连线整理](site-routing-20261113.md)，仅本地。

- 2026-11-13 用户否定大幅线框概念图，要求参考 iroh 官网。当前三段采用较轻标题与紧凑图文并排、局部浅色画布、HTML 标签和 SVG 弧线；P2P 切换文件／网页资源，Agent 重播委派与回收，Terminal 切换访问设备。没有恢复 GUI 截图或新增实时渲染器；原首屏、故事、导航和下载保留。最新继续入口见 [iroh 参考重做](site-iroh-20261113.md)，仅本地。

- 2026-11-13 用户要求取消短句尾部句号，并将 Meet 后的 GUI 截图改为概念图。已替换为三个独立的响应式矢量构图：资源直连、智能体委派／汇集、持续运行的终端与访问端变化；保留首屏真实 GUI 与五步图标故事，原截图及专用捕获场景已移除。当前入口见 [概念图记录](site-concepts-20261113.md)，本轮仍仅本地。

- 2026-11-13 用户否定上一版三段，讨论后接受以 P2P、Khor Agent、Terminal 重做。现分别展示跨设备访问同一 localhost 网页、Khor 委派两台设备上的 Codex／Claude 并回收结果、同一真实终端在 Desktop／Phone 的静态视角切换；原首屏与 Meet 动画保留，手机不加载 App。12 张新图替换旧 19 张，默认演示消息／下载回归正常。39 项定向 Chrome、两组 TypeScript、演示与 Astro 构建通过，源码检测为零项。细节见 [三项亮点重做](site-pillars-20261113.md)，本轮未发布。
- 2026-11-13 用户授权 Meet Khor 后新增连接、Sessions、工作区介绍，取代此前完全不设功能介绍段的范围。三段以简化关系构图和真实 GUI 局部串起同一个订单导出修复；桌面文档／双栏 Git 差异叠放，手机使用可读近景和真实窄屏单列差异。没有增加交互 iframe 或 WebGL，恢复自然滚动，主题随浏览器、减少动态效果可完整静态阅读。原首页、图标故事与下载路径保留。实现、捕获来源和定向验收见 [三段产品介绍](site-tour-20261113.md)。本轮仅本地，预览 `http://127.0.0.1:23820/#connect`，未发布。
- 2026-11-13 用户确认「1 按你推荐；2 全部精修」：手机改为完整真实首图，桌面继续交互；清理 DEV 隔离入口，保留网页缩放。选中任务压成一次完整请求与明确结果，深浅首图同步；字幕改短时淡入淡出以消除暂停叠字，手机总段长压至 6.5 屏且保留首幕 0.7 屏；首屏字重／断行、44×44 导航命中区、平台 App／CLI 注释、触屏 Coming soon、完整 CLI 折行和页尾按钮信息组均完成。37 项集中 Chrome 检查、两组 TypeScript、演示及 Astro 构建通过，无页面错误；有界检测为零项。本轮未发布，普通 URL 的 Safari 真机已获用户「可以正常放大和滚动」反馈；底层完整交互 GUI 的 Safari 缩放根因尚未确定。细节见 [Safari 排查及本轮精修](site-zoom-20261113.md)，证据 `/Volumes/disk0/Khor/evidence/site-story-20261113/polish`。
- 2026-11-13 用户要求先本地修手机导航，并明确滚动后用黑色毛玻璃。本轮未发布：手机导航从与按钮相同的 44 高增至 68，按钮仍保留 44 触摸区域、上下各留 12，并计入系统顶部安全区。左上原始图标由 24 放大到 44，与导航内容区及下载按钮等高。首屏顶部保持通透与深色字，开始滚动即切为 72% 黑底、24 模糊与浅色字，滑回顶部恢复；不再等离开整个首屏才显现背景。Chrome 定向核对手机初始 / 下滑 / 回顶 / 黑场与桌面初始，修前留白为 0 且下滑仍无模糊，修后各状态正确、无溢出或页面错误；fog 脚本 TypeScript 与 diff 检查通过。证据 `/Volumes/disk0/Khor/evidence/site-story-20261113/mobile-header`，不扩大浏览器或原生构建范围。
- 2026-11-12 用户新增 GitHub Pages 发布授权，并询问通过现有 Cloudflare / cloudflared 解析 khor.cloud。已新增静态基路径支持和官网安装器输出，具体发布状态以 [Pages 发布记录](site-pages-20261112.md) 为准；先前“仅本地、无发布授权”的限制在此网站发布范围内被最新指令替代。
- 2026-11-12 用户要求宽屏安装组为一行：1200px 起依次横排 Other ways to get Khor、平台图标、or Khor CLI、安装命令；窄屏分行。命令、复制反馈和平台可用性不变。
- 2026-11-12 CLI 入口验收：Chrome 检查复制值为完整 INSTALL、命令不被图标反馈覆盖、勾显示后自动复原、成功文字仅在读屏状态中、Copy SVG ID 无冲突；下载页共用控件正常，手机图标常显，390 / 320 宽无页面横向溢出。网站复制脚本类型检查与 Astro 静态构建通过，未重跑 GUI / 原生构建。
- 2026-11-12 最新首屏文案：“Work across your devices and agents.” 后手动换一行，接 “Access agents, edit files, track changes, and connect to web apps from any device, peer to peer.”。平台组增加 Other ways to get Khor；用户后续决定 Coming soon 仍悬停 / 聚焦显示，不常驻。图标下方增加 or Khor CLI 与 INSTALL 命令，复制图标桌面悬停显示、触屏常显，点击后短暂换成勾。首屏、页尾和下载页共用 CopyButton，成功无可见文字反馈，命令不被覆盖；证据 `evidence/site-story-20261112/cli-install`（绝对前缀 `/Volumes/disk0/Khor`）。
- 2026-11-12 Coming soon 遮挡修复：手机 Chrome 同尺寸复现提示上半部落入 GUI，命中探针返回 iframe；平台行现建立 z-index:3 的独立层，高于 z-index:2 的 GUI。提示位置和交互保持原样，手机同一重叠点验证提示完整位于上方。证据 `evidence/site-story-20261112/platform-tooltip`（绝对前缀 `/Volumes/disk0/Khor`）。
- 2026-11-12 用户随后要求放回完整文案，但不必换行：现将 “Work across your devices and agents. Edit files, track changes, and use your web apps. From any device, peer to peer. Efficient, secure, and delightful.” 合为一段小字，无 br 或分段，仅按可用宽度自然折行。此条替代下一条删减。
- 2026-11-12 用户进一步要求删除 “Edit…” 及其后的文案。首屏小字仅保留 “Work across your devices and agents.”，大字 Khor、按钮、GUI 和平台入口保持现有布局。此条替代下方两段英文文案记录。
- 2026-11-12 首屏布局与文案：用户要求左上恢复原 icon + Khor，正文只用大字 Khor、小字介绍、下载 / 文档；右侧 GUI，下方五个平台图标入口。当前文案采用 “Work across your devices and agents. Edit files, track changes, and use your web apps.” 与 “From any device, peer to peer. Efficient, secure, and delightful.”。下载箭头移除，首屏按钮固定短标签而目标继续自适应。macOS / Linux 已有资产可进入安装选项，Windows / iOS / Android 灰显并显示 Coming soon；图标来自实际 App。公司 kapestack 链接保留在页脚。40% 玻璃、Fog 参数和下方故事未改。证据 `hero-platforms`；此条覆盖早先 kapestack 顶栏和大号标语布局。
- 2026-11-12 最新首屏修订：标题 “Work across your devices and agents.”；去掉按钮下方推荐版本提示；窗口底色从 50% 降到 40%，深浅首图同步。用户取消 GUI 放大，桌面与手机都只保留原位交互。固定下载回复曾被 `[data-khor-agent-turn] button` 误禁用，已单独放行；其余复杂操作照旧受限。iframe 共享同一 Fog 画面，原玻璃控件不再重复叠黑。Chrome 定向验收与实现说明见 [材质与下载修订](site-material-20261112.md)。此条替代后续旧放大和 50% 描述。
- 2026-11-12 用户要求文字渐变切换：将字幕裁切位移替换为固定位置的 opacity 互补渐隐渐现。前 8% 交接、图形同步起动、总行程与首幕停留均不变。减少动态效果仍完整显示静态文案。额外的 Git 中文说明按用户要求删除。此条替代下述裁切带方式与补充文案。
- 2026-11-12 全页收整：文字与图形从段首并行起动，字幕前 8% 以裁切带交接，图形前 86% 演化，末 14% 留完整姿态；替代下一条「先换字再起动」方案。保留首幕 .5 / .7 屏阅读与正常总行程。首页品牌黑场独立于 GUI 的浏览器主题；手机先用真实首图，点查看后载入同一个可操作 iframe，以原生字号展开，底部横条查看完整桌面，关闭 / Escape / 切换主题不丢草稿。减少动态效果改为紧凑静态故事。统一三页导航和自适应下载，App / CLI 分组并补齐经实际 GUI 标签核对的上手文档。Chrome 与静态构建记录见 [整页收整](site-refinement-20261112.md)。以下记录按时间保留，冲突以本条为准。
- 2026-11-12 文案时序修复：用户手机截图显示连接线已绘制而 “How about” 尚未出现。Chrome 在同一段 45% 位置复现：轮廓约完成 70%，下一标题 opacity=0，旧标题仅约 .048。`iconSegments` 现明确每段前 12% 换字、14% 起动图形；标题完成后保持到段末，互补淡入避免中段空白。ARIA / 分页跟随文案，Meet 背景随标题提前到达，浅色模式白字可读。原生采样显式加入两个分界，兼容模式共用 `copyPhase`。首幕 .5 / .7 屏阅读保留。Chrome 手机深色 / 桌面浅色 / 强制后备检查四次演化中文字先到、整段保持、图形未提前、倒滚和无页面错误，类型检查及静态构建通过。红绿截图与读数 `/Volumes/disk0/Khor/evidence/site-story-20261112/caption-timing`。
- 2026-11-12 用户认为动画首幕应停留更久，要求参考成熟网站。核对 Apple AirPods Pro 页面原始滚动配置与 CSS，以及 GSAP ScrollTrigger 官方 FAQ 后，为首幕单独加入桌面 .5 屏 / 手机 .7 屏的阅读区间；这两个值是 Khor 本轮设计选择，并非 Apple 的固定标准。阅读时四台设备与标题维持首姿态，底部细线填充反馈滚动，随后才进入原有 4.8 / 6.2 屏演化，不挤压后续阶段。所有轨道仍共用一个原生时间轴，后备也支持负进度阅读区间；首章节书签回阅读起点，后续书签计入新增停留。Chrome 桌面 / 手机尺寸 / 强制后备均检查阅读区间、滚动反馈、演化衔接、倒滚与后续章节点，类型检查与静态构建通过。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/scroll-hold`。
- 2026-11-12 用户确认「做」后完成滚动重构：原始 SVG 拆为少量静态片段，由一条原生 WAAPI ScrollTimeline 驱动位置 / 透明度；文字取消提前半段切完，桌面 / 手机行程为 4.8 / 6.2 屏，CSS 与 JS 共用稳定 sticky 几何。移除逐帧去饱和 / 亮度滤镜和 SVG 移动遮罩，Meet 用静态羽化裁切及相反 HTML 位移揭示完整原图；灰节点加不透明底面避免轮廓透出。旧浏览器有缓存属性的可见范围 RAF，减少动态效果按章节静态显示。Chrome 已检查桌面、手机尺寸、强制后备、减少动态效果的五个姿态及倒滚；完整原图对照截图平均 RGB 差低于 1/255，未重画图标。121 帧普通采样属性变化 15 次，CPU4 诊断为 12 次（旧版 5260 次）；桌面模拟不作为真机 FPS 结论。类型检查与 Astro 构建通过；证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/scroll-timeline`。完整实现记录见 [滚动调研](site-scroll-research-20261112.md)。
- 2026-11-12 用户要求延长首屏渐变，并先调研后续动画。渐变从 120px 延长到至少 40svh。调研见 [site-scroll-research-20261112.md](site-scroll-research-20261112.md)，记录 2.6 屏过短、文案提前半段切完、逐帧约 44 次属性更新与稳定视口问题；建议单一原生滚动时间轴、分层材质、移动端更长行程。动画方案尚未实施，桌面 Chrome 模拟不作为真机 FPS 结论。
- 2026-11-12 桌面 WebKit 窄屏锁存修复：1050 视口下复现 root `data-narrow=true`，即使 iframe 的布局 / 媒体查询已恢复 1360，React 仍保留初始窄屏状态，窗口拉宽也不恢复。移除全部布局 zoom 与事后模式切换，从首次载入起只用 transform 呈现固定桌面视口。WebKit / Chrome 各核对 1050→1440→1050，所有根布局为 1360，narrow=false，固定回复正常。另按用户要求移除 “Khor Desktop / 试着发条消息”、系统 / harness 标识及三组图文，DOM 只剩首屏→故事→下载；导航「功能」改为「了解」到故事。截图与数据 `glass-hero/desktop-viewport-*-after.png`、同名 JSON。
- 2026-11-12 按用户要求把玻璃背景从 70% 调到 50%，前景 opacity 仍为 1，深浅首图 alpha 更新为 127/255。修复系统按钮被裁：原外框固定圆角 14.4px，在小预览上过大；将缩放变量提升到窗口共同祖先，让圆角按原生 24px 与 GUI 同比缩放。WebKit 手机视口下原圆角裁掉 17 个按钮轮廓采样点，修后圆角约 6.19px，裁切点为 0；桌面也为 0，窗口没有白色外描边或内描边。对照数据 `glass-hero/controls.json`，最新手机和深浅截图同目录。GUI 打包与脚本类型检查通过。
- 2026-11-12 最新首屏恢复左介绍 / 右 GUI；Khor 图标与产品名横排，左右共同落在原参数 Fog 中。网站 hero 只设置生产 `data-window-transparent` 和 `--window-opacity:70%`，复用实际单背景面；前景 opacity=1，父窗口 blur(24px) 对 Fog 取样。不伪造原生运行时或打开原生后端。深浅 WebP 改为 RGBA，背景 alpha 178、文字最高 255，首图与实际 App 都可透出 Fog。Chrome 已确认左右位置、玻璃属性和固定回复，WebKit 手机布局仍为 1360；深浅与手机截图在 `/Volumes/disk0/Khor/evidence/site-story-20261112/glass-hero`。
- 2026-11-12 iPhone 预览白块修复：用本机已有 WebKit 2336、390×844、iPhone UA / 触屏复现。外层 CSS zoom 让 iframe 的实际 `documentElement.clientWidth` 从 1360 变成 356（`innerWidth` 仍说 1360），App 切成窄屏，之后又被外层缩小，剩余区域为白块。触屏 / 窄屏改用 transform，鼠标桌面仍用 zoom；ready 前检查真实布局宽度，不一致时回退，并等两帧再交出首图。修后 WebKit 与 Chrome 的根布局 / body 均为 1360×850，根 `data-narrow` 为 false，预览外宽 356，完整三栏铺满，固定回复可发送。类型检查与静态构建通过；只做此问题定向检查，未下载浏览器。红绿数据、截图在 `/Volumes/disk0/Khor/evidence/site-story-20261112/phone-preview`。注意附栏内部有合法的 `data-narrow-box`，断言应看根，不能查任意后代的 `data-narrow`。
- 2026-11-12 Codex 参考改版已获「先做」授权并本地实现：kapestack 导航、原参数 Fog 居中首屏、大幅 GUI、支持标识、三组交替真实 GUI 配图、短图标故事、下载收尾。完整方向与依赖说明见 `site-page-direction-20261112.md`。Fog 两个分发文件与许可证本地固定；减少动态效果使用本页画布的干净帧，没有外站文字。Chrome 已检查 WebGL、消息回复、链接、完整 Meet、明暗与窄屏，错误与外部请求为空；证据 `evidence/site-story-20261112/codex-layout`（绝对前缀 `/Volumes/disk0/Khor`）。旧的左字右 GUI、九屏顺序均为历史版本。
- 2026-11-12 图表与头像精修：六台机器分别使用浅绿、深靛、珊瑚、金黄、紫和深绿头像；使用原生几何及出厂色板的有限示例记录，不改变真实 App 的画笔或 token。额度主项扩至六个 Claude 模型、30 天记录，日汇总直接加总逐模型的四类 token，堆叠趋势、散点、统计表同源。MacBook Pro 补成 44 个进程（约 12.62 GiB 小于当前 14.2 GiB 总占用；CPU 总计约 215.8%，折算 12 核为 18%），日趋势有工作节律与短编译峰，内存更平稳、磁盘缓慢增长，最后一个桶与仪表同值。Chrome 确认六张不同脸、六个可见模型点、汇总一致且无页面错误；截图和读数在 `/Volumes/disk0/Khor/evidence/site-story-20261112/telemetry`。深浅首图同步更新。
- 下一步页面方向待讨论，尚未改首页布局与文案：全面参考 OpenAI Codex 页，使用用户指定 Vanta Fog 原参数；左上角为 `kapestack`，链接 `https://www.kapestack.top`，Khor 是产品身份。讨论草案与完整参数见 `site-page-direction-20261112.md`。
- 2026-11-12 用户进一步纠正「展示 Khor GUI，而不是 Claude」：此前首项虽在 Khor App 外壳里，实际仍为 Claude ACP 会话。已将首项改为 `wide/all` 内置 Khor，会话用真实 `MachineChat`、`KhorHeader` 和 `KhorAgentResult` 渲染本地命令回执，模型为 GPT-5.4；其他四项保留 Cursor、Codex、Claude、Shell 类型。三标签、右侧 Markdown、固定下载回复与不灰化的限制继续保持，首图同步更新。
- 导航清晰度修订：移除整个 iframe 外层的 transform scale，改为 CSS zoom 参与布局，避免玻璃与 SVG 图层先合成后缩小。Chrome 已对比静止、按下、松开三态；实际鼠标按屏幕坐标切到机器页再返回成功，Khor 内置会话有两轮记录，发送后得到固定下载回复，页面错误为空。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/khor-gui`；没有扩大浏览器测试矩阵。
- 2026-11-12 最新范围：保留真实 Khor GUI，用户已撤回 Claude Code TUI 要求。机器、会话、额度和工作区各只开放第一项；会话、资源侧栏和工作区显示三个标签，只保留当前选中项可用；「…」及复杂操作不开放。不可点项保持正常颜色和不透明度，不做灰化。主会话仍可发送消息并获得固定下载回复。
- 网站数据收拢为 `world.ts` 的有限自洽世界，列表、对话、项目路径、文件内容与 Git 修改数引用同一来源。主任务为订单导出日期与列名修复，三个未提交文件、8 项示例测试结果与 Markdown 记录一致；其余列表展示报销校验、支付回调、周会纪要等普通任务。第一台 MacBook Pro 补齐 CPU/GPU/内存/磁盘仪表、日趋势和进程；额度扩至六个正常账户名。右侧和首个工作区直接展示包含表格、代码、清单的富 Markdown；网站 FileView alias 仅改变默认阅读姿态，生产 App 源码不改。
- Chrome 聚焦检查已确认四类列表仅第一项可点、三标签固定当前、所有不可点项 opacity=1、首个工作区的 Markdown 与固定下载回复正常，页面错误为空。演示类型检查、演示打包与 Astro 静态构建通过，首图按新场景更新深浅两张 WebP（合计约 508KB）。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/coherent-demo/focused.json` 及同目录截图；未扩展 WebKit 或原生验证。
- 2026-11-12 点击失效修复：先用浏览器保存的 `site-v3/hero` + `khor.place=devices` 复现，旧逻辑等待聊天输入框和资源列才解除首图，导致 pointer-events 一直为 none。改为生产 App 任意页面挂载即发 ready，父页先注册事件再设置 iframe URL，并以同源 load DOM 兜底，首图不接收指针。Chrome 同一恢复入口现为 pointer-events auto，真实鼠标依次切换设备→会话、发送固定下载回复、再切设备成功，错误为空。顶部导航与当前 App 源码的 LandingStrip/样式一致，没有单独覆盖选中样式；已核对选中高亮随当前页变化、颜色与生产 rail-on-ink token 相同。红/绿收据和截图在 `/Volumes/disk0/Khor/evidence/site-story-20261112/interaction`。
- 2026-11-12 最新修订：Desktop 外层补上 macOS 圆角边框与阴影。用户指出真实 Khor 没有额外标题行，红黄绿按钮已改为占原有导航第一行的左侧留白，与导航图标及右侧标签同高；网站演示独立绘制，不启用原生后端。移除「打开体验」链接，保留直接输入发送。网站演示扩至六台设备，系统包含 macOS / Windows / Linux / iOS / Android；会话类别包含 Claude / Cursor / Codex / Khor / Shell，字形由实际 App 渲染。设备连接状态为合成的在线示例；其他 Lab 的默认连接状态和类别不变。Chrome 已确认五种系统及会话类型，Khor 会话可发送并得到固定下载回复，无页面错误或实际 API 网络请求。新证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261112/macos-frame`。
- GUI 主题按用户新要求跟随访问浏览器：默认 auto，不再把初始明暗写死进 iframe URL；网站 fixture 启动时同步覆盖旧演示存下的主题，资源体验链接也移除强制浅色参数。生产 App 的主题监听负责实时切换，不重新加载 iframe。首图提供深浅两张 WebP，由 picture media 选择。Chrome 已从浅→深→浅验证背景和主题同步、草稿保留；证据同目录 `theme.json`。红黄绿与原有导航行中心均为源视口 y=26，导航起点 x=80，按钮末端 x=60，无额外标题行。
- 最新版首屏改为左侧 Khor、宣传语、设备下载按钮与文档，右侧完整 Desktop。`product.ts` 按共享 list-w 的四倍固定桌面视口、等比缩放，不因预览宽度切成手机布局；加载前使用 1360×850、2× WebP（约 143KB）作静态首图。已删除 Mobile 图与 CLI 展示。
- 网站专用 bootstrap 在浏览器内初始化资源标签和示例 README，展开资源栏，继续使用生产 App；不接真实 Lab。示例消息正确结束初始轮次，新消息独立回复。开发 fixture 仅新增可选的回复起始/分块间隔，其他 Lab 仍沿用原有默认值。
- Chrome 已确认完整三栏、发送与固定回复、回复下载链接到达当前设备对应资产；测试拦截了下载导航，没有下载实际安装包。页面错误与实际 `/api/` 网络请求均为空；网站演示与动效/下载脚本的类型检查、Astro 静态构建通过。截图及收据：`/Volumes/disk0/Khor/evidence/site-story-20261111/desktop-hero`。下方三界面组合记录为此前版本。
- 产品首屏新增 `ProductHero.astro` 与两张 2× WebP（合计约 168KB），Desktop / Mobile 截自同一网站专用 App 演示场景。演示 fixture 新增可选设备名和会话预览字段，只在网站场景覆盖；其他 Lab 的默认内容不变。CLI `khor sessions` 与 `khor git status` 的格式已对照 `crates/commands/src/lib.rs`；未连接真实设备或执行真实任务。
- 新首屏使用单一产品 H1，原图标故事改为 H2，`#story` 为了解入口；滚动测量同时观察首屏尺寸，避免新布局影响原有故事偏移。桌面 Chrome 1440×1000 首屏为一屏，图片成功加载、无 iframe、无页面错误或横向溢出；390px Chrome 检查窄屏与了解链接。演示前端类型检查与静态构建通过。证据：`/Volumes/disk0/Khor/evidence/site-story-20261111/product-hero`，原始前端截图：同父目录 `product-capture`。
- 最新修正：用户反馈每幕开头需要滑动一段才有动作。已删除硬停留区间以及图形转场前 18% 的空档，改为在 `0 / 2 / 4.1 / 6.2 / 9` 屏的五个完整姿态之间连续推进；保留九屏总行程，以较慢演化维持阅读距离。
- One mind 收拢从转场起点立即开始，保留后半程中心与内圆浮现的先后关系；Meet 环境明暗加入从第一段滚动就生效的变化，后续仍为原来的左上扫光。章节按钮落在五个完整姿态，停止与倒滑行为不变。
- Chrome 在四次转场起点分别前进 80px，均已观察到响应：首节点开始移动、节点开始归拢、外环 dashoffset 从 1 到 .962、Meet 环境明暗从 0 到 .023。停止后姿态不变，倒滑回到同一中间态；动效 TypeScript 与 diff 检查通过。本轮仅 Chrome，证据 `/Volumes/disk0/Khor/evidence/site-story-20261111/immediate-scroll`。下面的固定停留区间与 18% 延迟记录均属于此前版本，不再作为当前行为。
- 开场精修：How about 保留原文，排为「How about / letting them / work together?」三行，取消单独缩小字号；五幕共用同一标题字号。文字交接缩至同一阅读位置的小幅位移，减少两个大标题同时出现。
- 桌面图形从 `min(56vw,80svh)` 收至 `min(52vw,74svh)`，给三行标题与左侧节点明确间距；原图内部比例不变。文字容器保留明确最小高度，避免此前 Chrome 漏绘。
- 内圆增加从原图实色 stop 提取的平面环，Meet 前隐藏全部原始环材质，扫光后才恢复渐变、阴影和边缘反光；五颗节点仍是同样的纯色 → 原始材质流程。
- One mind 转场里，节点在 12%–88% 区间减速归位，中心与内圆在 42%–96% 区间浮现，尺寸不增长。整幕背景的非底色光照压至原始 42%，标志本身的标准材质保留；滚动提示在首幕交接后淡出。
- 本次只做 Chrome 观感查看与动效 TypeScript、diff 检查。七个采样位置无页面错误；五幕字号在 1440px 窗口均为 77.76px，Meet 前内圆材质透明度为 0，归位途中中心仅约 0.107，而四个外节点已接近最终形态。截图与记录：`/Volumes/disk0/Khor/evidence/site-story-20261111/opening-polish`。下半部产品展示待开场定稿后另做，Desktop / Mobile 体验未恢复。以下记录为此前迭代。
- 最新观感修订：Devices 使用不等大的散布节点；Harnesses 以四条独立弧线和不同的标记切换区间打破同步；One mind 才统一归位。运动仍随滚动进度，停留区内不自行漂浮。最终 focus=1 时明确移除外围节点 transform，保持标准图稿几何。
- 本次只在 Chrome 查看首幕、错开切换、Harnesses、One mind 和 Meet；One mind / Meet 的四个外节点 transform 均为 none，TypeScript 与 diff 格式检查通过。截图：`/Volumes/disk0/Khor/evidence/site-story-20261111/scattered-nodes`。下面的滚轮节奏数据属于上一轮距离调整，本次未重跑。
- Chrome 目视还发现零高度的绝对定位文字容器会漏绘 With 一句中的 many；给 copy 明确的最小高度后，同一截图完整显示。修前 `harnesses.png`、修后 `harnesses-text.png` 保留在上述证据目录。
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
