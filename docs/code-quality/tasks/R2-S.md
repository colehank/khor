# R2-S：会话恢复与生命周期

状态：首slice真实旧逻辑两业务红已固定；green lease闭包实现中，尚未编译/生成/验收。Owner：round2_history；独审：round2_reviewer；集成与运行槽：root。

调查基线：作者树 `khor-round2-history`、`fix/round2-history`、HEAD `45e8651dd8d9d04982186956b9e8d9d5d21c29bf`，tracked clean。A4已有分层证据但Root集成组合核仍独立。无本任务native/App/browser/服务器在运行。不得修改用户main或操作真实session/进程。

## 原要求与完成边界

沿 `SESSION-CONTINUITY.md` §2/§5 S3、`ROUND-2-EXECUTION.md` 包S：R2-002/003/007＋L01/L02。此前未有R2-S卡或SESSION-PERSISTENCE.md，不创建虚假历史占位。用户已授权整个第二轮，无main/ship授权。

- 长期会话身份/历史、一次运行、界面连接、执行任务分开；关闭视图不终止宿主，宿主结束不删除记录。
- GUI正常退出、GUI崩溃、宿主死亡、机器重启、远端断网分别验，不用持久host认证replay代替完整GUI/terminal恢复。
- 存活host优先重连；明确结束后才按真实能力提供恢复/新Shell运行。普通Shell不能恢复内存或自动重放旧命令。
- 避免第二writer；远端不可达不等于执行失败或可自动重开；结果不明的副作用不重做。
- 只读观察alias不获得控制权；A4 source binding/页/raw/取消owner继续复用，不新建第二历史框架。
- H源全文搜索/复制欠项单列，不与S同时开框架。

## 已读图谱与当前真实链

先query_graph(file_summary/callees_of/tests_for)，graph build fa3b落后HEAD、部分tests零覆盖，定点源补核；动态`.begin`图跳到同名content begin的结果不作真实调用证据。以下均按源码核。

| 事实 | 当前owner/源 | 恢复边界 |
| --- | --- | --- |
| 会话登记 | node/live.rs `Meta`：完整Khor id、kind、title、pid、started_ms、category、vendor_leaf、place | Meta不等于活跃执行证明，不是界面连接身份；pid不代表Codex app-server任务仍活/已死 |
| 内容身份 | A2 canonical ObservationRef；A3 journal source tuple；A4 ContentLocator/ReadBinding | namespace＋完整native ID＋content-home固定，不依metadata-owner在线；观察不自动resume |
| terminal持久host | node/host.rs HostFile含port/cookie/host_pid/child_pid；协议HOST_VERSION5，真实cookie/version/Ready门 | host存活时可重连并接收屏幕replay；机器重启后原PTY内存不可恢复。L05已验证仅该基础链 |
| GUI持久host | node/gui_host.rs `_ghost`持ACP；相同HostFile独立GUI greeting；存活时Core连同一个host | GUI退不应kill；Node.reopen_gui_as是显式恢复，会核live sighting及Codex writer锁，不能普通打开时偷调 |
| ACP原生恢复 | actual OpenedBy New/Load/Resume；A3首次base＋Khor接纳prompt/实际事件 | Ready不证明full；外部离线段用CaptureGap事实，不能把resume当自动完整历史 |
| GUI配置上下文 | `.khor/acp-context/<blake3(full session)>.json`保存非空Environment(MCP/额外目录)；agent key在Meta/source，启动cmd在存活host启动参数中 | 未据此声称完整launch recipe/模型配置已可自动重建；恢复须核真实adapter能力与来源 |
| 连接注册 | core/chat.rs CHATS与core/term.rs TERMS仅进程内Arc；`holds:AtomicUsize`；共同Landing按id计数 | 关闭App后这些连接状态自然消失；session/host/journal仍在。当前view没有独立attachment lease |
| 远端重连 | core/rejoin.rs，torn far pipe→stall→backoff rejoin，Gone和Unreached区分；Connection::replace拒已close对象 | 不可把同一个reader对象已处理的late replace当所有同id重开安全 |
| live GUI历史 | ChatView fresh和batch.lost均自动chatReplay；Core Replay→GuiOp::Replay→ACP load，运行时Replay有独立预算 | A3 journal尚未供live GUI重挂hydrate。replays=false的托管ACP即使有journal，重挂仍只有新事件；不能据A4 still分支绿称已修 |

## 首个最小slice提案：明确连接代次，先拿实际反例

当前源码可推导确定时序，但尚未native执行/未称实际红：

1. 同id open A进入Landing.begin并在connect阻塞。
2. A卸载，leave(id)在无registry时记early=1。
3. 活跃view B开始open，同id opening=2；B先连上并land。
4. Landing按“先到先取”返回left=true，B连接被shutdown，chat_open却返回Ok(false)。B live callback于是直接poll，且因为fresh=false不请求history。
5. A后连上并land，拿到hold、注册reader；A的前端已stopped，不再history请求。旧attempt实际取得活连接，活view接到的是“已复用”但没有自己的成功attach。

现Landing既有单测明确只按总数验证“先land吃leave”，未验证A/B归属；已有chat_io/term_io fake TCP与命令队列夹具可复用。不要因为时序相关就把它命名为原L02根因。

建议首提交只做必要的私有连接接纳/完成接缝与实际Core反例，保业务语义先取得红：

- 预计写路径：`apps/gui/core/src/landing.rs`、`chat.rs`、`term.rs`、既有`chat_io_tests.rs`、`term_io_tests.rs`。
- 私有closure接缝把真实connect效果注入既有open流程，生产仍唯一调用原attach/归属/cookie路径；测试复用owned TCP pairs、显式barrier控制A/B完成顺序，不新env开关/全局测试路由/另造服务。
- 反例要断言活跃B确实能从自己TCP接收标记，A晚完成被弃；不能只assert is_err或把无socket失败冒授权/代次正确。失败cleanup释放barrier并join自己的threads。
- 先分别固定GUI与terminal、正常两view共享、旧view晚leave不关新view；不重原大矩阵。具体filter/计数/源freeze准备后再向root领单native槽。

修复方向待真实反例后固定契约：view/open尝试有独立opaque lease；leave针对原lease，不能只按id减当前holder。必须覆盖leave先于open完成及先于慢worker开始；仅加客户端generation忽略不能代替Core资源回收。若必须cheap Begin ACK消除早cancel竞态，应明确额外RTT、owner上限/过期/claimed持有至实际worker退出，不顺便造通用请求框架。公共接口变更先同root固定，不留旧签名兼容层。

后续公共owner需求（尚未改）：core/api.rs、Tauri lib.rs/native_io.rs、GUI api.ts、ChatView、TerminalPane、可能gen_bindings以及既有签名夹具。当前先申请上述五个私有源/测试，不提前抢TerminalPane等共享UI。

## 后续S片段（不并发实施）

1. live GUI历史改为复用A3/A4 canonical有界读取，控制/status/permission pipe与内容呈现分开。去掉普通重挂依赖ACP load；不把journal与live chunk无ID拼接/文本hash去重。首无load ACP夹具：已有富内容→关闭连接/重开GUI→原内容可见、无new/load/resume/重复prompt；连接仍可控制同一host且不建第二writer。
2. 退出/崩溃/宿主死亡/机器重启与far断线的动作状态分别核；原生恢复显式动作，Shell新运行标分界，不重做旧命令。
3. 启动转发有界：acp/start_configured目前opening_notes最多32768条但无byte预算，返回前还转入unbounded channel。A3 journal流式落盘不证明这段已界定。不能偷加32MiB总历史cap；需围绕现事件owner/握手有限fence给合法长base完整保留和可取消流式交接。
4. L01/L02按原真实入口另验，不能以新连接例或上述能力补丁销账。

## 原遗留精确入口（root移交，未运行）

### L01＝040取消deadline

- `P5/integration-affected-combination-run08/recorded-failed-original-cases.json`，证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`。
- 原filter `khor_agent::cancellation_receipt_tests::retry_deadline_bounds_actual_worker_starts_without_starving_a_fresh_run`，ignored。
- 当前源实际文件 `crates/node/src/remote_cancel_tests.rs:1209`；khor_agent.rs通过`#[path="remote_cancel_tests.rs"] mod cancellation_receipt_tests`接入。
- 所需实际`KHOR_TEST_ACP_STUB=@stub`/`KHOR_TEST_MCP_EXECUTABLE=@cli`以原receipt精确env为准，不能盲用旧exe或只跑zero匹配。
- 原run06/case-040-node.log SHA `2a0093a1eb5ad2ca71491f51d9999015585c0f040e663256dd0fdde95e01d8c5`，原run06/result.json SHA `66c55f9e4f1809272562634550825af321927b4ad4c5c5595b7fa983f8d254b7`。旧红不回填。

### L02＝native网页关闭/重开交接

