# Q039 当前 inventory 非作者源码审查（首轮）

状态：首轮已完成交回；候选仍有待修边界，尚未Cargo/真实回归，不称本批所有selected doc已可见并继续。root已接受收窄auth分型与inventory局部失败治理，后续新候选需独立复核。

## 固定来源与方法

集成wave1 HEAD `76db834bb75aaeb727df571a9dfc580ab65ee31a`，root冻结4源（仅read），审前后SHA相同：

| 文件 | SHA256 |
| --- | --- |
| crates/node/src/link.rs | 7daf0e4e616033fd0c3ffbb6947a733688f0fc7020592a42d62f42fca955571f |
| crates/node/src/proto.rs | 77ddd5997f53f03a284557ec4596182d79ab49b06b346ce15eb189476edf0581 |
| crates/catalog/en.toml | 98a9af17113f2444dcefce75e67cebeafa251e18f2716f41f1bef9979f860fd9 |
| crates/catalog/zh.toml | fde6aaf894139bfee096611327d9fc402585d2bb5a99f06e3017c74b8ddfc549 |

已读作者Q039-current-inventory目标与中央用户纯当前开发协议决定，保留鉴权/撤销/数据epoch隔离；无main合并/ship。按review-changes图优先，minimal/detect实际索引仍main8704，未覆盖4源，回退actual diff/宏展开/调用/测试。没有改root源/Cargo/gen，不用rustfmt解析当类型绿。

## 待修 B1：全局auth响应扩大了当前调用面

初候选dispatch_inner统一admit失败由原Refused改为NotAdmitted。同步inventory消费者有意处理它，但普通ask消费者大量只针对Refused保留why，其他分支成peer_non_answer(Debug)；部分原Refused分支还返回空默认，属于更广泛auth错误语义治理而非一处标签替换。Watch另有流式admit路径仍Refused，并未形成全auth统一契约。

精确当前raw auth测试仍只匹配Refused，非旧端fixture：pane_history_wire.rs:166、history_wire.rs:135、files_wire.rs:280/299/318、act.rs:265、recognition_wire.rs:127、khor_agent_wire.rs:214、delegation_wire.rs:458；已逐条核陌生iroh身份实际请求。普通业务Refused（如act unknown_action）应保持可区分。尚未运行这些测试，不伪称已取得运行红。

独立选型：本批建议B，仅Versions admit失败发typed InventoryNotAdmitted（或NotAdmitted但注释严格限定inventory）；普通请求仍其当前拒绝契约。并非旧Khor兼容，而是避免未审查的跨API语义变化。A全auth统一有价值，但需统一ask_on/early/direct exchange/Watch的分型及why输出，不能只在几十个match机械加pattern；应独立批次有完整auth/普通Refused矩阵。root已接收B，4源冻结仍是A候选，等待新source。

## 待修 B2：inventory准备阶段仍按整轮早退，不能保证per-doc继续

这是明确源码控制流风险/缺口，**尚无本轮实际执行证据**，root已授另一agent取真实local/remote bad-doc红。需分别观察后续Seen/Pins/独立doc是否推进，不以helper单元镜像断言代替真实Node入口。

- opening约10467：每个selected doc计算digest，with_doc!里面已知arm的*_loaded()?向外opening传播；调用sync_device的self.opening(d)?在dial前终止。坏accounts/titles等会挡后续selected doc，尽管它们和Seen/Pins独立。
- answer_versions约10692：相同with_doc!的已知arm *_loaded()? **同样向外answer_versions传播**，导致整份inventory返回Refused/后续doc不继续；外层let Ok(...) else continue不能捕获宏内部?。本审查最初误将其理解为所有load Err被continue，已在发现后立即纠正并报root，未取到真实“坏known doc仍success”的红，不将该初判作为事实。
- 宏最终返回Err的unknown doc/invalid wide则由let Ok else continue静默跳过；当前合法ROUND_DOCS名称双方都支持，不可把unknown条件套在已知accounts加载失败上。可用单独unknown inventory请求验证严格错误语义；不能冒称它已复现。

