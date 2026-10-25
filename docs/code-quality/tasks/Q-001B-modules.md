# Q-001B：逐模块覆盖清单

固定基线：main 8704a503a27b36ad58f922690de50e766f8bd485；日期：2026-10-25。职责摘要取自源码模块文档及入口巡检，不能理解为功能正确性证明。正式发现、触发条件与验收见 [Q-001B-report.md](Q-001B-report.md)。

本清单按文件模块列全 154 个 Rust src 路径和 11 个 TS src 路径；内联生产模块另列。每文件行数包括空行、注释、内联测试，仅用于核对基线。tests 路径数包含 helper，不等于 test case 数。此首轮为全范围结构清点加关键边界取证，不是全实现逐行审查。

状态：K = 本行列出的关键实现路径已追踪（文件其余部分仍可能只有结构阅读）；S = 职责/API/入口结构巡检，行为深审待后续；T = 测试或 fixture 登记/部分抽读，未运行；G = 构建/生成入口登记，完整构建链不在本代理深审范围。没有把 S/T/G 升格为“已安全”。

## 范围总表

| 模块 | src 文件数 | src 行数 | 外置测试路径数 | K / S / T / G |
| --- | ---: | ---: | ---: | --- |
| crates/a2a | 3 | 880 | 1 | 2 / 1 / 0 / 0 |
| crates/acp | 4 | 1,711 | 1 | 1 / 2 / 1 / 0 |
| crates/catalog | 4 | 553 | 1 | 2 / 1 / 0 / 1 |
| crates/cli | 1 | 35 | 62 | 0 / 1 / 0 / 0 |
| crates/commands | 3 | 8,031 | 0 | 1 / 2 / 0 / 0 |
| crates/core | 4 | 8,022 | 0 | 1 / 3 / 0 / 0 |
| crates/detect | 2 | 1,340 | 0 | 0 / 1 / 1 / 0 |
| crates/net | 7 | 3,727 | 12 | 3 / 4 / 0 / 0 |
| crates/node | 101 | 103,169 | 61 | 15 / 83 / 3 / 0 |
| crates/sync | 23 | 10,857 | 3 | 2 / 17 / 4 / 0 |
| crates/web | 2 | 621 | 1 | 2 / 0 / 0 / 0 |
| apps/agent | 11 | 1,090 | 8 | 7 / 4 / 0 / 0 |

## crates/a2a

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/executor.rs](/Users/zgh/Desktop/working/khor/crates/a2a/src/executor.rs) | 112 | 执行侧 task/turn lease 与取消、进度、结果记录 | K — task/turn/cancel 状态推进；真实代理取消待验证 B-05 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/a2a/src/lib.rs) | 112 | Execution-home-owned A2A task foundations. See docs/A2A.md. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/store.rs](/Users/zgh/Desktop/working/khor/crates/a2a/src/store.rs) | 656 | 执行 home 的任务账本、请求幂等、context/caller 校验与恢复 | K — 任务持久化、caller/context、终态/恢复；容量与跨版本待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/delegation.rs](/Users/zgh/Desktop/working/khor/crates/a2a/tests/delegation.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/a2a/Cargo.toml).

## crates/acp

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/bin/acp-stub.rs](/Users/zgh/Desktop/working/khor/crates/acp/src/bin/acp-stub.rs) | 399 | A scripted ACP agent for the tests — the fake_sshd pattern, ACP shaped: real transport (child process, stdio JSON-RPC), scripted behaviour, no model a… | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/interaction.rs](/Users/zgh/Desktop/working/khor/crates/acp/src/interaction.rs) | 167 | Validate before consuming an interaction: invalid input must leave it answerable. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/acp/src/lib.rs) | 1102 | The ACP live channel: khor as an Agent Client Protocol client. | K — 初始化/请求/事件/permission/drop；故障矩阵待补 R-B9 |
| [src/management.rs](/Users/zgh/Desktop/working/khor/crates/acp/src/management.rs) | 43 | A login connection has no session: authentication must work before session/new. | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/conversation.rs](/Users/zgh/Desktop/working/khor/crates/acp/tests/conversation.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/acp/Cargo.toml).

## crates/catalog

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/bin/gen_ts.rs](/Users/zgh/Desktop/working/khor/crates/catalog/src/bin/gen_ts.rs) | 6 | Prints the TS face of zh.toml. The GUI build pipes this into its source tree so both ends read one wording file and never drift. | G — 生成入口已登记；未作完整构建链审查 |
| [src/lang.rs](/Users/zgh/Desktop/working/khor/crates/catalog/src/lang.rs) | 139 | Which language this process is speaking (docs/MORE.md 语言). | K — 语言状态读取/作用域；多前端语言契约未验 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/catalog/src/lib.rs) | 64 | The wording catalog (docs/UX.md 文案): **zh.toml and en.toml** in this crate are the one place user-facing words live. This is their build-time Rust face;… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/reword.rs](/Users/zgh/Desktop/working/khor/crates/catalog/src/reword.rs) | 344 | A sentence another khor said, read back into its key and arguments and said again in this process's language (round NET-9). | K — 错误文本重述路径；结构化 error 域后续评估 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/gate.rs](/Users/zgh/Desktop/working/khor/crates/catalog/tests/gate.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/catalog/Cargo.toml), [build.rs](/Users/zgh/Desktop/working/khor/crates/catalog/build.rs).