- 中央 `Q-035.md` 四原native场景段及`Q-035-review.md`；两过strict-late-close/native-eight-cap，两失败late-close/close-reopen末尾page_steps channel closed。
- `P5/Q035-native-four-case-review01/report.json` SHA `46be305a9827f2de9972db4aeb7b8a652d5dc5751fe48bab80597243ce98808a` 引用原run01–04。与Core terminal/GUI连接计数不是已经证明的同根因。
- 保旧2pass/2fail；后续确切native场景需当前实际App来源、owned窗口/网页句柄与正常关闭边界，JS mock绿不冒native修复。

## 当前执行/恢复

- A4 UI绑定slice的5组实际浏览器结果、所有源码/gen指纹、own Vite stop、双盘mirror见R2-A最后段与`observation-a4-check/content-browser-1`；整A仍按全部能力合同对账。
- 本S卡截至创建只读源＋限定方案，无产品编辑/新测试/新运行结果。下一步由root分配首五源写权、固定反例后领槽。Root仍管共享owner及唯一native/browser槽。
- compact后先读STATE→本卡→SESSION-CONTINUITY/ROUND-2-EXECUTION→实际refs/diff/slot；不能按这段方案恢复或kill用户真实会话。


## 首S反例candidate-1已准备（尚未native）

root已授首五路径；写前HEAD45e/clean核实。仅4源实际改：chat/term public guard保留并调用私有open_*_with(connect)，原接纳/landing/registry/reader/返回逻辑全文保留，只原attach调用改connect()；refactor-equivalence.json机械比对两函数body逐字一致（除原guard移至wrapper及connect效果注入），未改Landing计数/公共签名/归属/cookie。

两个同名测试在既有chat_io_tests和term_io_tests，精确filter active_reopen_retains_its_own_connection_when_an_old_open_lands_last预期实际2项。各2对owned localhost TCP、唯一id、entered/release明确控制A在Core begin之后阻塞；A leave、B先完成，B自己的peer发送before-A及after-A标记，必须从实际Core poll/stream读到。旧A线程在断言前release并scope join；marker等待2s、connector gate3s，成功/失败均有本地attachment Drop关闭，不接真Node/provider/agent/用户host。测试只private连接生命周期链，不冒cookie授权或原native网页L02。

reopen-red-candidate-1完整5路径SHA（Landing未改仍在manifest）、源码patch/纯提取等价证明/精确运行计划与内盘mirror齐。预期业务红而不是compile红；未运行Cargo/App，先独审源码再报单槽。后续view lease契约仍须覆盖leave早于worker开始，不先改公共API或发明短命cache。


## 首S旧逻辑reopen-red-1：实际两业务红

独审核candidate5指纹/提取逻辑不变/正向TCP marker后，root单槽授权精确filter。实际tool51792/Cargo28823终态exit101/16.098s，884Rust/Cargo/catalog/fixture输入逐SHA before==after；编译成功，精确两例0passed/2failed/0ignored/151filtered，执行0.03s。两正向断言均为B自己的marker无法由Core读取，准确错误“没有这个session：{唯一chat-reopen-id或term-reopen-id}”。没有barrier超时/编译失败/zero-filter；native实际业务红而非仅源码推测。

测试old connector均在断言前release＋scope join，只有进程内TCP pairs和线程、未起额外helper/agent/Node进程；libtest随正常测试失败终态退出、ownedTMP空。source/binary指纹与cleanup在session-recovery/reopen-red-1；原红不覆盖，未green实现/修改公共协议/gen/App。唯一native已主动归还root。后续明确lease契约需覆盖early leave-before-worker；这两个连接计数反例不能销账原native网页L02。

## Green闭包契约（root已授，实施前固定）

### Wire/DTO与命令

- 唯一新增DTO `AttachmentKind`，serde/TS `"chat" | "terminal"`；由真实gen_bindings导出，不手写TS。
- 唯一新增命令 `attachment_begin(id:string, kind:AttachmentKind) -> string`。Core另收实际`root:&Path`，它来自Tauri当前配置或HTTP已认证上下文，绝不从JSON payload取root。返回随机128-bit opaque view lease。
- `chat_open(id,lease)->bool`（fresh意义保留）、`term_open(id,lease,cols,rows,history?)->void`；两者一次claim已ACK lease，不静默补Begin/回退id计数。
- `chat_leave(id,lease)`/`term_leave(id,lease)`也可释放未claim lease。claimed时只设取消，不提前释放真实工作槽；open的connect前后核取消，取消完成的socket不发布。晚到Begin ACK由已dispose前端补leave；ACK丢失只留pending，不开socket。
- 所有attachment读与操作都带同一`id+lease`，Core均从真实root核范围：chat_poll/wait、say(含token/images/files)、answer/respond/control/mode/option/stop/replay；term_attach/stream/wait、key/resize/ready/forget、会话绑定drop/progress/cancel。不把late Stop/key按id路由到新资源。无attachment的临时文件清理/原始上传阶段按自身owner原契约，不假造live lease授权。
- Core `submit_*`仍同步接纳既有FIFO/handle，Tauri native_io只解析多一个lease并用真实root校验，不能每键/Stop重新Node.open或sysinfo。静态API/handler inventory、生成器同步，不保legacy签名。

### Owner与并发行为

- 继续允许两个同时活跃view共享同session的current Arc、reader与命令队列；不存在“只能一个active读者”的隐藏假设。
- 实码已核chat_poll只按各调用者since切片clone Frames，不drain；term_stream/take_since按各since复制Feed，不消费公共游标。各view独立前端cursor，广播/共享历史不会互相偷帧。新增正向例必须两lease分别读到同一真实marker、释放一方仍可读/写另一方。
- 一个`Registry<T>`私有领域owner管理current[(root,id)]、lease→pending/claimed/确切资源、资源Arc；替代CHATS/TERMS裸HashMap＋Landing＋holds计数。两个kind各有typed registry；pending容量总共64（跨chat/terminal共享permit），不是各64，也不是active-view上限。
- pending TTL30s；只过期未claim条目。claimed持permit至真正worker Drop，取消不释放在途容量。published live view不受此TTL/64上限；它由显式view release/Core进程寿命持有。
- current资源gone后可被新Arc替代，但旧lease仍只指向旧Arc；旧read最多返回它自己的结尾/receipt，旧write明确closed，不能返回新current。释放旧lease只影响旧Arc，最后holder离开时只在current指向同一资源时移除，close在registry锁外执行。
- 相同id的新连接不是旧view的新授权。每次open尝试均有lease，不用文本hash、短命provider cache或未声明single-view假设。
- 本片段不销host/run incarnation、远端rejoin中旧queue跨新host的代次、live journal hydrate、startup内存链、L01与native网页L02。对已认证既有handle的快路不作新的全表扫描。

### 写路径/验证闭包

root已授：新core/attachments.rs、lib/gen导出；chat/term/landing/API/io tests；Tauri lib/native_io；GUI api/ChatView/TerminalPane和最小签名/实际owner脚本。旧Landing须在两个consumer切完后移除，不留双owner/legacy fallback。可能将前端Begin/dispose生命周期作为私有小模块`apps/gui/src/lib/attachment-view.ts`（仅客户端lease owner，不是第二网络框架）；该新增leaf需向root明确后写。

测试分批：Registry纯owner/真实TCP red两例转green（含早leave-before-worker、取消claimed仍占槽、pending过期、root/kind/id拒、两view独立cursor、旧gone Arc晚leave/Stop/key不影响new）；实际UI迟到Begin/切换/卸载补release；原FIFO/send/draft/附件既有正向及签名闭包。原2红源固定保留，不能换成宽泛is_err或删断言追绿。源定向可审后再报native/gen最小批，当前无新Cargo/App。


## Green工作中检查点（未编译，不冒通过）

root已批准全部公共闭包，含新增客户端attachment-view.ts及最小实际owner脚本、chat-files.tsx仅绑定put closure、两raw chat_put route（core/bin/bridge.rs与web/lib.rs）、两integration签名测试。Root0478独立App build已终态/无App；S无新native。

已写新attachments.rs单owner+9未运行模型例，并接chat/term current/claim/share/publish/leave，移除生产holds/Landing引用（旧landing文件待最终删除）；读wait完成从原Arc产批，不再id二次查当前。公共Core签名正加真实root+lease，API/Tauri/GUI/现有fixture尚未闭包，当前树不可称可编译。Term记录实际attach far给pane_history/DropJob，复用既有Node.pane_history(machine,id,..)，不扩Node协议。Drop进度/取消的表正按view lease隔离，留same-view job的stopped指针代次，不让旧view取消新view任务。

新前端owner attachment-view.ts已准备：一次Begin/一次open、late ACK补leave、dispose释放同key、旧facade不回退新资源、原错误与有界清理摘要分开；未接API两facade/ChatView/TerminalPane/chat-files，未运行JS检查，不能冒这些异步契约已经验证。


## Green-candidate-1：实际公共闭包已写，未native/生成

