# GUI 会话上甲板 Lab

状态：实验分支 `lab/gui-session-layout`，用户尚未确认，不合入 main。

Khor、Claude、Codex 的 GUI 对话采用顶部返回／药丸，中间会话内容，底部输入框。滚动历史时顶部甲板保持展开。药丸的纵向入口朝内容区展开：顶部下拉、底部上拉，横向手势使用同一个正式识别器。终端、文件、网页保留原布局。

预览使用正式 App、ChatView、MachineChat、SessionScreen、DeckBand 与 Composer，仅桥接数据由已有 AI Elements 示例提供。布局开关只在此 Lab 的 Vite transform 中传入；生产 App 不传该开关。没有真实模型调用，也不读取或修改真实节点。

```sh
cd apps/gui
node_modules/.bin/vite --config lab/gui-session-layout.vite.ts --host 0.0.0.0 --port 23795
# 打开 http://localhost:23795/lab/gui-session-layout.html
node lab/check-gui-session-layout.mjs
```

面板支持会话、宽窄屏、深浅色、模拟键盘与原布局对比。浏览器模拟的安全区、键盘和触摸尺寸只用于评估排版；iOS 原生键盘动画、惯性与通知展开仍需后续真机验收。当前为待讨论方案，不修改正式设计裁定。

2026-10-23：23 项正式组件的浏览器检查通过，覆盖三类会话、主题、键盘、多行输入、横竖屏、宽屏、下拉总览、长按取消及 Lab 控件。`npm test --ignore-scripts`（含 TypeScript）通过。截图在 `/Volumes/disk0/khor-gui-session-layout-proof/`。main 工作区未修改。

## 输入区统一实验

2026-10-23 用户要求先做 Lab 查看，尚未批准合并。默认采用统一输入区，勾选「对比原来的输入区」恢复原来的模型行、权限与信息入口。顶部甲板位置可独立对比。

- 顶部模型短名称打开会话设置。ACP 沿用宿主提供的模型、思考与权限选项；自动执行状态留在顶部入口，完整模型 ID 收在详情。Khor 通过执行节点的 Pi SDK 查询模型、思考与自动压缩选项，选项校验后立即保存；不从示例清单推断真实能力。
- 底部统一「＋、发消息…、发送／停止」。添加菜单只含附件操作；没有附件时不显示上传目录。添加后显示短目的地、附件和移除操作。上传目的地也能在选文件前从添加菜单更改。
- 「附带示例文件」通过正式文件输入事件加入一个内存中的示例 Markdown。预览 bridge 消耗并丢弃上传字节，不写文件；模型、权限、发送和停止也只操作内存示例。
- 顶部设置与「＋」直接复用 App「…」的 `DropdownMenu`、`MenuViewport`，共用玻璃材质、圆角、菜单行和滚动上限。选择模型／思考／权限时在同一菜单里切换选项列表，支持「返回」、方向键与 Esc；不在菜单内堆表单控件。「上传位置」先关闭菜单，再打开持久挂载的公共目录选择器。
- Khor 菜单明确显示「执行权限：逐项确认」。实际执行由 Rust MCP 边界审批：读取直接执行，修改操作等待用户允许。目前是固定策略，没有可切换的自动授权档位。选择 Khor 场景后勾选「演示 Khor 待审批」，可查看正式审批组件的批准／拒绝／停止流程；示例不会执行工具。

```sh
node lab/check-gui-session-composer.mjs
node lab/check-acp-session.mjs
node_modules/.bin/tsc --noEmit -p lab/gui-session-layout.tsconfig.json
```

20 项输入区浏览器检查及完整前端默认检查通过。输入区截图在 `/Volumes/disk0/khor-gui-session-layout-proof/composer/`。本轮仍未运行原生 iOS 键盘与屏幕阅读器验收。

菜单修正验收：24 组场景／主题／尺寸比对通过，顶部和附件菜单的实际背景、圆角、阴影、玻璃滤镜与 App「…」一致，菜单行高和视口边界通过。模型／思考／权限选择、返回／Esc／焦点、目录选择器，以及 Khor 批准／拒绝／停止交互通过；Rust 的取消后旧批准不执行回归通过。23 项布局、20 项输入区与 ACP 会话回归、默认前端检查和生产构建通过。菜单截图在 `/Volumes/disk0/khor-gui-session-layout-proof/menu-fix/`。

真实协议和节点适配范围见 [ACP 会话基座](../../../docs/architecture/acp-unification.md)。预览仍用隔离的内存 bridge，不能用它证明实际供应商账号已登录。
