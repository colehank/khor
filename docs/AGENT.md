# Khor agent

khor 使用用户登记的 API 账户，通过 Pi 处理设备网络里的任务。当前运行组件固定为 Pi 1.4.0；实现位于 `apps/agent` 和 `crates/node/src/khor_agent.rs`。

Khor session 定位为用户与 Khor agent 的常驻主对话，普通文字直接交给 agent；后续 `@` 用来引用机器、agent 或工作 session。智能体间委派采用 A2A，任务与 session 的边界、已实现的原生 A2A HTTP 和本机／跨设备 ACP 执行及协议边界见 [A2A.md](A2A.md)。

## 使用

1. 从 ＋ 登记 API 账户。账户描述在网络内同步，密钥只保存在账户的 home。
2. 打开 khor 会话，在输入框下方选择账户、填写模型名称。
3. 普通文字提交给 khor agent；`/` 命令继续直接执行。文件邀约使用现有文件通道。

执行节点是所选账户的 home。手机和浏览器通过认证后的 khor 网络向该节点发任务、批准或停止操作。节点不通时明确拒绝，不悄悄换设备或改用另一份凭据。当前一台执行节点同时处理一个 khor 任务；忙碌期间拒绝新的任务和配置修改。

## 运行与工具

Rust 宿主管理账户、任务标识、审批、记录和网络路由。独立 Pi 进程负责模型请求与工具循环，使用 coding-agent SDK 的会话存储和自动上下文压缩。SDK 的默认文件、shell、宿主目录的 Skills 和扩展发现全部关闭。

Pi 官方 MCP client 通过私有 JSONL 管道与 Rust MCP 分发器通信。当前开放设备和 session 列表、目录、文本读取、搜索、创建/关闭 session、发送留言；这些是 khor 网络工具，不从全部公开 CLI 自动生成。另有 agents、agent_connect、agent_disconnect、delegate、task_status、task_wait、task_answer、task_cancel、task_list 九个协作工具，负责连接、召唤、跟踪和停止本机或远程智能体。原生 A2A 服务通过 HTTP 直接连接，对方无需 Khor；本地程序和配对设备上的智能体沿用 ACP。外部 `khor mcp` 与内部 agent 使用同一份工具目录、参数校验和 Node 业务实现。

只读工具直接执行。写操作先在 Rust 落盘待批记录，再等用户批准；批准绑定本次任务、工具与实际参数，只生效一次。模型输出、工具描述和 MCP 的 readOnlyHint 都不能自行授权。等待期间用 MCP 进度通知维持请求；客户端取消、断开或停止会撤销待批，失效批准不能再触发写操作。

## 状态与恢复

- 每次提交有稳定 token；同一 token、同一请求返回原记录，改变输入则拒绝。
- 执行前认领并落盘。接收同步记录不会开始模型请求或执行工具。
- 回答、待批和最终状态更新同一条会话记录；进度版本拥有独立的已读快照。
- Pi 的完整模型上下文保存在执行节点 `.khor/agent/sessions/`，包含工具调用及结果。进程重启后继续读取该上下文，不靠显示文字重建。
- 任务记录保存在 `.khor/agent/runs/`。重启发现执行锁已释放而记录仍在运行时，标记中断；不会自动重放网络写操作。
- 停止中止 Pi 请求和等待中的审批，并按持久子任务关联请求停止委派。已经提交给远端的操作可能已经生效，停止不表示回滚。
- 密钥只经私有 stdin 交给 Pi，不写入任务记录或模型提示；子进程环境不继承模型提供商凭据。

## Pi 组件的边界

Pi 官方 [SDK 文档](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md) 区分会话核心、资源加载和可选的 MCP / ToolSearch / Codemode 扩展；接入 SDK 不会自动启用全部 CLI 组件。

| 组件 | 当前接入 |
| --- | --- |
| MCP | 已接入 khor 自身的网络工具；第三方服务器的配置/OAuth UI 尚未开放 |
| 会话存储、上下文压缩 | 复用 Pi SDK 的 SessionManager 与 compaction |
| ToolSearch | Pi 官方已有；当前工具集较小，直接声明，尚未启用延迟加载 |
| Codemode、Skills、Extensions | 官方可用，当前没有自动启用 |
| 长期 Memory | 未实现跨会话事实提取、更新、检索和遗忘；完整会话上下文不是长期记忆 |
| Durable | Pi 官方实验组件，当前不引入第二套持久任务系统 |

## 构建与发布

`npm ci --prefix apps/agent` 安装固定依赖；Bun 1.3.14 执行 `node apps/agent/build.mjs [Rust target]`，生成不依赖用户 Node 安装的 `apps/agent/dist/khor-agent`。

编译原生宿主时设置 `KHOR_AGENT_BUNDLE` 为上述产物的绝对路径。node crate 会将运行组件嵌入 khor 可执行文件；首次运行按内容哈希提取到私有目录，校验内容后启动。编译组件时关闭 Bun 的项目配置与 dotenv 自动加载。手机构建不嵌入运行组件。发布 workflow 已包含交叉构建和嵌入步骤，安装器仍可分发单个 khor 文件。

开发构建也可以使用可执行文件旁的 `khor-agent`，或 `KHOR_AGENT_SIDECAR` 指定的开发组件；未打包的 debug 构建可用 Node 启动仓库里的 `apps/agent/src/host.ts`。

## 验证

- `npm test --prefix apps/agent`：类型、账户/provider、MCP adapter 和真实 SDK 的本地模型恢复测试。
- `KHOR_TEST_AGENT_BIN=./dist/khor-agent node --test test/host.test.ts`（在 apps/agent）：同一测试验证独立运行组件。
- `cargo test -p khor-gui-core --test khor_agent --test seen_snapshot --test console_receipts --test command_tables`：Rust→Pi→MCP、审批、停止、幂等、上下文与已读边界。
- `cargo test -p khor-node --test khor_agent_wire`：真实 IPC/设备网络、账户归属、密钥留在 home、未配对设备拒绝。
- `npm run console`（在 apps/gui）：真实输入框、配置、流式 Markdown、批准/拒绝、命令回归。

测试模型只监听本机回环，使用临时账户与目录，不调用用户的付费模型。