27路径完整snapshot/patch/SHA在 `history/session-recovery/green-candidate-1`，内盘`round2-history-current/s-green-candidate-1`同存。旧landing.rs已实际删除；唯一Registry与pending/claimed/active语义不再留计数fallback。所有Core/API/Tauri/native_io/raw chat_put/GUI同id+lease接线，root仅来自实际服务上下文。chat-files只收同一ChatView绑定的put/putPaths closure；terminal drop先捕获同一facade，不在leaf另Begin，不改键盘协议codec。

原fixture已适配lease，旧行为断言保留；新增两真实TCP正向例分别验证两个view独立读到同一marker/同一cursor、leave一方后另一方继续写同一peer。原red两个A慢/B先land的marker断言保留并要求已取消A失败。新增Registry9例尚未运行，Tauri/original parser oracle同时要求lease；原上传/当前终端握手integration签名已补。

纯JS `green-js-1`owner10实际exit0/输入前后一致。包括实际TerminalPane sender闭包受控A.key阻塞→排queued-A→reopen B→release A，queued-A不进B，后续new-B只进B。此前独审发现迁移中outbox字符串动态lookup当前attachment会串线，已改为每个batch带确切view并在reopen丢弃旧未发送段。不是host incarnation恢复已经解决。types-before-producer真实exit2留存：2项缺尚未真实生成AttachmentKind、2项leave Promise<null>/void不匹配；后者已必要源码修正，未用手写gen/type stub追绿。第一次JS准备命令cwd误用GUI却仍repo前缀，未创建文件/未执行测试，工具错误保留，不称业务红。

root已授三个既有ChatView browser脚本与新transport-only attachment-leases fixture窄接线；明确Begin合法ACK、scope/一次claim/closed拒control。该fixture不模拟真实shared resource/队列/TTL，不替代Core实际TCP行为。全部browser尚未运行。原非相关断言不删、不把unknown新Begin当null。

已申请严格native批但尚未获本批槽：三lib check→Core Registry9/chatIO14/termIO9/stage8/put1→terminal_current2/upload_cancel3(nonignored)→Tauri native_io2/command_tables4（共52实际行为例）→真实bindings staging+发布。无App/真agent/用户host；阶段失败停、5GiBfloor、source前后SHA、固定生成器最后绿源才发布。原L01/原网页L02、live journal hydration/host runtime恢复/startup字节预算仍未销账。


## Green-1实际终态/必要fixture纠正

root正式授52项严格native批后真实session19840/Cargo74909启动，`session-recovery/green-1`真实exit101/89.908s，1479输入before==after。三lib check0/21.468s；Registry9/9实际0/66.095s；ChatIO14实际12pass/2fail/0ignored（0.40s测试运行），两败均HTTP旧JSON漏lease，准确错误`missing arg: lease`。A慢/B先land原red转绿、同socket双view marker/firstleave后second真实SetMode/无drain均已执行通过。严格停批，term/stage/put/current/upload/native_io/tables/gen均未跑、未发布产物，不能冒52全绿。TMP实际空已移除，未启动App/独立agent，终态无Cargo/rustc，native槽主动交回Root给W首leaf。

`green-candidate-2`完整27路径、双盘source/SHA/patch，唯一相对candidate1差异是chat_io_tests.rs的三份HTTP JSON增加既有attached.lease，含原invalid-token例（防止缺lease掩盖token契约）；所有生产输入字节与green1完全同。原断言不删/不放宽。green1原log/result/source/driver-after-terminal SHA/cleanup留存并mirror。后续申请仅重验ChatIO14+原未执行阶段，3lib/Registry9按明确source映射复用，不把新test-source说成全SHA相同。当前等root重授下一槽，未自动retry/gen。


## Candidate-3：重试不等旧Begin ACK（仅既定view生命周期）

在等待W native槽时，核到Terminal retry `await previous.dispose()`会让B.open依赖A的未返回Begin ACK，与独立view lease取消合同冲突。最小改为换新facade并void清理旧facade、有界汇报清理错误；新B直接open。旧facade自身stopped和lateACK补leave机制不变，不丢原清理、不复用key。不保证强制中断同步connect，依然只view生命周期。

新增第11个JS例直接抽取实际TerminalPane retry ownership块，旧源取immutable candidate2完整TerminalPane（SHA fb143b0c…e6cd5）运行真实业务红exit1：calls只有retire-A、缺open-B；当前块（SHA e58fd876…ecce）11项实际0。日志、精确脚本与Terminal源码在`retry-ack-js-1`双盘保存。不是对旧完整repo源冒同源测试。

`green-candidate-3`27源完整SHA/patch已双盘；对candidate2仅TerminalPane.tsx和check-attachment-view.mjs两JS/TS变化；Rust对green1仍只有前述三测试JSON补lease，全部Rust生产字节不变。Root明确W15先跑，本作者未持native/未自动重跑，待独审此窄diff与Root确认W真实终态后续原批。当前整S未完成，原L01/L02/live重启持久恢复欠项不变。


## 首view-lease slice固定94131dc8：精确分批验证齐，整S未完

固定`94131dc8e887580038c8d955ed73b58d63594a2a`，27路径（含Landing删除），commit后tree clean；独审逐git-show=manifest放行。green2真实session95761/首Cargo79994，exit0/246.613s，1479输入before==after。续43项精确实际通过：ChatIO14/termIO9/stage8/put1/terminal_current2/upload_cancel3/native_io2/command_tables4。upload App worker1ignored原样保留，不冒已跑；GUI native_io两mock-runtime实际testbin79s编译、两例0.01s执行，未启动真实App。

与green1严格源码映射：只chat_io_tests三JSON＋TerminalPane和新JS检查脚本变化；生产Rust逐SHA全同，green1已绿3lib check及Registry9可复用，合计52实际行为覆盖，不是同一个全SHA批52。`green-final/validation-map.json`明确各原结果和限制，旧业务red/fixture red/types首红均保。

真实gen_bindings空staging、同binary managed发布逐SHA同，165文件（164TS+inventory）；binary SHA`4062a7b38f69e7737506f97eff09678bbe10e4c0694d544aa7fff1a7704aa5eb`，`green2/generated-sha.json`完整manifest。catalog无变化不重gen。post-producer `green-gui-1`实际owner11/history20/observation19/mobile/types全0、输入前后同。独审已核固定27与1479输入、165输出、分批收据，无阻断可集成。

Native槽明确释放；原stage两个既有fixture在本run外盘TMP留下1787B，原tmp-status保留。核test PID80883终态、仅两个已知synthetic目录/无symlink后仅删该run TMP，独立cleanup记录，现TMP不存在，无Cargo/rustc；不清用户状态/缓存。证据green1/green2/gui1/final已双盘mirror，无compiled复制。小bundle26,624B SHA`6dfe0ad3acdb6c37d87f75d24cdf681764048fdf67b3a6f6e3d65eb50a1818cc`已verify并双盘SHA同，见s-lease-94131.bundle/recovery-current.json。

下一仍需本slice原计划实际UI迟到Begin/切换/卸载验收。仅两test-only脚本pending：check-chat-poll加late-begin-retry真实ChatView受控A ACK→reload B开/poll→补ACK只leaveA/不开A→unmount leaveB/晚poll不复活，poll/replay增强fingerprint（不改产品/已固定producer）。snapshot `browser-candidate-1`两源/patch，待独审+Root browser槽，未运行。拟五Chrome场景poll2/cancel2/replay1，不扩大用户App验收。首view-lease slice完成不销整S：live journal hydrate/host runtime重启代次/startup队列字节预算/原L01/原网页L02仍按原合同排队。


## 实际UI首slice结束7246b665；后续S只读图谱恢复点

`7246b665a41262da32c62bf6929be1c56aceedc3`独立test-only两脚本，tree clean；独审双SHA=候选/fixed。browser-1真实session42174 exit0/9.537s，Chrome五场景(poll late-begin-retry/switch-owner、cancel old-receipt/web-admission、replay retry-mid-replay)全部通过；193产品/165gen/脚本source前后同，全tracked diff亦逐字同。B真正文本渲染、cursor7/同B key、lateA只清A不开socket、unmount stale帧不出现/不复活均实际断言。三个Vite端口空闲、own profile进程[]、TMP0B已清，未触用户Chrome。首次post-terminal ps census UTF8解码失败留receipt，未作清理前更正为errors=replace、仍按精确own TMP路径核；不冒测试失败或重跑五case。已交回browser槽，并正式释放TerminalPane给M串行使用。browser evidence和小bundle双盘保存；首slice不等整S完成。

### 后续S两条实际链（7246干净基线，只读，未获写权/未运行新复现）