## crates/cli

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/main.rs](/Users/zgh/Desktop/working/khor/crates/cli/src/main.rs) | 35 | CLI 进程入口与 commands 转发 | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/add_ssh.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/add_ssh.rs), [tests/agents.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/agents.rs), [tests/attach_version.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/attach_version.rs), [tests/cagent.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/cagent.rs), [tests/carry.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/carry.rs), [tests/chat_upload.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/chat_upload.rs), [tests/codex_launch.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/codex_launch.rs), [tests/codexagent.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/codexagent.rs), [tests/codexagent_sched.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/codexagent_sched.rs), [tests/console.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/console.rs), [tests/diagnose.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/diagnose.rs), [tests/drop_road.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/drop_road.rs), [tests/epipe.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/epipe.rs), [tests/far_open.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/far_open.rs), [tests/far_view.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/far_view.rs), [tests/far_words.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/far_words.rs), [tests/file_return.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/file_return.rs), [tests/gui_refused.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/gui_refused.rs), [tests/gui_stop.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/gui_stop.rs), [tests/home_relay.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/home_relay.rs), [tests/hop.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/hop.rs), [tests/idle_cost.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/idle_cost.rs), [tests/journal.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/journal.rs), [tests/load_day.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/load_day.rs), [tests/netlab/hop.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/hop.rs), [tests/netlab/lab.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/lab.rs), [tests/netlab/nat.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/nat.rs), [tests/netlab/paths.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/paths.rs), [tests/netlab/relay.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/relay.rs), [tests/netlab/uplink.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab/uplink.rs), [tests/netlab.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/netlab.rs), [tests/open.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/open.rs), [tests/open_once.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/open_once.rs), [tests/pipe.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/pipe.rs), [tests/port_pin.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/port_pin.rs), [tests/pull_resume.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/pull_resume.rs), [tests/pull_road.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/pull_road.rs), [tests/quit.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/quit.rs), [tests/quota_why.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/quota_why.rs), [tests/reach.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/reach.rs), [tests/redial.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/redial.rs), [tests/relay.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/relay.rs), [tests/relay_https.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/relay_https.rs), [tests/relay_qad.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/relay_qad.rs), [tests/resume_probe.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/resume_probe.rs), [tests/roads.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/roads.rs), [tests/roads_cost.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/roads_cost.rs), [tests/roster.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/roster.rs), [tests/rtt_probe.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/rtt_probe.rs), [tests/run.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/run.rs), [tests/scratch/mod.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/scratch/mod.rs), [tests/serve_keeper.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/serve_keeper.rs), [tests/serve_up.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/serve_up.rs), [tests/slow_relay.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/slow_relay.rs), [tests/sync_bytes.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/sync_bytes.rs), [tests/sync_mixed.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/sync_mixed.rs), [tests/term_echo.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/term_echo.rs), [tests/two_serves.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/two_serves.rs), [tests/watch_tree.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/watch_tree.rs), [tests/wires.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/wires.rs), [tests/words_probe.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/words_probe.rs), [tests/ws.rs](/Users/zgh/Desktop/working/khor/crates/cli/tests/ws.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/cli/Cargo.toml).

## crates/commands

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/execution.rs](/Users/zgh/Desktop/working/khor/crates/commands/src/execution.rs) | 689 | Request context for the CLI, slash composer, and future agent tools. Output is delivered at the print boundary; no process stream is redirected. | K — 执行范围 RAII、结果/输出 cap、private invocation；CLI 全命令矩阵未验 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/commands/src/lib.rs) | 6317 | Shared command engine for the CLI, slash interface, and future agent tools. Every verb delegates to the node API (docs/KHOR.md: CLI equals GUI). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/relay.rs](/Users/zgh/Desktop/working/khor/crates/commands/src/relay.rs) | 1025 | khor relay: an iroh relay, embedded in khor (docs/CONNECT.md 中继: 自家 khor relay; docs/NET.md 中继). | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置 tests/ 路径为 0；不代表没有测试，src 中仍可能有内联测试。

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/commands/Cargo.toml).

## crates/core

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/avatar/mark.rs](/Users/zgh/Desktop/working/khor/crates/core/src/avatar/mark.rs) | 858 | What a kind mark painted on a face wears: **which ink, and how much veil under it** — derived per face, here, next to the face itself. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/avatar.rs](/Users/zgh/Desktop/working/khor/crates/core/src/avatar.rs) | 2881 | A device's face: a geometric figure derived from its identity. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/core/src/lib.rs) | 4113 | 共享会话、节点、命令和 agent 数据模型；含 kind/category 内联域 | K — 协议/会话/agent 数据结构抽样；非逐类型序列化审查 |
| [src/map.rs](/Users/zgh/Desktop/working/khor/crates/core/src/map.rs) | 170 | Where the machines sit when the network is drawn as a mandala. | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置 tests/ 路径为 0；不代表没有测试，src 中仍可能有内联测试。

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/core/Cargo.toml).

## crates/detect

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/detect/src/lib.rs) | 435 | The PTY fallback: reading an agent's state off the screen it draws. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/tests.rs](/Users/zgh/Desktop/working/khor/crates/detect/src/tests.rs) | 905 | 屏幕状态识别 fixture/回归测试 | T — 测试/fixture 已定位，部分抽读；未执行 |

外置 tests/ 路径为 0；不代表没有测试，src 中仍可能有内联测试。

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/detect/Cargo.toml), [build.rs](/Users/zgh/Desktop/working/khor/crates/detect/build.rs).

## crates/net

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/carry.rs](/Users/zgh/Desktop/working/khor/crates/net/src/carry.rs) | 735 | **Roads khor carries itself** (round ROAD-2, audit/ROAD-BRIEF.md T5 and T6): an iroh path over a byte stream somebody else already holds. | K — 帧/queue/连接替换与释放；transport 故障注入未跑 |
| [src/endpoint.rs](/Users/zgh/Desktop/working/khor/crates/net/src/endpoint.rs) | 1491 | Binding, dialing, and the pairing ticket. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/identity.rs](/Users/zgh/Desktop/working/khor/crates/net/src/identity.rs) | 101 | The machine's identity key: load and persist. | K — 身份首建/读取/权限；B-01 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/net/src/lib.rs) | 10 | Connection substrate (docs/NET.md): who this machine is, and which road its connections are on. Relay tiers and pairing land here as they are built. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/path.rs](/Users/zgh/Desktop/working/khor/crates/net/src/path.rs) | 238 | Which road a connection is on, right now. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/root.rs](/Users/zgh/Desktop/working/khor/crates/net/src/root.rs) | 326 | The user's root key: generate, keep, and carry as 24 words (docs/CONNECT.md 身份：用户根钥匙). | K — 私有文件创建/并发发布；fallback 平台语义待验 B-01 |
| [src/trust.rs](/Users/zgh/Desktop/working/khor/crates/net/src/trust.rs) | 826 | Device certificates and the roster, signed by the user's root key (docs/CONNECT.md 身份：用户根钥匙, 分期 2). | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/carry.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/carry.rs), [tests/congestion.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/congestion.rs), [tests/mdns.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/mdns.rs), [tests/mdns_cost.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/mdns_cost.rs), [tests/pkarr.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/pkarr.rs), [tests/proxy.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/proxy.rs), [tests/qad_home.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/qad_home.rs), [tests/relay.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/relay.rs), [tests/relay_jitter.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/relay_jitter.rs), [tests/relay_restart.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/relay_restart.rs), [tests/relay_stuck.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/relay_stuck.rs), [tests/relay_token.rs](/Users/zgh/Desktop/working/khor/crates/net/tests/relay_token.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/net/Cargo.toml), [examples/qad_probe.rs](/Users/zgh/Desktop/working/khor/crates/net/examples/qad_probe.rs).

