# Q037 ACP阶段2：有界GUI socket fanout

状态：2026-10-26阶段2已分3笔提交、代码树clean冻结；最终gui_host25/25与相邻兼容通过，待root非作者复审/集成。

所有者：fanout_owner。最终授权路径为Node gui_host.rs/cancel tests/fixture、Fanout模块及其测试、
ACP lib.rs/event_fence_tests.rs、共享Unix process叶及其新独立测试，共9项；中央STATE由root维护。
工作副本 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`，
branch `fix/quality-acp-lifecycle`；初始阶段1基线013efc40，最终依赖修复base91c80317、当前HEAD24b6478f。
没有新worktree/分支，没有main/ship操作；唯一Cargo/native槽已明确归还root。

## 最新交付（此前“待跑/未提交”段落均为保留历史）

工作树`fix/quality-acp-lifecycle`已clean冻结，base91c80317，HEAD **24b6478f**。
依序独立提交：

1. `8d916386`：Darwin exit-in-progress的EPERM最多100ms等待可观测退出，仍须WNOWAIT归属anchor和严格单leader成员证明；新真实回归2项。
2. `aa6011fb`：ACP实际forwarder有限barrier接口与4项窄测试，保留真实response优先于close。
3. `24b6478f`：managed ACP host、代次/公平完成、Control后Replay、bounded per-client writers/capacity publisher、首次Stop共用2s取消终止帧、Closed后有限fence drain。

已明确向root释放唯一Cargo/native槽，没有本任务遗留构建/测试进程。未操作main/ship。

| 固定实际验证 | 结果与证据Q037下 |
| --- | --- |
| 最终完整gui_host（含11真实父host、receiver fence1、packet10、parts2/command1） | **25通过0失败**，2专用worker函数由父实际执行；`acp-stage2-final-host-fence`，62.93s（含编译总84.65s） |
| 共享process（含新Unix2、原生命周期/Control） | **12通过0失败**，1专用worker由父实际执行；`acp-stage2-final-shared` |
| Node owned ACP/OS bridge | **12通过0失败**；`acp-stage2-final-node-acp` |
| Pi其余组 | **29通过0失败**，5专用worker函数由父实际执行；`acp-stage2-final-pi` |
| ACP lib（含实际fence4及既有lifecycle） | **13通过0失败**；`acp-stage2-final-acp-lib` |
| 原conversation | **19通过0失败**，2需要真实agent的可选用例未运行；`acp-stage2-final-conversation` |
| 最终正常Node lib（非test配置） | `cargo check --locked -j2 -p khor-node --lib` exit0；`acp-stage2-final-normal-fence` |
| 独立std packet socket | **10通过0忽略**；`acp-stage2-final-std-packet`；与Node packet10重复验证，不重复计独立覆盖 |

以上各组exit0/source_stable=true/无guard中断。后补private closed-fence guard后只重跑受影响gui_host与normal lib；
其余组件仍为同一Unix/ACP/packet字节指纹，未把未改组件再编故事。全套不同测试合计110通过；std packet10是跨harness重复。
全部原始红、setup失败、诊断sample原样保留，尤其`drip-green`实际Close失败不计绿。
最终所有13个host fixture PID迟后只读signal0均ESRCH，且每个Gone用例内部另有立即ESRCH断言。

最终机器交付：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/acp-stage2-delivery.json`，
含9文件SHA、提交refs、Node测试binary SHA/size、逐组实际命令/PGID/两盘minimum、source前后及清理观察。
最终binary SHA256 `f1fabfb0b4a0a4526a20576019fe82bebb0be23bef72438bc2804df55cd5ac05`。
最后两组最低外盘8,744,865,792B、内盘8,749,715,456B，均>5GiB。

### 明确保留的边界

- bounded的是每client outgoing backlog与bridge；SDK events/ops及fence barrier request仍有unbounded channel。
  有限fence不等于总队列/RSS有界，本轮没有声称全内存预算闭环；后续按真实flood证据单批处理。
