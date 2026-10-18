# 正式终端配色实现与验收（2026-10-19）

分支 `feat/terminal-colors`，worktree `/Volumes/disk0/khor-terminal-color-lab`。

## 实现

- `terminalAppearance` 统一输出 xterm 的主题与最低对比度。
- 跟随 khor 保持 khor 背景、前景、光标，ANSI 16 色按明暗分别使用现有 Latte / Mocha。
- 增加自动 Catppuccin 和 Tokyo 家族；旧固定预设、DIY 的选项身份与颜色保持固定。
- 可读性保护按观看设备独立保存，旧配置默认标准 4.5；关 1、轻度 3、加强 7。
- 同一个外观通知负责当前／保留终端、结束快照、tmux 历史层和设置预览。
- 自定义取当前实际版本，预览隔离，保存与删除配色保留独立保护选项；JSON 仍只表达颜色。
- 主机颜色应答、进程启动参数和 PTY 字节未修改；旧 RGB 背景保留。

## 自动检查

新增 `npm run terminal-colors`，并纳入 `npm test`。旧代码在缺少统一解析器时先红。
覆盖默认／关闭／非法配置、旧数据、配对／固定／DIY、克隆、扩展色表、重载、跨窗口与背景面同步。

`npm test --ignore-scripts`、`npm run build --ignore-scripts`、lab tsconfig、`npm run gates` 通过。
已有 appearance 浏览器门补上相关断言并通过语法检查；本轮通过 CUA 操作真实组件验收，
没有整段运行 `check-appearance.mjs`，没有进行原生平台运行时或透明窗口的合成后验收。

## 可重复的真实组件验收

```sh
cd apps/gui
./node_modules/.bin/vite --config lab/terminal-colors-production.vite.ts
```

主界面 <http://localhost:23624/lab/terminal-colors-production.html?bridge=23624>

独立设置窗口 <http://localhost:23624/lab/terminal-colors-production.html?bridge=23624&settings=1>

这两个入口使用正式 `TerminalPane`、`EndedPane`、`TerminalThemeSection` 与真实 xterm，
transport 返回受控输出。tmux 历史页直接使用生产 `holdsTmuxHistory`，读数据是本地夹具。
不连接实际 khor 节点，不创建或修改用户会话。颜色选择仅保存到独立端口的浏览器存储。

工具栏「检查连接」在页面内读取真实 xterm 的选项、实例身份、缓冲、滚动与 RGB 单元格，
将结果写入可展开的验收数据；网络计数用于检查外观变化有没有导致重新 attach。

### Chrome 实际操作记录

- 浅色跟随：背景 `#f4f4ef`，ANSI red 为 Latte `#d20f39`，标准保护 4.5。
- 向上滚动 30 行，再选自动 Catppuccin、保护 7、切深色：当前、保留、结束三个实例身份、
  缓冲长度和滚动位置均保持；主题变 Mocha，保护均为 7，没有新增 attach。
- 从独立设置页选择自动 Tokyo、保护 3：已有三个视图都更新。
- 固定 Nord 在浅色观看端仍为 Nord；自定义修改背景未保存时主窗口仍为 Nord，取消后不变。
- 独立设置页关闭保护后重载仍是关，三个已有视图的选项均为 1。
- tmux 历史层使用生产控制器，初始 202 行，切深色自动 Tokyo、保护 7 后仍 202 行、
  原滚动位置、原实例；其他三种终端一起更新。没有再读历史就能换色。
- 390 × 844 下，文档宽度为 390，两个选择器右缘均为 370，选择保护强度可用；恢复桌面视口后仍为标准。
- 所测提示行的 RGB 背景单元格始终为 `0x1f1f1f`，真实颜色模式始终为 RGB。

实验关注可读性、状态与配置传播。真实 Codex 的主题缓存／tmux 颜色探测归属仍是后续独立问题；
本轮不会因主题变化重启程序，也不会把背景 RGB 重写成另一种颜色。
