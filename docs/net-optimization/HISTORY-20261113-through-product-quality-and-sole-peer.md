# feat/net 当前状态与接续入口

更新于 2026-11-13 02:42 UTC（下列实际收据时间优先）。用户“定好目标，开始”已恢复执行；完整目标 [GOAL](GOAL.md) 与 [PLAN](PLAN.md) N0–N5 **未完成**。
用户ToDesk级体验、iroh/Rust极致优化及原约束均已记入GOAL/PLAN v3。root串行，不派发子智能体。目标服务旧paused状态不覆盖本次明确恢复授权。

此前STATE逐字保存在 [上轮完整状态](HISTORY-20261113-iroh-recovery-and-stability-preparation.md)，其中继续链接所有更早历史、原失败、收据及冻结输入。
历史SHA256 `3e1fe2e38c7f7d90fa34166977390a8e2918564a7ab2370d01cf5c906f970753`。本页只描述当前事实和下一步；历史中“当前在途”按其时点理解，不能再次启动已关闭批次。

## 当前执行：产品集成与完整验证

旧新真实混版观察协议v2已完成并闭批：`turing-20261113T011900Z-19b49119`，control **de74a134bb5a1d2416fbf1736bd05c50ab696731**，source仍准确3b734073。**0/160.457763s、原Rust1过/0失败/0忽略**；原双向row/pin、old/new两常驻host及其sleep子进程均实见，原身份/hash/group空及23原reportfiles核验。controller仅用私有subreaper补齐快速分离子进程见证，并按inode缓存ELF hash；独立零产品运行校准通过。原产品fixture与最终行为断言未改、编译执行零、实际混版runtime一次、替代零。proof E/该run/mixed-observer-v2-verification.json；ledger pending=false/behavior_verified=true。只接受此窄互通合同，不是完整N5、旧pre-Versions回退、手机、WAN或能耗接受。旧两次失败原样保留，见[IROH-MIXED](IROH-MIXED.md)。该混版批次已无Turing在途；后续在途见下方独立compile收据。

已建立独立集成树 **khor-net-product-integration-v1**、分支 **integrate/net-product-v1**。最初基于本地main57a69；随后核实其是公开main **c3b098bdbf52a9b3b38e887ae74b90155292b544** 的祖先，已将实际公开main合入本任务集成树，绝不将旧GUI候选当最终产品。集成merge **a3e07d34133b3a1b4d4ee66d82fd8d0cab37436e**、tree **da403915d443fe2e52f5f0a2a4d32030dd12244b**，自动合并无冲突，apps/逐字保留公开main；三份锁的iroh1.4.1/Noq1.4.0一致。原feat/net运行时不变，用户main、已安装App、其他任务Lab23961均未动。集成源码尚未经过完整Quality，不晋升或发布。

已同步本轮文档并冻结单父验证源 **117c39e5432741198c924177fae1775ff292eada**，tree **ace4fd8223ed7fcd8d2b29477d9e651a2df53ff2**，唯一parent为公开main c3b098。完整本地merge链保留于 `refs/khor/net-integrations/product-v1-merge` →1cd5092f；集成树HEAD现在117c39且clean，apps/及.github/逐字等于公开main。只推独立 `colehank/khor:verify/net-product-v1`，原生完整Quality **43348869616** 已于02:35:11 UTC **FIRSTATTEMPT 13/13全部SUCCESS**，suite=all/interaction_scope=all，API核准确head/event/attempt；包含三OS runtime、Agent、frontend、Lab、GUI、Node原生及真实SSH安装认证、Mac更新/WebKit、iOS原生UI等全部原门。E/net-product-quality-v1.json为完整run/jobs证据，E/net-product-integration-v1.json已记quality_succeeded。8份原artifact均已下载至E/public-quality-net-product-v1，逐份GitHub原SHA/字节/ZIP CRC核，archive-retention.json保存；未提取或执行其中native产物。此结果不是候选手机、WAN、能耗或全部N0–N5接受。没有main/tag/release/已安装App操作。

同时补齐实际唯一middle首个业务场景。独立树 **khor-net-product-sole-peer-v1**、branch **experiment/net-product-sole-peer-v1**，base117c39，仅新增fixture模块与mod声明，产品源码不改。X/M同LAN Direct；X在resident启动前双向阻断其仍配置的Relay，Y双向阻断全部UDP，M→Y只剩真实TCP Relay。通过M配对roster，无X→Y业务预热、不等hop或no-relay长期宣告；后台若已准备好必须如实记录，不称冷前台握手。首个目录请求要求5秒内返回目标独有marker、X→Y确经M、两腿实报Direct/Relay且当时no_relay=false；原3秒hop预算未改。协议E/net-product-sole-peer-plan-v1.json，先compile-only，再预声明三独立runtime/替代零；缺hint/旧Direct/负向策略/能耗及手机仍另验。

