# Q037 R1 GUI entry / stream fixture：独立源码审

Reviewer：node_test_review，非本片作者。2026-10-27。只读当前冻结源及既有证据；没有Cargo、测试、进程操作、源码改动或commit。Q052候选02与其plan02 WIP不在本审范围。

## 当前结论

**三个冻结路径的最小适配及最终control03独审通过，当前未发现阻止这4父定向验证的源码/控制阻断；执行仍须root授唯一槽。尚未编译/运行，不把入口修好算GUI工具已completed，不把清理源码意图算实际PID/目录已退。** 原control01/02问题和SHA作为历史保留，当前最终结论以control03段为准。

先读STATE/PLAN及Q-037-remote-r1当前入口契约。CRG先行返回main@8704a503，main_if_requested只给旧console相关歧义符号，不覆盖当前MCP/GUI未提交源；没有据此认定0影响或覆盖，通过实际source回退。树无.codegraph，未调用CodeGraph。

实际树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，HEAD `8242b5bda383b456a0122fc2613ca4e274620187`。候选 `Q037/remote-r1/gui-entry-stream-candidate01-20261026T223943Z` 与内盘 `recovery/inflight-source/Q037-REMOTE-R1` 同名目录共28文件逐SHA一致；全部22源与完整2350inputs均逐SHA等于当前。

三个delta路径与SHA：

- `apps/gui/core/tests/khor_agent.rs`：`468c946b10063e45a09449f3def8fc2a22f60d1c58f9674dc304e7006288dda8`。
- `crates/node/src/khor_agent_output_tests.rs`：`e11682a901ab6f733fde4be42612dab6578d84784b6ac5919c8d8404ec790b78`。
- `apps/gui/core/Cargo.toml`：`67176f3b32b9f9117cd58b511639fda1ca675c21feb266d099f5c20e395da036`。

`three-source-delta.patch` SHA `de3066e6f522c6569e2212011f4c70a943926a605259eb157adccfdd74dcceaf`。完整R1生产未变，不重新声称已覆盖R2或旧全部106组合。

## GUI自执行入口与选择器

Cargo只为现khor_agent target增加harness=false。custom main（test文件24–110）首个执行动作就是现生产 `khor_node::mcp_worker::main_if_requested()`，Some即exit；没有在此前println、接受测试选择或启动HTTP/model fixture。生产hook实际要求第一且唯一arg为 `_mcp_worker`，读取真实stdin Request并只写协议result；多余arg返回失败。因此正常Node worker工厂re-exec当前GUI integration exe能进入同生产协议，而非旧libtest的unknown-option入口。接受日志不会污染worker stdout。

普通选择器独立判断：

- 只有单独 `--list` 才列精确两个父并exit0，没有运行父；runner必须把它当入口控制项，不能记通过父。
- `--exact <完整已知父名>` 精确匹配；缺名、unknown、裸位置过滤、`--list`混其它选项、重复exact、重复受支持选项均exit2，错误走stderr；不会成功running0tests。
- 仅允许 `--include-ignored`、`--nocapture`、`--test-threads=1` 各一次；空选择执行两个现普通父，exact执行一个，串行。无ignored child被列为新父。
- 每父catch_unwind保持原assert失败为failed并最终exit1；panic hook仍输出stderr。汇总真实chosen数/成功失败/filtered，无“任意Err算通过”或吞panic。命令必须用此入口支持的`--exact NAME`顺序，不能照搬`NAME --exact`。

建议runner实际控制项：--list准确双名/无业务输出；unknown complete name、裸未知arg、重复exact和list混arg都精确exit2且无成功父summary；`_mcp_worker extra`非零且无acceptance stdout；真正worker成功由实际network/tool父的completed oracle建立，不能只用空stdin的错误响应当worker成功。这些控制不另计普通父。

## 原GUI oracle保留

从新文件去掉新增main/refuse部分、移除原两处#[test]并还原一段入口注释后，`struct Fixture`起所有原helpers和两父函数与prior output-fixture candidate01逐字相同（独立比对，不只引用作者scope-verification）。以下核心仍在：

- loopback真实模型HTTP、Bearer精确选定account credential、真实MCP devices工具completed、agent message/network ready以及receipt不泄漏key。
- 同token幂等不重发model、同token改文拒绝、第二turn上下文携前一task/工具结果、准确工具命名集合。
- close实际权限blocked，configure不清approval、stale answer拒绝；人等待32秒且model请求仍为3，原approval id保留；拒绝后done与operation declined。
- 第三turn运行中model切换仅影响后续配置、Stop实际cancelled且error=None/stderr有取消文本；cancel_status/cancel_retry仍经GUI真实路由核run和durability JSON，不伪称远端ACK。
- restart父仍要求running receipt重开后interrupted+ended_at，并且没有重新启动Pi session目录。

