# W2-A：Windows tabs 写锁 + Windows 可编译节点测试 — 实施计划

> **状态（2026-11-14）：已实施并通过验收** —— 代码提交 `936698a6`、`dc72eaec`，验收证据与评审裁定见 [W2-TABS-WINDOWS-LOCK](W2-TABS-WINDOWS-LOCK.md)。下文勾选框保留为实施时的原始步骤记录，不再逐项回填。

> **For agentic workers:** REQUIRED SUB-SKILL: Use executing-plans（inline 执行）或 subagent-driven-development。步骤用 `- [ ]` 跟踪。

**Goal:** 让 Windows 上 `khor tabs` 与窗口两个进程的 read-modify-write 由真实文件锁串行化（不再静默丢标签），并让 `crates/node` 的 lib 测试在 Windows 上可编译、可运行。

**Architecture:** 复用 Rust 1.89+ 标准库 `std::fs::File::lock`（Unix `flock` / Windows `LockFileEx`）作为跨平台事务边界：`change()` 在读取前取独占锁，读-改-写全程持锁；`save()` 继续用“临时文件 + rename 覆盖”保证读者看不到半份 JSON。Windows 的 `rename` 可覆盖已被打开的文件（已用 rustc 探针实测），因此不需要平台分支的写路径。唯一需要平台分支的是测试里取“另一个存活进程 PID”的代码（Unix `getppid`，Windows `sysinfo` 父进程）。

**Tech Stack:** Rust 2024 / rustc 1.100.0 / MSVC BuildTools 2022 / cargo（`--locked`）；Khor workspace crate `khor-node`；测试为 Rust 内建 `#[test]`（lib unit tests）。

**Spec / 契约来源:**
- 原始任务契约（task-7）：`实现 Windows tabs 写锁：替换 tabs.rs 中 unsupported stub，使用 Rust std File::lock/unlock（当前 rustc 1.100 已在 Windows 支持）或等价 Windows 文件锁，串行化窗口与 khor tabs 的 read-modify-write；Unix 行为保持不变；补/修 Windows 可编译测试并验证。只改 tabs.rs 及必要测试模块。`
- `docs/SHELL.md` 第 2 条（窗口与 `khor tabs` 两个进程共用一个 tabs 表）。
- `docs/code-quality/ENGINEERING.md`（验证按变化选择、错误与状态归属、注释保留平台/并发不变量）。
- 候选实现参考（只读输入，非直接采纳）：`38c768db`、`242e8e8e`、`fd5380f7`（Steward task-7 产出；final 版本采用统一锁，但 `tests.rs` 改动不完整，需重新用 TDD 验证）。

## Global Constraints

- 只在 `F:\#Gong\#AI4S\khor\worktrees\s2-windows`（分支 `steward/s2-windows-20261114`）修改；不 push、不合 main、不签名、不发布。
- 不新增依赖；不手改 `Cargo.lock`。
- Unix 行为必须保持不变（只去掉 cfg 门；unix 语义不变）。
- 构建/测试必须带 `--locked`；构建环境显式注入 vcvars64 + cargo/NASM PATH + `TMP/TEMP=F:\Temp` + `CARGO_TARGET_DIR=F:\khor-target-w2`（不再使用旧 target 目录，避免跨分支缓存污染）。
- 任何“通过/修复”的声明必须附本次运行的命令与完整输出证据（verification-before-completion）。
- Windows 上“未运行”的检查（Linux/macOS 回归、真实 GUI 双进程）明确标注 `NOT_TESTED`，不伪装成通过。
- 修改行为前先写/确认失败测试；每个任务结束提交一次，提交信息说明验证命令。

## Review Focus

1. **两个写者并发（窗口 + `khor tabs`）**：40 个并发 `open` 后必须一个不少（`two_writers_do_not_lose_each_others_tabs`）。
2. **读者在写者进行中读取**：`list` 绝不能读到半份 JSON 而回退成空表（rename 原子替换；Windows 探针已验证旧句柄读旧内容、新路径读新内容）。
3. **进程在持锁中被杀**：锁随句柄释放，下一个写者不得等待或清理陈旧锁（不引入锁文件垃圾回收）。
4. **非 Windows 构建**：Linux/macOS 的 libc 分支必须保持原样；本机不能跑，标记 NOT_TESTED，靠 CI 兜底。
5. **Windows 文件语义**：`.khor/tabs.lock` 与 `tabs.json.<pid>` 临时文件不得残留；长路径/无 `.khor` 目录时自动创建。

