# pi 工具适配约束

2026-10-22：Pi 1.4.0 已接入，当前实现、组件范围和构建方式见 [AGENT.md](AGENT.md)。下面保留 2026-10-13 的接口调研快照；其中版本与「后续适配」描述不代表当前依赖或完成状态。

## 版本与事实

pi 官方仓库现由 `badlogic/pi-mono` 重定向到 `earendil-works/pi`。本次核对固定在 commit `a4d040d30f2cb30fffbd841b3f0a7052158e2e21`，其中 `@earendil-works/pi-agent-core` 为 `1.2.0`。后续接入应固定实际依赖版本，再重新核对类型；旧 `@mariozechner/*` 示例不能直接当作当前接口。

- [AgentTool 与结果/事件类型](https://github.com/earendil-works/pi/blob/a4d040d30f2cb30fffbd841b3f0a7052158e2e21/packages/agent/src/types.ts)：工具声明包含名称、描述、TypeBox 参数 schema 和执行函数。核心执行顺序为 `execute(toolCallId, params, signal?, onUpdate?)`。
- [ToolDefinition 与 slash command 类型](https://github.com/earendil-works/pi/blob/a4d040d30f2cb30fffbd841b3f0a7052158e2e21/packages/coding-agent/src/core/extensions/types.ts)：coding-agent 工具在上述四个参数后另接 `ctx: ExtensionToolContext`；slash handler 则接收 `(args: string, ctx: ExtensionCommandContext)`。工具参数是结构化对象，slash 输入是待解析字符串，两者并非同一种协议。

当前 pi 结果有三条不同用途的通道：`content` 是模型可见的文本/图像；`details` 用于日志和 UI；`structuredContent` 是匹配可选 `outputSchema` 的机器结果，源码明确说明它不直接发送给模型。当前版本也支持 `isError`，不能沿用旧文档“只有 throw 才能标记失败”的假设。[结果类型源码](https://github.com/earendil-works/pi/blob/a4d040d30f2cb30fffbd841b3f0a7052158e2e21/packages/agent/src/types.ts#L424)

OpenClaw 当前已拥有自己的内置 runtime，官方文档说明不再依赖外部 agent framework，旧 runtime 名 `pi` 为兼容别名。因此参考其边界划分，而不是复制历史 pi adapter。[官方 runtime 文档](https://docs.openclaw.ai/pi)；核对源码快照：`67bd58e2f1cbbcc6b4133c441b01c02512266c33`（package version `2026.10.2`）。[当前工具适配器](https://github.com/openclaw/openclaw/blob/67bd58e2f1cbbcc6b4133c441b01c02512266c33/src/agents/agent-tool-definition-adapter.ts)将 hook、结果转换和失败日志放在适配边界；[工具装配层](https://github.com/openclaw/openclaw/blob/67bd58e2f1cbbcc6b4133c441b01c02512266c33/src/agents/agent-tools.ts)负责上下文、工具策略与执行目标。

## 对共享 Rust 能力层的建议

以下是 khor 的建议契约，不是 pi 自带的 Rust API。

1. 每个能力有稳定 ID、输入 JSON Schema、输出 JSON Schema 和一个 Rust 执行入口。slash 解析器产出 typed input；未来 pi adapter 根据同一声明生成工具。不要让工具通过拼接 slash 字符串调用业务逻辑。
2. 能力返回可序列化的状态、结构化数据和可读摘要；失败携带稳定错误码。CLI/UI 渲染该结果；未来 pi adapter 将摘要放入 `content`，数据放入 `structuredContent`，呈现信息放入 `details`。模型需要的数据也须明确进入摘要，不能假设模型能看到 `details` 或 `structuredContent`。
3. 执行上下文由宿主捕获：工作区、pane/session ID、cwd、执行目标以及请求 ID。模型参数只表达业务输入；Rust 在执行前解析和校验目标，避免焦点切换把进行中的操作路由到另一个 pane。
4. 每次调用有独立 operation ID。进度事件带该 ID、递增序号、阶段和可选数据；最终状态只落一次。pi 的 `onUpdate` 是部分结果回调，适配器可由事件生成部分结果快照；结束后不再发布进度。
5. 取消必须由 Rust 执行器实际停止或回收子进程，再报告 cancelled。未来将 pi `AbortSignal` 转发为同一 operation 的取消请求；UI Escape 与 agent abort 共享取消通道。超时、失败、取消保持可区分。
6. 保留并发边界：pi 当前核心允许 parallel 执行。会修改同一 pane、session 或工作区状态的操作在 Rust 按目标串行化；不能依赖 agent 恰好一次只调用一个工具。对非幂等写入默认不自动重放。

## 跨层验收清单

- slash 与直接结构化调用对同一输入得到相同能力 ID、数据与状态；路径空格、引号、Unicode 不改变参数。
- 缺失字段、未知能力、无效 ID 与不支持操作在副作用发生前返回稳定错误。
- 提交后切换焦点，原操作仍只影响捕获的工作区/pane；已删除目标产生明确失败。
- 两个并行调用的进度和取消互不串线；同目标修改按约定串行执行。
- 长运行 CLI 可见进度；取消后子进程被回收，只有一个最终事件，后到进度被忽略。
- 非零退出、超时、取消分别呈现；有结构化错误数据时保留它。
- 后续 pi 接入时单独验证 TypeBox schema 投影、`outputSchema`、`isError`、`AbortSignal`、`onUpdate` 与工具上下文，使用实际锁定的 SDK 版本。
