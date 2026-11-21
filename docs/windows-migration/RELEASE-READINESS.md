# W8 · 发布就绪包

本文件把 W0–W7 的成果整理成可交付物：能做什么、怎么装、已知限制、证据在哪、还差什么。
**合 main 与发布由用户决定**；本分支不 push、不签名、不发布（`ROADMAP.md` §2.7）。

## 1. 里程碑状态

| | 内容 | 状态 |
| --- | --- | --- |
| W0 | 基线 | ✅ |
| W1 | 构建 / 诊断 / CI 基线 | ✅ |
| W2 | node 层（`1061 passed / 0 failed / 14 ignored`） | ✅ |
| W3 | 进程 / 终端 / 文件 smoke（12 项） | ✅ 含例外：T5 attach 记 NOT_TESTED |
| W4 | 网络 + Session（26 项，双 node 回环） | ✅ 含例外：两条 ConPTY replay NOT_TESTED |
| W5 | GUI / Web | ✅ **2026-11-16 收口**，见 [W5-GUI](W5-GUI.md) |
| W6 | 打包 / 更新 / 回滚 | ✅ 含例外，见 [W6-PACKAGING](W6-PACKAGING.md) |
| W7 | 性能矩阵 / Win10 | ✅ 矩阵；Win10 NOT_TESTED，见 [W7-PERF](W7-PERF.md) |
| W8 | 本文件 | ✅ |

### 1.1 W5 收口时修的三件事

1. **会话列表不渲染**——不是列表的问题，是 Tauri 的 custom-protocol IPC 在 Windows 上启动后即停摆
   （每次运行只到达 9 条命令，裸 `fetch` 到 `ipc.localhost` 40/40 超时）。改为窗口内 loopback 传输。
2. **打开会话后终端面板空白**——资源协议同样停摆，懒加载的 `TerminalPane` chunk 永远载不进来。
   改为分块 + 启动期 `modulepreload` 预取。
3. **窗口自身**——两重标题栏（缺 `decorations(false)`）、自绘控件失效（`window_minimize` 等不在
   `api::answer` 里）、无边框后拖不动（Tauri 的 `drag.js` 走的是失效的 invoke）。

### 1.2 会话稳定性（2026-11-16 修）

Windows 上的会话宿主从未 detach——unix 有 `setsid()`，Windows 分支缺失。于是宿主是开启者的普通子进程：
CLI 退出、`khor serve` 停止都会带走它，会话回来就是 `Failed / 宿主被结束`，且**永久**无法恢复。

修法：终端宿主（`host.rs`）与对话宿主（`gui_host.rs`）都加
`DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`；并处理了两个 flag 写入者互相覆盖的问题。

实测（无任何常驻）：CLI 退出后会话保持 `Idle`，12 秒后仍 `Idle`，宿主进程独立存活。对话会话同样。

## 2. 三档试用

| 档 | 状态 | 怎么用 |
| --- | --- | --- |
| **CLI** | ✅ | `khor.exe`（release）。`khor --help`、`khor sessions`、`khor open -d -- cmd.exe`、`khor usage` |
| **GUI 本机** | ✅ | 见 §2.1 |
| **安装包** | ✅ NSIS | 见 §2.2 |

### 2.1 GUI

**不要设 `KHOR_HOME`。** 默认 store 根就是用户主目录，`vendor_home` 因此指向它——额度页才有 token 数据、
`doctor` 才认得出本机安装的 agent。设了隔离 store 会让这些看起来"没实现"（见 [W5-GUI §7](W5-GUI.md)）。

```
F:\Temp\khor-target\release\khor-gui.exe
```

已实测：会话列表渲染并实时更新、打开会话、终端输入输出（截图里有 cmd.exe 横幅与命令回显）、
额度页 token 图表（中位/峰值/模型明细/缓存占比）、窗口自绘控件与拖动、无控制台窗口。

> 用 **release**，不要用 `debug`：debug 是 CONSOLE 子系统，会带一个控制台窗口。

### 2.2 安装包

```
F:\Temp\khor-target\release\bundle\nsis\Khor_0.2.0_x64-setup.exe   (35.65 MiB, per-user)
F:\Temp\khor-target\release\bundle\msi\Khor_0.2.0_x64_en-US.msi    (49.31 MiB, per-machine)
F:\Temp\khor-target\release\bundle\msi\Khor_0.2.0_x64_zh-CN.msi    (49.31 MiB, per-machine)
```

NSIS：安装 → 启动（1.1 s 到可交互）→ 卸载 **6/6 通过**；升级到 0.2.1 → 回滚到 0.2.0 **8/8 通过**。
MSI 需要管理员权限（`Error 1925, ALLUSERS=1`），本席位无法提权 → NOT_TESTED。

## 3. 已知限制

