# Q052：终端统一当前协议

## 当前交付（2026-10-27，root）

源码作者提交 `95428b255ab5b8266817bbcada956641f4978538`，治理集成提交 `8d1033bbfb7ab32582b0e55f0b380c13c45a6130`。共12路径，包含当前host协议统一及真实clear/replay修复。root已逐文件核集成后的12路径SHA精确等于source04冻结，作者树与集成树clean；main未改。

source03实际28个不同功能回归通过。原09整run因两个私有停止tmux socket清理失败，原失败收据保留；root另精确后清。source04只修该fixture生命周期，其余1920输入含生产完全同03。plan10实际唯一目标构建74.58s通过，原tmux2回归2/0/0、无skip，运行6.42s；源1921/实际normal库与exe稳定、所有自有组退出、TMP和socket清理完成、无guard/forced，整run exit0。root已独立读raw/results/final并核42份双盘一致。

source04独审在 `Q052/terminal-candidate04-root-review`；运行证据 `Q052/terminal-green10`。当前作者验证已收口并提交集成，**集成分支定向native验证与最终同源App验收尚待后续，不把作者树通过直接写成App通过**。此前准备/失败记录如下，均保留历史含义。

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

## source03：真实 clear/replay 缺口最小修复（作者交付，未运行）

Root已接收07原始失败及只读归因，另授03三源写权；02/root审历史及05/06/07原件保持。07原TCP父第201行实际发现before-clear重现，不以改cursor规避。实现仍fix/quality-terminal-current@13a9459f6c2dfd8d12912e6e2325b58de6e06999；总scope12路径/11现存源不变，02→03仅host.rs、core term.rs、term_io_tests.rs。

host抽窄pub forget_parser(parser, scrollback)，机械复用Model原normal-screen清理：保cursor当前行及后续、列、属性/input modes/visibility，清旧screen/history；Model的Beside归属/offset/原normal guard仍留Model。Core写锁内顺序为Forget成功→parser/feed同临界区清理→可选Ctrl-L，线上op仍Forget/Input；首write失败不清本地，任何write失败继续Err。局部清理采用paint/take_since一致parser→feed锁序；life=max(旧life,current fed−retained len)，fed不跳号，caught-up cursor仍连续。Alt本地不清，与真实host Model和GUI normal guard一致。Reader paint返回已释放parser/feed才取conn发Resize；resize/paste临时parser guard亦在conn前释放，无现有parser→conn交叉持锁。无ACK/新协议/TS/依赖/公共catalog修改。

新增4普通父均通过生产paint和submit_forget组合及真实TCP帧读取：`a_clear_preserves_the_prompt_modes_and_caught_up_cursor`（含第二次clear）、`a_later_feed_trim_cannot_revive_the_screen_a_clear_forgot`（完整bounded OSC titles超过真实FEED_CAP而不滚掉旧screen）、`a_redraw_is_requested_only_after_the_local_screen_and_feed_clear`（持parser锁证明Ctrl-L等待清理，随后真实redraw保留）、`an_alternate_screen_is_kept_and_a_failed_forget_does_not_clear`。原term_io全file逆去新方法+4父逐字还原02，原TCP terminal_current整个file逐字同02，包括201与最终同socket Input。Host原formatter/parser replacement尾机械等值记录在source-equivalence。

仅rustfmt emit stdout skip_children parse、git diff --check、full/delta reverse patch --check通过；没有Cargo/typecheck/native/生成器运行，不将source静态通过称绿。当前1921source除上述3源外其余1918逐字同02；571frontend/155genTS/277actual06dist全部未变，不冒新App验收。后续仍须独立源码审，以及同源当前normal构建、新4/原Q05223父与受影响Host model/snapshot/真实CLI清理适用验证；native_handoff仍仅compile-only。

冻结：external `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q052/terminal-current-candidate03`，internal recovery/Q052同名，27文件逐SHA相同，完整source/before/fullpatch/delta/1921inputs/gen/dist/静态记录齐。

- manifest `208f5e65a019a4ac651844e113e9a8aaf982b24d89910d6583049ecd2dd4d9bc`
- fullpatch `6e089967b58dce35b9e8219492f5686a7159cdc003cc93e238c40dadc240a08e`
- delta02 `d6d438dda3b5116e48c8d70a80ec351123e2c5dcf3bd7f02eddb74f49faf0575`
- source-inputs `208f4f12a6535d6f92cc7f6496109cecab993fb94288b9bd8b6377689787689b`
- term.rs `710e6e2fce8c942e91f2504a038dca9e9e6fee36b51a1ca6269e6f516706e936`
- term_io_tests.rs `8506789852f8bb3df9556f726ff717bbde3144ac8e12924ca64faf382a91c54d`
- host.rs `a49f4a72cade30e648e611c6269b845d49baf30cb777f66daf3203d0067894c0`

作者冻结三源，不commit/不编译，不自审；本卡本次追加写权交还root，validation-plan保执行收据写权。Q052仍open。

## 2026-10-28 实施、验收与集成补记

本节接续上文候选冻结之后的实际结果，不改写旧失败。完整执行账在[Q052验收计划](Q-052-validation-plan.md)的plan09及source04/plan10末节：source03实际28个不同业务父通过，native_handoff仅编译、不计业务父；原plan09因两个自有stale socket保cleanup非绿。source04仅修host_shell_tmux.rs的fixture有限退出与精确socket清理，生产和其余1920输入相对03不变，保全部原业务oracle。

plan10实际必要构建0/74.5767s、两个原tmux父2/0/0/6.4232s，总83.0786s；source1921与产物指纹稳定、owned全退、scratch_absent及cleanup_completed均true。它承接未改26父并重验受影响2父，不能相加写成30个不同业务父，也不回填plan09原清理失败。

作者95428b255ab5b8266817bbcada956641f4978538已由root集成为8d1033bb，12源码路径与source04逐SHA一致、当时集成clean；因此上文“不commit/不编译”为历史候选状态。本片已完成该次实现与定向验收。后续当前协议/源码变更与整合结果按[STATE](../STATE.md)和确切来源映射记录，不将旧28父冒充最终全部源码重跑；真正App及物理设备边界继续保留，当前不据此标完整P5或全部终端平台done。
