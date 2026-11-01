# N 状态查询与文档同步并发

状态进行中，owner /root/round2_input。当前授权/固定源及实际结果见文末；以下为最初排队记录。R2-004既定范围：集群发布后hinton首次--all、turing settled --all各一次document transfer state busy；后续成功不抹原失败，不证明已修。首空闲作者槽再实施，root不加第三实现线。

## 已完成一次只读定位（2026-10-29）

main图谱对完整错误文本/这条调用链无有效覆盖，fallback定点查源码。仅证实同一字串来自doc_transfer_state::Error::busy，不能据字串判断实际争用者：

- node/doc_transfer_state/io.rs::gate 的进程内LOCAL.try_lock或实际gate文件try_lock会Busy；RootPermit/RootGate还有同线程锁序拒绝、admission等待/配额等独立原因。
- node/link_doc_transfer.rs::on_worker已有单重型WORK队列＋最多8个WAITING，WAITING满也Busy；State只在blocking step内存活，不能直接改成跨await持锁。
- doc_transfer_state::Context::apply devices遇needs_recovery也Busy。
- Node::open/open_async/open_hosting进入RootPermit，trust::recover_pairing_before_bootstrap在确需恢复时另开State；普通查询是否在此失败还是其同步RPC阶段，尚未用运行证据区分。
- 既有metadata_contention测试证明真实锁可产生typed Busy且释放可重开，仅此不证明原--all场景或修复。

证据对应集成d4232d11源；本次没有运行真实用户home/机器、没有并发fixture或修改代码。

## 派发时的最小行动

先在私有home和明确进程/锁归属的夹具，对一次只读查询＋后台同步取运行证据，保原Busy与当前阶段/锁持有身份和时间线。入口先区分constructor、metadata step、root admission、worker quota，阶段诊断保typed Busy/Storage差别且不输出敏感配置。调查预算为一个固定并发夹具和一轮对照，未复现就记录缺失证据/退出点，不无限放大到全同步栈。

证据证明根因后才选择最小修复：普通只读快照或已存在的有界队列/锁序契约；不盲目sleep重试、不吞Busy/把错误当空列表、不削弱文档事务/配对信任门。修复用原夹具验证并发成功、超载/取消仍如实失败、锁不跨await，以及状态查询原IDs不丢。root管理native槽；不触用户服务/kill真实agent。当前未能标完成。

## 2026-10-30 派发与只读候选

Root 已将 input 作者从 B 固定源移交本项；同树 khor-round2-input，新 fix/round2-state-concurrency@5c726b0a，B df92 全链/内盘恢复仍保留。当前仅只读和 fixture 准备授权，没有 native/App 或产品锁改动。graph minimal/semantic 无本链覆盖，精准 fallback。

已核真正 CLI --all 先同步 Node::open，随后 sessions_arranged 读 devices/roster/rows，列表本身不进 document WORK/WAITING。constructor 先本进程 RootPermit ordinary≤1s，再 attach 的跨进程 trust gate.try_lock 立即 Busy；普通 recovery 只有 needs_recovery 才 State.open。后台 doc_step 则 WAITING8→WORK1→blocking State.open（LOCAL+metadata OS gate），devices apply 另取 root_gate。字串同源不说明实际阶段。候选根因为跨进程 constructor attach 与 devices apply 的短 root gate 竞争，但尚无实际运行证据，不能宣原 hinton/turing 已定位或已修。

精确 source-plan.json 在 evidence/round2/network/state-query：拟一个固定私有 fixture，actual doc_step devices receive 在拿到真实 root gate 后 cfgtest checkpoint；owned child 与 public Node::open_hosting(false)→sessions_arranged 查询并发，一次 query/释放后一次 control，记阶段/typed primitive error/IDS，不网络/Pair/agent/用户home。新增 query_concurrency_tests.rs 与 doc_transfer_state.rs 仅 cfgtest mod/checkpoint 写权已向 Root 请求；实际 Node one-parent 编译/运行另待准入。没有运行 Cargo、启服务或修改产品。

### 固定 source 候选（仍未运行）