- GUI accept/reader线程仍沿既有detached-process生命周期，未重构为全局join/有界Hello admission；
  本轮明确join的是fanout writers与host JoinSet请求/publisher，reader EOF显式退役writer。
- 未声称Windows runtime、物理设备、真实模型或重新构建GUI/native App已验收；完整Q037及全仓治理仍由root总账推进。
- shared target跨树旧ACP artifact曾造成setup编译失败。已强制本树ACP实际重编且前后字节SHA不变；
  最终集成仍须按root纪律强制第一方实际同源依赖，不能仅依赖顶层source_stable。

### SDK顺序及Closed尾部证据

锁定SDK3.5.0 incoming actor逐个await通知handler，再route后续response；本crate callback同步入第一队列，
transport future结束/被drop后driver才发Closed，故正常runtime callbacks不在Closed后再发Note等。
但opening阶段public target可先转发已排Closed再合成Ready；因此最终guard不依赖“Closed永远最后”的全局假设。
`acp-stage2-closed-fence-red`实际receiver Closed+Ready在100ms失败，未冒称真实SDK当前可构造inflight序列；
最终只在remaining>0时继续有限drain、0/未定时暂停，整gui_host25/normal lib复验通过。

## 契约与接线

- std-only，caller序列化一次完整length+body帧后传`Arc<[u8]>`，不依赖GuiNote/业务类型/新crate。
- `Fanout::new()`，`add(who, TcpStream)->io::Result<()>`，`send(Arc<[u8]>)`，
  `send_to(who, Arc<[u8]>)`，`remove(who)`，`close()`。send返回unit仅表示尝试入队，
  不代表远端receipt；序列化错误由caller处理。
- reader EOF/error须`remove(who)`，它标记队列abort并在锁外shutdown socket，唤醒空队列writer和OS write。
  单独drop一个TcpStream clone不等同于shutdown整个连接。
- caller须先send已编码Gone，再显式close。close停止admission，所有writer同时进入draining；
  共用2s grace后shutdown并join全部，不是每client各等2s。重复与并发close等待同一收口。
- 每client最多128 outstanding frames及64MiB outstanding bytes，**包括inflight**。queue满只退役该client。
  这是此host积压预算，不是全协议最大合法frame：当前`host::write_frame`自身没有outgoing cap。
  Graph先查gui_host.rs/host.rs，实际源码确认Replay总32MiB；单History由caller编码，完整32MiB+4帧在预算内。
- writer每次最多64KiB syscall、100ms OS write timeout，连续2s无任何成功write才退役；成功partial write重置idle clock。
  正常frame没有2s总期限，持续前进的大replay允许超过2s。close grace只属于显式收尾。
- registry/queue锁内没有OS write/shutdown/join；send不等待writer。writer只持Shared Arc，不持Client/Fanout，
  无强引用环。completed handles由下次add回收或最终close join；remove保留join归属。

## 验证与边界

独立harness只声明此模块，`rustc --edition 2024 --test`，输出证据根
`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037/fanout`。
首轮rustc成功但当时尚未运行：随后df观察外盘5,034,296KiB（4.80GiB）低于5GiB线；
已向root报告并暂停。root按manifest窄清理后明确恢复小型std编译与测试；无Cargo/Xcode操作、无本agent遗留编译进程。
首版rustfmt因tests文件尚未建立先失败，建立实际tests后格式化成功；这不是业务测试红。

已写8项真实loopback socket场景：慢client不读时主线程send迅速返回且另一client收后续消息；
32MiB+4完整帧逐64KiB慢读超过2s；byte/frame预算均含inflight；超大单帧提前退役；
Gone顺序完整且并发/重复close幂等；多慢client共享close deadline并join；reader EOF退役idle/blocked writer并由add回收。
上述8项均已真实执行通过0失败0忽略。budget测试显式等到首帧已由writer取走且仍outstanding后再发第二帧，
不靠40ms调度假设证明inflight计数。root之后仍须真正host目标红转绿与集成Cargo验证。

