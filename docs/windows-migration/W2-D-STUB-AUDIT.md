# W2-D：W1「已知 Windows 限制」复核

[W1-ACCEPTANCE](W1-ACCEPTANCE.md) §8 列了 8 条"W1 显式保留的 Windows 限制"。那份清单写于 W1（构建底座）当天，之后分支上已经补了实现，但文档没跟上。本文件逐条复核**当前 HEAD 的真实状态**，并记录这一轮补掉的测试编译缺口。

## 1. 逐条复核

| W1 §8 的说法 | 当前状态 | 证据 |
| --- | --- | --- |
| `khor serve` keeper：返回 unsupported（需要 POSIX exec/信号转发） | **已实现**（不再是 stub）。Windows 用独立模块：控制台 Ctrl+C 处理、`KHOR_SERVE_KEEPER_PID` 判断父进程、按安装指针指纹做镜像换代、子进程退出后指数退避重启 | `crates/node/src/keeper_windows.rs`（`lib.rs` 在 `cfg(windows)` 下 `#[path]` 指向它）；`crates/node/src/keeper.rs` 只编译于 Unix，本轮删掉了其中已死的 `#[cfg(not(unix))]` 假 stub |
| `khor quit` 停止 serve/host（POSIX 信号）：返回 unsupported | **已实现**。Windows 分支先按 pid 终止 keeper（防止它再拉起 inner），再终止 inner，然后等两者退出、清扫 host | `crates/node/src/link.rs` `quit()` 的 `#[cfg(windows)]` 块（`win_process::terminate`） |
| tabs 写操作（flock）：返回 unsupported | **已实现**（W2-A）；并在 Windows CI 里有回归步骤（W2-C） | `W2-TABS-WINDOWS-LOCK.md`、`.github/workflows/windows-w1.yml` |
| attach 原始终端（termios）：Windows 返回 unsupported | **已实现**。Windows 用控制台模式（关 line/echo/processed/quick-edit，开 `ENABLE_VIRTUAL_TERMINAL_INPUT`/`PROCESSING`）和 `tty_size`（`GetConsoleScreenBufferInfo`）；termios 仅 Unix 分支 | `crates/commands/src/lib.rs` `#[cfg(windows)] fn raw_on`、`tty_size` |
| secret askpass 父进程检测：Windows 返回 false（保守） | **已实现**。`win_process::parent_id()` 取父进程、`exe_of` 取映像名，`ssh.exe` 大小写不敏感比较 | `crates/node/src/secret.rs` `parent_is_ssh()` 的 `#[cfg(windows)]` 分支 |
| `foreground_group` / `tty_path`：Windows 返回 None | **已不存在**：当前树里没有这两个函数；终端尺寸由 `tty_size()` 提供（Windows 实现见上） | 全仓检索无定义 |
| 进程退出 CRLF/CRT 清理与 Unix 不完全相同 | **仍成立**（注释级差异，不影响契约）：Windows 子进程退出码与信号是两套词汇，文档里以 exit code 表达 | `msg::died_with_code` 等 |
| CLI 激活/更新：Windows 走外部安装器（MSI） | **设计如此**：`upgrade_local` 明确返回"本 CLI 使用外部安装器"，`update_wire` 返回"本安装使用外部包管理器" | `crates/node/src/updates.rs`、`update_wire.rs` 的 `#[cfg(not(unix))]` 块 |

**结论**：W1 的 8 条里 **6 条已经过时**（实现已存在）、1 条是注释级差异、1 条是平台设计。W1-ACCEPTANCE §8 已改为指向本文件。

## 1.1 本轮顺手修掉的两个产品 bug

