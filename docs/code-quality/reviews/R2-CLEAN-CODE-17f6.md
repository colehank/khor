# 第二轮最终集成 Clean Code 独立审查

2026-10-31，reviewer：round2_reviewer。审查对象为 `/Volumes/disk0/Khor/worktrees/current/khor-round2`，`17f6afb08a6ef207663cabd106012d23424fc434`。开始时实际工作树 clean。本报告不修改中央 STATE、34 项验收标准或产品。

## 结论与方法

本次集成检查未发现新的生产源码准入阻断；这不等于 34 项完成。下列具体未解决问题及平台未知仍保留。未重新执行任何测试、构建、生成、OS probe、浏览器或 App；没有替换旧失败结果。

先调用知识图谱取得结构上下文；图谱实际构建在 main `e7c335c8`，不覆盖当前集成，故随后只读当前固定源码和已有逐片审查。main→17f6 有 411 个变更路径，包含 vendored SDK、测试与文档；本次不是声称重新逐行审过全部路径。已闭合领域复用任务卡与 R2-REVIEW 的逐片结论，对当前组合的状态 owner、IO 边界、身份、取消清理、错误、重复及注释再核实际接缝。所有源码位置以下均相对于上述固定 worktree。

## 34 项到领域、原则和现有证据的映射

| 既有项目 | 当前源码与具体原则结论 | 复用的独审/实际证据与剩余边界 |
| --- | --- | --- |
| R2-001 | D 调查证据与产品状态分开；不能从资源门或其它通过推断旧卡死已修。 | `tasks/R2-D.md`、原调查记录；原现场和真实 GUI/WK 根因仍未知。 |
| R2-002、003、005、007 | `crates/node/src/live.rs:3369` 的 GUI 状态从同一 Meta 快照读取；`session_recovery.rs:103/269/312` 将可读身份、offer、显式执行分开；`live/recovery_attempt.rs` 独持 bounded attempt/action 高水位；`session_content/journal.rs:187` 独持物理 File reservation。当前 published observation 的控制资格仍经 `live.rs:1129`，不是只凭 observe 前缀、文件或 Meta.exit。 | S/A 各逐片审查；Node85431、原 R1、67MiB Store、Recovery17、actual70988 的有限公开 Join/Load 业务证据。未据此宣称远端、Resume/Shell 真实公共入口、App 或所有 OS 身份全 Known。 |
| R2-004 | `doc_transfer_state/io.rs` 的 bootstrap/joint admission 复用实际余额，持 metadata 时不等待 root；release/recheck 属同 owner，未引入整个 Node 重试或重复副作用。 | `tasks/R2-N.md` 的95acd及实际 current CLI 两个 FD-fenced positive、历史 false 保留；不是原集群唯一根因证明。 |
| R2-006 | `history-source.ts:13` 管 lease/取消，`history-reader.ts:18` 管单请求与跟读；`history-paint.ts:32/81` 唯一 fold/真实贡献映射，`history-find-stream.ts` 复用该 step，不另造 parser。原 record/range 与正文贡献分开，raw gap/未知端点不冒 complete；deferred replay 不重计 coverage。 | R2-H/R2-REVIEW 的来源、分组、相位、whole-copy、ceiling、Range逐片审查；Reader28、Find32 actual worker、fullH28700 原7+Range全过。WK、OS Save/Clipboard、当前 App 仍未由 Chrome 证据覆盖。 |
| R2-008、009、010 | `content-link-intent.ts:3/20` 只决定用户意图，`term-links.ts` 保 xterm cell/软折行坐标，`client-opener.ts` 持客户端 IO；不以网络预检改写意图。回工作区使用当前 navigation/root，不携带旧 cwd 或清草稿 owner。 | `tasks/R2-C.md` 各独审、真实 xterm CJK 边界反例与 DOM 定向证据。系统 opener、窄屏与当前 App 组合尚待。 |
| R2-011、012、027、L03 | `file-refresh.ts:3` 把 composition/edit/read revision 接纳放在纯协调器；`use-field-dismiss.ts:5` 同一区域含 portalled candidates，订阅/定时器由同 effect清理；`use-draft-count.ts:5/30` 以私有 Symbol 和同步 accepted owner报告脏状态，Quit不靠迟到 render。FileBody、Workspace及其它入口不复制正文状态。 | `tasks/R2-E.md`、W/E逐片审查与失败保存门；真实系统IME、三入口拒写/Cancel/last-key normalQuit尚待。standalone probe和旧 A5 partial不替代它们。 |
| R2-013、022、024 | `lib/scroll-edge.ts:24/97` 用 WeakMap 区分 DOM fade owner，清理由本 owner移除属性；`styles/app.css`、`styles/session-content.css` 和 sticky group共用呈现规则，不把材质当业务状态。 | `tasks/R2-G.md`/W共享卡的独审及 sticky/材质定向证据；深浅/三材质的当前 App几何仍待。未据源码复核改任何视觉规则。 |
| R2-014、015、016、017、018、L02 | `src-tauri/src/page_navigation.rs:16/70` 由 view/navigation identity发事实；`browse/loading-rule.ts:68` 按 issued revision/view/navigation接纳，终态不复活，失败不伪100；`use-loading.ts` 的呈现 timer不制造 native成功。phone/page owner与消息路由仍分开，非另HTTP200判WK成功。 | `tasks/R2-B.md` 与L02旧真实两case独审；当前 WK首帧/错误/手机viewport/手势及原网址现证仍待。旧666 L02成功不是 current全App验收。 |
| R2-019、020、021、023 | `workspace-tabs.ts:14/35` 以 kind+path/commit ID保 preview/pinned身份；`workspace-memory.ts` 持真实文档/阅读状态，`workspace-refresh.ts:5` 仅一个在途读、旧consumer结果不落地；`directory-watch.ts:24/101` lease唯一、晚到begin释放自身、scope预算显式回退。 | `tasks/R2-W.md`、W-SHARED/E逐片审查及watch分层/当前受影响Core表检查；真实连续导航、长列表/外部变化+IME组合仍待。 |
| R2-025、026、028 | `deck-joystick.ts:26` 显式 phase/clock owner，release复用cancel；`term-key-model.ts:41/76` 有实际 UTF8/条数预算，`term-key-codec.ts` 把可配置动作落到既有 terminal编码入口，配置解析不直接发字节。试按隔离不生成第二真实输入 owner。 | `tasks/R2-M.md`/M025 的独审与真实隔离PTY/键盘路径证据；真手机连续手感、软键盘入口仍待，未声称 vendor有选区能力。 |
| R2-029 | `lib/md/blocks.ts:177` 在既有 lexer保护区外处理括号公式；`Markdown.tsx:674` 普通/流式都复用该步骤，不引第二math renderer或处理代码块。 | `tasks/R2-029.md` 原反例及独审/type证据；当前 App 文件预览尚待。 |
| R2-L01、L05 | retry deadline、正常App Quit与独立宿主的意义各自保留；actual70988 的 planned provider TERM与bridge teardown不冒 App normalQuit。 | R2-S/V与原L01/L05独审；旧focused实际不等于完整S；current App 尚未执行。 |
| R2-L04 | `scripts/native_provenance.py` 为唯一来源门，producer执行与runtime有效环境分离；生成/包选中/实际CargoJSON各自绑定，无手写TS产物或伪prior。 | actual28762六生成阶段、48300 fulltypes/Vite、Core44421、C59004、production9014逐项独审，原false全保。actual9014三canonical产物供紧邻runtime；不是业务或真机通过。 |