## 最终固定源码交付

最终证据`Q037/fanout/final/{source-before.json,source-after.json,result.json,run-0.log,run-1.log,run-2.log}`。
原首轮8绿证据保留在父fanout目录，不覆写。最终rustfmt check、rustc均exit0，8项真实socket测试6.17s通过，
source_stable=true，没有ignored测试。

- `gui_host_fanout.rs` SHA256 `dca382eb7dd3bf30e73bb9eb19d4f95addcb207e852a64f1d4e64ab94dc0a582`
- `gui_host_fanout_tests.rs` SHA256 `f0c32e0c276cfc14f25b3b0fd3cffc8399c73423652119ff09a428e64ce71716`
- 最终standalone binary SHA256 `89e8587d565ef978e4782717126cf52c0ad2341db4ce411b714eea14cccce3ec`
- 编译/测试前后可用：外盘8,115,224,576/8,113,061,888B；内盘8,184,377,344/8,183,697,408B，均高于5GiB。

仅这两个新源码和此卡由本任务编辑；gui_host.rs并行dirty由root持有，未add/commit/覆盖。

## 角色移交：host completion / Fanout接线（进行中）

root已接收两个新模块；随后只读独立复审root transport候选（host SHA256
`d5e5ded7a5383ce1b0c0fee7b97434df4ffbd1400080afbe250a9d0614d4b3a5`），发现
Replay在Control pending时误把begin_operation拒绝视为process retiring，以及incoming常Ready导致completion饥饿。
root接受两项阻断，并将gui_host.rs、gui_host_cancel_tests.rs、fixtures/acp-cancel.py独占写权移交本任务；
允许Fanout接线，禁止先提交root既有transport候选。其他现有文件仍不在授权范围。

预定：generation绑定completion、优先确认同代次ACK、有限notes fence后发布Turn/History/ControlResult；
Control期间Replay排队，完成后启动。已注意ACP额外events forwarder使简单host events.len()不能证明最后Note已送达；
向root提出在forwarder内增加有限批barrier再回执，等待额外路径授权，不用yield/sleep代替协议顺序。

真正新增回归准备覆盖合作Cancelled最后Note顺序/超过旧grace同PID/下一轮、Control→Replay队列、
持续notes/ops公平性、慢GUI隔离与Gone立即ESRCH、control timeout退役；仅准备源码，Cargo/native槽仍归root。

### 跨层fence窄接口授权与当前冻结源码

root另明确授权`crates/acp/src/lib.rs`与新`event_fence_tests.rs`，作为独立接口followup边界。
实际锁定SDK为agent-client-protocol **3.5.0**，Cargo.lock checksum
`9bbe69c878aef64a3d92041e256e75acdb1bf2ea8e55ef3313e69dbb2f5bd75a`。
已直接核对本机registry源，非只依据旧replay注释：

- `/Users/zgh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/agent-client-protocol-3.5.0/src/jsonrpc/incoming_actor.rs`
  SHA256 `95d564d8abb2f0e6814d2d9800df06e22b29bbcda993a40cba3c8e8a2cb7362c`。
  194–224行Notification分支在for内await dispatch_dispatch完成，226行后才处理同actor的下一Response。
  因本crate on_receive_notification闭包仅同步notes.send而无await，response被路由前，该消息前的Note已入第一队列。
- 同SDK `src/jsonrpc.rs` SHA256 `5e3da6af16cbef049f50b2a71ae8f5b1d5a861108f0b4d8ba8dc01eecd01ebb7`，
  block_task约7052行等待response_rx，在返回响应值前ACK；caller处理阶段不再hold SDK dispatch。
- 上述保证只对应已锁定SDK及本crate当前同步callback，不能推广到任意SDK版本或自建异步转发。

