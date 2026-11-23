# Windows W2-A：tabs 写锁验收

## 1. 结论

Windows 上 `khor tabs` 与窗口两个进程的 read-modify-write 已由真实文件锁串行化，Windows 的 unsupported 写桩已删除。

**验收：PASS（本地 Windows 11 x64，CI 钉版工具链 rustc 1.95.0）**

- tabs 套件 31/31 通过（修复前 1 passed / 29 failed）；
- 同分支全量 lib 测试：失败 **156 → 125**，通过 **931 → 963**，新增失败 **0**；
- 独立评审 **PASS_WITH_NITS**，12 条意见逐条裁定见 §7；
- 未合 main、未 push、未签名、未发布。

本文件是本地证据，不宣称 Windows 全功能完成，也不替代远端 CI 与用户验收。

## 2. 环境

| 项目 | 值 |
| --- | --- |
| OS | Windows 11 专业版 10.0.26100 x64 |
| 分支 / HEAD | `steward/s2-windows-20261114` / `dc72eaec` |
| rustc / cargo | 1.95.0（59807616e，2026-04-14），x86_64-pc-windows-msvc |
| rustfmt | 1.9.0-stable（59807616e1，2026-04-14） |
| MSVC | Visual Studio 2022 BuildTools，MSVC 14.44 + Windows SDK |
| NASM | 3.02 |
| 工具链位置 | `F:\khor-toolchain\{rustup,cargo,nasm,pnpm}`（`RUSTUP_HOME`/`CARGO_HOME` 重定向，C 盘不落文件） |
| 构建缓存 | `CARGO_TARGET_DIR=F:\khor-target-w2`，`TMP=TEMP=F:\Temp` |

**工具链重建说明**：2026-11-14 晚本机 Rust 工具链、NASM、pnpm 被外部清理删除（`rustup toolchain list` 为空、`C:\Users\admin\.rustup` 空）。本轮按用户要求全部重建到 F 盘并固定为 CI 钉版 1.95.0。重建证据：`.w2\phase0-env.log`（rustc/cargo/rustfmt/NASM 版本 + `cargo check --locked -p khor-node` `CHECK_EXIT=0`，4m54s）。

## 3. 改动

| 提交 | 范围 | 内容 |
| --- | --- | --- |
| `936698a6` | 6 文件，+34/−7，全部 `#[cfg(test)]` 范围 | 让 `cargo test -p khor-node --lib --no-run` 在 Windows 可编译。4 处 Unix-only 测试加 `#[cfg(unix)]`（FIFO、launcher、`Session::pid` 追踪、`ExitStatusExt::signal` 断言），2 处真实移植（父进程 PID 改用 `sysinfo`；kill 退出码断言按平台分支）。 |
| `dc72eaec` | `crates/node/src/tabs.rs`，+41/−24 | 删除 `#[cfg(not(unix))] change` 桩；`save`/`Hold`/`change` 去掉平台分支；`Hold::take` 用 `File::lock`（Unix `flock` / Windows `LockFileEx`）跨越读-改-写；`save` 保持「临时文件 + rename 覆盖」；新增 `a_held_lock_makes_the_second_writer_wait`。 |

范围核对：无 `Cargo.toml`/`Cargo.lock`/依赖改动；除上方两提交外，分支区间只有 `e483a44a` 的计划文档。

### Windows 语义依据

仓外探针（`.w2\probe\probe2.exe`，rustc 1.95 Windows 实跑）：

- **A**：`std::fs::rename` 覆盖一个被读句柄打开的文件 —— OK；旧句柄继续读旧字节，新路径读新字节；
- **B**：`File::lock` 对第二个独立打开的句柄阻塞，直到第一个解锁；
- **C**：write-only 句柄也能取锁（`OpenOptions::new().write(true)` 满足 LockFileEx 的权限要求）。

`File::lock` 自 Rust 1.89 稳定：Unix 映射 `flock`，Windows 映射 `LockFileEx`，两者都是「按打开句柄持有的整文件锁」，句柄关闭或进程被杀即由内核释放。因此**不需要**平台分支的写路径，也不需要锁文件的垃圾回收。

**已知假设与后果**：A 是实测结论，不是 `MoveFileEx` 文档保证。若某环境里 rename 因 sharing violation 失败，`save` 返回 `cant_write` 并删除临时文件 —— 是**显式失败**，不会静默丢表。这条假设已列入 §8 的待补回归测试。

## 4. RED → GREEN 证据

