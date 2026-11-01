# 下一轮任务：浏览器加载、背景、访问失败与手机布局

日期：2026-10-28。状态：**proposed / 使用反馈已收录，尚未授权实现**。
调查基线：main `e7c335c8dd81de091bd12b92cd704a63f25d03f3`。所有者：主智能体。
本专项承接用户“这些都放入下一轮”后的浏览器反馈，与 [SESSION-CONTENT](SESSION-CONTENT.md)、[SESSION-CONTINUITY](SESSION-CONTINUITY.md) 分开交付，范围总账见 [ROUND-2](ROUND-2.md)。本次仅只读调查和保存规划，不启动产品修改、Lab、构建或发布。

## 1. 用户反馈与目标

| 项目 | 反馈 | 下一轮目标 |
| --- | --- | --- |
| B1 / R2-014：加载进度 | 进度条反复从头扫到尾，无法表达一次加载完成 | 一次顶层导航进度单调前进，成功结束到末尾并消失；失败/取消结束为对应状态，不冒充成功。 |
| B2 / R2-015：默认背景 | 深色主题下网页加载阶段始终白底 | 网页容器从首帧起使用当前 Khor 主题的默认底色，覆盖空白、加载、失败和重试；页面自己的背景由页面决定。 |
| B3 / R2-016：访问失败 | 当前 Mac Khor 打开 `https://lan.cibol.khor.cloud/guest` 显示“打不开：连接被拒绝” | 取得真实导航错误和运行路径，复现并修正具体故障；错误分类准确，重试针对失败目标，不以猜测的网络可达性阻止打开。 |
| B4 / R2-017：手机布局 | 网站底部导航被 Khor 甲板遮挡，Safari 对照没有同样遮挡 | 网站获得的可用视口与甲板占用一致，固定/粘性底栏可见且可点击，末尾内容可达；几何变化与原生视图同步。 |
| B5 / R2-018：列表左滑 | 浏览器列表左滑只有置顶 | 与其它列表一致显示置顶/取消置顶和删除；置顶项也遵循一致的删除契约，不能提供必失败按钮。 |

用户截图证据：`/Users/zgh/Desktop/截屏2026-10-28 16.44.16.png`（Mac 失败）、`/Users/zgh/khor-drop/IMG_5539.png`（Khor iPhone）、`/Users/zgh/khor-drop/IMG_5540.png`（Safari）。手机图分别是 lan 域名 guest 页面与非 lan 域名的已登录页，**不是同页面、同登录态的严格对照**。实施验收需控制这些差异，不据截图反推像素尺寸。

## 2. 只读审查事实

### B1：循环来自主动动画，尚无数值加载进度

- `app.css:2623` 对无 share 的进度条设置 1.4s 无限循环，从 scaleX(0) 到 scaleX(1)；`SessionBar.tsx:606` 的浏览器传布尔 loading，没有数值 share。
- iOS `phone_page.rs:2166` 的原生甲板同样设置 repeatCount=INFINITY；`native-deck.ts:480` 传动画参数，没有当前加载比例。只改 DOM CSS 不能修原生绘制。
- `lib.rs:1823` / `phone_page.rs:628` 只报告 started/finished，没有把网页加载估计值传到界面；`use-loading.ts:56` 的 600ms 重定向合并及 `loading-rule.ts:28` 的 30s busy 上限不是成功完成证据。
- 循环动画本身不证明网页在无限重载。目标应表示本次主文档导航，不等待网页今后的 AJAX、轮询或长连接全部结束。

### B2：网页子视图未接入默认主题背景

- 桌面 `lib.rs:1784` 的网页子 WebView builder 未设置主题背景；`repaint_the_ground:2308` 只更新 main 窗口与 main WebView。
- iOS `phone_page.rs:1257` 创建外部 WKWebView 时未同步容器/scrollView 底色；已有 `wear_the_ground` 的主界面路径没有覆盖这些网页视图。
- 截图与接线缺口吻合，尚未对当前原生视图属性/像素采样。不能将网站明确指定的白色背景算作 Khor 默认底色错误。

### B3：Mac 地址可达，实际 WebView 失败根因仍未确定

- 截图显示连接失败文案，没有提供底层错误码，不能称为 HTTP 403、网站权限拒绝或 Khor 主动拦截。
- `browse/page-fail.ts:25` 将 NSURLErrorDomain 的 -1004（无法连接）和 -1005（连接丢失）合并成“连接被拒绝”；`page_fail.rs::word` 虽保留 code/domain/description/url，但本次 runtime.log 没有找到对应原始事件。
- 2026-10-28 本次 Mac 只读 HEAD 探测：该 URL 绕过代理直连 HTTP 200，remote IP `172.16.185.103`；HTTP 地址返回 302；显式经过系统代理 `127.0.0.1:7897` 的 HTTPS 也返回 200。只证明探测当时这两条路径可达，不证明截图时 WebView 请求相同或故障已消失。
- 当前保存的 page tab 指向上述完整 HTTPS URL，device 与当前本机 endpoint ID 一致；`BrowserPane.tsx:277` 对 here 使用空 machine，应走本机直接打开。尚未采集实际 WebView 配置与失败导航，不能先归因借用远端网络。
- 调查中曾向 app 的另一个监听端口 57046 发起一次有界 CONNECT 探测并超时；随后 endpoint 元数据确认它是 **IPC 端口，不是借网代理**，该结果排除出网络故障证据。没有关闭、重启或修改该服务。
- 后续只针对失败导航保留原始错误、完整目标、导航代次与实际直连/代理信息，复现后再修。遵守 C1：不新增可达性预检、自动改写目标或改变用户选定的打开方式，不把诊断探测变成产品开网页的门槛。

