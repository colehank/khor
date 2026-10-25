# Q-001B：Rust、agent 与协议首轮审计报告

审计者：backend_audit。日期：2026-10-25。固定基线：main `8704a503a27b36ad58f922690de50e766f8bd485`。源目录：`/Users/zgh/Desktop/working/khor`。本报告只记录此基线，不代表后续 wave1 的修复状态。

## 结论与审查边界

最适合立即落地的三项修复是 **B-01 首次 identity 创建的并发覆盖、B-02 A2A 直接 Message 的 context 无法续聊、B-03 inference/OAuth 错误被空字符串脱敏破坏**。它们都有明确的源码因果链，改动范围较小，能各自建立回归门。B-04 的 live sync 大缺口恢复需要单独设计协议兼容；B-05 的取消截止需要先明确“请求取消”和“执行已停止”的状态语义。

范围内 11 个 crate、154 个 Rust src 文件、apps/agent 的 11 个 TS src 文件已建立逐文件覆盖清单，共 138,946 行 Rust src 与 1,090 行 TS src（包含注释、空行和内联测试，**不是生产 SLOC，也不是逐行审查计数**）。另外登记 142 个 Rust tests 路径、8 个 TS test 路径及构建/生成入口。完整明细见 [Q-001B-modules.md](Q-001B-modules.md)。所有模块做了职责/API 的结构巡检；只有标记 K 的文件中列出的边界做了实现追踪。大型会话、SSH、终端、usage、vendor adaptor 等模块仍有明确的深审缺口。本轮不是“全仓每行已审查”。

先使用 code-review-graph 的 get_minimal_context，再使用 architecture、semantic_search、file_summary、review_context、callers_of、tests_for；仓库没有 `.codegraph/`，未运行 CodeGraph。图谱虽报告 head_matches_build=true，源码内容仍存在过期：host.ts 为 119 行而基线为 130 行，acp/lib.rs 为 884 行而基线为 1,102 行，若干新增模块缺失。随后才回退 rg/nl 与固定基线文件。图谱的 tests_for=0 不作为无测试证据。

没有修改业务代码，没有编译、运行测试、重启服务或发布。以下“源码确认”是可从本提交控制流推出的缺陷，**不表示本轮已执行复现**；现有测试条目也只是阅读/定位，并未执行。工作树在审查起止均无修改，HEAD 未变。

## 优先级与分类

P1：身份或主流程持续不可用，优先安排有针对性的修复。P2：明确功能错误或边界缺口，应排入近期批次。P3：长期资源/结构债，按收益安排。无 P0 证据。

| 编号 | 分类 | 优先级 | 触发与后果 | 建议批次 |
| --- | --- | --- | --- | --- |
| B-01 | 源码确认缺陷 | P1 | 同一新 store 并发打开，可能返回两个不同机器身份；私钥创建后才 chmod | identity 原子发布 |
| B-02 | 源码确认缺陷 | P2 | A2A 直接 Message 返回 context 后，合法后续请求在本地被拒绝 | A2A context 所有权 |
| B-03 | 源码确认缺陷 | P2 | inference/OAuth 的空 key 进入 replace，错误文本插满脱敏标记并二次膨胀 | 统一脱敏函数 |
| B-04 | 源码确认的协议活性缺口 | P2 | 某文档合法历史 delta 超过 4 MiB 后，live wire 重试不能补齐该缺口 | sync 大缺口恢复设计 |
| B-05 | 可疑活性机制，待故障注入 | P2 | sidecar/ACP 不确认取消时，普通 Stop 无共同退出截止 | 取消与进程生命周期 |
| B-06 | 源码确认的输入处理缺口；资源影响待量化 | P2 | 已认证 JSON API 不限 body，忽略读失败，将非法 JSON 当 null 分发 | Web 请求边界 |
| B-07 | 源码确认的无界保留；规模风险 | P3 | 每个新 once token 的进程内锁永久驻留 | token 锁生命周期 |

## B-01：identity 首次写入不是原子的 create-once

**证据。** [identity.rs:16](/Users/zgh/Desktop/working/khor/crates/net/src/identity.rs:16) 的 16–31 行先 exists 再生成/写入，成功后返回本次生成的 key；[identity.rs:35](/Users/zgh/Desktop/working/khor/crates/net/src/identity.rs:35) 的 35–43 行使用可截断已有目标的 fs::write，然后才 chmod 0600。真实入口 [node/lib.rs:1058](/Users/zgh/Desktop/working/khor/crates/node/src/lib.rs:1058) 的 1058–1071 行在 claim_store 后直接调用该函数；[node/lib.rs:813](/Users/zgh/Desktop/working/khor/crates/node/src/lib.rs:813) 和 844–862 行的 claim/stamp 不提供创建锁。

