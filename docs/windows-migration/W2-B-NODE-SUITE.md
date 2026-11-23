# W2-B：Windows 节点套件收敛验收

## 1. 结论

`crates/node` 的全量 lib 测试在 Windows 11 x64 + rustc 1.95.0 上从 **156 条失败**收敛到 **4 条**，其中 **3 条是平台无关的预存红**（在 W2-A 之前的基线上就是红），**1 条是 W1 已记录的顺序/负载敏感 flake**。没有一条是本次改动引入的。

**验收：PASS_WITH_EXCEPTIONS**

```text
cargo test --locked -p khor-node --lib -- --test-threads=1
1048 passed; 4 failed; 14 ignored
cargo check --locked -p khor-node   exit 0
cargo fmt --check -p khor-node      exit 0
```

## 2. 轨迹（同一命令，rustc 1.95.0）

| 节点 | 结果 | 说明 |
| --- | --- | --- |
| `936698a6` 基线（W2-A 之前） | 931 / 156 / 14 | 编译修复后、写锁改动前 |
| `dc72eaec` W2-A 之后 | 963 / 125 / 14 | tabs 写锁 |
| `0236678a` B4 之后 | 1032 / 38 / 14 | 路径、句柄、夹具 |
| `d42d4d46` B5 之后 | 1047 / 5 / 14 | usage/quota/place/road/ipc/watch |
| `46f6f81a` 本文件 HEAD 前 | 1048 / 4 / 14 | watch 与 attempt 快照 |

## 3. 产品 bug（真修，不是 gate）

| # | 位置 | 症状 | 修法 | 提交 |
| --- | --- | --- | --- | --- |
| 1 | `git` 测试夹具 | `fast-import` 拒绝 `\\?\F:\...` 的 `--export-marks`（Invalid argument） | 交给外部程序的路径剥掉 verbatim 前缀 | `592c937b` |
| 2 | 7 处「写完再只读打开 fsync」 | Windows 拒绝在读句柄上 FlushFileBuffers（os error 5） | `durable::sync_file`：写句柄打开再 sync | `e4c2394a` |
| 3 | `session_content/journal.rs` | 记录文件句柄在 pending 目录内，rename/remove 目录被拒（os error 5） | 句柄改 `Option<File>`，rename/remove 前释放、rename 后在新位置重开 | `e4c2394a` |
| 4 | 夹具的 `set_modified`/`set_times` | 只读句柄不能设时间（os error 5） | 写句柄打开 | `e4c2394a`、`0236678a` |
| 5 | `workspace::basename` | 只按 `/` 切，Windows 行名变成整条路径 | 按平台分隔符切 | `0236678a` |
| 6 | 搜索结果期望 | 返回 `\` 分隔的路径 | 测试按平台分隔符归一比对（产品返回原生路径是正确行为） | `afcf6656` |
| 7 | `place::shorten_under` | 只认 `{home}/` 前缀，Windows home 以 `\` 结尾永不匹配 | 分隔符单独取，剩余部分统一用 `/` 书写 | `d42d4d46` |
| 8 | `ipc::call_refreshing` | Windows 上拒绝连接要 ~2s（本机探针实测 2.02s），500ms 预算被第一次连接吃光，刷新端点永远试不到 | 每次连接自身受预算约束；超时与拒绝一样只问一次端点文件 | `d42d4d46` |
| 9 | `live` 进程 cwd | sysinfo 在 Windows 返回带尾分隔符的目录，行里按字符串比较失败 | 归一化（`components().collect()`） | `d42d4d46` |
| 10 | `watch::watches_directory` | 按原始拼写查表；且 Windows 一个递归句柄覆盖整棵子树 | 双端解析后比较；递归句柄对其子目录也算已观察 | `d42d4d46`、`46f6f81a` |
| 11 | `quota::account_home` | 无 Windows 分支 → `as_the_account()` 恒 false，测试绊线永不触发 | Windows 用 `USERPROFILE` | `d42d4d46` |

## 4. 测试侧修复（夹具按平台写）

- `usage` 夹具用 Unix 符号链接搭一个假 home（`#[cfg(unix)]`），Windows 得到空 home，所有计数为 0 → `mount()` 在 Unix 链接、Windows 复制。
- `testproc`：`sleep`/`true` 在 Windows 不存在 → `ping`/`cmd exit` 等价进程。
- `absolute()`：`/Users/x/proj` 这类 Unix 绝对路径在 Windows 是**相对**路径，节点按"必须是绝对路径"拒绝 → 夹具改用平台认可的绝对路径。
- recovery 快照：跳过 `*.lock`（其字节不是状态），其余被写者锁住的文件等待或跳过，而不是整趟失败。
- 设置页期望补上 `window-*` 三项（该测试在基线也是红，属预存期望落后）。

