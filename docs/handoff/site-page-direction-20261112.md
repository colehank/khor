# Khor 官网页面改版 · 已本地实现

用户先要求完成头像与图表，再讨论 Codex 风格的布局与文字；随后明确「先做」，已在 `feat/site-story` 本地实现页面重排。没有远端构建、合并或发布授权。

## 已确认的约束

- 用户最新要求左上角恢复原始图标 + Khor；kapestack 公司链接保留在页脚。此前采用 OpenAI / Codex 层次的顶栏方案被本条替代。
- 全程静态网站、本地演示数据，不接真实服务；保留 Khor 内置 GUI、设备自适应下载和浏览器明暗跟随。
- 不可点击项保持正常颜色；此前确定的有限交互范围保留。
- 动态背景使用下列 Vanta Fog 参数，不能自行改色、速度、缩放或开启交互。

```js
VANTA.FOG({
  el: '#khor-hero',
  backgroundAlpha: 1,
  baseColor: 6916440,
  blurFactor: 0.5,
  gyroControls: false,
  highlightColor: 8232555,
  lowlightColor: 16742489,
  midtoneColor: 16775656,
  minHeight: 200,
  minWidth: 200,
  mouseControls: false,
  scale: 2,
  scaleMobile: 4,
  speed: 1,
  touchControls: false,
  zoom: 1,
});
```

来源：https://www.vantajs.com/?effect=fog#(backgroundAlpha:1,baseColor:6916440,blurFactor:0.5,gyroControls:!f,highlightColor:8232555,lowlightColor:16742489,midtoneColor:16775656,minHeight:200,minWidth:200,mouseControls:!f,scale:2,scaleMobile:4,speed:1,touchControls:!f,zoom:1)

## 最终实现

最新文案定稿：“Work across your devices and agents.” 后另起一行，接 “Access agents, edit files, track changes, and connect to web apps from any device, peer to peer.”。平台组标题为 Other ways to get Khor；用户最新决定灰显项继续悬停显示 Coming soon，不常驻。增加 or Khor CLI 及安装命令，1200px 起和组标题、平台图标横排一行，窄屏分行；悬停显示复制图标（触屏常显），复制后短暂换勾，无可见成功文字。以下此前文案与提示方式为历史记录。

文案最新修订：用户恢复完整英文文案，合为一个小字段落，不手动换行；“Work across your devices and agents. Edit files, track changes, and use your web apps. From any device, peer to peer. Efficient, secure, and delightful.”。下述两段排法为上一轮记录。

后续用户定稿优先：顶栏改回原始应用图标 + Khor；首屏左侧第一层为大字 Khor，第二层为两段小字英文，第三层为下载 / 文档，无箭头；正文不再重复应用图标，GUI 保持右侧。下方为 macOS、Windows、Linux、iOS、Android 的原生系统标记按钮，可用性来自 release 资产；未发布平台灰显，悬停 / 聚焦 Coming soon。kapestack 公司链接保留在页脚。文案为 “Work across your devices and agents. Edit files, track changes, and use your web apps.” / “From any device, peer to peer. Efficient, secure, and delightful.”。以下大号标语、kapestack 顶栏等为历史阶段记录。

2026-11-12 全页收整补充：品牌故事固定黑场，GUI、下载页和文档继续随浏览器主题。文字与图形并行起动，不再为每句文字单独占用等待区间；原始首幕停留和正常总行程不变。最新用户要求取消放大，完整 Desktop GUI 只在原位置缩放展示与操作，手机也不再有覆盖按钮。减少动态效果使用紧凑静态故事。共享导航接通自适应下载，下载页按 App / CLI 分组，GUI 安装和设备连接路径已补齐。完整记录与证据见 [整页收整](site-refinement-20261112.md)。

Codex 当前页面先给居中的产品图标、短标题、一句补充和双按钮，随后用大 GUI 建立产品印象；能力段落交替排布，最后再给入口。我们可借用这个节奏。

用户随后要求恢复「左侧介绍、右侧 GUI」，启用毛玻璃，并去掉下方介绍段落与 GUI 旁注。当前顺序为 kapestack 导航 → Fog 上的左介绍 / 右毛玻璃 GUI → “You have many devices” 图标演化 → 下载收尾。窄屏按介绍 / GUI 上下排列。不显示系统标识、harness 标识、三组图文或 “Khor Desktop / 试着发条消息”。图标故事保留全部原始姿态和最终图稿，先为首幕分配桌面 .5 屏 / 手机 .7 屏的阅读停留，再进入桌面 4.8 屏 / 手机 6.2 屏的演化；原生 ScrollTimeline 共享规范化演化地标 `0 / .22 / .47 / .73 / 1`，含停留与 sticky 视口总高分别 6.3 / 7.9 屏。详见 [滚动实现](site-scroll-research-20261112.md)。

首屏标题为 “Work across your devices and agents.”，副文案为 “Your agents, files, and web apps. From any device, peer to peer.”。用户随后要求删除额外的 Git 中文说明，首屏只保留上述英文介绍。Fog 只用于首屏，末端以至少 40svh 的渐变接到后续故事的黑场。参数集中于 `apps/site/src/scripts/fog-options.ts`；本地固定分发 three.js r134 与 Vanta 0.5.24，许可证随附。减少动态效果与加载失败时使用本页同参数画布的干净截图，不改速度或颜色。

首屏 GUI 使用生产 App 的单层半透明背景（最新为 40%），保留前景完整 opacity；首图由父窗口模糊取样 Fog；交互时将同一个 Fog 画布流（或同参数静态帧）提供给 iframe，让内部真实玻璃控件可取样，不重复叠加暗底。没有启用原生后端、调用 Tauri 或改变用户 App 设置。加载首图也保留 alpha。外框不画白边，圆角和 GUI 共用缩放系数，避免手机上红黄绿按钮被裁切。Chrome 已核对左右位置、真实背景 alpha、完整前景和固定回复；WebKit 手机布局仍为完整 1360 视口。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/glass-hero`。

Chrome 已确认实际 WebGL 画布、kapestack 链接、唯一 H1、大幅 GUI 发送与固定下载回复、三组图文、完整 Meet 形态；1440 与 390 视口无横向溢出，减少动态效果不创建画布。页面错误与运行时外部请求为空。证据在 `/Volumes/disk0/Khor/evidence/site-story-20261112/codex-layout`。

参考截图保存在 `/Volumes/disk0/Khor/evidence/site-story-20261112/page-references`。已用 Chrome 打开 Codex 与完整 hash 参数的 Vanta 页面，Fog 控制面板和实际画布均已查看。
