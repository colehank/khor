# R2-S-STARTUP：启动事件的字节界与FIFO交接

Owner round2_input，独审round2_reviewer，集成/唯一native/browser/App槽Root。原R2-S仍归history，不编辑该卡。属于原R2-002/003/007已登记startup bytes/queue缺口，不是新专项。用户已授权第二轮，无main/ship。

## 接手现场

N原branch fix/round2-state-concurrency@95acd426保留；其10提交已Root无冲突整合至d18161fa73ea2eb96f6bd3fda146a013944747b6、6源逐SHA相同。input作者树原clean，新fix/round2-startup-bounds从d181创建，没有reset/覆盖dirty。无本作者native/browser/App。Root准备其私有App；本片仅source，actualnative另授。旧N immutable证据不变。

先读中央STATE/PLAN及R2-S、debug-issue；图谱minimal/semantic无本链匹配，图索引main e7c，精准fallback核当前d181。只读前案history/startup-readonly-cc744/findings.md SHA99829a1a063865f24328ee00afbfc8876d2c0d8941034ba6cd03369b1d0e02a9仍提供事实起点，不把cc744与当前gui_host全源混同。

## 当前事实

- acp start_transport先unbounded Event channel，再opening_notes Vec32768（只计条数）→第二unbounded forward/receiver，完整预装后才返回Handle/receiver。上游handler已解析的Note不受该Vec条数限制。
- SDK通知handler会await自有callback后处理响应；SDK另有自己的outgoing/task等通道，单条frame/已解码对象暂存不能被本片宣成8MiB硬RSS。
- Ready=session已打开；gui_host后创建绑定canonical Journal，Ready→fence→有限drain→commit_base→host ready文件是不同事实。没有完整持久化不能提前发布full。
- 原EventFence4096先等ack再消费、later traffic不能无限延长、EOF尾/closed target错误和runtime cancellation均是必要合同；bounded预装await不能引入无consumer死锁。

## 已授权写权

crates/acp/src/lib.rs、lifecycle.rs、event_fence_tests.rs及必要private事件缓冲/测试叶；node/gui_host.rs及仅必要session_content/journal.rs startup pending交接。Root已明确H不并写。Core/Tauri/API/export/gen/Node其它adapters不归本片。gui_host_cancel_tests两处rawreceiver调用点如需配套需精确增授，已报Root，尚未写。Management独立login通道不消费start_transport返回值，不顺带改。

## 具体候选合同（尚未产品实现）

一个per-transport EventStore owner替代两段unbounded与opening Vec；编码payload window8MiB/256条覆盖排队及I/O在途，控制问题另有限16项/单项128KiB。SDK已解码incoming Note、一个当前dequeue解码对象的额外暂存单独说明，不伪造进程总RSS上限。先计序列化byte/count再进首自有queue；大于窗口单条走独占spill，窗口不是总history cap。

内存外FIFO写owned临时part，8MiB目标segment（巨单条可独立segment），只留head/tail/accepted/read seq常量级metadata，至多writer/reader两个FD；已完整消费segment即删除。长load不设32MiB/条目总history截断；能否继续只受真实磁盘/文件系统资源，写入失败必须显式refuse/取消并保旧canonical，不截断冒full、不自动第二次New/Load。具体磁盘资源防护/cleanup由同owner实现，不引新service。

本地EventReceiver封装recv/try_recv/len；不再把未界定payload复制进第二unbounded。Open结果通过原oneshot，close/failure/fence通过独立有限控制lane，receiver可消费前不await preload。FIFO保持原Note顺序，Ready只SDK事实。Opened后核真实native session归属，按现Journal.note写pending，完整有限prefix持久化后才commit/hostready。

Fence应包含本store身份+accepted seq有限前缀；GUI按remaining_through(prefix)扣数，后续事件不扩这次base/turn结束。Ack不是canonical持久化证明，4096先ack后drain仍可用（超memory窗口的数据可在spool）。

取消CloseGate立即原子通知/停止接纳，writer每64KiB检查，part关闭删除、permit释放、owned工作完成后保现provider.confirmed_exit/reap；Ready/Close/失败不能等待payload容量。任何I/O/身份/cleanup错误如实拒绝，不能伪DriverClosed=已reap。

## 首有限验证计划（尚未native）

先同fake ACP load在响应前发超过memory窗口的有序notes，显式barrier阻塞打开/数据交接，取真实自有retained encoded byte/count highwater；取消时先释放/关闭自己的barrier，确认关闭/cleanup不困数据queue。baseline实际retention超window为业务红，编译/setup不算。其后同规则饱和/取消转绿，合法64MiB（超过32MiB）base按批FIFO完整交接且不false Ready、无第二Load；再必要EventFence4096及现EOF/runtime合同。没有扩agent生态/全协议矩阵，实际native先固定源/精确case与resource预算→独审→Root授唯一槽。

## 当前状态

source/design only，尚未产品改动/编译/测试运行/新gen/服务；方案已报Root，先确认精确owner/预算与必要测试胶水，再按小片落实。

## SDK前置队列实读发现（先报Root，未改源）

实际registry3.5.0链并非全由callback背压：ByteStreams BufReader.lines无显式单行上限；Lines transport actor解析完整JSON frame后unbounded_send入Channel，独立protocol actor在notification handler.await期间仍可能累积已解析Frame。自有EventStore本身8MiB不能据此宣整个startup内存有界。SDKoutgoing/task等别的无界通道亦不能混称已解决。

具体source SHA与摘录链落history/startup-source-d181/source.json、sdk-ingress-findings.md（源码只读，无registry/vendor改动）。最小候选是在physical ingress入队前按byte/count取credit、整frame dispatch/drop后还；必须覆盖batch/unknownresponse/malformed，不只Note callback ack，不造第二parser/ID truth。genericConnectTo默认中继还可能丢特定Driver metadata，已向Root补报透明frame ownership credit方案与其需额外依赖写权的边界。Root尚未裁定该额外patch落地；当前作者tree仍d181 clean，无产品改动/native。

## 受控SDK与baseline meter检查点（2026-10-30）

Root已明确授权精确locked SDK3.5.0受控补丁，含jsonrpc/transport/incoming、private ingress、acp_agent/lib及role/acp与mcp active_session必要透明消费适配；schema/derive未动。实际archive checksum9bbe69c878aef64a3d92041e256e75acdb1bf2ea8e55ef3313e69dbb2f5bd75a，114原文件2,228,415B与registry逐SHA一致。84736111纯原样vendor、8ff41e2d纯声明；a154a46a按Root要求迁入vendor/agent-client-protocol，复用既有vendored_sources来源键，HEAD114原blob不变，行为始终另存。原third_party证据不改，完整dirty备份及内部恢复镜像已保存。根Cargo.lock未手改，实际path source转换待唯一槽真实Cargo。

当前baseline代码只增加整帧encoded ownership计量，尚不等待容量，不宣完成有界：raw complete line先计量，admit到decode期间同credit，透明Admitted随既有Channel/中继/clone直到整frame处理或drop释放；v2初始化和MCP现消费点保guard作用域。默认8MiB/256只是反例的预期合同，单巨帧真实bytes与line/decode瞬时项分列；SDK其它outgoing/task通道未据此声称全部有界。

唯一新压力case使用真实AcpAgent子进程与默认ConnectTo到Lines/protocol，首Khor通知callback测试barrier阻塞；AcpAgent源码本身直接使用Lines，不经过ByteStreams，后续gui_host自身owned ByteStreams链仍需实际交接验证。正常64KiB帧累积到至少两倍window后请求现CloseGate，先核同PID/birth/executable、实际child消失、DriverClosed、所有credit归零、Ready未发布、Load1/New0，再以window断言取得预计业务红。RSS是实际ps样本，不是内核峰值或8MiB硬RSS保证。当前未编译/未执行/未Cargo或metadata。只new leaf rustfmt与diff-check通过（早先edition格式失败保日志后已修正），未将source检查冒native结果。

后续先固定meter/单expected-business-red准入及真实lock转换细则，由Root安排唯一槽；拿到业务红后才落SDK容量等待/waker和Khor FIFO EventStore。现EventStore、EventReceiver、GUI canonical handoff仍未实现。两gui_host_cancel_tests的receiver调用点必要适配已获Root写权；其余H共享路径不写。

### 3781b5b4 meter source fixed / actual-red尚未运行

固定11叶3781b5b4adf7491487fe247e97e12d03bf710bc8；source-receipt9c415dbd40c5c533a14b681905b984345383ebe6facc06de2f27e8b754b2d1c9，位于history/startup-source-d181/meter-source。完整11源/patch/receipt已内盘startup-meter-3781b5b4逐SHA镜像。reviewer已源码窄审无确定阻断；不代表native/ByteStreams/有界实现完成。

Root要求测试producer也沿既有native_provenance严格门：原guardonly ed8/85d准入候选未执行，原字节保ingress-red-source/superseded-guard-only；现v2 driver ad9fd0710486cd0c715131496e5db7999ad555a3f7855d6b096c0dd90b17d6db / contract b4c00483179d9d46dd42518234b1e7d75dedf2f4451b788cfcf973e8869e7ab5，AST0且内盘v2镜像同，已交独审尾核，尚未运行。唯一允许源码转换是实际Cargo no-deps metadata将SDK3.5.0同一lock entry去registry source/checksum；完整parsedTOML其它记录和全source须不变，异常留实物停。之后真实full locked/offline metadata必须有resolve，现prepare exactACP libtest/no prior/apply-refresh→actualCargoJSON→verify/require→同actualrow私有copy→sole expected-business-red；300总/双盘5GiB/原owned严格门不变。driver不手写/commit/restore锁，不把预计失败冒Root当前验收绿。

### 首native预备真实早停与retry2（压力case尚未运行）

Root正式授批后实际session15877 terminal2，driver47219/Cargo47258。首no-deps metadata实际0，但不触发resolve/锁更新，source delta为空；合同准确以AssertionError(set())早停，0.440223s。未进入fullmetadata/prepare/compile/pressure，不算预期业务红。3781 source/env全同、rootlock原registry、strict group消失/remaining空/private footprint空/TMP空并已删除。actual-failure-receipt52d4d9ef5e11bc1ded309e036faef4718862b32c1410f143e314f8746a15d452，原actual与内部attempt1镜像保持。

Root允许仅第一实际invocation去掉--no-deps，使Cargo真实fullresolve生成path lock；retry2目录history/startup-source-d181/ingress-red-source-2，driver逐字同ad9fd071，contract647abeafdc5dba4360001f4ddf57eb9ac1ca3476958c7f91df579231dc608c64（补原failed事实，其余V/guard/case不变），AST0/内部retry2镜像同。reviewer命令差异尾核通过，候Root唯一槽；尚未运行。

## SDK入口实际红→绿检查点（未完成整S）

retry2 source47991实际metadata101：因首full未带hostfilter请求未缓存Android包而offline拒绝，未compile/test。actual Cargo仍只写出SDK3.5.0同entry source/checksum两行删除；只读整TOML/全source证明其它package/dependency/源码都同，receipt64e0fed5d97ba66f85203c3eb0297762200f51ef137e7b8bc56edfd48a34f56b；实际锁8df88cbc923b5ec3dca734ff1efcef6d6a2b1b0ae93b63cdee224c5921aa7e6e。该实际锁独立fixed9c342a9f，不手写/回滚，不冒该metadata成功。后续仅Root已验host-filtered locked fullmetadata，不再迁锁。

retry3 source9c34/exec17068实际fullmetadata0→SDKcompile101/E0004：TransportFrame::to_json借用match少Admitted臂，未pressure。原唯一diagnostic/false/cleanup/sourceenv均保。0298b20d仅加借用frame.to_json递归一行，不改变wire/guard。fixture参数改直接pin实际Python.framework runtime（普通Python工具self ps发现bin launcher会reexec；非ACP case），原exact身份断言不削。

**实际基线业务红**：source0298/driverc046/contractefa396，exec90088 terminal0/11.071351s：hostmetadata0→原native_provenance no-prior prepare/actualCargoJSON verify/require→ACPcompile0→唯一exact业务101/0pass1fail。峰25,240,210 encoded B/384frames，normal line最大65,730B/oversize0；真实RSS样本3712→40928KiB（非kernel峰/硬上限）。该window断言在真实child身份/出生/父/组/实际exe、DriverClosed、childgone、全部credit0、Readyfalse、Load1/New0之后才执行。receipt e2088a68ba84de452ae517b05217285da7e542af218905b5ece3e55eefd03e94；内盘24项/私有testexe低级gzip并解压SHA核同，manifest ad7622a11d58c9ec8d39e4f10790c0460081af6d7cff257820f60f13262ad597。reviewer已实际尾核通过。

**SDK capacity源与同case绿**：fixed e3268412f672799071915bda8d6c5656f73b5fea仅private ingress/KHOR两叶。单physical producer的AtomicWaker先register→同Mutex核bytes/count，饱和Pending；last receipt减账解锁后wake，owner闭合解锁wake，成功解锁后take清waker；合法giant仅frames0独占/真实bytes计费，仍非总historycap。原exact fixture731dea98逐字未改。source/准入均独审通过。driver b5e28244d519a235f9e72a8c4fe20e9d73c2fddc0371d961fae0ca1324f3c841 / contract2633960cd4a3875a567f710577acf916b539c2076010881ba5db52a47cec423d。

actual exec79152 terminal0/10.750296s：完整hostmetadata/V no-prior/compile0、唯一原case0/1pass0fail。峰8,347,600B/127frames≤8MiB/256；待解析complete line65,730B单列（不冒已被encodedcharged覆盖），RSS样本3760→19360KiB；原actualchild65884身份/closed/gone/credit0/Readyfalse/Load1New0全过。strict group/remaining/footprint均空/TMP[]已删/source-env同；native正式归Root。actual-receipt f7044dd36fdd561415cc4b795c692f5947e582366f0c9fc570514c1783ce9e5c；copy83b28a57f14eb5c97c5b8e745e122b98bc68302c4550dc557838bacd0f6ea636；Vreceipt3d822aff3fb68235d656eb865dddda297cce9c00e92762e32338224e96ab28ff。内盘24项manifest9199f46c134e10974acfde26b038ccf91f8462a275fc5727bfce7f4764893121，exe gzip解压同SHA。

仍未完成：本次只实证AcpAgent/ConnectTo/Lines正常帧的byte饱和/取消。tiny-count边界、giant分支、Node真实ByteStreams长base、Khor EventStore/spool/有限fence/完整canonical前不HostReady还待后续同S闭包。单未解码partial-line无硬上限，既有SDK其它outgoing/task通道不据此称全部有界。原三个预备/compilefalse不回填，原业务红保持，不把小片绿称S/第二轮全部完成。

### SDK稳定交接与EventStore独立WIP

SDK七链handoff component-handoff.json e3b5156c4612b064cd8d743d65c337fbaf31109207aeacddcae77ebf5ad8df8e包含d181..e326完整7refs/120最终路径逐git-show与current全同、实际red/green全source与tools/proof映射；patch280335c012fb4689a8892130e0dccdc32f89888b1edd8e4728962b13e12503d8及完整licensedsource已内盘startup-sdk-e3268412-handoff镜像。Root已整合该稳定片为82b45cf3（120文件全等，非整S完成），不修改作者接续dirty。

后续当前仅untracked crates/acp/src/event_store.rs，未mod/未产品消费/未编译或native。已写单IO worker的磁盘FIFO/常量writer+reader2FD（含rotation）/8MiB分段+合法giant/完整record前缀/已读分段删除；内存预算先count原serde再分配编码queue，8MiB/256包含encoding/queue/write，giant独占真bytes；互动credit16/max128KiB按answer/drop生命周期。actor/decoder单结果/recv/fence/opening/GUI glue还未写，不能据private primitive称已接入。root当前e326保持。

最新468行完整dirty source3b83c2ee6451dc20f3375c2da8ef82a6120d6b070e9aa55a1e50e057d9fdcbd8、CONTINUE.md e569b950bfc038ad866084afdc6a0ac958c46b72a6acc0fccef8d2b99f875656、recoveryb26362124178b2467fd095a856e8aefa77019a29380e3635fcc616808196f823位于history/startup-source-d181/event-store-wip-2；内盘startup-event-store-wip-2逐SHA镜像同，rustfmt叶检查0，无native准入。

接线关键待保：CloseGate.driver_closed内部会request，不能将每次request无条件当数据abort。driver select明确close分支cancel并join IO；正常EOF保accepted有限尾再terminal。cancel必须同mutex/Condvar同步并唤醒容量/数据/IO休眠/fence与completion，不能只靠I/O64KiB检查；runtime keeper guard外建保证unpolled/drop取消worker。单IOjob、单read-request布尔、单decodedoutput不可改成无界请求/结果中继。后续只同原S闭包继续，无并行H共享source/no native自启。

### EventStore完整source检查点231e1f69（尚未编译）

固定 `231e1f69e4fa354283714d12763c7437bd5d8934`，父为已验SDK `e3268412`，10叶与source-receipt `0f349d81cfb5ac8d89d39c077e42c5e3085fa588a77068969892f21c5245f3aa` 逐git-show SHA一致。完整源、patch、验证边界位于 `history/startup-source-d181/event-store-source`，内盘 `recovery/round2/startup-event-store-231e1f69-source` 同字节；此前WIP5回执 `16f3b62505b33aed56d679eb73a7342a0cb6909186845ea29dbefba81362191b` 保历史，不覆盖原SDK红绿。

已接线但只source：单EventStore owner持单blocking I/O job、单read-request布尔与单decode结果；8MiB/256计encoding/queue/write，合法单巨帧完整计费且独占；8MiB段盘FIFO、writer/reader常数FD/标量head-tail、分64KiB取消检查。receiver recv/try_recv/poll_recv不作磁盘重组或decode。question自己持16项/每项128KiB credit到answer/drop，不因出队提前归还；管理认证的共享question构造亦复用同Store，未造None/免记账分支。打开前取消/失败abort+join；live正常结束保accepted有限尾，Closing冻结末seq后等实际driver/provider事实再确认Closed；abort Closed只是stream error，不冒reap，退出仍Handle.shutdown/provider唯一确认。Node按store-bound finite prefix计剩余，不再采mutable len。

