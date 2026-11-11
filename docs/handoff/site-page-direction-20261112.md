# Khor 官网下一轮页面方向 · 待讨论

用户要求本轮先完成头像与图表，给出预览；下一步全面参考 https://openai.com/codex/ 的布局与文字，讨论后再实施页面重排。本文件是讨论草案，不表示已改版。

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

## 观察与建议

Codex 当前页面先给居中的产品图标、短标题、一句补充和双按钮，随后用大 GUI 建立产品印象；能力段落交替排布，最后再给入口。我们可借用这个节奏。

建议顺序：kapestack 导航 → Fog 居中首屏 → 大幅可操作 Khor GUI → 支持的设备与 harness → 三组能力介绍（智能体协同、文件与 Git、设备上的网页）→ 简短图标演化 → 下载收尾。原九屏图标故事建议缩短，避免占用产品说明的主要篇幅。客户背书与套餐部分应按 Khor 实际内容重新组织。

主标题可以先沿用已确认的 “Work across your devices and harnesses.”，副文案只解释智能体、文件和网页。Fog 建议用于首屏；GUI 与图表使用干净底色，以保留产品可读性。具体标题和图标故事长度待本轮预览后讨论。

参考截图保存在 `/Volumes/disk0/Khor/evidence/site-story-20261112/page-references`。已用 Chrome 打开 Codex 与完整 hash 参数的 Vanta 页面，Fog 控制面板和实际画布均已查看。
