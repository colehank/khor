# 终端配色 lab

入口：<http://localhost:23623/lab/terminal-colors.html>

```sh
cd apps/gui
./node_modules/.bin/vite --config lab/terminal-colors.vite.ts
```

分支 `lab-terminal-colors`，外挂盘 worktree `/Volumes/disk0/khor-terminal-color-lab`。
仅增加 lab 文件；正式主题、主机颜色应答和用户会话均未改动。
没有启动 Codex 或 PTY，不调用后端 API，也不保存设置。

## 对比

三个终端使用仓库实际安装的 xterm 6、FitAddon 与可选 WebGL addon：

1. 原有跟随模式的对照：khor 底色 + 旧 ANSI 16 色表。实现后仍保留此历史基线。
2. 现有上游预设 Catppuccin Latte / Mocha，或 Tokyo Day / Night，完整色表不改值。
3. 同一套预设加 xterm 的 `minimumContrastRatio`，可选关闭、3、4.5、7。

不是给 Codex 加特例。输出是受控 ANSI 夹具：默认前景、ANSI 16、256 索引、显式 RGB
前景和背景、DIM、反色、下划线。Hook failed 是复刻截图所需的静态文字，没有运行 hook。
输入框只读；xterm 的 onData 仅在本页捕获颜色查询回复，不接任何主机。

- **沿用深色端的输出**：三个终端消费相同字节，程序背景固定为从黑色合成的 RGB。
- **按当前背景重新绘制**：分别发出 OSC 10/11 查询，使用各终端真实回复重绘夹具。
  这是程序主动重绘后的理想情况，不表示 khor 可以重写真实进程的主题缓存。
- **通用颜色样本**：额外检查 ANSI、256、成对 RGB、DIM 和反色。

背景合成复用 Codex `style.rs` 的算法形状：浅底叠黑 4%，深底叠白 12%。
参考 <https://github.com/openai/codex/blob/main/codex-rs/tui/src/style.rs>。
本实验复现“默认文字随观看端变化，而显式 RGB 背景保持旧值”的机制，
未捕获用户截图中 session 的原始输出，不能据此确定那张截图的唯一根因。

## 测量与边界

DOM 模式直接读取 xterm 实际绘制文字的 computedStyle，结合显式背景计算 WCAG 对比度；
WebGL 模式只展示输入颜色的原始对比度，不把 DOM 样式误当成 GPU 实绘测量。

2026-10-18，Chrome，浅色端观看旧深色输出，提示行实测：

| 方案 | 原始 | 实绘 |
| --- | ---: | ---: |
| 现有跟随 | 1.06:1 | 1.06:1 |
| Catppuccin Latte | 2.06:1 | 2.06:1 |
| Latte + 4.5 保护 | 2.06:1 | 5.02:1 |

关闭保护后回到 2.06:1；重新按浅色背景绘制后，Latte 提示行为浅灰底，实绘 6.45:1。
这些数只描述所测提示行，不代表整屏最差值。xterm 对 DIM 采用目标的一半；部分图形字形
排除对比度修正。半透明背景也需要另做合成后验证。

## 生产应答机制实验

`terminal-color-answers.rs` 直接引用生产的 `answers.rs` / `modes.rs`，无复制的账本实现：

```sh
rustc --edition=2024 --crate-name terminal_color_answers lab/terminal-color-answers.rs -o /Volumes/disk0/khor-checks/terminal-colors/answers-probe
/Volumes/disk0/khor-checks/terminal-colors/answers-probe
```

深色先回、浅色先回两臂均确认：第一套 OSC 10/11 回复到达程序，另一套被丢弃；
无人回复时 fallback 继续使用此前被接受的颜色。正式 `answers::tests` 9 项通过。
该实验不覆盖真实网络时序、tmux 自身颜色缓存、Codex 的原生启动和主题通知处理。

## 已验收

- lab 独立 tsconfig 类型检查通过。
- Chrome 实际操作：浅深主题、两组成对预设、保护开关、重新探测、三种输出场景。
- DOM 数值与实际前景修正一致；WebGL 加载及视觉对比通过，没有用 DOM 数值冒充 GPU 测量。
- 390 × 844 窄屏：方案可切换，document.scrollWidth = innerWidth = 390，无横向溢出。
- 使用隔离端口、无后端；用户会话、主题设置和 main 不受影响。

## 建议的正式实现顺序

1. 使用已有成熟浅深配对预设，保留手动预设与 DIY；“跟随 khor”是否保留 app 原底色需明确。
2. 将可读性保护纳入同一个终端配置解析器，覆盖在线终端、保留终端、快照和设置预览。
   修正的是呈现，不能改 PTY 原字节或把所有 RGB 背景一律洗掉；默认强度需真机验证。
3. 调查多端应答归属、主题通知与 tmux 缓存；明确历史 RGB 输出不会因换观看端自动重绘。
4. 原生 iOS、macOS、Windows、Linux 和透明窗口做实际输出验收，再决定发布。