首次准备 `turing-20261113T020722Z-4abf5dc7`，source59e2fe9c/control e532：format前置红，**零编译/零产品runtime**，10原files/hash/identity/group空已收且ledger闭；只应用其准确formatter补丁。修正source tree **a448adc27f7502deb7293bbfa376d96de539df3b**、snapshot **08d0afa45e3aaf15ba73b8fd6928c30737f6de37**，描述符E/net-product-sole-peer-source-v2.json。增加原native_provenance实际test producer与同producer CLI准入，未改verifier、业务门或生产源；E/net-product-sole-peer-preexecution-amendment-v2.json保存执行前修订。

compile-only `turing-20261113T021226Z-a2207feb`已 **0/129.161091s**，18原reportfiles/hash/identity/group空、原native verifier test+CLI同producer及三锁版本核验，proof compile-provenance-verification-v2.json与ledger闭。实际test SHA0ce7704b、CLI f129a57c，编译通过不算业务绿。

唯一middle实际固定三次 **已完成并闭批**：[SOLE-PEER-PHYSICAL](SOLE-PEER-PHYSICAL.md)。run **turing-20261113T022908Z-54fab5de**、control **9f6d4d093dee569c27b4443e59b75b5fd4074879**，0/78.088398秒，准确source a448adc2/snapshot08d0afa4。三次首目录 **531136/524952/516796μs**，原Rust各1过/0失败/0忽略，前态均Idle、no_relay=false，目标独有marker、实选经M、Direct/Relay两腿与XRelay/YUDP实际drop counter均过，全部owner/group清理、18原files/hash/identity/专属proof核，ledger闭。编译零、固定3/替代0；不再追加相同候选试验。夹具已提交 **ade7decb4aa2d8616fb028b0b0221873d9d6f494**、tree仍a448adc2，S树clean。只补该受控首业务病例，不是全部NET-16或N0–N5接受。

原runtime准备022103Z-df6a99d7（control175fdc）错误把环境SHA作值，找不到git；022453Z-ee449360（controlfc3e）原初始来源准入过后，controller新增KHOR_TEST_KEEP/NO_MDNS改变完整环境指纹，被原门拒绝。两者均**零编译/零产品runtime/零业务人口**、原收件/identity/group空与专属proof/ledger闭。v3在原producer环境下运行，只设private TMPDIR；原Scratch清理、子命令mDNS隔离及全部原行为门保持，完整日志/owner事实留证，scratch homes不保留。原native verifier没有放宽。此处失败属于控制前置，不隐去，也不计业务失败或重试成功。

当前本任务无Turing在途、无pending ledger。原资源/源码/可信执行约束保持；Quality43348869616也已终态全绿，不能重复dispatch。root继续串行，旧协调器61277不恢复。

下一步：按需核对已保留artifact的具体payload用于新集成Lab；冻结同一实际业务夹具的公开main/候选对照（先验证共享helper兼容，不能拿不同observer作速度因果），继续Release基线、缺失/过时第二腿hint/负向策略、地址变化、手机/原生Lab和custom退出核查。完整N0–N5未完成；最终性能协议在执行前绑定最终arm与原门，不挪旧候选测试作新集成验证。

## 已保留的混版v1失败

首个真实runtime `turing-20261112T192613Z-c2c8ebe2`，control **ce4998f58fe1cc70d8b84e1854582d8828d27223**、source3b，controller1/187.838121s；原Rust1过、138.35s，四项row/pin均同步。原observer见65个owner干净退出，但只见old host，198行原后置断言失败，accepted=false。21原files/hash/identity/group空，proof mixed-runtime-screen-v1.json及ledger保留。原祖先采样可能漏掉快速分离进程，重复哈希约264MB ELF也会延长采样；这是框架可见缺口，不独立证明旧失败唯一根因。v2独立声明修正观察协议后才执行，未放宽原host/child门。

