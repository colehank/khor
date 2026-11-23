# Windows 迁移 W0 功能矩阵

本矩阵以当前 `steward/s2-windows-20261114` 分支 HEAD 的源码、既有文档和发布/CI 配置为依据。状态是“W0 基线判断”，不是 Windows 实机验收结论。

## 1. 判定口径

- **REQUIRED**：Windows 10/11 x64 上承诺可用的用户能力。
- **SHIPPED**：现有发布配置或产品文档明确承诺多平台发布的能力；不代表 Windows 已构建。
- **IMPLEMENTED_UNRELEASED**：源码已有跨平台分支或 Windows 目标，但发布矩阵未包含 Windows。
- **HISTORICAL**：设计或历史文档提及，但当前源码/发布配置未承诺。
- **PLATFORM_SPECIFIC**：只在某平台有意义的实现或限制。
- **EVIDENCE_GAP**：源码入口存在，但缺少 Windows 真实构建/运行证据。

所有“源码存在”均不等于 Windows 已通过；真实证据必须在 W1 及以后补充。

## 2. 能力矩阵

| 领域 | 能力 | Windows 状态 | REQUIRED | 证据入口 | 缺口 |
| --- | --- | --- | --- | --- | --- |
| CLI | `khor` 命令入口 | IMPLEMENTED_UNRELEASED | 是 | `crates/cli`、`crates/commands` | Windows 构建、PATH 安装、shell 补全 |
| GUI | Tauri 桌面窗口与 WebView 前端 | IMPLEMENTED_UNRELEASED | 是 | `apps/gui/src-tauri`、`apps/gui/src` | Windows 打包、WebView2 运行时 |
| Web | `khor web` 内嵌前端 | IMPLEMENTED_UNRELEASED | 是 | `crates/web/build.rs`、`apps/gui/dist` | 前端 dist 生成链在 Windows 的可用性 |
| Runtime | 本地节点、资源、路径和身份 | IMPLEMENTED_UNRELEASED | 是 | `crates/runtime` | `cfg(windows)` 分支与真实文件系统语义 |
| Node/Agent | Agent 进程、会话与 IPC | IMPLEMENTED_UNRELEASED | 是 | `crates/node`、`apps/agent` | Windows 进程生命周期、sidecar 构建 |
| Session | Session 宿主与跨设备会话原语 | EVIDENCE_GAP | 是 | `crates/node`、`crates/runtime`、`docs/AGENT.md` | 端到端 Windows 会话恢复与审批 |
| Device network | 设备配对/连接/发现 | EVIDENCE_GAP | 是 | `crates/net`、`crates/a2a`、`crates/acp` | Windows 网络栈、mDNS/防火墙、真实配对 |
| Terminal/PTY | 终端、ConPTY、交互流 | EVIDENCE_GAP | 是 | `apps/gui/core/src/term*.rs`、`crates/node` | ConPTY 行为、取消、窗口尺寸、TUI |
| Process | 进程树、Job Object、取消和清理 | IMPLEMENTED_UNRELEASED | 是 | `crates/node/src/*windows*`、`crates/node/tests/windows-process-harness` | 真实 Windows harness 退出码 |
| Files | 文件读写、复制、打开/删除语义、ACL | EVIDENCE_GAP | 是 | `crates/node/src/files.rs`、`apps/gui/native-files` | Windows 路径/ACL/替换语义 |
| Sync | 文件同步与冲突处理 | HISTORICAL/EVIDENCE_GAP | 视 W0 审批 | `crates/sync`、`docs/KHOR.md` | 与设备网络一起验收 |
| Remote control | 远程节点控制与审批 | EVIDENCE_GAP | 是 | `crates/node`、`crates/a2a` | Windows 端到端控制 |
| Update | CLI/GUI/组件更新、回滚 | IMPLEMENTED_UNRELEASED | 是 | `crates/update`、`apps/gui/core/src/updates.rs` | Windows 安装布局、签名/校验、回滚 |
| Install | 安装路径、版本切换、卸载 | EVIDENCE_GAP | 是 | `scripts/install-components.sh`、`docs/INSTALLATION.md` | Windows 安装器/服务/卸载 |
| Credentials | 凭据存储与隔离 | PLATFORM_SPECIFIC | 是 | `crates/node/src/secret.rs` | Windows Credential Manager 或显式受限方案 |
| Packaging | Windows MSI/NSIS/zip 产物 | IMPLEMENTED_UNRELEASED | 是（W6） | `.github/workflows/release.yml`、`tauri.conf.json` | release matrix 无 Windows，bundle.active=false |
| Signing | Windows 签名与校验 | NOT_STARTED | 是（仅验证，不发布） | 发布文档 | 用户未授权签名；W6 只做验证设计 |
| Performance | 启动、同步、文件与终端性能 | EVIDENCE_GAP | 是 | W0 基线、W7 | 基准负载、预热期、资源增长阈值 |
| CI | Windows 构建/测试 job | IMPLEMENTED_UNRELEASED | 是 | `.github/workflows/quality.yml` | 只有 process/fanout job，不是全量构建 |

## 3. REQUIRED 范围（待审批确认）

以下为 Windows 10/11 x64 的 REQUIRED 建议清单：

1. CLI 与 Web 基础命令可运行；
2. GUI 桌面窗口可启动并连接本地/远程节点；
3. 设备配对、连接、断线重连；
4. Session 创建、恢复、关闭与跨设备同步；
5. 终端/ConPTY 交互；
6. 文件读写、复制、打开/删除；
7. 进程启动、Job Object 管理、取消与清理；
8. Agent 进程与 IPC；
9. 安装、更新、回滚与凭据存储；
10. Windows 构建产物与 CI 证据。

以下不纳入默认 REQUIRED，需用户显式追加：

- Windows ARM64；
- Windows Server；
- 移动端 Windows；
- 签名、商店分发、自动发布。

## 4. Windows 10/11 边界

- **Windows 11 x64**：主开发与验收环境；承诺范围内。
- **Windows 10 x64**：承诺范围内；需在 W7 至少保留一次兼容验证。
- **Windows ARM64/Server**：不在默认范围；需用户追加。
- **Linux/macOS/iOS**：现有平台保持兼容，不因 Windows 迁移回归。

## 5. 证据缺口（W1 必须补）

- 本机 Windows 版本、架构、MSVC/SDK/WebView2；
- 前端 `npm run build` 真实退出码；
- `cargo build -p khor-cli` 真实退出码；
- Windows process/fanout harness 真实执行；
- Tauri 桌面构建与启动；
- 安装/更新/凭据真实 Windows 行为；
- 设备网络与终端端到端。

## 6. 引用

- [BASELINE](BASELINE.md)
- `Cargo.toml`
- `apps/gui/package.json`
- `apps/gui/src-tauri/tauri.conf.json`
- `.github/workflows/quality.yml`
- `.github/workflows/release.yml`
- `docs/AGENT.md`
- `docs/SESSION.md`
- `docs/KHOR.md`