## 5. 显式 gate（`#[cfg(unix)]`，计入 NOT_TESTED）

共 **34** 条，全部是"Windows 上该能力不存在"而非"跑不过"：

| 位置 | 条数 | 理由 |
| --- | --- | --- |
| `adaptor::tmux` + `live::tests` 的 tmux 用例 | 16 | 夹具是 `#!/bin/sh` 的假 tmux；tmux 本身不存在于 Windows |
| `satellite::script_tests` 与两条脚本对比 | 8 | 卫星脚本经 POSIX `sh` 执行（`sh -c`、`find/grep` 脚本） |
| `pipe::tests::the_proxy_line_survives_a_shell` | 1 | ssh `ProxyCommand` 的 POSIX shell 引号语义 |
| `files::make_tests::a_machine_with_no_opener_...` | 1 | 用 `/usr/bin/true` 作 opener |
| `vitals` 两条磁盘卷断言 | 2 | 读挂载表（`/proc/mounts` 语义），Windows 无对应 |
| `tests::finding_the_program` | 5 | 登录 shell / `/bin` / launchd PATH 语义 |
| `watch::native::aliases_...` | 1 | 夹具根是 POSIX 绝对路径（`/real/root` → `/alias/root`） |

## 6. 剩余 4 条失败（明确不修 / 不属本切片）

> 2026-11-15 更新：下表 4 条已全部处置，见 [W2-E](W2-E-UPSTREAM-SYNC.md)。3 条 `live` 期望对齐上游
> （`7195c55c`）；fanout flake 未复现；处置过程中另修两处 Windows 问题（`fb47796f`），
> 当前全量 `1055 passed / 2 failed / 14 ignored`，两条红均为受限令牌父进程禁命名管道的环境项。

| 测试 | 状态 | 定性 |
| --- | --- | --- |
| `live::tests::a_session_that_is_both_hooked_and_discovered_is_one_row` | 红 | **预存**（基线 936698a6 即红）。上游 `84c9e6fe` 删掉了 claude 适配器读 `name` 字段的逻辑（"directory identity ignores the vendor's generated name"），但该测试仍期望 `one-aa`；属上游期望未同步，与平台无关 |
| `live::tests::a_word_read_off_the_screen_does_not_cover_what_the_vendor_says` | 红 | **预存**（基线即红）；同一处 vendor 词/来源排序的期望未随上游改动同步 |
| `live::tests::a_row_written_before_provenance_existed_is_treated_as_reported` | 红 | **预存**（基线即红）；同上 |
| `gui_host::packet_fanout::tests::a_history_producer_waits_for_real_capacity_without_retiring_the_reader` | 偶发 | **W1 已记录的负载/顺序敏感 flake**（W1 文档 §7.2；远端 windows-2025 CI 10/10 通过）；本机满载时会红，隔离也偶发 |

处理决定：三条预存红需要**产品语义决策**（`name` 字段是否恢复），不属于 Windows 平台适配；已登记到 `W2-E`（上游同步）并写进 NOT_TESTED 清单。flaky 一条保留，W2-C 里给 fanout 加确定性等待。

## 7. 环境前提（复现这份结果需要）

- rustc/cargo **1.95.0**（CI 钉版），全部装在 `F:\khor-toolchain`；`CARGO_TARGET_DIR=F:\khor-target-w2`，`TMP=TEMP=F:\Temp`，先 `vcvars64`。
- **Node 在 PATH 上**：`tests` 里的 agent 账号用例会以 `node apps/agent/src/host.ts` 起 sidecar。
- **`apps/agent` 依赖已安装**：本机用 `pnpm --dir apps/agent install --store-dir F:\khor-toolchain\pnpm-store` 装的（pnpm 报 ignored build scripts，不影响 host.ts 运行）。CI 由 setup-node + 包管理器负责。
- 未经这些前提跑出的红，先归类为环境问题，不要当产品回归。

## 8. 回滚点

- 本切片起点：`936698a6`（W2-A 之后、W2-B 之前是 `dc72eaec`）。
- 逐片回滚：`592c937b` → `e4c2394a` → `afcf6656` → `d42d4d46` → `46f6f81a`，每片独立、可单独 revert。

## 9. 下一步

- **W2-C**：W2-A 评审收尾 6 项（Windows CI 跑 tabs 套件、仓内 rename-vs-reader 回归、锁测试握手、place-pins 幂等、临时文件清理、`Hold` 重入防护）+ fanout 确定性等待。
- **W2-D**：`khor serve` keeper、`khor quit`、attach 原始终端（ConPTY）、`secret` askpass、`foreground_group`/`tty_path`。
- 之后按 [ROADMAP](ROADMAP.md) 进 W3。