图先minimal/query（图built94131，HEAD7246；产品同，仅browser脚本新ref）再定点读源码。ACP `start_transport`的notification callback先`events_tx.unbounded.send`；opening阶段把Note塞Vec，只有32768条计数上限、无byte预算；ready后`while try_recv`把剩余事件全部灌另一unbounded forward，再Ready，再start_event_forwarder又unbounded→unbounded。故仅给opening Vec加byte cap不能约束源队列/forward队列；源流持续追加还能延长try_recv阶段。Host要等Handle返回才打开journal并消费，不能让callback等待一个尚未存在的host consumer（会把RPC完成卡住）。后续应评审ACP私有startup spool/有界内存队列+有限watermark，保完整合法长历史，不设历史总量32MiB上限；IO失败/取消保旧committed capture、清私有pending。精确主路径 `crates/acp/src/lib.rs`，受影响收事件/fence消费者须图谱列齐再授，不凭当前host journal64KiB writer外推startup已界。

Live NoLoad恢复链：ChatView fresh或batch.lost自动attachment.replay→Core Control::Replay→GuiOp::Replay→host Replay.start→Handle.replay_inner无条件ACP LoadSessionRequest，即使facts.replays=false。host joined分支只补Agent/settings/commands/usage/Turning/Ask，不发送既有正文。与此同时所有托管ACP已有canonical Journal及H/A4的bounded page/raw来源；所以重建Core后NoLoad仍会空白/显示no-replay，即便盘中内容完整。Canonical read本身可复用，不应给每vendor复制恢复管线。

两项无硬前置：可用“host继续存活、只重建Core attachment”的NoLoad case独立验证hydration，完全不经过ACP startup；startup long load另测。后续hydration最小必须有host单loop串行capture fence（journal完整record边界+文件/source version+对应live-frame顺序），先读EOF再attach或拿UI时间戳会丢/重复。Journal.append当前每record64KiB BufWriter flush，可在同writer取精确EOF；H cursor目前没有外部可信at-EOF constructor，需在adaptor边界新增受校验构造，不能前端手拼不透明cursor。仍按256KiB scan/128frames/512KiB response/8pages512frames2MiB cache和raw64KiB消费，不把全journal复制成GuiNote replay灌内存。期望paths为Node gui_host/journal/read reference/adaptor、Core chat转换、ChatView/history模型、真实producer；新的anchor/control DTO须Root先裁最小签名，不在本只读调查中造第二框架。

固定后续复现建议：已有private ACP fixture开NoLoad/noecho模式，发送并捕获文字/图/文件，确认canonical可读；只leave所有Core视图再新lease attach到同host，须恢复已存正文且继续发新turn，不得偷偷ACP load/resume；在fence两侧受控追加同样合法chunk，精确各一次；源替换、离页取消沿既有H版本/lease。启动另用受控通知流打满旧32768或大byte队列，须流式保完整合法记录、内存budget稳定、失败/close/取消保旧；不把原L01 case040或网页page_steps L02销账。

Root最新调度先M/R2-026+028自定义键行，再回上述S未闭项；本S缺口留在本轮，不移下一轮，不因切M丢上下文。M不改Term协议编码，复用现lease-bound send出口。


### M UI固定后S恢复提案（2423f2b2，只读，尚未新增写权/运行）

M纯与UI已固定/独审，真实触屏/Codex仍待；Root允许下一S只读准备。图先minimal（图覆盖仍主要TS）后定点读当前gui_host/chat/ChatView/journal/reference/adaptor。再次确认NoLoad fresh/lost无条件Replay，host joined只Agent/settings/usage/Ask，canonical journal存在也不hydrate。

新的必要边界：同session两Core view共享host连接/pipe，只有joined发送一次fence不够（后view可能已错过或trim掉）。建议per-view token化只读Capture op，host同单loop返回自己的token+canonical ReadBinding+完整EOF/文件身份的H可信Cursor。fence前正文已在journal，后正文才属live；不能时间戳/文本hash合并，也不把整个journal重新灌History Vec。另一个view的token不得重置当前view，旧generation迟到必须忽略。

最小首片路径提议：node/gui_host.rs；session_content/journal.rs只取完整record EOF；adaptor/transcript_page.rs增加可信EOF cursor构造；reference.rs若需canonical identity派生；现gui_host_cancel_tests.rs与fixtures/acp-cancel.py私有模式。host Capture是读本地writer事实，不暗发ACP load/control，不另起服务；bootstrap未Ready时明确未就绪，不造无界pending queue；错误不宣称全量capture。先独立host/fence/read定向checkpoint，再接Core/GUI。

第二片需Root再派共享签名：Core现S lease-gated chat_capture、typed frame、真实producer/Tauri/API；ChatView只接本token，H reader seed trusted older cursor/watchfalse，保原256KiB/128/512KiB/8页512帧2MiB预算，fence前仅正文交bounded past/状态权限仍消费，fence后live。首屏hydrate期间不接新draft发送，避免Khor已接纳prompt与optimistic draft重复；Stop/权限不因此失效，离页取消仅H read不stop真实agent。直接复用H/A4单pipeline/来源代次，不加第二cache或恢复框架。

尚未选定新wire shape、未改生产、未启动native/browser/App。Root已收到此两片提案；W稳定公共watch DTO到来时先由同owner短串行接shared。ACP startup无byte界/原L01(新只读材料history/session-recovery/L01-read-preparation-6448.json)/网页page_steps L02仍独立，不冒NoLoad首片销它们。


## NoLoad首Node闭包已获授权，candidate2待编译

Root已授五实际路径，GuiOp/GuiNote确认定义在gui_host.rs，不碰W proto/lib。当前`capture-node-candidate-2`完整source/patch/manifest与contract已保存；未native/未Core/GUI/gen。新增Capture token为固定16byte array，Captured回应Result<CaptureFence,CaptureFailure>，NotReady/ReplayInFlight/Unavailable为明确失败，whole msgpack frame≤20KiB。bootstrap即答NotReady不排新无界pending；仅现host单loop按流序发至请求连接，不触ACP load/new/resume/control。Node枚举新增后Core穷尽match还需下一桥，不冒整体可编译。

Journal.capture只回domain ReadBoundary（canonical ReadBinding＋H Cursor），不依赖gui_host传输DTO；host转换/执行wire预算。原append已flush完整record，Capture不新增fsync/不提升durability承诺。独立published read路径与writer FD版本核对，固定6metadata调用/1byte末尾newline探测，无全文scan；pending/poison/换inode/半行均拒。fd观察是当前时点边界，不锁未来文件不变；后续source换代仍H拒旧raw/重置，桥必须识别，不把fence当immutable快照。

拟精确5 native：新EOF两项（前后相同record/空EOF、半行/换inode拒）、Journal一项（未commit/身份/poison），真实NoLoad owned host一项（两distinct token，A再请求作为有序流barrier排除B回复泄漏的half-close假绿），再原host journal回归一项。新Python模式仅NoLoad=false能力、noecho接受2prompt并各发同文本chunk；实际A4 content_begin/ClientRead/content_page读fenceA前/后分别1次、fenceB前2次，canonical原记录中富prompt与2delta完整，实际received日志无load/resume。45s worker/55s peer/reap沿现夹具，不运行用户agent。新测试未编译，不称绿；W v3当前持唯一native槽。

前端只读方向已与Root对齐：旧paint.past/live本身会继续无界增长，不能接一段有界past再无限append live。后桥应canonical journal作正文唯一来源，复用H三预算；live只控制/状态/有界wake，optimistic与persisted prompt的稳定身份须另核，不hash正文、不靠禁止发送遮盖。此Node片不销全S/H、startup或L01/L02账。Root预留W公共短片可在保存此未编译checkpoint后同树新branch接线，避免临时ignore新Captured穷尽match。


Source-only checkpoint `ac53589d9466313dc4cb4ba4139588714119fbda` 五路径/tree clean，git-show SHA=candidate2 manifest；独审源码无阻断。**未编译/未运行5测试，Core穷尽match仍待桥**，不能集成后冒整项目绿。NotReady/ReplayInFlight只有源码分支，当前拟成功host case不会覆盖这两个真实失败时序；不称三失败全验。checkpoint/bundle已双盘。当前等待W唯一native/后续shared短branch调度，不自行启动。


Node5 strict runner已备 `capture-node-1/run.py` SHA05ac39bd771b4d6f8407ef152dfdea83e4ef460caec47adaeb4ce8e145e8161d，独审准入：实际metadata→prepare Node lib testtrue→Cargo仅libtest no-run JSON→verify/require_provenance→receipt所选exe五exact各强验1pass/0ignored。300s step/600s total/5GiB/owned/source-env-tools/BUNDLE26核，noCore/CLI/gen/App。当前等待Root正式槽，未启动、未创建run TMP/MIRROR；runner单文件另镜像，不占运行时MIRROR目录。


### capture-node-1真实首批失败，未重试

Root正式授槽后实际session13345/driver64125/Cargo64186运行，真实终态exit1/96.018s；严格metadata/Node实际libtest编译/verify/require_provenance已通过。五exact中前三（EOF2＋Journal1）各1pass/0ignored；第4actual NoLoad私有worker失败，严格停，第5原journal回归未执行。source/env稳、ownedremaining[]、TMPabsent，槽已归还。原source/log/proof/result与内盘mirror保。

