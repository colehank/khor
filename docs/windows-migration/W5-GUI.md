# W5：GUI / Web 桌面端

## 1. 目标与验收门

见 [ROADMAP](ROADMAP.md) §1 W5：Tauri 窗口在 Windows 启动、连接本地/远程节点、WebView2 正常；
前端构建在 Windows 可重复。验收门：冷启动到可交互、打开会话、终端面板可用，附截图/日志。

## 2. 环境约束

1. **路径不能含 `#`**：Vite 7 拒绝含 `#` 的工程根。本轮用无 `#` 的 worktree：
   ```powershell
   git worktree add -b w5-gui-check F:\Temp\khor-gui dbc30156
   ```
2. **本机没有 npm**（只有 node 与 pnpm）。GUI 的脚本写的是 `npm run gen`，且依赖 npm 的**提升布局**：
   `src/lib/highlight.ts` 直接 import `@shikijs/langs/*`、`@shikijs/themes/*`，`src/lib/md/file.ts` import `hast`，
   而 `package.json` 只声明了 `shiki`、`hast-util-*` —— 这些是 npm 提升出来的**幻影依赖**。
   pnpm 默认隔离布局解析不到，必须：
   ```powershell
   pnpm install --frozen-lockfile --node-linker=hoisted --shamefully-hoist --store-dir F:\khor-toolchain\pnpm-store
   node scripts/generate.mjs
   node_modules\.bin\tsc.cmd --noEmit
   node_modules\.bin\vite.cmd build
   ```
   注意：`apps/gui/.npmrc` 在本机 pnpm 11 里未生效（`pnpm config get node-linker` 仍为 undefined），
   所以用命令行参数；该文件只建在一次性 worktree，不进仓。

## 3. 结果（2026-11-15，Windows 11 x64）

| 步骤 | 命令 | 结果 |
| --- | --- | --- |
| 依赖 | `pnpm install --frozen-lockfile --node-linker=hoisted --shamefully-hoist` | 完成（558 包；esbuild 的构建脚本被 pnpm 拦下，但 vite 可用） |
| 生成 | `node scripts/generate.mjs` | exit 0，生成 `src/gen/bindings` |
| 类型 | `tsc --noEmit` | **exit 0** |
| 前端 | `vite build` | **exit 0**，34.68s，`dist/`（主 bundle 2,929.10 kB） |
| Tauri | `cargo build --locked -p khor-gui`（`CARGO_TARGET_DIR=F:\Temp\khor-target`） | **exit 0**，5m09s，`khor-gui.exe` |
| 冷启动 | `khor-gui.exe`，`KHOR_HOME`/`KHOR_RUNTIME_DIR` 指到 `.cache/w5` | 窗口出现：标题 Khor、会话侧栏、搜索、机器行「DESKTOP-1CC5MUC 本机」；WebView2 数据目录已建 |
| 会话可见 | 同 home 下 `khor open -d -- cmd.exe` → `shell/281f2d7c1c2f1718` | 会话落盘成功，但 GUI 列表 60s 内仍是加载骨架，未渲染该行 |

截图：`.cache/w5/screen.png`（首次冷启动）、`.cache/w5/screen2.png`（建会话后）。

## 4. 未通过 / 未测

### 4.1 终端读不到输出：已修复（2026-11-16）

**根因**：portable-pty 0.9.0 在 `CreatePseudoConsole` 里硬编码了
`PSUEDOCONSOLE_INHERIT_CURSOR | RESIZE_QUIRK | WIN32_INPUT_MODE`。其中
**`PSUEDOCONSOLE_INHERIT_CURSOR`（0x1）会让没有控制台的父进程收不到任何输出**：

```text
flags = 0 → 收到 "khor-conpty-ok"
flags = 2 → 收到
flags = 4 → 收到
flags = 1 → 什么都收不到
flags = 7（portable-pty 用的组合）→ 什么都收不到
```

GUI、服务、测试 harness 都没有控制台，所以 Windows 上终端面板与 replay 永远空白。