现首版：实际forwarder优先barrier，快照原队列len、有限同步转发后回执；正常EOF已drain成功，
runtime销毁/任务取消有显式失败watch唤醒waiter。Handle::fence_events不走while_open，保持已交付RPC与关闭的区分。
host请求task收到真实RPC result当下同步complete_operation(ticket)，再发Resolved；Control timeout发Abandoned、从不ACK。
主循环核对同opaque ticket，Fenced只决定有限外层events.len()栅栏后的UI发布。Replay在Control期间去重排队。
Fanout已caller序列化一次接新packet模块，reader EOF显式remove，Gone后close等待drain/join。

新增真实host父场景源码6项：合作取消越过生产10s grace仍同PID/下一轮、Control→Replay、持续notes+ops、
慢GUI与第二GUI Stop、control timeout未知退役、握手refusal/timeout即reap；外层worker各有45s界。
Control/handshake timeout测试用同一私有host实现注入短duration，生产公开入口仍28/120/10秒。
新ACP窄测试4项使用实际start_event_forwarder，覆盖有限批、持续后续流量、EOF/target关闭、未poll runtime销毁。
当前**仅解析/源码准备，尚未Cargo编译或执行这些新增测试**；旧8/8只属于packet模块及前卡指纹。

root再次发现外盘低于5GiB，已要求冻结源码、暂停新编译，根因归属排查中。冻结SHA256：

- gui_host.rs `3e449af5da1e7f31f0b86cec45758d6271489b914895a61e43073ce9fc26a035`
- gui_host_cancel_tests.rs `afdced69954b161487a31f7f150c817e808f6f49b65194866374c7c002b4c273`
- fixtures/acp-cancel.py `d545608d477b4bae83b0cb6cfcc1b19a93a66be939f6af8d354cdf0f4bf57117`
- acp/lib.rs `d7d011708f494c562f0082c43f477861bd6f4efe1c8cb6c1df0e6acd4a82d771`
- event_fence_tests.rs `b32f39185d7478bf23e553371161b866b06abcefa6aec6c5245d29300c016ace`

未提交root既有transport候选；没有本agent新增Cargo/Xcode进程。

### 补充验收：whole replay多小帧

root确认必须补合法32MiB总量由大量小History帧组成的真正replay验收。当前host单次同步for瞬间发布，
128帧预算可能因producer burst而退役快读client；已有单32MiB帧绿不能覆盖此情况。
资源恢复后先运行该新回归，若真红再做有界可让出publisher/游标：full表示replay backpressure，
用实际capacity通知等待，主loop仍可处理Stop；不得只增128常数或yield固定次数猜进度。
root已允许按证据扩std Fanout显式QueueFull/Closed与空间通知，正常live流超预算退役语义需单列。
当前仍冻结首版源码，尚未增加或运行该额外场景。

### 首轮实际host/fence绿与whole replay真红

- `Q037/acp-stage2-interface`：Node真实接口编译exit0，47.48s、source_stable=true。
- `Q037/acp-stage2-host-first`：同源实际7父场景通过0失败，44.29s；2个ignored专用worker由父实际启动，不能当未跑。
  包含原不合作Stop/reopen、慢GUI下另一GUI Stop、持续notes+ops、ControlReplay排队、合作取消最后Note顺序及超过生产10s grace同PID下一轮、
  握手refusal/timeout返回即reap、control timeout未知完成退役。最低两盘6,775,156,736/7,802,679,296B。
- `Q037/acp-stage2-fence-first`：实际forwarder4/4通过0忽略，含编译5.65s、source_stable=true。
- `Q037/acp-stage2-many-replay-red`：仅加test+fixture、不改当时生产；16,384个带序号小History、真实GUI持续读、总量<32MiB。
  编译20.19s成功后，实际worker0.86s因HistoryEnd前Disconnected真红（父也失败），不是setup或空跑；source_stable=true。
  已保存当时全8项source实际快照、marker、日志。fixture PID80619后signal0已ESRCH，无遗留agent。
  最低外盘6,377,967,616B/内盘8,104,611,840B；原128帧预算未调大。