准确失败为gui_host_cancel_tests.rs666新增附加cleanup oracle：A write-halfclose后等待mpsc channel EOF四秒超时。到此之前token/cursor/wholeframe、A后续同token有序barrier（强证B回复不泄A）、actual A4页同文chunk1/1/2、富prompt原值及canonical2delta断言均已实际执行；closeB/reap与之后的noLoad assert尚未执行。peer真实received日志只有initialize/new/prompt/prompt，无load/resume是观测日志事实，不能外推未到达assert绿。

只读fanout.remove代码应abort＋socket.shutdownBoth，但进程已按owned终态，无活线程栈，半关闭EOF真实原因未确定，不冒fixture/产品根因。已向Root申请仅将不属capture目标的write-halfclose EOF期待替换为明确Both释放自有A，再按原closeB/reap；所有capture/最终noLoad断言保留，未知半关闭生命周期债另记，不能claim它或L01/L02被修。独审同意此scope方向，尚等Root正式改/续test4＋5许可；未删断言/加时/产品改/自动重跑。原三绿须按旧source SHA映射，不能重标成新一批五绿。


Root已正式准纯cfgtest收口，candidate3 manifest36aba0059e9a24f93cb2a2d200966cad47e611bfd423d9bb7ed44ed5b82a26e4仅gui_host_cancel_tests.rs改Write+EOF loop→Both/drop ownReceiver，原其他四/全部生产SHA与ac535/Native1同，A token有序barrier及全部capture/NoLoad/CloseB/reap断言保。续严格runner capture-node-2/run.py SHAdd6cd60cc2bb82d36ff3e714ee06b804a2547d4b51dcc666645903c538e24749，仅out/candidate/测试两exact和计数2变更，独审源码/runner准入无阻断。当前W v4先用native，未启动retry；candidate3与runner另镜像（不预建运行MIRROR）。不回填初败，不重跑未改前三/不假定新两绿。


## NoLoad首Node闭包固定 b14f73a8，5项分批真实通过；尚未Core/GUI

`ac53589d` source首checkpoint＋`b14f73a8574ecfed81fbebce0b6d119f2bb1a514`纯cfgtest cleanup修正已固定/tree clean；最终5路径git-show SHA=candidate3。独立`capture-node-final-b14f73a8/source-manifest.json`与validation-map明确映射。Root尚未集成这两个Node ref，以免Captured未接Core穷尽match让W shared生成受阻。

`capture-node-2`实际session9913/driver76618/Cargo76758，exit0/93.371s；V严格actual Node libtest producer/verify/require_provenance后，NoLoad owned host1 exact实际0/0.65s、原host journal回归1 exact实际0，各1passed/0ignored。2156source前后同、env/tools稳、owned[]、TMPabsent，native槽已归还。选中真实testbinSHA d8bb677c8fa3a9ed4b172c4d3e970863d7f13261d460dc7b23c868acfb79ee2e（360847592B），不复制编译binary进恢复盘。

与首批仅gui_host_cancel_tests.rs的11行纯cfgtest cleanup不同，全生产及其它四完整源逐SHA同；原EOF2/Journal1真实绿按精确来源复用，两批合计5actual行为覆盖，不冒一次全SHA5绿。首批halfclose四秒失败保留且根因未知，不认为被Both清理修好。现在NoLoad全部最终断言已实际到达：NoLoad能力false、两distinct token＋A有序barrier、同文chunk1/1/2、完整wire≤20KiB、原图片/文件、B Close/Gone后reap；peerreceived只有initialize/new/prompt/prompt，无load/resume。NotReady/ReplayInFlight真实host失败时序未测，只源码/类型合同，不能冒成功case覆盖它们。

全部strict/源码/host日志/收据及最终manifest已双盘，bundle s-capture-b14f73a8.bundle 6292B SHAeeea2c7b2a9153f38a4f28db8681d7f22f9dbc88850babb93561b9d8f339e83e已verify。新内容尚未接用户GUI，不能称NoLoad用户重启恢复完成；canonical正文唯一来源、origin UUID穿透、raw关联、startup/L01/L02/H搜索复制仍下一闭包。

Root当前顺序：待最终5指纹独审后，同现tree短shared branch从e409a99333188415e589970e3b010507eda81e43依次取bf11a6ea/dae8b19c/e65e8a92/1dc66584/0f911558接W三hooks/5DTO。S ref/证据保在fix/round2-history；不临时ignore Captured。现在尚未切，catalog新增git-history-updated一key已申请Root（按钮准确复用file_reread，不是不存在的refresh），业务leaf不重写。

2026-10-30后续实际：已按Root正式授权保存S分支b14，短切`fix/round2-watch-shared@6c20c13b`并完成两个shared固定ref `57de4543/72527c7b`。实际strict Core/GUI lib check、Core DTO1+HTTP2、tables4及真实bindings/catalog全0，native槽归还；完整事实见同目录[R2-W-SHARED](R2-W-SHARED.md)。S b14未提前并Root，Captured Core穷尽桥仍后片；没有为W生成放进wildcard/ignore或改变S源。当前继续等W消费者组合/独审，不据shared绿关闭本卡任何未闭S项。

## S canonical hydration 源码候选，未 native/gen

Root正式派发后在同树建`fix/round2-journal-hydration`，72527依序取已验ac535/b14成为331c65a4/82455217；原分支保留。随后按Root精确依赖取W前端fa62/9318/26fd成为3fe081fd/5feb678d/919dbeef，不取重复shared，不复制Root生成物冒新Captured类型。

当前18源候选位于`history/session-recovery/hydration-candidate-3`（candidate1/2原始源与结果仍保）。Core `ChatFrame::Captured`穷尽转换成typed `CapturedContent::{Available{binding,cursor},Unavailable{reason}}`，`chat_capture`经现view lease/FIFO送Node固定16byte token；HTTP、native_io dispatcher、GUI attachment facade单owner接线。前端32hex token只取自身当前capture，旧token/旧reader generation不重置新内容；每次安装reader更新raw epoch，原raw组件/IO不跨source续读。

正文只从canonical journal走H source/reader/window：初页以host完整EOF cursor为exclusive before，随后从同边界forward；hidden首读仍保可信cursor。wake在page在途只保一个fresh intent，IO结束后补有界读取，不创建并发reader；换capture先dispose原source。H仍256KiB scan/128entries/512KiB响应/8页512entries2MiB缓存，raw单段64KiB。live正文Note变小`ContentChanged`，权限、配置、发送receipt及状态保原；旧arriving/replayFailed/无限live正文拼接已移除，fresh/lost不再自动ACP load Replay。少量optimistic/control projection最多32条，未回答权限保可操作；这是临时输入/控制视图，完整已接纳原文保在journal，未用正文hash去重或以禁发送隐藏缺口。

既有Sending.token原UUID通过SayWith送host唯一writer。`accept`完整prompt append→active_turn持久发布成功→小PromptOrigin{turn,token,start,end}写成功后才dispatch；任一IO失败沿原poison退休。append_range用同File位置/64KiB流式buffer，不为取range重读/拼接附件，不增加capture fsync。origin不承担授权/turn去重，canonical binding提供source身份；typed frame命名`PromptRecorded`，只关联已存在canonical正文，不证明ACP接收/执行、不settle发送ledger；Core只接纳非空且end等于紧随origin record起点的range。超过单record预算的大prompt由此小origin补全精确rawRef，按原分块读取。Crash缺origin不推断发送成功；原prompt仍保原始记录，关联未知，不自动补发。普通无token的委派/CLI明确None，原断言保留。

纯验证：candidate1实际history22/mobile0，native/gen未跑时tsc2仅新未生成frame8条＋旧Wconsumer9条；两个reader/script SHA在candidate2保持同，可精确复用22（隐藏EOF/初读follow同fence、在途wake合并已实测）。新增`check-journal-body.mjs`抽实际生产private reducer验证4项：live不接正文、32临时projection/未回答权限、按token退optimistic但同文不同turn保两条、大record origin无body复制。它不是浏览器/真实Core恢复证据。

已写真实Core NoLoad worker候选`chat/journal_recovery_tests.rs`：自有Python ACP与真实gui_host/认证连接→Core双view leases/UUID发送→两capture/H页1/1/2→完整大prompt rawRef和64KiB首段→全leave后同host重开再读2，日志必须零load/resume；复用原history_bridge AdmissionTransport只stub真实cookie Begin/Cancel，不替代parser/fileIO/host。其本身尚未编译或运行，不能称成功。新增Core token/旧lease TCP与typedfailure/wake tests、Journal同token不同turn/大payload范围测试亦待实际编译。

