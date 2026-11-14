# Codex / Claude GUI 与 ACP 接入调查（2026-11-14）

结论：日常新建会话各提供一个 Codex、Claude 主入口即可；没有必要要求用户在两个同名“对话”之间猜测。外部 ACP 保留为高级接入以及已有会话的兼容路径。终端视图仍有独立价值。此结论不意味着可以删除外部注册、替换旧会话的适配器或合并上下文；本轮只完成调查，未擅自变更这些入口。

## 三件不同的事

| 路径 | 真实执行链 | 运行程序的来源 |
| --- | --- | --- |
| Khor 内置 Codex 对话 | Khor GUI → 通用 ACP host → `khor _codexagent` → `codex app-server` | 目标机器用户的 Codex（支持 KHOR_CODEX 覆盖） |
| 注册的 codex-acp 对话 | 同一个 GUI/ACP host → 注册的 ACP 命令 → Codex App Server | 适配器的 Codex 依赖或 CODEX_PATH 指定的二进制 |
| Khor 内置 Claude 对话 | 同一个 GUI/ACP host → `khor _cagent` → `claude --input-format stream-json` | 目标机器用户的 Claude（支持 KHOR_CLAUDE 覆盖） |
| 注册的 claude-acp 对话 | 同一个 GUI/ACP host → 注册的 ACP 命令 → Claude Agent SDK | 该版本 SDK 使用的执行程序及配置 |
| 终端 Claude/Codex | Khor PTY → 厂商 CLI/TUI | 目标机器的 CLI；完整终端操作界面 |
| 厂商官方 GUI | 厂商客户端及其服务 | 不等于任意一个 ACP 客户端，也不因为模型相同而自动具备同样功能 |

本地依据：`crates/node/src/lib.rs` 的 Picked::launch；`cagent.rs`、`codexagent.rs` 的初始化、会话和事件处理；`acp_management.rs` 的 picked_agent；`NewSessionPage.tsx` 的 GUI/终端选项。内置“对话”本来就在使用 ACP，它不是直接把厂商 GUI 嵌进 Khor。

OpenAI 把 App Server 定位为集成认证、历史、审批和流式事件的接口；采用它只能获得底层能力，客户端仍须实现界面和事件处理。[Codex App Server](https://developers.openai.com/codex/app-server/)。Claude Agent SDK 提供 Claude Code 的工具、执行循环及上下文管理，也不是一个现成 GUI。[Claude Agent SDK](https://code.claude.com/docs/en/agent-sdk/overview)。

## 当前注册版本与差异

本机只读注册记录：codex-acp 固定适配器 2.2.2；claude-acp 固定 0.70.0。注册存在不代表所有目标机器具备同一运行时。macserver 的非交互默认 PATH 未找到对应程序，不等于已排除 GUI 启动环境或注册命令可执行性。

Codex ACP 2.2.2 本身也是 App Server 适配器；其 npm 依赖是 `@openai/codex ^0.160.1`，可用 CODEX_PATH 覆盖。因此它与 Khor 内置入口可能运行不同的 Codex 二进制，适配器版本固定也不等于所有传递依赖字节固定。[固定版本清单](https://github.com/agentclientprotocol/codex-acp/blob/v2.2.2/package.json)、[固定版本说明](https://github.com/agentclientprotocol/codex-acp/blob/v2.2.2/README.md)。

Claude ACP 0.70.0 基于 Claude Agent SDK 0.3.232、ACP SDK 1.3.0；声明图片、工具权限、编辑、命令、MCP 及嵌套子智能体记录支持。嵌套记录必须由客户端协商，不能仅因安装了适配器就宣称 Khor 已完整实现。[固定版本清单](https://github.com/agentclientprotocol/claude-agent-acp/blob/v0.70.0/package.json)、[固定版本说明](https://github.com/agentclientprotocol/claude-agent-acp/blob/v0.70.0/README.md)。没有把上游 main 的较新功能当作已安装 0.70.0 的能力。

## 不能假设等价的地方

- 认证、模型与权限：两条路径最终受实际执行机器上的登录、配置、模型权限和运行时版本控制。模型名能显示不等于该账户能调用。必须读取真实模型/配置能力，不能给厂商名写一张静态能力表。
- 会话与恢复：内置适配器保留原生会话/线程 ID 并实现恢复；外部 ACP 有自己的 session/load 映射。切换入口不能直接沿用另一条路径的 contextId。升级适配器不会自动重建已有运行进程和上下文。
- 工具与界面：内置 Codex 处理消息、思考、计划、工具、原生补丁、审批、模式和模型配置；Claude 处理消息、思考、工具、审批、命令与配置。厂商新增事件需要继续映射。外部适配器也可能使用未被 Khor 协商的扩展；均不能据此承诺官方 GUI 的完整体验。
- 子智能体：Khor harness 自己的 ACP/A2A 委派树，与某个厂商内部的子智能体记录是两类关系。后者需要适配器协商和客户端渲染；不能把普通工具调用冒充独立子会话。

## 前面两次故障说明

“内置不能读取能力、外部 ACP 可以”曾有管理入口只认注册名称的问题。目前 main 的 acp_management 已统一通过 picked_agent 解析内置/外部，且有原生名称准入与 setup/create 测试。此处证明入口准入已修；没有用假执行程序的单元测试替代真实安装、登录和初始化验证。

之前 `gpt-6.1-sol` 账户拒绝在旧上下文中重试仍失败，升级适配器后新上下文成功，用户已确认。它说明旧上下文/运行进程不能当作新版本验证；不能据此认定 ACP 天生不支持该账户，或将所有错误都归因于前端。

## 建议的产品落点与验证界限

统一厂商主入口，连接信息里显示真实目标机器、适配器来源与版本；高级选项允许明确选择外部 ACP。默认选择应建立在目标机器实际可用的可执行程序、初始化和能力检查上；保留旧会话原适配器。终端入口继续保留。实施入口收敛前应对两条路径各跑同一组真实新会话探针：初始化/模型列表、普通问答、图片、工具审批允许/拒绝、取消、恢复、子任务及错误；不迁移现有上下文、不改用户认证。

本轮结论来自已定位的代码、只读注册和固定版本上游源码；未在用户账户上创建付费模型测试，未声称两条路径的真实能力已全部对齐。