## crates/node

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/a2a_http.rs](/Users/zgh/Desktop/working/khor/crates/node/src/a2a_http.rs) | 870 | Native A2A HTTP targets. The remote machine needs an A2A agent, not khor. Local registrations and credentials never join the replicated ACP registry. | K — SendMessage/Task/Message/恢复/HTTP 边界；B-02 |
| [src/accounts.rs](/Users/zgh/Desktop/working/khor/crates/node/src/accounts.rs) | 3645 | Registered API accounts and what they have used — the 额度 page. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/acp_management.rs](/Users/zgh/Desktop/working/khor/crates/node/src/acp_management.rs) | 209 | Execution-node ACP setup jobs. Agent launch commands resolve through the registry. | K — 管理操作路由抽样；provider 配置失败矩阵未验 |
| [src/acp_mcp.rs](/Users/zgh/Desktop/working/khor/crates/node/src/acp_mcp.rs) | 40 | Translate ACP server descriptors at the two native CLI boundaries only. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/adaptor/claude.rs](/Users/zgh/Desktop/working/khor/crates/node/src/adaptor/claude.rs) | 2715 | Claude Code: one vendor, two signals. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/adaptor/codex.rs](/Users/zgh/Desktop/working/khor/crates/node/src/adaptor/codex.rs) | 1850 | Codex rollouts and live thread writers, on macOS, Linux and Windows. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/adaptor/mod.rs](/Users/zgh/Desktop/working/khor/crates/node/src/adaptor/mod.rs) | 1231 | Vendor adaptors: turning an agent's own files into khor sessions. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/adaptor/tmux.rs](/Users/zgh/Desktop/working/khor/crates/node/src/adaptor/tmux.rs) | 2553 | tmux: the container, not a vendor. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/agent_accounts.rs](/Users/zgh/Desktop/working/khor/crates/node/src/agent_accounts.rs) | 735 | Provider-owned setup runs only on the chosen credential home. | K — credential home 与 setup/cancel 边界抽样；真实 OAuth 未验 |
| [src/answers.rs](/Users/zgh/Desktop/working/khor/crates/node/src/answers.rs) | 556 | **The host's ledger of the questions a program asked its terminal** (T-4/T-5 of the terminal audit, audit/term.md, spec rules 4 and 5). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/backup.rs](/Users/zgh/Desktop/working/khor/crates/node/src/backup.rs) | 62 | Keeping the store out of the system's backups (docs/IOS.md 第二步). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/beside.rs](/Users/zgh/Desktop/working/khor/crates/node/src/beside.rs) | 885 | What the host's screen model keeps beside vt100 (item 66): the OSC 8 links on its cells and the pictures drawn among its rows. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/cagent.rs](/Users/zgh/Desktop/working/khor/crates/node/src/cagent.rs) | 1660 | khor's own claude shim: an ACP agent over stdio whose brain is the user's claude binary speaking its stream-json protocol. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/cagent_config.rs](/Users/zgh/Desktop/working/khor/crates/node/src/cagent_config.rs) | 82 | Native control/initialize model metadata; no model-name heuristics. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/carry.rs](/Users/zgh/Desktop/working/khor/crates/node/src/carry.rs) | 467 | **khor inside ssh** (round ROAD-2, audit/ROAD-BRIEF.md T5): a member that ssh reaches but that reaches neither the internet nor a relay is a full peer t… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/chat.rs](/Users/zgh/Desktop/working/khor/crates/node/src/chat.rs) | 263 | A board's documents: the khor-wide conversations (wide/<name>, the broadcast first), ridden on the chat CRDT. Read state lives in the seen CRDT the no… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/code.rs](/Users/zgh/Desktop/working/khor/crates/node/src/code.rs) | 607 | VS Code on a machine: whether it is there, and a web face pointed at one workspace (工作区批5). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/codex_process.rs](/Users/zgh/Desktop/working/khor/crates/node/src/codex_process.rs) | 318 | Read-only classification of native Codex command lines. Never launches a process. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/codex_terminal.rs](/Users/zgh/Desktop/working/khor/crates/node/src/codex_terminal.rs) | 473 | Codex's native thread-id OSC title identifies the currently selected thread. It is selection metadata, never transcript/screen text or a process-start g… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/codexagent.rs](/Users/zgh/Desktop/working/khor/crates/node/src/codexagent.rs) | 1075 | khor's own codex shim: an ACP agent over stdio whose brain is the user's codex binary speaking its app-server JSON-RPC protocol. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/codexagent_config.rs](/Users/zgh/Desktop/working/khor/crates/node/src/codexagent_config.rs) | 91 | Values come from app-server model/list, including provider-specific effort names. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/codexagent_updates.rs](/Users/zgh/Desktop/working/khor/crates/node/src/codexagent_updates.rs) | 214 | Codex 0.163 app-server items -> ACP v1 updates. No execution or approval here. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/commands.rs](/Users/zgh/Desktop/working/khor/crates/node/src/commands.rs) | 1312 | **The command ledger** (roadmap 1.2): what the shell in a hosted terminal ran, where, and how it ended — read by the host off the same bytes it already … | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/config.rs](/Users/zgh/Desktop/working/khor/crates/node/src/config.rs) | 484 | khor config: the preferences this machine keeps (docs/MORE.md 设置). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/cwd.rs](/Users/zgh/Desktop/working/khor/crates/node/src/cwd.rs) | 125 | **Where the process stands** (docs/SHELL.md 第十四轮 第 4 条). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/day.rs](/Users/zgh/Desktop/working/khor/crates/node/src/day.rs) | 854 | The day strip: what this machine's list said about each session over the last 24 hours, one minute at a time. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/delegation.rs](/Users/zgh/Desktop/working/khor/crates/node/src/delegation.rs) | 831 | Shared delegation tools for native A2A HTTP targets and detached ACP hosts. Khor's authenticated mesh can select the connector/execution home. | K — MCP→ACP/A2A 路由、cancel/receipt；后代终止时序待验 |
| [src/doctor.rs](/Users/zgh/Desktop/working/khor/crates/node/src/doctor.rs) | 1233 | khor doctor: one checklist, one row per thing that can be wrong (docs/MORE.md 自检). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/drops.rs](/Users/zgh/Desktop/working/khor/crates/node/src/drops.rs) | 239 | **A file put where a session's machine can read it** (item 60): the road under dropping a file onto a terminal. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/egress.rs](/Users/zgh/Desktop/working/khor/crates/node/src/egress.rs) | 182 | **Whether this machine has a relay to go out by** (2026-10-21, C7). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/face.rs](/Users/zgh/Desktop/working/khor/crates/node/src/face.rs) | 355 | The web face's **installation** facts: which port it binds, where its key lives, and how that key is read, minted and rotated. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/favicon.rs](/Users/zgh/Desktop/working/khor/crates/node/src/favicon.rs) | 866 | A page's icon, for the corner of its row in the browse sidebar (docs/SHELL.md 第四轮 第 2 条:「在浏览器, 是头像+网页icon」). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/files.rs](/Users/zgh/Desktop/working/khor/crates/node/src/files.rs) | 1705 | The files landing's serving half: what one paired machine shows another of its disk. Listing only — the payload path is the transfer kind's, and what 待批… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/git.rs](/Users/zgh/Desktop/working/khor/crates/node/src/git.rs) | 1418 | What git already knows about a workspace, read in this process. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/gui_host.rs](/Users/zgh/Desktop/working/khor/crates/node/src/gui_host.rs) | 1505 | The GUI session's host: one detached process holding one ACP agent. | K — ACP turn/permission/Stop/delegation 期限；B-05 |
| [src/hardware.rs](/Users/zgh/Desktop/working/khor/crates/node/src/hardware.rs) | 195 | Is this the same physical machine as last time? | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/history.rs](/Users/zgh/Desktop/working/khor/crates/node/src/history.rs) | 557 | The last day of this machine's four readings, five minutes at a stride — what the chart under the machine card's dials draws (2026-09-02:「最近24小时cpu」; 20… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/hop.rs](/Users/zgh/Desktop/working/khor/crates/node/src/hop.rs) | 872 | **The peer hop** (round ROAD-3, audit/ROAD-BRIEF.md T6): two machines with no direct road meet through a third khor machine that reaches both. The middl… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/host.rs](/Users/zgh/Desktop/working/khor/crates/node/src/host.rs) | 3292 | The persistent host: one detached process per 持久 session, owning the PTY its command runs in (docs/SESSION.md 寿命). The opener's terminal can go away; th… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/integration.rs](/Users/zgh/Desktop/working/khor/crates/node/src/integration.rs) | 176 | **Shell integration: how a hosted shell is taught to mark its commands** (roadmap 1.1) without touching the person's own startup files. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/ipc.rs](/Users/zgh/Desktop/working/khor/crates/node/src/ipc.rs) | 1047 | Local hand-off: a resident serve holds this key's one endpoint (two live endpoints on one key knock each other off — learned in production), so one-sh… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/join.rs](/Users/zgh/Desktop/working/khor/crates/node/src/join.rs) | 831 | Adding a machine over ssh (添加机器; docs/NET.md 卫星 and 转正). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/journal.rs](/Users/zgh/Desktop/working/khor/crates/node/src/journal.rs) | 1631 | **The connection journal** (NET-STABLE 第一步, 2026-10-08): what the resident saw on its way to each other machine, in order — every dial and the roads it … | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/keeper.rs](/Users/zgh/Desktop/working/khor/crates/node/src/keeper.rs) | 594 | The keeper: what stands between khor serve and dying silently. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/khor_agent.rs](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs) | 993 | The network's agent: one executor, durable receipts, and a private Pi pipe. Replicated receipts are display data and never enqueue model or tool work. | K — Pi prompt/secret/MCP/abort/进程收尾；B-03/B-05 |
| [src/khor_agent_updates.rs](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent_updates.rs) | 210 | Bounded display snapshots of standard ACP updates, never execution input. | K — ACP display 更新、脱敏/截断；B-03 |
| [src/legacy_codex.rs](/Users/zgh/Desktop/working/khor/crates/node/src/legacy_codex.rs) | 149 | Upgrade compatibility for already-open shells carrying the retired Codex shim. New terminals never install a shim. Old aliases forward verbatim and cann… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/node/src/lib.rs) | 5984 | One device's node: identity, the session surface (docs/SESSION.md), and the live link to the rest of the network (docs/NET.md). | K — Node open/claim identity 与 domain 入口抽样；B-01，其余多为 S |
| [src/link.rs](/Users/zgh/Desktop/working/khor/crates/node/src/link.rs) | 12224 | The live link: serve loop, pairing, and sync rounds over iroh. | K — sync round/失败传播、OpenAgentOnce 抽样；B-04，其余多为 S |
| [src/link_tree.rs](/Users/zgh/Desktop/working/khor/crates/node/src/link_tree.rs) | 148 | Node 的远端目录 snapshot/落地协议接入 | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/list.rs](/Users/zgh/Desktop/working/khor/crates/node/src/list.rs) | 456 | List order: the one place the session list is arranged. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/live.rs](/Users/zgh/Desktop/working/khor/crates/node/src/live.rs) | 5522 | The live kind: sessions backed by a real local process — an agent TUI reporting through hooks, or a command wrapped by khor run (临时: it lives and dies… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/lock.rs](/Users/zgh/Desktop/working/khor/crates/node/src/lock.rs) | 234 | khor lock: the password on the faces (docs/MORE.md 安全). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/mcp.rs](/Users/zgh/Desktop/working/khor/crates/node/src/mcp.rs) | 636 | khor's verbs, offered to an agent as tools (docs/AGENT.md). | K — 工具读写分类、审批后执行；新增工具策略回归矩阵待补 |
| [src/merge.rs](/Users/zgh/Desktop/working/khor/crates/node/src/merge.rs) | 241 | Three texts in, one text out: the merge behind a refused save (工作区批7). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/modes.rs](/Users/zgh/Desktop/working/khor/crates/node/src/modes.rs) | 439 | The terminal modes a byte stream switched on, kept for the bytes a replay no longer holds. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/once.rs](/Users/zgh/Desktop/working/khor/crates/node/src/once.rs) | 106 | **A session made at most once per token** (round NET-7, from ux's first-run walk): the phone asked the Mac for a session, the Mac quit before the answer… | K — receipt/进程锁保留；B-07，崩溃原子性另议 |
| [src/pause.rs](/Users/zgh/Desktop/working/khor/crates/node/src/pause.rs) | 207 | khor pause / khor resume: this machine stops talking to the network without leaving it (docs/MORE.md 暂停). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/pictures.rs](/Users/zgh/Desktop/working/khor/crates/node/src/pictures.rs) | 412 | Pictures a program drew, kept beside the replay ring instead of in it. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/pipe.rs](/Users/zgh/Desktop/working/khor/crates/node/src/pipe.rs) | 272 | **ssh inside khor** (round ROAD-1, audit/ROAD-BRIEF.md T2): a byte pipe from this process's stdin and stdout to host:port as another khor machine's ne… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/place.rs](/Users/zgh/Desktop/working/khor/crates/node/src/place.rs) | 415 | **常去的地方: the directories this machine's own records say work happened in, folded to the project each belongs to.** | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/post.rs](/Users/zgh/Desktop/working/khor/crates/node/src/post.rs) | 495 | **One channel** (STORE-BRIEF phase 5): a message to the broadcast is text, plus the machines it is pushed to, plus files — composed once, resolved once,… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/preview.rs](/Users/zgh/Desktop/working/khor/crates/node/src/preview.rs) | 528 | What a file is, and what it looks like when it is not code (工作区批6). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/proto.rs](/Users/zgh/Desktop/working/khor/crates/node/src/proto.rs) | 2794 | Wire frames between nodes. msgpack: field order is wire order, new fields go at the tail with serde defaults, skip_serializing_if is banned — a mid-fr… | K — msgpack 尾部兼容、is_reading/rides_early 抽样；所有变体矩阵待补 |
| [src/quota/claude.rs](/Users/zgh/Desktop/working/khor/crates/node/src/quota/claude.rs) | 865 | Claude Code's subscription: one [super::Provider], two strategies. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/quota/codex.rs](/Users/zgh/Desktop/working/khor/crates/node/src/quota/codex.rs) | 688 | Codex's subscription (a ChatGPT plan): one [super::Provider], two strategies (docs/SHELL.md 第九轮). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/quota/mod.rs](/Users/zgh/Desktop/working/khor/crates/node/src/quota/mod.rs) | 814 | What a vendor subscription on this machine has left — **one abstraction, a description per vendor, and a chain of ways to read it** (docs/SHELL.md 第九轮). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/reach.rs](/Users/zgh/Desktop/working/khor/crates/node/src/reach.rs) | 338 | **Give an address, every machine tries it** (round ROAD-1, khor reach; audit/ROAD-BRIEF.md T1). A person names a host and a port (ssh's 22 unless told… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/reactor.rs](/Users/zgh/Desktop/working/khor/crates/node/src/reactor.rs) | 228 | How long the serve's reactor was unavailable — a high-water mark. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/recognition.rs](/Users/zgh/Desktop/working/khor/crates/node/src/recognition.rs) | 764 | Per-machine agent integration. The report is read from vendor configuration, never a saved UI boolean. Codex uses native title settings, not a launcher. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/road.rs](/Users/zgh/Desktop/working/khor/crates/node/src/road.rs) | 860 | **The file road** (audit/STORE-BRIEF.md, foundation A): one way for bytes to land on a machine, whatever asked for them — a file dropped on a terminal, … | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/roadbook.rs](/Users/zgh/Desktop/working/khor/crates/node/src/roadbook.rs) | 643 | **What this machine writes about its own roads** (round NET-12, the 我的 khor dial): for each other machine, how this one reaches it — direct, by which re… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/roads.rs](/Users/zgh/Desktop/working/khor/crates/node/src/roads.rs) | 435 | How this machine's recent dials went, per far machine (docs/ CONNECT.md 度量与验收: 首包时间、直连占比) — the resident's memory behind [khor_core::RoadTally]. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/satellite.rs](/Users/zgh/Desktop/working/khor/crates/node/src/satellite.rs) | 2960 | Satellites: machines only ssh can reach, spoken for by a sponsor (附属机器批; docs/NET.md 卫星). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/secret.rs](/Users/zgh/Desktop/working/khor/crates/node/src/secret.rs) | 404 | The secrets khor keeps: the password of an ssh host the person chose to log in to with one, or the passphrase of the key file they picked for it (添加机器 认… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/serves.rs](/Users/zgh/Desktop/working/khor/crates/node/src/serves.rs) | 296 | Every khor serve this user is running on this machine, whichever store it serves (SHELL.md 第五轮 第 0 条). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/shell_start.rs](/Users/zgh/Desktop/working/khor/crates/node/src/shell_start.rs) | 166 | A command typed into a freshly-created shell. The target chooses its own shell and home; the host holds the command until a terminal can answer it. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/subscriptions.rs](/Users/zgh/Desktop/working/khor/crates/node/src/subscriptions.rs) | 282 | Discover logins on their own machine, then register only the selected one. Neither discovery nor import transports a credential or calls a usage API. | K — 登录发现/注册、account_map 编码；凭证刷新未深审 |
| [src/tabs.rs](/Users/zgh/Desktop/working/khor/crates/node/src/tabs.rs) | 1161 | What this screen has open in its browsing tabs (docs/SHELL.md 第 2 条). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/testdir.rs](/Users/zgh/Desktop/working/khor/crates/node/src/testdir.rs) | 80 | Temp trees for this crate's unit tests that **take themselves away however the test leaves** (SHELL.md 第五轮 第 4 条). | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/tests.rs](/Users/zgh/Desktop/working/khor/crates/node/src/tests.rs) | 3424 | node 主库单元测试集合 | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/top/gpu.rs](/Users/zgh/Desktop/working/khor/crates/node/src/top/gpu.rs) | 152 | What each process is doing to the graphics cards, by pid. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/top.rs](/Users/zgh/Desktop/working/khor/crates/node/src/top.rs) | 1138 | What is eating this machine right now: the processes at the top of the CPU, the answer 进程 top on the machine card draws (2026-09-02:「环下面显示进程top」). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/transfer.rs](/Users/zgh/Desktop/working/khor/crates/node/src/transfer.rs) | 433 | The transfer kind: bytes ride point-to-point, never the CRDT (docs/NET.md 同步). A file message in the broadcast is the summary — it travels the CRDT to e… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/tree.rs](/Users/zgh/Desktop/working/khor/crates/node/src/tree.rs) | 372 | A bounded directory snapshot and one atomic directory landing. Paths on the wire are relative with / separators. Links and special files are refused e… | K — 路径合法性、总量 cap、无覆盖落地；平台故障注入未跑 |
| [src/trust.rs](/Users/zgh/Desktop/working/khor/crates/node/src/trust.rs) | 1582 | Trust on this machine: which root it verifies with, which roster decides who counts, and the verbs that change either (docs/CONNECT.md 身份：用户根钥匙, 分期 2). … | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/tunnel.rs](/Users/zgh/Desktop/working/khor/crates/node/src/tunnel.rs) | 599 | Borrowing a machine's network (docs/NET.md 借网): a paired peer opens a byte pipe through this machine to some host:port this machine can reach. One exi… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/usage/ledger.rs](/Users/zgh/Desktop/working/khor/crates/node/src/usage/ledger.rs) | 437 | What khor has already counted, kept so that a transcript the vendor deletes does not take its history with it. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/usage/tests.rs](/Users/zgh/Desktop/working/khor/crates/node/src/usage/tests.rs) | 1814 | Against a hand-written tree, not this machine's own. | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/usage.rs](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs) | 3029 | What the agents on this machine have spent, read off their own files. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals/gpu/mac.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/gpu/mac.rs) | 483 | macOS: the GPU's own numbers, read out of the IO registry. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals/gpu/nvidia.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/gpu/nvidia.rs) | 187 | Linux: NVIDIA cards, through the driver's own library. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals/gpu.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/gpu.rs) | 273 | The graphics hardware's share of a reading. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals/local.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/local.rs) | 619 | This machine's readings, **taken in the background and read from memory** (docs/SHELL.md 第十六轮). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals/model.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/model.rs) | 280 | Which machine this is (SHELL.md 第五轮 第 1 条): a stable kind the face picks an icon from, and the name the machine gives itself. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/vitals.rs](/Users/zgh/Desktop/working/khor/crates/node/src/vitals.rs) | 518 | This machine's live readings (khor_core::Vitals). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/watch.rs](/Users/zgh/Desktop/working/khor/crates/node/src/watch.rs) | 516 | **A directory tree pushed, not asked** (round NET-4): the machine that holds the directories watches the ones a face has open and says when one changed;… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/wide.rs](/Users/zgh/Desktop/working/khor/crates/node/src/wide.rs) | 482 | Khor-wide sessions (STORE-BRIEF foundation B): a session that belongs to the whole khor, not to one machine. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/wires.rs](/Users/zgh/Desktop/working/khor/crates/node/src/wires.rs) | 1608 | The wires this machine holds to other machines (docs/NET.md 连接): **one long-lived QUIC connection per far machine and protocol**, opened on first need, … | K — stable_id 防旧连接删新连接、拨号锁/backoff；网络实验未跑 |
| [src/workspace.rs](/Users/zgh/Desktop/working/khor/crates/node/src/workspace.rs) | 377 | **工作区: a directory on a machine that a person said they work in.** | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/wsfacts.rs](/Users/zgh/Desktop/working/khor/crates/node/src/wsfacts.rs) | 492 | **A workspace row's facts, kept by the machine that holds it** (round NET-8, for UX-15's workspace row: 「3 个改动」, the branch, and when it last changed). | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/a2a_http.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/a2a_http.rs), [tests/acp_setup.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/acp_setup.rs), [tests/act.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/act.rs), [tests/agents_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/agents_wire.rs), [tests/avatar.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/avatar.rs), [tests/broadcast.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/broadcast.rs), [tests/command_ledger.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/command_ledger.rs), [tests/cost.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/cost.rs), [tests/cwd_pin.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/cwd_pin.rs), [tests/day_record.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/day_record.rs), [tests/delegation_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/delegation_wire.rs), [tests/departed.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/departed.rs), [tests/early.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/early.rs), [tests/file_handoff_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/file_handoff_wire.rs), [tests/files_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/files_wire.rs), [tests/gui_session.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/gui_session.rs), [tests/hearing.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/hearing.rs), [tests/history_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/history_wire.rs), [tests/host_replay.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/host_replay.rs), [tests/host_shell_tmux.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/host_shell_tmux.rs), [tests/host_snapshot.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/host_snapshot.rs), [tests/host_tmux_attach.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/host_tmux_attach.rs), [tests/idle_or_down.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/idle_or_down.rs), [tests/ipc.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/ipc.rs), [tests/khor_agent_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/khor_agent_wire.rs), [tests/lang.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/lang.rs), [tests/live.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/live.rs), [tests/load.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/load.rs), [tests/machine_pause.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/machine_pause.rs), [tests/mouse_reports.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/mouse_reports.rs), [tests/native_handoff.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/native_handoff.rs), [tests/net10_probe.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/net10_probe.rs), [tests/no_host.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/no_host.rs), [tests/one_channel.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/one_channel.rs), [tests/open_from_phone.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/open_from_phone.rs), [tests/pairing.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/pairing.rs), [tests/pane_history_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/pane_history_wire.rs), [tests/peer_pause.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/peer_pause.rs), [tests/quota_live.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/quota_live.rs), [tests/real_disk.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/real_disk.rs), [tests/recognition_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/recognition_wire.rs), [tests/refused_dial.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/refused_dial.rs), [tests/relay_name.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/relay_name.rs), [tests/relays.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/relays.rs), [tests/resume.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/resume.rs), [tests/screen_face.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/screen_face.rs), [tests/slow_disk.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/slow_disk.rs), [tests/slow_reading.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/slow_reading.rs), [tests/subscriptions.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/subscriptions.rs), [tests/table_lost.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/table_lost.rs), [tests/takeover_safety.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/takeover_safety.rs), [tests/tmux_extkeys.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/tmux_extkeys.rs), [tests/tmux_mouse.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/tmux_mouse.rs), [tests/transfer.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/transfer.rs), [tests/transfer_race.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/transfer_race.rs), [tests/tunnel_wire.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/tunnel_wire.rs), [tests/usage.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/usage.rs), [tests/util/delegation_pi.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/util/delegation_pi.rs), [tests/util/mod.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/util/mod.rs), [tests/vitals.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/vitals.rs), [tests/workspace_facts.rs](/Users/zgh/Desktop/working/khor/crates/node/tests/workspace_facts.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/node/Cargo.toml), [build.rs](/Users/zgh/Desktop/working/khor/crates/node/build.rs), [examples/day_fixture.rs](/Users/zgh/Desktop/working/khor/crates/node/examples/day_fixture.rs).

