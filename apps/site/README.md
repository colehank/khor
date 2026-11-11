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
- `src/scripts/story.ts` 按滚动位置进入章节，每句触发一次完整演化；One mind 时外珠缩到最终尺寸，中心珠以最终绿色浮现；How about 一步形成外围灰阶结构，Meet 再为外围上色。合盖和手机接续共用原地调整尺寸的同一 iframe，保留任务与输入。
- 减少动态效果模式直接显示成形图标，保留手动打开体验与下载。
- `src/releases.ts` 是下载页和 `/download.json` 的共同来源，仅列已验证的发行资产。
- 站点 token 从 `docs/design-system/tokens.json` 生成；不另存调色板与字体。
- 演示产物在 `public/experience/app`，由构建生成并忽略；没有重新实现一套 App 页面。

## 部署

本轮只作本地预览，没有合并或线上部署。
现有 `deploy/push.sh` 继续负责静态站与下载镜像的分阶段同步；页面同步保护 `/dl`，
版本校验完成后才推进下载指针。详细约定见 [镜像维护](../../scripts/release-mirror.md)。
中继、寻址和官网分别部署。