| 限制 | 影响 | 说明 |
| --- | --- | --- |
| 首包仍 2.79 MB 入口 | 冷启动约 0.7 s 到首屏 | 已从 1280 ms 降到 664–712 ms；剩余优化清单见 [W7-PERF §5](W7-PERF.md) |
| 安装后主程序名为 `khor-gui.exe` | 显示名是 "Khor"，文件名不是 | 改 `mainBinaryName` 需同步改 cargo bin 名，牵动全部构建与既有证据 |
| MSI 需提权 | 普通用户装不了 MSI | 用 NSIS（per-user） |
| 应用内更新器不可用 | 只能靠安装包升级 | 需要签名授权；安装器级升级已验证可用 |
| 已 `Failed` 的会话不会自愈 | 旧会话仍是 Failed | 修复只保证今后不再被误杀；shell 无恢复路径 |
| 按进程名杀会带走宿主 | `taskkill /IM khor.exe` 会终止会话 | 升级/卸载流程需注意；unix 侧已在 `docs/SHIP.md:80` 记录 |
| 首包预载 250 个 chunk | 启动时流量换解析延迟 | 若上游修好资源协议，应改回资源走 loopback，见 [W7-PERF §5.5](W7-PERF.md) |

## 4. NOT_TESTED 归属

每一项都必须有人认领，不允许"跳过当通过"。

| 项 | 为什么没测 | 谁 / 下一步 |
| --- | --- | --- |
| MSI 安装 → 启动 → 卸载 | per-machine 需管理员，本会话审批被禁用 | 有管理员权限的机器执行 `scripts/windows-installer-check.ps1 -Installer <msi>` |
| 应用内签名更新器 | `crates/update` 需签名产物；签名未授权 | 发布流程拿到签名密钥后验证 |
| Windows 10 x64 | 本席位只有 Win11 | 一台 Win10 机器跑 `windows-smoke.ps1` + `windows-perf.ps1` |
| 手工拖动窗口 | 需要真实指针；CDP 无法伪造 `start_dragging` 的 OS 光标 | 人工验收：拖标题行 |
| 拖文件进终端 | raw-body 路由已实现，但未在 Windows 实测 | 人工验收：拖一个文件到终端 |
| `npm ci` 路径 | 本机无 npm | CI（用 npm）复核幻影依赖 |
| **macOS 回归** | 本席位无 Mac | Mac 上跑 `check-boot.mjs`、`check-native-events.mjs`、`scripts/web-face.mjs`。**必看**：本次共享的 `dist` 增加了 250 个 `modulepreload`，Windows 专用改动已用 `cfg(target_os)` 收窄，但这一条影响所有平台 |
| 双物理机配对 | 只有一台 Windows + 一台 Linux（LAB 已装 Rust，可作第二节点） | 用 LAB 做一次真实配对/重连 |
| `attach_version` / 两条 ConPTY replay | 见 [W3](W3-SMOKE.md) / [W4](W4-NETWORK.md) | 上流行为差异，已记原因 |

## 5. 证据索引

| 片 | 命令 | 退出码 | 结果 |
| --- | --- | --- | --- |
| node lib | `cargo test --locked -p khor-node --lib` | 0 | 1061 passed / 0 failed / 14 ignored |
| 会话列表后端 | `cargo test -p khor-gui-core --test list_sessions_probe` | 0 | 5 种排列 80–120 ms |
| 根许可自争用 | `cargo test -p khor-gui-core --test self_open_probe` | 0 | 126 ms |
| 宿主回归 | `cargo test -p khor-node --test host_output_smoke --test takeover_safety --test resume` | 0 | 1 + 1 + 7 passed |
| GUI 构建 | `cargo build -p khor-gui`（`CARGO_TARGET_DIR=F:\Temp\khor-target`） | 0 | — |
| 安装包 | `cargo-tauri build --config tauri.windows-build.conf.json` | 0 | 3 个产物，见 §2.2 |
| NSIS 安装\卸载 | `pwsh scripts/windows-installer-check.ps1 -Installer <nsis>` | 0 | 6/6 |
| 升级\回滚 | `pwsh scripts/windows-upgrade-check.ps1 -Base <0.2.0> -Newer <0.2.1>` | 0 | 8/8 |
| 性能矩阵 | `pwsh scripts/windows-perf.ps1 …` / `bash scripts/windows-perf.sh …` | 0 | 见 [W7-PERF §1](W7-PERF.md) |

## 6. 合并建议

**建议合入 main，但先做两件事**：

1. **在 Mac 上跑一遍回归**（§4 最后两项里标"必看"的那条）。本次为了 Windows 改变了共享的构建产物
   （分块 + 预载），虽然 Windows 专用逻辑都用 `cfg(target_os)` 收窄了，但 `dist` 是两个平台共用的。
2. **认领 NOT_TESTED 表**：MSI、Win10、签名更新器三项需要一个明确的所有者，否则发布时会被当成"已完成"。

风险最低的合并顺序：先合 W5 的传输与稳定性修复（纯 Windows 路径，`cfg` 收窄），
再合 W6 打包配置与脚本，最后合 W7 的文档与性能脚本。