**修法**：按仓库既有 `[patch.crates-io]` 惯例 vendor 一份
`vendor/portable-pty`，只去掉这一个 flag（`vendor/portable-pty/KHOR.md` 记录原因与撤销方式）。

**验证**：

| 用例 | 结果 |
| --- | --- |
| `crates/node/tests/host_output_smoke.rs`（新增：宿主读到 `cmd /c echo` 的行） | **PASS 2.1s** |
| `cargo test -p khor-node --test resume` | **7 passed**（含远端终端重连 replay，`caught up in 263.6ms`） |
| `cargo test -p khor-node --lib` | **1061 passed / 0 failed / 14 ignored** |
| `cargo test -p khor-cli --test console --test quit --test ws` | 2 + 1 + 3 passed |

**残留**：ConPTY 不转发 `ESC [ ? 1 h`（DECCKM），所以 replay 头部没有该模式；
`host_replay` 的那条用例保持 `#[cfg(unix)]`，见 [W4-NETWORK](W4-NETWORK.md) §4。

### 4.2 其余
| 项 | 状态 | 说明 |
| --- | --- | --- |
| 会话列表渲染 | **通过**（2026-11-16） | 根因是 Tauri 的 custom-protocol IPC 在 Windows 上启动后即停摆，不是列表本身。改由窗口内 loopback 传输承担命令，详见 §6 |
| 打开会话 / 终端面板 | **通过**（2026-11-16） | 端到端证据见 §6.3：截图里是 cmd.exe 横幅、键入的 `echo khor-terminal-ok` 与其输出 |
| `npm ci` 路径 | **NOT_TESTED** | 本机无 npm；CI 用 npm，需在 CI 复核幻影依赖仍能通过 |
| 幻影依赖修复 | 未做 | 应在 `package.json` 声明 `@shikijs/langs`、`@shikijs/themes`、`@types/hast` 并重生成 `package-lock.json`（需要 npm，本机没有） |

## 5. 结论

- **构建链在 Windows 上可重复**（在 pnpm hoisted 前提下）：generate / tsc / vite / Tauri 全部 exit 0。
- **GUI 冷启动与渲染通过**：真实窗口、WebView2、侧栏与机器行都出现。
- **端到端通过（2026-11-16 补）**：会话列表渲染、实时更新、打开会话、终端面板输入输出全部验证，证据见 §6.3。当时的"端到端未通过"是两个传输层缺陷造成的，与列表/终端自身的代码无关。

## 6. 收口（2026-11-16）

### 6.1 现象与排除

窗口能画机器仪表盘，但会话列表永远停在骨架屏，页面无任何报错。逐条排除（每条都有实测）：

| 假设 | 结论 | 证据 |
| --- | --- | --- |
| 后端查询慢/失败 | 排除 | `list_sessions` 5 种排列各 80–120ms；`POST /api/sessions` → 200 |
| `RootPermit` 进程内自锁 | 排除 | 同进程第二个 `Node` 126ms 应答（`apps/gui/core/tests/self_open_probe.rs`） |
| tokio 阻塞池饱和 | 排除 | `off_main` 追踪：进入 14ms、完成 310ms、正常结算 |
| 窗口最小化节流 | 非根因 | 恢复窗口后 `document.hidden` 变 `false`，IPC 仍然死 |
| WebView2 profile 损坏 | 排除 | 全新 profile 表现一致 |
| Tauri IPC 队列上限 | 不存在 | 通读 `tauri-2.11.5/src/ipc/`，无 pending map、无计数器 |
| DevTools 可判断 IPC | 假象 | 隔离世界收不到 `invoke` 回调，DevTools 发起的 invoke 永远 pending |

### 6.2 两个真根因

