# Khor 的 ACP 会话基座

已将验收后的布局与配置接入正式组件。`npm run session-navigation-ui` 使用内存桥接示例验收正式 App，真实适配由下面的协议、原生 CLI 和节点测试验证；预览不会调用真实模型。原 Lab 作为布局讨论记录保留。

## 统一边界

GUI 读取 ACP v1 的 `configOptions`、`usage_update`、能力和交互。选项 ID、类型、分类和可选值由 agent 提供；模型变化返回完整选项列表，GUI 替换列表。未知类型不冒充已知控件。旧版 agent 没有提供的功能不显示为可用。

Khor 的 Pi 进程继续使用 SDK 和 Rust 私有凭据／MCP 管道。`session-state.ts` 将其配置和事件映射到同一组 ACP 数据形状；这不等于把 Pi 私有管道变成第三方 ACP 服务，也不引入另一套前端模型表。

| 能力 | Khor / Pi | 内置 Claude | 内置 Codex | 第三方 ACP |
| --- | --- | --- | --- | --- |
| 模型 | Pi ModelRuntime 目录和手工配置的 ID | 原生 control/initialize models | 原生 model/list，含分页 | configOptions |
| 思考 | SDK 当前模型的档位 | 原生 supportedEffortLevels + 会话 flag settings | 原生 supportedReasoningEfforts + turn/start effort | thought_level 分类；也兼容其他分类和 ID |
| 自动压缩 | 可配置，并报告压缩状态／摘要 | 原生代理决定 | 原生代理决定 | 布尔选项及 compaction 更新，取决于代理 |
| Context | SDK 实际读数；未知窗口不显示百分比 | 保留原有原生 usage 适配 | last.totalTokens / modelContextWindow | usage_update 快照 |
| 登录 | Pi 原有 API key / OAuth 工作流，凭据留在账户 home | CLI 自己的登录 | CLI 自己的登录 | 新建会话前的 agent-defined authenticate，支持结构化补问 |
| 会话 MCP | Rust-owned Khor 工具 | ACP descriptor → 原生 MCP 配置 | ACP descriptor → 原生 MCP 配置 | session/new、load、resume 标准注入 |
| 执行权限 | Rust MCP 固定逐项审批，读取放行 | 原生模式和审批 | 原生模式和审批 | agent 提供的模式、配置与 request_permission |

Context 是占用量快照，不能用累计账单 tokens 代替。模型切换和压缩时清除旧读数，收到新测量后再显示。Pi 对未知网关模型的默认窗口不作为实测窗口。配置记录新增的 options map 带 serde default，兼容旧数据；切换模型会重新验证思考档位。

## 认证与结构化交互

认证使用独立 ACP 连接，不需要先创建 session。智能体启动命令由执行节点的注册表解析；用户显式配置的 MCP server 则通过标准 descriptor 交给 agent。任务绑定节点 root 和随机 job ID，支持轮询、重试填写、取消和超时。填写结果不进入会话历史或浏览器存储。

表单支持字符串、枚举、数值、布尔值与多选；执行节点验证必填、类型、范围和选项后才消费问题。错误答案保留原问题。URL 补问显示目标主机，用户点击后才打开 HTTP(S)；完成通知可结束对应 URL 问题。过期、取消后的答复不能再次执行。

会话 host 在回放时继续处理补问和权限。历史只返回请求它的窗口；回放失败不展示半段历史，并给出实际错误。晚加入的窗口收到待答问题，同一问题的答案同步到所有窗口。历史缓冲有大小上限。

## MCP、执行目录与生命周期

新建第三方会话页可配置这一轮的 MCP servers 和附加目录。目录属于执行机器，必须是绝对目录；HTTP/SSE 和 additionalDirectories 受握手能力约束。MCP 描述不写入同步的 agent 注册表，执行上下文保存在执行节点的私有文件中，供恢复同一会话使用。

调度员模式向第三方 ACP 注入标准 Khor MCP server；内置 Claude/Codex 继续保留原生调度说明和权限行为。Pi 的 Rust 审批边界保留。

Khor 的审批入口在 `khor_agent::McpSession`，不依赖 Pi 是否自带权限弹窗。修改操作先保存 `AgentApproval` 并阻塞执行，GUI 的 `KhorAgentResult` 展示实际工具与参数；答复绑定 run 和 ask。取消／停止会撤销待答请求，随后到达的旧批准不能执行工具。当前仅有固定的「逐项确认」策略，没有实现 Khor 自身的自动授权档位；顶部菜单明确展示这个边界。

恢复优先使用代理声明的 session/resume，否则使用 load。close、delete、logout 按能力开放；删除记录有单独确认。client filesystem 和 terminal 继续关闭：连接 GUI 的手机不承担执行机器的文件访问和进程运行。