## 本次再核的组合接缝

1. **状态 owner 与权限。** `live.rs:3369–3389` 明确 GUI slot结束不代表 provider已退出；`recovery_attempt.rs:20/526` 用16KiB文档及借用 RowGate 的 Held隔离权限。`session_recovery.rs:129–150` 在物理 reservation 内核 full binding/observed资格/有限alias，再同gate重核 expected_run；无把路径能读、read-only offer或旧状态当可 spawn authority。`live.rs:1111` 删除 readonly Ref与相同物理 reservation/既有alias短gate同生命周期，避免已审删除 TOCTOU 回来。

2. **阻塞 IO 与临界区。** `session_recovery.rs:336–418` 的 endpoint/ghost连接在 blocking工作与gate外；Join先 durable Pending，握手后同ticket/HostFile/Meta再核，握手失败的重放保持Unknown，不转fresh Native。`new_shell_run_here:447–505` 先记录 Pending再调用原 Shell owner，empty command/defaultshell、previous/root新SID关系清楚；任何不明 Ready/响应不伪Refused或再次spawn。通用close不属于该新恢复协议，见下方未解决边界。

3. **UI投递与执行状态不混。** `use-session-recovery.ts:59–65` 的 offer可按row revision失效；`:67–115` 已提交action只由target+mount generation退休，不因本次动作自己改 ended/idle而丢答。`session-recovery-intent.ts:68–129` 用同一短 IDB readwrite transaction保存32个target以内、每项8KiB exact payload，事务完成后才RPC，不持网络锁、不在后台重试；清除只比完整payload。Node仍独持执行代次。`DetailPane.tsx:339/361/373/720` 以id/home/generation退休单座位，`TerminalPane.tsx:2355/3049` 区分新用户输入与已接受IO/协议reply，不触发global Stop。