Root 已增授 link_doc_transfer.rs 仅3行 cfgtest 模块注册，避免扩 on_worker/doc_context 生产可见性。固定 05419f43→43bf74b38dafff4cb511fddebb95669a259a5100，三源 tracked clean。Context.apply 原锁后只有 cfg(test,unix) checkpoint；fixture 实际 helper 经现 on_worker→State.receive 小 devices chunk，不是全网络同步。手握同 Child unreaped 句柄、root/PID/PPID/阶段 proof，并两次 /bin/ps 实际出生+exe核同后，才把实际 CLI sessions --all 指向 owned receiver；finally Release guard 无论断言成败都发释放，deadline/reap有限。仅一次 held query＋一次 release control，正式10无进程 live/roster IDs 保留；辅助 RootPermit.attach / State.open 真实 typed Busy 探针不把字串转换成类型。成功或失败都不冒原集群根因/产品修复。

旧实际私有 CLI b83f147e…8584 可只读复用。216个 Node/commands/core/sync/CLI 相关输入中213全文件SHA同producer5ddc；doc_transfer_state.rs/link_doc_transfer.rs精确去本次cfgtest后与producer字节同，tabs.rs仅B5 pinned close差异，不涉constructor/query。没有说216同SHA或整个binary新source。JSON对账 cli-source-compatibility.json、contract/run.py 和 source-receipt.json 同存 network/state-query。

计划一次Node --lib --no-run actual Cargo artifact→private SHA copy，再一个exact ignored父case（其owned ignored helper另有1个libtest结果但不计第二独立case）。jobs1/incr0/offline/sharedtarget/外盘TMP/双盘5GiB/每cmd300s，helper hold10s/process3s/reap2s。只AST/rustfmt/diffcheck0，实际编译/运行未授未做，交reviewer准入后由Root排唯一槽。

## 单 fixture 实际终态（尚未产品修复）

Root 授唯一native后按原43bf/contract df35/driver9fa3运行一次，exec99879真实terminal0/83.5387s，actual Node libtest compile0，1个parent＋1个ownedhelper两份1/0总结（不是两个独立case）。actualartifact/privatecopy SHA 3bb0f6c8386b86326ffea8bb4bfea7f25b23e47941487ba35b98615ee3c2614f；全2186 source/env同，remaining/footprint空、TMPabsent。driver38025/Cargo38043/parent40248/helper40278/CLI40292,40307均实际不在，已正式归槽Root，无新runtime。

真实CLI held query exit1，原stderr逐字 `document transfer state busy\n`，219ms；释放真实devices Context.apply的root gate并完成import后，唯一对照同command exit0/195ms。10个原会话ID全保，baseline全部ID也保。actual root/PID/PPID/OS birth+exe proof、原raw stdout/stderr和typed RootPermit.attach Busy / State.open Busy 均耐久。原privateCLI只读复用，未重编CLI/Core/gen/GUI/App。

确证：这个有界受控跨进程窗口下，普通CLI的constructor root OS gate.try_lock竞争可以产生该原样错误；不是本fixture WORK/WAITING quota（只有一个已实际进Context.apply的worker），新夹具没有Pair recovery，列表本身无State/worker调用。metadata primitive Busy单独记录，不把它误当CLI来源。**不能据此断言原hinton/turing的唯一争用者，也还没有产品修复。**下一步仅在Root裁定后选最小ordinary query/constructor策略；当前生产锁、事务、错误与协议均未更改，不盲重试/吞Busy。

actual-receipt.json逐SHA保13实际证据和原source checkpoint；内盘独立state-query-43bf74b3-actual1镜像不覆盖source-only镜像。

## 最小生产修复 source（150715f2，尚未编译/green）

Root根据已证受控机制授 io.rs 生产窄写权：a3486f0c→150715f2。ordinary RootPermit 同一 OPEN_WAIT 截止点延续到 attach，仅对确切 OS trust gate.try_lock WouldBlock 在剩余预算内等；没有重跑 Node::open 或任何构造/恢复/发布。attach入口和每次try前均查截止，free gate也不能复活expired permit；try/control None仍即时try，ordinary metadata-held→root立即Busy，不等待metadata，不新quota/协议，不绕trust fence。原total budget到期仍typedBusy，原filesystem本身不可抢占的现实未改变。