没有减permission32秒或改工具completed为允许failed/任意终态；没有将GUI API改为mock或让runner越过这些断言。旧GUI completed失败原日志仍需保留，当前源适配不能反推历史原因已由runtime证实。

## output stream ownership/drain

`drain_cancellation_owners`仅提取原failed-sidecar父已有6秒有界等待，保持`release_test_root`真实结果为Ok才继续，5ms轮询、超时assert失败。该函数会禁止该root再调度、pump/reap当前owner；任何step/metadata仍在即Err，直到实际退出才移除该root排队引用。因此不是sleep替代join，也不吞清理失败。

streamed_sidecar_child现使用真实Node的device_str为Saved.record.owner，符合已存在R1校验；没有伪造receipt/task绑定。execute线程真实join后才drain，再读原checkpoint/final receipt并断言。原stream父和8个case定义区间独立逐字相同；所有分片key尾部屏蔽、stdout/stderr redaction、输出上限、UTF8、live checkpoint、done/failed/eof/cancelled及完整事件写盘oracle没有变化。8cases仍一个普通父，ignored child只为其内部进程，不重计。

现子进程20秒fuse、checkpoint5秒与新有限drain6秒均仍有界；失败日志必须保存。没有更改MCP预算/生产调度或增加后台服务。

## 运行清理与证据要求（仍待实证）

GUI原Fixture Drop依然stop HTTP thread+remove root；成功路径join HTTP thread，但没有测试专用release_test_root入口（GUI集成依赖normal Node，cfg(test) helper不公开）。`cancel_retry`返回metadata结果后仍可能有有限后台owner，因此源码不证明删目录时一定零owner；这不是本次偷偷放宽的条件，也未仅凭可能性定新增阻断。下一runner必须实核所有观测到的worker PID/PGID退出、源/binary稳定、自有TMP未被迟到owner重建；若活owner或残项，不得用model/GUI父exit0宣称cleanup齐，应保原残项/日志并停报。若实测确有晚写，再在真实生命周期边界做最小测试隔离修补，不能直接扫目录遮盖。

待作者冻结的runner应仅stream父1+GUI父2+paired父1，总4不同普通父；GUI custom main父无ignored helper充数，stream8cases只算1，paired真实BoundSend/Cancel oracle不变。独立核：当前2350全输入、必要actual test/CLI产物来源/manifest路径、当前artifact fresh或同源已编译沿用证明、精确父名字+passed/failed、STEP/TOTAL/MIN AST和实际引用、jobs1/offline/sharedtarget、有限TERM/KILL/wait/drain、before/after actual binaries、PID和TMP、双盘原log。不得复跑/累加前6或前15充数；本卡不执行。

## runner01 先行suite/selector审（尚未授最终执行）

已读candidate内 `run-gui-stream-four.py`，SHA `719a32e157e6d0a681856ad73649230a84ed123707bb11dd70b3e151dad0c55d`，及remaining-exact/runtime-prerequisites和实际被AST提取的原R0函数。这里只做文本/AST读取，未import/执行。

- suite确为stream普通父1、GUI两个原普通父、paired Bound普通父1，共4唯一。命令是`--exact NAME`，custom main接受；每组最后summary必须1 passed/0 failed/0 ignored。stream8case/ignored child不能充数，旧6/15与R0不重跑。
- 控制组真实运行--list精确双名、unknown exact/unsupported option精确exit2且无业务summary；空stdin `_mcp_worker`要求一个JSON Err且无acceptance stdout。这是协议入口负对照、0普通父，不冒真实工具成功；工具completed仍由原GUI父断言。
- 只复用root保存normal CLI/stub/paired binary manifest，逐SHA/size/mtime核保真；旧GUI/libtest/agent-wire仅列unused历史copy。新GUI与Node libtest必须本树Cargo JSON的实际executable+fresh=false。GUI目标harness=false改动不能复用旧GUI binary。源/ignored frontend原件核对为existing bytes，不调用frontend/gen、不声称本轮fresh producer。
- 旧R0管控只AST提取sha/inputs/free/check_inputs/process_rows/run六函数，重命名实际STEP/TOTAL/LIMIT为300/720/5GiB并以3IO门运行；不执行旧模块顶层frontend/Cargo/R0流水。动态ast_guard通过runtime free调用继续核常量，不只是写份静态AST收据。
- suite/selector/helper与复用选择未见新增阻断。root已识别并要求control02补：leader退而PGID仍活的drain、fallback kill不得留passed、TMP完整inventory与all-owner-gone/lsof0空目录rmdir、source false/last group活必须影响exit。runner01尚缺这些门，**不授最终可执行结论**，等待owner冻结control02后delta审。原01保留，不拿后版本覆盖历史SHA。

## control02独立终核：native leader已退仍有同PGID成员的漏门待修