**触发。** 同一个全新 KHOR_HOME 的 GUI、resident 或 CLI 并发首次打开：A、B 都看到文件不存在，各自生成 K1/K2，最后一次 fs::write 决定磁盘内容，但两个调用者都返回自己的 key。并发读还可能观察到截断后、写完前的文件。Unix 默认 umask 为 022 且父目录允许遍历时，文件在 chmod 前可短暂为 0644；已有读取者也可能先因权限检查拒绝启动。该权限窗口有明确环境前提，不等于已发生密钥泄漏。

**影响。** 当前进程使用的 DeviceId 与磁盘重启后的身份不一致；配对、路由和本地记录可能指向不同身份。首建的原子性与权限保证均不满足文件注释宣称的稳定私钥契约。

**现有验证/缺口。** [identity.rs:69](/Users/zgh/Desktop/working/khor/crates/net/src/identity.rs:69) 测顺序持久性，82 行起测试已存在的宽权限文件被拒绝；未覆盖并发首次创建、首次字节权限或读半成品。[root.rs:87](/Users/zgh/Desktop/working/khor/crates/net/src/root.rs:87) 已有私有临时文件加 hard_link 发布先例，但 104–108 行 fallback rename 仍可能覆盖，不能直接视为所有文件系统上的完整解决方案。

**建议与验收。** 创建时就设 0600；完整写入私有临时文件，再用不会覆盖赢家的原子发布；失败竞争者重新读取赢家。明确不支持 hard_link 的平台/filesystem 策略。隔离临时目录做多进程 barrier 注入：所有成功调用返回的 public key 必须等于最终磁盘 key，读取者不见半写文件；umask=022 下检查创建时权限。修复仅使用临时身份，不触碰真实配对 store。

## B-02：A2A Message context 有回执、无可用的所有权记录

**证据。** [a2a_http.rs:579](/Users/zgh/Desktop/working/khor/crates/node/src/a2a_http.rs:579) 的 579–607 行接受直接 Message，并校验 role、内容大小和 context/task 一致性；609–617 行仅把响应保存在 inputs 回执。[a2a_http.rs:722](/Users/zgh/Desktop/working/khor/crates/node/src/a2a_http.rs:722) 的 722–728 行校验后续 context 时只搜索 tasks。agent 提示明确要求对后续请求复用返回的 context，见 [host.ts:61](/Users/zgh/Desktop/working/khor/apps/agent/src/host.ts:61)。

**触发。** native A2A target 的首次 SendMessage 返回直接 Message，带新 context C，不返回 Task。相同 caller 对相同 agent 用 C 发新的 request ID；本地 tasks 中没有 C，于是在发出 HTTP 前报 “A2A context does not belong to this caller and agent”。仅当该 context 恰已有 Task 回执时才不受影响。

**影响。** 合法直接回复的后续对话被拒绝，重试相同已完成请求虽能回放，却不能继续该 context。

**现有验证/缺口。** [a2a_http 测试:63](/Users/zgh/Desktop/working/khor/crates/node/tests/a2a_http.rs:63) 的 inline fixture 不带 context；462–471 行仅检查 Message 结果和相同 request 回放。324–338 行验证的是 Task context 的后续请求。它们不能覆盖此缺陷。

**建议与验收。** 将上下文所有权从 Task 身份中分离，或从已持久化的 Message/Task 成功回执统一查询；保持 caller、目标 agent、endpoint 绑定，不能为 Message 伪造 Task。扩展 inline fixture 返回 C，验证同 caller 可继续、不同 caller/agent 被拒绝、重启后继续有效，并保留未知 send 不自动重放的约束。

## B-03：inference/OAuth 的空 key 破坏错误文本

**证据。** [khor_agent.rs:239](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs:239) 的 239–244 行在 account.inference 存在时令 key 为空字符串，302–305 行改用 credentialPath。失败事件在 [khor_agent.rs:667](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs:667) 的 667–671 行无条件 replace(key, "[redacted]")，最终错误在 713–716 行又做同一替换。Rust 空字符串模式会在字符边界插入替换文本；第二次会连第一次的标记一起展开。