源用例：原fence4断言语义迁实际Store，新增closing事实门/Closed末seq和opening拒Ready2；Store私有4（sticky failure、等待receiver/idle job取消、delivered question持credit、合法9MiB完整记录），budget1（byte/count/giant/close唤醒/control额度），disk1（真实create_new rotation碰撞/poison/part清理），合ACP12。原SDK pressure源码731dea98未改，拟另精确回归1。Node原raw Closed-before-tail仅抽共享私有poll闭包，原100ms/20ms断言不改，明确只是调度算法证据；新1025×64KiB长Load parent/helper用真实owned ByteStreams→Store→canonical Journal，分producer checkpoint核HostReady尚未发布，最终逐条计序/内容/无重复、actual Capture尾页可读、Load1/New0、正常Close/reap/part清理。RSS仅实际进程样本，非kernel peak/8MiB RSS硬限。保原failed-bootstrap parent/helper组合。

拟实际批：host-filtered locked fullmetadata→原V no-prior ACP/Node libtest各actualCargoJSON/verify/require→私有SHA copy；ACP6+6+1、Node3 parent＋2owned helper，共16parent/2helper（非18独立场景）。300总、双盘5GiB、jobs1/incr0/offline、strict owned/TMP/source/env，no CLI/Core/gen/App。当前只rustfmt私有叶/fixture与diff-check0，尚无此SHA Cargo或实际用例结果；native仍候Root正式单槽授权。Replay退役未写，H清单待跨owner实施，不能把旧32MiB Replay链冒已消除。

#### Reviewer发现的夹具修正27d（无native运行）

231e原候选保留；`27d6434494db35ad0baf2cf22b717422433a2a6e`仅修新长Load测试。64KiB正文加JSON包装本来就超过原inline64KiB，正确结果为Reference；现经同binding/cursor真实content_record每段≤64KiB读取最后完整ref，核offset连续/完整raw/1024全文，不减payload、不调生产阈值。外层强核owned helper唯一1passed/0failed/0ignored再透出stdout；执行门只对long-load期望inner+parent两条各1summary，避免零case假绿。源回执e9561d35ea0d18aedbdabb994740d18a308a3cc27c2b992dda537a14cfcf212d与10叶/git-show双盘同。

待审runner位于 `event-store-native-source-2/run.py`，SHA a2fbaf1eca221277ef7f7ffe3abebd5391f0c31b1b05e6c5db29044eccfc4993，contract d62c6d804f8fb9e074fa168fc6b5bd7128b492eda2b97fbf1afb370ee2300b44；AST0/22source+2helper pins0/HEADclean已纯核。沿原V全链与guard硬门，尚未metadata/编译/实际测试，候Root正式单槽。不可把source-only修正写成原231e实际失败或已转绿。

### Store实际组合118.53s全绿（限定本片，不关闭整S）

Root清理已独审的旧666专用缓存后，正式授source3：driver `2e78be1949158c5a0172bf2d4d1d3e973668a88e17a0402d9091f19b09b793c4` / contract `12b8df98b692086ff48a3994a30872f0f6dcd7a4578f0a1f16036519baaa7690`。仅加前置KHOR_AGENT_BUNDLE必须unset，既有build.rs/not(khor_embedded_agent)是支持的libtest变体；focused实际Python→acp_process→ByteStreams不走KhorAgent/sidecar。原150MB copies估计被纠正为历史363+22MB至少约400MB规划头寸，非硬上界；实际本次ACP23,465,768B、Node287,135,800B，不回填旧估计。

**actual95235 terminal0 /118.530588s**，driver64896。host-filtered locked fullmetadata0；原V no-prior prepare/actualCargoJSON verify/require与私有SHA copy各ACP/Node全0；ACP finite-prefix6 / bounded-store6 / 原SDKpressure1，Node原Closed scheduler1 / failed-bootstrap parent1+ownedhelper1 / 新long-load parent1+ownedhelper1全部强summary通过。共16 parent＋2 helper不是18独立场景。27d/source/env全同，无timeout/forced/remaining/privatefootprint，TMP entries[]且目录已删，native已正式交回Root。

真实长Load payload **67,174,400B/1025条** 经owned ByteStreams→Store→canonical **67,352,665B**。producer16/32/64MiB阶段仍无HostReady，最终Ready时current完整/有序无重复，canonical同binding/cursor原raw引用分块读末行1024全文；Load1/New0，原Close/Gone/childreap/私有partgone强验过。RSS实际进程样本13136→27072→27120→38000KiB，非kernel峰/8MiB硬RSS，也不将私有预算单测等同整个进程所有队列。SDK原pressure峰8347600B/127，pending complete line65730另计，child65144已消失/retained0/Readyfalse/Load1New0；巨帧分支本次覆盖Khor Store9MiB，不冒SDK单巨wire路径已实测。

actual-receipt **97b82bea3d3e3a500798c990c9b20549246860330b91176007a00fef1f016388**、result **0dc562015ba52afeb7523c1d6af52156de554e5ce3863d8d7ea6c148a1904872**，位于event-store-native-source-3。actual ACPcopy **3a2aac9a4d0c29bfa012ab63ba595a5a9fdaaa1c874ca05aff7ab78c5d62cb38** / Nodecopy **cafaa586341c472689cc70934502d0483b82bb61100d9b5e7c10a8c4f6277e23**；49小件与两完整gzip镜像内盘startup-event-store-27d64344-actual1，mirror-manifest **8d8ef24a830363a0afc4bcfdf76a8dadad063568e47b9f9401295a04ed3500f7**。压缩前UID501/regular/nlink1/无打开句柄/源SHA、压缩后流式解压SHA均核，原私有exe未删。

仍保边界：无当前GUI/App/发布/全平台证明；管理认证复用仅本批编译，未单独auth peer行为。Node依赖ACP非test编译有一个仅测试用stats getter dead-code warning，后续必要清理可cfg(test)，此刻保27d已验源不改；不是业务失败。旧Replay死API仍待退役（不再造spool），当前互动/Stop/Close/历史完整等语义迁移和H恢复、当前App组合均需继续，不据本片绿标整S完成。已交非作者实际尾审，未重跑或弱化原断言。

### Replay退役source4dd9（尚无联编/运行结果）

Root接受Store27d实际组件后授同worktree新 `fix/round2-replay-node`；旧27d/source/native原字节保。固定 **4dd9db6c4caf6f567c7ab7940e7b483d0957ce88** parent27d，15授权叶/treeclean。源回执 **cac462edf6f8c13fe0a45e8ae56215bf6b96b4fd3a9ad87f3c32aded398d0598**，完整 `MIGRATION.md` **0754f41c2a56545d63539e140ba7965e2c16bf06716f521af4a31497cd11c664**，位于history/startup-source-d181/replay-retirement-node/source-4dd9db6c；内盘replay-retirement-node-4dd9db6c-source逐SHA同。此前WIP1/2/3全源/patch恢复副本亦保。

source移除GuiOpReplay/GuiNoteHistory+End/CaptureFailureReplayInFlight、replay_wanted/Replay/ReplayDelivery/32MiBcollector/publication/Handle.replay与仅供其用的Handle cwd/environment副本；Facts.replays/Agent.replays、初始LoadResume/OpenedBy、Store有限prefix、canonical唯一writer、真实turn/control/Stop/Close全部保。无第二spool/compat/API占位。stats只测试getter加cfg(test)，不据此把旧27d实际证据改成新SHA重跑。

GUI_GREETING升 `/2` 是退役能力/帧集合break门，PTY HOST_VERSION5/ring.replay未动。**此前“枚举按index重排导致读错”归因撤回**：实际proto::encode→rmp_serde::to_vec，locked rmp-serde1.3.1 encode.rs写variant名字、忽略u32 index；结构payload字段才positional。保GUI1/unknown/PTY负例、覆盖GUI2正例/future3；host.rs只PTY负例新增GUI2，Core同义负例由H7d单writer。

测试逐项语义迁保见MIGRATION表：ACPNew真实MCP/context通过Prompt Elicit保持答复/close；旧初始Load内Elicit无法在open返回前向UI暴露的独立缺口由Root登记NEXT-ROUND，不新增openingAPI。initialLoad finiteprefix/两历史chunk与NoLoad实际restore拒绝；两个CLIshim原ID/权限/Stop后继续/EOF/drop强断言保，fresh无历史有限prefix，原第三owner在旧owner结束后初次Load规范化past+user_message_chunk先于新prompt。NodeGUI两view真实Prompt有效/无效/迟到Elicit答复与广播、单view token Capture/真实canonical page/另view300ms已证明通路后无泄露；NoLoad诚实new空capture。journal双Capture含重挂同binding/字节不变，原再次Load只一base、raw换文件拒绝保。

Node原cooperative/flood/slowGUI/timeout等当前case保；五旧Replay场景迁真实当前负载：large live-flood Stop实际SDKcancel/最终Note-beforeCancelled/同进程下一turn与canonical尾、Close5s reap；30MiB真liveNote的raw drip保持64KiB/80ms/5MiB/>2s/15s读探测与另一view的4s/3s cooperativeStop/下一turn+继续读；16384条初次Load经实际Capture/H所有older页无跳无重、原5s基础marker+12s全历史数据timeout及字节预算、Load1/New0；pendingControl中Capture不reload、原4s control完成/同进程。只退掉死API专属HistoryEnd/已ACK UI取消不发SDKcancel/repeated publisher2s grace判据，Root已明确认可，未将其伪装成现live-turn。

Root增授的gui_refused只kind_of移旧两臂/显式Captured，其他两窗拒绝/fixture/时限不动；gui_session私有stub locator必须canonical absolute regular可执行/无symlink，受控test-evidence模式缺KHOR_ACP_STUB即拒，不能在实际阶段隐式Cargo或猜exe父路径。普通本地便利分支独立，不称来源证明。CLIagents当前Capture/H helper仅source迁移，保持原真实写入/拒绝/close断言，未运行ignored real model。

当前仅15源Rustfmt语法解析/私有Python AST/diff-check0，**无Cargo/typecheck/实际gen/native/App结果**。已交非作者最终source审，H共享47fe/fixtures ee341/Core7d是另一作者片，Root将集中整合同源后真实producer/针对实际受影响测试，不手写gen或临时旧wire兼容。整S/第二轮继续active，34项不据source完成缩范围。

4dd9最终非作者15源尾审已通过：源码git-show/当前/快照与回执cac462ed、迁移表0754f41c逐SHA同，clean，无确定源码阻断。Node新五场景/私有Python与agents Capture helper实际逐源核过；这只提升source审状态，仍无联编/运行结果。组件handoff位于source-4dd9db6c/component-handoff.json；H共享/Core负例单作者片与Root集中producer是下一必要步骤，不把此结论写成新Store/整S实际绿。


### Root255退役组合下一定向验收计划（仅准备）

Root已组合Store与Node/ACP15叶+H共享到 **2552334ab1d55e227f4dacb90127f2e1b930f60f clean**；新源不能套旧27d实际绿。本作者原4dd9/27d分支及所有原false/actual均保。graph先查显示旧511ad索引未覆盖相关targets，随后仅按实际Root源码定位exact与依赖。

计划目录 `history/startup-source-d181/replay-retirement-native-2552334a`：runner **0de78602412d831aa3298672f1c4ecf7e76a8bf1b8fdcfb6793c7889be887396**，source回执 **7cbf02c07a4b1c4deb011688bd5b3a56243f4eca22cc544b74eca2d08f289e14**（2331源frame），计划回执 **383b35d45b7b0e9dbf99891e3439ff8b6f00f5301292f71747ca4128ff9e24e8**；内盘replay-retirement-native-2552334a-plan镜像 **a664cba6c712ad5672cfc4aa8e8c00f3cb6e6832e5f8e8741af30144b78acc19**。仅AST/实际fn与target存在/计数静态检查，**无metadata/Cargo/native/gen/App**。

三批各需独立Root正式槽、各总300s/单命令仅收紧剩余、双盘5GiB/jobs1/incr0/offline/原owned/TMP/source-env/V严格门。每selected先各自no-prior prepare，再对同一package实际CargoJSON逐verify/require/唯一row/privatecopy；不借fresh waiver。A contract **e0c205e92ccf86a08a006fa92f925ca9935de24d7c85cb3fa2ac43415b827186**：actualstub+conversation同次producer，3exact（initialLoad完整prefix/context+PromptElicit/unsupportedLoad）。B contract **f71d3ca0533340a89e08bbda125073f539c3f7fc7f6073667311642a3ebdc10f**：Node lib/gui_greeting/gui_session同次producer，13父+7owned helper；含迁移5/journal/noLoad/PTY1、greeting2、真实双view/ownCapture/noLoad3。C contract **d6b2a2bd8e184b232ba83ac1028a7aefa10c0951a4948e2bc0cd025cb79ca458**：actualkhor+4integration targets，同源两shim+两窗拒绝3exact。总19父/7helper，不重无变化Store13/64MiB。

CARGO_BIN_EXE编译时指原target路径，runner明确逐次核其actualrow/proof/privatecopy相同SHA，不假称环境变量能覆盖它；B gui_session实际只收A经V私有stub的canonical pin，受控模式不能隐Cargo。B借Aproof只在require时重建三项明确phase路径env，再finally恢复，不把它当B依赖rlib复用凭据。

H同源actualbindings/catalog来源/noEmit优先。C额外经khor-web/build.rs嵌dist，当前 **assets:null明确禁止C实际命令**，须H/Root当前255真实Vite资产逐SHA回执后repin；A/B无web/dist依赖。agents新Capture helper唯一个caller仍ignored真实第三方用例；无既有hermetic共用case，默认stub不真实写获批文件，不能假替换/删原assert。Root接受仅编译与人工第三方runtime边界，不新fakepeer/不跑真实模型。预计A20–50s/B70–150s/C90–200s仅排期范围，不是硬保证；每批硬300s，首次联编失败原样保后最小收口。私有copies粗估A25–60MB/B0.65–1.0GiB/C1.0–1.6GiB另加未知编译输出，不能当max；driver不删缓存。已交非作者准入，尚未授本批执行。


#### A实际编译与行为续批（Root255）

原A exec **44970 terminal2/14.876s**：actual fullmetadata0/compile0，两selected分别no-prior V/require/unique actualrow/privatecopy真实成功；首behavior还未spawn，runner误传str给guard而触发str.parent。原false不回填，不当产品业务红。原stub **5b0394c37c5dcf3e56f48bfb0be93e55c218b42519eb998fcdaaf33095acfa98**/14,889,944B；conversation **a1225d6cfa2e6acb7cdec21546b8d46bd47660977cec94b011d711fbf6f605cf**/27,187,552B。

经独审窄续runner a2e95d41/contract7da4df0e，仅Path(binary)入口修正+复用已证明producer/原path+copySHA/原producer环境require后finally恢复，未重编。exec **63174 terminal0/2.5998s**，context/MCP+PromptElicit、finite initialLoad、unsupportedLoad三exact各1passed/0failed/0ignored；source/env同、owned/group/footprint净、TMP absent。actual回执 **7c825028b0b43b12c6fb589b259e522ad35615fe35c2df91383b4ed9e56cc6e2**、result **7eb9b9587703dc8555bf78708e0fbffd2fe0d7eeabb4778a459ab1ce52b13478**；内盘replay-retirement-2552334a-acp-actual含41原小件+两完整gzip解压SHA同、原exe保，mirror **290ef8168cada2f8e0a7f667a9ce62715adf1e9af4bcebcc93b98faa9bebaa82**。Root确认非作者尾核已过，只收A三行为，不冒Store/当前App/全S。

旧B0de/f71准入失效不可执行。新node-source-2 runner **3d01a0b8987d4106b3136cbbb13ffc3cea396c3c98d866d91ea4fbc242c0f3b5**/contract **8e3ba459b8b7c4a9347a52c83fc842103c5c29adb23f989718a958ea5f58d996**/source回执c9a13482：同Path修正+显式pin原44970 producer false(两成功stage)和63174 successful A3/sourceframe，原stub私有copy三方同，require仅核原producer环境而非错用续runtime。B原13父7helper/三selected/no-prior/300/强门不变，仅静态准备/交审；H新gen/noEmit/Vite已优先领槽，B尚未获执行授权。C仍assets:null阻断，等H当前255实际资产后独立repin；agents只编译不跑真实vendor已Root接受。


#### B当前16ee实际13父/7helper通过与可恢复归档

Root因两处TS死History判断修前进到 **16eec79ead2152b0b1afab08597cd782b5929084 clean**。完整2331源字典相对A255只history-paint.ts/ChatView.tsx两项SHA变；A3仍按255实际+该精确兼容映射陈述，不能将旧V fullframe偷改为新Root。新B **node-source-3-16eec79e** runner9469394e/contract32faa920/source8b627efc先用当前locked fullmetadata，再单独normal acp-stub actual build+独立no-prior V/unique test:false/privatecopy，接Node lib/gui_greeting/gui_session三selected同次CargoJSON各no-prior V；Path入口类型点已修。没有再编/再跑A conversation/三行为。

B exec **60432 terminal0/175.605934s**，16stage全0：meta/stub/Nodeproducer＋13exact父/7owned helper原强summary与原时间/字节断言通过。五迁移真实负载、16384初次Load完整canonical、control中Capture/noReload、flood Stop/Close/30MiB drip、journal重复Capture+重挂、真实NoLoad、GUI2/PTY5负例、两face互动/ownCapture/unsupported新空Capture均到达。源/环境/所有观察owned PID/组/footprint净，TMP absent；不冒Store13/64MiB/Core counterpart/管理认证/currentGUI/App/整体S/RSS硬界。actual回执 **6389689062a4b7e9fc8ebb42c24c892fdd995b29ba965679ea78d4f035274662**、result **45e9a355d10220ba87f1d0a2b7c0893eed72c15641b3c53e57b7b4183b6c5713**，非作者108原件/原target/私有copy/四V证明已实际尾核通过。