**其一：custom-protocol IPC 停摆。** 在 `invoke_handler` 首行加逐条追踪，每次运行都只到达 9 条命令，之后永远为空；页面同时发 38 次 `sessions`，Rust 一次没收到，两边都无报错。决定性实验：页面直接 `fetch('http://ipc.localhost/lock_state')`——不经过 invoke、不涉及命令——**40 次全部超时**。缺陷在 wry 的 `WebResourceRequested` deferral（`wry-0.55.1/src/webview2/mod.rs:992/1008`）：响应要靠 `PostMessageW` 跳回主线程，该跳不执行时 deferral 永久泄漏，Chromium 每源 6 条连接占满。上游无修复版本可升（[tauri#15435](https://github.com/tauri-apps/tauri/issues/15435) 同类问题维护者明确称暂无解法）。

**其二：资源协议同样停摆。** 命令改走 loopback 后列表正常了，但打开会话时终端面板空白、一条 `term_*` 都不发。终端是懒加载的（`views/DetailPane` `lazy(() => import("@/views/TerminalPane"))`），其 chunk 永远载不进来：从运行中的页面 `fetch('/assets/...')` 同样不返回，而 `Suspense fallback={null}` 把它变成一块空白，无声无息。

### 6.3 修法与验证

- **`apps/gui/src-tauri/src/window_transport.rs`（新增）**：在应用进程内起 loopback HTTP 服务（端口 1432），分派到与 dev bridge、web face **同一张** `khor_gui_core::api::answer` 表（190 条命令）。窗口改为加载 `index.html?bridge=1432`，前端 `call()` 的首个分支（`api.ts`）直接走本机 HTTP，死的 custom protocol 不再被触碰。事件、窗口控制、拖拽、原生菜单是 Tauri 自己的 API，不受影响。
- **`apps/gui/vite.config.ts`**：`build.rollupOptions.output.inlineDynamicImports = true`，把懒加载模块并入首包，页面不再需要在启动后取任何 chunk。代价：首包 2.9 MB → 14 MB，只在启动时付一次。

| 用例 | 结果 |
| --- | --- |
| `cargo test -p khor-gui-core --test list_sessions_probe` | **1 passed**（recent/category/device/state/place，80–120ms） |
| `cargo test -p khor-gui-core --test self_open_probe` | **1 passed**（126ms） |
| `cargo build -p khor-gui`（`CARGO_TARGET_DIR=F:\Temp\khor-target`） | **exit 0** |
| 窗口启动日志 | `khor: window transport on 127.0.0.1:1432` |
| 会话列表 | **0 骨架**；新建会话数秒内出现（实测 `shell/c7d0561afec4a17f`） |
| 打开会话后到达 Rust 的终端命令 | `term_open`、`term_attach`、`term_ready`、`term_resize`、`term_key`、`term_wait ×448` |
| 终端 DOM | 3 canvas、1 `.xterm-helper-textarea`、1 `.xterm`，seat 子节点 23 |
| 终端画面（`Page.captureScreenshot`） | cmd.exe 横幅、`F:\...>echo khor-terminal-ok`、输出 `khor-terminal-ok`、新提示符与光标 |

### 6.4 残留

- 首包 14 MB 是本次修法的代价；若上游修好资源协议，应回退这一条并改回代码分割（文档已写在该配置项注释里）。
- `?bridge=` 意味着窗口命令走 loopback；`term_drop_stage` 等 raw-body 路由已按 dev bridge 的分派实现，但**拖放文件到终端**未在 Windows 上实测（NOT_TESTED）。

## 7. 试用的正确姿势（2026-11-16 更正）

**不要设 `KHOR_HOME`。** 默认 store 根就是用户主目录本身——`Node::store_under` 在本机拥有
`<root>/.khor/owner` 时直接返回该 root（`crates/node/src/lib.rs`）——而 `usage::meters_for` 用的
`adaptor::vendor_home(&root)` 默认等于 root。于是默认运行时：

- `vendor_home` = `~` → `~/.codex`、`~/.claude` 被真正读到；
- 额度页的 token 图表、模型明细、缓存占比全部有数据；
- agent 识别（`recognition`）也能看到本机真实安装的 Codex。

反过来，把 `KHOR_HOME` 指到一个隔离目录会让 `vendor_home` 变成那个目录，于是：

- `khor usage` 报「No spend these past few days」；
- 额度页只有订阅额度、没有 token 图表；
- `doctor` 的 `agents` 说「no agent registered」。

**这不是 Windows 的缺失，是隔离 store 的应有行为**（macOS 用隔离 store 同样为空）。
早前交给用户的试用命令带上了 `KHOR_HOME=F:\Temp\khor-try\home-nohash`，因此看起来像功能缺失。

### 7.1 证据

| 项 | 结果 |
| --- | --- |
| `cargo test -p khor-node --test real_disk khor_can_read_every_spending_record_on_this_machine -- --ignored` | **1 passed**，读到 10-31 → 11-16 的真实记录，`one cold pass` 12.14s |
| `khor usage --days 3`（默认 store） | 报出 11-16 / 11-15 / 11-14 的 Input/Output/Cache 与模型分类 |
| `khor usage --days 7`（隔离 store） | 「No spend these past few days」 |
| `khor usage --days 7`（隔离 store + `KHOR_VENDOR_HOME=~`） | 与默认 store 相同的完整数据 |
| GUI 额度页（默认 store，截图） | 7% 仪表、剩余 49,300、按天柱状图（11-09 峰值 502.3M、中位 28.9M）、模型散点、明细表 **2.1B 总量 / 98.4% 缓存占比 / 4 模型 / 30 天** |

GUI 额度页与 macOS 截图同一套内容，确认**功能对等，不是缺失**。

## 8. 会话形态

用户指出会话应当参照 Codex / DSH Desktop 的对话形态，而不是只有终端。仓库已有
`views/ChatView`、`MachineChat`（`views/DetailPane` 里按会话类型分派）。**Windows 上已验证可用**：

| 步骤 | 命令 / 观察 | 结果 |
| --- | --- | --- |
| 识别本机 agent | `khor agents known` | ACP 注册表 40 个、22 个可启动；本机装有原生 `codex.exe` |
| 登记 ACP 桥 | `khor agents add codex-acp2 -- cmd.exe /c <pnpm.cmd> dlx @agentclientprotocol/codex-acp` | `Registered: codex-acp2`，并报「says it has 32 conversations」 |
| 开会话 | `khor open -d --agent codex-acp2 --title acp-probe` | `tui/01a1e489-…`，会话出现在列表 |
| GUI 形态 | 点该行 | **对话视图**：标签页、`连接待确认`、`模型设置`、会话记录区、`对话已结束` |

**Windows 上的两个前置条件**（都是环境而非 Khor 缺陷）：

1. **`.cmd` 批处理需要显式解释器**。直接登记 `pnpm.cmd` 会被拒，错误信息明确：
   「batch sidecars require an explicit command interpreter」。前缀 `cmd.exe /c` 即可。
2. **ACP 桥要跑 npm 包**，本机有 pnpm 无 npm/npx，用 `pnpm dlx` 等效替代。

### 8.1 「宿主被结束」是缺常驻进程，不是缺陷

`khor open -d` 建的会话在 **没有常驻 `khor serve`** 时，宿主是那个短命 CLI 的子进程，
CLI 退出即 `Failed / 宿主被结束`。实测对照：

| 前置 | `khor open -d -- cmd.exe` 之后 | `khor sessions` |
| --- | --- | --- |
| 无常驻 | CLI 退出后 | `Failed` |
| 先起 `khor serve` | CLI 退出后 | **`Idle`（存活）** |

GUI 自己带常驻，所以从应用里新建的会话不会这样。

## 9. Windows 窗口自身（2026-11-16 修）

### 9.1 两重窗口控制栏

用户截图发现顶部有两组最小化/最大化/关闭。病根：`components/Region` 的 `WindowControls`
**只在 Windows 渲染**（注释写「Windows has no buttons of its own over it」），但窗口构建时
没有 `decorations(false)`，于是系统标题栏与页面自绘控件同时出现。

修法（`build_the_window`）：

```rust
#[cfg(target_os = "windows")]
{
    win = win.decorations(false).shadow(true);
}
```

macOS 仍走 `TitleBarStyle::Overlay`，Linux 保留系统装饰，两者不进这段。

### 9.2 自绘控件在 loopback 传输下失效（同批修）

页面那三个按钮走 `call("window_minimize")` 等。改走 `?bridge=` 后它们被送进
`api::answer`——而那是**存储**的命令表，没有窗口方法，于是回 400：

```
window_minimize -> 400: "no such command: window_minimize"
```

修法：`window_transport` 先认这五个命令，转给真实的 Tauri 窗口
（`window_transport::window_verb`，窗口构建时 `attach` AppHandle）：

| 命令 | 传输层答复 |
| --- | --- |
| `window_minimize` | `window.minimize()` |
| `window_maximize_toggle` | `is_maximized()` 后 maximize/unmaximize |
| `window_close` | `window.close()`（close 而非 destroy，草稿守卫挂在 close 上） |
| `traffic_room` | `0`（Windows 标题行没有系统按钮占位） |
| `window_appearance_support` | `{"supported":false,"blur":false}` |
| `set_window_theme` / `set_window_appearance` | `null`（窗口由页面绘制，无需着色） |

验证：

| 用例 | 结果 |
| --- | --- |
| `POST /traffic_room` | 200 `0` |
| `POST /window_appearance_support` | 200 `{"supported":false,"blur":false}` |
| `POST /window_maximize_toggle` ×2 | 200 `null`，窗口最大化后再还原 |
| `POST /window_minimize` | 200 `null`，窗口最小化 |
| 点页面自绘的最小化按钮 | 窗口 `IsIconic` 0 → 1，恢复后回 0（端到端） |
| 截图 | 只剩一组控件，无系统标题栏 |

### 9.3 无边框窗口无法拖动（同批修）

Tauri 自己的 `drag.js`（`tauri-2.11.5/src/window/scripts/drag.js:104`）在
`[data-tauri-drag-region]` 上按下时调
`window.__TAURI_INTERNALS__.invoke('plugin:window|start_dragging')`——正是那条会停摆的
custom-protocol 路。装饰去掉后窗口没有别的地方可抓，于是**完全拖不动**（用户 2026-11-16 报告）。

修法：`window_transport::drag_shim` 在 **capture 阶段**接管 mousedown，按 Tauri 自己的区域规则
（clickable 元素阻挡、`"false"` 禁用、`"deep"` 整个子树可拖、裸/`"true"` 只认直接目标）判断，
然后走 loopback 调新增的 `window_drag` 动词（→ `window.start_dragging()`）；
双击则调 `window_maximize_toggle`。capture + `stopPropagation` 是为了不让 Tauri 那个监听器
再跑一遍（它会发一个永远不回的 invoke，还会与双击最大化打架）。

验证：

| 用例 | 结果 |
| --- | --- |
| 标题行 `elementFromPoint` | `DIV`，`data-tauri-drag-region="deep"` |
| CDP 在该点按下 | 传输层收到 `[transport] window_drag` → `start_dragging()` |

> 说明：真实拖动需要真实指针，本机无法用 CDP 伪造（`start_dragging` 用的是 OS 光标状态）。
> 这里证明的是**接线**：页面正确识别区域并把拖动交给窗口。真机手拖属于人工验收项。

### 9.4 打开 Khor 伴随一个控制台窗口

**debug 构建才有，release 没有**。`main.rs` 第 1 行就是
`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`，实测 PE 头：

| 二进制 | 子系统 |
| --- | --- |
| `release/khor-gui.exe` | `WINDOWS_GUI`（2，无控制台） |
| `debug/khor-gui.exe` | `CONSOLE`（3，带控制台） |

所以给用户的试用要用 **release** 或安装包；debug 是给调试看的。