**触发/影响。** 通过 inference 账户执行，只要 sidecar 返回 failed（或其他路径最终返回 Err），原本可读的 provider/网络错误就被大量脱敏标记污染。failed 事件路径发生两次替换，增加内存/持久化文本大小。这是错误可用性问题；本轮没有据此认定真实凭证泄漏。

**现有验证/缺口。** [append_text:449](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs:449) 已明确跳过空 key；[khor_agent_updates.rs:9](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent_updates.rs:9) 也有空值防护。现有 MCP 测试针对撤销审批，host fixture 主要覆盖成功路径；未看到该空 key 错误路径的回归门。

**建议与验收。** 收敛一个空值安全、语义幂等的脱敏入口，明确文本长度上限与 UTF-8 截断。对空 key、非空 key、中文错误分别验证；用无真实凭证的 fake sidecar 发 failed，最终 receipt 应保留原始可读错误且不泄漏非空 fixture secret。此次最小修复可先仅修空值分支，通用 error 类型调整另批推进。

## B-04：超过 4 MiB 的 live delta 无有界追赶路径

**证据。** [wire.rs:32](/Users/zgh/Desktop/working/khor/crates/sync/src/wire.rs:32) 定义 4 MiB 上限；[wire.rs:137](/Users/zgh/Desktop/working/khor/crates/sync/src/wire.rs:137) 的 137–147 行一次计算完整 changes_since，超限即 Err，无分页 cursor。235–245 行 outgoing 与 313–329 行 answer 使用同一路径。answer 会先合并对方小 delta 再返回本方超限错误，这是已存在的正确保护，但不能让落后方获取本方大历史。版本未补齐时，下轮仍会算出同一超限差集。

**调用后果。** [link.rs:10398](/Users/zgh/Desktop/working/khor/crates/node/src/link.rs:10398) 的 10398–10423 行对必同步文档错误结束该轮；旧协议路径 [link.rs:10763](/Users/zgh/Desktop/working/khor/crates/node/src/link.rs:10763) 的 devices/seen 顺序同样通过 ? 传播。单文档落后超过上限会长期无法通过 live round 补齐，并可能阻断该轮后续必需同步。文件 road/离线 block-store 是另一条路径，本结论**不等于所有传输方式都无法恢复**。

**现有验证/缺口。** [wire.rs:23](/Users/zgh/Desktop/working/khor/crates/sync/src/wire.rs:23) 的注释记录过 256 KiB 上限导致同类故障，后来提升到 4 MiB。现有 [catchup.rs:42](/Users/zgh/Desktop/working/khor/crates/sync/tests/catchup.rs:42) 的 42–50、85–89 行直接调用 Doc changes_since/merge，绕过 wire 限制，且是 ignored measurement；不能证明真实 wire 对任意合法大缺口可恢复。本轮没有生成 >4 MiB 的自然工作负载，因此实际到达阈值的频度未知。

**建议与验收。** 独立设计有界分块或受控 snapshot/catchup fallback，并保留 epoch、flush-before-ack、版本幂等性和旧端兼容。不能把再次抬高常数当成完整恢复策略。用不可压缩的合法 CRDT 数据构造 >4 MiB 缺口，经真实 Peer outgoing/answer/absorb 及一次 transport 集成，验证有限大小帧重复运行后版本收敛；中断/重启后继续，不丢 writer op，也不跨 epoch 合并。

## B-05：普通 Stop 依赖子进程合作，没有统一退出截止

**证据。** Pi [khor_agent.rs:374](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs:374) 的 374–389 行只置 stopped、发 abort、取消子任务；591–603 行轮询 events 超时后继续，不基于 stopped 启动期限。收到 done 后的 692–694 行仍无界 child.wait/reader.join/drain.join；未收到 done/EOF 时也没有停止期限。executor claim 被 313 行的执行线程持有。

ACP 普通 GUI [gui_host.rs:1184](/Users/zgh/Desktop/working/khor/crates/node/src/gui_host.rs:1184) 的 1184–1203 行发 handle.cancel 后等待 Turn；[gui_host.rs:1068](/Users/zgh/Desktop/working/khor/crates/node/src/gui_host.rs:1068) 的 10 秒截止仅在 delegated.is_some() 时生效，普通 Stop 不设置 cancellation_sent。

**触发与置信边界。** 需要 sidecar/agent 不处理取消、取消后不结束 turn、或发 done 后不退出等异常合作条件。源码能确认缺少这个期限，尚未证实当前所有真实 provider 会遇到它。因此列为可疑机制，不写成已复现的生产挂死。

