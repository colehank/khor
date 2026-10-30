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