---

### Task 1: Windows 下让 `cargo test -p khor-node --lib --no-run` 编译通过

**Files:**
- Modify: `crates/node/src/tests.rs:445-465`（Unix `getppid` / Windows `sysinfo` 父进程）
- Modify: `crates/node/src/mcp_worker.rs:250-253`（`#[cfg(test)] pid()` → `#[cfg(all(test, unix))]`，Windows 无 `Session::pid`）
- Modify: `crates/node/src/journal.rs`（`mkfifo` 测试加 `#[cfg(unix)]`）
- Modify: `crates/node/src/satellite.rs`（`/usr/bin/true` + unix symlink 的 launcher 测试加 `#[cfg(unix)]`）
- Modify: `crates/node/src/link_pairing_tests.rs:182-190`（`kill_wait` 的 `ExitStatusExt::signal` 按平台分支）
- Log: `F:\#Gong\#AI4S\khor\.w2\task1-red-compile.log`、`task1-green-compile.log`（不在仓库内）

**Interfaces:**
- Consumes: 无
- Produces: 可编译的 `khor-node` lib 测试二进制（后续所有测试步骤的前提）

**基线实测：8 个编译错误 / 5 个文件**（`task1-red-compile.log`；只列首错位置）：
`tests.rs:447 libc`、`link_pairing_tests.rs:182/189 os::unix+signal`、`journal.rs:1860/1864/1866 os::unix+as_bytes+libc`、`satellite.rs:3040 os::unix`、`mcp_worker.rs:252 Session::pid`。
原则：**能移植的移植（父进程 PID、kill 退出码），Unix 语义的测试显式 `#[cfg(unix)]` 并计入 NOT_TESTED**（Windows-systems 技能：cfg(unix) 测试是 skipped，不是 passed）。

- [ ] **Step 1: 记录 RED（编译失败是这一步的“失败测试”）**

Run: `cargo test --locked -p khor-node --lib --no-run`（Windows/MSVC 环境）
Expected: FAIL，8 个错误（保存完整日志）。

- [ ] **Step 2: 按上面的 5 处修改**

（`tests.rs` 用 `#[cfg(unix)]` 保留 `libc::getppid`，`#[cfg(windows)]` 用 `sysinfo::System::new_all()` → `get_current_pid()` → `process(...).parent()`；
`mcp_worker.rs` 的 `#[cfg(test)] fn pid` 改为 `#[cfg(all(test, unix))]`；
`journal.rs`/`satellite.rs` 两个 Unix-only 测试加 `#[cfg(unix)]`；
`link_pairing_tests.rs` 的 `kill_wait`：Unix 断言 `signal()==Some(9)`，Windows 断言 `status.code().is_some()`。）

- [ ] **Step 3: 记录 GREEN（编译通过）**

Run: `cargo test --locked -p khor-node --lib --no-run`
Expected: exit 0（保存完整日志）。

- [ ] **Step 4: 运行受影响的单个测试（行为验证）**

Run: `cargo test --locked -p khor-node --lib tests::a_word_reported_by_another_process_loses_to_the_vendors_live_file -- --exact --nocapture`
Expected: `1 passed`（Windows 上也要通过）。

- [ ] **Step 5: 提交**（提交信息见初版；附完整验证命令）

---

### Task 2: RED — 证明 Windows 上 tabs 写入当前被拒绝

**Files:** 无修改（只记录证据）
**Log:** `F:\#Gong\#AI4S\khor\.w2\task2-red-tabs.log`

- [ ] **Step 1: 跑 tabs 单元测试（基线版本 = 未改 tabs.rs）**

Run: `cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1`
Expected: FAIL；`open(...).unwrap()` 在 Windows 上 panic，因为 `#[cfg(not(unix))] change()` 返回
`"tabs writes are unsupported on Windows: no flock to serialize the window and `khor tabs`"`。
记录失败测试清单（至少包含 `two_writers_do_not_lose_each_others_tabs`、`a_write_leaves_no_draft_behind`）。

