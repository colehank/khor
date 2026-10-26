# Q037 MCP worker：接口与实施边界

状态：16源已提交 `3e676325daab05d513133c859e295393cd54ad0e`（fix/quality-mcp-cancel，tracked clean），同源CLI构建、106个不同普通父测试及Windows独立harness类型检查通过；尚未集成main/发布。接管者 mcp_worker_owner，2026-10-26。

## 当前交付状态（优先于下方历史提案与冻结点）

- 实测源仍是 `candidate-20261026T145036Z` 的16文件，完整前后无delta；khor_agent88036f8e、worker b3aef631、mcp eb8ec6c4、worker tests3010c845。全部源码与测试名称/命令/PGID/资源/制品SHA见 [delivery-verification.json](/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/mcp-worker/delivery-verification.json)。中央卡在plan树；实现提交与逐文件对照见 [commit-source-receipt.json](/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/mcp-worker/commit-source-receipt.json)。
- 首次build-01仅因本树缺apps/gui/dist被web build.rs拒绝，68.411s/exit101，是setup失败，不是业务红。root另登记Q041d，不在本片改构建入口。14个第一方entry/build.rs仅更新mtime且SHA相同；后续gen入口3源同样核mtime-only。
- root批准复用现有locked GUI cache，以本树新建ignored node_modules symlink；当前Darwin arm64 version/integrity/link匹配，81个other-platform optional省略，local no-elk override逐文件字节匹配。正常 `npm --offline --prefix apps/gui run build` 实际执行gen_ts+gen_bindings、tsc、Vite，36.646s/exit0，1615输入稳定；155生成文件、277 dist/15,141,838B及producer SHA逐项记录，entry `/assets/index-DIGiSBlA.js`。没有借旧dist/旧ship binary，也没有安装依赖。
- build-02真实CLI+stub构建14.388s/exit0；normal Node libcheck18.009s/exit0；Node lib test编译62.655s/exit0，日志证明当前树实际重编。共同输入扩展为2047（含所有tracked GUI、generated、dist、依赖lock）；Windows类型检查另加独立harness Cargo.lock为2048。全部在各轮前后稳定；原16源逻辑不变。

| 同源行为组 | 不同普通父测试通过数 | 实际日志目录（Q037/mcp-worker下） |
| --- | ---: | --- |
| 原真实IPC取消、held gate下一Prompt、16Stop单sweep、late reply隔离 | 1 | ipc-cancel-green-01 |
| worker实际CLI/锁/授权/父身份/输入与encoded预算/SSH broker/durable host/实际ACP Answer取消窗 | 9 | worker-behavior-01 |
| session批准顺序、generation、no-id/null、原批准撤销 | 4 | session-behavior-01 |
| 默认shared process与Control/receipt/Darwin生命周期 | 15 | process-regression-01 |
| 默认ACP process/I/O | 12 | acp-regression-01 |
| 默认Pi/account/output/lifecycle（跳过前两组已验项） | 28 | pi-regression-01 |
| host/fanout/upload | 25 | host-regression-01 |
| finite SSH owner/process | 12 | ssh-process-regression-01 |
| **合计，按名称去重** | **106** | 完整106个名称在delivery-verification.json |

原IPC首红与新绿使用同一13s oracle；新实测实际Handoff/Get barrier后Pi35.786ms ESRCH，原IPC reply gate继续扣住直到下一Prompt done且新Pi ESRCH，之后才release/join。专用worker与环境driver内部同名子test、ignored helpers不另计。shared unwind测试的预期catch_unwind panic原样保留，最终测试ok，未过滤错误。

最后实际制品：CLI `98b869893f34299188cd5442b3f673e6d88b81f05abe14980ca78596f1dd2620`（257,287,672B）；ACP stub `22d438cb2dadb502c88f7393a50de1a58a8ab743cc2458b4bcac68bdd0d06b79`（15,578,504B）；Node test `5edbb33cf7ae74a7909bd650ffd02e89a88d967ca0427721e7d5db233b375870`（229,015,368B）。路径和mtime在回执，每组均核实际同SHA。

