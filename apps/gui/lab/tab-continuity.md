# 标签连续性 lab

本地入口：<http://localhost:23619/tab-lab>

```sh
cd apps/gui
./node_modules/.bin/vite --config lab/tab-continuity.vite.ts
```

本 lab 使用隔离夹具，不连接现有会话或启动 PTY。目录、文件、网页均为演示内容。
切换「空白起步／拥挤标签／重置／头像状态环」会重新载入夹具；「桌面／窄屏」切换视口。

## 已接入的真实组件

- 内外层直接共用 `TabStrip`：同样的尺寸、悬浮关闭按钮、容器宽度适配和鼠标重排；内层不再复制标签 DOM/CSS，只保留层级对应的选中底色。
- `ContentPicker`、`FilePage`、`MobileContentPage`、`MobileContentSheet`、`SessionScreen`、`TabSheetRows`、`NewSessionPage`。
- `SessionFace` / `FaceMark` 的状态环使用同心 SVG 描边，保留现有宽度、透明间隙、状态色与忙碌呼吸。头像状态环场景显示六种状态的标签和列表尺寸。

## 可体验的流程

1. 再点当前会话，展开一枚「新会话标签页」。
2. 反复点内层 ＋，每次增加独立空白标签。
3. 选择文件或输入网址，原标签原位变成内容标签，ID 与顺序不变。
4. 不同标签各自保存目录、查询、地址和文件草稿；同一会话内普通切换保留已挂载内容。
5. 外层会话切换沿用 240ms 玻璃移动，内层内容切换采用 180ms；两层各自解析选中区域，互不捕获。重复点当前标签、点击 ＋／×、展开／收起不触发玻璃跳转；关闭选择相邻标签，最后一枚关闭后回到纯会话。
6. 收起会话药丸同时关闭侧区；窄屏通过同一层底栏总览新增、切换和关闭内容。
7. 拖动内层标签只重排内层，不切换当前内容、不影响外层顺序。

## 当前验证

- `node lab/tab-continuity-model.spec.mjs`：空白标签独立性、上下文隔离、原位转内容、重排、相邻选择、关闭最后一枚。
- `./node_modules/.bin/tsc --noEmit -p lab/tab-continuity.tsconfig.json`。
- `node scripts/check-px.mjs`、`node scripts/check-icons.mjs`。
- Chrome 实际操作：多次新增、文件／网页原位打开、内层鼠标重排、收起同步关闭侧区、窄屏总览关闭最后一枚、浅深色状态环。

动效记录显示在产品画面之外，由真实玻璃替身的创建事件读取层级和时长，方便确认哪些操作触发了移动。

主窗口仍是演示终端；网页内容固定为本地示例，不能用于验证真实网络导航或后端会话识别。
生产实现与本 lab 一并交付；正式 App 的独立节点验收记录见下。尚未发布安装包。

## 正式实现验收（2026-10-18）

- `npm test --ignore-scripts`、`npm run build --ignore-scripts`、`npm run content-model --ignore-scripts`、`npm run gates --ignore-scripts` 通过。
- `cargo test -p khor-node tabs::tests:: --lib`：27 项通过，包含空白标签独立、丢失回执重试、原位转内容后重试不回滚。
- `cargo check -p khor-gui --lib` 通过；Rust 构建目录使用外挂盘 `/Volumes/disk0/khor-target`。
- Chrome 中操作正式 App，连接隔离节点：连续新建、文件与网址原位打开、独立搜索、重载恢复、跨会话切换、宽窄屏切换、总览新增、逐个关闭至纯会话通过。共享关闭按钮实测垂直中心偏移为 0。
- 草稿按标签、机器、路径隔离；实际编辑 guide.md 后在同一标签改开 notes.md，notes.md 保持原文，guide.md 的离开保存写入正确文件。
- 更新后的 `check-session-content.mjs` / `check-content-ui.mjs` 已语法检查；本轮未整段执行这两个浏览器脚本。
- 本轮未在原生 iOS、Windows、Linux 上做运行时验收。Chrome 网页地址与标签转换通过，不替代原生 WebView 导航验收。
