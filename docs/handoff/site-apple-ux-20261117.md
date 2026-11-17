# Khor 官网 UI/UX 精修 · 2026-11-17

后续用户反馈：移除 Continue anywhere 中额外加入的会话结果截图、atlas-web 与说明注释。
该幕恢复标题、正文和概念图；相关组件、四张局部图片与 CSS 已删除。
首屏 GUI 的中文示例、40% 窗口底色及动效修复保留。下文截图片段记录为历史状态。

状态：仅本地实现，未提交、未部署。worktree 为 `khor-site-story`，branch 为
`feat/site-story`，基线 `7accd420`。预览：<http://127.0.0.1:23820/>。
本记录接续 [偏好与两端形变](site-preferences-20261117.md)，保留此前未提交的语言、主题、布局和材质形变工作。

## 用户随后调整（以此为当前状态）

用户明确反馈“还是之前的好，恢复可交互，下载”。已恢复桌面 GUI 自动加载和直接操作，
移除进入／退出试用按钮与父页面对 Escape 的接管；手机仍沿用稳定首图策略。
Download 固定按钮文字和平台字形恢复，手机进入 `/download/`，删除网页访问设置主入口与说明；
平台／CLI 安装区常显，Coming soon 恢复悬停提示，顺序恢复原先排列。
保留中英文与主题、缩短后的序章、真实消息片段及其他布局调整。
用户随后还要求恢复 GUI 透明度：窗口底面回到 40%，中英文、浅深主题的完整首图也从真实 App 重新截图。
本次透明度恢复证据在 `../transparency-restore/`，不修改文字透明度或生产 App 的默认材质。
下方是调整前的实现记录，交互、下载和 80% 窗口底面条目已被本节覆盖。

恢复检查证据为同目录 `restore-verification.json`、`restore-check.log`、`restore-build.log`、
`restore-desktop.png`、`restore-mobile.png`；仅本地，不发布。

## 已执行的方向

用户在 Apple UI/UX 审视后选择推荐方案：保留五句序章但缩短；真实 GUI 主动进入；
解决手机访问／下载预期、产品价值证据及内容可读性。

- 首屏默认真实 GUI 海报，不设置 iframe src，不捕获滚轮。精细指针设备显式点击
  Try the interface 后加载 App，等待真实就绪信号再原位显示；Back to page、Escape、
  点外侧、离开预览视口均可退出。加载超时保留海报并提供 Try again，支持再次进入。
- 手机和粗指针仍移除 iframe，仅使用完整真实首图，不增加放大层，不改变网页两指缩放。
- 五句及其原始 SVG 演化保留，开场 sticky 总长度从约 8.3 屏降为桌面 4.0／手机 4.05 屏。
  演化仍在每段前 60% 完成，后 40% 保持完成态。网络展开后有 .3 屏阅读距离。
  中间章节自然滚动；Agent 的 1 屏阅读、.85 屏合拢、Build 的 .8 屏保持不变。
- 用可读的 Devices／Agents／Mind／Together／Khor 书签代替无文字的点，首屏另有
  Connection／Sessions／Khor Agent 直接章节链接，不增加滚轮接管或强制翻页。
- Continue anywhere 保留关系图，追加实际 App 的消息片段。片段内容与首屏共享
  world / scenario，是同一任务已修改文件和八项检查通过的真实渲染截图；不是重画聊天界面。
- 首屏与收尾共用平台推荐；手机为 Set up web access，使用 Web 字形并明确 Mac／Linux
  主机要求，指向 `/docs/web/`。导航 Download 始终进入安装页；手机安装页有网页访问说明。
  导航 Docs／Download 使用可见文字，GitHub 保留最右侧图标。
- 其他安装方式在首屏用原生 details 收纳，当前可用 macOS／Linux 排前；未来平台是静态
  Coming soon 状态，不提供无效操作。删除不再使用的悬停提示脚本与 CSS。
- 窗口底面不透明度从 40% 改为 80%，保持真实玻璃控件和完整前景。指定 Fog 参数未改。
  背景流仅在进入试用时创建，退出即释放；没有第二个 WebGL 实例。
- 宽屏导航、首屏及后续正文共用内容对齐轴；小屏收尾按钮与主机条件说明完整可见。
  修正减少动态效果时第一幅网络图的首帧显示，三幅图无需滚到视口才初始化可见状态。

## 真实资产

完整首图是 Chrome 1360×850、DPR 2 的实际 App 透明截图；消息片段来自
1200×850、DPR 2 的同一实际 App。EN／ZH、light／dark 各四张，WebP quality 92。
只做 PNG → WebP 格式转换，没有后期绘制 UI。中文版本沿用共享项目的英文内容，
App 自身控件与导航按浏览器语言变化。

文件位于 `apps/site/src/assets/product/desktop*.webp` 与 `session*.webp`。
App 或 fixture 改变时，应重新截图，避免首图与进入后的界面不一致。

## 验证与证据

本轮仅使用 headless Chrome，无桌面测试窗口、WebKit 矩阵、真实设备连接或原生构建。

- 桌面 1440×1000 EN／dark：默认 iframe 未加载，海报上滚动 650px 时页面前进 650px；
  进入、Escape、再次进入、按钮退出通过。
- 手机模拟 390×844 ZH／light 与 320×700 EN／dark：iframe 移除；主要入口为网页访问；
  安装页提示、语言和主题正确。无横向溢出、重复 ID 或脚本错误。
- 全新减少动态效果页面：三幅网络图首帧均可见。
- 2560×1440 light：导航品牌与首屏正文起点均 x=515；原生平滑章节导航落点正确。
- 320×700 收尾主机条件说明底边 y≈662，小于可视高度 700。
- 阻断 iframe 请求：12 秒后保留海报并显示 Try again，没有空白界面替换。
- 演示类型检查、相关站点 TypeScript、完整静态构建通过；最后 CSS 清理后再执行 Astro 构建。

证据目录：`/Volumes/disk0/Khor/evidence/site-apple-ux-20261117/implementation/`。
`verification.json` 保存主要交互和响应式结果，`finish.log` 保存窄屏收尾、宽屏对齐和错误恢复结果；
`desktop-*`、`mobile-*`、`small-*`、`wide-*` 与 `preview-error.png` 为实际页面截图。
构建输出为 `build.log`、`astro-final.log`。

Chrome 的手机模拟不等于 iPhone Safari 真机验收；本轮未声称再次完成 Safari 缩放验证。
此前用户真机确认的静态首图策略保持。

原 Apple 视角审视记录为
`.impeccable/critique/2026-11-16T19-18-09Z__apps-site-src-pages-index-astro.md`；
本轮落实其中全部优先问题，不以新的分数代替用户体验验证。