已读 `Q037/remote-r1/gui-stream-control02-20261026T225231Z/run-gui-stream-four.py` 完整01→02 diff，SHA `6731e6ad8c60815e9d3c14a0afcf76dae410d495087550d74dad67e3719fb2e1`。没有执行/import它，也未操作进程。源码22/2350与4父suite无变。

02已闭部分：build stop_owned:49支持leader退但group_members仍活时TERM/KILL有界drain；任何finally fallback都加error；sourcefalse/last build group活影响最终state/exit；五个actual private exe（cli/stub/paired/new libtest/new GUI）执行前后footprint；TMP完整lstat inventory，regular/unknown原件保留，仅all-ownedgone/no-errors/lsof0的空目录才逐级rmdir；guard观察或copy失败均不能绿。选择器/真实worker负对照和四父集合与01相同，本轮未扩大运行。

**仍有确定控制阻断（非生产缺陷）：native cleanup还按leader PID而非PGID查活。** 248–256遍历reused R0的owned_groups，250以rows.get(old leader pid)决定是否kill，256也只收旧leader PID仍存在的行。R0 run采样记录的owned条目本身只含group leader。故一个已观察native PGID的leader先退出、非leader child仍在该PGID时，不会进入fallback，也不会列remaining_native_owned_groups。若child是普通sh/sleep而非五个actual exe，footprint不能补救；若其cwd/openfiles不在TMP，TMP lsof0也不能补救。于是可能在实际仍有自有group成员时保持passed。build的stop_owned补丁不作用于这段native收口。

最小修复仅control：对每个已记录PGID，在当前rows中找所有`row.pgid==knownPGID`成员；有成员即按已观测自有组做有限drain，leader缺席也不能跳过；最终remaining同样按PGID收全部成员。任何强制retirement仍error/non-green。无需修改source、四父suite或生产，原02保留、新control另冻。建议只用静态成员矩阵或独立小control自检覆盖leader已退/nonleader活，不运行生产test来掩盖控制错误。待该窄delta闭合后才给可执行结论。

## control03最终窄delta通过；审卡写权交root

实际 `Q037/remote-r1/gui-stream-control03-20261026T225801Z/run-gui-stream-four.py` SHA `32389755d0cee466b2f266621f8e6720d6e88e8b0078196a0a65939ecc0bd74f`，外内3文件逐SHA一致，02原件不覆盖。只读02→03完整diff，确认仅native已观察PGID全成员remaining/drain：

- AST ScopedNative将原R0 run的remaining改为按owned.values中的known PGID取rows全体成员。因此leader先退/generic child仍活会使本父非绿，不能继续下一父再等finally偶然收拾。
- 只在纯AST环境实例化两个transformer并变换原六函数，admitted/ps_timeouts/remaining_checks实际计数均1；没有import整个runner或执行旧R0顶层。使用leader缺席、generic member存在、另有无关组的字典矩阵，确认仅已观察组成员被捕获。
- finally也按已记录known PGID全成员决定fallback，TERM5/KILL5有限drain后再次全成员核空；无需旧leader仍存在。任何发生fallback均加error，依旧不能绿。没有扩大到未知PGID或按宽前缀杀用户服务。
- control02其余已认可的source末门/五actualexe footprint/TMP完整inventory仅空dir+lsof0/copy异常非绿、300/720/5GiB/3IO、精确四父和真实worker选择器均未变。全部2350输入最终重新逐SHA=current，22源冻结保持，suite4再次核唯一/原名不变。

**最终结论：control02已确认漏门在03闭合；当前source+suite+control03未见新增阻断，可由root另授槽执行stream1+GUI2+paired1。未Cargo/未执行真实进程/未取得业务绿；原GUI工具失败及cleanup未知必须由本次actual raw/PID/TMP结果判定。**

本卡最终写权现在明确交还root，reviewer后续不再修改，root可提交。下一角色仅在明确归槽后作为Q052作者执行已独审plan04，不自审该实现或抢槽。

## Root：control03实际4父通过、整run仍清理红（2026-10-27）

原source/control审记录保留。`gui-entry-stream-four-20261026T230149Z`实际GUI compile63.632s、Node compile51.458s；stream、GUI network/tools、GUI restart、paired Bound四父各1/0/0，原permission32秒/真实tool completed/认证绑定Cancel oracle均未降低。

整run exit1：后台metadata在GUI fixture移除Root后重建14 regular files/1752B，17PGID/22PID已退、0forced、lsof0、原22源/2350inputs与实际5binary稳、60dual核齐。Root `root-gui-stream03-runtime-audit`独审原raw/manifest/binary与cleanup red。S22 Node/GUI actual exe已copy runtime-artifacts/R1-gui-stream03。该晚写实证推动生产owned close API；不是改cleanup门或手工清目录后称本run绿。close01/02/03是后续不同source，旧4父不能证新API正确。原r1h残项仍保留待单独精确后清，原失败收据不回填。
