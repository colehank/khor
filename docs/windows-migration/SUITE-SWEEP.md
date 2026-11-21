# Windows 集成测试普查（2026-11-15 → 16）

`full` 权限、工作区外构建（`CARGO_TARGET_DIR=F:\Temp\khor-target`）、`--test-threads=1`。
每条都是真实进程/真实 socket 的集成目标，不是 mock。

## 1. 通过

| 目标 | 结果 |
| --- | --- |
| `pairing` | 6 passed |
| `open_from_phone` | 1 passed（60.7s） |
| `resume` | 7 passed（含远端终端重连 replay） |
| `machine_pause` / `peer_pause` / `takeover_safety` | 各 1 passed |
| `sync_doc_isolation` | 5 passed / 1 ignored |
| `transfer` | 4 passed |
| `a2a_http` | 4 passed / 1 ignored |
| `files_wire` | 1 passed |
| `history_wire` | 1 passed |
| `usage` | 4 passed |
| `vitals` | 1 passed |
| `broadcast` | 2 passed（68.7s） |
| `command_ledger` | 4 passed |
| `departed` | 1 passed |
| workspace_facts | 3 passed |
| `host_output_smoke` | 1 passed（宿主读到 PTY 输出；ConPTY 修复的回归） |
| `recognition_wire` | 1 passed（先把断言对齐 9e3f33d1 的“Claude 状态钩子退役”） |

合计 **47 项通过**（另有 2 ignored）。

## 2. 本轮改掉/门掉的

| 目标 | 处置 | 原因 |
| --- | --- | --- |
| `pairing` | 修夹具（`5dc3be49`） | 端点文件用 `pid: 1`，Windows 没有 pid 1 |

| `host_replay` | `#[cfg(unix)]` | ConPTY 不转发 `ESC [ ? 1 h`（DECCKM），replay 拿不到该模式；其余正确 |
| `cwd_pin` | `#[cfg(unix)]` | Windows 不能删除/重命名进程的当前目录，用例前提不存在 |
| `day_record` | 整文件 `#[cfg(unix)]` | 三个用例都要 `Node::import` 的“已验证原生身份”，本夹具没有 agent home；strip 逻辑由 `khor_node::day` 单测覆盖 |
| `recognition_wire` | 更新断言后通过 | 用例还在期望 `Enable(Claude)` 变成 `configured`；产品在上游 9e3f33d1 已退役 Claude 状态钩子，`entry()` 恒为 `automatic`、enable/disable 是 no-op。属测试滞后，不是产品缺陷 |
| `no_host` | 整文件 `#[cfg(unix)]` | 夹具是一个 `#!/bin/sh` 假 tmux，并用 `std::os::unix`；Windows 上**编译不过**（E0433/E0599），tmux 也不存在 |

## 3. 编译门

```text
cargo test --locked -p khor-node --no-run     exit 0   # 全部 node 测试目标可编译
cargo test --locked -p khor-cli  --no-run     exit 0   # 全部 CLI 测试目标可编译
cargo fmt --check -p khor-node / -p khor-cli  exit 0
```

（`no_host` 是这轮补掉的编译断点；`relays` 中途出现过一次 E0786 无效元数据，是之前被 kill 的 cargo 留下的陈旧产物，重试即通过。）

被 gate 的条目都在文件里写了原因，不当作通过。

## 3. 发现（按影响排序）

1. **ConPTY 输出（已修）**：`portable-pty` 0.9.0 把 `PSUEDOCONSOLE_INHERIT_CURSOR` 传给
   `CreatePseudoConsole`，在没有控制台的父进程下让 conhost 完全不出输出。已按仓库惯例 vendor
   （`vendor/portable-pty`）去掉该 flag；`host_output_smoke`、`resume`（含远端重连 replay）、
   node lib 全量、CLI console/quit/ws 全部转绿。详见 [W5-GUI](W5-GUI.md) §4.1。
2. **GUI 会话列表不渲染**：节点与 web face 正常（日志 `khor web face: 0.0.0.0:5467`），列表停在骨架。
3. **ConPTY 不转发 DECCKM**：`ESC [ ? 1 h` 被 conhost 消费，replay 头部拿不到该模式（`host_replay` 因此仍 `#[cfg(unix)]`）。
4. GUI 的 npm 幻影依赖（`@shikijs/langs`、`@shikijs/themes`、`hast`）——npm 提升才成立，需在 `package.json` 声明并重生成 lock。