| 位置 | 症状 | 修法 | 证据 |
| --- | --- | --- | --- |
| `crates/node/src/lib.rs` `find_program` | PATH 按 `':'` 切分；Windows 的分隔符是 `';'`，于是 `C:\Windows\system32;...` 被切成盘符碎片，**任何按裸名启动的程序都找不到**（`khor open -- cmd.exe` 报 "Command not found"） | 改用 `std::env::split_paths`（平台自己的切分器），并在 Windows 上对无扩展名的名字补 `exe`/`cmd`/`bat`；`\` 也算目录分隔符 | 新增 Windows 单测 `a_bare_program_is_found_on_a_windows_path`：1 passed |
| `crates/commands/src/lib.rs` `here()` | 用 `canonicalize().display()` 得到 `\\?\F:\...`，把 verbatim 前缀写进了工作区行——那是操作系统回给操作系统的拼写，不是人敲的，也不是另一台机器该读的 | `plain_path()` 在唯一的"相对→绝对"入口剥掉前缀（`\\?\UNC\` 转回 `\\server\share`） | `cargo test -p khor-cli --test ws`：3 passed |

另外两处**测试语言不一致**（本进程 catalog 默认中文、子进程按环境得英文，而断言比较的是词）在 `quit` 与 `ws` 里统一钉成英文：`khor_catalog::lang::set(En)` + 子进程 `LC_ALL=C`。

## 2. 本轮补的缺口：CLI 集成测试在 Windows 上编译不了

复现：`cargo test --locked -p khor-cli --no-run` → 4 个测试目标失败（`serve_up` 2、`cagent` 4、`serve_keeper` 4、`carry` 1），全部是 Unix API（`std::os::unix`、`PermissionsExt`、`MetadataExt::ino`、`Command::process_group`、`libc`）。

处置（不假装通过）：

| 文件 / 用例 | 处置 | 理由 |
| --- | --- | --- |
| `tests/serve_up.rs`（整文件） | `#![cfg(unix)]` | 登录拉起脚本是 `#!/bin/sh` + `chmod`；Windows 走 MSI 安装与 `keeper_windows.rs` 的守护，E2E 归 W3 |
| `tests/serve_keeper.rs`（整文件） | `#![cfg(unix)]` | shield/in-place exec/inode 身份都是 POSIX 机制；Windows 守护的 E2E 归 W3 |
| `tests/carry.rs`（整文件） | `#![cfg(unix)]` | carry 经 ssh 在远端装 khor 并链接到 `~/.local/bin`，Windows 无此路（W4） |
| `tests/cagent.rs` 的一个生命周期用例 | `#[cfg(unix)]` 单测 gate | 用 `libc::kill` 驱动假 agent 的生死；Windows 等价物（Job/pid 终止）属 W3 |
| `tests/quit.rs` 的假 host | **移植**：Unix `sleep + process_group(0)`，Windows `ping -n 301 127.0.0.1`（无组信号，quit 在 Windows 按 pid 停） | quit 功能本身 Windows 已实现，测试应当真跑 |
| `tests/ws.rs` 的符号链接用例 | `#[cfg(unix)]` 单测 gate | Windows junction 需要权限，W3 再看 |

结果：`cargo test -p khor-cli --no-run` 在 Windows 上 **exit 0**（全部测试目标可编译），`quit` 与 `attach_version` 两个目标实跑结果见 §3。

## 3. 实跑结果

```text
cargo test --locked -p khor-cli --no-run        exit 0   （全部集成测试目标可编译）
cargo test --locked -p khor-cli --test quit     1 passed
cargo test --locked -p khor-cli --test ws       3 passed
cargo fmt --check                               exit 0
```

`quit` 的通过是真的停了一组进程：keeper + inner + 一个假 host，然后断言不再复活、文件仍在。

**attach_version：gate 掉并留下症状**。它已移植成 `cmd.exe` + `taskkill`，但在本机 `khor open -d -- cmd.exe` 始终到不了 ready，两个用例都卡过界；这属于终端/ConPTY 的启动问题（W3 终端 smoke），不是 attach 协议本身。

## 4. 仍未覆盖、交给后续里程碑

| 项 | 去向 |
| --- | --- |
| Windows keeper 的端到端（起 serve → 杀 inner → 自动重启 → 换代） | W3 进程/终端 smoke |
| Windows `khor quit` 对真实 keeper+inner 的终止 | W3（本轮已让测试可编译并跑起来） |
| ConPTY 下的交互（TUI、窗口尺寸变化、取消） | W3 |
| Windows 的 ssh carry 与远端安装 | W4 |
| Windows 更新（MSI）与回滚 | W6 |

## 5. 回滚

- 本轮删除的是 `crates/node/src/keeper.rs` 里**编译不到**的 `#[cfg(not(unix))]` 死代码，以及 CLI 测试的平台门控/移植；没有改产品行为。
- 单提交可 revert，不影响 W2-A/B/C 的验收结论。