同一个io新case明确expired-held/expired-free、control/metadata零wait；保原same-root等待、reentry/timeout、async cancel及LocalSet合同，计划5条定向unit。正向仍原一个私有fixture，改为异步启动真正当前CLI并实核其birth/exe与lsof确切gate FD后才release，不用固定sleep猜释放；一条waiting query+一条release后control保原10IDs及baseline。原43bf/actual1全部保持原字节，原CLI报Busy是业务观察红，原诊断test本就断言Busy通过，不能改称同一个未改harness exit1。

生产io变了，Root明确要求新实际CLI，不能用旧b83冒green。作者tree没有dist；Root在其cc744组合批做真实bindings/catalog/types/Vite，待交不可变真实dist来源/逐SHA后仅供N新CLI嵌入，GUI资产来自独立Rootcohort，N不冒其与本tree全GUI同源。green-source/run.py已沿原guard实际artifact流程最小准备，AST0；**contract故意尚不存在，缺真实assets事实前不能执行或准入**。这里只源固定/独审无阻断，未运行新Cargo/CLI/前端。下一候选待Root资产和统一唯一槽。

### 真实资产已解锁 / 最终准入（仍未执行）

Root给出只读 cc744 frontend-assets-4，manifest SHA52d5e6664b3343f0b69ada45043fd8da3696c27ac710a3f3f3962d6de5e73094，277文件/15,208,767B；作者逐SHA核、普通copy到ignored apps/gui/dist并再核source未变。它是Root实际types0/Vite0产物，只供N Rust CLI编译输入，不冒N UI/API全源一致或GUI行为。copyreceipt43e31394757fdc7a1f312f9b2846e455269c817bb523c4b979bcc333a07d08b5。

150715源未动；最终 green-source/run.py SHA f3827cab547245fe56539d8d01a7dfb010b79b5f051b034e74f361e7ee79d4b3，contract314b5cbe8e8a921d6d341e535f204243e1db098cff4a8a2e793d3e4d7b3694fb已独审准入。先current Rust CLI实际Cargo target row→privatecopy，再Node libtest→5io unit＋onequery parent/helper；总300s逐phase仅收紧到剩余预算，双盘5GiB/严格owned/TMP/source-env/asset各阶段核。没有新前端producer/独立Core target或tests/GUI/App/网络sync；正常CLI依赖会真实编译，不称V完整proof。旧43bf actual诊断不覆盖。Root现GUI libtest14未终态前不启本批，实际native执行仍待明确唯一槽。

## 1507正向与7482诊断实际失败（原false保留）

1507 green-source/actual1真实exec24467 exit2/154.5523s：current CLI actual Cargo/copy0（SHA9142722a…acc9a）、Node compile0、5io unit各1/0；原强positive在已观测root FD并release后actual CLI仍Busy，parent0pass1fail/0.72s，整体未绿。source/env同/strict owners净，ownTMP110files/20915B按原inode可逆rename到retained-tmp，原false和所有mtime保，不覆写成功。附timeline是文件观察，不是Busy决策点证明。已归槽。

7482 metadata-source/actual1真实exec2449 exit2/73.8693s：Node compile0；parent在Fixture.node243 Node::open_hosting(false)早Busy，case1.85s、helper/CLI均未启动、实际publication机制没有验证。source/env同/guard无timeout/forced/remaining/footprint、TMPentries[]且目录absent，6073/6092/8029等全部实际gone并正式归槽。新鲜receiver/donor的具体label/acquire还是attach没有日志，不能补造锁持者或将其归为fixture故障。Root要求核初始化CPU/FS是否耗掉1s，当前绝对until源码确实涵盖了Node对象构造；既不重试/预热，也不擅自略过信任门。

## 正确等待预算与单反例 source c571（未运行）

Root选定“累计锁等待额度”：local实际等待＋OS竞争等待仍总计≤原1s，constructor CPU/FS不耗该额度；try0/metadata→root仍不等。c5715e4055eeaacb69f75855792295acbd1e0e42仅新增36行test-only owner API反例，生产仍原1507规则。真RootPermit.acquire admitted后，明确OPEN_WAIT+50ms非等待初始化间隔，同OS文件前后均实际try_lock成功/drop、metadata false，然后attach应能成功。没有Node预热/Node重试，也不声称捕获原243唯一原因。

