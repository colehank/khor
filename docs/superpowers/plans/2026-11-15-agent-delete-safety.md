# Windows 删除安全收口 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development (recommended) or executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Khor 的 Windows 开发在 `workspace-write` 下闭环，并在 full access 下也拦死所有触及 C: 的删除/覆盖。

**Architecture:** 仓库侧提供路径校验原语与 `dev.py` 的 probe/clean/doctor 子命令，把构建缓存与探针目录收进 worktree；机器侧装一个本地 DSH 插件在 `tools/pre-execute` 拦截 C: 路径；另加每日备份与一次 F: 盘收口。

**Tech Stack:** Python 3.12（`scripts/khor_lab`，pytest）、PowerShell 7、Node ESM（DSH 本地插件，`node --test`）、Cordis 插件 API。

**Spec:** `docs/superpowers/specs/2026-11-15-agent-delete-safety-design.md`

## Global Constraints

- 只允许在会话工作区 `F:\#Gong\#AI4S\khor` 内写入；工作区外只能通过一次用户批准的 full-access 调用。
- 任何删除在动手前必须打印解析后的绝对路径；默认 dry-run，`--yes` 才执行。
- 回绝名单（任何工具都不得删/覆盖）：`%USERPROFILE%`、`%APPDATA%`、`%LOCALAPPDATA%`、`%SystemRoot%`、`~/.dsh`、任何盘根。
- 路径校验必须解析 reparse point（junction/symlink），链上出现即拒绝。
- 不使用 `Remove-Item -Recurse -Force` 作为实现手段；Python 用 `shutil.rmtree(..., onerror=...)` 且先 rename。
- 新增 Python 代码遵循现有 `scripts/khor_lab` 风格（无第三方依赖，标准库 + pytest）。
- 测试运行：`python -m pytest scripts/tests -q`（沿用仓库现有入口）。

## Review Focus

- 路径尾随空格/点、`CON`/`NUL` 等保留名：必须被拒绝，不能变成"删到别的目录"。
- `.probe` 内出现指向工作区外的 junction：GC 必须拒绝，而不是跟随。
- 手改 manifest 把某条改成 `C:\...`：执行时必须拒绝，整条清单不删任何东西。
- 命令行里字形上出现 `C:` 但无害（注释、字符串常量）：允许误报为拒绝，不得误放行。
- 构建正在进行时跑 `clean`：先 rename 失败即跳过，不能产生半删目录。

---

### Task 1: 路径校验原语

**Files:**
- Create: `scripts/khor_lab/safepaths.py`
- Test: `scripts/tests/test_safepaths.py`

**Interfaces:**
- Produces: `class UnsafePath(ValueError)`；`resolve_owned(path: str | Path, allow_roots: Sequence[Path]) -> Path`；`assert_no_reparse(path: Path) -> None`；`require_marker(directory: Path, marker: str = ".khor-probe.json") -> dict`；`DENIED_ROOTS: tuple[Path, ...]`

- [ ] **Step 1: 写失败测试**

```python
def test_rejects_home():
    with pytest.raises(UnsafePath):
        resolve_owned(Path.home(), [Path.cwd()])

def test_rejects_drive_root():
    with pytest.raises(UnsafePath):
        resolve_owned(Path("C:/"), [Path("C:/anything")])

def test_rejects_outside_allow_roots(tmp_path):
    with pytest.raises(UnsafePath):
        resolve_owned(tmp_path / "x", [tmp_path / "allowed"])

def test_accepts_child(tmp_path):
    allowed = tmp_path / "allowed"; allowed.mkdir()
    assert resolve_owned(allowed / "a", [allowed]) == (allowed / "a").resolve()
```

- [ ] **Step 2: 运行确认失败** — `python -m pytest scripts/tests/test_safepaths.py -q`，期望 `ImportError`。

- [ ] **Step 3: 实现** — `resolve_owned` 先 `Path(path).expanduser().resolve()`，再 `assert_no_reparse` 逐级检查 `st_reparse_tag`，再拒 `DENIED_ROOTS`、盘根、`allow_roots` 之外；`require_marker` 读 JSON，缺文件抛 `UnsafePath`。

- [ ] **Step 4: 运行确认通过**，再补三个用例：junction 被拒（用 `tmp_path` 内 `mklink /J` 失败则 skip）、manifest 指向 C: 被拒、尾随空格与保留名（`"nul"`、`"x. "`）被拒。

- [ ] **Step 5: Commit** — `git add scripts/khor_lab/safepaths.py scripts/tests/test_safepaths.py && git commit -m "feat(lab): add path ownership validation primitives"`

---

### Task 2: `lab probe new|gc`

