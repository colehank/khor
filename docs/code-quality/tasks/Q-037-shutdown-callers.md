# Q037 调用方关闭边界审计

状态：已确认缺口、待设计/修复/实际验收。Owner root。依据 R1 c81/8292 源码与集成8395同一调用面；node_transfer_owner完成只读追踪，未改代码。不能把底层 runtime-close 测试通过写成全部 App/serve 退出已干净。

- 当前 commands/CLI 没有单数 `agent` 命令，也没有 `khor_agent_on` 调用；`retry` 是文件接收的另一业务。因此不存在已审的 `khor agent … Stop/Retry` 一次性 CLI 命令尾留案例，不新增该命令。
- 真入口 GUI core→`Node::khor_agent_on`，先经 resident IPC，无 endpoint 可本机 `khor_agent_here`；不能假定总经 resident。本机 Stop 需要本进程 running executor，RetryCancellation 可自行准入 metadata。
- Tauri Exit 已明确调用 `close_agent_runtime`，当前所审 `serve_as` 只有启动 acquire，没有 close。其正常 return/cancellation 与 keeper orphan/deaf/watchdog 强制 `process::exit` 需要分别审查；强制路径的可用性契约不能因等待卡死清理而丢失。
- static Scheduler/driver 不会在 `process::exit` 自动析构。Native child 有原有5秒 fuse，这是有界尾留，不是 join，也不证明退出后零逻辑写入。Step 尾留在 driver 之前已存在，fastStop又允许metadata未完就返回；新driver允许返回后启动已准入后续Step，未增加新的child种类或取消原fuse。

下一步：沿实际 serve/GUI teardown 找出正常退出与强制恢复边界；确定停止新准入、仍允许运行中 owner 完成、Root closing/closed 与 transport 关闭的顺序。现 runtime close 只管理本进程 agent producers/取消 owners，不等于整个 Node IO shutdown；不要把全部异步服务清理塞进它或用网络结果冒根目录写入回收证明。正常退出验收须包含真实 API local fallback、并发已准入请求、所有 owned Native PID/PGID/claim 与 Root 不再晚写；强制退出的保证另列，不能伪称正常 drain。

本卡不阻止已完成的source07取消检查点提交；是完整P1/P5仍未关闭的明确调用方风险。

## 当前源码进一步核对（集成36e3918f，root）

实际实现位于 `crates/node/src/link.rs::serve_as`（不是lib.rs）；graph旧快照只作定位，实际源码已读。入口先acquire，然后bind/publish/listener等多处可早返；非resumable endpoint accept返回None是正常break，循环尾当前直接Ok。连接handler/本地handoff/pump等异步任务另有Arc，不能只在尾部加close便宣称所有入口停止。`remote_cancel::acquire`允许显式新工作重开已Closed的Root；closing期间会拒绝，CloseGuard结束后新工作可重开，符合当前API用途，但说明退出要先停止/收拢调用方，不能把Closed当进程永久退出闸。Tauri真正Exit当前close的Busy只记录日志；不提供整个运行中executor已停止的保证。

下一实现仍需沿现有服务owner确定正常drain与强制watchdog的边界；当前没有在未收拢服务入口时添加假关闭，也没有把强制恢复改成可能无限等待。此为已确认待修调用方风险，未提交生产修改。

## 2026-10-27 shutdown_owner 源码候选（未编译/未运行）

- 实现树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，`fix/quality-shutdown-lifecycle`，基线 `ec404dd71df62c15703c818f65fabb699be1877b`。截至本记录有未提交源码；只改 Node `khor_agent.rs`、`link.rs`、`lib.rs`、原 `khor_agent_runtime_close_tests.rs`、新增私有 `khor_agent_runtime.rs` 及 Tauri Exit 调用。来源工具/Q055 helpers未改；未使用Cargo/native/生成器，未碰App/服务/未知PID。
- 小型本进程 Root registry 持有 agent API futures 的准入、实际executor JoinHandle 和serve lease；Node私有OnceLock首次API/显式serve绑定代次，普通Node::open不acquire。正常serve stop只关闭本代，真实close成功、calls/handles/drain/旧serve lease全退后，新Node新serve owner才可开新代；旧Node/迟到handler不能借新代重开。Tauri真正Exit另加terminal，阻止所有新代。
- 新 `link::stop_serving` 是正常可重启停止，`link::shutdown` 是最终Exit；均先seal再发真实serve stop door，只有实际循环清理/agent drain后的ack可返回Closed。无serve的真实API local fallback由同一drain处理。`Node::shutdown_agent_runtime` 请求本地executor停止、收拢已准入API（包括登记Running之前的Prompt）、实际join后才调用原close。Busy/错误/取消waiter均保seal及Root；原close/acquire独立语义保留。启动bind/publish/handoff失败只放本次lease，不terminal seal已有API；重复serve不接管其它owner。watchdog/orphan/deaf process::exit及native fuse未改。
- serve本地JoinSet持有直接sampler/pump/connection/IPC/wake任务，正常退出seal→关listener/endpoint→abort/join直接任务→agent drain。已有endpoint watchers随closed结束；不把NodeIO子任务、durable GUI hosts当作agent close对象，不能据本片宣称整个Root任意IO均join。
- 新增4个ignored acceptance父，均在原runtime_close_tests模块，内部隔离worker：`final_shutdown_joins_actual_fallback_and_fences_admitted_and_late_calls`；`canceling_final_shutdown_waiter_keeps_the_seal_and_actual_executor_owner`；`normal_serve_stop_restarts_with_a_new_owner_and_rejects_the_old_node`；`early_serve_publish_failure_does_not_seal_an_existing_fallback_owner`。前二真实API无endpoint fallback启动自有Python Pi，实际retry-record writer屏障，并发late Read/Prompt/Retry/acquire拒绝、PID/PGID ESRCH、native metadata intent/claim/Root不晚写；后两实际serve生命周期/代次与bind后真实filesystem publish失败。内部worker不是额外业务父；这些是fixture/native进程，绝非Tauri真实App Exit验收。
- 仅 `rustfmt --emit stdout` 解析与 `git diff --check` 已过。graph只覆盖main旧快照，detect_changes仍返回main/SHIP，不能作为本候选影响证据。四父、既有R1/close/paired、正常GUI构建及真正App Exit均未验；槽由root协调，未自行使用。