## crates/sync

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/accounts.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/accounts.rs) | 593 | Registered API accounts: whose usage the 额度 page draws a ring for. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/agents/registry.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/agents/registry.rs) | 440 | The agents khor knows how to start without being told a command. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/agents.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/agents.rs) | 510 | The agent registry: which ACP agents this person has told khor about, replicated everywhere — the open half of "whose session is this" (批⑥, khor as a te… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/chat/doc/tests.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/chat/doc/tests.rs) | 401 | ChatDoc 单元测试 | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/chat/doc.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/chat/doc.rs) | 709 | The document model of one conversation. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/chat/integration.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/chat/integration.rs) | 169 | Cross-module integration: doc + store + plan composed the way a real driver composes them. Single-module behavior is tested next to its code. | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/chat.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/chat.rs) | 289 | Chat: one CRDT document per channel, persisted through the generic block store. A channel is a machine's window — one machine, one channel, every device… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/devices.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/devices.rs) | 1972 | The device table: who is in the network (docs/NET.md). One CRDT map, replicated everywhere — joining any one device reveals the whole network. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/dirpins.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/dirpins.rs) | 163 | Pinned directories: one flag per (machine, absolute path), replicated everywhere — the files landing's shortcuts. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/glue.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/glue.rs) | 28 | The Doc methods every loro-backed table implements identically. Extracted at the third copy (chat, devices, seen). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/lib.rs) | 28 | Decision-class data that replicates across the whole network (docs/NET.md): CRDT documents, their on-disk block stores, and the two transports that move… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/pins.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/pins.rs) | 281 | Pinned sessions: one flag per session id, replicated everywhere (docs/NET.md — the same decision class as [crate::seen]). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/plan.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/plan.rs) | 185 | How two sides align over files. Pure planning — no IO, so any transport (live stream, ssh) can drive it. | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/recents.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/recents.rs) | 541 | Where this person has already been: one last-visit instant per (kind, machine, place), replicated everywhere (docs/NET.md — the same decision class as [… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/roster.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/roster.rs) | 207 | What is on the session list: the **roster** (docs/SHELL.md 第八轮, 2026-09-22:「session 里放什么，应该是用户决定，不是我们自动」). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/seen.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/seen.rs) | 346 | Read state: one watermark per session, replicated everywhere (docs/NET.md: seen is a decision — clear on the phone, the desktop badge clears too). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/store/tests.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/store/tests.rs) | 1112 | BlockStore 单元测试 | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/store.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/store.rs) | 1494 | Persistence for CRDT documents: a stack of append-only block files anyone can read or write. | K — flush/loaded block/compaction/atomic write 抽样；掉电持久性未验 |
| [src/testutil.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/testutil.rs) | 32 | Shared scaffolding for the chat tests. | T — 测试/fixture 已定位，部分抽读；未执行 |
| [src/titles.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/titles.rs) | 231 | User-given session names: one override per session id, replicated everywhere (docs/NET.md — the same decision class as [crate::pins] and [crate::seen… | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/webpins.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/webpins.rs) | 160 | Pinned web pages: one flag per (exit machine, URL), replicated everywhere — the browser landing's shortcuts (docs/NET.md 借网). | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/wire.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/wire.rs) | 635 | Encoding for the live path: version vectors and increments as strings a request/response frame can carry, for any [Doc]. When to send, to whom, and ho… | K — delta cap、epoch、flush/ack、round 追踪；B-04 |
| [src/workspaces.rs](/Users/zgh/Desktop/working/khor/crates/sync/src/workspaces.rs) | 331 | Workspaces: the directories a person declared they work in, one flag per (machine, absolute path), replicated everywhere. | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/catchup.rs](/Users/zgh/Desktop/working/khor/crates/sync/tests/catchup.rs), [tests/churn_probe.rs](/Users/zgh/Desktop/working/khor/crates/sync/tests/churn_probe.rs), [tests/peers_memory.rs](/Users/zgh/Desktop/working/khor/crates/sync/tests/peers_memory.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/sync/Cargo.toml).