**Files:**
- Modify: `scripts/khor_lab/cli.py`（或现有子命令分派处，先读再改）
- Create: `scripts/khor_lab/probe.py`
- Test: `scripts/tests/test_lab_probe.py`

**Interfaces:**
- Produces: `new_probe(root: Path, kind: str) -> Path`；`gc_probes(root: Path, older_than: timedelta, dry_run: bool) -> list[Path]`

- [ ] **Step 1: 失败测试** — `new_probe` 建出 `.probe/<utc>-<hex>/` 且写 `.khor-probe.json`；`gc_probes` 只删带 marker 且超龄的目录，无 marker 目录留下；dry-run 不删。
- [ ] **Step 2: 运行确认失败**。
- [ ] **Step 3: 实现** — marker 里 `{"id", "kind", "created_at"}`；GC 逐条 `resolve_owned(root/".probe")`，`rmtree` 前再核 marker。
- [ ] **Step 4: 运行确认通过**。
- [ ] **Step 5: Commit**。

---

### Task 3: `lab clean --inventory|--manifest`

**Files:**
- Create: `scripts/khor_lab/clean.py`
- Modify: `scripts/khor_lab/cli.py`
- Test: `scripts/tests/test_lab_clean.py`

**Interfaces:**
- Produces: `inventory(prefix_root: Path, pattern: str) -> dict`（写 manifest JSON）；`clean(manifest: dict, allow_prefixes: Sequence[str], dry_run: bool, yes: bool) -> dict`（返回 receipt）

- [ ] **Step 1: 失败测试** — manifest 条目 `path` 不在 `allow_prefixes` → 抛 `UnsafePath` 且目录仍在；特征不符 → 跳过并记 `skipped`；正常条目 → 目录消失、receipt 含 `bytes`/`removed_at`。
- [ ] **Step 2: 运行确认失败**。
- [ ] **Step 3: 实现** — 每个条目：`resolve_owned` → `assert_no_reparse` → 核 `.rustc_info.json`/`CACHEDIR.TAG` → `os.replace` 改名为 `<name>.deleting-<ts>`（失败即跳过）→ `shutil.rmtree`。
- [ ] **Step 4: 运行确认通过**。
- [ ] **Step 5: Commit**。

---

### Task 4: 探针环境重定向

**Files:**
- Modify: `scripts/khor_lab/environment.py`
- Test: `scripts/tests/test_lab_environment.py`

**Interfaces:**
- Produces: `probe_environment(root: Path, home: Path) -> dict[str, str]`，必含 `HOME`、`APPDATA`、`LOCALAPPDATA`、`WEBVIEW2_USER_DATA_FOLDER`，且全部指向 `home` 之下。

- [ ] **Step 1: 失败测试** — 断言四个键都在、都 `Path(v).is_relative_to(home)`。
- [ ] **Step 2: 运行确认失败**。
- [ ] **Step 3: 实现** — 在现有重定向 dict 里加三项，路径 `home/"AppData/Roaming"`、`home/"AppData/Local"`、`home/"WebView2"`。
- [ ] **Step 4: 运行确认通过**（含原有断言不回归）。
- [ ] **Step 5: Commit**。

---

### Task 5: `lab doctor` + 文档 + 忽略项

**Files:**
- Modify: `scripts/khor_lab/cli.py`、`scripts/khor_lab/config.py`（相对 `cache_root` 按 root 解析）
- Modify: `docs/development/LAB.md`、`docs/code-quality/ENGINEERING.md`、`.gitignore`
- Test: `scripts/tests/test_lab_doctor.py`

**Interfaces:**
- Produces: `doctor(root: Path, values: dict) -> list[tuple[str, bool, str]]`（名称、是否通过、修复提示）

- [ ] **Step 1: 失败测试** — cache_root 指到工作区外 → 该项 False；指到 `<root>/.cache/lab` → True；`CARGO_TARGET_DIR` 不在工作区外时 True；`CARGO_HOME`/`PNPM_HOME` 在工作区外时给出“改依赖需要一次批准”的提示项（True + 提示文本）。
- [ ] **Step 2: 运行确认失败**。
- [ ] **Step 3: 实现** — `.gitignore` 加 `/.cache/`、`/.probe/`；首次运行写 `.khor-lab.local.json`；`LAB.md` 加 Windows 段（三条：缓存/探针/环境重定向 + 依赖变更的一次批准流程）；`ENGINEERING.md` 加“删除必须走 lab 原语”。
- [ ] **Step 4: 运行确认通过**。
- [ ] **Step 5: Commit**。

---

### Task 6: 护栏匹配器（纯函数）

**Files:**
- Create: `tools/delete-guard/guard.mjs`
- Test: `tools/delete-guard/test/guard.test.mjs`