**影响。** Pi executor 锁和运行记录不释放，新 prompt 持续 busy；ACP 普通 GUI 保留 in_turn。有效取消确认尚未到来时，也不能只把状态置 cancelled 来掩盖仍在执行的写操作。

**现有验证/缺口。** [khor_agent.rs:905](/Users/zgh/Desktop/working/khor/crates/node/src/khor_agent.rs:905) 已测试 cancelled/stopped MCP 不能消费后到 approval，应该保留。[gui_stop.rs:53](/Users/zgh/Desktop/working/khor/crates/cli/tests/gui_stop.rs:53) 的 fake 收到 interrupt 后主动发结果，证明合作取消，不覆盖不合作进程。

**建议与验收。** 单独建立 cancellation requested → acknowledgement 或 deadline → owned process cleanup 的状态机；普通 GUI 与 delegation/Pi 共用产品语义，但先分别实现避免扩大一次变更。受控 fake 分别忽略 abort、保持 stdout、done 后不退出、派生持有管道子进程；验证停止时限、资源释放、下一轮可用及 receipt 不谎报远端操作已回滚。kill 权限限于自身创建并持有的进程/进程组。

## B-06：已认证 Web JSON body 的读入、解析没有明确边界

**证据。** [web/lib.rs:315](/Users/zgh/Desktop/working/khor/crates/web/src/lib.rs:315) 的 315–319 行无限 read_to_string，忽略读失败，再把 JSON parse 错误降成 Value::Null 并 dispatch。服务只有 [web/lib.rs:66](/Users/zgh/Desktop/working/khor/crates/web/src/lib.rs:66) 的 4 个 worker，118–124 行每线程同步处理一个请求。认证/Origin 在 238–248 行先于读取，是已存在的保护。

**触发/影响。** 持有有效 key 的错误客户端发送超大 body、慢 body、截断 JSON 或非法 UTF-8。已确认的问题是没有应用层 cap/解析拒绝；实际 tiny_http 传输层超时和并发内存影响仍需受控量化。无效 body 被当作 null 参数继续分发，错误反馈可能来自错误的业务层；本审计没有据此宣称必然执行了写操作。raw upload 有独立路径和限制，不应混为同一问题。

**建议与验收。** 为 JSON 请求设统一大小界限，保留必要的空 body/no-args 兼容，明确读失败/非空非法 JSON 的 400，超限 413，并检查读 deadline。用本地隔离 web fixture 发送超限、慢读、截断、非法 UTF-8；spy/no-op handler 验证非法输入不被分发，四个慢请求不让正常请求永久饥饿。GUI core dispatch 的业务语义由 Q-001A 负责，边界改动需对齐。

## B-07：once 锁表没有生命周期回收

**证据。** [once.rs:70](/Users/zgh/Desktop/working/khor/crates/node/src/once.rs:70) 的 70–82 行将每个 token 的 Arc<Mutex> 放入全局 OnceLock<HashMap>，guard drop 后 map 仍保留强引用。磁盘 receipt 在 48–67 行按年龄回收；内存锁无同类回收。

**触发/影响。** resident 长期处理互不相同的创建 token，表随历史 token 数量单调增长，即使请求都完成。每项成本较小，尚未测量真实工作量，所以为 P3；不是短期大内存故障结论。

**建议与验收。** 使用有竞争安全证明的弱引用/清理策略或请求所有者作用域；不要简单在 unlock 时 remove，避免 waiter 与新请求得到两把锁。并发相同 token 必须仍串行，N 个已完成且无 waiter 的 token 可回收。与 OpenAgentOnce“先创建再记录”的已有 crash window 分开评估，不用锁表修复宣称达到跨崩溃 exactly-once。

## 关键流程覆盖与已存在的保护

