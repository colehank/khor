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
- `src/components/ProductHero.astro` 是左文右 GUI 的产品首屏，只展示 Desktop，外层带 macOS 圆角窗口边框。红黄绿按钮与 App 的导航图标、右侧标签共用原有第一行，不另加标题栏；仅网站演示在原有窗口控制留白处绘制按钮，不启用原生后端。完整 App 布局包括会话列表、对话与项目文件；首图加载后接管为可操作的同源 iframe，不连接 Lab 服务或真实设备。没有「打开体验」入口。
- 示例数据包含 macOS、Windows、Linux、iOS、Android 五种系统的六台设备，Claude、Cursor、Codex、Khor、Shell 五种会话类型；全部是浏览器内的合成连接状态。会话字形沿用生产 App 的类型标记。
- `src/assets/product/desktop.webp` 与 `desktop-dark.webp` 来自本地 `/experience/app/index.html?scene=content&hero=1`，Chrome 1360×850、2× 像素密度、WebP quality 92，分别在浏览器浅色和深色偏好下截图。App 或 `apps/gui/dev/site/scenario.ts` 更新后重新截图。首图与可操作 GUI 均跟随浏览器明暗偏好；GUI 使用生产 App 的 auto 主题监听，实时切换不重载会话。所有任务与文件均为浏览器内的演示数据；发送消息固定回复 “Let's try Khor, download it here.”，链接进入同一套下载推荐。
- `src/scripts/product.ts` 保持完整桌面布局并按展示宽度缩放；`src/scripts/download.ts` 根据平台与可用架构信息推荐已有的发行包。macOS 的旧 UA 不用于猜测芯片，信息不足时进入平台选择页；未提供安装包的平台展示所有选项。
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