### 真红后的候选修复（尚未运行）

std packet模块新增poll_send_to：replay生产者在预算满时等真实空间释放Waker，不把有限producer burst判为慢GUI；
写完成、abort、close均唤醒，waiters同样限于帧预算，不能新增无限等待队列。live send/send_to仍满预算即退役该连接。
新增2项实际socket源码测试：inflight占满后capacity唤醒且完整tail、close唤醒Closed/超大单帧立即拒绝。

host Replay编码一次shared bounded blob、各reader独立游标await capacity；聚合publisher在JoinSet内，
主loop不await它，Stop通过watch取消余下history，再按序HistoryEnd。operation已在真实Resolved时ACK，publisher不再重置/延长kill期限。
完成Published票据只关闭对应replay delivery，不ACK任意新operation。正常后续Note在publisher阶段走live而非丢入旧buffer。
若driver已Closed，保留已知完成的有限history发布后再Gone；writer失速仍2s自行退役、publisher被唤醒退出。

root临时接回唯一native槽做Q043实际App资源观测；本任务已明确释放并暂停新编译/socket suite，等待归还后固定候选复验。
目前上述修复仍是源码，不能用首轮绿或原8绿代替本候选实跑。

### Capacity修复后实际固定源码验证

`Q037/acp-stage2-capacity-socket`：std实际socket **10/10**通过0忽略，6.23s；
包括原8项、真实inflight满后reader读取触发容量Waker并完整tail、close唤醒Closed及超大frame立即拒绝。
source_stable=true，两盘最低外5,861,953,536B/内8,500,912,128B。

`Q037/acp-stage2-host-capacity-first`：最新Node真正Cargo编译33.67s，完整host **10父测试通过0失败**，47.54s；
2个ignored专用worker函数均由父实际执行，共10次隔离worker调用。新增whole many-small-replay已由前述Disconnected真红转绿；
UI publisher取消新增明确ControlResult(cancelled)先于HistoryEnd，不把部分history冒充完整；关闭backpressured publisher也实际退役/join并Gone即ESRCH。
原合作/公平/ControlReplay/slowGUI/timeout/握手/reopen矩阵全部保持绿。source_stable=true，未触护栏，
两盘最低外5,606,490,112B/内8,419,266,560B。运行结束已向root归还槽，允许闭合rcgu manifest清理。
证据含固定源码快照、每场景实际marker、前后hash、binary.json与cleanup.json；未提交。

此轮生产/测试SHA256：

- gui_host.rs `69e292316c69f61a87cb0fd4f045e6bf7fcdc9fdd91df6483410deb994b5ac51`
- packet模块 `914dbce7959ad4e24129dba8cfe02daa855b806f910a5a1868f940964f546653`
- packet tests `b438095680aad83a89ab764ee19e97cf8de0406fb73d0a8c3cf392cea5a591ee`
- host tests `7db068adc6c2bc7f2a7c66ecd883a5a7c354ff7cbbd7a7449578616e84665f4d`
- fixture `532497695702da594df5530235cf2cba3fe3d5cdf6c09fa63c0c80af243e54d3`
- ACP lib/fence tests仍为前述d7d01170/b32f3918，实际forwarder4绿对应同一源。

源审保留关注点：UI Stop已唤醒watch并且主loop不阻塞，但最后cancelled/error+HistoryEnd仍等实际capacity；
极端持续小进展writer可能长期占满budget，导致UI取消收口较慢。已向root提出只给“用户已取消后的final error+End入队”2s grace，
到点只retire该client，正常大frame仍无总期限；当前已绿源冻结，等待root裁定及资源清理后再做窄改。

## 用户恢复后的drip取消边界（源码准备，尚未实跑）