全部日志在 `F:\#Gong\#AI4S\khor\.w2\`（仓外）。仓库只保留本文件与 sha256。

| 步骤 | 命令 | 结果 | 日志 | sha256（前 16） |
| --- | --- | --- | --- | --- |
| RED | `cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1` | **1 passed / 29 failed**（Windows 桩报 unsupported） | `task2-red-tabs.log` | `2A24672A19A1AA08` |
| GREEN | 同上 | **31 passed / 0 failed** | `task3-green-tabs.log` | `5F174E9ABC09AA95` |
| 编译 | `cargo check --locked -p khor-node` | `CHECK_EXIT=0` | `task4-check.log` | `2EADD2AB9A3497C2` |
| 稳定性 | tabs 套件连跑 5 次 | 5×31 passed | `task4-tabs-run-1..5.log` | `24A1DC132C824C2A`… |
| 锁机制 | `tabs::tests::a_held_lock_makes_the_second_writer_wait --exact` | 1 passed | `task4-lock-isolated.log` | `78FA4635A3A70954` |
| 全量（1.100） | `cargo test --locked -p khor-node --lib -- --test-threads=1` | 963 passed / 125 failed / 14 ignored | `task4-full-lib.log` | `0A3BDD04D759EA49` |

### 1.95.0 复验（本轮，CI 钉版）

| 步骤 | 结果 | 日志 |
| --- | --- | --- |
| 测试二进制编译 | `TESTBUILD_EXIT=0`，3m48s | `p1-testbuild.log` |
| tabs 套件 | 31 passed / 0 failed | `p1-tabs.log` |
| 锁测试隔离 | 1 passed | `p1-lock.log` |
| 全量 lib | 963 passed / 125 failed / 14 ignored（342.31s） | `p1-fulllib.log` |
| `cargo fmt --check -p khor-node` | `FMT_EXIT=0`（rustfmt 1.9.0） | `p1-fmt.log` |

1.100 与 1.95 的 125 条失败**逐名完全一致**（`Compare-Object` 差集为空），说明这批失败与工具链无关，是真实的 Windows 适配缺口。

## 5. 全量基线对照（A/B）

| 基线 | 结果 | 说明 |
| --- | --- | --- |
| `582627b0`（W2-A 之前） | lib test **编译失败 8 errors** | 不能作为失败数基线；这本身是 Task 1 的 RED 证据 |
| `936698a6`（编译修复后、写锁改动前） | **931 passed / 156 failed / 14 ignored**（357.21s） | 真正的 A/B 基线 |
| `dc72eaec`（HEAD） | **963 passed / 125 failed / 14 ignored**（342.31s） | 本切片结果 |

差异：`+32 passed`、`−31 failed`、`+1 test`（新增锁测试）。逐名比对结果：

- **新增失败：0 条**；
- 修复 31 条 = 29 条 `tabs::*` + `git::status_tests::a_real_checkout_answers_with_a_branch` + `tests::one_search_reaches_every_kind_and_orders_them_one_way`。后两条与 tabs 表写入恢复同步变绿，是否为直接因果未单独归因（也可能是顺序敏感），不计入本切片的成果声明。

剩余 125 条按模块分布（移交 W2-B 分诊）：

```text
live 34 | tests 21 | session_content 15 | git 11 | khor_agent 9 | satellite 8
usage 4 | workspace 4 | watch 3 | files 3 | vitals 2 | ipc 2 | quota 2 | adaptor 2
wsfacts 1 | delegation 1 | road 1 | place 1 | pipe 1
```

抽样性质（详见后续 triage）：`\\?\F:\...` 扩展前缀传给 `git fast-import` 直接 `Invalid argument`（`git.rs:1945`）；搜索结果分隔符 `\\README.md` vs `README.md`；`os error 5（拒绝访问）`；以及 `/etc/hosts`、`/usr/bin/true`、tmux、FIFO 这类 Linux 专属夹具。

## 6. tabs 覆盖范围

31 条 tabs 测试全部通过，其中与本切片直接相关的三条：

- `two_writers_do_not_lose_each_others_tabs`：4 线程 × 10 次 `open`，断言 41 行一个不少（丢失更新的结果面）；
- `a_held_lock_makes_the_second_writer_wait`：持锁期间第二个写者必须阻塞（丢失更新的机制面）；
- `a_write_leaves_no_draft_behind`：成功后不残留 `tabs.json.<pid>`。

## 7. 独立评审裁定

评审方式：fresh 只读评审（独立上下文），只看 diff 与仓外证据，不运行 cargo、不修改文件。
结论：**PASS_WITH_NITS** —— 代码层面无阻断，Unix 语义经逐行 diff 证明等价（去掉 `#[cfg(unix)]` 不改变任何被编译的 item）。