### 额外已确认的管理producer缺口（不能以本候选掩盖）

`Action::Manage` 还可启动provider setup与ACP setup。`agent_accounts.rs::manage_agent` 的16-job表持有raw Child、stdin、detached stdout thread，结果与失败cleanup可重新Node::open并写credential/account/config；旧10分钟清理不是normal join，也不全在原CloseClaim内。`acp_management.rs` 的setup thread经 `khor_acp::manage`，当前 `crates/acp/src/management.rs::Management::drop` 仅abort SDK driver，无TransportLifecycle ProcessReaped收据。`acp_open` 的 `spawn_gui_host_context` 则是明确durable host，不能误杀。

当前候选能阻止新Manage准入，但没有把已启动的上述管理producer写成已join；全agent正常关闭仍需其owner接入/真实回归。已向root报告最小额外文件范围并等待范围协调；中央STATE仍由root维护。本卡不标done，当前源码未冻结/未提交。

## 2026-10-27 Management 扩展与 source02 冻结（仍未编译/未运行）

root已确认管理setup是本轮正常退出必须覆盖的producer，并批准最小扩展 `agent_accounts.rs`、`acp_management.rs`、`crates/acp/src/management.rs` 与ACP lib同义re-export。上节管理缺口是发现时事实；当前源码已实现下面收拢，但不能写成验收通过。

- Pi setup Job 复用已有 `agent_process::Session/Control` 与有限Input writer，spawn前登记可停止Job；初始stdin write在owned reader线程内。Session/native进程组、stdout reader、Input writer、stderr drain和实际account/config commit/failure credential cleanup全部在producer JoinHandle内，shutdown实际join后才退Root。terminal状态poll不会提前移除尚未完成的Job；expired/取消均请求Control，不新建daemon。cold shutdown不初始化原有15s jobs清理线程。
- setup spawn/工厂错误由本次Registration/Setup RAII回收，未确认process/IO收据latch本Root为unconfirmed，Busy保seal。已完成result/账户提交后的shutdown不将已提交credential伪装回滚；已有原Pi完整账户测试仅末尾新增真实shutdown清理，原业务断言保留。
- ACP setup复用既有 `acp_process::spawn` 的owned streams和TransportLifecycle。库增加 `manage_on` 并同义re-export，复用原initialize/authenticate/elicitation driver。Node退setup前drop driver、request_shutdown并等待真实confirmed_exit；5s仍未确认则latch Root，不能用SDK driver abort或Rust线程结束冒Native reaped。`acp_open` durable GUI host明确排除，未修改其进程契约。
- Source02新增两父：`shutdown_joins_pi_and_acp_setup_readers_and_canceled_waiter_cannot_allow_late_credentials`（真实Pi/ACP同Root setup，native reap后仍hold Rust cleanup，取消shutdown waiter保seal，释放后实际Closed、late credential和Root不重建）；`shutdown_preserves_credentials_and_account_committed_before_setup_process_exit`（真实credential已写、Node已commit account而native stdout仍开，shutdown需join且保数据）。连同上节4父共6 distinct业务父，内部worker不额外计数。
- Source-only历史source01六payload已双盘保全；当前coherent source02十payload亦双盘保全，外盘 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/shutdown-lifecycle-source02`，内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery/Q037/shutdown-lifecycle-source02`；manifest SHA256 `4f079e4d86c5956c4ba1d3c1bd2d660e630a4a05364bde323a1f93925b9eca7f`。含逐payload SHA与tracked.patch（新增runtime.rs以payload保存）；基线ec404、dirty未提交。
- 已做静态：十payload Rust解析、git diff --check、Python fixture AST、6父selector唯一性、Q055两个helpers与R2 remote_cancel/delegation逐字节仍等于HEAD。**Cargo=0、native功能父=0、App真实Exit=0。** 原R1/close/paired断言未删/未放松，需后续同源build、新6父＋适用旧R1/close/ACP setup回归、GUI正常构建及真正App Exit。当前不使用共享native槽，由root调度、独立只读审查与集成；不标done，不commit中央卡。