用户明确“继续”，root恢复本任务独占写权并要求先真红再窄修。现场仍为013efc40，7项dirty及暂停前所有SHA一致；
已绿生产gui_host SHA69e29231/packet914dbce7保持未改。root fresh preflight清闭合rcgu后外盘6,725,099,520B，
先把唯一Cargo槽授Q039-A；本任务只增测试/fixture，不编译或运行socket套件。

新`stopping_a_progressing_drip_replay_has_a_bounded_delivery_close`真实host回归准备：
fixture先发送30MiB大History再4096小尾帧，总Replay仍<32MiB；目标GUI用raw TCP接受大输出（不误套host的1MiB输入read_frame上限），
SO_RCVBUF设64KiB，每64KiB/80ms持续消费。读过5MiB且大首帧仍未完成时，publisher的128frame已满；
另一正常GUI发Stop。要求4s内replay busy解除、同ACP下一Prompt实际成功，并断言Stop后drip reader仍在进展，
避免把无读stall冒充本边界。未通过前不修改生产、不抬budget、不把正常大frame套2s总期限。

预定窄修仅给“收到UI取消后ControlResult(cancelled)+HistoryEnd入队”一个共用2s grace；到点retire该client，
唤醒capacity waiter并让publisher退出。正常未取消完整帧仍由idle/stall deadline管理。root正非作者源审当前10绿候选。

恢复中root在Q039-A正常lib编译发现阶段1futures仅列dev依赖，原测试构建掩盖该normal依赖缺口。
root修复d20d7121已cherry到本树 **91c8031780103f0cac5ca6d16c564d9456d77474**，只改Node Cargo.toml和CI check，
无新版本/锁变更。Node Cargo.toml SHA256 `3d42fd65cc4a530f6d3933e1d4c5ec93dbaeb010a210ca8a9b6da7509e71a9fd`。
本任务host/packet生产指纹未改；此前10绿仍仅老manifest的lib测试，不代表normal构建通过。
后续run_guard已纳入新Cargo.toml指纹，最终须补正常lib/integration编译覆盖；等待Q039-A归还槽。


### 上游模型容量中断：保留恢复点

只读collaboration.list_agents现场显示root、native_helpers、store_owner均为
`Selected model is at capacity. Please try a different model.`；本任务未切模型、未将该错误当作构建授权。
仍等待fresh空间与唯一Cargo槽，未执行新Rust/Cargo/大socket suite。已绿生产69e29231/packet914dbce7不变；
新增drip tests/fixture实际源码会与本卡一起保存到内盘inflight-source/Q037-ACP新独立快照，
恢复时先核源，不能按摘要覆盖原外盘dirty。新drip测试尚未编译或运行，不计业务红/绿。

### 恢复后drip真红与窄修/Close新红（当前未交付）

root恢复并明确授唯一Cargo槽后，fresh两盘12.37GB/7.78GB。
`Q037/acp-stage2-drip-red`真实编译31.81s成功，目标父/实际worker因“cancelled replay kept the session busy while its reader continued progressing”失败，
worker11.91s、source_stable=true。marker实际31,457,428B首帧，6.821567秒已持续读取5,242,880B后再Stop，
且1.5s后重复Stop；不是无读stall。fixture29842后ESRCH。

随后仅对已取消UI终止帧加入同一个绝对deadline：首次Stop主loop写入watch Option<Instant>，重复Stop不改值，所有recipient/两帧共用；
正常HistoryEnd正等容量时也监听取消，到点仅remove该client。正常大frame仍无总期限。

**`Q037/acp-stage2-drip-green`目录名称仅当时意图，实际exit101不是绿**：取消后2.014826s同PID下一Prompt EndTurn，
且原drip reader又进展约1.5MiB；但随后Close等Gone原5s Timeout，整场不能计绿。源码/原始marker/日志未改。
root要求保留5s及Gone立即ESRCH，未放宽。