内盘replay-retirement-16ee-node-actual **mirror65bb535a1aeba389319dca1bfdb04afeade4b98cc40786d3f011687eee4eccbc**：113项含receipt+4完整gzip，全部SHA/流式解压同。Root为production16ee头寸**明确授权仅这4 B privatecopy**归档移除：UID501/regular/nlink1/inode+SHA/lsof无打开、4gzip复核/文件目录fsync/双盘durable index后逐个unlink。实释放 **439960128B**，外6732111872→7172046848B；原target4exe/source/proof/result均保。archive index **029c24a51ffd140620e7d8d18e3a06ed3f95f42f04f1010f74dbdce7f3189d05**、result **66019e66b8719c8905fe1a2593b11a224ecde21d1509ef54154f0e27e2b2c78d** 两盘同；今后需使用privatecopy先按index恢复原路径/完整SHA，不能将已归档文件缺失伪作原运行失败。C不消费B这些tests。

H actual gen lineage255（Rust原字节同）、当前16ee noEmit0/Vite0 receipt087f4263给C真179gen/278assets/2331frame。C **cli-source-2-16eec79e** runner16f9437c/contract7e869a2e/sourcee08e01d2/admissione82e05e8已非作者准入，normal khor+四integration各no-prior V/原path+copySHA；仅两private fake-peer shim+gui_refused3exact，agents compile-only保人工第三方runtime边界。**C尚未执行**；Root当前先生产default GUI/CLI build，不能自开native。E同源devURL文件输入验收只准备，非S本批实际结果。

B4归档非作者恢复闭包亦已通过。仅校正文案量纲：**439960128B为4文件logical总和；原df实际增439934976B，差25152B**；原JSON分别保logical总数与before/after字节，记录正确，不改原receipt/index/result，不重跑或重读已可变target。


#### 原5GiB门中止生产后精确空间交接（均已非作者FS尾核）

Root production16ee exec61197真实terminal1/190.34s：CLI actual/V/copy成功、GUI在链接被外盘5GiB硬门中止，原false/严格source-env-gen-assets/remaining[]/TMPabsent保。以下是Root逐项明确授权的可恢复归档，不是放宽Native门，不碰用户main/release/current16ee私有CLI。

1. 旧666 CLI＋ce69 CLI/GUI三个已终态nlink1 privatecopy：各UID501/regular/nlink1/SHA与原historical selected证明/lsof0核同；外盘完整gzip再decode同、双盘durable恢复index后unlink。logical1086306024B、外gzip330474867B，现场df5462495232→6193848320B（实增731353088B）；validation/archive-retired-nlink1-privatecopies index **fa06ffcee304ecd5e7660ee3abfb1441a88dfd45ecbf51502cd704565ba4c819** / result **ce9dae41082d9e23d4048609e4e148689815af99a9821186c4afdb3087ee153d**。旧ce69二进制不能再直接运行，必须按索引恢复；当前E只新16ee模板待实际devURL variant，不借旧备份冒新版。
2. 旧production84两个private名字初核nlink4/5而非1，前置即停0gzip/0unlink；H固定完整9alias报告b04a6f6b，Root再精确授权两完整inode组（CLI4/GUI5，已终态App2–5）。全组9项identity/hash/open/终态核同，一gzip/组完整decode+双盘index后逐alias按remaining nlink4/5→…→1核ino/SHA再删，逐步durable进度。唯一inode logical743790720B而非9名累加；外gzip229619667B，df6193836032→6685360128B（实增491524096B）。validation/archive-production84-privatecopies index **79b81351e4be31a00bdcfce946f4970694ab3770785db9e77360fff7b8a0eff8** / result **454c480fe5fc0fde01dda1dc1ac2f69320a8fb55a3e7f92562b1fa89665a5a44**。旧App false/原结果不改，无任意多link豁免；gzip外盘一份，小件双盘，未称全部大件双盘。
3. B三已完成target testexe（非stub）：Root原selectedSHA/nlink1/lsof0/原61197 CLI+GUI两raw2910实际路径无引用，已有内盘完整gzip再核→双盘index→仅3target精确删。logical425070184B，df6685306880→7110332416B（实增425025536B）。node-source-3-16eec79e/target-test-archive index **40916cd76766b0af1795a0c27a010518d219ad5642431c9dc6b89ba1afb5a844** / result **7c1b8e324873e3891b74096b9e68a48f34c1c00c8110fa7fec9a1962007871a9**；未动normalstub/currentCLI/其它target，原B证明/结果不改。
4. 原95235 Store两private（ACP23465768＋Node287135800B）只读清单a37452bf后Root明确授；原49镜像8d8ef24两完整gzip/identity/nlink1/lsof0再核，双盘indexfsync后仅这两private unlink，不新写外盘gzip、不额外动target。logical310601568B，df7148982272→7459610624B（实增310628352B），内盘5917409280B。event-store-native-source-3/privatecopy-archive-95235 index **f8d4181b3863e77ca55f293b73adbd3cc5832fceef8cea6f53600a0a52ff0a4c** / result **8ae9ae50470ba5421c50a4426cbd7069c51f34fb9adb770319e552a90fb49cc8**。Store Node旧target路径已在历史编译中被B同名输出替代并经上项授权删；本项只归档其独立private，不伪称原target当前仍在。

全部四组FS索引/进度/结果/恢复gzip闭包已独审通过；精确时点df与logical分列，不将背景变化归成确定净bytes。Root明确停止更多空间/666GUI/库盘点，已停；旧666GUI nlink3保持原状，无全target清理。最后FS终态直接交H，其browser13 actual98901已全部通过且严格净；Native现归Root GUI-only exec48383（metadataGUI+新GUI，不重CLI/gen），作者不poll其handle、不C/E并行。

C仍已准入未执行（16f9437c/7e869a2e、H087f资产），等Root GUI-only实际terminal后明确授。E文件输入只准备 validation/app-16ee-editor-preparation（receipt87db5ab3），原launcher d4edc12a/observerd50ddf13不变，outer只镜像basename695bb4ee；无contract.json不可运行，实际devURL23846变体/CLI/V/source/gen/config及新鲜用户时段待Root，默认custom-protocol GUI不能冒该observer已接。

Root GUI-only48383随后实际false/103.54s（Cargo0，V拒generated output current invocation/matchingprior），严格源/环境/gen/assets/owned/TMP均净，Parent/reviewer仅精确诊断不盲重编。作者C/E/空间保持停止。现productionCLI original+private copy均只读核355300120B/完整SHA **b6360a48d716eea4ac4385dc9610088fe56300e4aea114c93287ac3d0e0713c7**，current-cli-preservation-16ee.json29b12190/原CLIproof88882c89，两个路径均保，未以unbundled normalCLI替代。


### Core既有退役闭包草稿（无编译）

Root在消费者fixture收口期间授极窄只读任务：graph旧511无目标覆盖，随后读真实Core测试/清单。`replay-core-native-draft/contract.draft.json` **2cd233e752495bf9d1c0d551fc684fbd3408eaa9fb5499d6a6680f4e2f02e29f** / draft回执 **c304a8a8547198da59184000d42fda5fe6d23273c91902de4bd3fd04e6adcd46** / 内盘mirror33c81654；**head:null/无driver**，非执行合同，不随4个UI/testfixture小HEAD反复pin。观察时Rootffa079c1，Rust/Core未改，最终消费者freeze后同C一次实际source/tools/driver固定。

Package khor-gui-core，runtime只原三exact：`chat::io_tests::controls_keep_fifo_order_and_complete_only_after_their_frames_are_written`；`chat::host_frame_tests::host_frames_keep_protocol_content`；`chat::io_tests::capture_failures_are_typed_and_live_body_updates_are_only_bounded_wake_signals`。真实当前FIFO SetMode/Stop/Capture、length-prefixed MessagePack→当前frame、NotReady/Unavailable typed原因+1MiB正文<64B wake/配置Note均是必要退役接线。**FIFO原case读完三peer帧后wait，没有受控pre-write barrier，不额外冒写前wait尚未完成的时序证明**；无新case/矩阵。

Integration `terminal_current`（新增GUI2拒绝literal）和 `chat_upload_cancel`（GuiOp exhaustive去Replay）仅编译，不冒CorePTY/upload行为绿。本机官方CargoBook确认选integration自动构建ordinary binaries，当前Core有bridge/face_samples/gen_bindings；因此同一次 `cargo test -p khor-gui-core --lib --test terminal_current --test chat_upload_cancel --no-run --locked --offline --message-format=json` 诚实覆盖6selected，各独立原no-priorprepare/verify/require；三normalbin只编不执行、不生成TS。Root明确只runtime Corelibtest需私有copy，其余5留raw+V+实际SHA给target刷新前非作者核，不复制5份大件、不改helper或模型。

资源粗估仅libtestcopy150–350MB，6个target链接输出0.6–1.2GiB＋未知中间输出，单link RSS2–4GiB仅排期非hardmax，暖编译约50–150s＋runtime<5s预期非保证；原300total/双盘5GiB/jobs1/incr0/offline/owned-sourceenv-TMP保持。非作者已窄核范围无阻断，review-boundaries只澄清证明边界，不给运行准入。无metadata/Cargo/native/产品/H脚本写，未重H Native11/6/Store/A/B，C亦等最终HEAD。


### S3代次与Load/Resume源码检查点（2026-10-31）

作者复用原input树，`fix/round2-recovery-contract`，保4dd9/27d/SDK及所有旧actual分支；不新worktree/cache。S3 DESIGN-v2 61a08594已非作者设计放行，源码按小片分开，不把source检查点当完整恢复保证。

- 首片 **3efd516f**（live/gui_host/lib）已非作者源审并Root整合c4aa：GUI代次RunKey，report/exit同row gate拒旧，GUI状态唯一嵌入原子Meta；列表无row锁/不因忙锁丢行，Meta学习RMW保代次。5反例原样保。Root首实际36944 metadata0/compile101只E0382，5例未执行；独立单叶 **b443c973** 仅同步block_on借本代key、10处loop report改借用、outer final exit仍原key，源审通过，未将481其它源码混进Root小修。Root随后报native2五exact各1/0/0，属于其独立实际回执/尾核，不算作者在下面新源重跑。
- 第二原语片 **48184638**（6路径）已非作者源审：known native在provider前拿同File JournalReservation并验完整binding，初始化成功后原File直接交Journal；ACP实际capability guard不支持只Initialize即拒，无New/Load/Resume替代；Transport只带出真实Session::pid，不冒出生/退出证明；私有stub每child可选method名log及原unsupported case wire断言。仅源解析，尚未实际跑这两个reservation cfgtests/新增wire前提。
- Load/Resume独立片 **2250720ea8f429816b59226675b3bb7ab20077ff** 已非作者8源尾核、clean。完整Load写新的pending generation→fsync完整prefix→rename/active pointer；旧current和历次generation/旧opaque绑定保持精确可读，v1固定legacy/v2含32hexselector与独立sourcehash；失败后完整新gen不删且poison不Capture/Ready。Resume保留active base并追加所有实际收到同session Note，有限prefix sync后才发布新RunKey/Ready；Opening report/exit不碰旧Meta。New得到真实native响应后先source check再绑定。原journal/Resume host cases保超时/退出/Capture门，Resume唯一peer分支在reply前发一个合法新Note，断言wire Resume1/Load0/New0与raw/投影恰一次；不把这当历史replay。

225新增两journal、一selector、一opening-phase测试均仍**源码未跑**：raw X/X/Y是精确保留3记录而非3 bubble；多次Load旧v1/v2绑定保留；prepared后追加须重新prepare；abort/pointer失败保旧与complete gen；invalid selector拒绝；未发布phase不report/exit旧Meta。7Rust解析+1Python AST+diff共9/0，**无Cargo/typecheck/gen/host/provider/App/browser**。回执 `history/session-recovery/s3-load-source-1/receipt.json` **bfb069b71a44de454cb659bb57c3bfaa3735998e9b7caa9f417094ac1f29b5ab**，checks **4756edb808174c55eb567b113746acc6f2a5c0609253b22c983fe1269910f046**；原内盘design目录load-source-1-manifest **3d0f2fa800330c1f372bc46aef2830df56f66cb171366a250f72c9e2dc9f14cc**，只4小件70,343B，无binary/full payload/gzip镜像。parse_record与parent/H starts_message事实逐字同。

**仍缺且不能跳过：** bounded≤16KiB high-water/latest Attempt、allargs/token durably claim preghost、实际provider OS birth/cleanup Unknown门、各phase/base pointer/Meta/Ready都在原短row gate核epoch；不跨spawn/await/bulk IO持gate。现成功handshake后本地Err可先drop Journal再外层cleanup，Meta.exit也可能是cleanup Unknown后的exit1；它只退Khor slot，不是provider confirmed-exit证据，lockfree同样不足。R7必须保exit-present/providerUnknown仍拒新spawn，CLI --resume必须进入同一admission owner，不能旁路UI动作。base已pub→runpub失败要保两份历史但不Opened/Ready。共享恢复DTO/UI、显式Shell新run及前任关系尚未授权实现/实际验证，不自动归NEXT。NoLoad正常重挂草案不关闭hostdeath/offline/newShell/重启目标。

当前继续source-only。以后native需Root新正式单槽、actual同源producer/V/受影响有限exact，并按STORAGE外20GiB/内10GiB+峰值估算；单必要privatecopy批末退役，无旧大gzip双盘惯例。不得重旧Store13/64MiB/A/B凑整S绿。


## S3 InitialNew 与采样入口源码检查点（2026-10-31）

作者树 `fix/round2-recovery-contract` 已固定 `a71eac86`（InitialNew 非序列化 authority）与 `5f9b9da2`（pre-supervisor 同步 observer）；两片均经独立源审，Root 已顺序整合为 `ff01ab36`／`fe9f8cd0`。无新增 Cargo、内核 probe、host 或 App 运行。seed 的同 Journal lease／Control／key／epoch 与 cleanupUnknown 撤权、已发布 New 碰撞前拒均仅组件源码事实；真实 New、provider 生灭与逐 phase 发布接线仍由下一片完成，不能登记 S3 全部恢复通过。

canonical 收据：`evidence/round2/history/session-recovery/s3-initial-new-source-1/receipt.json` SHA `ac5d3cfb50eec6226ab058cc776c70037f2210dba7742ab65c5aa8d93be3691a`；`s3-spawn-observer-source-1/receipt.json` SHA `e7ec567567acb3dd504a570607748f9bb8acbadc6d5df3d09846388e6e3109ae`。源码补丁／checks／SCOPE／compact 索引内盘共约50KB；无 binary、gzip 或完整日志双镜像。observer 保原监督线程整块逐字 SHA `79f3022c8502d476571ce5941093021ffa26d6f6bfcde2a9f8182cc2f82a4e16`，Unix actual child ID／其它平台 None，不复用曾否决的后置 Requests mutex 采样。

Root `57551` 原8项组件已实际通过（8父＋2隔离 Python helper），保持原 f1 来源映射，不为这两个 source 片重复整套；OS 事实叶仅 host 编译、无实际 probe。当前真实 caller 小片正在接 `spawn_observed`、首次 New seed、Journal setup 错误仍留同 File、base／Meta／ready 同 epoch，以及同 Transport 清理。known-native preghost claim、公开能力 offer/action、ended Shell fresh SID 仍逐后片，未授 Core/TS/proto/Shell 写权。


## S3 真实 New／known-native 调用源码片（2026-10-31）

New 调用组合 `36a9b355 → d6f9b200 → c48ee71b → b1ccf4aa` 已非作者源审无剩余阻断：actual observed child、首次 exact Journal seed／同 File 错误保留、有限前缀后 same-epoch base／Meta／Ready、parent trygate＋原绝对30s确认 durable Opened、matching partial Meta slot 收尾与 provider receipt 独立。碰撞 quarantine 同 agent/home/fullID；任一现有 binding／Meta 冲突拒绝，旧高水与 result 保留。持续 attempt 写失败不短路同代 Meta.exit 收尾。未运行、不能挪用 f1 native8。

known-native 正常路径已固定 `2f850029bddc2ed233b7a2d55dd65284f70b61ad`，5源508+/93-，交 reviewer 待尾核，作者树 clean。parent 冻结 actual cwd/context/allargs 并 preflight manifest/binding、preghost bounded claim；child same File reservation／实际 host／Starting → 真实 observer provider → Load/Resume current-epoch base／Meta／Ready，parent 核 exact key；早失败／正常终态均同 Transport 结算。ForegroundAbsent 不冒 ProcessExit，只允许当前 Owned 同 boot/namespace/leader/group 完整事实；未退 Khor slot 还需登记 host 完整 Absent。Unknown/barrier 不借此清，legacy GUI 无 Attempt 不补造恢复权限。

当前只5份 rustfmt 解析＋diff0，非 Cargo/type/kernel/provider/App 结果。原 Python peer、H parse_record 读侧尾、generic close 均逐字未改。原 isolated helper 适配真实生产 prepare 与 Ready validator，保持原5s/原业务门；aborted unpublished New 现在明确要求没有 semantic binding，取代旧预绑定假设，no current/pending/Ready 和 reap 门保持。source 收据 `s3-known-wiring-source-1/receipt.json` SHA `f69490bd5e82d57097c07c664c2c94ee008f349f1181691dd4f40d0f958cc0b5`；HANDOFF SHA `6f4d031c68339b7af8912080726f97798052b60aacb18f78bed1228b451f4fa2`，内盘仅 source/compact 60KB。

