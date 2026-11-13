# Meet 后的概念图 · 2026-11-13

> 此大线框版本已被用户否定。用户给出 iroh 官网作为参考，当前实现见 [iroh 参考重做](site-iroh-20261113.md)。以下保留为历史。

用户要求「不要每句话后面都要接一个 ‘.’」「不要下面 Khor 的介绍都用 GUI 截图，要好看的概念图」。本轮替换 Meet 后的三项媒体，保留既有黑场、系统字体、Khor 标记和首屏真实 GUI，不发布、不合并 main。

## 当前实现

- P2P：电脑与手机通过两条直接资源路径连接，分别对应 `src/export.ts` 和 `localhost:5173`；文件／网页留在来源设备，接收端读到同一资源。为桌面与手机分别绘制构图。
- Khor Agent：沿用图标的中心节点和内环几何，一项请求分流到 Codex、Claude、Cursor，结果路径再汇集回 Khor。系统与 harness 标记出自生产 App，保留原 SVG 的填充、描边与遮罩属性，实例 ID 分别命名。
- Terminal：一个固定在 MacBook Pro 的 `npm run dev` 进程，左右访问端说明 Desktop 到 Phone 的变化。它是可解释持续运行关系的矢量概念图，不是可输入的 GUI 或终端截图。

三个构图均为明确几何的 inline SVG，不加载介绍截图、canvas、视频或交互 App。进入视口时各自做一次有限演示，文本与主体从第一帧可读；减少动态效果下完整静态呈现。原来 Desktop／Phone 截图选择器随截图版本一起移除。

标题、首屏短简介、五句动画字幕、收尾与页脚取消尾部句号；保留问句、文件名、URL 及产品 GUI 内正常内容。旧的 12 张介绍 WebP（约 1.44 MB）、TourImage、tour-assets、介绍专用 GUI 截图场景与 sample browser adapter 已删除，默认首屏 fixture 恢复此前实现。

## 范围与验证

入口 <http://127.0.0.1:23820/#connect>，另外两项为 `#khor-agent`、`#terminal`。线上未更新。

证据目录 `/Volumes/disk0/Khor/evidence/site-story-20261113/concepts`，包含三项宽窄屏各一张、首屏、方向记录和验证 JSON。页面上下文归档为该目录的 `page-desktop.png` 与 `page-mobile.png`；长图用于布局上下文，不能代表滚动图标故事的完整时间序列。

- 33 项集中 Chrome 检查通过，无页面错误：无介绍截图／iframe／canvas、宽窄屏无文字裁切或横向溢出、有限动效、短句尾部句号、手机不请求 App、Chrome 缩放、320 宽和减少动态效果。
- 网站脚本 TypeScript、默认 GUI 演示打包与 Astro 静态构建通过；CSS 声明使用既有 token，沿用原 760px 断点。
- 有界设计检测为 `[]`。首次画面检查修正生产 SVG 根属性丢失、字体 token 名称与 Chrome 颜色插值异常，再捕获最终图。没有新增 Safari 真机、原生、Rust 或真实 P2P／模型／PTY 验证。

最后一轮补回手机 P2P 网页标记的经线，并移除无归属连接线。独立视觉结论见同一证据目录的 `finish-verdict.md`，文档核对见 `documentation-review.md`；本轮没有修改设计权威文档或全局 token。