## crates/web

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/key.rs](/Users/zgh/Desktop/working/khor/crates/web/src/key.rs) | 83 | The web face's key: the one thing a browser must show to get data. | K — key 读取/匹配边界；浏览器/平台联动未验 |
| [src/lib.rs](/Users/zgh/Desktop/working/khor/crates/web/src/lib.rs) | 538 | The web face: khor's own GUI, served to whatever browser opens this machine's address. A phone, somebody else's laptop, a tablet on the sofa — one addre… | K — 鉴权→body→dispatch、worker 生命周期；B-06 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[tests/face.rs](/Users/zgh/Desktop/working/khor/crates/web/tests/face.rs).

构建/manifest/example 入口（G：已登记，未完整审查）：[Cargo.toml](/Users/zgh/Desktop/working/khor/crates/web/Cargo.toml), [build.rs](/Users/zgh/Desktop/working/khor/crates/web/build.rs).

## apps/agent

| 文件模块 | 行数 | 职责摘要 | 状态与后续 |
| --- | ---: | --- | --- |
| [src/acp.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/acp.ts) | 34 | ACP v1 与 Pi 会话的 TS 适配器 | K — ACP session 适配/请求操作抽样；真实模型配置变化未验 |
| [src/dev.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/dev.ts) | 122 | 开发入口与 stdio/管理模式选择 | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/gate.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/gate.ts) | 124 | agent 工具执行门与取消/审批协调 | K — 工具门/取消的 TS 一侧；Rust 保持最终授权 |
| [src/host.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/host.ts) | 130 | Rust 私有 Pi sidecar 协议宿主 | K — Pi start/credentials/tools/prompt/关闭；B-02/B-03/B-05 边界 |
| [src/management.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/management.ts) | 47 | provider 账户管理与登录操作 | K — provider 管理与 credentials 输出路径；真实登录未验 |
| [src/mcp.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/mcp.ts) | 47 | MCP transport/client 与请求生命周期 | K — stdio JSON-RPC request/response 与 abort；不合作服务未验 |
| [src/models.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/models.ts) | 221 | provider/account/model 的映射与未列出模型处理 | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/session-state.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/session-state.ts) | 39 | Pi 配置/模型状态转换成 ACP v1 数据 | K — ACP state/model/config options 映射抽样；版本变化待验 |
| [src/session.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/session.ts) | 69 | Pi AgentSession 配置与持久化会话 | K — Pi session 创建/恢复/配置；故障注入待补 |
| [src/tools.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/tools.ts) | 151 | MCP 工具定义与 Pi 工具适配 | S — 职责/API 已结构巡检；实现与故障行为待深审 |
| [src/verbs.ts](/Users/zgh/Desktop/working/khor/apps/agent/src/verbs.ts) | 106 | khor 工具/动词描述与 schema 入口 | S — 职责/API 已结构巡检；实现与故障行为待深审 |