nonwaiting-red-source driver4ce16b887f7a9deeabb1a4ffd13488735d86ca14e82b71d8015bd3b215a05cd9 / contract4ba8dbee34bb5b9397699468127d3134887908fe8391f8bb28c5ee2a18707a36已独审准入，未native，排H11/gen实际终态之后由Root授槽。仅Node actualcompile+这个exact业务红（预期101、0pass1fail、physicalfree/delay/Busy特定marker）。未改guard.run，调用者只在其完整strictcleanup后对唯一预期101解析原结果并硬核所有安全字段；任何unknown/timeout/forced/remaining均停，300总界不变。不跑5unit/CLI/meta/原positive/Core/gen/App。

1507新增idle绝对deadline过期必拒的unit仍原样保存和5绿旧证据，**其把非等待初始化wall time算进等待额度的前提与正确合同冲突**；红后会明确更正为“真正剩余等待额度为0必拒”，不能偷偷改成绿。原positive FD/CLI exit0/全IDs等原业务断言保持，metadata型Busy路径仍须实际闭包。当前partial-receipt.json分层保存两次false，不称N已修。

## c571单反例实际红 / 累计等待source96510a3d（未编译）

Root授槽后nonwaiting-red-source/actual1真实exec95905终态0/79.6614s：actual Node compile0，唯一exact test exit101、0pass1fail，原物理gate两次free/meta false/1053ms非等待间隔后attach=false/typedBusy，精确业务红成立。source/env同、无timeout/forced/remaining/footprint、TMPabsent，35591/35610/37653均gone且正式归槽。此仅证明1507绝对clock错误包含非等待工作，不补称原243的唯一实际失败点。原false7482/1507及原43bf诊断不覆盖。

Root允许最小生产修正，固定96510a3d0c7b664bb64ee34c4cc3af195e5e240f、仅io.rs：RootPermit在local admission成功时冻结剩余Duration，构造CPU/FS不消耗；对确切OS gate首次WouldBlock才启动这份余额的一次deadline，不每轮重置。local原有1s入门deadline未放宽，actual OS等待只拿余额；try/control None仍只try、metadata→root不等待。成功取permit后先drop registry mutex再可能drop exhaustedpermit，避免Drop重入同mutex。没有Node整体重试/预热/metadata等待/恢复绕过或新协议。

原c571反例逐字保留；旧1507 idle绝对deadline前提明确更正：unit名改ordinary_attach_spends_only_remaining_wait_and_control_never_waits，以实际余额ZERO测试耗尽，保held/free/control/metadata拒绝；同unit补20ms剩余OS竞争不能重拿完整1s。rustfmt/diffcheck0，未编译。

wait-allowance-source/run.py SHA051b7596be4ebe02ee7cf5c41af65b4326be82e4af18a236df3376b0f0b64cb9、contract e17be29e86e2e654bdee123729dec3cc10df22bc47500a265aa47afc2209288d、source-receipt93c51c5dda2a1e9abb7e0ea8219ef13aa473f7eb1b83f65be3f484671a124715待独审/Root准入。一次Node实际compile→6条精确IO（含原gap转绿、修正unit、same-root/reentry/asynccancel/LocalSet）→原metadata diagnostic父/helper，300总/双盘5GiB/strictowned/sourceenv不变。历史actual1507CLI914仅该第二阶段诊断actor，明确与当前io不同，不冒当前CLI productpositive；若达到真实next/rootfree/metaheld且<1s而Busy，才证该路径。最终仍必须新actualcurrentCLI过原FD/exit0/全部IDs正向，尚未关闭N。

### 965实际compile早停 / 注解窄修29612abd（未重跑）

exec9495真实terminal2/17.9023s：compile101唯一E0282（waiting_until=None的类型推断环）；六IO和metadata均未执行，不计任何业务红/绿。driver43813/Cargo43856/rustc43970实ps均gone、source/env同、无timeout/forced/remaining/footprint、TMPabsent，已写terminal-cleanup并正式归槽。原wait-allowance-source/actual1的false保留。