混版第一次准备 `turing-20261112T190345Z-5b6f85b7`，control054/source3b，1/459.251317s，26files/hash/identity/group空已核；**没有mixed-runtime.log或contract，零互通runtime**。错误是控制脚本139行假定cargo test不会重建CLI，普通build ELF dcd289与实际test producer的418df3不同；原old/new只读copy均保持原hash。失败原样保存，E/该run/mixed-preflight-failure-v1.json，ledger闭controller_preflight_failure，不是产品互通红。E/iroh14-mixed-resume-plan-v1.json SHA7d388c89dba9d0067283e70def0a2188fae81eb166d577a248989b871a9d3601声明只复用已准入old只读ELF、从原verified test producer选择实际CLI，用原require_provenance再次核文件/源码/context；不改verifier，不再编译，原runtime/host/child/断言body逐字相同（SHA11f48a3e67608f0f543bd07d707a2a91db4e1b1912a91bdb9e279b61abc6603a），允许实际runtime一次/替代零。ctrl/proof在E/iroh14-mixed-resume-control-v1.json。

## 已收齐的长稳对照

暂停现场已核：coordinator61277不存在，原candidate用户取消已完成收件/identity/group空，ledger无pending，Turing原独占锁可取。保留baseline完整5/0、28journal、22CPU与32目录请求原证据；按E/iroh14-policy-stability-resume-plan-v1.json（SHA a67d29b161db39ea368f55b916d2ca2c0f1251ef55c5aa917de9810705bb1a9e）只补一轮完整candidate。源码/控制/五原case/600秒/原门/资源均不改，原用户取消部分保留；不重跑baseline，不追加失败替代。随后执行mixed，再建立Release性能基线。准确补齐run **turing-20261112T180542Z-c09f6a3c**，PID2328430，18:06:03 UTC启动，revision iroh14_policy_stability_candidate_resume_v1，control f862/source adf39a4c。启动exec已终态，root恢复为唯一串行ledger writer，旧coordinator不再启动；计时期间不插入自己的其他Turing重型任务。

该candidate现已 **0/3138.550709s、5过/0失败/0忽略**，11原reportfiles收件/hash/identity/group空、原resume专属审计及ledger闭；28journal覆盖与CPU64上22记录过。两arm原p1/p1h/p1j/p1s零move/retired/network changes，p5每档四次150/300/600ms零Relay/零punch。原取消candidate已52.58秒重编，恢复直接复用同树adf已有构建，build0.54s；不是跳过不同源码的编译验证。

E/iroh14-policy-stability-resume-results-v1.json为完整描述性对照：两边各64原始attempt记录/32逻辑目录请求全ok；p1s最大4690364→868648us，p5最大828673→920018us；p1h alpha包4242→5683（+33.970%）、字节745939→1028707（+37.908%）。这些含本地IPC/目录处理、单端计数与中断后时序差异；不能宣称全局延迟改善、成本最优、p99、能耗或单补丁因果。批次完整关闭，accepted/promotion均false，不追加同源挑绿。

## Actions已通过公开khor-build恢复

用户问能否通过公开仓库恢复Actions。只读查明私库colehank/khor-build的43205919189被付款/额度拦截、prepare零step；macserver最新SSH仍banner timeout255。colehank/khor已经public，私库失败后其公开release43227326496仍成功。其现有Developer Lab（410114960，workflow blob404aed5fddbe4c8ecd9b8569ec9d4afcb4d25d9f）含ubuntu前端/检查与macos-26标准托管原生构建，可作为待确认的替代路线；不触发release。

用户“开放吧”并澄清“为什么让khor-build开放不行？”，已按原意将 **colehank/khor-build改为public**，GitHub API复核public/admin；全部既有分支和历史保留。原复用公开产品库khor作为builder的提议已被此决定取代。此前未向khor推送快照或dispatch；本轮另以独立verify/net-product-v1分支运行其原生完整Quality，具体版本和边界见本页当前执行。E/khor-build-visibility-transition-v1.json绑定授权与结果。 控制库README已同步公开约定，docs-only commit9b671817e34846df9e14c213a562ab88309cb45a；未改任何workflow或产品main，原Lab run仍绑定6ed3ce控制版本。此前付款失败和SSH超时保留，不把建议私有误作永久技术限制。

公开前核466个远端branch、8819个可达对象、4606个普通blob及212个bundle头，后者均指向已可达源码图，无额外prerequisite；唯一密钥格式命中是与公开main逐字相同的vendor/tiny_http示例TLS key。无tag，历史工作流无外部PR/评论等代码触发器，默认token只读；自托管runner存在，现有任务均手动dispatch。E/khor-build-publication-source-audit-v1.json及actions/branches-before保留；这只是已知格式源码检查，不宣称全日志/历史artifact凭据排除。