Windows已装std下 `cargo check --locked --offline -j1 --manifest-path crates/node/tests/windows-process-harness/Cargo.toml --target x86_64-pc-windows-msvc` 2.996s/exit0；只覆盖共享Session/profile/Windows平台与独立harness类型，**没有Windows runtime、没有full Node Windows跨编**。CI仍按harness实际exit判断，不写死旧组数。

所有运行shared target、CARGO_INCREMENTAL=0、locked/offline；root临时降jobs1。最低内盘7,110,774,784B/外盘6,922,944,512B；没有容量/墙钟/I/O guard触发。所有登记runner/PGID末查ESRCH，候选CLI/stub/test及自有sidecar无残留。独占TMPDIR的Node compile cache及本轮pi账户fixture在保留264文件完整metadata/SHA并确认lsof无open handle后，仅删除自有3,364,028B；scratch现空，清理manifest/receipt已存。没有删用户source/App/服务/其他cache。

唯一Cargo/native槽已交还root并转transfer_init_owner；作者后续仅交付/只读校核/获授权提交。该片只收口嵌入Pi MCP的本地owner与有限sweep：远端仍pending/not-confirmed，不保证所有cancel送达/公平，不等于remote stopped；外部stdio MCP、可查询远端receipt/lease等仍开放。批准持续busy时的既有延迟保留。top-level sweep线程句柄为最多4个进程寿命owner，不宣称全部后台线程独立join。main/ship无授权。

提交父为`7550e4f05f82cf410b2cc25e8a446b393cfa5f76`（1d264之上的首红fixture重放）。集成须保留该测试准备依赖，不能只遗漏首红test去取生产改动。当前Git commit、worktree、tested freeze的16源SHA逐文件完全一致；Git实际只提交授权16路径。pre-commit自动CodeReviewGraph索引报告的17 changed/0 flow/77 gap不是运行覆盖结论，本卡只按实际106父与源码指纹记证据。原真实首红对象/分支`refs/heads/audit/mcp-cancel-red-20261026`仍指25478cc7，未改。

