# Khor 官网页面改版 · 已本地实现

用户先要求完成头像与图表，再讨论 Codex 风格的布局与文字；随后明确「先做」，已在 `feat/site-story` 本地实现页面重排。没有远端构建、合并或发布授权。

## 已确认的约束

- 左上角品牌是 `kapestack`，链接到 https://www.kapestack.top；Khor 是产品名，对应 OpenAI / Codex 的层次。
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

Codex 当前页面先给居中的产品图标、短标题、一句补充和双按钮，随后用大 GUI 建立产品印象；能力段落交替排布，最后再给入口。我们可借用这个节奏。

用户随后要求恢复「左侧介绍、右侧 GUI」，并启用 GUI 毛玻璃。当前顺序为 kapestack 导航 → Fog 上的左介绍 / 右毛玻璃 GUI → 支持的设备与 harness → 三组交替图文（智能体、文件与历史、设备与网页）→ 图标演化 → 下载收尾。窄屏按介绍 / GUI 上下排列。不引入客户背书与套餐。图标故事保留全部原始姿态和图层，行程缩为 2.6 屏（章节地标 `0 / .55 / 1.15 / 1.8 / 2.6`，含 sticky 视口总高 3.6 屏）。

首屏标题为 “Work across your devices.”，副文案为 “Your agents, files, and web apps. From any device.”。Fog 只用于首屏，后面的图文段使用干净底色。参数集中于 `apps/site/src/scripts/fog-options.ts`；本地固定分发 three.js r134 与 Vanta 0.5.24，许可证随附。减少动态效果与加载失败时使用本页同参数画布的干净截图，不改速度或颜色。

首屏 GUI 使用生产 App 的单层半透明背景（最新为 50%），保留前景完整 opacity；父窗口用 24px 模糊取样 Fog，解决 iframe 不能从内部取样父页面的问题。没有启用原生后端、调用 Tauri 或改变用户 App 设置。加载首图也保留 alpha。外框不画白边，圆角和 GUI 共用缩放系数，避免手机上红黄绿按钮被裁切。Chrome 已核对左右位置、真实背景 alpha、完整前景和固定回复；WebKit 手机布局仍为完整 1360 视口。证据 `/Volumes/disk0/Khor/evidence/site-story-20261112/glass-hero`。

Chrome 已确认实际 WebGL 画布、kapestack 链接、唯一 H1、大幅 GUI 发送与固定下载回复、三组图文、完整 Meet 形态；1440 与 390 视口无横向溢出，减少动态效果不创建画布。页面错误与运行时外部请求为空。证据在 `/Volumes/disk0/Khor/evidence/site-story-20261112/codex-layout`。

参考截图保存在 `/Volumes/disk0/Khor/evidence/site-story-20261112/page-references`。已用 Chrome 打开 Codex 与完整 hash 参数的 Vanta 页面，Fog 控制面板和实际画布均已查看。