后续仍需授权并实现 caller-stable token 的公开 action、权威 home hex 的 readonly offer、真实 Initialize Load/Resume 能力持久事实与 ended Shell fresh SID/previous 关系；当前 private CLI 一次 explicit 调用生成新 token 不能冒这些已完成。不运行新 native，待 fixed 源审＋Root 按20/10GiB reserve、显式峰值和单槽授批。

NEXT（Root明确不扩当前片）：generic Node.close → Live.close_session 仍按 GUI HostFile 的 host PID 发组 SIGTERM／删除 row，SDK provider 独立组，不能因此证明 provider reap／所有关闭入口安全。新恢复代码未改变或新增依赖该路径；本片 close 范围仅受控 GuiOp::Close。精确事实见 `s3-known-wiring-source-1/NEXT.md` SHA `36eef8baf9d1183fc413a2147dc1bcf63f90d1382ca608645fba9623ae295e12`。


## S3 能力、caller action 与公开 home 路由源码片（2026-10-31）

`2f850029` 已独审并 Root 整合 e313，Root 实际14117原8父＋2helper通过，证明其受控New/Load/Resume与private正常路径，不冒下面新public源运行。

- `5da45a04`（6叶）已非作者源审：每physical启动真实typed v1 InitializeFacts、同epoch/full native/effective launch的Attempt support、private稳定token早重放；旧/参数变事实Unknown，不改旧Meta且每次仍真实Initialize guard。receipt `s3-capability-token-source-1/receipt.json` SHA `a9e6e539a22c8d9b8be3e5551ff5dc0995970e4291d0508bd7e13af15a078c92`。
- `ad30da36`（2叶）已非作者源审：同≤16KiB文档内execution owner与最近caller action各一份，Join不撤原producer/report/exit权限，Shell Pending先durable、重复/不确定不第二B。receipt `s3-action-owner-source-1/receipt.json` SHA `8ee919fa7e23d60d0d08417edd7b4c1863c2089037d28a137ee63bd6c33acd78`。H共享serde/TS/目录DTO仅精确单pick，无手写gen。
- `3da8f84f`（10叶）公开实际接线已固定交审，**当前尚未非作者尾核**：三async Node API／IPC／peer exact DeviceId与执行侧复核，offer纯read_snapshot／无冷login-shell探测，actions非reading/0RTT；same-token claimed ghost，不重走fresh CLI token；先durable JoinPending＋nonClone本调用票，再锁外真实GUI2/cookie握手与samegate完整端点重核，重复/失败不变成Native；Shell新SID／默认shell／空command／previous-root先于新host，旧A不改，缺cwd typed DirectoryRequired与新选择intent。receipt `s3-public-actions-source-1/receipt.json` SHA `bc359eac8244b61d88dabdeb97b1bf5a2b4e5ddf960bec5d2ecb8aae35a3d640`。

本三片只 source/rustfmt 解析与 diff 门；无作者Cargo/typecheck/OSprobe/provider/host/CLI/Core/App/browser/gen，原实际证据不挪源。R7既定同例内补token/Join/Pending/late ticket/Shell/MessagePack分派断言、Store旧fixture补零写只读断言，均**未执行**。公开pure-observation／外部TUI无Meta offer范围仍向Root核定，不缩为NEXT；generic Node.close旧provider组问题仍按Root原决定NEXT且未依赖/改动。接下来需独审收口、Root唯一native同源生产闭包与真实S验收；不能以这次source checkpoint关闭完整S。


公开Meta-path后续收口：`3da8f84f → 1c11f947 → 91439aba` 已非作者源审无剩余该组件阻断；后两仅canonical home/token、pre-mutation allargs/nonce/epoch纯decision typed Refused，IO/store/claim/ghost不确定不被捕成Refused。末tiny receipt `s3-public-actions-source-1/typed-refusal-fix/join-nonce-fix/receipt.json` SHA `f584b65760ef1c4db9f0461b83886c1ab823f9c4b9c55e3d7df6ee9abbd9648e`。作者当前914 clean，无运行。环境派生参数变更时同token可能typed argsRefused，不承诺无条件历史回放，UI不可自动换nonce。

Root明确将pure observation/qualified-native无Meta补入原public入口后继（不列NEXT）：真实Observation ID是observe/hash(home,namespace,agent,fullnative)，现GUI/Attempt/registered journal多处硬id_for(native)必须透传独立logicalSID，不能偷造新tuiID。读/控制实际还包括ObservationKind重复投影、selected_content观察分支优先、Node与H Core静态prefix拒绝；只sameidentity且正式published GUI执行资格可切canonical/控制，未恢复仍拒。另defaulttui/observeNative/observeAcp最多3实际别名必须在同physical JournalReservation期间有限核exact-native owner/Unknown/Reserved，避免换alias绕过未知provider；不同时持两rowgate、不扫全局、不新registry，New冲突Unknown barrier只能落证明同native的oldalias。reviewer已给窄设计边界，Root/H必要写窗协调中，尚未改后继产品源。泛genericclose仍原NEXT，不因alias守门冒根治。


## S3 logical Observation／来源与消费者（2026-10-31，source-only）

`7313fe13 → 0799a603` 已非作者源审并Root整合：独立Khor逻辑SID贯穿三既定ID布局/claim/bind/publish/parentReady；物理Journal File覆盖三alias fence到target Reserved，child仅豁免自身exact claim；New collision cleanupUnknown隔离真实full-bound旧alias。receipt `s3-logical-alias-source-1/receipt.json` SHA `03035657d460227ddec95fd4ed42d79873acc2ec8926db7f10dc133d9ba7ced8`。不变默认New/CLI ID，不迁旧数据，不全扫registry。

`ed947b39 → c4cd4b08` 已非作者源审无该组件阻断：完整Observation/binding/GUI Meta/run/host/Attempt RunPublished资格统一canonical、列表和既有input-owner；Owned且slot未exit才可继续真实GUI2/current lease准入。remote列表使用同一reported快照替换旧只读投影，不删除元数据。删除门已修独审发现的TOCTOU：actualRef→samephysicalFile→仅既有alias目录短trygate，任何Meta/Attempt（含Reserved/Unknown/历史）拒删，检查和remove同owner；无row不建空alias。remote这里只删本机cache，不冒远机全局锁。receipt `s3-published-consumers-source-1/receipt.json` SHA `a08f83a0dec55ce0bb5eaf747f476364b2d85444a250109d711b5c89630434ff`；`delete-fence-fix/receipt.json` SHA `fef0201cf4b5148e605a5f50a97ff109075384db1a3220d77fdc9d87c66106c3`。genericclose仍NEXT。

`abad2bc8 → 15433371` 已fixed交独审，作者树 `15433371a14a56e4a16693c2cdf47ccbce756ecf` clean。8源实际noMeta/Ref/public logical native_at接线＋2源R7后继：公有三async签名/尾cwd不变，只有现private offer/native request+resident携原ObservationRef数据；home重验完整hash/home/namespace/agent/native与本机已有unique transcript/原cwd parser、同home AgentSeen或同native canonical binding/Meta。来源可读性不担执行权；only当前实际alias receipt Exited或ForegroundAbsent正事实可恢复，任何同源Unknown/Reserved/barrier仍readonly，mtime/EndTurn/list/Meta.exit/lockfree/PID miss不替代。action同physical File下重验→保存真实引用→sourcecheck/alias fence→原logicalID Reserved；child按前片重读引用，existing endpoint与lease才是控制门。無新RPC/parser/adapter/registry、无fakeMeta。

该后继当前**未源审放行、未类型或运行**。receipt `s3-observed-actions-source-1/receipt.json` SHA `2de2b0aa61511aa68a87353a7e50304b07abb68c21727c4fda295baeca225c07`；`readonly-source-tests/receipt.json` SHA `587aa08d0a23e258dbf59b6f29e37624c1b41c4dbdea4433e00a002331f8302d`。原R7同例新增实际private Node offer over模型owned files：未保存Ref、无Meta观察目标，Unknown birth+Meta.exit仍Unknown/None；可信adapter exit模型receipt后同SID Ended/Native；两offer完整私有root目录/bytes零变，无targetrow/ref，未pub input拒，sameFile才存Ref与真实Msgpack SomeRef。只是未跑的FS/owner模型子断言，不冒OS/SDK或新case数。全部作者轻检查仅rustfmt parse/diff0，未Cargo/gen/native/browser/App/provider；历史原8绿不挪到这些字节。下一保持checkpoint只响应review/compiler具体阻断，Root统一同源gen/types/Vite和必要原8父+2helper，不新增矩阵。


actual89125 受影响Node闭包：compile0＋原no-prior V过，原run1–4 reservation/Load各1/0/0，run5 R7在live.dir(realObservation.id).unwrap实际panic（recovery_attempt.rs2022），run6–8及helper未到。原false保留。根因是生产LiveKind::dir仍对所有leaf套clean_leaf的24上限，真实Observation tuplehash64被拒，故公有观察恢复入口也不可达，不是改fixture可消除的问题。

独立窄fix `a92ff5d91c2fda20c9603d5e4b784661f44c2a5b`（2源11+/1-）已非作者源审：只有kind=observe接受恰64lowerhex并保完整hash路径；其它kind原leaf24门不变，不迁移/截短/新授authority。R7原unwrap/原删除/无写/同ID断言全保，追加fullpath/short/uppercase及普通TUI64拒子断言。两parse+diff0，无运行/类型结论，authorclean；source收据 `s3-observed-path-fix-source-1/receipt.json` SHA `a50c50504b3b80481b1b7245d1c52b284dc1829049bae7e88591a7a779a5d77f`。Root将精准cherry后安排必要续批，原失败不回填。当前App/CLI最小验收计划只读梳理按Root要求暂停，未实现/未启动任何工具实例。


续actual14794（Root946）compile/V过，run1–7原父各1/0/0（R7全部观察FS子断言已到）、journal worker1/0/0；run8 firstResume worker在994实际capability断言失败：存盘{load:true,resume:true}与5da新增expected{load:false,resume:true}不符，后续早Note/close门未到。原false保；TMP状态为1自有event目录＋其1个.part文件，非strict全净，未自清理。

实际因果经独审：peer journal-resume原Initialize一直loadSession=true且resume={}；v1后getter原样持久化两能力正确。本次opening选Resume不应抹掉同时支持Load的事实。独立 `e6fd319546214331fe38d16adbd29da9a84ef744` 单叶source已非作者审过：只expected load改true＋3行能力/operation区分注释，原resume1/load0/new0更保留两cap可用时的Resume优先目标，partial origin/早Note raw+projection一次/Close/reap等所有门完全不改。不改peer或生产。receipt `s3-resume-support-expectation-source-1/receipt.json` SHA `bd82e7c8fc7903840532ee3a1a74edede06f5d7c1a3b82efe3fa4d0af5f41131`，1parse+diff0、无运行，Root精确pick后另授必要剩余续批。未重UI/types，App计划仍按Root暂停。


必要CLI消费者编译闭包：actual48124在agents.rs kind_of穷尽match缺Capabilities/Elicit/ElicitAnswered/ControlResult而compile101，未到运行/Vverify。独立 `886809fe4947118848e2ebf31930ae186b7973f7` 仅4显式诊断tag，已独审并Root40793整合；无wildcard、ignored用例/真实agent运行/生产变更，原false保。receipt `s3-cli-agents-variant-fix-source-1/receipt.json` SHA `45032c76a8fbe3a02307d035429eaae2b093fc01d1a611847ff768e70b6be944`。旧44421 normalbridge虽生产码同，whole-frame旧proof不能冒40793 current，需Root最终同源bridge producer，V规则不改。

完整S下一最小私有runtime场次仅计划（无driver/无实际实例）：`s3-public-runtime-plan-1/PLAN.md` SHA `e27856440ae800149cfb79461516b3949f149c255bd56d833a31ff8e08ffa6f8`，receipt `61b0f994b8c766bc7d2380a1d226401be88cd843892e07cc29057ee1deaa097d`。首场currentbridge真实Core JSON三pub＋实际CLIparent seed，journal-load liveJoin/已退后Load/同token一次；后依次原Resume/Unsupported、Shell目录新intent/单B/previous-root、两个paired私有home offline/hostdeath，最后L05/NoLoad270真App正常Quit两次与同host重挂/第二prompt。不新RPC/解析器/通用harness，不等真人才能验所有Node语义。binary/proof/driver与明确owned signal/runtime单槽仍待授，当前source-only；normalQuit不冒hostdeath/remote/reboot，NoLoad/固定syntheticLoad不冒真实模型记忆。旧NoLoadcopy与当前peer的全文件diff还含未执行resume早Note分支，必须准确重pin/模式映射，不能沿称当前仅Timer字节差。cli agents add会独立Init/list探测，setup计数与owner分层，不偷算New/Load。genericclose仍NEXT，计划不把hostkill/删row当provider已退收据。


C续actual22992编译通过，首Claude shim在209 reply snapshot断言失败（0.05s），仅pump已处理3配置＋1命令Note，未Codex/refusal；原false和TMP6自有条目保，不能称全净。实际不是notes.len旧count。独立源读fake assistant→result、shim先Note→turn、SDK incoming await通知handler/notes.note→再routeRPCResponse，确认RPC完成与异步EventStore消费者投影无同步；不能单凭log断定producer漏Note或已绿。

Root已授test-only消费合同适配 `d2a700e0007f0cb2e867cecb05ab5ddb7067dec7` 两叶fixed交审（无运行）：原pump持单bounded1 prefix request/oneshot，在Note提交后的下一轮用原remaining_through==0确认已完整处理原fence_events有限前缀；之后的新消息无需drain。EOF/Closed先关请求入场并fail pending/queued，scope/read失败与abort/drop均使调用失败，不假ACK。main3处与resume1处读notes前同步，沿用5s消费界；不改producer/fixture/RPC、无sleep/yield。静态inverse重建两parent全文为exact，因此原reply/usage/commands/permission/Stop/EOF/oldpast/缺历史及所有时限/ignored真实Codex尾均逐字未改。两parse+diff0/authorclean，receipt `s3-cli-shim-prefix-source-1/receipt.json` SHA `70c51109e4cdb4d28e62b00e69cbba2caa8d484ad93f87d61597b58704b2ca8e`。待非作者尾核和Root原C3新source准入，不重UI/Core7/Store，S bridge计划不扩。

`d2a700e` 两叶已非作者源审通过：independent inverse全文复原、handler后ACK/错误退出、bounded1/5s/原Ask并行均核，无新增源码阻断；仍未实际C续跑。

### 2026-10-31 first public Load runtime adapter (source only)

Root17f finalproduction9014 now supplies normalCLI a4c46e60 and bridge74cd6265; adapter borrows both existingprivatecopies and unchanged originalV proofs, does not build/copy. Fixed source canonical `history/session-recovery/public-load-runtime-source-1`: driver776a1600 / draft3446856e / source-receipt30f92aa1. Ready=false and slot=null; AST parsing only, no driver import/execution, products/tree unchanged atd2a700e. Producer environment values individually hash exactly to actual producer environment; finally-restored provenance context and env-i private runtime are separate.

Scene is realCLIregistration Initialize1 explicitly counted → CLIparentNew1 → actualCoreHTTP liveJoin/replay → one chat/canonical oldbinding → only exact registered+observedprovider TERM/5sactualhostreap+AttemptExited → publicLoad sameSID/newrun/Load1/New0 → stablepayload replay/no secondprocess → oldbinding page identical/newreceivedLoadbase. Final sameownedprovider/reap thenbridge plannedteardown, strictremaining/TMP. No App/Resume/Shell/twohome/realAPI or genericclose safety claim. 300outer, original5s consumer/reap, external20/system10reserve+1/.25GiB estimatedpeak,5GiBemergency unchanged. Source sent reviewer via followup; runtime remains Root-only new admission.

FirstLoad latest prepared source4 (source1–3 unexecuted history retained): driverbd1747c7/draft009a80b9/receipt78b655ac, canonical `public-load-runtime-source-4`, compact internal45KiB mirror only. Removed private HOME; actual HOME unchanged, KHOR_HOME/TMP private, existing KHOR_PATH prevents login-profile loading. Cold offer may honestly report Unknown; actual before/after Initialize facts true/false and post-explicit-action offer Load required. Reviewer's original5s absolute consume deadline/poll-clamp and current provider-parent/group checks fixed. Final two oracles compare full native identity and restored parent environment. Reviewer source3 no residual known interface/owner blocker; source4 final two-delta tail requested, not runtime authorization. Ready=false/slotnull; no CLI/provider/bridge/App/Cargo actually run, no product changes.

Source4 non-author tail complete: reviewer rehashed all driver/draft/receipt and producer file pins; no new source blocker. This is source admission only. Root still must grant unique runtime and pin ready/runtime_authorization; no actual firstLoad run yet.

FirstLoad source4 launch stopped at Python qualification before Scene/output/roles (Root launch-preflight-failure.json original pinned; no business case). Source5 exact qualification-only: drivere74a81c7/draft6be7acf7/receipt281e6c28, explicit OSlauncher0c9a/orig_argv0/currentPID ps image same; reported raw opt Python resolves distinct canonicalFramework/bin b502, both actualSHA pinned. Existing Scene class inverse byte check equal source4; no business/timeout/owner change. Revised driver AST only, one read-only sys/ownps observation exited0, no provider/CLI/bridge/Scene. Still ready=false/slotnull, submitted reviewer tail. Compact source only mirrored.

First true business attempt89017 (source5) failed original5s live consumer: actualsame-token SendCommitted+MaxTokens already arrived, adapter wrongly waited historical PromptRecorded. CLIregistration/seed0; public liveJoin/replay/chatOpen actualsuccess; actualreceived/accepted/canonical prove prompt1, predispatchwrite, same-token origin/range/twoDelta/tool/MaxTokens. Earlier method-counts.json was preprompt prefix, not final count. NoCapture/intendedproviderexit/Load reached. Forced cleanup EPERM aggregate has no per-killgroup log, so exactgroup unknown; originalfalse/remaining[]/directrolesreaped/TMP1dir1part preserve, no false strictclean.

