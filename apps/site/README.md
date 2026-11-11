# khor.cloud

Astro 官网、文档和真实前端的交互演示。演示入口为
`apps/gui/dev/site`：复用生产 App 与浏览器内的示例数据，不连接实际设备。

## 本地迭代

```sh
npm ci --prefix apps/site
npm ci --prefix apps/gui
npm run demo:build --prefix apps/site
npm run dev --prefix apps/site -- --host 127.0.0.1 --port 23820
```

前端需要已有的 `apps/gui/src/gen` 生成契约。当前 worktree 复用 RC12 官方
frontend artifact 的 gen（Release run 42824017790，产品 GUI 源码与当前基线相同），
没有触发 Rust 编译。全新环境按仓库前端生成流程准备契约。

改官网文案、动效和 CSS 后由 Astro 自动刷新；仅修改演示数据或 App 组件时重跑
`demo:build`。`demo:check` 检查演示与生产前端类型；`npm run build` 先打包演示再输出静态站。

- `src/components/Mandala.astro` 读取完整的 `mandala-glass.svg` 和既有 session 字形；`icon-story.ts` 在原图层上控制演化，不替换成另一幅图。
- `src/components/ProductHero.astro` 是产品首屏：Desktop、Mobile 使用真实前端的静态截图，CLI 使用已核对命令格式的示例输出。三种界面共用首页适配任务；没有嵌入交互体验。
- `src/assets/product` 的两张 WebP 来自本地 `/experience/app/index.html?scene=gui&items=3&machines=3&theme=light`，Chrome 视口分别为 1080×680 与 390×780，2× 像素密度、WebP quality 92。App 或 `apps/gui/dev/site/scenario.ts` 更新后重新截图；全部任务、设备与输出都是演示数据。
- `src/scripts/story.ts` 在产品首屏后按滚动位置连续演化图标，停止滚动即停在当前姿态。One mind 时外珠归位缩小，中心珠和内圆保持最终绿色与几何；How about 画出外环双细边与菱形轮廓，外珠仍灰；Meet 的左上扫光恢复原始材质与外珠绿色。
- 减少动态效果模式按当前章节直接展示静态姿态，图标故事仍从四颗灰珠开始。产品首屏、功能、资源体验与下载均可直接访问。
- `src/releases.ts` 是下载页和 `/download.json` 的共同来源，仅列已验证的发行资产。
- 站点 token 从 `docs/design-system/tokens.json` 生成；不另存调色板与字体。
- 演示产物在 `public/experience/app`，由构建生成并忽略；没有重新实现一套 App 页面。

## 部署

本轮只作本地预览，没有合并或线上部署。
现有 `deploy/push.sh` 继续负责静态站与下载镜像的分阶段同步；页面同步保护 `/dl`，
版本校验完成后才推进下载指针。详细约定见 [镜像维护](../../scripts/release-mirror.md)。
中继、寻址和官网分别部署。
