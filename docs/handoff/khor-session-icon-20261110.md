# khor 会话类型图标（2026-11-10）

用户确认：把刚定稿的坛城提炼成单色小图标，统一新建入口、列表和标签。

唯一单色图稿：`apps/gui/src-tauri/icons/src/mandala-glyph.svg`。`BrandIcons.IconKhor`
读取它的 viewBox 与路径，所有会话表面通过已有的 `sessionMark` / `sessionKind` /
`SessionFace` 使用。只有 khor category 使用品牌标记；广播、传输、借网与终端保留专属
类型；未知 Agent 保留通用标记。新建入口与创建后的 khor 会话使用相同判断。

最终以用户导出的 `khor-session-avatar.json` 为准，原文件完整保存在图稿同目录。
65% 占比；外圆/菱形线宽 40、内圆 45；外珠半径 55、心珠 69、内圆半径 170。
CSS 只对 khor 浮现/类型底座的实际图形应用 65% 比例，角标继续使用原尺寸槽位。
Claude、Codex、Cursor、终端、未知 Agent 等其他类型的图稿和尺寸保持原样。
预览机型、主题和对照尺寸不进入生产判断；机器底图、自动墨色和基础衬色继续跟随
真实机器记录。应用母图、参数和安装资源不变。

`npm run khor-conversations-ui` 已扩展真实组件的入口/行/标签一致性与专属类型保护检查，
覆盖桌面/手机及浅/深色，保存截图。远端运行该入口及完整 Quality 后才能推进 main。
