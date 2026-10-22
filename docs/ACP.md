# ACP 会话发现与续接

2026-10-22 核验。

## 协议与实现

- 稳定协议仍为 ACP v1；SDK 版本号不等于协议版本号。
- Rust SDK：`agent-client-protocol 3.5.0`，schema `1.12.0`；显式启用 `process`、`stdio`。
- npm 注册预设：Claude Agent ACP `0.94.0`、Codex ACP `2.2.2`。
- 内置 Claude / Codex 对话由 khor 的 `_cagent` / `_codexagent` 对接本机 CLI。
  npm 适配器属于另一路注册入口。更新预设不会改写用户已保存的命令，也不会替换活动会话的进程。

协议来源：[会话列举](https://agentclientprotocol.com/protocol/v1/session-list)、
[会话建立与恢复](https://agentclientprotocol.com/protocol/v1/session-setup)。
适配器来源：[Claude](https://github.com/agentclientprotocol/claude-agent-acp)、
[Codex](https://github.com/agentclientprotocol/codex-acp)。

## 发现的边界

本机运行中的会话来自原生文件、进程和终端信息。历史 ACP 候选来自注册适配器的
`session/list`，在导入界面主动刷新后保存；列表轮询不会反复启动适配器。

列举先检查能力，再保留 cwd 筛选条件读取全部 cursor，按完整会话 ID 去重。
后续页失败或 cursor 循环会报错，不把半份列表当成完整结果保存。
没有列举能力与列举结果为空是两种答案。

这不覆盖其他系统账户、无权限读取的目录、未注册的适配器、已删除的历史，
也不能凭一条线程记录附着到任意外部原始 TTY。

## 数据目录

同一解析函数用于发现、历史、识别配置、用量与原生子进程：

1. 显式 `KHOR_VENDOR_HOME`：其下 `.claude` / `.codex`。
2. 节点根为系统账户的真实家目录：分别遵循 `CLAUDE_CONFIG_DIR` / `CODEX_HOME`。
3. 其他根：根下 `.claude` / `.codex`，不继承用户真实数据目录。

显式 `Discovery::at`、`Meters::at` 保持固定目录的 fixture 语义。
GUI 与受托管终端的原生子进程使用同一解析结果。

## 打开与转换

- GUI 恢复要求适配器支持 `loadSession`；成功必须是原 ID、回放历史、可以继续对话。
- 原生历史读取失败会报错，不呈现为空白历史。
- Claude、Codex 均有“转为对话”入口；转换前检查原生 CLI 与工作目录。
- Codex writer 锁证明线程仍被持有，不能证明某个终端 PID。无可靠 PID 时拒绝接管，
  不终止可能承载多个会话的共享 daemon；原终端退出后仍持有 writer 时也拒绝恢复。
- khor 管理的 Codex 终端恢复在 CLI 支持时使用 `--no-daemon`，使该终端独立持有
  线程，便于安全转回 GUI。通过有时限的 `--help` 探测能力；普通用户终端命令不变。
- 转为终端只支持有已知原生 CLI 和原生记录的会话。通用 ACP 代理不因此自动获得 TUI 入口。

## 验证

常规门：

```sh
cargo test -p khor-acp
cargo test -p khor-node --lib adaptor::
cargo test -p khor-node --test takeover_safety --test gui_session
LC_ALL=zh_CN.UTF-8 cargo test -p khor-cli --test cagent --test codexagent --test agents --test open
cd apps/gui && npm test
```

真实适配器可只列举、不创建会话或模型轮次：

```sh
ACP_AGENT_CMD='node /path/to/adapter/dist/index.js' ACP_EXPECTED_MIN=26 \
  cargo test -p khor-acp a_real_agents_full_listing -- --ignored --nocapture
```

`native_handoff` 为显式启用的原生 GUI → TUI → GUI 验收，要求隔离探针目录和
该目录内的测试 UUID；不会在普通测试中运行。真实模型续接的核验在隔离数据目录内
创建会话，记住一次性随机文本，退出进程，再以同一 UUID 恢复并询问该文本。
不以 initialize 成功或能力声明替代这一验证。

### 本轮实测记录

- 新版 Claude 适配器列举 49 个根会话；新版 Codex 适配器列举 169 个、7 页。
  khor 客户端实际读取的数量与逐页 JSON-RPC 核验一致。
- Mac 本机 Claude 2.1.307、Codex 0.163.0：同 ID 恢复、历史回放、跨进程记忆续接、
  GUI → 原生 TUI → GUI 均通过。未接管用户正在使用的会话。
- ACP 客户端、两种原生 shim 的权限／取消／历史测试、终端宿主、前端门通过。
- 全量 node 库测试为 839 通过、8 忽略、5 失败。5 个失败均已在修改前的
  `35ee9c02` 构建复现：两条 Claude 旧标题断言、默认 daemon 客户端类别断言、
  设置分组旧字段断言、外置卷临时目录的 macOS 回收站路径断言。
  本轮没有把这些失败作为新回归，也没有宣称全量通过。