| 流程 | 本轮追踪范围 | 已观察到的保护 | 后续缺口 |
| --- | --- | --- | --- |
| Pi 会话/审批 | apps/agent host/session/mcp/gate 与 node khor_agent/mcp/delegation | Rust 持有工具授权；取消先于已排队审批；MCP progress；私有执行回执 | B-03/B-05；真实 provider 异常退出及子进程 FD 继承 |
| ACP | acp/lib 的请求/事件/permission；gui_host 的 turn/stop；agent ACP bridge | dropped permission responder 回 Cancelled；管理/turn 串行门；delegation 取消截止 | 普通 Stop 截止；initialize/load/switch 的故障矩阵未穷尽 |
| A2A | a2a store/executor、node delegation/a2a_http | caller/context 约束、持久 intent/receipt、任务终态保护、未知 send 不盲重放；固定 origin、不跟随重定向、超时/响应 cap | B-02；ledger 容量与跨版本恢复继续审查 |
| 网络重连/同步 | net carry、node wires、link 的 round，sync wire/store | 有界 carry queue；stable identity 防旧连接清掉新连接；dial 串行/backoff；flush 后再记录已接收版本 | B-04；NAT、relay、SSH/file-road 全链路只做结构巡检 |
| 文件/资源 | tree 校验与原子落地；once；选读 process owner | tree 禁止 traversal/绝对注入并限大小；目标无覆盖原子落地；收尾 drop events 防有界管道死等 | B-05/B-07；PTY/tmux/SSH 进程树退出未完整验证 |
| 身份/凭证 | identity/root、agent_accounts/subscriptions、Web key | 宽权限私钥拒绝；凭证由本地路径装载，不放工具参数；Web 先鉴权再读 body | B-01/B-06；平台 keychain 与 OAuth 生命周期仅结构级 |
| CLI/协议 | cli main、commands execution、core/proto 数据结构 | execution RAII 恢复 thread-local 范围；stdout/stderr cap；private_invocation 不同步秘密参数；msgpack 追加兼容策略 | commands 大分发、所有变体是否正确归类 read/write 需系统化矩阵 |

## 可独立推进的修复与重构批次

这些是候选批次，不代表本审计替 root 批准了范围或实施。先修确认问题，再做结构变化，避免把多个协议语义同时迁移。

| 批次 | 范围/目标 | 与其他批次的关系 | 完成证据 |
| --- | --- | --- | --- |
| R-B1 identity 原子发布 | net/identity，必要时 net/root 的小型私有文件发布原语 | 可独立；平台 fallback 必须明确 | B-01 并发首建、首次权限、赢家重读测试 |
| R-B2 A2A context 所有权 | node/a2a_http 的 ledger/query 与 fixture | 可独立；不改 A2A Task 状态机 | B-02 Message context 首次/重试/续聊/重启与隔离测试 |
| R-B3 agent 错误脱敏 | khor_agent 与 khor_agent_updates 共享空值安全函数 | 最小修复可独立；避免同时拆整个 executor | B-03 错误可读性、fixture secret 不泄漏 |
| R-B4 bounded cancellation | Pi executor / GUI ACP host 各自生命周期 owner | 与 R-B3 有 khor_agent 文件冲突，串行合入；不同运行时先分开实现 | B-05 不合作进程与下一轮可用测试；真实停止语义说明 |
| R-B5 sync 恢复能力 | sync/wire + node/link 的能力协商与 fallback | 需设计、协议兼容评审，不能夹入纯文件拆分 | 超 cap、断线续传、epoch、持久 flush、旧端兼容 |
| R-B6 Web JSON 接入 | web/lib body/parse/timeout 边界 | 可独立；与 Q-001A 对齐空参数契约 | B-06 HTTP fixture；非法输入不 dispatch |
| R-B7 once token owner | node/once 局部回收 | 可独立，小收益后排 | B-07 同 token 串行与历史 token 回收 |
| R-B8 node 分发边界抽取 | link 的协议授权/dispatch 与 sync rounds 分离；commands 的命令解析/执行分离 | 在语义修复稳定后，分两个串行 PR；不以行数为拆分理由 | 请求变体 read/write/early 策略保持，现有 wire/CLI 兼容测试通过 |
| R-B9 运行时资源归属 | host/live/gui_host 的 process handle、stdio、subscription 生命周期逐个收敛 | 单独调查；不要一次统一所有 vendor shim | owner/drop/终态清单、真实进程树退出和重连不失 session |

R-B8/R-B9 的价值来自同一模块同时拥有协议、持久化、进程和 UI 状态转换造成的变更交叉；不是“文件大就拆”。当前 link 12,224 行、live 5,522 行、node/lib 5,984 行、commands/lib 6,317 行只是定位信号。先建立稳定领域边界和依赖方向，再机械移动；不得借重构改变 wire 形状、secret 落地或自动重放策略。

## 交接与下一轮验证

首轮报告、模块清单与源码引用已完成，等待 root 将候选映射到治理任务及修复提交。本代理没有写 STATE/PLAN。建议 root 首先独立复核 B-01～B-03，并把确认修复的失败门与通过记录放到对应实施任务；B-04～B-06 先用隔离 fixture 定义行为，再决定协议/产品约束。模块清单中的 S/G/T 状态保留为下一轮审查债，不能在首轮结束时自动升级为 K 或“已安全”。