### B4：滚动 inset 不等于网页可用布局视口

- 全局 iPhone 浏览器启用原生甲板；`BrowserPane.tsx:198` 将 WKWebView frame 延伸到甲板末端，网页铺到甲板后方。
- `native-deck.ts:505` 计算甲板 inset，`phone_page.rs:2190` 仅设置 UIScrollView contentInset/scrollIndicatorInsets。没有明确让网页 fixed/sticky 元素随甲板避让的布局视口/安全区契约，与截图遮挡吻合；未做同 URL 实机复现。
- 会话网页 `MobileContentPage.tsx:35` 是网页 flex 区域与 DOM 甲板分别占位的另一条路径，需要共同验收，不能直接称其同样故障。

### B5：删除操作被共用左滑筛选排除

- `SideList.tsx:274` 默认只将 pin 和 destructive 操作放进左滑；`BrowseLanding.tsx:337` 的 close 未标 destructive，普通浏览器也未传显式 swipe，结果只剩置顶。
- bare 分支 `BrowseLanding.tsx:401` 又只传关闭，形成另一种不一致；`tabs.rs:560` 现有后端拒绝关闭 pinned 标签，不能只加按钮。
- 用户本次要求优先于旧注释中的“只叫关闭、不作为 destructive”约定。删除移除对应浏览器标签/列表项并释放其资源，不附带清除 Cookie、浏览历史或网站数据。

## 3. 实现与验收约定

| 项目 | 实现方向与必须验证的行为 |
| --- | --- |
| B1 | 一份导航身份与加载状态供 DOM/原生甲板消费；接原生加载估计值，重定向不反复归零，成功才到 100%；失败/取消收束，迟到事件不覆盖新导航，已加载标签切回不重新跑。验慢页、缓存命中、重定向、失败、取消、reload、切标签。 |
| B2 | 创建前设置默认底色；主题变化同步保留的网页视图；深/浅/跟随系统下，新建、加载、失败、重试、隐藏标签切回均验。页面明确指定的背景保留，不强制改第三方网页样式。 |
| B3 | 在同机同 URL 复现真实 WebView 错误，区分连接失败、中断、代理/隧道、DNS、TLS 和网站响应；保留原错误再给正确用户提示，不靠自动重试掩盖。当前尚无唯一根因，不预定网络层重构。 |
| B4 | 用单一几何来源处理甲板展开/折叠、地址编辑、键盘、旋转；先验证原生视口/安全区能否可靠表达遮挡，不可靠时让真实 WebView frame 留出甲板区域。不得针对该网站追加 margin/padding。验同 URL/登录态、fixed/sticky 底栏、100dvh、长页面末尾及点击；全局浏览器与会话网页分开覆盖。 |
| B5 | 明确配置置顶/取消置顶 + 删除，与其它列表保持交互及错误反馈一致；置顶状态与后端删除规则统一。验普通、置顶、当前、后台、最后一个标签，删除不影响其它标签；左滑不抢纵向滚动与边缘返回。 |

## 4. 执行边界与上下文恢复

- 继承 [PLAN](PLAN.md) 的工作树、多智能体、独立审查和 native 构建唯一槽规则，以及 [ENGINEERING](ENGINEERING.md) 的 Clean Code 约定。原生生命周期/背景一线、布局/列表一线、独立复现审查一线；公共桥接和全局样式先指定唯一 owner。
- 改 UI 前读设计系统。真实 macOS/iOS WebView 与原生甲板验收不能用 Chrome DOM 截图替代；未覆盖平台如实记录。当前没有运行新专项测试、启动 Lab、操作用户 App 或更改产品。
- 外部网络探测为诊断读取，没有登录、提交表单或改网站。现有 App、用户 session、借网连接与后台服务保留。
- 恢复先读 STATE → 本文 / SESSION-CONTENT / SESSION-CONTINUITY → ROUND-2 / PLAN，再核实际授权与 refs/diff。三项专项范围分别记录，跨专项优先级待下一轮开工时对齐。
- 仍在下一轮问题收集阶段，明确允许后才开始实现；不把问题记录或上轮 ship 授权当作开工令。新独立问题记录，不自动扩成全浏览器重写。