**Interfaces:**
- Produces: `decide(tool: string, args: Record<string, unknown>) -> {allow: boolean, reason?: string}`

- [ ] **Step 1: 失败测试** — 事故命令 `Remove-Item $home -Recurse -Force` → deny；`Remove-Item C:\Users\admin\x -Recurse` → deny；`python -c "import shutil;shutil.rmtree(r'C:\x')"` → deny；`cmd /c rd /s /q C:\Users\admin` → deny；`Remove-Item F:\khor-target\t -Recurse -Force` → allow；`Get-Content C:\Users\admin\.dsh\a.json` → allow（只读白名单）；`write` 目标 `C:\x` → deny；`plugin_manager set_plugin` → deny。
- [ ] **Step 2: 运行确认失败** — `node --test tools/delete-guard/test/`。
- [ ] **Step 3: 实现** — 规则顺序：meta-guard → C:/用户目录 deny（只读白名单除外）→ 破坏性动词的字面路径校验；allowlist 从 `config.json` 读，缺省 `["<workspace>", "F:\\"]`。
- [ ] **Step 4: 运行确认通过**。
- [ ] **Step 5: Commit**。

---

### Task 7: 插件接线与安装

**Files:**
- Create: `tools/delete-guard/package.json`、`cordis.patch.yml`、`host.mjs`、`config.json`
- Test: `tools/delete-guard/test/host.test.mjs`（假 ctx 记录注册）
- Install: `C:\Users\admin\.dsh\local-plugins\dsh-delete-guard\`（需一次批准）

**Interfaces:**
- Consumes: Task 6 的 `decide`
- Produces: Cordis 插件默认导出 `apply(ctx)`，注册 `tools/pre-execute` 监听；deny 返回 `dsh-tools` 的 `PreToolDecision` 拒绝形状

- [ ] **Step 1: 先读契约** — 读 `F:\DeepSeek Harness\resources\app.asar\dsh\node_modules\@deepseek-ai\dsh-tools\lib\*.d.ts` 确认 `PreToolDecision` 的确切字段，把结果记在本任务注释里。
- [ ] **Step 2: 失败测试** — 假 ctx 的 `on` 收到 `tools/pre-execute`；调用监听器，deny 时拿到拒绝决定、allow 时 `next()` 被调用一次。
- [ ] **Step 3: 实现 host.mjs** — `apply(ctx) { ctx.on('tools/pre-execute', async (exec, next) => decide(...).allow ? next() : denial) }`，并追加 JSONL 到 `~/.dsh/logs/delete-guard.jsonl`。
- [ ] **Step 4: 安装并验收** — 复制到 profile 的 `local-plugins`；在 `cordis.patch.yml` 加一行；重启 DSH 后：full access 下跑事故命令 → 被拒；`Get-Content` 读 C: → 通过。
- [ ] **Step 5: Commit**（插件源码在仓内 `tools/delete-guard/`）。

---

### Task 8: 每日备份

**Files:**
- Create: `scripts/dsh-backup.ps1`
- Test: `scripts/tests/test_dsh_backup.py`（只测参数解析与清单生成，实际复制手工验）

- [ ] **Step 1: 失败测试** — `-WhatIf` 打印源/目标清单，含 `sessions`、`storages`、`states`、`cordis.patch.yml`、`delete-guard/config.json`。
- [ ] **Step 2: 实现** — 复制到 `F:\Backups\dsh\<UTC>\`，保留 7 份；`-Install` 用 `Register-ScheduledTask` 注册当前用户 03:00 任务。
- [ ] **Step 3: 执行安装**（已授权）并手工跑一次。
- [ ] **Step 4: 校验** — 复制的 session JSONL 行数与源一致。
- [ ] **Step 5: Commit**。

---

### Task 9: F: 盘收口

**Files:**
- Create: `docs/superpowers/receipts/2026-11-15-f-drive-cleanup.json`（manifest + receipt）

- [ ] **Step 1: 盘点** — `python scripts/dev.py lab clean --inventory F:\khor-*`，逐条核对签名/大小/时间。
- [ ] **Step 2: 搬暖缓存** — 同盘 rename `F:\khor-target-w2` → `<worktree>\.cache\target-w2`，导出 `CARGO_TARGET_DIR` 到 `.khor-lab.local.json`。
- [ ] **Step 3: 核源码检出** — `git -C F:\khor-build ... log -1`，确认 HEAD 在主仓存在；不存在则保留并报告。
- [ ] **Step 4: 执行** — 一次批准的 full-access 调用跑 `lab clean --manifest ... --yes`，逐条打印。
- [ ] **Step 5: 复核** — `Get-ChildItem F:\khor*` 只剩 toolchain、evidence、target-w2（已在工作区内）及任何被保留的检出；receipt 写入文件。