拟最小native闭包待独审/Root授槽：普通Core+GUI libcheck→actual Node libtest Journal11+parts2→actual Core io16+NoLoad wrapper1（内部ignored worker实际1）+bounded6+原wire frame1→actual GUI native_io2+command_tables4→真实bindings staging/managed逐SHA，顶层43行为另内worker1。不变catalog不重gen，不App/真实用户agent；CLI upload构造仅必要None/..适配，CLI资产闭包不在此批。不冒NotReady/ReplayInFlight实际宿主时序已覆盖（typed投影测试与真实失败时序不同）。startup forwarding字节界、L01/040、page_steps L02、旧半关闭未知债和H source搜索/复制均继续留账。


Candidate3窄纠偏：保当前SayWith双方一致协议，按用户未上线决定不新增旧端variant/compat/fallback。reviewer确认Origin非法range的None本来即走generic自身rawRef分支，产品未改此处，仅加范围0..自身record的精确回归。新增真实manifest原子发布失败注入：完整Prompt原文保留、Origin不出现、capture/note拒并退休（尚未native，不冒该断言通过）。Origin只为canonical展示替代同token临时输入，不是远端执行成功。

Capture失败现独立可见错误＋显式重试，不把gone当状态或禁发送。NotReady/ReplayInFlight单timer指数重试，上限5s，hidden/零高度不IO，当前token有序拒迟到，离页/retire清timer；Unavailable只显式。三个生产私有closure抽测覆盖初始无historyView→NotReady→hidden不发→可见新capture→旧token拒→可信EOF安装/保wake、ReplayInFlight及显式重试/owner退休，controlled transport/clock不等真实host失败时序。源码18path候选/纯记录与strict runner一起交独审，尚未native/gen/App；Source-check历史22沿exact reader/script SHA复用，不为卡中候选变更无故重复。

## 当前 echo 对照基准：20源 retry 尚未执行

Root准将合法live用户echo关联纳当前S：只用Khor自身journal外层可信local turn（writer只在持久Prompt后设置），不文本hash/vendor名单/全量扫描/无界turn map。无turn的bootstrap/外部user chunk与不同turn同文必须保；所有原record及echo自身rawRef必须可看。生产projection目前**尚未改**。

新增单例`local_turn_echoes_keep_raw_records_without_duplicating_canonical_prompts`，实际Journal写无turn bootstrap＋两个不同本地turn同文Prompt/各echo，再经真实H read_page/parse_record。指定旧业务期待left5/right3、0pass1fail；绿后还必须达到两echo自身范围/SDK原payload及turn断言。candidate4-red2固定18源，生产prefix与candidate3字节同。

`hydration-echo-red-1`实际exec53721/driver56344/Cargo56405终态exit1/37.016s：metadata0、Node libtest编译101。只有reference.rs:372、wire_tests.rs:412两处旧cfgtest构造漏新token参数；**没有任何业务测试执行，不算复现红**。原日志/源/结果保留，source/env稳、remaining[]/cleanup[]/TMPabsent，native槽已归还。

Root准两构造补显式None、原断言不变。retry `hydration-candidate-4-red3/source-manifest.json` SHA`25efdb40c917d7421abf87cc98c1ee5ef7142a8df1482164e7c04bf49ec21dbb`共20源，原18全同，新增两cfg路径各一行；全部Journal.accept调用已定点rg核含换行writer，TCP无参accept无关。runner `hydration-echo-red-2/run.py` SHA`8881894185d774e4ab688d37850fb7561eeefe9a48bc66718392f51035badd80`，仅新目录/候选映射不同，保strict实际Node libtest producer/来源后单exact：原owned helper不改，只接受其完整原结果actual101且无超时/强清/残留，再强验指定marker/left5/right3。编译、IO、其它panic、零匹配不算红。独审已窄准入，**等待Root正式重授槽，未启动**。

原candidate3/43基础实际批也未执行，准入不冒绿。业务红真实成立后才改private parse投影并冻green候选；新增echo使后续拟44顶层＋实际内worker1，不重复一项green再全组重跑。B四导航DTO/page_state共享胶水经Root批准排在S固定producer批终态后，当前不插入S冻结源；无App/新gen。

## 当前真实 Native 收口进度（2026-10-30）

Echo retry `hydration-echo-red-2` exec96199/driver59109已真实terminal0/84.770719s：actual Node libtest producer/proof0，唯一exact实际exit101、0pass1fail，指定`OWNED_ECHO_DUPLICATION` left5/right3，case1.44s；2185source/env稳、remaining[]/cleanup[]/TMPabsent，已归槽。首批compile101不回填。红后才改private journal parser八行：仅SDK验证通过且Khor自身outer localturn非空有界的user chunk投影为该record自身Reference；原SDK原文/turn/范围保、bootstrap/null/无turn及Native不变。测试整段与真实红同SHA。

固定绿20源`hydration-candidate-4-green` manifest `2ffe94eb9ec29833d0bdb1866f8ee0c8544358f5fb698eff81e35ef65140e2dc`已独审。`hydration-green-1` exec94322/driver69280真实terminal1/276.300304s：普通Core+GUI签名check0、实际Node/Core producer proof0，Node Journal12+parts2、Core IO16+真实NoLoad1（实际内worker1强数）+预算6+原wire1，合38顶层＋inner1全0；同文两echo raw断言到达，Core多view/fence1/1/2/大raw/全leave重挂零load-resume断言均到达。2185source/env稳、owned[]/TMP净。GUI libtest Cargo0，但strict generated output provenance失败，后NativeIO2/tables4/bindings未运行/未发布；**整批不是44绿**。

GUI具体旧OUT exact`debug/build/khor-gui-a7d478be9174cb97/out`含282文件/6,450,624B，其中279成员mtime/ctime前于prepared；marker当前。Root/独审批可逆retain，`hydration-green-2`实际把inode36248101全282 SHA保留到evidence、原位empty，fresh GUI Cargo0；其自身生成校验通过。该批exec73676 terminal1/31.065586s再停于catalog/detect依赖生成：实际GUI用f711f046/525cb316 OUT，而Node/Core真实priors覆盖的是2b0c5b/50dd的另一variant，prepare reusable判断不触这两build.rs，postverify严格拒；没有改helper/手写receipt/伪mtime/执行余6，source/env/owned/TMP仍净。

Root选既有窄API续法，`hydration-green-3/run.py` SHA`f9aedc0a14e0c02024624ca1926de411bd5da0a71e801729490440623ef91616`已独审准入，plan SHA`973caee0cb03ab95621018f9246622a7d2a5214dc7166a55eba9a079362ff873`：新GUI inode36272260/282逐SHA/lsof identity复核后再次可逆retain；由actualmetadata取catalog/detect真实lib target，no-prior `prepare_build(apply_refresh=True)`只作source hint、不执行standalone或声称provenance，完整refreshed记录保（保守graph可能多源）；随后原GUI prepare用真实Node/Core priors，actual strict GUI producer→仅NativeIO2/tables4/实际bindings。旧38＋inner1分列精确来源reuse，不重跑。不插B公共源、不Catalog standalone/CLI/App。当前**exec78796/driver93532已实际启动**（最初metadata Cargo93550），尚待真实terminal；不以running.json当完成，也不重启第二实例。

后续GUI消费者真实浏览器仍须保旧已知正文/阅读锚点、旧raw来源不可串到新reader、合法多chunk呈现等断言，不凭Native闭包称全部GUI恢复验收。Full source搜索/复制、startup bytes、L01/L02与半关闭未知债仍待原项；B共享四DTO窗口在本固定批terminal后排队。

## Hydration green3 terminal：仅真实编译成功，strict provenance拒绝（2026-10-30）

exec78796/driver93532，metadata93550与GUI Cargo93645均真实exit0；整批137.562527625s、green=false。错误为 `Fresh first-party artifact lacks a matching producer: khor_acp`。剩余NativeIO2/tables4与bindings未执行、未发布；38顶层+inner1仍仅按green1相同2185输入分列复用，不回填为本批测试。source/environment前后同；remaining=[]、cleanup.errors=[]、TMP已不存在；driver/Cargo现场均gone，native槽已明确交Root。

精确282成员GUI OUT可逆retain及两existingAPI no-prior refresh hints均按已审脚本执行；实际完整hint记录含catalog lib/build/core lib、detect lib/build，不称仅触两文件或独立build证明。旧false green1/2/3及retained输出原样保留。无helper/gate/mtime伪写。全量来源诊断已交reviewer，停止逐包producer补试；本作者只读 `fresh-unit-diagnostic.json` 用frozen prepared keys扫描全部fresh firstparty，唯一未匹配为khor-acp同key/variant但实际538749…与prior164db2…的artifact路径/内容不同，最终方案由独审核全图。

完整S20 source/HEAD/tracked patch/未跟踪源全文与旧169gen快照：`history/session-recovery/hydration-source-preserved-919d`；旧gen明确无Captured新类型，不能冒已发布本轮协议。B shared source后续单独提交，不混入这20源的原证据或旧batch。