外置测试路径（均未执行；关键抽读见报告对应发现）：

[test/acp.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/acp.test.ts), [test/gate.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/gate.test.ts), [test/host.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/host.test.ts), [test/management.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/management.test.ts), [test/models.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/models.test.ts), [test/session-state.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/session-state.test.ts), [test/tools.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/tools.test.ts), [test/verbs.test.ts](/Users/zgh/Desktop/working/khor/apps/agent/test/verbs.test.ts).

构建/manifest/example 入口（G：已登记，未完整审查）：[package.json](/Users/zgh/Desktop/working/khor/apps/agent/package.json), [build.mjs](/Users/zgh/Desktop/working/khor/apps/agent/build.mjs), [scripts/providers.ts](/Users/zgh/Desktop/working/khor/apps/agent/scripts/providers.ts).

## 内联生产模块补充

以下没有单独源文件，随所属文件计数；平台 cfg 分支仍各自登记。内联测试模块的存在按文件统计，不逐个声称已审查。

| 所属文件 / 内联模块 | 状态与边界 |
| --- | --- |
| [crates/catalog/src/lib.rs :: count](/Users/zgh/Desktop/working/khor/crates/catalog/src/lib.rs:21) | S：文本数量格式；由 catalog gate 辅助，未逐语言场景验证 |
| [crates/core/src/lib.rs :: kind](/Users/zgh/Desktop/working/khor/crates/core/src/lib.rs:39) | K：共享 kind 词汇与会话分类抽样 |
| [crates/core/src/lib.rs :: category](/Users/zgh/Desktop/working/khor/crates/core/src/lib.rs:59) | K：category/兼容性数据边界抽样 |
| [crates/web/src/lib.rs :: assets](/Users/zgh/Desktop/working/khor/crates/web/src/lib.rs:28) | G：OUT_DIR 生成静态资源入口 |
| [crates/node/src/accounts.rs :: trouble](/Users/zgh/Desktop/working/khor/crates/node/src/accounts.rs:105) | S：账户错误域，真实 quota/OAuth 边界待深审 |
| [crates/node/src/backup.rs :: ios](/Users/zgh/Desktop/working/khor/crates/node/src/backup.rs:49) | S：系统备份排除平台分支，未在设备执行 |
| [crates/node/src/secret.rs :: os (macOS)](/Users/zgh/Desktop/working/khor/crates/node/src/secret.rs:249) | S：Keychain 适配，平台权限/错误待深审 |
| [crates/node/src/secret.rs :: os (Linux)](/Users/zgh/Desktop/working/khor/crates/node/src/secret.rs:279) | S：平台秘密保存适配，失败恢复待深审 |
| [crates/node/src/secret.rs :: os (other)](/Users/zgh/Desktop/working/khor/crates/node/src/secret.rs:341) | S：其他平台 fallback，未执行 |
| [crates/node/src/subscriptions.rs :: account_map](/Users/zgh/Desktop/working/khor/crates/node/src/subscriptions.rs:41) | K：JSON 形状跨 MessagePack 边界防字段移位 |
| [crates/node/src/gui_host.rs :: refused](/Users/zgh/Desktop/working/khor/crates/node/src/gui_host.rs:396) | S：GUI ACP 拒绝词域；调用路径部分在 K 范围 |
| [crates/node/src/delegation.rs :: wire_json](/Users/zgh/Desktop/working/khor/crates/node/src/delegation.rs:780) | K：可省略 JSON 字段不直接穿位置型 MessagePack |
| [crates/node/src/vitals/model.rs :: mac](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/model.rs:127) | S：机器型号平台适配，未设备验证 |
| [crates/node/src/vitals/local.rs :: mac](/Users/zgh/Desktop/working/khor/crates/node/src/vitals/local.rs:99) | S：本机统计平台分支，资源释放待深审 |
| [crates/node/src/top/gpu.rs :: nvidia](/Users/zgh/Desktop/working/khor/crates/node/src/top/gpu.rs:65) | S：进程 GPU 查询，driver 缺失/权限行为未验证 |
| [crates/node/src/usage.rs :: claude](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:1150) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: codex](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:1378) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: gemini](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:1574) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: pi](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:1789) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: roo](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:2044) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: qwen](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:2278) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: junie](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:2431) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: amp](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:2556) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |
| [crates/node/src/usage.rs :: openclaw](/Users/zgh/Desktop/working/khor/crates/node/src/usage.rs:2881) | S：provider transcript/token 口径适配；仅结构登记，去重/截断/版本变化待逐 vendor 深审 |

## 未覆盖实现与下一轮入口

- node 的 host/live、adaptor/*、accounts/quota、usage 各 vendor、satellite/join/hop/pipe/tunnel/road、文件搜索/编辑/删除及 terminals/pictures/beside 只有结构登记和零星调用链阅读；必须按状态机/持久边界逐批追踪，不能从当前无 finding 推导安全。
- node/lib 与 link 虽为 K，但 K 仅限表中指定入口。大部分请求变体、read/write/early 分类、重连时序及 legacy 兼容仍需系统化覆盖。
- sync 各文档 schema 与迁移以 S 为主；本轮 K 集中在共同 store/wire。不能因此认为每一种 CRDT 字段/迁移都已证明收敛。
- ACP/Pi/A2A 的 stub/unit fixture 已定位或抽读；真实 provider、OAuth、网络抖动、平台子进程树、磁盘满/掉电均未运行。本轮不要求接触真实账户或配对 store。
- GUI core/native 由 Q-001A 负责；vendor 代码、打包/签名/发布链由 root/其他任务负责，不在此清单范围。没有把上游源码计入第一方模块。

清单生成时重新检查 clean baseline；源码未写入。审计产物仅位于中央 tasks/Q-001B*.md。