建议采用独立Result闭包/真正helper捕获宏内所有?，让local inventory准备与remote inventory响应都携带明确per-doc失败，再继续其他doc并将失败与SyncProgress聚合；当前版本可一起修改schema，无须legacy回退。root已决定本批修并先取真实2红。未由本审查写任何新协议。

## 已看方向合理的逻辑与仍缺运行证据

- sync_device只在typed admission响应触发一次vouch_to，成功后重发同一个opening，普通Refused/未知响应不回sync_everything。vouch原有按peer时间限额、签名offer保持；take_vouch核anchor签名、fresh roster、双方admission、newest seq与gone_ids，未见绕过撤销的新路径。table_lost.rs真实回归应验证新分型自愈；该旧测试源码会检查round Ok及双方Trusted，不能以5个新pure helper测试替代。
- sync_documents在进入后遍历全部differ，无mandatory/optional吞错；每doc journal keyed错误，保first_failure，Apart累积并成功描述为partial，不再nothing_to_move。普通失败仍返回Err；有移动仍刷readings，即使有部分失败。设备成功且bytes>0触发follow_after_sync，失败不触发；broadcast成功有bytes才take_arrived。sync_doc_isolation原真实“设备失败不follow且Seen/Pins继续”仍应跑，另应有optional失败后后续doc继续的实际正反例。
- rounds两轮bootstrap只余pair_with的devices使用（约9687），当前inventory走rounds_from known version一次。Pair仍是现行初始bootstrap，不误称旧端fallback已完全从仓库清除。
- 新5 tests能验证拒绝解析/当前roundtrip/typed label/partial描述/first failure聚合，但不能证明真实auth-gate选择性、Vouch限频/成功重试、per-doc循环或follow触发。这些目前都是计划或既有待跑，不宣称通过。
- 本轮Python tomllib只读解析en/zh成功，sync-partly-apart双语placeholder集合均{0,1}，无重复TOML键解析错误。Rust catalog msg函数与GUI生成物实际生成/编译仍未运行，需后续Cargo/gen槽；不要拿旧ignored gen证明新词条存在。

## proto文档收口：当前约束与旧政策引用

header可保留的现行约束：Khor节点使用共同当前schema；msgpack struct按位置编码、字段顺序有意义，optional即使None也保留wire位置，不在wire struct用skip_serializing_if改变数组；当前变更同步更新producer/consumer/roundtrip；不从一般Refused猜协议能力/触发数据回退。鉴权门、签名/撤销、0RTT reading与写请求等待handshake、MAX_FRAME/资源上限仍是当前安全约束，非旧版本兼容。

精确已悬空引用（初候选行号）：Pair约29–35（“head规则/旧issuer”）、SatLs约144–149（“module head's first question”）、Watched约153–157（“head measures tail append”）、Versions约524–528（明确仍声称拒绝后two-exchange全量fallback，已与本代码矛盾）、OpenOnce约575–580（“module head/旧端退Open”）。SatVitals约135–141、SatTouched约210–216、DropFileInto约652–657也仍以旧端拒绝作政策理由，应换成实际当前语义，不能只换header后留错误因果。

proto还保留旧形状专项测试/注释（如约1412 unknown-op old enum、1843 old-vitals、2381 old-Usage）；清理时区分当前未知variant不能被误读的codec安全断言与专测旧Khor端的假设。用户数据默认字段/落盘格式是否仍需保护须按实际存储消费者判断，不因协议旧端测试删除就擅自破坏用户数据。无需在本批新增旧端fixture/矩阵。

## 交回与建议

root接收B1选型B与B2后续修复，生产4源写权交回；本审查到此冻结不再读正在修改中的混合源，不继续Cargo或生成。当前建议“需修后复审”，不把首轮静态审查写成通过。新候选应附：真实local/remote坏selected doc原红/绿、table_lost/current isolation、当前codec与正常lib类型编译、catalog生成结果及固定源SHA。native_helpers持唯一Cargo槽，独立审查不占它。