最终外盘交付目录 `Q037/mcp-worker/delivery-20261026T152714Z`，内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-MCP/snapshot-worker-delivery-20261026T152714Z`；保存16源码、2047/2048完整manifest、原始logs/producer/cleanup/PID回执及提交patch/commit逐字receipt。提交没有重跑测试、更改源码、再次占用Cargo槽或修改main/ship。

## 当前实施检查点（优先于下方提案）

root批准原allowlist及对应tests，选sweep B。生产已实现hidden hook/one-shot worker、Running同锁注册、McpSession唯一异步操作、直接terminate/reap/三IO join、Windows typed worker profile及两durable spawn。CLI/GUI共享hook已实证转发，无入口额外改动；没有依赖/锁/DTO/前端修改。对应测试额外接线为output_tests的Running新增字段与receipt_tests的cfgtest PID字段。

worker输入2MiB，tool text64MiB、总encoded输出128MiB+64KiB（先完整编码预算校验，超限只发完整Err）、stderr256KiB私有尾部不回显；不clone完整response进入外层MCP帧。取消在结果解码前复核Control，丢弃已捕获结果。原Pi/ACP默认10s grace不变；worker使用显式terminate_now。执行器将已解析PATH交给worker，owner不自己运行login shell。

本片sweep只在execute已退役所有本地MCP worker后登记一次，Stop不建线程，因此16次Stop仍只生成一个sweep，late track_child不落在sweep观察之后。全局最多4个保留JoinHandle的sweep owner，每个5s总预算；本地主线程不等待。复用原cancel_children，仍可能慢child阻后续，**不保证全部remote cancel送达，也不确认remote停止**。既有CommandRecord.data_json记录remoteCancellation {state:pending,confirmed:false,sweep:scheduled|not_admitted,reason?}；容量不足/超时/失败时仍pending，durable links不删。公平/可查receipt另卡。

sweep顶层线程句柄在后续登记时join已完成者；没有后续登记时保留最多4个直至进程退出，不能称所有后台线程都joined。内部Worker自身必须确认process reap与三个I/O join。Unix父进程退出不会自动杀独立PGID，故真实child内另有操作保险fuse（Tool300s、Sweep5s），仅当自己仍是PGID leader才kill自己的group；直接shell调用绝不kill调用者group。Windows父进程退出由kill-on-close job退役contained子树。fuse线程是child进程寿命资源，normal进程退出终止它，不声称单独join。

候选验证包括：原真实IPC红gate直到下一Prompt完成才放开、16Stop实际sweep创建计数与late-release receipt；真实A2A ledger.lock，以产品task-gate持锁作barrier，取消后真实worker PID ESRCH/IO join且ledger仍扣住时下一read完成；同源CLI合法read/写批准/parent身份、单JSON与输入超限、总encoded输出超限完整错误、pre-cancelled spawn、旧cancel不影响新request、真实_host存活、真实delegate+ACP stub的_ghost存活与parent身份。Windows harness新增default不能breakaway而MCP可显式durable breakaway、temporary child仍退役；未Windows实跑。

真实CLI测试显式ignored并要求KHOR_TEST_MCP_EXECUTABLE为本轮新构建的绝对路径；ghost测试另需同轮KHOR_TEST_ACP_STUB。原首红父测试仍正常test，现要求确切CLI路径，无libtest argv/噪声替身。接下来必须明确运行ignored tests，不能把其跳过计绿。

已做新文件与修改叶rustfmt解析、git diff --check；**没有Cargo、Python fixture、native App/服务运行，尚无候选绿**。下一步freeze SHA/内盘snapshot→root独立源审→申请唯一Cargo槽，实际运行再记结果。

冻结：`Q037/mcp-worker/candidate-20261026T140735Z/source-freeze.json`，15候选源/842构建输入。mcp_worker85219cc1、khor_agent537a687a、worker tests cbb6510d；内盘镜像 `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-MCP/snapshot-worker-candidate-20261026T140735Z`。源与tracked.patch已存，未Cargo，已向root申请独立源审与唯一槽。冻结后保持源码不动。

最新冻结优先：`Q037/mcp-worker/candidate-20261026T141449Z/source-freeze.json`，同15源/842输入，内盘同名snapshot。只读自审发现父bridge再包request id/envelope后仍可能超encoded预算，已告知root并在3源补同cap有界编码（含newline）与明确tool_error fallback，增加大id+近cap正文case。最新mcp_worker495011b9、khor_agent3956c66d、tests4764595e；140735旧快照保留。仍未Cargo，rustfmt与diff-check过。

## root 首轮源审后的修正（待重新冻结，尚未运行）

- root授权额外 `mcp.rs` 最小改动：task_answer在ask解析通过后、构造/发送Action::Answer之前，按已知task ID写 `track_child(..., message="task:<id>", task=Some(id))`；保留原post-response实际ID补充，批准/parent身份不变。delegate原先已有pre-track，不能拿它现有link掩盖task_answer缺口。
- 新真实窗口测试先用候选CLI delegate启动真实acp-stub/实际_ghost并等真实permission；随后切到另一个parent run发送task_answer，实际Handoff→Node.delegation_here→实际ACP permission回答确认生效后，仅扣住IPC reply。此时取消/reap worker，新parent已知task durable link必须存在且不被删除，不能等post-response才track。fixture持reply gate与thread，失败Drop释放/join；actual host由独立guard Close。此轮准备源，不冒充已执行红/绿。
- 源审实际env消费者后，补SSH_AUTH_SOCK（OS broker地址）、lowercase代理/SHELL/COMSPEC、endpoint的KHOR_RELAY/RELAY_ONLY/SUCCESSOR/CA/PUBLIC_RELAYS/BIND_IP/NO_MDNS/NETLAB与carry SPARE_ROAD；常量处直接引用第一方常量。保留SSH/secret的显式隔离目录KHOR_SSH_DIR/KHOR_SECRET_DIR，使测试不落用户真实SSH config/Keychain。继承provider API key/SSH_ASKPASS/KHOR_ASKPASS不传；Dest自行按明确password road生成合法askpass仍沿原语义。
- 新环境测试以独立libtest子进程设置synthetic父环境，不改共享进程env；真实worker→真实durable_host执行/usr/bin/ssh-add，实际连接自有Unix agent socket并交换identities request/empty reply；实际shell检查broker路径/连接policy/隔离路径保留、provider/继承askpass变量不存在，只写ok marker不回显值。Node只用私有root与loopback relay设置，监听器/host/子进程均有自有guard。
- 核完整khor_agent/updates/output：该run创建data_json默认None，原链没有其他data_json写入；但直接替换会妨碍后续元数据，故改object merge，非object/损坏原值保留于previousData。既有真实failed-sidecar fixture注入existingFixtureMetadata并断言failed收口后仍保留，remoteCancellation仍明确pending/confirmed=false；无DTO/UI扩展。
- 准备验证数更新：mcp_worker::tests共9个不同测试（常规encoded1、需CLI ignored8，含真实answer窗口与环境broker）；环境driver内部同名子test不另计。另generation1、原IPC父1与既有process/output相关回归。仍未Cargo，旧两快照不覆盖。

修正后最新冻结：`Q037/mcp-worker/candidate-20261026T142944Z/source-freeze.json`，16候选源/842构建输入，内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-MCP/snapshot-worker-candidate-20261026T142944Z`。关键SHA：worker b3aef631、khor_agent60238742、mcp eb8ec6c4、tests3010c845、output tests b37153d9。142811为同候选补endpoint0600权限前快照，完整保留；140735/141449也保留。新叶rustfmt解析与diff-check过，尚未Cargo/fixture运行；已准备交root新鲜源审与槽调度。