已在该公开构建库上传准确3b候选snapshot4a688a33，request **e6c996efe74043858e4b0d2c000c2c7b**，完整Lab **43282299770**，control main **6ed3ce0429f767525319b6ace6c374919a48ce16** / workflow blob **9c9a39eaa9c01f00b0e0f00c9b9cd4fda2651555**；port23962。**FIRSTATTEMPT四job全部成功**（frontend/native/checks/retire），计费拦截已解除。E/public-native-lab-3b-v1/verification.json核准确源码/native_contract、GH原archive hash及内层native-host.zip hash；native manifest Darwin arm64/独立Lab identifier/managed_frontend+real_backend_verified，GH原生probe source/tree一致。前端原validator通过，浏览器results48及hot通过；完整Quality/手机/本地原生接受仍未完成，**本机原生执行零；尚需配置managed Lab供用户验收**。三原ZIP/原manifest/probe均留E/public-native-lab-3b-v1，已有其他任务Lab23961保持。E/khor-build-public-lab-request-v1.json为准，零替代重复提交。

提交工具已在 **45044a84f1dfac9339bfc934a8c4c3341b1864b3** 修复：CLI和Lab共享require_source_builder，仅显式授权的colehank/khor-build同时匹配请求名与GitHub实际full_name时允许public；其他公开库仍在snapshot/upload前拒绝，原私库流程保留。真实CLI已成功提交工具验证run **43283055508**；control验证分支commit **adc701423672e2ea2136e781fbdac6a3e6461897**，未改control main。GH托管Linux三suite **3+14+15=32通过**，artifact13972678652 SHA **7cce5b4a6198ae81994c614e5789763f42551e2d62243b68713c9ac4cdc03282**、source45044/tree624dce/snapshot8410dbe逐项核验，proof E/khor-build-public-tooling-v1/verification.json。本机只做AST/diff结构检查；此结果不是完整Quality或3b原生Lab接受。

该3b候选不是最新GUI发行源；只能作为首个网络候选Lab。最初本地main57a69预览已被本页当前产品集成进展取代；公开main c3b098才是现产品基线。最终同源Quality/真机验收仍待，不把3b原Lab测试挪作新集成源码验证。现有其他任务Lab23961保持。

## 历史暂停（2026-11-13 00:57，本地）

以下描述历史暂停；本次18:05 UTC明确恢复已覆盖停止授权。coordinator61277已按准确命令身份停止；candidate长稳 turing-20261112T164807Z-a0e7a1d2 由其owned runner SIGTERM正常清理，原返回1/KeyboardInterrupt，原files收件、identity及group空已核验，ledger终态user_cancelled_partial_run，**不是产品回归或长稳接受**。baseline完整5/0与28journal/22CPU/32目录请求原证据保留；candidate部分观察不构成完整配对结果，未启动混版。当前没有本任务自动推进；恢复必须先核该取消收据并显式修订中断后的协议，不能盲目重启已消耗的fixed arm。

## 暂停前执行时点：两组长稳对照

- run **turing-20261112T155449Z-56a8ac18**，PID **2128254**，15:55:10 UTC启动。
- task `netlab-stability`，revision `iroh14_policy_stability_baseline_v1`。
- control **f862ab2defb48d21236c2bdcca736c9a3d108773**，baseline source **515e99f0d886de8035801801a10e7587167cf78b**。
- receipt及E/source-refs.json已存，启动exec已终态。最近只读仍running，p1已原600秒绿（两端0move/0retired/0punch、四journal完整），p1h/p1j/p1s也已各原600秒绿，p5在途（4/5原结果，未最终收件）；p1s含20次300ms spike，模拟交互最长4.7s。原门只验切换/退役，不能据green接受交互尾部，该4.7s须与候选同场景对照并继续定位；不要重复提交。
- 先baseline完整终态、收件、identity和行为核验，再candidate **一次**。两组各约50–60分钟，原90分钟守护不变；失败全留、零替代重试。
- 计时期间不插入自己的其他Turing编译、产品测试或共享source-cache写入。本地源码/文档/证据审计可继续。

