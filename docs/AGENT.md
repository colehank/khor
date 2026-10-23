# Khor agent

khor 使用用户登记的 API 账户，通过 Pi 处理设备网络里的任务。当前运行组件固定为 Pi 1.4.0；实现位于 `apps/agent` 和 `crates/node/src/khor_agent.rs`。

Khor session 定位为用户与 Khor agent 的常驻主对话，普通文字直接交给 agent；后续 `@` 用来引用机器、agent 或工作 session。智能体间委派采用 A2A，任务与 session 的边界、已实现的原生 A2A HTTP 和本机／跨设备 ACP 执行及协议边界见 [A2A.md](A2A.md)。

## 使用

1. 从额度页 ＋ 选择「模型账户」，搜索 Pi 提供商、选择保存凭据的设备，按提供商提示配置 API Key、OAuth 或云服务身份，再选择默认模型。原有额度查询与订阅导入入口保留。
2. 打开 khor 会话，在输入框下方选择账户及当前模型。可以搜索模型目录，也可直接输入模型 ID。已有额度账户从行菜单「模型调用」补充推理配置。
3. 普通文字提交给 khor agent；`/` 命令继续直接执行。文件邀约使用现有文件通道。

执行节点是所选账户的 home。手机和浏览器通过认证后的 khor 网络向该节点发任务、批准或停止操作。节点不通时明确拒绝，不悄悄换设备或改用另一份凭据。当前一台执行节点同时处理一个 khor 任务；忙碌期间拒绝新的任务，但允许选择下一轮使用的账户和模型。当前请求及其审批仍绑定开始时的配置。

## 模型账户与额度查询

提供商和认证方式从固定的 Pi 1.4.0 公共 API 生成 `apps/agent/providers.json`；只向用户提供支持聊天且有配置流程的提供商，另有 OpenAI 兼容服务入口。模型目录由对应提供商读取，读取失败时保留已缓存的选项，并允许手输 ID；列出模型不表示已经验证付费推理权限。

账户的默认模型存于同步账户元数据 `inference.default_model`；会话当前选择继续使用 `AgentConfig`，修改其中一项不会改写另一项。已经开始的回复保持其模型与审批，后续请求捕获新的配置。khor 的广播主对话在同一执行设备上延续上下文；Claude/Codex 托管 GUI 会话继续使用各自 ACP 宿主及其模型选择。

账户名称、归属设备、提供商、服务地址和默认模型在设备网内同步；私密凭据留在归属设备。手机或其它设备可通过已有认证路由发起登录与配置，凭据输入只传给选定设备。登录支持提供商的分步输入、授权链接、设备码和手工回填；关闭会取消后台操作，闲置操作会过期回收。Pi 的 Bun OAuth 注册入口显式嵌入所有登录实现，独立发布包不依赖用户安装 Node 或 node_modules。

额度查询密钥与推理凭据用途分开。OpenAI/Anthropic 的额度 Admin Key 不自动当推理 Key；旧有兼容网关、DeepSeek、Moonshot、OpenRouter 的可推理 Key 可以直接复用。为已登记的订阅账户配置 Pi 不会覆盖 CLI 的额度登录。新模型账户未配置额度查询时明确显示「未配置额度查询」，不向用量接口提交推理密钥。OpenAI 兼容服务可以显式选择本地服务免认证。

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
- 推理凭据由 Pi ModelRuntime 保存在执行设备的 `.khor/agent/credentials/<account-id>/auth.json`，同提供商的多个账户彼此独立；刷新令牌使用 SDK 的跨进程存储锁。旧版可推理 API Key 仍经私有 stdin 交给 Pi，设置账户默认模型时迁入独立存储；凭据不写入同步记录或模型提示。子进程环境不继承模型提供商的 API Key。

## GUI 与 ACP 的分工

Claude、Codex 的 GUI 会话使用既有 ACP 宿主。khor 的 Pi 任务继续走原有私有 JSONL / MCP 通道，把 SDK 的文字、思考、工具开始／进度／结果转换为 ACP v1 `SessionUpdate`，由 Rust SDK 校验后写入原任务记录。这里统一的是事件与界面；khor 尚未因此成为一个可被外部编辑器连接的 stdio ACP 服务。

三类会话共用 `lib/acp-view.ts` 和 AI Elements 消息、思考、工具、计划与审批组件。只有来源实际报告的内容才显示；审批仍由各自宿主授权。Codex 原生 unified patch 保留 hunk 范围与原始行号，不能当作完整文件。Claude 已报告的编辑作为对话产物保留，完成时另显示工具文字回执；显式清空内容仍移除它。

khor 记录的显示快照最多 256 KiB / 512 项，合并连续文本和同一次工具的进度；过大的工具显示块不保留，界面提示详情截短。模型上下文仍由 Pi SessionManager 保存，显示快照不承担恢复执行。旧记录缺少这些字段时继续显示原来的回答；不会伪造旧的思考、工具或计划。接收同步快照不执行内容。

版本核对（2026-10-23）：Pi 固定 1.4.0，ACP Rust SDK 固定 3.5.0；按官方当前稳定的 ACP v1 接入。Codex 映射对照本机 0.163.0 的 `app-server generate-json-schema`，包括命令输出与结束状态、MCP 结果、计划和文件补丁。Claude 沿用现有适配器与权限通道。

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
- `KHOR_TEST_AGENT_BIN=./dist/khor-agent node --test test/host.test.ts`（在 apps/agent）：同一测试验证独立运行组件。另运行 `test/management.test.ts` 检查包外 OAuth 唤起／取消及账户隔离。
- `cargo test -p khor-gui-core --test khor_agent --test seen_snapshot --test console_receipts --test command_tables`：Rust→Pi→MCP、审批、停止、幂等、上下文与已读边界。
- `cargo test -p khor-node --test khor_agent_wire`：真实 IPC/设备网络、账户归属、密钥留在 home、未配对设备拒绝。
- `npm run console`（在 apps/gui）：真实输入框、配置、流式 Markdown、批准/拒绝、命令回归。

测试模型只监听本机回环，使用临时账户与目录，不调用用户的付费模型。

- `npm run pi-accounts`（apps/gui）：正式模型账户控件，宽窄屏 × 浅深主题，默认／会话选择隔离、运行时切换、凭据输入、OAuth／设备码与取消。`subscription-import` 保留额度查询和订阅导入回归。
- 本地 `http://localhost:23782/?bridge=23782` 是正式 App 的隔离验收入口，模型账户数据为示例，不输入真实 Key。真实 SDK、私有存储和跨设备协议另由上述原生回归验证。