source6 ACK correction fixed and nonauthor-tailpassed: driver67dea062/draftc665e5e7/receiptf62c93f7, public-load-runtime-source-6. Livegate only same-token send_committed+originalMaxTokens; original5s and allbusiness assertions retained; durablePromptRecorded fact now correctly compared throughactualCapture history mapper withraw prompt_origin token/turn/exactbyteRange, plus accepted.before_dispatch/twoDelta/tool/text. NoextraRPC/producer/fixture/guard edits. AST/readonly only; readyfalse/slotnull, no source6 runtime. Compact source mirror only.

70988 first publicLoad independent functional adjudication reached all actual Join/Load/token/oldbinding/receivedbody/two owned provider-reap oracles. Originalresult false solely two postReady49B .why: parent successful Ready deletes marker, ghost final EOF writes why because marker absent. Root exact-retained those identities with proof/lsof/postrun-cleanup31a4; originalfalse unchanged, TMP thenabsent, no rerun/product fix.

Next same LoadScene literal Resume/unsupported two serial privateRoot source2 ready-source, independently tailpassed: driver1230baff/draft8fa6fc10/receiptb0dc1bf0, canonical public-resume-unsupported-source-2. Reuses current17f/9014 two normal copies/no-prior require and exactPython; originalhelpers/5s/300/20+10GiB reserve+1/.25 estimate same. Resume actualdualcaps earlyNote raw+projection exact1/oldprefix/oldbinding, same-token no provider; NoLoad firstexplicit Refused and oneuser-newintent recheck Init-only, same-token noInit, exactoldbase/Meta/binding unchanged. Expected exact Readywhy retained only same-run actualended/observedownerabsence/regularuidnlink/content49/SHA/lsof/durable before+progress/final indexdirfsync; unknownTMPfalse. No Load businessrerun or newframework, no runtime; readyfalse/slotnull. Root currentApp alone, future explicitruntime required. Compact source mirror only.

Actualrestore57013 source2 failed consumer whole-page equality after Resume: first24HTTP actualdualcaps+New/prompt/oldexit/explicitResume1+samepayloadreplay/newcapture/earlyNote1all reached; old opaqueinput returned same3entry body/key/ranges/flags, only3record cursor+page.newer currentfilelen/modified490→900 differ (encoded2058same), raw first490hashsame. Not producer loss. No planned newproviderexit/retention/Unsupported reached. Originalfalse/forcedcleanupEPERM/why+eventDir+part retained.

Root oracle ruling + nonauthor review: Resume same-file append legitimately issues newer opaque cursor version; keep oldinputbyteexact and exactold semanticrecordbody/order/physicalrange/pageflags, do not parse or mutate opaque tokens. source3 fixed accordingly + failure-context traceback/lastHTTP, onlyAST. Independentlytailpassed driver5ef87b7a/draft066f7901/receiptf14df636, canonical public-resume-unsupported-source-3; Unsupported unchanged-file equality and independent Load-generation fullpageexact untouched. readyfalse/slotnull, no rerun untilRootadmit. Compact source mirrored; Shell/offline preparation onlyPLAN68dca54c, not executable/admitted; paused for this necessaryconsumer correction.

Actualrestore57844 source3 reached fullResume semantic/raw/prefix/earlyNote+same-token+actualplannednewproviderexit/bridgegone. Failed onlyretain480's single49B content: seedpendingturn true source1340 cancellationgrace52B bbfc2b03…d854, newidlewhy49Ba630. Originalfalse/cleanup_errors[]/remaining[]/2why retained; Root independently archives/adjudicates functionalResume, no rerun for packaging.

Next exact ONLY still-unrun Unsupported originalbranch source1: driverd4c87cc8/draft870283b3/receipt553b7c42, public-unsupported-source-1. Literal maincases[journal-no-load]/contractsame/aggregate1; noResume executes. All scene class methodsASTsame exceptknownexact49/52byte whyset; same-run/actualexit/ownparent/absence/UID/nlink/lsof/fsync/unknownTMP guard unchanged. FullUnsupported Init-onlyRefused/replay/userfreshintent recheck and oldbase/Meta/bindingasserts retained. No product/guard/time/proof changes, onlyAST; source submittedreview/readyfalse/slotnull. Compact source mirror only.

ActualUnsupported80962 failed real5s absence gate after typedRefused/capsfalsefalse/providerExited0/ancestry. Newghost21890/provider21891, bridge21864; oldMeta staysoldPID, specificwaitpid frompublishedHostFile notcovernewpreReadyrefusedhost. Realreceived Init3; methodcounter priorprefix2. Historicalowners hasnoSTAT/timeoutrows, soZombie notyetproved; originalfalse/cleanupEPERM/finalremaining[]/seedwhy retained. Productsource dropstdChild742 andwhyreturn768 withoutwait iscausalowner gap; notfixbyignoringZ/adding5s/endingbridgeearly.

Root grants one finite read-only observer source prep: public-unsupported-observation-source-1 driver7fbdbeb4/draft0b78bbad/receiptbd548fb4, independentlytailpassed. Onlycatchoriginalsame5s failure beforeexistingcleanup: exactself.rows andattempt/owner/bridgebirth+poll thenfreshps ONLYregisteredhost/provider withSTAT/UID/PPID/PGID/lstart/comm2s bounded; originalerror rethrow, noadopt/signals/waitpid/deadline/guard/productchange. readyfalse/slotnull; ASTonly, noactualobserverrun. SourceSCOPE givesexactChild ownership/realwaitnextplan conditionalonZvsrunningproof, notglobalreaperregistry. Compactsource mirrored.

## 2026-10-31 exact GUI ghost owner closure / next source-only validation

Observation98492 stopped at impossible sampler-lifetime requirement, no late adoption allowed; observation2 read-only exact Attempt+sameTransport Exited qualification now keeps unknown provider ancestry honest and never adds signal ownership. Actual85368 then independently proved same host38947 parent live bridge38859/group38947/birth09:32:30/STAT Z twice5s apart; provider38948 absent/Exited0. Originalfalse/why/EPERM evidence retained.

Product author5a8d2596 two-leaf checkpoint (gui_host + existing cancel tests), independently source-reviewed and Root874c7c68 integrated: allocate named waiter before OSspawn, owns actualCommand/Child through exactwait, bounded1 PID/ioError only; definite prespawn errors versus uncertain response classified, same30s total, ECHILD only already-reapedghost and not providerreceipt. No global reaper/waitpid(-1)/providerkill/close rewrite. Supervisor not changed. Not yet compiled/runtime on this source. Canonical s3-ghost-child-owner-source-1 receiptbd4bf09f; compact patch/index internal only.

Next focused Node plan ghost-child-native-source-1: driverc9500ad8/contract219cbec1, Root874, sole gui_host::cancel_tests::a_ghost_waiter_reaps_even_when_the_pid_receiver_leaves exact1/0/0, zerohelpers, two short private/bin/sh children. Reuse4045 originalV/owned driver with only early authorization gate. No CLIworker/stub/provider. Existing cancel/startup directlyhost_main bypass changed opener; no8+2/Store/Load/Resume rerun. Fulllockedhostmetadata/no-prior actualNode libtest/V/onecopy→finalSHA+lsof retire, original300/5GiB, storage reserve20/10 plus estimated6/2GiB peak. Source/AST/pins only, not executed, reviewer requested.

Current Unsupported pending proof substitution public-unsupported-current-preparation-1: run.py byte-identical observation2 37d8; draftba742f74 currently head874 with producer/binaries/environment null, readyfalse/slotnull; historical9014 production inputs removed from currentpins. Wait H actual currentnormalCLI/bridge/proofs/environment then final independentpin/grant. Entire original Unsupported branch/Init4-New1-Load0-Resume0-Prompt1/same-token noInit/recheck/ANY PID-or-group absence5s/strict unknownTMP remain. No extra binary copy, no product write/runtime. Shell/offline plan stays pending after this focused closure.

## Current874 runtime provenance and follow-up bounded scenes

Root reports focused waiter15850 actual1/0/0 independentlytailpassed; not wholeS. Actual38590 three currentnormalproducers green333.899s (budget900build, not this runtime300), CLI7ace6db8/bridge4378e065/GUIcd42b929; whole2340source/env/currentproof pinned. public-unsupported-current-source-1 driverbyteidentical37d8/draft7e08/receipt579f independentlysourcepassed; no old9014 proof ascurrent.

ActualUnsupported33238 false9.473s: firstRefused actualhost74117/provider74120reaped passed; same token replay and epoch3Refused reached. New74133/74134 freshqualificationps exit1/empty andguardrows absent, actualProviderExited0; failed onlyhistorical sampledprovider.executable `(Python)` Path.resolve==launcher assertion491. This is a no-absolute-path lossy display, not provenexec transition/wrongprovider. received.jsonl actualInit4/New1/Prompt1; savedmethodcounts3 wasoldprefix. Secondreplay/oldMeta/base finaloracles notexecuted. Originalfalse/EPERM/why retained.

Root-approved narrowreadonlyclassification source2: driverd6ca1e14/draft3e923c40/receipt7fd1f367. NonabsoluteimageUnknown observation only, absoluteconflictsstillreject; originalsampledparent/group, privateallargs/epoch/run/realhoststamp/sameTransportExited, ANY currentPID/group includingZ/reuse original5s unchanged. No signaladmission/ownedupgrade or positive retire_provider changes; inverseexact parent provesonlyassertionblockdelta. ASTonly, currentproducerpins retained, readyfalse/slotnull, sentreview. No production/build/runtime byauthor.

Shellfirstfinite source public-shell-runtime-source-1 driveree4273a5/draft0f386a35/receipt844d2e13 submittedreview, not executed. Reusesfixed37d8 owner/guard/proof HTTP helpers as subclass; realCore open_shell/terminal route forA markeronce onlyafterReady/actualexit, realSessionRow.execution_home, missingcwd DirectoryRequired/noSpawn/replay, newusercwd intentfresh epoch/nonce→B default/bin/sh empty startup/previousA rootA, samepayloadsingleB liveandended, oldA full existingsemanticfilehashes retained andactualexit0. Originalpublic sessions read owns existingPTYspecificreap (not GUIwaiter claim); bothactualPID/group absence5s. RealHOME/KHOR_PATH/noENV/BASH_ENV/loginflags, no usercommands. Separatetwo-real-paired-home offline source remains pending, not local-as-remote simulation.

Actual currentUnsupported4303 source2 terminal0/6.260s fullygreen and nonauthor tailpassed: Initialize4/New1/Load0/Resume0/Prompt1, bothintent/replay noInit, priorMeta/base/binding exactpage retained, first+recheck actualPID/group absence5s includingZ/reuse, strictcleanup[]/remaining[]/TMPabsent/sourceenvstable. This closes prior33238 lossy historicalimage consumer false through actualfullbranch; no rerun needed. No blanketS claim.

ActualShell12290 source1 terminalfalse3.466s at first open_shell HTTP400, calls0/noA/Meta/Once, cleanup[]/remaining[]/TMPabsent/sourceenvstable. Body was notcaptured, cannot assert exacterror text. Sourceclosedcause: open_shell_on_with usesresolve(machine) notresolve_or_here, fixturemachineempty wasnot supportedtarget. Necessarysource2 driver28e28ce7/draftdba4baa7/receiptd48b5f1a source-reviewed: backenddevices unique me.name suppliesactualinitialtarget, DeviceRow.id==A execution_home, originalbody/Ready/once/5s/35s/300 andowner guards unchanged; local HTTPError boundedbody/status/request observation thenrethrow, noretry/productchange. Source-only, Root owns nextslot.

Twohomeoffline source1 driver429de1bb/draftae739dde/receipt93ad4b2a fixedsourceonly andsentreview: two realprivateNode DeviceIds and actualinvite/pair/sync beforeseed; one actualNoLoadCLIseed, remoteexacthomeLive/Join/replay; stoponlytargetresident while sameGUIhost/providerstaylive, readerOffline/None+explicitUnknown, noLocalMeta/provider/newsetup andtargetMeta/Attempt unchanged; samekey/store restart withoutrepair, originaluncertainpayload replay Join sameSID/run/owner/noNew. Shared originalobservedledger+directPopen inventory, no lateadoption; secondhomeTMP/source/env adds strict cleanup. Original300/5s/31public/60pair+sync, reserve20/10+1/.25estimatedgrowth; original55speer unchanged andnormalcontrolledexit<50s. Exactownedwhyretain only originalproviderexit thenallroleabsence, unknownresiduefalse. Currentnormal874producer borrowed, nobuild/copy/runtime.

Offline scope does not cover hostdeath with providerstillalive: current positive retire_provider requires livehost parent relation and actualControl settlement. Do not splice blindhostkill/insuranceexit/genericNode.close to claim it. It remains explicit currentS unverified (not silently NEXT); no extra product or matrix automatically started.

Offline source1独审仅报第二pool先shutdown会延后原ownedcleanup；source2 driver40d40a31/draft50600873/receipte0fe0ce1已窄修并独审通过。先合并已显式创建的directroles/stdoutstderr交原finish做stop/Child.wait，后shutdown第二pool；poolerror写cleanup/secondhome状态/greenfalse，末seconds覆盖等池耗时并保原300。run AST与source1逐字同意图，全filepins相同，无业务/owner/信号/时限放宽。readyfalse/slotnull，未运行，独立两home实际仍欠Root授槽。compact source only镜像，无binary/gzip/大日志重复。


## 2026-10-31 Offline closed / host-crash exact source checkpoint

Shell actual9775 terminal0/8.756s and non-author tail passed: real device route, DirectoryRequired without spawn, fresh chosen-cwd intent only B, previous/root A, empty startup/default shell, same-token B while live and ended, old A semantic files and all exact PTY exits retained. No repeat required.

Offline94947 originalfalse was first conjunct owner-history growth (7→10), not local execution Meta: new resident metadata descendants had lossy images and were not proven agent providers. Source3 kept every new PID in the original owned ledger/strict ALL teardown, replacing overbroad zero-descendant oracle with actual reader Meta/HostFile/canonical absence plus unchanged exact remote owner/Attempt/counters. Offline27250 then hit actual pending bootstrap before seed. Source4 7c1c9429 admits one exact-literal same-ticket completion only within original total60s, preserves opaque bootstrap snapshots/identity, no new invite/claim/nonce, and second pending fails. CurrentJSON 1bee1eb8/receiptb3b6f397/manifestb1a03981 binds actual Root0f606 producer37775 current CLI7ace6db8/bridge4378e065, full22key producer env and original proofs; no old874 proof relabel.

ActualOffline63373 full branch is now independently tail-verified: invite/pair/sync and exactly one saved-ticket continuation; online Join/replay, execution resident gone while original host/provider remain, exacthome Offline/None and mutating request Unknown with no LOCAL AGENT fallback, restart same two private identities and replay the original uncertain payload to same SID/run/owner Join, no new setup (Initialize2/New1). All positive owned descendants, both child TMPs and exact known why retention satisfy original gates. Oldfalse evidence preserved. This proves two local private homes/resident loss, not physical machines/reboot or hostdeath/provider-live. Do not rerun.

Hostdeath/provider-live remains required S work. Author fixed only one cfgtest leaf at 687e0e07d4cd19b33717d620583cef7dd4baa4db, parent5a8d2596, clean. Exact ignored Mac case live::proc_identity::host_crash_tests::a_crashed_host_cannot_restart_its_exact_live_foreground (1 parent/0 helper); no Cargo/runtime. Existing proc_identity production+parser tests prefix17,550B SHA828163a7…98a3f byte exact; new module reuses actual CLI/Core plus unchanged stdin-stall peer, native precise stamps and known-PID-only sysinfo image/UID/argv. Fresh exact registered host SIGKILL only; same original provider stays Present/reparents, desired public Unknown/None+stable typedUnknown/no setup and oldraw/binding/Meta/Attempt/history unchanged. Original provider PGID cleanup is freshly qualified, direct bridge/CLI Child actually reaped, no scan/adoption/global reaper. Known crash parts are fingerprinted before, only exact originals/lsof after all exact roles gone are cold-retained without unlink; unknown TMP fails, receipt explicitly not child_tmp_absent. Business red still cleans/retains before original panic propagates.

Canonical host-crash-regression-source-1 receipt5a8924514790e19646d2b095a3ee0788be1778ffb52a56628d2d97fe4a0dff33, compact104,070B mirror only. rustfmt parse0/diffcheck0/sourceprefix check, no type/compile/actual claim. Reviewer explicitly followup-woken. Current source suggests offer_action_allowed accepts Owned and may yield Unknown/Native, with explicit alias fence later Err; the desired test assertions are not weakened and no production fix included. Normal37775 runtime inputs remain pending explicit runner pin/strict source compatibility; whole source-frame differs after test append, never counterfeit current require. Existing outer ALL-descendant guard is unchanged, no extra grace/whitelist; reserve20/10 plus estimated6/2GiB (not upper bound), one Node copy only at later admitted batch.

Host-crash687e独立源审已通过，reviewer已告Root可单pick：原前缀17,550B/knownPID native+image UID argv/host-onlykill/desired Unknown/精确finally cleanup与原crashpart冷留门均核，无新代码请求。仍不是runtime准入或已发生业务红，actual normal inputs null/后续严格来源兼容待Root。


Root43ff已精确整合Hostcrash687e和W019App初值（Rootclean），作者生产源不再变动。H current7832 fullnoEmit0/Vite0、f59ef37f/e21360e0已actual。准备两个原工具最小adapter而未执行：production-host-crash-programs-prepared-1/build.py b42f12b3、contract72fd0e9a、sourceccd41aca、manifesta612480c，复用37775去掉GUImetadata/build/copy和OUTretention，保CLI+bridge两独立no-prior V/actualrow/copy、bundle26、900/300、估峰8/2和reserve20/10、原qualitytarget/5GiB。当前frontend实际pins已填，sourceframe2340全等、emptydelta；readyfalse/authnull。reviewer正式源准入已过并告Root，仅Root可授后运行，不冒产物已出。