### 独立源码checkpoint与后继B窗口

S20已完整固定为 `d83d9b73ff78717f54599f6ba73fa5f2566e59bd`，分支 `fix/round2-journal-hydration`。这是明确WIP源码保存，非Root集成、非44全绿、非新bindings已发布。Root允许当前false归档后转功能推进；同worktree新分支 `fix/round2-browser-shared` 的后继B差异独立提交。不得用B后继当前全树内容核green3旧输入；应从d83/gitshow、S20全文快照与当时2185 frozen frame核其实际链路。Provenance全图诊断由reviewer负责，禁止继续逐包hint重试。待B叶与共享胶水稳定后只领一次组合真实producer窗口；旧S38+inner1保持原来源映射。


### Post-H platform priority: full-App S recovery read-only route (no runtime)

H UI/browser B7 is now actual seven groups/fixed8f+test410; production Mac native IO/OS Save preparation comes first. A read-only S follow-up map is saved at history/session-recovery/full-app-readonly-preparation/plan.json and mirrored s-full-app-readonly-preparation. Current DetailPane routes uncontrolled/failed GUI to ObservedSession/session_content/H journal; active GUI attaches through Begin/claim, actual home owner/cookie connect, then own-token Capture/EOF→H bounded reader. Explicit resume remains separate Node.reopen_gui_as and rejects existing native writers. No automatic host spawn/load was found in current Core chat_open attach.

Smallest proposed actual GUI recovery proof after input's startup/EventStore interface is stable: one isolated NoLoad ACP fixture via existing public CLI open --gui -d -- python argv, App accepted synthetic1→normal CUA Quit→same HOME/store/current App reopen/same Khor ID→same host PID/birth/native ID and original canonical text→synthetic2 exactly once. Assert initialize/new unchanged and load/resume0, no implicit resend. Existing Core all-view leave/reopen is not this full-App proof. Abnormal App crash, owned host death, machine reboot, far offline and Shell new-instance boundary remain separate acceptance requirements. No Node/ACP/gui_host/journal source or M TerminalPane changed; no new runtime or RPC/gen authorized by this read-only plan.


### Replay retirement shared source checkpoint (2026-10-30)

Root approved the exact cross-layer contract in the read-only `replay-retirement-analysis/RETIREMENT.md` d92b6479: retire unused Replay/History/HistoryEnd/ReplayInFlight, keep advertised Facts/Agent.replays, initial supported Load/Resume, canonical Capture/H, input/Stop/Close and real shim history assertions. Input retains Node/ACP single ownership and its startup231e+27d frozen test candidate; it confirmed these enum removals but has not inserted retirement into that native batch.

Author preserved `fix/round2-browser-shared@410c1f1b` and created the same-worktree `fix/round2-replay-shared`. Seven-path source checkpoint **47fefbcf286647f254c019656b37385a3b2a1017** removes shared HTTP/native/API facade and dead frame/reason hooks; no Node/ACP/gen/canonical storage file changed. It is **not independently buildable with the still-retained Node enum variants**, not Root-integrated, not behavior-tested and not actually regenerated. The current codec test keeps valid Agent/settings/Note/Turning/Turn/Ask/Answered/refused/Gone assertions without a parallel OldNote model. FIFO still uses three real controls and written completions, now fixed-token Capture instead of Replay; upload assertions remain and its exhaustive event map includes current Capture.

Source wire inspection found existing rmp-serde variants encode names rather than Rust declaration indices; positional fields remain. Canonical JSONL Record/manifestVERSION1/binding/source-cursor formats are unchanged, and this source patch performs no migration or host/runtime/data deletion. No old-end compatibility is added. Pure syntax-only rustfmt(stdout) for five Rust files, TypeScript transpile syntax for two frontend files and diff-check exited0; no Cargo/gen/browser/App ran. Evidence and full before/fixed source+patch are double-mirrored at `history/session-recovery/replay-shared-source-1`, manifest **f567d56c6b679ed3fa1f9702fcec792d90a1f6a0ab9fccef57a5ad4a6cb635cc**, report **a899f0bbf34d2ae878315af70de525ba40f7a22439c32ea996232a4653009623**.

Remaining before an actual current green: migrate the legacy browser13 user capabilities to Capture/H rather than delete scenarios, finish input-owned Node/ACP and actual CLI shim migrations, then compile exact cross-layer source, current codec/FIFO/capture/upload/command-table tests and real bindings/GUI recovery checks. Prior H11/export6/browser7 are not relabelled as proofs of this new source. No second replay archive, compatibility stub or catch-all enum arm exists.


Shared retirement fixture source is separately fixed as **ee341cbeef05f36031bbe65b59687f4b64dd923a** (parent47fe). The package/script now says chat-recovery; thirteen old user capabilities map explicitly to Capture/H and live controls, not obsolete History/HistoryEnd frames. The existing attachment transport helper recognizes current Capture. A held claimed read is only stopped by Cancel and drops in its Promise finally; unknown APIs reject. Actual tab retry, old lease/token isolation, body preservation on read failure, true empty canonical source, control-result separation and final capture before Gone remain in the thirteen planned browser cases. These browser cases have **not run**. Typed capture closure extraction actually passed1/1, keeping NotReady/hidden/old-token/wake/Unavailable/stopped assertions and removing only obsolete ReplayInFlight input; four Node syntax parses/diff-check0. `replay-recovery-fixtures-source-1` full source/patch/mapping are dual-mirrored, manifest **ec9b851b8a3b4d90b2f571767d37af3845648550cd693862e7f66abece7c5161**, mapping **5de370498cb6154cdf7fc3501b4403a55dd6ac1cf97b3b6e5c3bdda57d8848f9**. Remaining smoke/ACP/read/preview legacy inputs are still explicitly pending; this new source is not a complete test migration or independent native/gen green.


GUI capability greeting retirement source closes the necessary Core PTY negative case in **7d123593b9f06f359eb6820b8b80853f029e20da**: `terminal_current.rs::core_rejects_noncurrent_welcomes_without_registering_an_attachment` retains old GUI/1 refusal and adds current GUI/2 to the same exact refusal/zero-client-op/no-attachment assertions. Normal PTY protocol5 positive case is unchanged. This is not an enum ordinal fix: rmp-serde serializes variant names; input's GUI/2 represents the removed capability/frame set. Only one test-array line changed, source-only syntax/diff checks0; no Cargo/native behavior ran. Full before/fixed/patch mirrored at `replay-pty-greeting-source-1`, manifest `687a0037d7e6091cf920012d22a2e02df58e6897717fa2b487d2acea64597063`. Input owns Node GUI-positive2/old1 refusal and matching host.rs PTY negative case.


### Remaining legacy fixture source migration

Root granted the new private `fixtures/canonical-chat-routes.mjs` only for fixed scenario DTO/HTTP lease plumbing. Source **9698149dc89374c23827a2a0aa2a14fe9ed360d3** (parent7d123593) fixes that helper and six old scripts: check-acp/read/ai-elements, ai-elements-preview, two narrow check-face GUI feeds, and the real-backend smoke Replay sections. No production Node/Core/native/API/gen changed in this fixture patch. All old History/HistoryEnd/chat_replay operations are removed from these six scripts; unrelated terminal/IME replay meanings stay.

The five controlled scripts now use scoped Begin/open/Capture with their actual token and fixture EOF, static H pages capped128entries/512KiB, claimed Cancel stopped until real reply fulfill/finally, and unchanged SDK/body payloads handed to the production historySource/reader/paint. It is not another normalizer/SourceIdentity/hash/sort/body fold/cache/worker owner. GUI fixture rows explicitly supply current can_control=true facts only where the scenario is a hosted GUI; other terminal rows remain unchanged. Original tool/status/output/diff/permission/plan/config/link/readability/geometry assertions remain. The obsolete NoLoad empty-bracket warning expectation retires in favor of positive canonical NoLoad body plus no false warning, keeping login and ordinary-agent facts assertions; optional missing-model assertions remain current-protocol checks.

Smoke does not import the helper or substitute its real backend: the visible tab reconnect menu must trigger actual own-lease/token Capture four times and restore recorded past exactly once. Lost control cap still must trigger Capture; since600records can evict old DOM/cache entries by design, the old positive source record must appear exactly once via production full-source Find while latest content remains after closing Find. The strong existing original-past setup assertion is retained; if a future actual run finds the old stand-in only supplied it through removed Replay, it needs actual supported initialLoad/H setup, never synthesized history or a removed assertion. No whole-smoke run was started.

Six JS syntax checks, TypeScript preview syntax-only transpile and diff-check exited0; helper import/construction also succeeded without running production behavior. **No browser/native/gen/current noEmit green is claimed.** Full before/fixed source, patch and semantic table are double-mirrored in `history/session-recovery/replay-remaining-fixtures-source-1`, manifest **cbab17e0fff902fa795655d5081c0432405fe38943cfbc9b01c3053abb4ed483**, semantic map **1938c14c6d604fb8ad7c16414be0cb47d084841b335a28aaf64493e26ee86d15**. Input's matching Node/ACP retirement source4dd9 has been received; Root must combine fixed sources and authorize exact checks/producers separately. A5 automatic App sequence has ended and is not restarted by this source work.


