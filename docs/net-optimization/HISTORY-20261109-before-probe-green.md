# feat/net 当前状态与恢复入口

更新：2026-11-09 05:37 UTC；root串行，无子智能体。活动目标“完成计划”仍active。
完整范围：[PLAN](PLAN.md)；约束：[REQUIREMENTS](REQUIREMENTS.md)；运行记录：[RUNS](RUNS.md)；原始结果：[REPORT](REPORT.md)。
此前STATE原样归档[本轮历史](HISTORY-20261109-before-accepted-green.md)；历史在途/候选文字仅代表当时状态，恢复以本页和实际工作区为准。

## 授权与工作区

速度/体验优先，个人可信设备网；活动目标授权N0–N5实施验证。没有合main、发布、中继采购/部署授权。
本机source-only：不运行Cargo/Rust/Node/Xcode编译、测试、formatter或下载可执行物；Git/Python stdlib/脚本检查可用。重型沿macserver/GitHub，不碰现装App、生产store、个人会话或其他任务树。
图谱先用，覆盖不足才定向读；不新建CodeGraph索引。外盘布局/治理入口遵守AGENTS，本任务不接管代码治理。

| 用途 | 位置 / 当前基准 |
| --- | --- |
| 产品 | `/Volumes/disk0/Khor/worktrees/current/khor-net`，`feat/net`；最新已验证生产2fc650b30a3ab16792adb7babe566b8cec1bfeb5；随后只有证据/计划文档提交，恢复核HEAD/diff |
| 控制 | `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，`feat/net`，0cff9671fb2f80945423132e0df46995f6ff3198已push，现场clean |
| 基线 | `/Volumes/disk0/Khor/worktrees/current/khor-net-baseline`，detached6485cb83d3b1f0bb93d4633825dc956623d78af9；仅paths/hop/lab/nat.rs、dev.py/netlab.sh六fixture，生产原样 |
| 证据 | `/Volumes/disk0/Khor/evidence/net/`；source-refs.json现116条不可变refs，全部终态；本轮116个REF:source逐项复核一致，无活动run |

## 本轮交付：N2总预算与Accepted回答取消回归已绿

**42567946141 Mac net-node SUCCESS**：base2fc650b30a3ab16792adb7babe566b8cec1bfeb5 clean、tree314894b33ed72a5ff2aeed47504d6c9e7f575bd9、snapshot78c6fb8eabf86ff755800188d9f92e64f8d4e3cf、control0cff9671fb2f80945423132e0df46995f6ff3198。
mac-accepted-owner-candidate-42567946141保存source/environment/raw/classification；根格式、net66pass+1ignored、hop9/wires10/journal28/dial_budget7全部绿（预算子集1.67秒）。这是Mac便携子集，不是完整Quality或N2全包。

| 边界 | 真实红 | 已绿范围 |
| --- | --- | --- |
| normal拨号锁调用期限 | 42556768499 | 42557583517及最新；40ms到期不改共享失败/peer down/backoff；已过期空锁不拨号 |
| early拨号锁调用期限 | 42557961726 | 42558820303及最新；独立入口保持相同合同 |
| 操作scope到真实ask | 42562932053两红：忽略40ms、300s被固定28s截断 | 42566727617首次五预算绿，42567946141再次绿；绑定后暂停测试时钟，只证明这两真实ask边界 |
| Accepted回答所有权/期限 | 42567527752两红：到期继续等待、调用取消后回答未STOP | 42567946141两项全绿；同一共享wire独立Sessions成功、stable_id相同、严格3请求、零dial_failed |

生产：resident入口一次建立原Op档5/30/300/3600秒减.5秒的不可变Tokio截止点；任务局部只承载值，网络阶段显式取早。锁/hop/有界拨号及延迟候选、普通交换/一次读重试沿用by；CallerExpired与peer超时分开，不记失败/退避。
独立ask无Op时由Request工作预算加现有dial25s/hop3s/最大road120s合成外层上限，各阶段仍可先结束；撤回套IPC档位的提议。更新拨号保留原10秒自身上限，并另受操作by裁剪；该更新入口已编译，专门运行时上限覆盖仍缺。
Accepted返回Aborts所有者，adopt后以原by等待其中handle；取消/到期drop owner停止回答流，保留已adopt健康conn。Rejected、握手失败和未adopt新conn清理保留。读写/0RTT准入、frame、正常IPC半关闭未改。
RetireUnheard仍要求armed、heard_count不变、elapsed>=Patience.ack；调用短取消本身不退役，真正死wire的独立保护不能移除。

fixture先warm真实连接，再观察第二请求、Node adopt且ask持续pending20ms证明生产Accepted回答路径；Noq is_0rtt在应用accept时取握手状态，只作观察，不能宣称包级重放验证。42566727617该旧前置失败不算行为红；42567204795仅格式门，未测试100ms延迟提议；该延迟已撤回。依据及先于结果的笔记见accepted-answer-fixture-notes-20261109.md/ACCEPTANCE。
42564749932是Node类型编译失败但net66已运行；42565998212/42566472411及此前格式失败均单列，不能混算行为红。全部准确版本和历史红保留RUNS。

## N1仍有Linux红，不能用Mac单元收口

Iroh37/Noq2已由42555870179实际绿：base d2849d4732c7e9b6dd2e953616ce583f0e520391、tree4a9c4d89cc01e33a480375d50cc9f77b6d2d7a83、snapshotb6a4e9240786b1b178aee1e11284e6164aecc377、controldd57be6c57090a811726983e700d0b39a40fd1e8；根格式438Noq/36路径/9home。37当前tuple近期copy接收只作复测提示，无提示允许故障后备用ACK快速Go；有提示保留决定后验证，原250ms节奏/4s兜底/正常两样本保留，无应用重放。

以下原Linux同basea087 clean、treec807d328e78bdfda1dbebd7bdd4c14b0eaa8cd25、controlc68b644，普通日志关闭：

| Run | 结果 |
| --- | --- |
| 42542824071 stalls | SUCCESS2/2；600秒/20次300ms CPU，alpha/beta0/0移动、零退役/打洞，最慢865.4ms、3019包312293B；p5原150/300ms门绿，600ms仅观察 |
| 42542984111 dynamic | FAILURE25/27；p3三轮1.1/1.2秒/818.4ms，前两轮超原1秒门；p2及21相位绿，零错/退役 |
| 42543254686 hop | FAILURE8/9；f4s首轮0.7秒回carried、10.9秒后退Relay、28.8秒再回，最终LinkRelay；另两轮0.4/0.6秒无回退，f4/f4i绿 |

f4s的Relay2.5秒双向注入在断腿恢复1.5秒后清除，后续整段不能当5秒Relay RTT。该回退未独立定因，不宣称37已解决。
42542228149诊断2/2不是SLO；旧Watch较快但CPU3/1和hop1513ms红仍留，不能跨源/机器算改进。恢复Linux后原p3三轮<1秒、CPU0/0、hop无回退，再最终准确源完整矩阵，不放宽门。

## N3测量模型已校准，真实应用验收未完成

42559476637实际UDP道路校准红：旧单FIFO把反向1B挡在未来正向包后；此前42559155803只新fixture字节类型编译失败。
仅测试模型改每方向独立FIFO，保留lane/几何/seed，road_model=independent_fifo_v2、schema v2，产品CC/优先级未改。
**42560015454 SUCCESS**：base1478bf9712ad50c00b502c78613cd932f20586b3 clean、tree33d5ef6d47cde04620db1baa470bd7fefdf0c99d、snapshot9a6f9717470e0a741aedbdd9dd1ecf0a32d18fc8、control0cff9671fb2f80945423132e0df46995f6ff3198。
root fmt/编译/校准1pass、三轮各2pass（167.64/167.09/167.29秒）；704完整samples/51摘要/15bulk、零失败/超时，每轮235/234/235，计数/索引/分位数逐项重算一致，sample-audit.json存hash/方法。
固定300msRTT/每向2%/20Mbit/200ms队列/20s bulk/2固定seed；只是Noq模型，不是Node/SSH/hop/手机SLO。bulk-echo合并p95：Cubic普通890.3ms/优先389.9ms，BBR3普通449.6ms/优先612.4ms，效果不一致，不据此改产品CC；screen每CC仅9样本，不报p99。旧shared_fifo_v1不能和新模型差值算产品改进。

## 执行通道与Linux限制

托管Linux prepare因GitHub账单/额度拒绝（42545673450两attempt、42545712302，无步骤/产品测试/source artifact）；已异步询问额度或已有Linux通道，未有回复，不改账单/采购。直接SSH macserver原iroh-ssh路由banner超时，未改路由/信任。
Mac自托管worker已实际跑产品，现API Git bundle通道多次验证：私库无父source/bundle快照、64MiB上限；worker按精确GITHUB_SHA取可信bootstrap/driver，校blob/SHA256/snapshot/tree后仅本地Git clone。临时repo-read token不继承Git/prepare/构建。
沿development.yml、mac-mini-worker单槽，仅net-unit/net-node/net-vendor/net-congestion，Rust1.95/jobs2；原安装guard/内存/空间/缓存策略未改，无个人key/token/签名/模型凭据或生产store。标准executor旧checkout/default inputs保持兼容。
host实际VZ=1、16GiB/10CPU、约76GB空闲，只有brew，无Lima/QEMU/容器；VM仍是NEXT-DESIGN的候选（固定Lima2.3.0/Ubuntu24.04 checksum，2CPU/6GiB/30GiB、无home/key/agent挂载、前台hostagent归guard）。未安装或启动VM，不能说Linux恢复；需先解决镜像/工具依赖和生命周期，再能力探针，且新后端同条件重跑基线/候选。

## 完整未完成范围与下一步

N0部分、N1仍红、N2部分、N3部分、N4未完成、N5部分；完整PLAN范围保留，不以本批单元绿结束目标。

1. **下一批N2实际wire合同**：短期限的普通交换只停本流/其他请求继续；原F7真实死wire仍退役；一次读重试沿剩余by；写入失答不重放；长请求/更新10秒的运行覆盖。已有scope持锁/Accepted两例不能证明全部。
2. **N2资源/任务边界**：hop两middle合计3秒、有界dial输者及延迟候选不在过期后开新尝试；pair_with裸probe及doctor::any_relay_answers内层Vec<JoinHandle>的取消/赢家释放；前台spawn与独立后台多阶段整体预算。NEXT-DESIGN已记录初审，尚未修probe。代理listener/传输job/Visit/nudge有意独立，不能继承已结束前台by。
3. **IPC边界**：client Op外层与serve接收后deadline起点不同，尚未跨进程传递剩余预算或取消。正常exchange本来就shutdown发送半边，服务端EOF不是取消；不得为贯通而重放已发送写请求。具体原源码hash见deadline-continuation-20261109。
4. **N1**：恢复Linux，独立定位f4s回退、原门复验及准确源完整矩阵。Mac单元不能替代namespace/netem/nft实测。
5. **N3**：真实Node32席位/content独立32预算、Custom1024帧/256KiB外层队列、SSH/hop公平推进、共享vs分连接、1/3/5设备后台、真实手机能耗，均需独立负载数据；不增加总资源伪造调度胜出。
6. **N4**：默认北京HTTP/TLS同机、自家登记集中北京、香港默认无独立pkarr；复用已有多登记/选家/继任机制，尊重only_mine/n0配置。双端完整/不对称路径、冷暖发现、整机/登记/出口故障域及容量实测和拓扑方案未完成，不采购/部署。
7. **N5**：48初步补丁（Iroh37+Noq11）仍需覆盖收口；Noq6计时42543928044、Noq4备用ACK隔离42539256792、Noq2/37本轮前绿保留。Noq1/5专项、其余边界、固定1.4.0逐行为等价尚缺；UPSTREAM静态hash不是上游运行，不升级/删补丁。原生Mac/代理/多机/手机/能耗与最终可信Quality仍缺。

本轮用户询问进度/工期时，已给出当前N2这一批粗估1–2小时、全N0–N5粗估3–5开发日且以Linux与真机条件到位为前提；这是粗估，不是缩范围或固定完成承诺。当前这批七预算/取消测试现已实际绿，但后续范围仍如上。

## 证据与恢复规则

先核worktree/branch/HEAD/diff、当前run与准确回执；无活动run时继续上述下一批，不重发已终态旧run。116 refs已逐项核，source-refs.json全部pending=false。
`git rev-parse refs/khor/net-evidence/<run>:source`必须等于回执tree；只git show读容器，不checkout覆盖/push本地证据refs。失败明确分format/compile/fixture/行为/执行器；tests_executed只按实际子集，历史红不删除。
Mac source/bundle分支只在终态、证据留存且所有权/SHA匹配后清理；复用源的请求不拥有原源分支。原attempt不覆盖。
外盘缺失只读`git show feat/net:docs/net-optimization/STATE.md`与PLAN；原外盘未提交内容不能按摘要覆盖。其他治理沿中央STATE/RESUME，不混授权。

## 新接续：普通wire与重试覆盖待远端

上一目标轮为progress（总预算与Accepted双红→绿）。现新增`dial_budget::wire`普通到期并发保活、真实一次读重试剩余预算、写失答不重放/读失答最多一次四用例，生产不变；协议已在ACCEPTANCE冻结，预期补足现有实现绿覆盖，尚未编译/运行。F7真正长黑洞未覆盖，不删除原保护。下一准确源Mac net-node应选中旧7+新4共11预算/流用例。


**42569493787 Mac net-node普通wire/重试四项覆盖在途**：base2aace27d1c7bf91aafd61600954bd8a3bd3814af clean、tree089f73291f6b809d1e2d4251e2e1ccfa36a5fd3c、snapshot35c95dc5d489b46db7382cc8a2426a8719d29875、control0cff9671fb2f80945423132e0df46995f6ff3198；117th ref/mac-ordinary-wire-coverage-42569493787已锚。生产不变，旧7+新4共11预算用例待格式/编译/运行；没有预记新用例绿或旧源红。


42569493787终态仅**格式门失败**：新增wire fixture的adopt链与一处matches缩进，远端patch原样应用；准确source/control/ref已核，117 refs终态，未编译或执行新4/旧7测试。生产不变，继续新源验证覆盖。


**42569954515 Mac net-node普通wire格式修正源在途**：basef42121b51e5ceb5252dd8594a8f6523553c33450 clean、treef198bbd5b488cf3f4e59f15b2ac1c77f6b11a1cd、snapshot40418eb1f61e2a436f583e846fc766f89e4bdfec、control0cff9671fb2f80945423132e0df46995f6ff3198；118th ref/mac-ordinary-wire-formatted-42569954515已锚。生产仍与2fc650b3相同，11预算测试待实际结果。


42569954515终态 **SUCCESS**：根格式/编译/net66+1ignored/hop9/wires10/journal28/dial_budget11全绿（预算6.73秒），新增四普通wire覆盖均实际执行：scope到期STOP本流且并发控制成功/同stable_id、真实读重试沿原by、失答读最多一次、Mkdir失答不重放。准确basef42121b5/treef198bbd5/snapshot40418eb1/control0cff967已核并下载，118 refs终态。生产与2fc650b3相同；新增绿覆盖不宣称这些用例在旧源取红，F7真正长黑洞仍缺。下一探针取消/首成功回归生产保持原样待实际红。

已新增probe三回归（直接取消、配对外层取消、首成功不等慢首URL并取消输者）。仅测试/fixture新增，pair_with与doctor生产未修，协议已冻结ACCEPTANCE；下一准确源net-node旧11应绿、新3预期红需先区分fixture/格式/行为，不能预记。


**42570381896 Mac net-node探针所有权/首成功三回归在途**：basea2cf865cf1c0f3eb5073ffac5612a8ab8b7a6ae1 clean、treef7d1528532848f0fccbe4b8eb674cc66183982e0、snapshot70ec29da02a513fed7c04e6a8b8810756783bbf6、control0cff9671fb2f80945423132e0df46995f6ff3198；119th ref/mac-probe-owner-red-42570381896已锚。生产pair/doctor原样，旧11+新3待格式/编译/实际前置与红，不能预记行为红。


42570381896终态仅**格式门失败**：probe fixture const/timeout布局与assert，远端patch原样应用；source/control/ref已核，119 refs终态，未编译/运行。probe生产仍原样，三行为红尚未取得；下一源继续。


**42570772550 Mac net-node探针回归格式修正源在途**：baseffbbcaac13a6d9f54591afdac1bebeeb5dbe415e clean、tree0a26b34f68b867d72eb047f9c6ec8234b68e5d59、snapshotcc5b77828bb5a5addb0db6ed8739824700c9697e、control0cff9671fb2f80945423132e0df46995f6ff3198；120th ref/mac-probe-owner-formatted-42570772550已锚，probe生产未改，旧11控制与新3红待实际结果。


42570772550终态**探针三项实际行为红**：fmt/编译/net66+1ignored/hop9/wires10/journal28绿，dial_budget11pass3fail（12.06秒）；只直接取消HTTP未停止、配对取消仍残留probe、成功second URL被slow first挡住。所有HTTP/配对pending/fast已响应且早于2.6s前置通过，无env proxy；不是fixture红。准确tree0a26b34f/snapshotcc5b7782/control0cff967已核，120 refs终态；下一修复用JoinSet收首成功并drop输者，pair_with用Aborts持有外层probe，保持原HTTP预算/proxy/诊断语义。

当前probe候选：doctor改JoinSet按完成顺序取首个true，成功/取消均drop并abort余下HTTP任务；pair_with用Aborts持有该外层任务，所有早返自动取消，dial成功后先drop probe再pair_over。原URL并发量/单HTTP2.6秒/proxy、空/全部失败false、超时诊断fallback不变。候选待旧11+新3实际绿；还不能收N2全包。


**42571393016 Mac net-node配对probe双层所有权修复候选在途**：based4a2e115b898fc337f5091b51fa71e45740c667f clean、treef278b9330f51268182b142729db2c86687f4af4b、snapshot3459894c81cb994310a97bfcba4ca39ae97e569b、control0cff9671fb2f80945423132e0df46995f6ff3198；121st ref/mac-probe-owner-candidate-42571393016已锚。doctor JoinSet首成功+取消输者，pair Aborts闭合外层取消；待十四专项/既有子集，不预记绿。
