# W2-E：上游同步与 Windows 文件身份收口

## 1. 结论（2026-11-15）

[W2-B](W2-B-NODE-SUITE.md) §6 登记的 4 条剩余失败已全部处置：

- 3 条 `live` 期望落后于上游（`84c9e6fe` 取消 vendor 生成名、`9e3f33d1` 让原生 Claude 证据压过报告与屏幕），
  按上游行为对齐断言，并补一条直接测排序的单测；
- 1 条 W1 记录的 fanout flake 本轮未复现。

处置过程中又暴露并修掉两处与 Windows 相关的问题：提交页测试耦合了仓库 HEAD 的文件大小，
transcript 文件身份在 Windows 上用创建时间（NTFS 隧道会让重建文件沿用旧时间）。

**验收：PASS**（`full` 权限、工作区外构建，ConPTY 修复后的一次全量）

```text
cargo test --locked -p khor-node --lib -- --test-threads=1
1061 passed; 0 failed; 14 ignored
cargo fmt --check -p khor-node        exit 0
```

（此前一次全量里 `khor_agent::accounts::tests::cancelling_account_registration_while_root_is_busy_does_not_publish_account`
在串行满载时红过一次，单独 `--exact` 通过；最后一次全量未复现，与 W1 的 fanout flake 同类。）

## 2. 逐条处置

| # | 测试 | 原判定 | 根因 | 处置 | 提交 |
| --- | --- | --- | --- | --- | --- |
| 1 | `live::tests::a_session_that_is_both_hooked_and_discovered_is_one_row` | 预存红 | 上游 `84c9e6fe` 不再用 vendor 生成的 `name`，行标题取目录名；断言仍期望 `one-aa` | 断言改为 `one` | `7195c55c` |
| 2 | `live::tests::a_word_read_off_the_screen_does_not_cover_what_the_vendor_says` | 预存红 | 上游 `9e3f33d1` 让原生 Claude 证据压过 hook 报告；断言仍期望 `Done/1` | 断言改为 `Blocked/0`，写明来源优先级 | `7195c55c` |
| 3 | `live::tests::a_row_written_before_provenance_existed_is_treated_as_reported` | 预存红 | 同上；无 provenance 的旧条目仍算报告，但不再压过原生证据 | 断言改为 `Blocked`，并补单测 `reported_words_outrank_the_disk_and_screen_words_do_not` 直接钉住 `outranks_disk` 排序 | `7195c55c` |
| 4 | `git::status_tests::head_answers_its_files_with_both_sides` | 本轮新红 | 测试读**本仓库 HEAD** 的改动文件；`SHOWN = 256 KiB`，而 HEAD 只改了 337,919 B 的 `live.rs`，没有可用的"小文本文件" | 改为在临时仓库用 fast-import 造两个提交，第二次只改一个 6→7 字节的文本文件 | `fb47796f` |
| 5 | `adaptor::transcript_page::tests::prepared_io_does_not_parse_after_cancellation_and_replacement_still_resets` | 本轮新红 | Windows 上 `FileVersion` 用创建时间做文件身份；NTFS 隧道（删除/移走同名文件后 ~15s 内重建会沿用旧创建时间）让被替换的文件"看起来没变"，`changed()` 返回 false，游标不重置 | 改用卷序列号 + 文件索引（`GetFileInformationByHandle`，稳定版 API） | `fb47796f` |

一条旁证（2026-11-15 实测）：在沙箱的私有临时目录里 `rename → 重新创建同名文件`，`st_ctime_ns` 前后完全相同，
即隧道效应确实存在；`FileVersion` 的旧实现因此无法识别替换。

## 3. 环境边界：受限令牌父进程与命名管道

以下两条在本机（DSH `workspace-write` 沙箱）内必红，属**环境**而非产品回归：

```text
khor_agent::accounts::tests::cancelling_account_registration_while_root_is_busy_does_not_publish_account
khor_agent::accounts::tests::pi_login_keeps_credentials_local_reuses_accounts_and_switches_only_future_turns
→ "khor agent 无法启动：拒绝访问。 (os error 5)"
```

定性证据（2026-11-15，本机）：

1. agent sidecar 的 stdio 走命名管道（`khor_agent_process_windows.rs` 的 `CreateNamedPipeW` + 客户端连接）。
2. 同一受限令牌下直接建管道并连接：
   ```text
   NamedPipeServerStream + NamedPipeClientStream.Connect(800)
   → Access to the path is denied.
   ```
3. DSH 沙箱包文档明确：两类受限模式都拒绝命名管道打开（只有匿名管道可用）。

结论：**W2 验收以文档环境（普通终端、完整用户令牌）为准**；这两条在沙箱里记为 ENV_BLOCKED，
不计入产品失败。W3 的进程/终端 smoke 同样必须在非沙箱环境执行，否则会把环境限制误判成产品缺陷。

## 4. 验收命令

```text
cargo test --locked -p khor-node --lib -- --test-threads=1     1055 / 2(env) / 14
cargo test --locked -p khor-node --lib git::                   35 passed
cargo test --locked -p khor-node --lib adaptor::transcript_page::  17 passed
cargo fmt --check -p khor-node                                 exit 0
```

## 5. 未决

- W3 进程/终端 smoke 需要非沙箱环境（普通 PowerShell 或 DSH `full` preset）；沙箱内的红先按环境分类，不写进产品缺陷。
- `cargo check --locked -p khor-node` 本轮未单独重跑（全量 lib 测试已完成编译）；W2 收尾时补一次。