## 普通notification保行为覆盖（不是业务红/修复）

root最初从diff未显示的上下文推断missing-id工具通知会新起worker；接管者核当前message完整源码、142944快照及7550基线，`request.get("id").is_none()` guard一直存在于cancelled处理之后、审批/start之前。root已明确撤回该回归判断；生产guard原样保留，不虚构业务红或修复。

新增1个常规McpSession测试，实际依次发送无id devices read、无id tell write及initialized通知，逐个核无operation、无Control注册、无pending/approval、Saved逐字不变；本地真实wide log数量不变且无tell正文。另显式id:null ping仍得到响应，用以区分present null与missing。该测试不依赖CLI、不调用模型，未执行；新叶rustfmt/diff-check通过。准备重新冻结后交root审查/槽，旧snapshot不覆盖。

此轮最新冻结：`Q037/mcp-worker/candidate-20261026T143713Z/source-freeze.json`，16源/842输入；内盘同名snapshot。仅khor_agent测试增加令SHA变为`6ad73afd89ac73576f448f18f5008398d15c9047a616f66ed3135c6652c116e3`；其他15源与142944相同。未Cargo，准备交root源审/槽。

## 独立源审后的批准顺序修正

doc_apply_review发现本候选每event后的full poll会在queued update→cancel之间，先消费UI已给的allow=true并启动写。作者与root确认静态时序成立，未编译/运行该帧，不称旧产品运行红。root批准窄修：抽出仅接收Running的poll_worker；每event只推进已运行worker/Stop/完成，idle timeout才执行原approval消费与heartbeat。持续busy时批准可能延迟的既有边界保留，本片不改变公平性。

新增常规真实McpSession case：私有Node/claim/Running，通过实际Action::Answer提交UI allow=true；真实有界事件channel已排合法ACP update→cancelled。按execute实际顺序调用worker-only after-event入口，核中间无operation/Control注册；再处理cancel及idle poll，核无approval、无write副作用（实际wide log无tell）。这是未发布候选的回归预防，不伪称旧产品红。

root曾授唯一Cargo槽，但fresh内盘4,885,816KiB低于5GiB，作者没有启动任何Cargo/更新mtime；root正在按归属manifest清自己的closed incremental，明确清理中不启动。修改完成需新freeze与root delta复核，再fresh双盘过门后开始。旧所有snapshot保留。

当前最新冻结为 `Q037/mcp-worker/candidate-20261026T145036Z/source-freeze.json`，16源/842输入；内盘同名snapshot。khor_agent SHA `88036f8ec2eae3c078fd908015bcf60d9f11ca9a5371e164d2c637b19b874a3c`，其他15源与143713相同。格式/diff-check通过，尚未Cargo。