后继host-crash-native-source-1/run.py22c4347d、contractd2307d7f、sourceb58becec、manifest400526a6源准备交审。复用15850单Node libtest原V/owned/300/retire，只选新ignoredexact1父0helper；runtime_programs=null/readyfalse/authnull硬停。Root明确批准使用实际normal producer同embedded bundle+TAURI_CONFIG编译variant，避免用unbundled变体改normal NodeOUT；两normal原proof必须在编译前与测试前原样require，同producer_environment临时还原后finally恢复完整proof env/HOME，原normal/copySHA持续核。只custom stdin-stall实际peer，无builtin/API/model/App，不重旧8+2。保真实业务红为false、不特赦101，冷留已登记原part不冒TMPabsent；实际程序pins待新两normal实际绿/独审后仅JSON填。

Focused22c4/d230/b58b/4005 pending source亦已非作者源审无阻断；程序None仍硬不可执行。只在两normalactual绿及非作者尾核后填最终JSON再审，无需改driver。准备source/patch/compact合同索引实81,655B内盘，完整2340sourceframe一处externalcanonical，无binary/gzip/log镜像。


Actual current43ff two-normal producer55254已terminal0/166.720s且非作者尾核：result7e802228bf9da127a447bf8967a3f913cd5caf23ca608a5b1d4c01eecf5695e7，CLIefc16d3d/proofdf8763e9、bridge4378e065/prooffa4d4c81，current2340source/22wholeenv/188gen278assets/两原no-prior actualV/strictowned/TMP均同且净。Focused adapter代码22c4347d未变，只填actual programs/proof/source/env JSON；finalcontract1b6b006145719eb9170b22358d4a5e1b33e84f5b99a718d7a9706bd19e1158c9、runtime-inputs-receipt3d6eaf4a、manifest44e2a51d。初始d230 pending原件另存，readyfalse/authnull、无author执行。已followup reviewer finalpins窄审。当前程序不是旧37775重贴，原proof需在其actualproducer环境下原样require；后续测试前仍同门，无妥协sourceframe。

Final1b6b/3d6e/44e2当前pins已非作者独核通过并告Root：21来源pins、两actualproof/currentcopySHA、2340source与22wholeenv同；driver22c4未变，readyfalse/authnull。无进一步源码请求，实际newexact仍仅Root可授运行。


Actual focused host-crash35829 is a genuine business red, independently reviewed: current43ff compile0/actualV0, execution Unknown passed but actionNative!=None. Exacthost64092Absent/provider64093same fullbirth/context/image/UID/argv/PGID stillPresent(parent1). Explicitrecover/replay/finalhistory未到，不宣重复spawn。原resultfalse/cleanupok/原212Bpart exactcoldretain/outerstrict净均保，未回填。

原S授权最小source修复443bb9735aa7d6dd4bfbb4f2ab9e3933833ea30f，author khor-round2-input/fix/round2-recovery-contract clean，父687e。3叶51+/8-（live/recovery_attempt/session_recovery），与Root43ff原3叶父SHA全同。共享replacement_confirmed仅Exited/ForegroundAbsent，offer不把Owned当退出；alias/reserve复用，原Refused+NotStarted特例保。Owned不提前阻断原bothAbsent→gate核stamp/key/epoch→ForegroundAbsent路径。原finite-alias fence只把3种明确ownership unresolved分离为私有ephemeral decision，旧New/child原Err文本不变；public nativeTarget才返回现typedUnknown，真实IO/identity/corruption/CAS仍Err，不catchall/strings分类、不改schema/API/registry。原token/allargs replay先、同Journallease/3alias/短gate不变。原Mac回归/peer不改，R7原case只增Owned拒/Exited+Absent正投影与bytes不写，明确Absent仅注入模型非OSproof。

canonical host-crash-ownership-fix-source-1/source-receipt.json SHAfb049ef58f20abefba51a438bfc039aff1d5f3692c3b345e6721c65cbdc0a52b；内盘patch/compact20,880B。3parse/diff0、无Cargo/运行，已followup reviewer固定ref/receipt。后继Root两normal当前producer→同embeddedNode libtest，先唯一hostcrash ignoredexact1/0/0，再同artifact原R7 live::recovery_attempt::tests::bounded_attempt_tokens_keep_arguments_results_and_unknown_owners_fenced regular--exact1/0/0，无另编copy/重8+2。须注意原22c4 loop对全部case硬带--ignored，若纳R7应最小按已有case.arguments传透或单独已来源复用该artifact，不可让R7零case冒通过；未自行改runner。


443bb9733叶fix已非作者源审无阻断，Root精准pick fb01eb387035ee273e86df2a126b0a7cc152ed72 clean（现场核）。后继host-crash-native-source-2 source-only已交独审：driverc605475d613c297e0a69f37ac4d2732dfba6b2bd6a8ed706b58967bb45b64be4 / contract36cbea31b88300c3e61360ebc3527e558b2f5cb13363eec539527fe70b1260fa / source2e01e6b4 / manifest7b0b731f。只改22c4 driver一行透传已有case.arguments，防原R7被硬编码ignored过滤成零case。先原host-crash ignored完整1/0/0，再复用同canonical artifact/privatecopy跑原R7 regular1/0/0，零helpers、一次编译copy；原V/ALLowned/300remainingclamp/coldpart/retirement保持。新runtimeprogramsNone/readyfalse/authnull，须H当前fb01 normal3producer事实+独审后只填CLI/bridge；GUI不由本runner消费。原35829false/22c4/实际旧合同原件不动。内盘source/compact39,455B，无运行、无产品后续改动。

Source2 c605/36cb/2e01/7b0b已非作者源审通过：一行per-case参数/38currentfb01source/AST/原V300-ALLguard-coldpart均同；programsNone/readyfalse/authnull仍硬停。新三producer实际绿+尾核后只填当前CLI/bridge pins/env再交最终准入，无自行运行。


iOS simulator既定唯一compile-only入口已source准备并交reviewer：evidence/round2/mobile/ios-simulator-check-prepared-1/check.py c84bc75a/contract12c1e564/inputs f293b07b/receipt5e6dab5a，146行复用原owned entry+phase/stable/TMP收尾，300共享两phase xcrunSDKresolve→cargo check -p khor-gui --lib --target aarch64-apple-ios-sim --features devtools/locked/offline/j1/json。readyfalse/authnull，不执行任何xcrun/Cargo/App/install/download。已只读核SDKSettings26.5/versioned alias真实指向iPhoneSimulator.sdk、58个已装Rustsimlib、既有qualitytarget sim debug缓存、project.yml deployment15；realHOME保、SDK/developer/tool/source2340/gen188/assets278完整稳定门。无normalV/prepare/exe copy，checkfacts只真实CargoJSON build-finished+rootlib无executable，绝不冒手机/WK行为。6/2估峰+20/10reserve/5GiB/ALLowned/TMP保持，内盘compact18,384B（全输入一处外canonical）。后Node双case终态后Root另授。

Root已自行填source2当前20610 normal3实际pins contract5a955e44并获独审，作者不重复写合同。new producer canonical production-current-s-final-prepared-1/result13ec7568，currentfb01三normalactual绿已独审；CLI d7ea18c7/proof942f17dd，bridge e60c7614/prooff480fdb8。作者仅只读确认目录，不运行，待Root双case实际结果/必要具体阻断；当前author443bb973 clean。

Simulator c84b/12c1/f293 source已独审无阻断，2340源/188gen278assets/58stdlib/SDK配置工具helper逐SHA核同；readyfalse/authnull仍待Root另授。仅compileonly准备，不冒App/物理手机/WK运行。


Node81606双exact实际已非作者尾核通过：hostAbsent+同birth/providerPresent实际offer Unknown/None，显式sameepochUnknown/replay不增Initialize等，旧Meta/Attempt/binding/raw/history页保持全到；原R7亦独立1/0/0，同实际libtest artifact。原no-prior唯一Cargo/currentproof/source2340/strictowned/冷part与outerTMP分列/private退役均核。结果6dd87b6b…64e8/oracles49c98d74…3d8d；35829原业务红不改。443bb修复可记此有限当前组件实际绿，不外推远端daemon/所有OS Known/物理重启。

iOS source1 actual首次在helper import前置失败（ModuleNotFound native_provenance），无xcrun/Cargo/TMP/编译/产品失败；原checkc84b、Rootadmittedcontract04c53614、setup-failurec57cd46b原件保。必要source2新namespace ios-simulator-check-prepared-2/check.py2f651bbf、contract5f4bfcbc、receipt1f125e06仅一行proof注册名mobile_check_proof→native_provenance；inverse全文exact、inputsJSON同，AST+只读import原helpers成功且require函数同对象。新readyfalse/authnull、18,052Bcompact，未执行check脚本/SDK探测/Cargo。reviewer已精确源审通过告Root另授，原预算/guard/assert无变化。


iOS actual17655 terminal2/54.6518s，SDK成功，Cargo因累计owned ledger513超过原512由guard强停(-15/forcedtrue/noTimeout)，不是并发513或compiler失败。原result68b0ac5aab7a2a2c05e0ba0d9812dc479244bd86dfc5aa0772162b4481f9f0df、owners513/remaining0/TMP净/source稳均保。observe先setdefault再按len(owned)判历史总数；实际aws-lc92295直接250记录、ring91832直接28，104rustc与众多clang及子级，单Cargo job不限制串行C进程历史数。没有完成check声明。

Root/reviewer允许的必要单场预算适配source3：mobile/ios-simulator-check-prepared-3/check.py74d0ff12/contract5ad39152/receipta3dee556/budget-evidence8ceb3fd1，readyfalse/authnull。只3句assert原512/合同4096→此独立check进程模块MAX4096，结果额外透明记录budget及累计保留身份数，其余inverse source2全文exact、inputs同。全局helper/产品/测试没改，未删旧身份/ALL含Z/PIDbirthgroup/remaining零与cleanup、300/空间/单target都保；超过4096仍失败、不自动扩大/不warm分批，4096是planning余量非实测需求或上界。23KiB以内compact，无运行。reviewer已精确源审通过报Root可另授，此compile-only不串成Root独立C/W App验收前置。


Root追加授权H剩余系统Copy私有场source准备（非产品写/不运行）：mac-copy-preparation-1/run-copy-root.py06c1ea4b，仅原current20610 verifyexisting wrapper的namespace与scope derivative，inverse/AST通过，独立300s、realHOME/env13/actualsameproof/normalQuit/ALLowned/TMP保；ownartifact readyfalse。源码核HistoryRecords Read original·1没有Copy，仅分段Read+Export；真正HistorySearch Copy this text part取完整projected正文，historyFindAction明确64KiB cap。因此giant raw184613与wholeplain184375不得当同payload，giant当前正确为显式tooLarge，不可用Export绿或64K前缀冒Copy成功。

原native70c1的1201prefix485922B/SHA774e5bb1…d60现仍同，giant index1200 physical301309..485922/rawa4f78b7b…a4507含newline；wholeplain38148890…f4c73无newline。source-only路线先专用newprivatefile known38B sentinel正常Cmd+A/Cmd+C，再换placeholder防paste-noop假绿，giantCopy拒绝后Paste/Save仍sentinel；随后原H_MAC_ROW_1199搜索命中右键Copy→Paste/Save精确14B/SHA016ac216…537a。无clipboard read/pbpaste、不读用户资料、不改nativefixture、不创建输出file；当前不存在目标系统复制 验收.txt，须Root运行场才明示创建。oraclesc8041ad4、routee9a98f74、receipt2572b55c，27KiBcompact已镜像，交reviewer源审。此场仅拒绝预算+正常小part实际系统Copy，不冒>64KiB成功/Unicode巨Copy/Export或其它H全绿。

Copy06c1ea4b路线/namespace/原oracle已非作者源核通过：1201prefix、14B small与184375B giantplain SHA独核同，64KiB实际Copy路径正确。仍readyfalse/未App；拒绝保sentinel与真实Paste exact必须实际判，不以Export/clipboardavailable代替。


Root授权C009 fixture import闭包修（H不再并写）：原mac-c009-preparation-1/fixture.py bee76392 fresh Python-I仅processes()即ModuleNotFound native_provenance，未到ps/prepare/capture。新同目录fixture-import-fixed.py133ce7d8仅加sys.path.insert(0,str(path.parent))一行，inverse原全文exact/AST通过；原bee7与manifest604d2951未改。固定版fresh Python-I只调用processes只读ps成功631行/ownPID在，实际native_provenance49abd路径正确，fixture.json/before.json/pty-owner.json/ended.json仍全不存在。无App/PTY/会话/产品/ownership/guard预算/no-lateadopt改动。receipt22d19ae0与9,049Bcompact已镜像并交独审；未来Root须显式使用新fixture-import-fixed.py+原action参数，PREP同目录，不冒旧manifest已描述新文件。Copy准备不再扩。

C009133c/22d1已非作者精确源审通过：唯一importpath一行/AST，原fixture/manifest仍保，Root显式fixedfilename配原action，无App或行为验收声明。


Root新G产品fix精准pick d4b835ba8c21e71549a10c67f83cc6226ce788bc clean，仅PathField40+/2-，旧20610不得冒当前整frame。Root授必要frontend短入口source准备 integration/frontend-current-d4b835ba-prepared-1/run.py47f6209b/admission65dfb6eb/source056dd516/manifestc0793ca8，readyfalse/runtime_slotnull，未运行。复用actual7832 aa41仅新compactnamespace与allowed明确增加PathField/session_recovery两路径，inverse其余全文相同；current2340、原188gen/旧278assets前缀同。保原cc92 actualgeneration19priorpins/catalog9/tools/actualstaging，精确18source delta；对7832为PathField+3已审NodeRust，不生成新TS/不Cargo。精确commands.prepared列Node26.4.0 tsc --noEmit --project apps/gui/tsconfig.json（Rootcwd）→同Node vite.js build（guicwd），不npmhook/下载/新target。原600total/300step/owned/TMP/8+2估峰+20/10reserve/5GiB保。

Reviewer已diff/AST/19priorpins/18exactdelta/2340source尾核通过，仍非actual绿；H已获预期frontend-receipt路径并保持新normal3producer currentfrontend pin=null，须Root实际fullnoEmit+freshVite0才填真实receipt。现内部只32–34KiBcompact，2340sourceframe一处外canonical，无binary/大日志镜像。


Actual frontend85579终态0/15.752503417s，作者只读闭包核完：result1c596e9ff3ca5c6b2984c7e1a7a57bdc79eed451f9ec6903de7962c1ce1588e3、frontend-receiptbedd563f0c2675319cc3d410b2abafde8ee41ae0ee6084dd4d81d99dcd171d91，canonical integration/frontend-current-d4b835ba-prepared-1。fullnoEmit0＋freshVite0，currentRootd4b/source2340 before=after=receipt=currentdictionary，188actualgen逐字保/278新dist逐SHA同，19prior/18generationdelta/22环境与工具helper实际门稳定。两phase均无timeout/forced/interruption，3observedowners最终全退/remaining[]、TMPabsent/cleanup[]；7件compact小镜像逐SHA核（canonical.json按原设计只内盘pointer，不要求外盘同名文件）。无App/行为/新normalbinary证明外推，原20610仍其本源。

已发H绝对真实frontend-receipt路径+上述SHA用于当前d4b新native3绑定，未运行额外工具/改Root，不再gen或重fulltypes。G PathField actual修后原生可视仍待Root新GUI场，不以此编译绿替行为。


Root授W023 source-only短App准备（当前d4b/原native56763由Root持槽）：platform/mac-watch-preparation-1/run-watch-root.py d450d3c3，仅H currentG verifyexisting入口namespace/scope两替换，inverse/AST同；不stage、不App、不写EditorWorkspace/Git。artifact4bd1037b readyfalse/当前programproof/env全null，原producer-V/envfinally/realHOME/13runtimekeys/App同inode/300/ALLowned/TMP/normalQuit保。source-receipt4acf0329/checksd1899c3f/route07b6479d已交非作者窄审。
实际Rootfixture initial17B/sha b598780e、parent0755/file0644/currenttargetuntracked/旧dirty均只读核；需运行时只该path git add→commit--only+新privatebranch，核commit仅target与其它dirty/index未变。原生batch的cleanfile/tree/Git刷新源链已核，保持App前台不重开Reload/焦点fallback，有界10s观察。0555失败+外写不同V3必须保draft，0755/CmdS原CAS会clash，使用现Save mine freshpin再保存exact22B draft（f0db16ac），不能放宽digest/把自动保存冲突当watch失败。任何退出Root显式恢复该专用parent0755，空dir只rmdir已建exactempty。IME/remoteFS/G/terminal不覆盖；仍所有实际observations与currentApp admission待Root，不以wrapperexit替行为。

W023 d450d3c3 namespace/路线已非作者源审通过并告Root；path-only Git与原CAS冲突/显式Save mine补漏获确认。artifact仍false/null待current实际来源/同一App stage绑定，无运行授权或行为通过声明。


Root G-2actual68689 wide904wide/过滤/candidateclick/commit/blur正常Quit与ALL/TMP净后，授C009 next最小源适配。platform/mac-c009-preparation-3/run-terminal-root.py2463e4d1仅currentG run-g-narrow-2的namespace/scope；新same-inode KhorCurrentGLab.app/currentg环境与actual56763 CLI9deb/GUI73da/proofs/22env完整保，ownartifact33be0295 readyfalse待Root单场admit，不另stage。fixture.py554062c8仅同步before envfilename/runningnamespace+显式sameApp路径两更严断言，原uniqueSID/before/capture/ended/positivebirth-group/原/binsh/noLateAdopt均保；b593短printfscript逐字同、fixture.json只startupsourcepath改prepared3，未写实际fixture数据。source-receipt30c26fc6/checks142bf7f2/HANDOFFbc508d11已交独审，26,479B小件一处外canonical，无binary/大镜像。仅拟数次actualphysicalrow/OSC8动作，原300内及时blankline exit0/精确reap后正常Quit，不铺matrix；旧shell/21c8802b6acf81bb仅baseline，不冒已运行。

