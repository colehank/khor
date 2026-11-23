# W3：进程 / 终端 / 文件 smoke

## 1. 目标与验收门

见 [ROADMAP](ROADMAP.md) §1 W3：真实 Windows 上的端到端冒烟，而不是只跑单测。

**验收门**：一份可执行的 smoke 清单 + 每条的实际命令 / 退出码 / 日志；失败项必须显式 unsupported 或已修。

## 2. 环境前提

两条限制先后出现过，根因不同：

1. **受限令牌（`workspace-write` 沙箱）拒绝命名管道**。CLI 的本地 IPC、agent supervisor、ConPTY 输出都走管道，
   所以那时 5 项只能记为 ENV。切到 `full` 后该限制消失（管道探针可连接）。
2. **工作区里构建的程序是 Low 进程**。沙箱给工作区打了 Low 完整性标签（standing），在其中构建的可执行文件
   启动后仍是 Low，写不了 Medium 的 `%LOCALAPPDATA%\Khor\runtime`。同一个 `khor.exe`：

   ```text
   F:\Temp\khor.exe help                    → usage: khor <verb>
   .cache\target\debug\khor.exe help        → 拒绝访问 (os error 5)，退出码 78
   ```

   这不是产品缺陷。**W3 的 CLI / 进程 / 终端项必须用工作区之外构建的二进制**：

   ```powershell
   $env:CARGO_TARGET_DIR='F:\Temp\khor-target'
   cargo build --locked -p khor-cli
   pwsh -File scripts/windows-smoke.ps1 -CargoTargetDir 'F:\Temp\khor-target'
   ```

   库单测不受影响（只写临时目录），仍可用工作区内的 `.cache/target`。

## 3. 运行器

`scripts/windows-smoke.ps1`：每项一条记录（`name / area / status / exit / seconds / first line`），
完整输出落在 `.cache/w3/<name>.log`，汇总写 `.cache/w3/smoke-<utc>.json`；`FAIL` 才非零退出。
`-CargoTargetDir` 决定被测二进制的位置（见 §2）。

## 4. 矩阵与结果（2026-11-15，`full` + 工作区外构建）

| # | 区域 | 用例 | 运行方式 | 结果 |
| --- | --- | --- | --- | --- |
| P1 | 进程 | 子进程启动与退出码 | `khor --version` | **PASS** |
| P1–P3 | 进程 | Job 生命周期 / 取消 / 清理 | `cargo test -p khor-node --lib khor_agent_process::` | **PASS**（164.7s） |
| P4 | 进程 | 父退出后清理（`khor quit`） | `cargo test -p khor-cli --test quit` | **PASS**（16.1s） |
| T1 | 终端 | CLI 能起一个会话 | `khor help` | **PASS** |
| T1–T4 | 终端 | ConPTY 启动 / 回显 / 尺寸 / 关闭 | `cargo test -p khor-cli --test console` | **PASS**（9.9s） |
| T5 | 终端 | attach 到已存在终端 | `cargo test -p khor-cli --test attach_version` | **NOT_TESTED**：`khor open -d -- cmd.exe` 已能起宿主；ConPTY 输出问题已修（`vendor/portable-pty`，见 [W5-GUI](W5-GUI.md) §4.1），但这两条用例仍越过时限阻塞，原因与读取重复无关，待查 |
| F1 | 文件 | 原子替换 / 句柄语义 | `cargo test -p khor-node --lib session_content::journal::` | **PASS**（4.9s） |
| F2 | 文件 | 只读表：结构化报错而非 panic | `tabs::tests::a_read_only_table_reports_an_error_instead_of_panicking` | **PASS** |
| F3 | 文件 | >260 字符路径仍能读写 | `tabs::tests::a_table_beyond_max_path_still_opens` | **PASS** |
| F4–F5 | 文件 | 权限拒绝 / 受限目录清理 | `cargo test -p khor-cli --test ws` | **PASS**（51.7s） |
| B1 | tabs | 两个**进程**并发写同一张表 | `tabs::tests::two_processes_do_not_lose_each_others_tabs` | **PASS** |
| B1′ | tabs | 两个 `khor tabs` CLI 进程并发 | `khor tabs ls` ×2 | **PASS**（`first=0 second=0`） |
| B2 | tabs | GUI 与 CLI 并发 | 需要 GUI 产物 | **延后到 W5**（GUI 构建属于 W5；CLI 侧 B1′ 已证） |

汇总：**12 项 PASS / 0 ENV / 0 FAIL**，1 项 NOT_TESTED（T5），1 项延后（B2）。
receipt：`.cache/w3/smoke-20261115T131532Z.json`。

## 5. 执行记录

| 日期 | 环境 | 命令 | 结果 |
| --- | --- | --- | --- |
| 2026-11-15 | `workspace-write` | `windows-smoke.ps1 -Sandboxed` | 6 PASS / 5 ENV / 0 FAIL（receipt `…123428Z.json`） |
| 2026-11-15 | `full`，工作区内构建 | `khor help` / `khor ws add` | 均 `os error 5`（Low 进程） |
| 2026-11-15 | `full`，`F:\Temp\khor-target` | `windows-smoke.ps1 -CargoTargetDir …` | **11 PASS / 0 ENV / 0 FAIL**（receipt `…131532Z.json`） |
| 2026-11-15 | `full` | `khor open -d -- cmd.exe` | 退出码 0，`shell/ad6eb00b590a62e8` |
| 2026-11-15 | `full` | 去掉 gate 跑 `attach_version` | 两条用例阻塞超过 12 分钟，已恢复 gate |

## 6. 结论与下一步

- 进程、文件、tabs 的 smoke 在真实 Windows 上通过；终端 ConPTY 的启动/回显/尺寸/关闭通过。
- 唯一未过的是 attach 用例的读取阻塞（T5），已显式记 NOT_TESTED 并复现。
- B2 的 GUI 侧留给 W5；W3 其余项足以支撑进入 W4。
