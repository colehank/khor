# Windows 开发安全设计：删除事故的收口

日期：2026-11-15
状态：待批准（已并入独立 builder 审查的五个问题）
范围：`khor` 仓库的 Windows 开发流程 + 本机 DSH profile 的一个本地插件 + 一次 F: 盘收口

## 背景与意图

北京时间 02:10:37，一条 Khor 探针命令把 `$home` 赋值给 `F:\Temp\khor-open-probe`，
赋值因 `$HOME` 是 ReadOnly 内置变量而失败，紧随其后的
`Remove-Item $home -Recurse -Force -ErrorAction SilentlyContinue`
仍然作用在真实用户目录 `C:\Users\admin` 上，删除了 DSH 的会话数据。

成因不是“某一条命令写错”，而是三件事叠加：

1. 开发流程要求写工作区之外的路径（`F:\khor-target-w2`、`F:\Temp\...`、Lab 默认缓存
   `%LOCALAPPDATA%\KhorDeveloperLab`），要写这些路径只能给 full access。
2. full access 下 ACL 沙箱（`dsh-pwsh-sandbox`）完全不参与，唯一剩下的防线是模型自己。
3. `~/.dsh` 只有一份，删掉即不可恢复。

本设计拆掉第 1 条这个前提，并补上工具层护栏与数据副本。

## 权限边界（已定）

- **C: 一律拦**：任何工具调用只要触及 C:（含 `$HOME`/`%USERPROFILE%`/`APPDATA`/`LOCALAPPDATA`/
  `SystemRoot`）就拒绝，full access 也不放开。
- **C: 之外可放开**：工作区、`F:` 上 agent 生成的内容与构建产物可以持有完全权限，不为效率设障。
- 因此本设计的目的不是“禁止全权限”，而是“让 C: 不在全权限的作用域里”。

## 目标

1. Windows 日常的构建、单元测试、探针全程留在 `workspace-write`，不再需要 full access。
2. 即使处在 full access，agent 的工具调用只要触及 C: 的删除/覆盖，一律拒绝，不能在会话内放开。
3. 探针目录唯一、不复用、不就地清理；清理只走带归属标记的清单。
4. F: 上现有的 `khor*` 目录一次性收口，之后不再新增工作区外的构建目录。
5. `~/.dsh` 有可恢复的每日副本。

## 非目标 / 反目标

- 非目标：内核级“绝对不可绕过”。那需要独立 Windows 账户或 VM；本设计是“工具层护栏 + ACL 沙箱 + 备份”三层。
- 非目标：改 Khor 产品行为、改 macOS 现有 `/Volumes/disk0` 流程。
- 反目标：为了让某条命令跑通而放宽沙箱；自动清理无法确认归属的目录；把 evidence 当缓存删。

## 已核实的事实