C009v3精确diff/AST/current56763两roleproof与22环境已非作者源审通过，Root已获结论；readyfalse/单场未授权，原noLateAdopt/uniqueSID/ended门未降，无运行/物理链接行为通过声明。


C009actual49137 result20e5ca5f…d9fc5正常exit0/ALL/TMP净、uniqueSID shell/760939224e72f59b before/capture/ended与原blankline正常结束均过；但Root物理行/OSC8点击未开目标，链接未通过，不先归提取器bug。source-only追加prepared3/POINTER-POSITIVE-CONTROL.md4cce79af与receipt775677fd（8现sourcepins），未改产品/wrapper/fixture/App。installed xterm Linkifier只mousemove建hover/currentLink，down仅记current/up需同link；canActivateTerminalLink还拒任何selection。真实DOMsmoke5335有mouse.move+waitcursor再click，buffer/callbacktests不证明native鼠标delivery。最短下一活SID只OSC8内短native drag→singleclick清选择→必要同点第二click，以underline/无selection/actualURLSidePanel分层判；无hover不先判extractor，hover已证且不路由才红候选。后同活PTYblankline/ended/正常Quit，原guard/time/cleanup不降，不用合成DOM或看死App。已交独审，无运行授权。


Root授C009v4 source-only下一短native正控：platform/mac-c009-preparation-4 driver3c33577e/helper45dd79e2/receipt4b420c3e，精确prepared3 root3→4 namespace与startup路径，b593script/targets不变；actual56763 sameApp CurrentG/currentg环境/roleproof原pin保，artifact094e2d4f readyfalse。新exclusive receipts不覆盖49137旧before/pty/ended。只有OSC8 native短drag→clear→samepointclick，成功且余量够最多一条wrapped-file physicalrow，300内及时blankline/ended/Q；原uniqueSID/noLateAdopt/ALL门无改。已非作者inverse/AST/currentproof-env源审通过，仍不运行、不stage/fixture写。

Copy source-only current兼容单独platform/mac-copy-preparation-2 run-copy-root.py6e4ef6f3/receipt670b2531/checksc5ae6285，精确producerfinal→g、qualityfinal→g、runtimeenvcurrentfinal→currentg、AppFinal→CurrentG四处；其它inverse全同，currentfinal-copy-root namespace与copy-oracles c804全文不变。原COPY-ROUTE全文保为历史内容路线、HANDOFF明确当前runtime由actual56763新pins/env决定，artifactd1d008e2 readyfalse。原sentinel保护剪贴板→giantpart预算拒绝保sentinel→小14B系统Copy/Paste exact，不能Export替代，无内容目标写/运行/Appstage。已交非作者窄审。


Copy prepared2 6e4ef6f3/c804/d1d008e2已非作者源审通过，实际56763两roleproof/22env已核，Root独占另授；后Root报告实际Copy连14B也失败，H正在独立修该已有实际失败，不能把source准备通过记成Copy行为绿。

C009v4 actual42114由Root确认正常exit0/ALL/TMP净：App93359/start14:11:14，guardpositive新SIDc842…/host95105/child95106，blankline /binsh exit0/ended全到。OSC8 label短drag后AXWindow Raise+ordinary click实际SidePanel ExampleDomain exact完整URL；file第二物理行drag后三行underline+ordinaryclick实际SidePanel完整path与R2_C009_ASCII_FILE_TARGET。UIobs320d18ae…48021/result3a91e7cf…c040a。与49137同生产源码，无产品修复；drag与Raise一起变化，不能唯一分解哪个必要。后可复用动作明确为真实窗口Raise后给xterm native mousemove/清selection再click；只实际这两种导航正向，剩dir/plainURL/Cmd/尺寸循环矩阵仍pending，不补猜测修复。


Root授input对reviewer作者S full-app-current-source-1做非作者审：273b/eb40的4receipt/16pins/三copySHA/proof22envhash及peer55→270唯一diff核同；发现观察日志scene.state跨HostFile先写/Meta后pub合法窗口及callback异常漏future原结果两adapter闭口，source1 review41882926保。作者source2 fragments82f3fa8a/contract56614bf1/receipt36abe865只app_phase独立修，input AST其余6topfn逐字结构同，5receipt/16pins均核；单Meta仅日志、guardsettle后始终future.result/原native保与callback优先已闭，非作者报告input-independent-review-source2.json56607aa1已Root。仅source组件通过，readyfalse非自运行coordinator：原Python/env/App同inode/当前V/300 envelope、AppA真实UI FIRST与精确预存App-only信号、AppB先看旧文再SECOND、cold真UI未知honestblocked不API暖场、正常Quit前正面provider退及最终ALL仍Root待接线/实际。不能标整S已过。

Root另授sim维护12584后源小修：maintenance/storage-20261031/sim-cache-retirement-1/retire-readonly.py da6f70b2/contract-readonly199a3206/receiptfd7d2488，只有status加--no-optional-locks与2合同filename；原851/24068contract/failed12584保。仅抽原inventory函数AST做只读stat，8精确readonlyGit资格前后fresh严格9be071e4相同（40017files/9594dirs/8links/8,530,496,469B），同inode/quarantine/result均未动。保护maps/fd-safe rmtree/原guard无放宽，readyfalse/authnull；已reviewer源审通过交Root单独fresh准入。未import main/运行retirement/rename/delete，无产品/Cargo/App写运行。


RootAA aa346f284730d87f2226419a279b3cea0067dd55精准集成Find修后，只源准备integration/frontend-current-aa346f28-prepared-1（driver396b87f9/admissiona27fe73a/source6bffba01/manifest0412ff64）。相对actual85579 fullsource恰check-history-find.mjs/history-find-record.ts/history-find-stream.ts3叶，2340源，188gen与旧278assets均同。driver仅compactmirror新namespace及allowed新增2paths（stream原已有），inverse其余全文exact，cc92原19pins/严格generationdelta20保。readyfalse/runtime_slotnull/no运行，原Node26.4 tsc fullnoEmit→directVite、600/300/ALL/TMP/env/source/tools/qualitytarget/8+2计划峰+20/10reserve/5门不变，不gen/Cargo/Node/App。已交reviewer优先源审并给H真实待产receipt路径，H AA normal3继续nullpin，未冒旧56763当前整frame。


AAfrontend396b87f9/a27fe73a已非作者精确diff/AST/19prior/20delta/2340当前3Hsource/原commands源审通过告Root，仍非actual，原188gen旧278assets不冒新Vite输出。

input依Root对reviewer作者S source3执行外壳继续非作者审：run65305b15/A2498e63a/B36e7a8ad/receipt7c0de992通过19pins/AST/fixed3+2ack/原source2保持等核。两独立原entry与每case共享300，A actualFIRST强oracle后fsync exactApp-onlySIGKILLplan、原non0保；B prior全role退后无API暖/actualcoldhistory与显式控件未知honestblocked，newprovider仍positive退后normalQuit，combinedALL/源码env/TMP原门保。报告 full-app-current-source-3/input-independent-review-source3.json5c20c234已Root，无本片源码阻断。两draft仍d4b/56763baseline readyfalse/authnull，AA已改fullframe，必须当前newproducer/frontend/proof/existingApp精确repin与Rootslot后才runtime；5actualRootCUA回执未发生不可伪填，当前不称完整S/App恢复已验。


AAfrontend actual26384 terminal0/15.413407916s只读实际尾核通过：resultbca04cc3cbb0c482dff4c101098fcd49e42e22858c9ecdd5620f9da6eeb35939/frontend-receipt770779dffc0352fa9d1a11e645248548d5020e86572267faf67dec4b06531a3e，canonical integration/frontend-current-aa346f28-prepared-1。types0+Vite0，无error/timeout/forced/interruption；currentAA2340before=after=receipt=实际Root，原188gen==cc92actual/new278asset完整SHA==currentdist，env22/工具/helpers/原19prior/严格20generationdelta/85579仅3source均实核。3owned freshknownPID ps全absent，remaining[]/TMPabsent/空inventory+lsof；内盘原compact7列件与manifest字节SHA同，canonicalpointer仅内盘按原设计，未复制大证据。新tail input-tail-review.json78d0662d已Root/H，可用于AA normal3最后绑定；不冒实际GUI/Copy/S恢复行为或旧56763当前整frame。未额外Node/Cargo/App/改Root，不再约用户限时人工验收。S source3保持待当前AA新normal产物与Appstage精确finalrepin后Root另授实际。


Root授S两个已结束Shell最小只读场准备：history/session-recovery/ended-shell-current-source-1/run.py320f4d69/contract727e5585/receiptd6e5195c，复用实际49554 mac-copy-inspect-prep3原LoadScene wrapper/data1 runtimeprofile/realHOME/privateKHOR/KHOR_PATH。Coreapi95/903与EndedPage源已图先检后读核：只sessions{by:recent}与固定两SID760939224e72f59b/c8424838303bbcd0的term_ended{id}，不需要lease/attach/ready。删Node/probe阶段，仅bridgeReady→3read→原正面identity terminate/directreap/ALL/TMP/sourceenv300。AA normalBridge/proof/source/env pendingnull且readyfalse/authnull，不拿旧56763冒currentAA、不Appstage/生成/编译/实际read请求。
原base.http会落完整response，本片必要私有read_summary同future/采样/4MiB/10s，直接仅存两行id/kind/word/hosted/attachable/can_control/execution_home/ended与screencolsrows/decodedlenSHA/at/4markerbool，recent只count；不先落全JSON、cookie、命令或screen正文。缺row/page/screen诚实报告，输出诊断本身不称UIrender绿；AX ended Terminal input disableStdin不能单独断丢数据。已交reviewer独审，源准备未import/运行driver/ps/port/改data1。Root实际AA native后才能finalpin与单槽诊断，后续一次cold screenshot不约用户限时人验。

ended-Shell320f4d69/727e5585/d6e5195c已非作者源审通过，3literalread/10s4MiB/同futureowner/不落正文/诚实缺screen门核过。保持AA bridge/proof/env null＋readyfalse，Rootactualproducer后仅JSONpins另授，不运行。Root另授input独审H下一CurrentG AA两hardlink刷新与Copyinspect3来源更新，待H固定材料；不执行stage/搬App/改qualityid。


Root委派input非作者独审H AA现有App/Copy/inspect准备已过：mac-current-aa-preparation-1 stage2ec38227/planbf48ef41/artifactc2e273a6，仅固定Appdir devinoUID和保原plist不写，原fresh owner/lsof/sourceV/envfinally/2hardlink/partialfailure保；旧CurrentGdir36364898与G immutable CLI36392932/GUI36393769/plist36393784全部identity/nlink/fullSHA实核同，未读构建中新AAcanonical/debugraw当旧基线。唯一App不搬/不复制/不改qualityid。Copy3 736fbe9f仅AAproducer/currentaa输出/currentaaenvfile三类inverse，其余body/原c804内容oracle引用/13env/300/ALL/TMP同。inspect4 a10b/3548 body逐字actual49554v3同，17currentTS仅historyfindrecord/stream2SHA变，原60/300/32/7RPC/cleanup保。新programpins/env null/readyfalse，报告AAstage/input-source-review.json9f682851…d33d9已Root/H；无作者stage/运行。
Root后报79806实际terminal0/284.878623s，三normalcanonical complete与5phase0/ALLTMP稳，H实际尾核中（此处仅Root事实，不代H尾核）。Root将据真正AA程序/proof/22env做最后JSONpin与单独stage/scene授权；不再重构外壳，source审点不冒实际stage或Copy行为已绿。


2026-11-01 input独立实际尾核与最后Copy提示收口：AA79806三normal实际尾核通过，input-native-tail-20261101.json09830afc＋combined cc07c622核5phase0/284.8786s、2340source/188gen278assets/22env/三no-prior实际row+copy+原require、ALL/TMP；同唯一CurrentG stage334723a1两hardlink与旧plist保、inspect36537 actual78569854小14B/gianttoo_large及cleanup都通过，不把inspect冒OS系统Copy。69036过夜原300timeout/forcedcleanup/ALLTMP空照存false，不是产品崩溃或业务绿。

S source3 A42497/B17620 input实际尾核11f1aa14通过其有限业务事实：A真实FIRST后精确App-onlySIGKILL→AppB先看FIRST再SECOND，samehost/run/fullnative且Init2/New1/Prompt2；B旧provider/host/seed全退后cold先读oldbody→真实控件一次Resume，Init3/New1/Resume1/Load0、新run/旧rawprefix与earlyNote1均到。两场原overallfalse仅A1/B2已知why，source/env/ALL/normalfinalQuit真到；非作者tail原件不改false。Root后精确metadata/SHA/creatorgone/lsof/exactset将scene/tmp同FS冷留diagnostics，receipt7f450784，不能重跑业务追包装绿。endedShell87058 API及62253真实cold画面已Root报告，当前仅Root事实未另冒input独立尾核。

Copy提示2叶39a19011已非作者review4d6192f7通过，Rootpick f23a167be48bb47d752f3f3a89ffa4efbef03a9f clean：catch只await同一个已经启动的body取实际readError，clipboard先提交、owner取消/换query拒绝原body、await后owner核保；无第二次read/新框架。现DOM红59580原前2通过后specificlarge15s超时，绿29753完整8组0/无force/ALLTMP净；新teststub模拟ClipboardItem包DOMException，不冒真实WK系统Copy已复验。

Root授最后f23源准备已完成：integration/frontend-current-f23a167b-prepared-1 runa1ec247f/admissionb880f51e/manifest088d94ad，actual26384为基只namespace+2Copy叶allowed增量，原commands/188gen/guard/预算/target/storage保，readyfalse。history/session-recovery/production-current-f23-prepared-1 buildc1e716fb/producer7496b045/admission699cec25/retentiona66b0dbc/manifest97ae3a8f：对actual79806仅compactMIRRORnamespace+2scope描述，inverse其余全文同；同quality881b/bundle26/900total300step/8+2计划峰20+10reserve/原5门/no-priorV/ALL/TMP。2340source当前f23恰2Copy叶delta；frontend实际SHA、retention-result仍null/readyfalse/无runtime。现GUI OUT仅fresh只读inode36397755/283regular nlink1/6603309B完整metadataSHA，不move、不造retention结果；Root需实际frontend后finalpin与原fresh owner/lsof/storage/identity重新准入精确留存，再唯一槽actualnormal3。作者未Cargo/App/Node/改Root或新cache。


f23最后App/Copy源准备交Root审：platform/mac-current-f23-preparation-1 stage原2ec38227逐字、plan9325a638/receipt946e34f3；原App36364898和plist36393784保，旧links仅actual79806 immutable canonical/fullSHA/devino/nlink2实核，新f23 artifact7e73a5e3所有result/proof/envnull/readyfalse。mac-copy-f23-preparation-1 run c922dd1f/scene0e87851e/receipt16a24334，仅producer/outputenvnamespace+giant-onlyscope字句；38B哨兵先真实复制→giant明确too_large提示→键入35B placeholder不copy→Paste/Save必须exact38B原SHA，不重14B/不Export替代。原300/13env/owner/ALL/TMP保持，无作者stage/App/运行。

f23frontend actual35387独立只读尾核4fe8b9c7通过：result0b2322bb/frontendb02797d9/types0Vite0/15.5458s，当前2340源前后同/原188gen/new278assets/currenttree、22env/工具helpers/19prior/22strictgenerationdelta/actual26384两Copy叶/catalog9+actualstage/7compact件全核同，3原PIDfreshps无、ALL/TMP净。核期间Root唯一native86710未触碰。

Root作者boot-identity-exact-f23-prepared-1源review c263e63b通过可执行范围：bfac450d对actual81606 c605仅删3句hostcrash冷part读取，原no-prior/libtest/metadata/ALL/300不动；实际只原regular pure exact1/0/0，无ignored/helper/真实重启，38sources/helper/tools/receipt实同。指出resources/restrictions/node_variant旧host-crash/R7/peer描述需Rootfinalpins时仅文案收口，不能冒真实boot。runtimeprograms null/readyfalse，无运行。

Root授device Rust库producer源准备：mobile/physical-library-f23-prepared-1 build97713c23/contract1c5ac8d6/inputse55c1af4/receipt a50d4420，沿ios-simulator-check-prepared-3原owned，现installed iPhoneOS26.5 SDK/58device stdlib只读核有；SDK→lockedoffline fullmetadata→build--lib aarch64-apple-ios/devtools，原300共享/单qualitytarget/jobs1/incr0/offline/ALL/Z/TMP和4096累计身份显式预算。实际CargoJSON .a fullSHA/noexecutable事实，不normalexeV/不copy库、不App/Xcode/sign/install/download/deviceprobe；Macbundle/旧TAURI先清后设原physicalPLAN round2physical配置，realHOME保，current35387 b027188gen278assets复用。外10GiB（8growth+2overhead）/内2，20/10reserve+原5门，runtime freshdu target+8<=50GiB/末actualdu<=50，作者未在Rootnative期间du。readyfalse/authnull，已交非作者源审，不宣actualbuild/真实手机通过。


f23 normal3 actual86710独立只读尾核94a6d82a通过：result091df6d970b223d7364cbede2451e51ca8522cce3ea4d54932499922d258c448，5phase0/339.57813s/noerror/force/timeout，2340source前后当前同、188gen/278assets=35387b027/currenttree、22原buildenv/tools/helpers/bundle26及quality881b核同。三actualCargoJSON unique row/freshfalse/normalbin testfalse/no-prior[]/messagesKey/原require当前全通过，target与canonical完整SHA同：CLI327a17f6/proof8a5c723f、bridgee60c/proofa530ba0b、GUIbf58434f/proofd4374506。原OUT283/6603309B/ino36397755已Root精确同FS留存且成员hash同，新GUIOUT同path不同ino，未把新OUT存在误判原留存失败；owned所有原birth freshsnapshot无、ALL/TMP净、8compact件同。小receipt input-native-tail-20261101.json94a6d82a0040613071460007c7bb070f95e0af9a9f35e466041dba0cdb7ca792，原父environment完整恢复。Root正stage/giantCopy，作者未运行/修改源码或触碰活App，不把normalbuild当Copy行为通过。