固定29612abd69a94f9bdcedf4d188f8b5cf84d7d4cf只有`let mut waiting_until: Option<Instant> = None;`一行注解，预算/断言均未变。新不可变wait-allowance-source-v2/run.py字节同原051b7596…64cb9；contract b55cfbccbdb5b656c26f84269c16126c2a57aaa67f85919b0276d0bb79ce2c79、source-receipt13a3240b8287ec63c318e1244ab956e7664efa2cce3bc19cc242579033dea42b。source io SHA8177c9f325cc0774addb13933fc0397819e90bcf3a1146b5bbb754b53bf247cf。rustfmt/diff0，待窄独审/Root新唯一槽；未自行重编。

### 296/v2真实6IO绿与第二阶段诊断成立（仍未产品positive）

exec79561真实terminal0/80.3410s，actual Node compile0；6IO各1/0（c571原非等待gap红→绿、修正余额/control门、same-root、reentry/deadline、asynccancel、LocalSet），metadata诊断父+ownedhelper两1/0，合计7独立parent不把helper另计。source/env同，18observedPID全部实psgone、strict remaining/footprint[]/TMPabsent，已正式归槽。原965 compile101不覆盖。

此次真正到达Journal.save checkpoint：valid399B next/gen3已sync/drop未rename；父子都证root OSfree/metaheld，needs_recovery=true。历史1507 actualCLI914在这个持段224ms exit1/原Busy（<原1s，排除该次query是deadline用尽）；释放后195ms exit0、10IDs和全部baselineIDs保持。原始观察/阶段proof/出生exe/headerchecksum和raw stdout/stderr均耐久。由此证实第二条constructor publication-recovery metadata门可造成Busy，**不是列表本体/真实PairReady/原集群唯一cause，也不是当前296CLI productpositive**。

wait-allowance-source-v2/actual-receipt.json逐SHA关联actualartifact、8阶段、sourceenv、cleanup与两份旧false。下一先设计joint admission：等待metadata前释放root OS fence，拿metadata后root只try，反向忙则先释放metadata再等root；共用现剩余等待额度，不在持RootGate时睡等metadata、不跳needs_recovery、不整Node重试/重做副作用。具体最小路径/锁序仍待Root授权后落源，真正新currentCLI原positive FD/exit0/allIDs尚欠。

## joint admission固定source bcce3da1（未编译）

Root授四路径，固定bcce3da1678a2c9f6b2a67a64f378645fb6ec37e。io RootGate仍是原localpermit/余额/物理lease唯一owner；bootstrap_metadata先验证同root且原物理file存在，再释放OS root，等待metadata。拿metadata后只调用私有acquire_file(false)一次try-root；若忙，完整drop(metadata含TLS)后才同余额pause/等root，再释放root重试joint。每次争用pause都扣实际等待，None/control不sleep，metadata-held等待一律拒，普通attach的反序guard未放宽。原State.open/worker路径仍try0，没有新模块/registry/API。

State.open_for_bootstrap取得对应两门后重读needs_recovery，仍需恢复才一次open_with_gate复用原Journal.load/reconcile/body；不再取第二metadata门。trust仅该入口；lib仅可变guard/call。Node构造、恢复、注册/bootstrap都不回放：register_self/devices_loaded是recover后新load_pile，bootstrap anchor/rootsecret/devices也新读；handoff前identity/chat/live/usage/relay配置无过期anchor/roster信任决策被当新事实。拿齐后State离开释放metadata，root继续保持到原bootstrap结束。

同原IO unit追加wrongroot不建目录、joint持meta/root确实物理Busy、放meta仍rootheld、dropowner才物理free，未新case/查询矩阵。io/State/trust rustfmt0/diff0；整4rustfmt1仅lib原未改S ContentSource格式，父lib独立rustfmt规范化输出逐字同，原check/父blob日志保留，不改无关格式。source-receipt3e67382648348737c73bee8f883c3e2d92c4646a03b917c1751b5c4e937fb32f已交独审。尚未编译/native；Root私有GUI批独占native。

原FD/exit0/全IDs positive保留且最终必须新actualCLI。原root释放→next创建有天然gap，不能把串联同一query冒为必经metadata；已向Root报同现metadata夹具增加一个等待→release正向的有限选项，未先改fixture。整个N未完成。

### 最终两条已知路径正向source95acd426（未编译）