| # | 意见 | 裁定 | 去向 |
| --- | --- | --- | --- |
| 1 | Windows CI 没有 tabs/全量 lib 回归保护，桩被加回来也不会红 | 接受 | W2-C：Windows job 增加 `cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1` |
| 2 | 验收文档缺失；建议仓内加「读者持有句柄期间 save/rename」回归测试 | 文档本轮提交；测试接受 | W2-C（测试） |
| 3 | 真实双进程（GUI + `khor tabs`）未测 | 接受为 NOT_TESTED | W3 smoke |
| 4 | 新锁测试缺握手，可能空洞通过 | 接受 | W2-C（先置 `started` 再断言） |
| 5 | `link_pairing_tests` 的 Windows 分支仅 `#[ignore]` 用例可达 | 接受为 NOT_TESTED | 记录 |
| 6 | `migrate_place_pins` 标记读改写不在锁内，首次双进程可重复迁移 | 接受 | W2-C backlog |
| 7 | 进程在 write 与 rename 间被杀会残留 `tabs.json.<pid>` | 接受（Unix 既有模式，Windows 新可达） | W2-C |
| 8 | GUI 侧 Tauri 同步 command 里做阻塞锁，争用可能卡 UI 线程 | 接受 | W3 评估 `#[tauri::command(async)]`/`spawn_blocking` |
| 9 | `Hold` 不可重入无防护，未来 with 闭包内再写会自锁 | 接受 | W2-C（文档 + debug 断言） |
| 10 | 隐式 MSRV 变为 1.89（`File::lock`） | 接受为记录项 | 本文档 §2；不改 `Cargo.toml` |
| 11 | `stored()` 把读错误当空表，外部占用时可能覆盖全表 | 接受，后续 hardening | 不在本切片 |
| 12 | 计划写「`.khor/tabs.lock` 不得残留」，与常驻设计冲突 | 以本文档澄清 | **锁文件常驻是设计**（只创建、不重写、内核锁）；要求无残留的是临时文件 `tabs.json.<pid>` |

## 8. NOT_TESTED（显式声明，不作通过）

1. Linux/macOS 的构建与回归（Unix 等价性仅由逐行 diff 证明）；
2. Windows 真实双进程（GUI 窗口 + `khor tabs` CLI）并发 read-modify-write；
3. Windows 跨进程 `LockFileEx` 的阻塞与释放（文档语义 + 同进程实测，无跨进程实测）；
4. 持锁者被杀死后下一个写者能进入（只有「第二写者被阻塞」的测试）；
5. 仓内「读者持有句柄期间 save/rename」回归测试（目前只有仓外探针）；
6. Windows 长路径（>MAX_PATH）、`tabs.json` 只读/ACL 拒绝 DELETE、第三方程序以无 `FILE_SHARE_DELETE` 打开时的 rename 行为；
7. 被 `#[cfg(unix)]` 跳过的用例：journal FIFO、satellite launcher（skipped，不是 passed）；
8. `link_pairing_tests` 的 Windows kill 断言（仅 `#[ignore]` 用例可达）；
9. Windows CI 上的 tabs/全量 lib 测试（CI 未配置，本轮只有本地证据）；
10. macOS GUI 双进程手工验收；
11. 崩溃/断电后 `.khor/tabs.lock` 与 `tabs.json.<pid>` 的残留与清理；
12. Windows 10 兼容（按用户决定延后到 W7）。

## 9. 回滚点

- 分支回退：`582627b0`（W2-A 之前的文档 HEAD）；
- 单提交回退：`git revert dc72eaec` 会恢复 Windows 的 unsupported 写桩（**不要**在生产分支回退到该状态）；
- 本切片不改变磁盘格式：`tabs.json` 结构、`.khor` 目录布局、锁文件名均为新增或不变。

## 10. 下一步

1. **W2-B**：125 条失败分诊（产品 bug / 测试可移植 / 环境）→ 分批修复，每批 RED→GREEN；
2. **W2-C**：本文件 §7 中标记 W2-C 的评审收尾项；
3. **W3**：进程/终端/文件 smoke，含真实双进程 tabs 场景。