physical库非作者源审指出原sim沿用outer remaining仅exact PID/birth漏原ALL已登记leader组成员，input按授权固定独立physical-library-f23-prepared-2 buildbadafacb/contract4e8313b6/receipt6b30c417。唯一脚本delta补remaining=identity OR当前row.group in完整历史tupleledger中pid==group的leaderPGID，未删历史、不放Z/未知新成员、不加signal/adopt；原每phase/helper/300/4096/metadata→lib/.a证据/storage全保。inputs仅新namespace qualityconfig路径换新同字节，原97713/1c5原件保。AST/inverse通过、readyfalse/authnull，已交reviewer窄尾核，未运行/未改Root。


Root授权input接G Processes实际88916红（R2-024原定表头代表）：H作者树39a clean，GUI字节与Rootf23同，27Rust后台差异不消费。已读khor-design/README及中央STICKY-GLASS最终4d541e5e用户决定，graph主e7c未涵盖现路径后读现source：Processes已real subgrid body/header，非displaycontents陷阱；现mask零点head.top/恢复head.bottom，中段原8%→26%正文alpha可与label同baseline ghost，现check-face只查端点/blur存在遗漏paint。无产品改动/浏览器运行。

test-only fixed19b066c3e9c51820d5d605059cc92827ae461db9（H树）仅现lab/scroll-surfaces.tsx＋check-scroll-surfaces.mjs，source receipt history/sticky-processes-paint-source-1/1e6ce6f2。--processes显式单场3列40行沿真实data-load-top subgrid/原band/CSS，原36矩阵默认保；截图同字体label带可见body/visibilityhidden，header/body/scroll几何逐字同。actualblur pair仅差分诊断，临时关仅header伪层blur第二pair隔离原mask直接glyph，不把合法blur色差判红；4PNG与JSON需Root给freshartifactdir独占写，原owned300后授权才能实际，未运行。Node --check0/gitdiffcheck0，Root产品before4source SHA全同。

候选先外部未应用e8a3ec8a candidate-unapplied.patch：仅scroll-edge heldflush+现data-top-heads起点推进至居中单行label下沿（现token computedlineheight＋headingheight），恢复点仍head.bottom/原CSSsmoothgradient，剩余lowerinset自然恢复，不加solid/tint/line/shadow/clip/新globaltheme参数，Commit/TermPeek/普通标题旧路径。原sticky-fade补同例text-clear/同终点/scroll不变。H保持19b clean旧产品供Root先红；不在source-only期宣浏览器红/候选绿，手机16142运行槽不并占。


Processes paint actual91553原Node1/ALL空仅fixture React未commit导致pane null，未到机制断言，不算新增机制红。Root授权test-only到达条件窄修H04d57fca60dc034cce64f284ca0f7aea883ecc42：仅--processes分支evaluate前等待真实data-scroll-proof processes/data-top-heads visible（含attached），原像素/时限/产品门不动，candidate未应用。source2 receipt98c65309、script a789ffec，exactinverse单wait/Node--check0/diffcheck0/4productSHA当前Root同，旧91553原件保，未作者浏览器运行。


G旧源Processes机制红actual77782（RootNode1/noforce/ALL空）已读paint原JSON：同head/body/scroll几何下direct3165pixels/max87，actualblur5249/max41；Root两PNG确认visible旧BODY8与PID/Status/Command同baseline，hidden仅label，匹配native88916。91553setup红独立保。Root授权后将原未应用候选逐字落H fixed b3a2d0ab666888ca45975f357e4bf8ced592de67（parent04d）两叶7+/1-；scroll-edge只heldflush+data-top-heads把start推进至现居中label tokenline下沿，end=head.bottom/原CSSgradient保，其他headers不变；原sticky-fade同例补clear/end/scroll/fullheading。没有新色/token/厚底/clip/线/影，原paint test/fixture完整SHA仍a789ffec/4dd79675，阈值断言未改。TSparse0/JSsyntax0/diffcheck0，不冒types/测试绿。receipt43a8c311与patch3faf6bec已Root/reviewer；Rootf23源未动，作者未浏览器/Native。下一仅Root新namespaceProcesses-only实际绿＋必要原pure suite，不开全theme矩阵。


Root授reviewer作者physical-library-g-followup-prepared-3由input非作者源审：97703e45/991fe253/df797a5f独立pins/AST/精确inverse通过，预算仅本standalone library total/step600、phase剩余clamp、最终green<=contracttotal和scope透明字段；helper默认300断言/finally复原，原ALL tuple+leaderPGID/4096/50cap/jobs1/offline/10+2/20+10/5/.a不copy不normalV保。input-source-review.json3ee3e970已Root，原16142bd3d false fullSHA实核、partialcache不冒新fullcold；nullHEAD/frontend/source-gen-assets/readyfalse/authnull硬停，等G实际来源/资源/槽，不运行或改交互300。


Root已精确整合G test19b/04d与productb3a以及独立TerminalPane注释清理至a228baef4540ac4fe03c40879d4cb50f89da69c4 clean。Root报告actual91894原stickyunit+Processes paint0/2.44s/direct0/actualblur0/noforce/ALLTMP净，非作者actual尾核reviewer处理中；input不重跑旧S/Copy/C矩阵。

Root授a228最终前端与normal3源准备已完成并交非作者窄审：integration/frontend-current-a228baef-prepared-1 run8de45589/admissiond1e424ff/manifest11d4a1bb，仅actual35387 MIRRORnamespace+5路径allowlist；2340source相对f23恰4G叶+TerminalPane comment，原188gen与old278dist实核同、19prior/strict27generationdelta/原commands/600-300/ALL/storage保。history/session-recovery/production-current-a228-prepared-1 build57f7745e/producercfa034eb/admission4aff8014/manifestafad926f，对86710仅MIRROR+2scope文字；原881b/bundle26/唯一target/V每selected no-prior/currentrequire/900-300/8+2/20+10/5门/ALL/TMP不动。两source-receipt同233af917 full当前frame，frontend actualSHA及retentionresult全null/readyfalse/authnull，不拿旧f23顶当前。OUT仅freshreadonly actual86710ino36407744/283files/6603311B完整metadataSHA，retention2db0480c，无move/result/runtime，Root仍须freshowner/lsof/identity/storage另授精确保留。作者无Node/Cargo/gen/App/Root源码变更；physical源3等待此currentfrontend后Rootfinalpin，STATE只Root写。


Root授a228最后Processes App源准备已固定交reviewer：platform/mac-current-a228-preparation-1 stage原2ec38227逐字/plan48edce0b/artifact82643f75/receiptdf22a6e0，old_producer实际86710currentf23 immutable，两现App硬链freshfullSHA/devino/nlink2同canonical，App目录36364898/原plist精确不变，未stage或copy。mac-process-table-a228-preparation-1 run d37c37c4/scenec303bb9d/receiptcbf5c562，仅原f23 wrapper currentproducer＋currenta228-process-table输出env命名，inverse其它scope/300/realHOME/13env/V/ALL/TMP/normalQuit全同。所有currenta228artifact/result/proof/envnull且readyfalse/authnull，待4570实际frontend/新normal3后Root最终绑定与唯一App刷新/有限Processes真App复验；不拿f23冒当前、不重theme/menu/S/Copy/C/用户会话，不运行。


Root授原手机App source-only计划完成：mobile/physical-current-a228-preparation-1 receipt a1bd9457/contracte4967216/commandsb54ce0fc/sourceinventoryb1aaa9a8。run11已有主App target仅main.mm调用ffi::start_app并linklibapp.a，appdeps空/noPBXShellScript/no远程SwiftPkg，currentRust mobile_entry_point同symbol；29小source/resource/project输入524516B仅库存，不copy项目/库/旧DerivedData/App/UItests。当前main/header/launch/icons/emptyentitlement25file和旧模板相同；旧Info实际缺当前camera/localnet用途/语言等，私有stage必须用current gen Info而非旧Info，保权限，仅私有name/display/bundleplaceholder调整。旧Rust浏览group不当编译源；scheme正常BuildAction仅App、UITest metadata不选不跑。

本地允许readonlysecurity cms/findidentity核profile c9aa SHA1d51869a/teamQR8/wildcard/2027-10-02/publiccertificateSHA1与唯一valididentity匹配，canonical仅存公有指纹/计数不存DER/设备UDID/私钥，无keychain修改或sign。findidentity不证明futurekeychain签名访问；可能需用户解锁/授权，当前不预约不可用用户、不做provision/download/phoneprobe。

current4570 receipt6a420/188gen278assets准确pin，currentdevice库仍pending：actualresult/facts/.a路径SHA大小和App/signature全null/readyfalse/authnull。未来Root先批小stage+onecurrent.a copy（不hardlink可覆写target），再原owned genericiphoneosarm64/manualexactprofile/jobs1/link-sign build300、4096完整ledger/ALL/Z/TMP/source-env工具frontend保；无secondCargo/gen/Vite/Xcode自动包下载/UITest/install/launch。资源提案外3GiB/内2，明确不同旧2GiB artifact allowance，须actual.a大小后Root独立批准与fresh20/10reserve/5门/target50，不暗改预算/不当hardbound。源码计划不冒App/手机通过；无Cargo/Xcode/App/手机操作，Root当前55538没触碰。已交非作者源审。


Device库actual3007只读非作者尾核f5754aeeee4fbe759570d514adb358c8687dd967b581bbf8c3dd2917209b53e3：原result97965false不改，3phase0/noerror/force/124.732s；当前2340source/188gen/278assets/45706a420/SDK2/stdlib58/toolshelpers与actualuniqueCargoJSON default/custom/devtools/executableNone/freshfalse查同，actual.a1083289296B/SHA b926ba26完整同。194原identity或registeredleaderPGID当前全absent，targetbefore+8及末41.843GiB<50通过。TMP62节点=24dirs/38regularnlink1/0B fullmetadataSHA/lsof1双空已保存；SwiftPM/plugin名字与recordedSwiftbuild后代支持诊断家族，不伪造每文件creatorPID。Root后按fresh双审单次tmp→diagnostics/swiftpm-tmp冷留存ad8a9e3e，原97965false字节保持，非重compile或守卫例外。

Root授同physical-current-a228-preparation-1最小可运行stage+Xcode envelope已固定stage-build.py6d7530b3（173lines）/execution-contractb6e0cbc8/coordinator-source-receipt75b3935f/adapterdiff251730ab，已交非作者审。所有libraryresult/facts/review/retention/archive实际pins仍null/compile_use_admittedfalse/readyfalse/authnull，不自动用刚获事实启跑。库消费门要求原false仅unknownTMP、3phase0/errornull/currentframe+独审f575完整事实+Root冷receipt保原resultSHA/原TMPabsent/62成员metadataSHA匹配；不强改旧wholefalse、不重Cargo、不删旧诊断。stage仅29小输入/currentInfo权限私有identity＋current.a一次copy；唯一privateDerivedData，Xcode/manuallocalprofile build300共享含所有phase剩余clamp/4096全部tuple+leader组/ALLZ/strictTMP/freeze/realHOME。事实分层为stage和进程回收、linkmap/privatearchive源、arm64IOS非sim、codesign/profile/Appinventory；不normalV/install/launch/phoneprobe/下载或keychain自动权限调整。actual.a*2+512MiB=约2.703GB fits提案3GiB，仍Rootfresh3/2+20/10reserve/5门/target50准入。作者仅AST/sourceprep，无stage/Xcode/sign/设备运行。


App81494原false c92aa4c9/Xcode65根因已源实核：profile1d51869a IsXcodeManaged=true，与原Manual矛盾，先前sourceplan漏核此字段；run11实际只有unsignedsim（CODE_SIGNING_ALLOWED=NO），不能作为physicalManual先例。独立生命周期事实：首场SWBBuildService归原owned且forcedcleanup，不能冒产品App崩溃或仅签名错误等于完整清理绿。原日志/oldTMP/c92不动，不重devicecompile。

Root授followup-1源fixed e2ffbaaa/contract9f32bf73/receipt7290f438/adapterbd21e518。Automatic+AppleDevelopment去PROVISIONING_PROFILE/specifier硬override，禁allowProvisioningUpdates/DeviceRegistration，无account/provision下载；已读当前Xcode安装manpage说明该flag许可Developer网站更新，缺localmatch继续原失败停，不自动localcodesign fallback或keychain权限改变。原project/唯一DerivedData目录devinoUID＋stage30leaf全metadata/SHA/privatearchive1.083GB只读fresh快照517877f9，零restage/第二库copy/新DerivedData；新result/log/TMP/linkmap仅原目录followup-1。原库f575+ad8门/300共享clamp/4096全部identityORregisteredleaderPGID/Z/TMP/storage50/freeze/签名profile后验保，首场10owned亦fresh重验无，不忽略service。若Xcode0但原guardforced仍留false，随后只可Root独立受控读验App，不解析成功串改overall。readyfalse/authnull，AST通过已交reviewer，未构建/sign/设备。


Root接管leaf公开证书5行小修后followup1原7edc保，actual57058 Xcode0/BUILD SUCCEEDED但原guard回收owned构建服务forced，whole5d62false不改，23ALLgone/源env稳定/oldTMP保；不把Xcode服务当App崩溃/不改helper白名单。Root仅数据解析原native字典，native-result pin和reviewera124实际尾核供后续只读验证，未重build/库copy。

Root授input独立verification-1 source：verify.py4f1dcb59/contract537dbf45/receiptb9935730，readyfalse/authnull已交非作者审，149行仅原owned300shared/remainingclamp/4096完整tupleORleader/Z/strictTMP/空间3+2+20/10/5。完整App17nodes419051183B/fullmetadataSHA snapshotbf3f2aed，三ownMachO mainstub89856/debugdylib418183104/preview35024各同arm64IOS读验。已源只读实际链接布局78250fec：真实clang++ -dynamiclib输出exactdebugdylib/-lapp一次、6-L/18候选只有project私有libapp.a=b926；main.mm ffi::start_app→实际main.o/filelist，主stubmap仅主path+同debugdylib引用，绝不搬archive fullpath in stubmap错误门/不造新map。RootCparsedXcode0forced/原5d62false/23oldALLfreshabsence/a124tail与current.a/facts/current6a420源genassets强pin，App树读验前后严格同。仅lipo/vtool/codesignverify/display/entitlements/publicleaf9308/profile1d518/teamID，所有输出只new verification logs/result/TMP/DER，无build/stage/sign/install/probe/改App或复制DerivedData；Root唯一actual，未作者执行。


ReadonlyApp80494实际8phase（3MachO lipo/vtool＋signatureverify/info）全0/无force/App17tree metadataSHA同/ALLTMP净，原whole7824false仅certificateCLI1：空格optionalprefix被当target，非Appsignature红。Root实际codesign.1 lines50–53明确optional须--name=value；旧原日志与5d62false保。Root授权input focusedverification2源固定92e16aff/contract0c8aa1a1/receipt7684de47，only2新ownedphase公开leaf9308提取attachedprefix和entitlements/profile，原8actual80494各logSHA/contract/result/Appsource链强绑定780e5aba复用，不重lipo/vtool/strictverify/info。原Appsnapshot/源genassets/300sharedclamp/4096ALLtupleORleader/Z/strictTMP/space3+2保，新输出onlyver2 DER/log/result/TMP。primarymanpage摘录583160b4源只读，无命令执行；readyfalse/authnull已交非作者审。组合facts明确8oldactual+newfocused分层，不改7824或Xcode5d62，不build/sign/stage/copy/install/phoneprobe。


## 2026-11-01 原 E 真人 IME 当前入口 source-only 交接

按 Root 收尾授权，仅 evidence 新建 `platform/mac-ime-current-a228-preparation-1`，未改产品/中央 STATE。当前 a228 与 actual55538 两 normal 当前 proof 固定；唯一 CurrentG App 不另 stage。复用 current Processes d37c37c4 guard，两个独立 workspace/session runner 只 namespace/scope 差异，inverse/AST通过；原每场300、13env、realHOME/privateKHOR、ALL/TMP/source/正常Quit保持。artifact b6477610 readyfalse，实际 runtime env/空闲port/专用样例setup/用户新可用性待Root fresh接纳，未启动限时窗口、未创建样例文件、无App/模型/API。

receipt `9a5ace276240729d23cee469527f7a18ff11bae6803429776a501558ba4e6d03`；PLAN `010c7526d437b670437378f44c3daac19c0342e557695eae06214e89774fc070`。原 imported EditorSession 的专用文件入口与 EditorWorkspace 两场分列，真人中文候选/删除/中英/外写接收/切回/Shift选区撤销/保存样例明确；不把原R2-028 Codex键行重开、不重复ASCII三入口、不用合成事件冒真人。已followup reviewer，尚未实际/待源独审。

非作者源审已通过，独审小件 `platform/mac-ime-current-a228-preparation-1/independent-source-review.json` SHA `889b49d2e074ec6acc80f9a047d339b7aec67a676c9c5a878b2d5fe7516b4caf`。仅源准备准入，不是App运行或真人通过；本轮 input 无更多独立源码工作，等待Root在用户新的方便时间后按原两短场安排。


## 2026-11-01 发布决定与历史记录边界

用户最新明确授权“先直接commit到main并ship”。本卡以上实施、失败及阶段权限记录原样保留；本次授权及发布状态以[本轮发布记录](../ROUND-2-RELEASE.md)为准。原 Mac 真人中文 IME 与真实 iPhone 操作仍未验，按用户决定移到发布后，不登记为通过；本卡记录提交不代表已合并 main、完成任一 SHIP 目标或已发布。