- [ ] **Step 2: 固化 RED 证据**

把失败清单与关键 stderr 摘要写入 `F:\#Gong\#AI4S\khor\.w2\task2-red-tabs.log`；在 ledger 记录
`Task 2: RED (Windows tabs writes refused; N tests failing)`。

---

### Task 3: GREEN — 统一跨平台文件锁与原子保存，并加确定性锁测试

**Files:**
- Modify: `crates/node/src/tabs.rs`（`save`/`Hold`/`impl Drop`/`change` 去掉 `#[cfg(unix)]`；删除 `#[cfg(not(unix))] change` 桩；注释改为跨平台事实）
- Test: `crates/node/src/tabs.rs`（tests 模块新增 `a_held_lock_makes_the_second_writer_wait`）
- Log: `F:\#Gong\#AI4S\khor\.w2\task3-green-tabs.log`

**Interfaces:**
- Produces: `struct Hold`（所有平台同签名 `fn take(&Path) -> Result<Hold, String>`，Drop 释放）；`fn save(&Path, &BTreeMap<String, Open>) -> Result<(), String>`（所有平台同语义：临时文件 + rename 覆盖）；`fn change<T>(&Path, &str, impl FnOnce(&mut Open) -> Result<T,String>) -> Result<T,String>`（所有平台同签名，持锁跨越读与写）。
- 被 `save_all` / `reorder` 等既有调用方消费（签名不变）。

- [ ] **Step 1: 实现统一锁与保存**

删除 `save`、`Hold`、`impl Hold`、`impl Drop for Hold`、`change` 上的 `#[cfg(unix)]` 与
`#[cfg(not(unix))]` 桩；`Hold::take` 使用
`OpenOptions::new().create(true).truncate(false).write(true).open(lock_path)` + `File::lock()`；
Drop 保持 `unlock()`；`change` 保持“先取锁，再 stored/with/settle/save”。

注释以已实测的 Windows 语义重写：

```text
// Windows' std::fs::rename replaces the destination even while a reader has it
// open (the reader keeps the old contents), so the atomic-replace save above is
// correct on both Unix and Windows. std::fs::File::lock maps to flock on Unix
// and LockFileEx on Windows, giving the same read-modify-write boundary.
```

- [ ] **Step 2: 新增确定性锁阻塞测试（放在 `two_writers_do_not_lose_each_others_tabs` 之后）**

```rust
    /// **A second writer waits for the lock the first one holds.**
    ///
    /// `two_writers_do_not_lose_each_others_tabs` shows the outcome; this
    /// shows the mechanism — if the lock were a no-op both threads would
    /// run `change` at once and this test would finish early.
    #[test]
    fn a_held_lock_makes_the_second_writer_wait() {
        let r = root("lock-waits");
        let held = Hold::take(&r.0).unwrap();
        let here = r.0.clone();
        let at = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let finished = at.clone();
        let waiter = std::thread::spawn(move || {
            let second = Hold::take(&here).unwrap();
            finished.store(true, std::sync::atomic::Ordering::SeqCst);
            drop(second);
        });
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            !at.load(std::sync::atomic::Ordering::SeqCst),
            "the second writer ran while the lock was held"
        );
        drop(held);
        waiter.join().unwrap();
        assert!(at.load(std::sync::atomic::Ordering::SeqCst));
    }
```

- [ ] **Step 3: 运行 tabs 测试（GREEN）**

Run: `cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1`
Expected: 全部通过（无 failed；记录总数）。

- [ ] **Step 4: 单独运行新增锁测试**

Run: `cargo test --locked -p khor-node --lib tabs::tests::a_held_lock_makes_the_second_writer_wait -- --exact --nocapture`
Expected: `1 passed`。

- [ ] **Step 5: 提交**

```bash
git add crates/node/src/tabs.rs
git commit -m "fix(node): serialize Windows tabs writes with std file locks

Windows kept the unsupported-write stub because it had no flock. Rust std
File::lock is flock on Unix and LockFileEx on Windows, and std::fs::rename
replaces the table even while a reader holds it, so the same lock+rename
transaction works on both platforms. Removes the platform split and adds a
test that a held lock actually blocks the second writer.

Verified: cargo test --locked -p khor-node --lib tabs:: (Windows MSVC)"
```