临时cfg(test)打印里程碑轮`drip-close-trace`1/1过；随后无IO Atomic阶段位首轮及4重复均过，只说明诊断轮，
没有据此宣称根因修复。全部生产源码诊断已移除。无诊断固定源重跑`drip-final-repeat-1`又真实Close红；
外部guard在其自有worker45395实际采样（worker-45395.sample.txt）显示host主线程及全部ACP owner/IO线程均已不存在，
剩listener/GUI reader/writer，提示host已在Gone前返回Err，而非主loop仍忙。采样仅是只读诊断，不替代业务结论。

当前测试helper仅在原5s失败分支检查已finished host JoinHandle并保存实际返回Err；成功路径不加打印/睡眠/原子。
`drip-close-return`编译/首轮过，当前同binary最多5次窄重复遇失败停，等待捕获具体Err。生产无trace。
只读追到Building.run的owner错误可能来自共享Session wait/terminate；Unix leaf不在当前写入授权，未改。
本阶段尚未根因闭环、未提交，不能把暂停前10绿或诊断轮绿当本候选可交付。

### Close根因收敛：Darwin exit-in-progress（已授权Unix叶窄修）

`Q037/acp-stage2-drip-return-repeat-2`再次5s真红，失败分支join到实际host Err：
`ACP process cleanup is unconfirmed: Operation not permitted (os error 1)`（owner receipt同错、0stderr）。
所以没有Gone是按未确认receipt失败返回，非成功清理；之前sample线程消失不能解释为成功。

root明确授权额外仅`crates/node/src/khor_agent_process_unix.rs`与对应新独立测试，禁止改其他平台叶/Node public接口。
Unix改前SHA256 `874b04751d841cdb5066251eb4da56d6e52f41ac0ea30204e96202fe8ea5c758`。
当前7路径、原Unix叶、manifest/锁、diff和卡已有内盘
`snapshot-drip-unix-eperm-20261026T094736Z`实际快照。

实际自有C探针在`Q037/unix-zombie-probe`：

- `zombie.c`、`zombie-repeat.c`及`zombie-pgrp.c`：setsid/setpgid的稳定unreaped zombie均killpg EPERM，
  proc_listpgrppids返回1（PID数量，非bytes），严格仅leader；连续WNOWAIT均SIGCHLD20。没有据猜测放宽count0/4。
- `exit-window.c`：自有setpgid子进程写ready后立即_exit，1000组中961组killpg EPERM时立即WNOHANG|WNOWAIT仍si_signo=0，
  后续blocking WNOWAIT才确认SIGCHLD+同leader且列表严格单leader。所有1000均实际waitpid回收；
  完整原始`exit-window-result.jsonl`保留。Darwin先使组不可signal，再公开可wait退出的短暂窗口，原EPERM fallback过早拒绝。

当前仅在Unix叶末尾加macOS test入口，新`khor_agent_process_unix_tests.rs`2项实际owned child测试：
32个ready后立即os._exit的终止窗口，以及活组SIGKILL/lost ownership拒绝控制；原terminate算法尚未修。
预定窄修仅Darwin EPERM下最多约100ms短轮询退出可见，成功仍须WNOWAIT归属anchor+原严格单leader成员列表，
其他/截断/未确认退出继续保留EPERM，不广泛吞错误，不改其他平台或公共接口。

首轮`Q037/acp-stage2-unix-exit-red`未到业务：共享quality-target被别树Cargo留下旧ACP artifact，rustc看不到本树仍存在的
start_configured_on/fence_events；source稳定不能代表依赖同源。已按setup保留，未计真红。
root确认后仅touch授权ACP lib.rs，前后字节SHA仍d7d01170、不清任何archive/binary；
当前`acp-stage2-unix-exit-red-rebuild`正在强制本树依赖重编并跑两测试，runner85160/Cargo PGID85173、工具session51280。
后续以真实日志/源快照更新。root保留最终集成要强制第一方实际同源依赖的纪律。