**串行自动衔接已运行**：本机coordinator PID **61277**，exec session **48333**，脚本E/ops/run-policy-stability-pair-v1.py。**接续先读 E/iroh14-policy-stability-progress-v1.json，实际phase/run优先于本页静态时点**；完整结果固定写E/iroh14-policy-stability-results-v1.json。local flock防第二个coordinator。该进程活着时，它是root唯一ledger writer，模型只读，不能另行stage/launch/collect/specialized verify或启动其他Turing重型任务。
它等待现有baseline，顺序收件/identity/原stability审计、在准确原receipt/group空后只stage/launch一次candidate，再顺序收件/核验/输出描述性成本对照。任一控制/元数据错误停止并保存原receipt，不重复提交；原产品红可按冻结协议保留后测另一arm。十次只读SSH连续失败或超过100分钟未观察终态会停协调而不替代原run，远端90分钟守护不变。协调器只执行本批，不晋升、不改源码、不自动派发混版。源码/语法/合成比较准备记录E/iroh14-policy-stability-coordinator-preparation-v1.json；两份原proof始终为行为依据，不凭自动化宣称接受。

执行前冻结输入：E/iroh14-policy-stability-execution-v1.json绑定双方源码、control、readiness与四份协议/审计/物理结果SHA。
协议v1 SHA **e89aaa0c657e0bb26e7dfd3f01312a417c9de0f87bd1bff41cc61407b0aa86c6**；v2 SHA **08c27588260363d228c415769d4e96fa01ec71217379e7292eeeae51ccc63b0c**。
两arm都用iroh1.4.1/Noq1.4.0。baseline为028原25%收益门与旧恢复完成顺序；candidate为3b绝对5/10ms收益门和ACK顺序修复的整体策略对照，不作单一因果归因。

| arm | overlay source / tree / snapshot | compile-only准入 |
| --- | --- | --- |
| baseline | 515e99f0d886de8035801801a10e7587167cf78b / f0005b7544d4e5d93eed5e3dfe19ac021960980b / 10bb1bd227abb145b3591d94dbb413f8b7d66f49 | 154359Z-45ffa1ec，0/77.939305s |
| candidate | adf39a4c1adbae1a7b35bf98e1a6e9815f7db1b7 / 716caddfe8c009c8055d5d3b839b654474130f5b / ac1b99c3fe292f00b449e233c372aa21c2ac7644 | 154632Z-eb0a7d58，0/77.974962s |

描述符E/iroh14-stability-{baseline,candidate}-source-v1.json；工作树khor-net-iroh-stability-{baseline,candidate}-v1，分支experiment/net-iroh-stability-{baseline,candidate}-v1。
两个compile-only均15原files、control e532f19a4bb8939a14a921eaf3d954ccdd77cf5e，format/locked netlab artifact/三锁1.4组/source diff空/hash/identity/group空，readiness proof与ledger已闭；runtime0。

两边相同fixture只新增p1h高RTT抖动wrapper和测量结束后peer/here原journal导出；去掉25插入行可逐字还原两父源。两树仅差三app策略文件与iroh恢复完成顺序文件，锁/scripts/其他源码相同；source audit SHA **8e8311b241ccc790c6cbd3f9a6dd20cf8b5600f2a79d3fae29db95a08e17d3c6**。
原p1/p1j/p1s/p5及新增p1h保持600秒窗口、p5每档四次150/300/600ms stall、CPU contention验证、单线程/单repeat/diagnostics0。p1h调用one_speed(65,5)，约130ms RTT。
原gate：两端各最多一次选择、零retired；p5短于500ms档零Relay切换/零punch。不缩观察窗口或改生产预算。
原journal KEEP300，约600行轮转；要求每份非空bootstrap journal最早保留事件不晚于窗口开始。28份raw导出核保留范围，不宣称上游事件从未丢失。
E/ops/verify-policy-stability-v1.py已备，准备记录E/iroh14-policy-stability-auditor-preparation-v1.json：旧真实四case/22 CPU记录及六种错误副本检查，零新增产品执行。核验器会保留原红，不用数据完整性检查替代Rust结果。
p1s/p5原helper忽略echo错误；单端接口包/字节不是全网成本或能耗；该批不接受普通延迟/p99/完整N1或手机。

## 最近修复：首个恢复ACK不再提前撤回有界质量确认