Root明确授同原metadata夹具一条currentCLI等待→release positive，保原rootpositive不变，不做有时序gap的单query串联。固定95acd42638f0f1f38b4bdabe32ab1b8e250e93fc接bcce，io私有gate_with_wait复用原打开体：普通State.open(None)一次try；bootstrap Some原permit等待LOCAL/同一已打开physicalmetadataFD，避免5ms pause期间FD被关/重复filesystem setup。仍只jointowner等待/共享余额，workers旧try0不变。

新增ordinary_query_waits_for_metadata_publication_and_preserves_ids：同PreparedFixture/realdevicesworker/Journal.save validnext checkpoint，先证rootfree/metaheld/needs和next399B校验，再启动当前actualCLI；出生/exe与确切metadataFD fence后，父独立root try成功，证明CLI已卸root才等meta；release后held和control都exit0/10IDs及baseline全保。没有固定sleep猜接纳阶段。原rootpositive函数SHA95a1bc8e…f75a5、旧metadiag函数SHA c1175011…8212逐字同；旧原run/断言全部保，最终批不重旧diag。reviewer已核bcce+95五源无阻断（尚未编译）。

最终joint-positive-source/run.py SHA5a679e039fe198ab3c5a479940dd17be04d12b77d02255919f3bc483e4efa1c0、contract09537d5d525f0221afbd12b069f0723635a802694e5ff43c07ebf09c4a0be1ca、source-receiptd7a20de9b470776cf0238845865381114fdec10d2d147bca6eef0fe3399f3565待准入。一次新currentCLI+Node实际artifact/copy→6IO各1/0→原rootpositive父/helper两1/0→新metapositive父/helper两1/0，8独立parents+2helpers不冒10独立case。jobs1/incr0/offline/sharedtarget/双盘5GiB/总300剩余严格owned/sourceenv/assets门原样；不CLI914/旧diag/GUI/Core独立目标/gen/App/新矩阵。cc744真实277assets仍只是CLI编译输入，不冒N UI整体同源。Root94771占唯一native，作者未启动本批；最终N还未完成。

## 最终currentCLI两条正向实际通过（待独审/集成）

Root授唯一native后joint-positive-source/actual1真实exec15681终态0/163.7677s；actualcurrentCLI Cargo0/copy SHAfd13b04ebee454fa4157e448fc56383e58cd1eab932687cddc6810505f2cfe6b，actualNode Cargo0/copy0acbe0f790f75c8fa76263d308cc2c807f69974aef8e3eb7faf18a8bf7d3a5e9。6IO各1/0，原rootpositive父/helper两1/0，新metapositive父/helper两1/0，8独立父+2ownedhelper，不计10独立case。source/env/assets各阶段稳定；44observedPID实ps全gone，strict remaining/footprint[]/TMPabsent，已正式归槽。

根锁场景：原exactrootFD fence后释放，held324ms exit0，control38ms exit0；元数据场景：本次Journal.next404B/gen3真实sync未rename、rootfree/metaheld/needsrecovery，实际当前CLI的确切metadataFD与出生核对后父独立tryrootfree，再release，held90ms exit0、control48ms exit0。两组10测试ID及全部11baseline/finalID均保，原positive函数/历史诊断函数SHA不变证明仍在。

这关闭了两个已实际复现的constructor锁竞争机制的实现/定向回归，不把它冒原hinton/turing唯一原因、真实PairReady/全网或GUI验收。原43bf/1507/7482/965失败与c571业务红、296阶段诊断全保。最终actual-receipt.json逐SHA关联新CLI/Node真实artifact、10阶段及源/环境/277asset独立编译输入和cleanup；没有运行V完整bundle来源链、没有新Core独立target/gen/App。source95acd仍clean，实际证据交独审尾核后由Root集成/关闭N实现；作者没有merge main/ship。

最终独审已核95acd treeclean/current源、actualreceipt全部证据、CLI/Node copy和45件内盘SHA，无尾核阻断；Root可集成fixedsource checkpoint，无需重跑。作者保持source冻结、native已归还。实际原集群唯一cause/完整Pair与网络/当前GUI/V全bundleproof仍未外推；本N实现和两个已证机制的定向验证闭包具备集成条件。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
