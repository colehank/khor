# W2-C：tabs 写锁加固与评审收尾

W2-A 的独立评审给出 12 条意见，其中 6 条被裁定为"本切片之后立刻做"。本文件是这 6 条的完成记录，外加一条已知 flake 的处置。

**验收：PASS（本地 Windows 11 x64 / rustc 1.95.0）**

```text
cargo test --locked -p khor-node --lib tabs::  34 passed; 0 failed
cargo fmt --check -p khor-node                 exit 0
```

## 1. 逐条处置

| # | 评审意见 | 处置 | 证据 |
| --- | --- | --- | --- |
| C1 | Windows CI 没有 tabs 回归保护，写路径被桩回去也不会红 | `.github/workflows/windows-w1.yml` 增加 `Node lib tests (tabs write lock)`：`cargo test --locked -p khor-node --lib tabs:: -- --test-threads=1` | 工作流文件；远端执行 **NOT_RUN**（本分支未 push） |
| C2 | 计划要求仓内「读者持句柄期间 save/rename」回归测试 | 新增 `a_reader_keeps_the_bytes_it_opened_and_the_next_open_sees_the_new_list`：持句柄读旧字节、新 open 读新列表 | `tabs::` 34/34 |
| C3 | 新锁测试缺握手，线程没被调度到时断言空洞成立 | 第二写者先置 `started` 再取锁；主线程等 `started` 后才断言"尚未运行" | `a_held_lock_makes_the_second_writer_wait` |
| C4 | `migrate_place_pins` 的标记读写在锁外，两进程可重复迁移出孪生页签 | `tabs::adopt_pinned` 改为幂等：同 device+url 已 pin 则返回既有 id；新增 `adopting_one_place_twice_is_one_tab` | `tabs::` 34/34；`place_pins` 1/1 |
| C5 | 进程在 write/rename 之间被杀会残留 `tabs.json.<pid>` | `save` 在持锁的成功路径上清扫其它 `tabs.json.*` 草稿（锁串行化使它们必然已死）；新增 `a_stale_draft_is_swept_by_the_next_write` | `tabs::` 34/34 |
| C6 | `Hold` 不可重入无防护，未来在 `with` 闭包里调写函数会自锁 | `change()` 加 debug 重入断言（`IN_CHANGE` 线程局部）；未加运行时锁，避免把测试期防护带进产品热路径 | 代码注释 + debug 构建 |

## 2. 未在 C 阶段解决、已登记的项

| 项 | 状态 | 去向 |
| --- | --- | --- |
| `gui_host::packet_fanout::a_history_producer_waits_for_real_capacity…` 偶发红 | 本机满载/隔离偶发；W1 已记录为顺序/负载敏感 flake，远端 windows-2025 CI 曾 10/10 | 保留为已知 flake；若 W3 的 smoke 再撞到，就在 fanout 里换成条件轮询而不是固定等待 |
| 3 条平台无关预存红（claude 适配器 `name` 语义） | 基线 `936698a6` 即红，上游 `84c9e6fe` 改了产品语义但测试期望未同步 | `W2-E`（上游同步），不属于 Windows 适配 |
| `Hold` 重入只有 debug 断言 | 产品构建不 panic；未来若真有人嵌套写，表现为挂起 | 已在注释里写明约束；W6 之前再评估是否升级为返回错误 |

## 3. 对 W2-A 验收文档的修正

- W2-A §7 第 12 条曾澄清「`.khor/tabs.lock` 常驻是设计」；C5 之后补充：常驻的只有锁文件，**草稿文件**（`tabs.json.<pid>`）在成功保存时会被清扫。
- W2-A §8 第 5 条（仓内缺 rename-vs-reader 回归）由 C2 关闭。

## 4. 回滚

- 本切片：`46f6f81a` 之后、`4e106479`（W2-B 文档）之上的独立提交，可单独 revert；不影响 W2-A/W2-B 的行为。
- CI 步骤若在远端失败（例如 rust 版本缓存），可只 revert workflow 那一段，代码不受影响。