4. **取消和清理有确定归属。** history source的cancel按原lease去重、late begin释放自身；reader/window/worker各有有界在途请求和Abort源，不用取消语义伪IO已结束。真实 disclosure、hidden/zero-height、Range新输入票据与cap贡献点已逐片及实际核过。ACP budget以真实bytes/records/interaction credit计费、Drop减账后唤醒；EventStore的单IOjob/单readresult与read-prefix fence区分accept、写入和consumer已读，不把ACK当消费完成。

5. **错误与重复代码。** typed Refused只用于可证pre-mutation参数/CAS，Unknown/Pending保不明副作用；FS、store、handshake错误不按字符串转确定拒绝。C的两个integration测试各持相同小prefix-pump helper，是不同测试二进制的独立receiver owner，不为DRY扩产品框架；GUI payload decoder是本地存储边界，不能替代Node token验证。原source6保留raw PromptOrigin和public SendCommitted两个事实，不合并成一个“成功”布尔。

6. **注释与模块成本。** private `session_recovery.rs`、`live/recovery_attempt.rs`、`live/proc_identity.rs`、journal budget/IO叶已有明确边界，比在UI或API里再次实现owner好。live/gui_host/TerminalPane仍大；这是可维护性成本，不能仅凭行数要求此轮再拆服务、加registry或改壳。关键注释说明原子范围、失效/Unknown与真正事实来源，和当前代码相符；一个已实证例外为ready诊断的生命周期，下节列明。

## 未解决发现与边界

### F1：ready marker消失被误当成“从未Ready”——已实证诊断残留

`crates/node/src/gui_host.rs:750–753` 成功确认后parent删除ready；`:1765–1767` ghost终结又以 `!ready.exists()` 写 `.why`，因此成功publish过的run在退出时仍可留下“未Ready”诊断。actual70988两个已结算ghost实留两份49B自有文件；这不是Load失败或进程仍活，却使原TMP门无法变绿。注释“why it will never be ready”与该路径实际生命周期不完全一致。

本轮没有改产品掩盖它。原result `3d457f8640646cf737bb32c999246a4dd7ccb398aeb2f298f4a37a35bb1c3893` 仍false；Root后来只retain两exact文件，postrun receipt `31a4c0209c963417574cde4d2bd738178ac7ac03d206b5c7e26b5307f3d9a9cd`，两inode/SHA未变、TMP现absent。可以分列“有限公开Load业务已完成”和“事后清理闭合”，不能改报原terminal0/strict全绿。是否修此产品生命周期须Root按原scope另定；此审查不授权新增实现或重跑。

### F2：通用close仍是既定NEXT，不能由新恢复owner证明其安全性

`live.rs:1589–1620` 的通用close仍使用旧HostFile PID/group、等待/host兜底并删row；`Node::close:4553` 仍走各kind原关闭路径。它没有消费新Attempt的full birth/run authority，不能把新恢复的ProviderExited/Unknown保证外推给所有close入口。该边界在2f/公开接线handoff已明确留NEXT，本审查未新增功能要求，也未运行它或以新Load强行为其背书。

### 尚待验证，不冒代码缺陷或平台通过

现有Node组件、Core、C和受控Chrome证明各自闭包；actual70988额外证明一个本地custompeer的公开Join/Load/同token replay/新旧canonical读取与两次真实登记foreground退出。尚不能证明真实模型、公开Resume、Shell实际创建、远机/离线、hostdeath/physical reboot、所有OS分支、当前完整Mac App/WK/IME/系统Copy/Save/Quit/真手机。34项中的平台条目必须按原总账逐项观察，旧D/L02/L05的局部证据不能替代。

## 处置与证据索引

无需为本报告重跑已绿Reader/Find/H/ACP/Node/Core/C/types或再做来源框架。新源码阻断为零；F1已单列实际后果，F2保原NEXT，其余为未完成验收边界。所有后续实现/运行仍由Root依原34项与当前合同决定。

- 原逐片审查与历史反例：[R2-REVIEW](../tasks/R2-REVIEW.md)，各领域任务卡在同目录；原验收要求：[ROUND-2-ACCEPTANCE](../ROUND-2-ACCEPTANCE.md)。
- 实际当前证据均在 `/Volumes/disk0/Khor/evidence/round2/history/session-recovery/`：`public-wiring-native-20261031-3/actual-node-1`、`core-current-native-20261031-1/actual-core-1`、`cli-current-native-20261031-3/actual-cli-1`、`production-current-native-20261031-1`、`recovery-public-ui-20261031-1`、`public-load-runtime-source-6/actual-load-1`。
- 当前生成/fulltypes分列于 `evidence/round2/integration/recovery-cohort-20261031-2` 与 `recovery-frontend-continuation-20261031-1`；前者overall false不能因六成功阶段被改绿。
- 审查只产出本小报告与review任务卡，无新测试、heavy执行、平台交互、source改写、二进制/大证据镜像或main合并。