[IROH-RECOVERY-ORDER](IROH-RECOVERY-ORDER.md)为完整设计、红绿及物理协议。冻结生产候选：
source **3b734073b410ec4130b4b57def3bf096b10c3e7a**，tree **473ee06d1a836d56d9e39bd10a7a4016fdacf530**，snapshot **4a688a33bd3da7f095cd8880ed1fd9519fb5ce09**；树khor-net-iroh-recovery-ack-order-v1。
只把complete_read内三行改为先recovery_probes、再mark_silent。反转即恢复红源，测试段及其他文件逐字相同，source audit SHA **a814baae82dab7b97cc92c24b620c8ee08647104f5d6a155d7c20c75bc2d2f5d**。
没有新状态、timer、quota或新鲜性豁免，原三次/250ms/2秒质量确认与四秒物理门保持。实际被补上的episode可能增加探测，不能由预算不变宣称成本相同。
旧read_paths已经传latest_rtt；真实问题是首个post-judgment ACK可让mark_silent在RecoveryCheck建立前撤掉Silence。不是median参数错误，也不是全部首连/NAT/手机失败唯一根因。

| 准入/固定批次 | 原结果 |
| --- | --- |
| 红150712Z-549ba3dd，source70a6f7d9 | 101/200.479139s；Noq467过，iroh原197过/原ignored1；只有新增first recovery ACK测试红，实际131ms/应85ms；node test compile绿 |
| 绿151200Z-d504e81e | 0/180.387027s；Noq467、iroh198含新回归过/原ignored1，node test compile/锁/源码diff保持 |
| 同源12app 151523Z-db3999ab | 0/379.316808s；276过/原ignored1，原七取消合同保持；不是完整Node/Quality |
| 物理R1 152244Z-802bb00c | 0/200.580265s；124全ok/final carried true/原warm及decision过；X/Y Go3649477/3640302us |
| 物理R2 152638Z-8914d68c | 0/214.366541s；同124全ok及两门过；Go3532073/3798379us |
| 物理R3 153157Z-8dea5447 | 0/152.258487s；同124全ok及两门过；Go3766136/3757942us |

