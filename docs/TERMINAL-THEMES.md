# 终端配色与桌面窗口背景

设置 → 终端提供「跟随 khor」、柔和深色、暖浅色、高对比度、Catppuccin Mocha/Latte、Dracula、Nord、Tokyo Night/Day。配色属于观看设备，远程会话不被修改。

自定义可以编辑或从预设创建配色，命名后保存。使用 HEX 颜色；选区支持带 alpha 的 HEX，导入也接受选区 rgba。ANSI 16 色与从 16 开始的最多 240 个扩展色可以独立指定。扩展色留空使用标准色表。程序输出的 RGB 真彩色保留。

导入接受原生 xterm 颜色对象或 `{ "name": "我的配色", "theme": { ... } }`；未填写的字段从当前配色补齐。导出生成同一结构：桌面 app 使用系统保存面板，网页下载 JSON，手机复制 JSON。文件上限 100 KB，自定义配色最多 50 个。删除当前配色后回到跟随 khor。

窗口设置在原生桌面 app 的设置 → 外观。`khor config set window-material solid|clear|frost` 与 `khor config set window-opacity 0` 使用同一设备配置，回到 app 时重读。默认实色；首次选择半透明或毛玻璃时为 90%，背景不透明度可调 0%–100%。只改主窗口背景；字、头像、状态、菜单和设置窗口仍清晰。macOS 的毛玻璃将窗口背后的模糊与网页调色分开，不再使用自带灰色的系统材质；运行时接口不可用时回到系统材质。Windows 使用支持的系统材质，Linux 使用桌面合成器透明背景，不提供原生毛玻璃入口。透明窗口关闭原生投影，防止 0% 时阴影沿字和按钮轮廓生成脏边；实色窗口保留原生投影。

程序指定的黑色工具块、RGB 背景、ANSI 背景和反色背景同样遵循设备的背景不透明度；RGB 与 ANSI 色值不被改写。每块区域只画一次，避免默认背景与程序背景叠成更厚的底。文字、块光标和选区保留各自的可读性，WebGL、丢失 GPU 上下文后的 DOM 备用渲染和结束快照采用相同规则。

## 代码边界

- `lib/term-theme.ts`：验证、设备存储、同步、xterm 主题解析。
- `lib/term-palettes.json`：纯色表数据，不与 app token 混用。
- `views/more/TerminalThemeSection.tsx`：选择、无主机 xterm 预览、DIY 和文件操作。
- `lib/term-glass.ts`：xterm 6 / addon-webgl 0.19 的背景绘制适配；主题保留实色原值，透明度只作用于背景绘制。跟随渲染器替换，重连 GPU 不丢规则；升级 xterm 时先跑 `npm run appearance`。
- `lib/window-surface.ts`：主窗口背景面、终端区域替换和原生窗口调用。输出/滚动不需要重新测量区域。
- `src-tauri/src/window_surface.rs`：原生启动背景与系统材质；启动 HTML 先得到原生配置，避免透明窗口先画成实色。

## 验收

`npm run appearance` 在同一份 ANSI 输出上检查 0%、50%、85%、100%：默认底、黑色底、RGB、ANSI、反色、白色前景和 0% 时仍可见的选区；WebGL、真实上下文丢失后的 DOM、GPU 恢复、1x/2x 与结束快照。修改配色仍保留同一 PTY、缓冲和滚动位置。

macOS 原生验收使用私有 home 和纯色 / 细条纹背景，确认 50% 只调色一次、条纹确实被模糊、0% 无灰底与轮廓投影。Windows / Linux 的原生材质仍需各自真机验收。

实现参考 [iTerm2 的背景区域绘制](https://github.com/gnachman/iTerm2/blob/069b6e0dd8f3f0d82fc08ea5d294d28f7c5b986e/sources/Drawing/iTermTextDrawingHelper.m)及 [窗口模糊调用](https://github.com/gnachman/iTerm2/blob/069b6e0dd8f3f0d82fc08ea5d294d28f7c5b986e/sources/TerminalView/iTermWindowImpl.m)，khor 的适配实现独立编写。

## 预设来源

第三方色表按其官方终端配置转换，保留原来的 ANSI 色号和颜色值。Catppuccin、Dracula、Nord 使用 MIT，Tokyo Night 使用 Apache-2.0；许可文本见 [TERMINAL-THEME-LICENSES.md](TERMINAL-THEME-LICENSES.md)。

- [Catppuccin Kitty](https://github.com/catppuccin/kitty) — Catppuccin contributors
- [Dracula Xresources](https://github.com/dracula/xresources) — Dracula contributors
- [Nord Xresources](https://github.com/nordtheme/xresources) — Sven Greb
- [Tokyo Night](https://github.com/folke/tokyonight.nvim) — folke

`term-palettes.json` 保留各预设的直接源地址。khor 自己的三种配色与「跟随」独立；终端主题接口参考 [xterm ITheme](https://xtermjs.org/docs/api/terminal/interfaces/itheme/)。