---

### Task 4: 极限验证（全量测试、重复跑、格式、check）

**Files:** 无修改（只验证；若发现本任务引入的问题，回到 Task 3 修并补测试）
**Log 目录:** `F:\#Gong\#AI4S\khor\.w2\`

- [ ] **Step 1: crate 编译（契约验收命令）**

Run: `cargo check --locked -p khor-node`
Expected: exit 0。

- [ ] **Step 2: tabs 测试重复 5 次（并发/时序稳定性）**

Run: `for i in 1..5 { cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1 }`
Expected: 5 次全绿（记录每次的 pass/fail 计数）。

- [ ] **Step 3: 全量 node lib 测试（极限）**

Run: `cargo test --locked -p khor-node --lib`
Expected: 记录完整结果；若存在与本改动无关的 Windows 预存失败，逐个列出并分类（不修，留作后续 W2 任务），并在验收文档写 `NOT_TESTED`/预存失败清单。

- [ ] **Step 4: 格式检查**

Run: `rustup component add rustfmt`（若缺失）→ `cargo fmt -- --check`（或仓库 `python scripts/check-rust-format.py --check`）
Expected: 仅检查本改动文件无格式差异；若工具版本不满足仓库固定版本，明确记录并改用 `cargo fmt --check` 的实际结果 + `NOT_TESTED` 说明。

- [ ] **Step 5: 工作树自检**

Run: `git status --short`；`git diff`（确认只含 `tabs.rs` + `tests.rs` + 文档）
Expected: 无意外文件；无 Cargo.lock 改动。

---

### Task 5: 验收记录与提交

**Files:**
- Create: `docs/windows-migration/W2-TABS-WINDOWS-LOCK.md`

内容必须包含：目标、改动文件、Windows 语义探针结果（rename 覆盖已打开文件 OK；`File::lock` 跨句柄阻塞 OK；write-only 句柄可锁 OK）、RED/GREEN 证据表（命令、退出码、通过数、日志路径与 sha256）、全量测试结果与预存失败清单、`NOT_TESTED`（Linux/macOS 回归、真实 GUI 双进程手工验收）、回滚点（基线 HEAD 582627b0）、下一步建议。

- [ ] **Step 1: 写文档**
- [ ] **Step 2: 提交**

```bash
git add docs/windows-migration/W2-TABS-WINDOWS-LOCK.md
git commit -m "docs(windows): record W2 tabs write-lock acceptance evidence"
```

---

### Task 6: 终审与收尾

- [ ] **Step 1: 全分支 review**（fresh reviewer：di 一次独立只读评审，按 `requesting-code-review` 的 code-reviewer 模板；使用 Codex 子代理 / kapestack astra max）
- [ ] **Step 2: 逐条裁定 review 意见并记录 Ruling**
- [ ] **Step 3: 汇总最终报告给用户（不含 push/merge）**

---

## 自审（写计划后）

- **Spec 覆盖**：Windows 写锁（Task 3）、Unix 不变（Task 3 仅去 cfg + Task 4 记录 NOT_TESTED）、Windows 可编译测试（Task 1）、验证（Task 4/5）、只改 tabs.rs 及必要测试模块（Task 1 的 tests.rs 属于“必要测试模块”）。
- **Review Focus 覆盖**：Focus 1→`two_writers…`；Focus 2→`a_write_leaves_no_draft_behind` + rename 探针 + 读句柄语义；Focus 3→Drop unlock + 无锁文件回收设计；Focus 4→全局约束 + NOT_TESTED；Focus 5→`a_write_leaves_no_draft_behind` + `.khor` 创建逻辑不变。
- **比例**：计划约 200 行，对应 ~40 行产品代码 + ~30 行测试；未代写实现细节（除锁测试骨架）。
- **遗留决策**：`sysinfo` 具体 API 调用以编译器为准（Task 1 允许最小调整）；全量测试若出现与本任务无关的 Windows 预存失败，只记录不修（避免范围膨胀），并作为 W2 后续任务输入。