## 来源与当前现场

唯一写树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，branch `fix/quality-mcp-cancel`，HEAD `7550e4f05f82cf410b2cc25e8a446b393cfa5f76`，tracked clean。已读中央 STATE/PLAN、MCP audit/cancel/red-review 卡。原首红25478cc7及证据保持，重放提交7550不是新组合运行结果。

按 explore-codebase skill 图优先尝试 semantic search；工具不收 repo_path，重试返回0节点且明确索引 main8704，不覆盖本树7550，故回退当前源码。当前树没有 `.codegraph/`，不自行索引。未把0节点作为无风险证明。

## 已核入口与最小接口

CLI `crates/cli/src/main.rs` 先调用 `khor_commands::console_main_if_requested`。GUI `apps/gui/src-tauri/src/lib.rs:4475` 先调用 `khor_gui_core::console::main_if_requested`（须核其转发）。该 hook 在 cwd pin、lang、host、menu、插件之前；桌面 `main.rs` 的 askpass 在它之前。worker command env_clear 不传 askpass key，因此不会走密码路径。拟在共享 commands hook 最前增加 `khor_node::mcp_worker::main_if_requested() -> Option<i32>`，在 shim 检测和用户配置读取前识别唯一隐藏 argv `_mcp_worker`。无 `mcp --once`，不扩公开命令。

新增 Node `mcp_worker` 模块，公开且只公开上述 hook，内部 `Worker::spawn`、`poll`、`cancel_and_join`。单次 stdin 有界 JSON 至 EOF、单次 stdout JSON Result，stdout 不混日志，stderr 只保留有界尾部。协议不做旧版本分支。

Tool 输入包含绝对 root、tool name/args、显式调用上下文：Embedded { run, authorized }。worker 自己 Node::open、核配置 home 为本 device、核 reads/写批准、构造 TaskRef { home: node.device(), id: run }，随后调用原 `mcp::call_with_parent`。不能在父执行线程保留可能阻塞的 Node::open/config/业务锁。外部 MCP 后续可复用 External 上下文（不凭空添加 embedded 配置限制），本次是否接 stdio 由 root 定范围。

`agent_process` 增 `SpawnProfile::{Contained, McpWorker}` 与 `Session::spawn_with_profile`；既有 `spawn` 保持 Contained。Unix profile不改变现有自有PGID；Windows仅 McpWorker job 加 BREAKAWAY_OK。`Control::terminate_now` 明确零 grace、finish=true，不发 Pi JSON abort。`host::spawn_host_with`、`gui_host::spawn_ghost_context` 只有这两个已确认 durable spawn 在 Windows 加 CREATE_BREAKAWAY_FROM_JOB；其余 Pi/ACP/temporary child 仍被原 job 收口。

## Owner 生命周期

Running 增当前 worker Control 槽；先创建 Control，在同一槽锁内检查 stopped 并注册，再 OS spawn。Stop 先置 stopped，再在槽锁取当前 control 请求 terminate_now；即使 Stop 在 spawn 之前到达，后续 supervisor 也会立刻退役这个 child。退出只清除自己注册的 Arc identity，不误清后续代次。

McpSession 持唯一 active operation { exact request id, Worker }；审批仍由父记录并保存，只有通过后 spawn，读操作直接 spawn。主 execute loop 不等待同步业务；每个 event/timeout 后 poll，持续处理 cancellation/EOF/Stop。取消仅匹配 exact id，先 terminate/reap/join stdin/stdout/stderr owners，之后才丢该结果或回应取消。完成只有 exact owner 仍活动时可回送；不把晚结果交给新请求。Session Drop 兜底 force+join，不丢线程。

Worker 复用 Session 的平台叶，3个有界一次结果通道承接 stdin、stdout、stderr；不是无限字节队列。自然 process-reap receipt 后仍 drain 缓冲、收齐 I/O，再 finish/join；取消/输出越界立即 terminate_now。所有 I/O 线程退出先由 Session cancel_io 解阻，然后 join，成功必须是 reap+join 都完成。

