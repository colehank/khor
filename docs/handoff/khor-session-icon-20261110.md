# khor 会话类型图标（2026-11-10）

用户确认：把刚定稿的坛城提炼成单色小图标，统一新建入口、列表和标签。

唯一单色图稿：`apps/gui/src-tauri/icons/src/mandala-glyph.svg`。`BrandIcons.IconKhor`
读取它的 viewBox 与路径，所有会话表面通过已有的 `sessionMark` / `sessionKind` /
`SessionFace` 使用。khor category 和自带广播使用品牌标记；传输、借网与终端保留专属
类型；未知 Agent 保留通用标记。新建入口与创建后的 khor 会话使用相同判断。

单色图稿含外环、菱形、内环与五颗珠。相对安装图稿只做小尺寸的线宽/外珠光学校正，
没有底板、材质或固定色；跟随现有头像墨色。应用母图、参数和安装资源不变。

`npm run khor-conversations-ui` 已扩展真实组件的入口/行/标签一致性与专属类型保护检查，
覆盖桌面/手机及浅/深色，保存截图。远端运行该入口及完整 Quality 后才能推进 main。