以上原文件SHA/source/control/identity/group空、对应proof与ledger均终态。**物理三轮批次已关闭，不追加同源试验。**
结果E/iroh14-recovery-ack-order-physical-results-v1.json SHA **f1a4c7ef3eb61470fb5f5c377265c224ecdebde91b460d1a80eb41296136e5d8**，fixed3/additional0/screen_passed=true/**accepted=false/promotion=false**。
372逻辑请求全ok，六个Go3.532–3.798s；不是首连失败率、p99或任意移动场景接受。
E/ops/audit-recovery-episode-cost-v1.py仅复算已留日志；每episode最多三reservation。R2全启动20episode/56reservation是10PING请求+46merge，换网后两Custom episode各2PING+1merge；不要把reservation全算额外发包。

## 最新稳定升级及custom退出

官网latest在本轮16:00 UTC再次核验仍为 [iroh1.4.1](https://github.com/n0-computer/iroh/releases/tag/v1.4.1)。官网BLE/Tor为experimental custom transport，不替代khor的两腿选择/信任/自动接管，不新增本轮承载范围。
独立升级基线02807c79e04a92c46fb2e6c14d2ecf107b98336c（khor-net-iroh-upgrade-v1）为iroh1.4.1/Noq1.4.0；C3b在其上修策略。
Canonical feat/net仍原运行时 **5064fe2a**、iroh1.2/Noq1.3；后续docs提交不冒充测试源码，没有集成/发布升级。
E/iroh-upgrade-patch-ledger-v4.json：60历史项=1上游替代库验证、2移除对照红而保留、57待退出核查；57不表示全都必要。42–44原隔离编号未合主。

- iroh22手工IPv6 trim已用上游relay_tls_server_name替代，库合同过；原IPv6环境可提前返回，非WAN/手机接受。
- Noq5移除091835Z-cb319e57：四原合同三过一红，合法ACK-only最大号已回收时误增RTT计数5→6；基线同合同绿，保留。
- Noq2移除093340Z-deb1f7de：始终reporting both时，握手响应前早数据错误发送ObservedAddr TX1/应0；基线绿，保留。
- E/iroh14-custom-native-comparison-20261113-v1.json新增静态核对4/6/15/16/27：1.4.0仍按may_send_data归集ACK/连接timer ping所有路径；1.4.1仍有Relay close/queue/route无界等待、forced major通知不公开。没有新删除或行为测试；不能由该静态检查宣称这些补丁对所有部署都必要。
- iroh21容量64覆盖可评估上游Lagged+paths快照，当前roads::follow的mine/Openings未在Lagged对账；需真实burst消费滞后合同。Path::ping是该历史条目另一行为，分开退出。E/iroh14-event-buffer-exit-audit-v1.json。
- iOS callback链存在，同interface地址-only变化不独立穿过status+接口类型去重；iroh另有监控，不能静态归因现场故障。E/iroh14-ios-notification-static-audit-v1.json；真机仍待。

028已闭证据：Linux084724全vendor467/197及Mac43186999295同人口、085339固定12app273、090144真实流36/独立74Fetch RPC、095017实际CLI取消两合同、100747诊断12静默/12准备/12STREAM阳性、102059三实际接口场景372逻辑请求、111507 IPv6重叠248、111859实际QUIC NAT端口124（30/30远端warm stream新tuple）。准确范围/收据见[IROH-UPGRADE](IROH-UPGRADE.md)与历史，不相加重叠人口，不挪用于3b最终验收。
完整Node旧dc738与新028：同1387人口、1274过/同38失败/75原ignored，无new-only；均非完整Quality接受。Git2.25.1/tmux3.0a前置只是部分原因，账户runtime等未闭，不能一概豁免。

## 已关闭的诊断和普通性能批次

所有旧红、非唯一根因和不可晋升结论仍保留；以下均不得原样追加挑绿，完整ID/proof在历史与对应设计文档。

- 028 peer首连105138红、110840可达但四秒门约9.3秒红；旧dc738110236约3.47秒绿。首连错误不是注入换网后失败，未定位。
- 被动pending诊断08ecab50固定三轮闭；确认copy候选80固定三轮为首连红/恢复红/一绿；均不晋升。[IROH-CONFIRM-COPY](IROH-CONFIRM-COPY.md)。
- hop准备诊断794固定三轮均可达但decision红；只采日志，无唯一首连根因。
- 绝对收益门源67固定三轮：一绿/两final carried false；保留失败，不接受单改收益门。[IROH-HYSTERESIS](IROH-HYSTERESIS.md)。
- 同app/旧依赖源16e943d8固定三轮均过；小样本及非匹配顺序不证明新版唯一回归。两次旧compile准备含错用1.4 metadata红，均留证。
- 原生TRACE源bc3064af固定两轮：一绿/一恢复5.41、7.40秒红；捕获迟恢复，未捕获持续无carried。日志缺唯一Noq drive id，不能合并相同stream号猜连接。Noq issue889大型并发wake报告非本3节点根因证据。
- 028 vs旧dc738普通固定六block全部闭：8640交互/12file完整，有文件收益也有交互代价和新681ms p95/1.10s最大、scheduled1.89s尾巴；[IROH-WORKLOAD-RESULTS](IROH-WORKLOAD-RESULTS.md)，不代表3b性能。
- 原prefetch18场景、rolling24场景与所有原240秒文件超时均保留；rolling12block/17280交互全ok、21文件完成/3超时，不接受全局最优或完整N3。四源码/四真实RPC/Job stop控制合同已闭，生产server/worker/host-child仍待。[PIPELINE-COMPARISON](PIPELINE-COMPARISON.md)。
- 历史严重6aa及block9终端1.2–1.56秒/scheduled8–12秒未闭；后来三轮未重现不抹掉原失败。旧idle16–17秒不满足极致体验。原quiet v1准备后仍有STREAM的十二红和v2诊断准备修复均保留，[IDLE-ACTION](IDLE-ACTION.md)。

## 环境、所有权和下一项准备

- PRODUCT `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch feat/net，创建基线6485cb83。
- CONTROL `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，HEAD **5d3691f252f9dee0297fa3c494374be6119bca29**；新controls只建Git对象/ref，不移动HEAD，既有__pycache__不覆盖。
- EVIDENCE **E=/Volumes/disk0/Khor/evidence/net**。root是source-refs.json唯一串行原子writer。stage/launch/collect/verify须等真实exec会话终态；unknown handle先读receipt，不再提交。
- 本机source-only：不跑Rust/Cargo/rustfmt/Node/Bun/Xcode或产品。重型Turing/macserver+用户明确公开的colehank/khor-build；本地Git、stdlib Python源码/元数据和bash -n可用。
- Turing原UID1110/flock/private user-netns/Rust1.95/jobs16/nice10、50GiB启动/40停止/90分钟守护保持；不sudo/改宿主网络。
- Mac50GiB准入保持；43189070840空间门红后43205919189在付款/额度阶段失败、零Mac操作。最新GitHub只读仍该run，无盲目dispatch；16:00后SSH再试仍banner timeout255。异步额度/常规SSH问题未获答复，不降门/清未知缓存。
- MacLab与Turing同LAN，macserver独立；localhost旧0.2.0 iPhone以前反复关开恢复不算候选自动接管。没有新Lab/手机窗口或候选安装接受。
- 无本任务main合并、发布、生产Relay采购/部署、已装App替换授权。中央GUI授权不套用。
- 图谱缺失/partial/stale或不覆盖vendor时，0节点不是安全证明；先用已有图谱，再定向源读，不自建CodeGraph索引。

混版协议和control已在本地准备：[IROH-MIXED](IROH-MIXED.md)。E/iroh14-mixed-version-plan-v1.json / mixed-control-v1.json，control **05419f73cadb7787ee5c4356004b79f672a2c1c7**（仅原d07eec56 net-node分支）。旧dc738/new3b准确CLI native来源→不可写副本→真实sync_mixed唯一正标记/两方向row+pin、独立两host/child观测与退出。原fixture/helper/native工具两源逐字相同，Python AST/bash syntax已查；**零新远端编译/产品执行/互通接受**。两长稳全部终态收件核验后再执行一次，不抢Turing计时。旧owned run110236的bundle/package须核SHA；缺OLD_KHOR直接绿不接受，短唯一TMPDIR/只碰私有tmux，保留homes以免兜底reap隐藏close失败。
E/turing-build-tools-inventory-20261112-v1.json确认Git2.25.1/tmux3.0a/libevent.pc缺失，只有只读库存；未安装/编译新工具，长测中不可用准备名义插入重型执行。
Mac Finder trash原AppleEvent−1712（32/33）重复确认已用尽，保留失败；权限/响应根因未证，不盲目同源重跑。

Release性能仍为准备，**零Release产品执行/尚无控制任务**。E/release-workload-plan-v1.json在任何测量前补齐：原低/高负载与ABBAAB六block保持，两arm独立target以避免CARGO_BIN_EXE_khor互相覆盖；每arm只构建一次、计时期间零编译，共享下载缓存。每case新声明600秒harness守护（不是原netlab.sh已有门），原120/240秒文件/业务预算及90分钟campaign守护保持。当前old dc738对3b为interim比较，不冒充最终main集成前后性能或N3接受。

## 完整剩余范围与完成定义

- **N0**：最终分场景UX门、充分尾部样本/p99、同条件完整前后对照、观测成本及成本前沿。
- **N1**：当前长稳两arm；严重/quiet/recovery尾部、固定好路径参考、跨连接质量/旧样本、启动/配对失败、动态延迟突变、后台流量/唤醒。
- **N2**：同一实际连接的独立/同时IPv4/IPv6地址/前缀/NAT变化，换网/休眠/黑洞/LOST；owner/cancel/late/revoke/carrier失效；身份/总期限/写入和0RTT不重放。
- **N3**：完整前台文件/预览与TTY尾部，公平/饥饿/背压、共享与分连接实测、真实server/worker/host-child取消、1/3/5设备CPU/字节/唤醒/能耗。
- **N4**：Direct/公共Relay/peer relay/SSH双腿，唯一middle自动救通（第二腿可Relay/旧信息），多个/移动middle、独立双栈变化，北京/香港整机/出口/寻址故障域、自家Relay约束、双家/地域/容量/运维设计。采购部署另授权。
- **N5**：60历史补丁逐项上游等价/保留退出，Noq1及各物理成本缺口、feature/integration、最终准确源码Quality/完整Node/native/proxy、Finder、混版与真实手机自动换网/peer relay/能耗。

完成须设计理由、状态/职责边界、持续验收矩阵和同条件报告均绑定准确版本。单次绿、三物理screen绿或库升级不等于完成。
此前2–4天、4–8小时为低置信估计且偏乐观，不因compact重置；目前没有可信整体完成日期。当前两组长稳约两小时只覆盖一项，不承诺整计划两小时完成。


## 最新排期估计（2026-11-13 00:47，本地时间）

用户再次问完整完成还需多久。按当前六块未闭工作，最新用于排期的低置信估计为：**完整计划还需5–10天持续开发与验证**；具备Lab试用条件的候选目标2–3天；当前固定两arm长稳剩约1–1.5小时。不是确认交付日期，不把当前测试批次结束当完整计划完成，也不从compact重新起算此前消耗。
估计以前提Mac构建通道尽快恢复、真机验收可配合为准；最不确定的是首连/唯一middle自动接管、CPU忙及传文件时长尾、新旧互通/最终整合，以及57项剩余custom的退出核查。此前估计偏乐观，后续根据明确门的结果修订，不用单次green代替完成。