### Current retirement source: actual generated types and frontend (2026-10-30)

Root255 combined source was actually built by the focused supported no-embedded-agent `khor-gui-core/bin/gen_bindings` producer in `integration/replay-types-2552334a-1`: exec82947 terminal1/94.666s. Metadata/build/strict proof/stage/managed publication all exited0, actual177 bindings plus catalog179 generated items; actual Node build-script output confirmed no embedded-agent cfg/env. The complete noEmit then correctly failed on two unreachable History comparisons (TS2367), and Vite did not run. Original false/source2331-stable/owned-empty/TMP-absent and its26+ files are preserved; it is not relabelled green.

Minimal two-line source **67772dff02e131bd041d3f5abbd0a9c489c92793** removes only those dead comparisons in history-paint/updateOf, retaining all Note painting/title behavior. Non-author review passed and Root precisely integrated it as **16eec79ead2152b0b1afab08597cd782b5929084**. Only these two TypeScript file hashes differ from the original actual generator's complete2331 frame; all Rust/Cargo/generator/tool/helper/catalog inputs and actual177 stage/managed bytes remain identical. No new Rust provenance is claimed for the changed frontend.

The authorized continuation `integration/replay-frontend-continuation-1` exec11451 actually exited0/15.224214625s: full project `tsc --noEmit --project`0 and direct Node Vite0. Source2331/env/tools/helpers remained stable, Root clean,179 actual generated files/278 actual assets, owned[]/TMPabsent;26 selected files are dual-mirrored. The frontend receipt **087f4263b5b58a20525a01f500187f06ad2458d35d8e66b51df5352fb9113817** explicitly separates old generation_head255/proof from new frontend_head16ee/types/Vite and contains full relative generated/assets/source dictionaries. Result **1bc6f1879d9164d68db1d6f5a7ca4e029b9c09051036807f3a2ac99dabb1c8e0**; dual manifest **3e33681176bf037d2ea421398e0af7618b292f2ba9128d89adc0e1bacc63ea16**. This is focused frontend/producer evidence, not production embedded-bundle App or behavior proof.

Next recovery13/current consumer browser, current Core FIFO/codec/capture plus changed-target compile closure, input's separate ACP/Node/CLI native stages and current full-App S recovery remain distinct. No App/Inspector automatic retries, clipboard/OS Save or human IME result follows from these checks.


### Current canonical recovery13: actual production browser

After input explicitly returned the final private-copy archival slot and Root's conditional grant, `history/session-recovery/recovery-browser-current-1` ran as actual exec98901 (driver64110/Node64127) on frozen Root16ee with actual frontend087f/179gen. All **13** migrated scenarios actually passed in Chrome: failed-empty/failed-partial/cancelled-delivery/empty-error/successful-empty/failure-then-success/live-interleaved/ordinary-control-error/successful-result/gone-reconnect/gone-flag-reconnect/retry-mid-capture/successful-capture-before-gone. This executes production ChatView/Capture/historySource/reader/cache/paint; only transport facts/leases are fixture DTOs, not another normalizer.

Actual process exit0/no timeout/no forced cleanup. Source/gen/environment/tools/browser-tools stayed identical; fixture removed, remaining owners[], cleanup errors[], TMPabsent.21 selected files are mirrored and independently byte-checked. Result **23b3e5797db038700a226067f8d4bfbc0785a57353d0ef6eeb2ddc3f8ee9d524**, report **6aaefd1ec9dafba7bbbdd6370380a30d72205bb58299b636a81ca6c5b529bc64**, dual manifest **db9ac1c86f3aaba56d021ef0e2c86f362a47c098a6260d2542b5bc541009aade**. The original Replay-specific scenario contract has migrated without deleting its current user-visible assertions. Browser slot returned; no following consumer batch auto-started.

This is not WebKit/system Save/clipboard/IME/physical-device or complete full-App S recovery evidence. Remaining changed consumers, real backend smoke initialLoad setup, current Core FIFO/codec/negative target and full-App/hardware combinations retain their separate pending status.


### Current consumer checks: original false results and fixture source migration

Consumer group1 actual41905 exited1/41.8616s: ACP first row click timed out before its positive marker; Read was not run. Root source16ee remained stable and all owned/TMP cleanup passed. Source inspection found both fixtures lacked explicit LockFacts while the production hook correctly fails closed. Two-script fixed **2cc2406346b9ce375d27e37f4fefe78832456243** adds only legal six-field private no-PIN responses; it does not change production auth. AI4 same bootstrap is isolated **28461e2e9cce3325cf0e932d861282f85695f8c1** and preview one switch-case is **0e18b68b4ea83b07ddd0d010ac812a2fbceeaecd**. All source-reviewed, not evidence those consumers passed.

Group1 retry actual48444 exited1/59.2497s with actual four old mode probes readingnull and later obsolete data-chat-add timeout; Read again did not run. Rootfbc strict/source/gen/tools/cleanup remained stable. These failures are not rewritten. The current accepted SessionPreferences owns header permissions/model/identity/context, not the retired composer controls. Root approved an explicit old-to-current semantic mapping (artifact acp-current-controls-mapping-1, b5c56422), including actual checked-radio name/id/modelLarge-b/fastcheckbox/default and none negatives, identity/no fabricated model, actual visible32% for original320000/1000000 SDK input, no usage Fact when absent and noUSD. Raw hidden attributes are not fabricated from the fixture. Tool/NoLoad/login/body/file/visual thresholds remain unchanged. Fixed source **1ff683ffbe13cdf90ec92647412e301001ee5066** is source-reviewed, not yet runtime green; wizard contexts also receive legal lock/hosting-device/matching saved-answer home facts, with original four-state/ask-count/ID/geometry assertions retained.

Independent full Read actual7229 exited1/17.5024s. Agent text/ink/measure actually painted; invalid tool fixture had no required toolCallId, and old chat-kind machine row did not reach MachineChat. Source **9a982fc7840488f8bcd852aed0828cf00c1318b3** only makes that tool and broadcast/wide row legal. On combined Rootffa, full Read retry actual96347 exited1/17.984s: the tool/blockquote positive now passes, but machine probes still had no column. Source inspection found old machine-log message lacked required ChatMsgRow.to and other current fields; original runs had no retained DOM/network snapshot, so this is not asserted as their sole runtime cause.

Fixed **901e5dc308791dfc464ebdbced40ce8774a3a95f** supplies the complete legal synthetic pure-text ChatMsgRow, removing retired word/delivery; original text/time/from/probe and all thresholds are byte unchanged. Failure-only bounded DOM/HTTP/console evidence goes to the existing log to preserve future observations, with no new service/owner. Manifest **1d50034449e31b835569f46a01136d9fe454f09753c2f766c7aec8a80651907b**, candidate8a623b6f; full before/fixed/patch mirrored. Original96347 result **59789c09e058568338c4fd872b403faae0671ab5ab8a3a8907602494372b37b3** remainsfalse/strictclean. All consumers still require actual appropriate follow-up on the combined source; no whole smoke/face/native/App result follows from these source fixes.


### Root337 independent consumer batches — actual results (2026-10-30)

Current Root337389902092ef2caaed015c61ab389601b85422, production UI/Rust unchanged from16ee; five audited test/helper files differ from frontend087f, 179gen/278assets unchanged. Full Read exec62914 actual0/17.612198792s passes its original complete gate. Full ACP exec74275 actual1/118.113258125s: login/mode/settings/optional/wizard states and geometry/idle/press positives observed, but three attachment positives all returned0/0, followed by obsolete data-new-session-add-agent30s timeout; later script assertions not reached. AI4 exec13652 actual1/5.237710875s: first context fails explicit collapse survives chunks true!=false; four contexts not completed. All three source/gen/env/tools/browsertools stable, no wrapper timeout/forced cleanup, remaining[]/TMPabsent; reviewer verified dual manifests23/22/24 entries. All originalfalse retained. Aggregate results b6aee6ee7371e76a86a57ffd6ca6872bccaa3c29e059aa98c809f2a45cc5d498 at history/session-recovery/consumer-three-current-337-results/results.json. No App/WK/Save/clipboard inference.

Root's actual production-function red independently proves per-Note boundary splits legitimate ACP hel+lo into two items while applyAcpUpdate gives hello; this is grouping evidence, not AI failure's sole cause. Minimal typed read-provenance/grouping design is proposed separately; product edits not yet granted. ACP paste now uses actual machine upload/path+optional inline-image semantics, and registry entry now lives in Launch management; stale assertions must be mapped to current supported user contracts, not blindly reselected or removed.


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
