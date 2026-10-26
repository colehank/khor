# Q052：终端统一当前协议

2026-10-27，owner node_test_review（实现作者，root独审）。复用 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，交接clean `13a9459f6c2dfd8d12912e6e2325b58de6e06999`（tree与集成767aa相同），切隔离分支fix/quality-terminal-current；不新worktree/target，不接Gen。Gen源码与卡已交还root，本片不再写它们。

先读权威STATE/PLAN及Q033定界；refactor-safely图first minimal/callers/file-summary，图build686f不匹配13a，仅作导航，当前无.codegraph。随后实际全仓Rust调用搜索补漏：host版本化包装供gui-core term/commands CLI attach及node与cli直接tests；图5callers未覆盖这些全部test。0flow不作coverage。

契约：成功Welcome严格khor-host/5，client首帧固定Version5；host完整首帧在固定deadline内读完，拒绝无Version/空/旧/未来/错误首op，不降级或min。删除Host Attached.version/三个versioned冗余包装及Term version状态，统一现有connect/connect_at/connect_view直接返回TcpStream，调用方只机械适配。保共享cookie+deadline/GUI自己的greeting、当前tmux redraw、model缺失ring用途、当前snapshot与history、renderer Ready门/启动只一次、Forget本地及host清屏、Far reconnect/FIFO。现API Forget bool形状保留，当前成功明确true；不动TS/UI。

允许路径：crates/node/src/host.rs、apps/gui/core/src/term.rs、实际必要机械调用crates/commands/src/lib.rs；直接tests host_snapshot/host_replay/open_from_phone/native_handoff/host_shell_tmux、cli attach_version/open、core old_host_wait（改为当前自包含适用验收）。必要新增同目录私有handshake tests可放host模块。严禁link/proto/lib/catalog/依赖/lock/UI/生成器修改。catalog由N1所有，本片协议不匹配复用msg.peer-non-answer适用错误包装，不新键。

验证计划：真实TCP current greeting→Version5先于Input；不匹配greeting拒绝具体catalog键、cookie拒绝原why、wholeframe drip/partial/silent/oversize期限；真实host+PTY拒绝旧/未来/无Version不执行启动，之后current Ready才放行且恰一次；保snapshot/history/Forget/tmux原oracle，core真实TCP核Ready/history/Forget与同socket继续，旧兼容probe不计删除成绿。暂不Cargo/native/gen/启动用户App或tmux服务，只有源码/格式/静态核与双盘candidate冻结，待root独审和另授槽；未宣称测试绿。

实施检查点：实际还补获term_io_tests.rs的第六个Term构造器，仅读完其3普通TCP/FIFO/关闭重开oracle后删除一个version fixture字段；inverse还原与baseline全字相同，断言未改。这是直接必要机械适配，不新增能力/生产路径。现新增terminal_current替代旧ignored old_host_wait探针，没有把删除legacy当绿；Node5握手父、CLI2父（12种拒绝首op循环仍1父）及core2父都只是源码未运行，原term_io3父仍全保。

完整firstVersion截止复用HANDSHAKE15s：旧300ms是超时仍降级接纳的窗口，严格当前协议不能把跨网络Welcome→Version往返误当仅loopback而在慢路上踢掉合法客户端。绝对deadline覆盖prefix/body、每次read剩余预算及完成后的截止复核；不peek后撤timeout，也不让每byte续15s。私有真实TCP helper以150ms短预算覆盖silence/partial prefix/partial body/drip，生产Hello/Welcome/首Version仍15s。成功当前client立即发Version，不等待该deadline；shared GUI greeting也走改进的bounded Welcome读取，后续适用现gui_greeting4作为回归。

## Candidate01交审范围（未编译/未运行）

生产三源host.rs/term.rs/commands lib.rs；必要直接测试共8当前源，另删除原ignored old_host_wait.rs，合计12变更路径/11现存源。strict拒绝覆盖完整greeting、客户端Version5必first、认证拒绝原why、首frame缺失/空/旧/未来/错误op/oversize以及全deadline片段/drip；非零Err不自动算通过。CLI PTY正向标记改分段printf，避免命令echo本身满足结果；保host clear后重attach历史消失及拒绝客户端不改现PTY尺寸/不执行输入。Phone旧兼容成功迁为明确拒绝，现startup输入/Ready×2/第二attach仍保证只执行一次；原snapshot/history/颜色/modes/延迟、tmux capture与native handoff断言仅机械connect API调整。

Core terminal_current是受控TCP peer，不声称真实PTY/App；它通过Node本次自有registry与真实term_open走local host door，具体错误catalog键拒绝非current，current时真实Version→History→观察无自动Ready→renderer Ready→Forget/重画→Resize→同socket输入继续。该test-only marker指向自己的peer进程，不调用Node::close（避免杀测试进程）；只term_leave、bounded peer join和owned root清理。真实PTY另由CLI及既有Node host tests覆盖，均待实际运行。

静态只执行已有Rust源emit stdout skip_children解析、两重写测试rustfmt --check、git diff --check、全仓Rust旧wrapper/版本状态调用检查、term_io_tests单字段inverse逐字proof、完整patch反向check；不能称typecheck/test绿。外盘Q052/terminal-current-candidate01与内盘recovery/Q052同名将含完整前后source、完整tracked+new patch、所有tracked/new非ignored输入SHA、candidate/本卡与具体test名单；以实际冻结回执为准。协议由用户明确决定，不伪造legacy业务红或删除测试变绿。

root现在唯一槽用于集成/后续N1，本作者没有Cargo/native/gen/用户App/tmux/服务PID，无新target或缓存清理，未commit。交root非作者源审后，再排正常Node/Core/CLI与定向TCP/PTY/phone/snapshot/history/原term_io3/gui_greeting4适用组合；tmux要独立owned socket，native_handoff仅明确隔离fixture另授，不运行现用户会话或物理平台。

## Candidate01冻结回执

external `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q052/terminal-current-candidate01` 与internal `/Users/zgh/Library/Caches/khor-code-quality/recovery/Q052/terminal-current-candidate01` 共28文件逐SHA一致；12变更路径（1删除+11现存），1921完整tracked/new非ignored现存inputs。source_manifest SHA `81165938195fc3c14494e5db87995e10cf78f59ee036f156c6601f2598914fbe`，完整patch SHA `42035b0dcdd8315cdf50269be70d11866a6e382ce75d0fc3f09030e03377f0e6`，反向git apply --check通过且不改index/源码。baseline13a/branch fix/quality-terminal-current正确。

生产SHA：host.rs `2536d1288706e296c39b4ef018d74f90c2464764202a2e951860b1a56c766096`；term.rs `7cc4920a931f33e9292238b9bc102ca1c4e6379ce34e16915f89cd86ae352b0d`；commands lib.rs `81e97aa9577037169d904c0bc42f30269999ede865d29a7dadd0c6060f63f61f`。完整11源SHA/bytes及删除项在candidate.json，前后全源都保存。最终available外5879201792B/内12345278464B，均≥5GiB。候选卡快照不含本回执。

源已停止变更，交root独审；无Cargo/typecheck/test/native/gen/服务/自有PID/新target/缓存清理/commit，不把9定向普通测试源码当9通过。本片不影响已交Gen；任何后续修订另冻delta，不无声改本候选。