- 当前 profile 挂的是 `dsh-pwsh-sandbox`（Windows ACL 受限令牌），`workspace-write` 下
  `TMP`/`TEMP` 被重写到 `...\Temp\dsh-XXXX`；写 `C:\Users\admin\.dsh\` 与 `F:\khor-toolchain\cargo`
  都被拒。**工作区外的写一律被拒，不只是 C:。**
- `sandbox-policy` 的“额外可写根”只对文件工具生效，对被沙箱的进程无效——所以不存在
  “给 `lab clean` 开个 F: 口子”这种配置。
- 事故命令的前缀与 `@deepseek-ai/dsh-pwsh-local` 一致，即走的是不接 `ctx.sandbox` 的执行器/会话。
- `tools/pre-execute` 是可扩展的 allow/deny/ask 瀑布，本地插件可在此拒绝调用；`ctx.tools.guard()`
  另提供单调 deny。
- `khor_lab/environment.py` 目前只重定向 `HOME`/`XDG_*`/`KHOR_HOME`，**没有**重定向
  `APPDATA`/`LOCALAPPDATA`；而 Tauri/WebView2 的桌面数据写在后者（builder 审查发现）。
- `CARGO_HOME=F:\khor-toolchain\cargo`、`RUSTUP_HOME=...\rustup`、`PNPM_HOME=...\pnpm\home`
  都在工作区外；离线、暖缓存的 `cargo build` 在沙箱内可跑通（builder 实测 serde），
  但新增/更新依赖需要写这些目录。
- profile 里还挂着 `plugin_manager`（可停用任何插件）、`subagent_codex`（真实 Codex CLI 子进程，
  不走 DSH 的工具管线，可选择 `dangerously-bypass-approvals-and-sandbox`）、`windows` MCP
  （已排除 App/PowerShell/FileSystem 工具）。
- F: 现有 14 个 `khor*` 目录，合计约 57 GiB：
  - 8 个 Cargo target（`khor-target`、`-build`、`-build2`、`-check`、`-harness`、`-harness2`、`-nodefix`、`-tabs`），约 29 GiB，可重建。
  - `khor-target-w2` 26.5 GiB，11/15 仍在写，当前在用。
  - 3 个源码检出：`khor-build`、`khor-build2`（detached HEAD，只有 `.err`/lock 未跟踪文件）、
    `khor-w2-base`（Administrators 拥有，git 报 dubious ownership），删前须确认 HEAD 已在别处。
  - `khor-w1-evidence` 存证据日志；`khor-toolchain` 是 rustup/pnpm/nasm 工具链，保留。
- `~/.dsh` 体量：sessions 0.8 MB、storages 0.9 MB、states 0；profiles 730 MB 主要是 node_modules，不备份。

## 方案取舍

- A 就地加固（选定）：把要写的东西收进工作区，沙箱长开；仓库提供清理原语；插件做语言层护栏；补备份。
- B 搬出本机执行：隔离最强，但 Khor 的桌面行为验收没法做，迭代成本高。
- C 远端构建：LAB.md 的远端 full 只覆盖 macOS 主机，Windows 原生构建没有现成能力。

## 设计

### 1. 工作区即边界

- 约定（写入 `docs/development/LAB.md` Windows 段与 `docs/code-quality/ENGINEERING.md`）：
  Windows 上临时目录、构建缓存、探针 home 必须在会话工作区内。
- 每个 worktree 的本机配置 `<worktree>\.khor-lab.local.json`（已在 `.gitignore`）：
  `{ "cache_root": "<worktree>\\.cache\\lab" }`，新 worktree 由 `dev.py` 首次运行时自动生成。
- Cargo：`CARGO_TARGET_DIR=<worktree>\\.cache\\target`，由 `dev.py` 的构建/探针入口统一注入；
  文档不再出现工作区外的路径示例。
- 探针环境（`khor_lab/environment.py`）新增重定向：`APPDATA`、`LOCALAPPDATA`、
  `WEBVIEW2_USER_DATA_FOLDER` 指向 `.probe\<id>\home\...`，让被启动的桌面程序把用户数据写进工作区。
  这是让“桌面验收也能在 workspace-write 下跑”的前提；若某个写入仍落在别处，记录该次探测失败，
  不退回 full access。
- `.gitignore` 增加 `/.cache/`、`/.probe/`。
- `dev.py lab doctor`（只读）检查：cache_root、target、探针根是否在工作区内；APPDATA 重定向是否生效；
  工具链根在工作区外时明确提示“改依赖需要一次批准”。任何一项不符即非零退出并给修复命令。

### 2. 清理原语（把“删除”从 agent 手里拿走）

新文件 `scripts/khor_lab/safepaths.py`，纯函数、可单测：

- `resolve_owned(root, path)`：规范化并解析 reparse point；拒绝盘根、`%USERPROFILE%`、`%APPDATA%`、
  `%LOCALAPPDATA%`、`~/.dsh`、以及不落在 allowlist 根内的路径。
- `require_marker(dir)`：目录必须含 `.khor-probe.json`（id/kind/created_at）。
- `assert_no_reparse(path)`：路径链上任何 junction/symlink 即拒绝。

`dev.py` 新子命令：

- `lab probe new --kind <k>` → 建 `<worktree>\\.probe\\<UTC>-<id>\\` 并写 marker，打印绝对路径。
  探针目录不复用、不就地清理。
- `lab probe gc [--older-than 7d] [--dry-run] [--yes]` → 只处理 `.probe/*` 且带 marker 的目录；
  默认 dry-run，`--yes` 才动手，逐条打印解析后的绝对路径。全程在工作区内，agent 可执行。
- `lab clean --inventory F:\khor-*` → 只生成 manifest，不删。
- `lab clean --manifest <json> [--dry-run] [--yes]` → 删工作区外的目录。manifest 每条含绝对路径、
  期望特征（`.rustc_info.json`/`CACHEDIR.TAG`）、预估字节、生成时间；执行时复核特征、复核无打开句柄，
  不符即跳过并记录 receipt。

测试 `scripts/tests/test_safe_cleanup.py`：拒绝 `$HOME`、盘根、junction、无 marker、allowlist 外、
manifest 特征不符、路径被占用等场景；每条断言“未发生删除”。

### 3. 删除护栏插件（`dsh-delete-guard`）

本地插件，监听 `tools/pre-execute`（deny 分支）。匹配策略改为**两级默认拒绝**，不再只靠动词清单：

1. **C: 与用户目录默认拒绝**：命令文本出现 `C:`、`$HOME`、`%USERPROFILE%`、`$env:USERPROFILE`、
   `APPDATA`、`LOCALAPPDATA`、`SystemRoot` 即 deny，除非整条命令匹配人工维护的只读前缀白名单
   （初始只有 `Get-ChildItem`、`Get-Content`、`Test-Path`、`Get-Item`、`git -C ... status/log`）。
   这一条不区分动词，所以 `python -c "shutil.rmtree(...)"`、`node -e`、`cmd /c rd`、`pwsh -File`
   都落网。
2. **破坏性动词**（`Remove-Item|rm|del|rd|rmdir|Clear-Content|robocopy /mir|/purge|git clean|
   Format-Volume|rmtree|rmSync|::Delete`）：操作数必须是字面绝对路径且不在 C: 上（工作区与 F: 均可）；
   变量、通配、拼接一律 deny。
3. **工具级 meta-guard**：`plugin_manager` 的 set/install/remove 动作一律 deny（只有 list 放行）——
   否则一次调用就能把本插件关掉；`write`/`edit` 目标在 C: 一律 deny。
   `subagent_codex` 保留（疑难问题还要用）：它起真实 Codex CLI 子进程，命令不走 DSH 管线，
   护栏看不到它。缓解：profile 里固定 `permissionMode` 不为 `dangerously-bypass-approvals-and-sandbox`，
   子进程默认 `never`（不 bypass），并把它列为残余风险写进文档。
4. 命中即返回 deny 原因，模型看到 `Error: ...` 并被告知改用 `python scripts/dev.py lab probe gc`；
   同时追加一行到 `~/.dsh/logs/delete-guard.jsonl`。
5. allowlist 在插件目录的 `config.json`，人工编辑；agent 改不到（写 C: 会被它自己拦住）。

**诚实边界**：这是策略层，不是内核边界。人类在普通终端里执行不经过它；`windows` MCP 目前没有
文件工具，但不能保证以后加入的 MCP server 不绕过它。要“绝对保证”只有独立 Windows 账户或 VM。

安装位置 `~/.dsh/local-plugins/dsh-delete-guard/`（`package.json` 带 `dsh.bundle.patch` +
`cordis.patch.yml`）。先在 `<worktree>\tools\delete-guard\` 开发并跑单测，再安装。

### 4. 备份

- `scripts/dsh-backup.ps1`：把 `~/.dsh/{sessions,storages,states}`、`profiles/desktop/{cordis.yml,
  cordis.patch.yml,package.json}`、以及 `local-plugins/dsh-delete-guard/{config.json,*.mjs}` 复制到
  `F:\Backups\dsh\<UTC>\`，保留最近 7 份（裁剪只在该根内、按目录名删）。
- 安装：`scripts/dsh-backup.ps1 -Install` 为当前用户注册每日 03:00 的计划任务（不需要管理员）。
- 恢复演练一次：用 `DSH_HOME` 指向副本起只读实例，确认能读出会话。

### 5. F: 收口（留有用的，删无用的）

工作区外的写被沙箱拒绝，执行时用一次已批准的全权限调用，或你在普通终端里跑同一条命令。

1. agent 用 `lab clean --inventory F:\khor-*` 生成 manifest 并逐条核对（只读）。
2. **保留**：`khor-toolchain`（rustup/pnpm/nasm 工具链）、`khor-w1-evidence`（证据）。
3. **不复用旧缓存**：`khor-target-w2` 曾尝试用同盘 rename 搬进 `.cache/target`，但实测同盘移动保留旧 ACL/完整性标签，
   被限制的子进程既不能写也不能删该树。结论：删除旧缓存，让构建在工作区内**就地新建** `.cache/target`（实测可写）。
4. **删除**：8 个旧 Cargo target（约 29 GiB）。
5. **判定后删**：`khor-build`、`khor-build2`、`khor-w2-base` 三个源码检出，先确认 HEAD 已在别处
   （detached HEAD 无 upstream，只有 `.err`/lock 未跟踪文件）；确认无独有提交后随清单删除。
6. 执行 `lab clean --manifest ... --yes`，逐条打印路径，结果写 receipt。

### 6. 依赖变更（工具链在工作区外）

- `cargo add`/`cargo update`/`pnpm install` 首次需要写 `F:\khor-toolchain\...`，沙箱会拒。
  这是**已知且唯一的**例外：文档给出固定命令，由你批准后以单次 full-access 执行，不长期开。
- `lab doctor` 在检测到需要写工具链的场景时直接提示该流程，避免“顺手切 full access”。
- 已有暖缓存、离线构建在 workspace-write 内正常（builder 已实测）。

## 已批准的一次性动作

1. 安装本地插件 `dsh-delete-guard`（一次写 `C:\Users\admin\.dsh\...`，走一次批准）。
2. 执行 `scripts/dsh-backup.ps1 -Install`（已授权我执行）。
3. `subagent_codex` 保留（疑难问题要用）；固定 `permissionMode` 不为 bypass，残余风险照写。
4. F: 收口按 §5 的“留有用的、删无用的”执行。

## 验收

- full access 下执行 `Remove-Item $home -Recurse -Force`：插件拒绝，目标目录无变化。
- full access 下执行 `python -c "import shutil;shutil.rmtree(r'C:\Users\admin\x')"` 与
  `cmd /c rd /s /q C:\Users\admin\x`：同样拒绝。
- `workspace-write` 下写 `C:\Users\admin\x`：被 ACL 拒绝。
- `plugin_manager set_plugin ... false`：被拒绝。
- `lab probe gc` 只删带 marker 的 `.probe/*`；手工塞入无 marker 目录不被删。
- `lab doctor` 在工作区外的 cache_root 上非零退出。
- 从备份副本读出最近一次 session。
- F: 收口后只剩 `khor-toolchain`、`khor-target-w2`（及保留的 evidence / 待确认源码检出）。