## 预算提议（需 root 确认）

- stdin 2MiB，沿既有 Pi EVENT_LIMIT；先序列化并核长，再 spawn，读端 limit+1 拒绝过长/尾随第二对象。
- tool text 64MiB、stdout JSON 128MiB+64KiB；理由是 task 单个4MiB、task_list一次50且本地store64MiB，不能沿用SSH1MiB。stdout是 String 的 JSON 包裹，canonical task JSON再包裹可近2倍；非canonical plain tool若越界明确报错，不截断成功结果。该上限是明确新预算，远端任意更大输出不承诺兼容。
- stderr 256KiB尾部，记录 truncated；错误不回显完整输入/凭据。
- 没有人为批准等待期限；worker执行设绝对300s保险，与现Delegation单请求上限一致且 Stop/cancel立即抢占。run级取消 sweep 单独短预算，不能沿300s。

## cancel_children 的范围决定点

现3调用点（Stop新线程、execute结尾、mcp_request结尾）必须消除重复无owner同步等待。建议本切片先由 root 确认两种实施边界之一：

1. 同切片：Running保留一次取消登记，唯一有限 coordinator 持 run级 sweep worker；worker内给总预算和每child预算、有限分页，慢child之后仍尝试其他child；本地execute不join远端sweep。coordinator持并回收所有 worker/线程，达到预算 kill/reap/join，durable child links继续保留；结果仅pending/failed/远端真实terminal，不声明本地reap等于远端stopped。需要新增 delegation API/receipt schema，变更较大。
2. 本切片只处理 MCP本地 owner：移除 execute和mcp_request同步sweep，Stop一次登记启动独立受控 sweep worker（总预算），复用既有cancel_children语义；不宣称慢child公平/receipt查询已修，后续独立红卡完成。重复Stop去重和所有资源有限收口仍须本片做到。

## 精确 allowlist 提议

生产：`crates/node/src/lib.rs`（mod）、新 `mcp_worker.rs`（可分 io/test 文件）、`khor_agent.rs`、`khor_agent_process.rs`、`khor_agent_process_unix.rs`（typed profile接线）、`khor_agent_process_windows.rs`、`host.rs`、`gui_host.rs`、`crates/commands/src/lib.rs`。若同切片cancel sweep扩展，另加 `delegation.rs` 和新 `mcp_cancel.rs`。CLI/GUI入口不必改；若 GUI console 不转发 commands 则再申请其实际文件。

测试：原 `khor_agent_mcp_cancel_tests.rs`（仅 cfgtest确切本轮CLI路径注入、保留gate oracle）、必要新 `mcp_worker_tests.rs`/fixture、`khor_agent_process_control_tests.rs`、Windows自有harness显式测试profile与durable spawn。原首红三源SHA保留历史，并不要求修改后仍相同。无Cargo依赖/锁/生成器/前端改动。

## 最小验证清单

1. 本轮实际 `khor` CLI build；Node cfgtest注入其确切路径，不借libtest解析stdout、不借旧ship binary。原真实IPC barrier+gate未释放时Stop释放map/claim、下一Prompt成功；cleanup仍先于断言。
2. 真实hidden worker一次合法读、一次approved写/未批准写拒绝、parent TaskRef/track_child一致；配置home不对拒绝；EOF/第二JSON/超限确定失败。
3. exact cancellation通知、Stop-before/during-spawn、旧结果不污染新请求；重复Stop不生成多sweep；stdin阻塞/双pipe大输出/超限/临时后代清理均reap+join。
4. durable open(_host)与delegate(_ghost)在worker自然退出/取消后仍活，temporary child随取消终止；Windows未实际跑时明确仅源码/CI覆盖，不冒充运行。
5. 既有Pi/ACP/shared process/control/receipt/SSH相关回归；source-before/after与真实重编、binary SHA、磁盘/PGID/墙钟守卫完整。本卡不授权Cargo/native槽，执行前另向root申请。

下一步：root确认allowlist、输出预算、sweep切片边界后才写生产；提交前冻结SHA并交独立源审。本轮不合并main/ship。
