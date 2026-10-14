# 终端配色与桌面窗口背景

设置 → 终端提供「跟随 khor」、柔和深色、暖浅色、高对比度、Catppuccin Mocha/Latte、Dracula、Nord、Tokyo Night/Day。配色属于观看设备，远程会话不被修改。

自定义可以编辑或从预设创建配色，命名后保存。使用 HEX 颜色；选区支持带 alpha 的 HEX，导入也接受选区 rgba。ANSI 16 色与从 16 开始的最多 240 个扩展色可以独立指定。扩展色留空使用标准色表。程序输出的 RGB 真彩色保留。

导入接受原生 xterm 颜色对象或 `{ "name": "我的配色", "theme": { ... } }`；未填写的字段从当前配色补齐。导出生成同一结构：桌面 app 使用系统保存面板，网页下载 JSON，手机复制 JSON。文件上限 100 KB，自定义配色最多 50 个。删除当前配色后回到跟随 khor。

窗口设置在原生桌面 app 的设置 → 外观。`khor config set window-material solid|clear|frost` 与 `khor config set window-opacity 75` 使用同一设备配置，回到 app 时重读。默认实色；首次选择半透明或毛玻璃时为 90%，背景不透明度可调 75%–100%。只改主窗口背景；字、头像、状态、菜单和设置窗口仍清晰。macOS 与 Windows 使用支持的系统材质，Linux 使用桌面合成器透明背景，不提供原生毛玻璃入口。

## 代码边界

- `lib/term-theme.ts`：验证、设备存储、同步、xterm 主题解析。
- `lib/term-palettes.json`：纯色表数据，不与 app token 混用。
- `views/more/TerminalThemeSection.tsx`：选择、无主机 xterm 预览、DIY 和文件操作。
- `lib/window-surface.ts`：主窗口背景面、终端区域替换和原生窗口调用。输出/滚动不需要重新测量区域。
- `src-tauri/src/window_surface.rs`：原生启动背景与系统材质；启动 HTML 先得到原生配置，避免透明窗口先画成实色。

## 预设来源

第三方色表按其官方终端配置转换，保留原来的 ANSI 色号和颜色值。Catppuccin、Dracula、Nord 使用 MIT，Tokyo Night 使用 Apache-2.0；许可文本见 [TERMINAL-THEME-LICENSES.md](TERMINAL-THEME-LICENSES.md)。

- [Catppuccin Kitty](https://github.com/catppuccin/kitty) — Catppuccin contributors
- [Dracula Xresources](https://github.com/dracula/xresources) — Dracula contributors
- [Nord Xresources](https://github.com/nordtheme/xresources) — Sven Greb
- [Tokyo Night](https://github.com/folke/tokyonight.nvim) — folke

`term-palettes.json` 保留各预设的直接源地址。khor 自己的三种配色与「跟随」独立；终端主题接口参考 [xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/)。