压缩事件按 compactionId 更新：省略字段保留原值，null 清空，summary chunk 按顺序追加。notice 只在当前连接展示，不写入 Pi 的持久 receipt，也不从历史重播。图片、音频和资源引用不再被文本投影静默丢弃。

## 接口与实现位置

- `crates/acp`：握手、会话上下文、认证连接、补问校验、生命周期。
- `crates/node/src/gui_host.rs`：多窗口、回放并发、待答交互和配置广播。
- `crates/node/src/acp_management.rs`：执行节点的 ACP 设置任务，以及带请求 token 的会话创建。
- `apps/agent/src/session-state.ts`、`management.ts`：Pi 配置、实际用量与 SDK 档位。
- `cagent_config.rs`、`codexagent_config.rs`：两个原生协议边界的动态配置。
- GUI core / HTTP / Tauri 共用 `chat_respond`、`chat_control`；wire 枚举只在尾部增加变体。
- `SessionPreferences`：三类 GUI 会话共用的模型、权限与上下文配置控件；`AgentInteraction` 与 `AcpAgentSetup`：会话补问、登录与 MCP 上下文。

Pi 通过 `khor_agent/manage` 的 session_settings / session_set_option 工作流校验和持久化；并发配置发生变化时拒绝覆盖旧快照。完成的设置结果会回传调用机器。第三方 ACP 使用 acp_initialize / acp_authenticate / acp_poll / acp_reply / acp_cancel / acp_open，同样经现有认证 mesh/IPC 路由到所选节点。

## 明确边界

- ACP 是客户端与 agent 的协议；Memory、ToolSearch 和工具发现策略仍由具体 agent 实现。本轮没有给所有 agent 虚构这些组件。
- env-var 与 terminal 类型的第三方认证目前展示其原生要求，不冒充 agent-defined 登录流程。Pi 的账户登录使用已有独立账户入口。
- 在 session/new 返回前要求额外交互的 agent 目前会明确拒绝创建，并回收进程；独立的认证连接支持创建会话前补问。
- 实验性的 fork 未开启。附加目录仅对明确支持它的代理开放；不猜测原生 CLI 未声明的目录能力。
- 结构化补问在目标 ACP 会话内可用；本轮没有新增经 A2A 转发完整表单的协议扩展。
- 未使用真实付费模型请求或真实 OAuth token 交换做验收；未完成原生 iOS 键盘、VoiceOver 与系统浏览器返回链路实测。

## 验证

前端默认检查、生产 build、Pi 源码与独立可执行文件测试、ACP stdio 测试、两个原生 CLI 的脚本替身和停止回归，以及 Lab 布局和交互检查均有独立入口：

```sh
# 使用隔离的测试节点和本机协议替身，不需要真实 key
cd apps/agent
npm test
node build.mjs
KHOR_TEST_AGENT_BIN=dist/khor-agent npm test

cd ../gui
npm test --ignore-scripts
npm run build --ignore-scripts
node lab/check-gui-session-layout.mjs
node lab/check-gui-session-composer.mjs
node lab/check-acp-session.mjs

# 仓库根目录；target 路径按本机配置，避免不同 worktree 同时覆盖缓存
cd ../..
cargo test -p khor-acp
cargo test -p khor-cli --test cagent --test codexagent --test codexagent_sched --test gui_refused --test gui_stop
cargo build -p khor-acp --bin acp-stub
KHOR_AGENT_SIDECAR="$PWD/apps/agent/dist/khor-agent" cargo test -p khor-node --test acp_setup --test gui_session --test khor_agent_wire
```

2026-10-23 验收结果：Pi 源码与独立二进制各 42 项通过（1 项依赖 release binary 的检查跳过）；ACP stdio 19 项及 lib 2 项通过；Claude/Codex 原生替身、调度注入、停止回归通过；ACP 设置任务、跨机器配置回传、9 项 GUI host 回归通过。节点 lib 在单独排除下述 4 项基线失败后，858 项通过、8 项忽略。Lab 的 23 项布局、20 项输入区检查，以及结构化补问、双窗口、压缩 patch、通知和 URL 检查通过。前端默认检查与生产构建、原生 GUI 编译通过。

节点全量 lib 单测有 4 项既有失败，均已在 `f6b42862` 基线复现：三个 live 会话命名／类别断言，以及 settings_group_into_the_seven_pages_and_keep_their_values。本轮不修改这些不相关裁定。

协议依据：[配置](https://agentclientprotocol.com/protocol/v1/session-config-options)、[初始化](https://agentclientprotocol.com/protocol/v1/initialization)、[会话设置](https://agentclientprotocol.com/protocol/v1/session-setup)、[认证](https://agentclientprotocol.com/protocol/v1/authentication)、[补问](https://agentclientprotocol.com/protocol/v1/elicitation)。实现固定 Rust ACP 3.5 / schema 1.12 与 Pi 1.4.0，不依赖 CLI 命令菜单保持不变。