## 2026-10-28 root独审修订：最后stop与代次reservation（source03，未运行）

用户最新范围：已经开始的本轮做完，后续新独立问题只记录下一轮；本轮不扩其它owner、一般NodeIO或新功能。source02保留为历史恢复快照，已按root独审意见继续修订，不将快照冻结等同全任务暂停。

root确认既定代次契约的一项阻断：原Serve::Drop先serving=false，锁外才按Root stop；本代已retired时，新serve可插入转代并启动producer，随后旧Drop误停新代。source03仅改2路径：`khor_agent_runtime.rs`在seal后始终保留serving reservation，锁外完成最后stop后才在匹配generation时释放；不依赖debug_assert保证安全。原启动失败无terminal seal与正常stop后新owner重开契约保留。

原restart业务父中加入同一真实serve最终Drop屏障：正常drain已ack、旧owner尚未执行最后stop时，显式新owner lease必须Busy；释放屏障并等旧serve实际完成后，原后续真实新Node/新serve启动与旧Node拒绝断言继续执行。没有新增专项父/矩阵，仍6业务父。source02其余8payload逐字节保留，Root/producer边界未扩。

source03双盘：外盘 `Q037/shutdown-lifecycle-source03`、内盘同相对目录；manifest SHA256 `4e682bd69dc369a5fe96ba61ff8ea9b11ad021a86f669f28abe75162af2e0d01`。两个变更Rust解析、diff-check、严格两路径差异与6父数量检查已过；Cargo/native/App仍0，等待root既定独审及授槽。本卡未提交中央index；源树仍dirty未提交，source01/02完整保留。


### source03非作者独审完成（node_test_review＋root）

当前10payload与双盘manifest相同，02→03仅runtime与restart父变。独审未确认现契约阻断：旧Serve reservation跨最后stop保留，API Call覆盖spawn→register，producer join锁外且reservation持续；Pi Session真实wait/IO join，ACP confirmed_exit来自native reap+IO join，超时/线程panic保unconfirmed。可以进入既定normal CLI/Node构建、新6父＋原R1 Node30＋既有账户父、normal Tauri caller check；真实App Exit仍P5。此时仅入口准备，尚未启动native，不加新专项。


### 实际验收准备与Pi复用（未执行shutdown native）

node_test_review未launch已归还槽，N2先实际运行；避免环境准备占空槽。shutdown_owner只读找回真实Pi产物 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/real-bundle-darwin-02/khor-agent`，实际SHA `af9555409de4af93cf166aeedc4b88f19196b5848dc89dd496ea0c650b79d646`/76474082B/0755/MachO arm64。当前shutdown树全部26agent输入与Q041 run02 before/after/build完全相同（无增减），package/lock及现locked cache亦同；bundle原件不在已归档14Rust二进制中，仍直接可用。真实accounts独立父给KHOR_AGENT_SIDECAR绝对路径即可，无需node_modules、下载或再build；新6父内部Python进程仍只作各自fixture，不能冒真实accounts。本片生产source03保持，无新的工具治理专项。


### shutdown plan02 READY（37父＋normal Tauri，未launch）

入口 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/shutdown-validation-plan02/run.py`，runner SHA `2bc5ee7cfbbfd58e78411a6db718a81122d5db76d1d05b90098a7c6554094c42`，data `addb46fde7ed7de1805d1451050c2e051e86df9b2a17968a44f884d497a2e66c`，5dual。沿原controller最小换2378当前输入/434ignored/normalCLI+Node两build，new6先跑（各外内summary不另计父）＋原R1 Node30＋真实accounts原1=37，最后正常Tauri caller check。独立ACP旧全矩阵不加，manage_on同义wrapper原driverbody不改且新6含实际initialize/authenticate/reap。root已读plan01全runner，并要求plan02仅两guard行显式target外盘持续5GiB（ROOT现内盘不可只查两内盘路径），原300/720/5GiB/owned/proof/资源门全保。当前READY无native，N2唯一槽未归还；source03仍未编译/运行/App验收。


### plan02实际启动（未有业务结果）

N2 full07 terminal归槽后，root正式授node_test_review唯一native槽。已启动session39349/controller86640，source03十payload/current2378与runner2bc5ee…94c42预核一致，metadata0/1.15s，现build-cli。仍normalCLI+Node两build/新6+旧R1 Node30+真实accounts1=37、normalTauri callercheck；未称App Exit绿。N2仅并行只读准备原Pair04同源单父采样，不会再启动native/编译。原双盘5GiB与有限/owned/proof/dual门不变。
