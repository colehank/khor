# feat/net 当前状态与接续入口

更新于 2026-11-13T12:10:50.492207+00:00。完整目标仍是[GOAL](GOAL.md)、[PLAN](PLAN.md)的N0–N5，**未完成**；逐项缺口见[REQUIREMENTS](REQUIREMENTS.md)。root串行，不派子智能体。

此前STATE逐字存入[历史](HISTORY-20261113-through-release-wake-counterexample.md)，SHA256 `5df14783b2790189a9cd5b4465995966163f585029fbeb9ceaae45ec9657b21b`；其在途/无pending只描述原时点，不能据历史重派。更早历史链接亦完整保留。本页是当前入口。

## 当前执行器与紧接工作

NAT固定对照已完整结束，root串行调度器PID53952及上一Release调度器20826均已核实际退出，两个progress保持completed；root当前是source-refs唯一writer。不能因历史receipt重新派发。

CUBIC普通Release固定3 high诊断 **111336Z-865645d6 1/504.032874s**已闭15files、identity/raw/CPU/owner/准确producer及selected还原全核。control **f0975c8945327ae2e1513b4a5e0778446dc680a6**；原协议SHA8dad4d48b065289461f3b3c98d35c5393d7ef272fea662dbd193e0e11c73b539。完整E/cubic-release-diagnostic-verification-v1.json。

2160/2160操作ok、零超时，3个192MiB完整文件且PTY关闭；原native门 **2/3通过**，首轮因direct_after=false失败（其余业务均成功），原红保留。pooled high bulk目录/终端scheduled p95 **351845/334029us**，文件吞吐中位 **36.734944Mbit/s**；先前C对应580146/350138us、24.307740Mbit/s。对冻结早先B/C的六格原延迟界限及吞吐界限都过，但这是非同时交错的诊断参考，**不是最终比较接受或已证根因**。不因业务全通删除选路/profile原门。

CUBIC producer **110223Z-34e7344c 0/586.890585s**已闭20files，实际编译8m54s，control44328e0e…，source tree21f37a7a。test SHA307f9f1759409f734464e07f458355abbcf177d5eb9a10821a6b83ae63647b3a；CLI SHAe1deef755b743684b606c51e2c950a07cc8d2f3a2b0444b0c929853e21dbd74d。原C父版本5个selected ELF/dep-info已保留；CUBIC自身5个selected也已存closed-selected-release-v1/该producer，未删除旧B/C/W。原opt3/strip/thinLTO/codegen1、无qlog/diag及原native验证器保持。

紧接工作：当前Noq1.4的CUBIC/BBR受控丢包、typing后burst和真正无应用请求的idle后短流。沿用已有独立双向FIFO congestion.rs及校准，不把旧Noq1.3数据当当前结论；先绑定准确source/protocol/producer。新测量树khor-net-cc-matrix-v1 / experiment/net-cc-matrix-v1，HEAD117、7文件staged；CUBIC共同fixture外只有congestion.rs新增quiet测量及构建依赖校验的两个Python文件，未改其他产品runtime。当前准确tree **23b5d7e9d841e1411ba84a68db91ed1f95d7bcff** / snapshot **c8a1ce24babfad4a68af86b4c0d530596932c1be**。唯一pending为runtime **turing-20261113T120549Z-13cd71dd / PID138181**，receipt=E/turing-20261113T120549Z-13cd71dd/receipt.json，control **7b1b5f0b588470af70afc7d04e6e0361c932c880**；协议SHA **0c3eaf80d795a78c3909f475ff285e015c483bbb8a12e3456c7ed8bdc76fa12a**。实际已核running，原FIFO校准、首轮bulk及typing三个native case的owned进度已完成，正在quiet；不把中途进度当全批结论。原子source-refs由root唯一writer，无本机后台调度器；终态后collect→identity→verify-cc-matrix-runtime-v1.py逐条重算原Noq summaries/quantile、实际私有namespace、CPU和19场景全量/owner，再闭ledger，不能重派。

producer **120149Z-9c128417 0/74.067894s**已闭22files，实际Rust编译18.97s，test SHA **7b8d34deba4507b1d96fe8c059e43bcc64c4ebae837c68c15d30a19d9e637fe7**；controlfb5a491a…、原Release opt3/strip/thinLTO/codegen1、无qlog/diag。原37+新增9=46项：旧校验器精确3fail/2error（均新增用例），修正代码46/0/0；原source/row/dep-info/hash门继续保留，未改业务期限。证据E/该producer/cc-matrix-native-compile-verification-v6.json。runtime实际绑定producer的5份原报告SHA，各case是新的private user/net namespace、仅启用私有lo；运行时CC参数仅传子进程，不污染生产凭据的编译环境key。原300秒owned watchdog保持，整个矩阵无再编译。

准备与产物拒绝均闭证保留：v1 **112934Z-a0f72d29 1/66.149648s**格式红、Rust/业务0；v2 **113430Z-875ee934 1/148.739250s**Cargo成功1m38s，但校验图误纳入catalog仅用于自身测试的core，拒绝准入；v3 **114315Z-d25da37f 1/23.848835s**归档返回值未解包、编译/业务0；v4 **114549Z-d70981f3 1/82.039474s**额外core库测试带入未声明GUI输入，被原校验拒绝；v5 **115002Z-61a5f244 1/106.299153s**普通库顶层dep-info不受原配对器支持，被拒绝。所有identity/空owner及原日志闭，均无矩阵runtime，不能把Cargo成功或未准入exe当产品绿。

v6改正根因：native_provenance按Cargo的typed dep_kinds选图，只对被请求test/example/bench加入自身dev依赖，不传播依赖库的dev边；normal/build及原source/row/dep-info/hash门保持，typed/legacy IDs不一致或未知kind拒绝。回到原单一khor-net congestion no-run命令，不再额外编译core。新增9个正/负回归，与原37项一起，在远端先对旧校验器要求精确5红（3fail/2error），再对新代码要求46全绿；有界ctime barrier只等模拟fixture时钟，不改原断言、生产时间戳或凭据。本producer46项红/绿对照与实际Cargo产物均已核准。此处**校验器已明确修正**，不能继续写成原校验器逐字不变；Rust测量及产品runtime不变。源SHA7dbd2c0f4048f8cc96a3c9303565b6298c4999daccdd4fe360825d21d225ac7b，官方Cargo语义来源记于E/cc-matrix-design-v3.json。旧未准入exe/dep-info在每次变动前独立保留，不作可信prior；旧CUBIC prior仍按实际单位key与哈希判复用。

E/cc-matrix-design-v3.json预冻0/2/2/0/0/2%双向丢包顺序，300ms/20Mbit/200ms队列、每臂3样本、原20秒bulk、10秒纯应用静默及原10秒Noq层ask；不是Node原2秒门的放宽。复用原Noq直接两控制器及FIFO校准，typing/screen与quiet/短流独立记录。CUBIC相对BBR的分量screen为全成功、每loss吞吐中位≥.9、每phase p95≤1.10×BBR+20ms；纯Noq结果不作最终产品接受。编译已闭并冻结实际producer/runtime control后开始矩阵，当前在途；无最终结论。全原工程门、相关恢复/稳定/闲置/手机与最终同源矩阵仍缺，CUBIC未获默认晋升；详[CUBIC-ABLATION](CUBIC-ABLATION.md)。

NAT已闭6块/12原native场景、744次真实逻辑读全部成功、180次暖远端stream-open。R baseline强属性 **3/3**、零mismatch；仅删patch38 helper的M38强属性 **0/3**，累计 **215** cached/actual tuple mismatch。M三轮暖请求正确数分别0/30、30/30、0/30；其中一轮暖读正确仍有其他路径缓存不匹配，不能概括为全部暖请求失败。固定R M M R R M、替代0，全部原业务/来源/实际注入/owner/selected还原均闭证。结论是**当前R/M源与Noq1.4的实测属性下保留patch38**，不泛化到所有移动场景。完整E/tuple-exit-nat-comparison-verification-v1.json、[TUPLE-REFRESH-EXIT](TUPLE-REFRESH-EXIT.md)。

NAT runtime control **35f36b80d5d7fd21b20a7cac66164d8426618a61**。producer均闭17files：R **102852Z-ad942c9e 0/92.207558s**，treecf9/snapshotb385；M **103337Z-208ba0bf 0/86.093514s**，tree2d7/snapshotc575。均net-diagnostics/lifecycle-only、无qlog，root/vendor fmt过；所有原selected含dep-info已归档，runtime物化准确source后还原producer相同字节，使用原native_provenance，不改凭据。原300秒owned watchdog及原30秒caller/120秒setup/45秒hold/240秒场景门均保持。

## 现场和授权

E=/Volumes/disk0/Khor/evidence/net；工作树均在/Volumes/disk0/Khor/worktrees/current/。本机source-only：Git、stdlib Python源/数据/metadata/ZIP/hash及SSH/GH编排；不运行本机Cargo/Rust/rustfmt/Node/Bun/Xcode或产品native。重型只授权Turing/macserver/可信GH。保护用户main、已安装App及其他任务Lab23961，具体其他任务source/PID查中央STATE。无main合并、发布/标签/TestFlight、已安装App替换或生产Relay采购部署授权。khor-build已获用户明确公开授权，当前public，不按旧私库额度阻塞。

Turing UID1110、排他flock、jobs16/nice10/Rust1.95、原50GiB准入/40GiB停止/90min outer，private user/net namespaces，不改宿主网络或sudo。Mac实际安装guard为50/40GiB、target100GiB，SHA **998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb**，不降低；jobs2。已有prune-test-programs仅清无占用owned旧测试exe、留每目标2个，规则/guard保持。SSH macserver本轮8秒banner超时，GH已实证可用。

## 准确源码和晋升边界

- canonical khor-net/feat/net：runtime仍5064fe2a、iroh1.2/Noq1.3；本页文档提交不代表集成候选runtime。
- I khor-net-product-integration-v1：clean **117c39e5432741198c924177fae1775ff292eada**，treeace4fd82；iroh1.4.1/Noq1.4.0，parent冻结公开main c3b098bdbf52a9b3b38e887ae74b90155292b544，apps/.github与该冻结main一致，不自动追随移动main。**原N3超时与本批目录长尾均未解决，拒绝最终晋升**。
- R khor-net-live-middle-retry-v1：clean **9081b94dce827cbd3fb9b9e7bb13633d762e3cd9**，tree **cf9d172bb92778ccfd97899ddea27bdf0e6d2f6e**。被测dirty snapshot a3cad264与该tree逐字相同。runtime仅hop.rs不同，另有原physical/unit fixture；窄恢复接受，未集成canonical/I。
- W khor-net-copy-wake-v1：HEAD117、四文件staged；tree **f5faf2a2fef7fd64f09908fc53c73b24bb2eae4d** / snapshot **8780bf503d753b820a7236931357b2dcebfa42a6**。C共同fixture treeadbc7d6a上runtime仅remote_state.rs取消off_direct wake guard，不含R。**本批普通Release真实红，拒绝作为超时修复/默认设计**；保留实验及所有绿/红，不集成。
- M38 khor-net-tuple-exit-v1：HEAD908、一文件staged；tree **2d702718ff8f664cc1e2fa5db2352e2d8e91ee8d**，snapshot **c5754ef1f3705c3ccfab3180a111bf058a0cca42**；只删patch38 helper/调用，常规select_path_on(true)保持。Mac源container0f11a5f7同tree。**库对照过，但Linux强属性移除臂0/3，保留补丁**。
- Cubic khor-net-cubic-ablation-v1 / experiment/net-cubic-ablation-v1：HEAD117、四文件staged；tree **21f37a7a2fb96abcd601db486c26b687767cade2** / snapshot **74c762dfffb849b9ec4cb5e68f588dd9698df50d**。以C共同fixture为底，runtime只将endpoint::transport BBR3Config切为原生CubicConfig，无vendor/W/R改动。**固定3 high闭，2160ok但原native2/3；仅诊断，未晋升**，详[CUBIC-ABLATION](CUBIC-ABLATION.md)、E/cubic-ablation-{source,design}-v1.json。不能因零丢包模型变好而忽略历史CUBIC丢包吞吐劣势；最终选择仍需完整loss/idle/burst/短流及原工程门。
- khor-build-net HEAD **5d3691f252f9dee0297fa3c494374be6119bca29**未移动；仅新增控制Git对象/refs分支，不改默认main。B/C/Q/A/T等旧隔离树与staged原件保留，准确版本/证据见历史和各专题，不按摘要覆盖。

## 完整Release三臂结论：两候选均不晋升

[COPY-WAKE-RELEASE](COPY-WAKE-RELEASE.md)：原固定B C W / C W B / W B C、18业务场景、**12960/12960**计划记录全留，替代0；三臂各3 low+3 high，原真实PTY/目录/16MiB及192MiB完整文件、2秒caller及全部原门保持。全部owned清理、actual source/CLI/selected ELF+dep-info还原和CPU准入闭证。root串行器20826已退出，12个run均已非pending。完整E/copy-wake-release-comparison-verification-v1.json。

B 6/6、4320ok；C 6/6、4320ok；W 5/6、**4200ok/2timeout/118blocked**。W block5 100245Z-0d77409d为有效产品红（1/349.996573s）：high在idle_before和bulk各一个原2秒终端超时，blocked为未发意图，完整192MiB仍成功/PTY已关；low虽720ok，单轮终端bulk/after scheduled p95 **8.868761/9.910280s**。本批W pooled low终端p95 bulk6.379894s、after8.396699s；B仅80.491/43.197ms。不得用吞吐或稳定性8绿覆盖。

C也未过原工程门：low bulk目录p95 **371017us** vs B239185，原界限283103.5；high **580146us** vs B452402，界限517642.2。high文件中位 B19.7153/C24.3077/W23.4936 Mbit/s不能抵销交互尾延迟。C本轮TTY无超时不覆盖原Release high 2/3超时失败，不能宣称修复。补核E/copy-wake-parent-recheck-analysis-v1.json。三臂cold TCP参考各原红保留，真实分层准入过，不把TCP原门改绿。

原公开c3对I117的Release ABBAAB旧批仍完整：8640记录/12file，C high1/3、3timeout/232blocked；qlog与native ACK各固定3（各2160ok）未复现原超时，不是修复或普通性能接受。Noq路径与回复观察只限其各自来源，不合成单一已证明根因。

## 已接受的窄恢复及稳定性证据

R [LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)：T3/sp7 064222Z-57c213de原三前置真，M已0.34–0.56s读到Y而X5s超时；原红保留。R compile070105Z-8e82e611、17files/whole hop92/92/0ignored/原native runner20unit；runtime **070832Z-6b87e77c 0/98.267049s**，原sp7固定3恢复3/3，总1013894/784788/878995us，真实Direct首腿+Relay第二腿、同PID、held hint/marker/drop及原总5s保持。仅有限Ask/Forced无target road且真开放首腿才在原预算复查；3秒最短间隔复用HOP_WITHIN，后台/死首腿仍指数退避。TRIES2/epoch/owner/策略/信任/单跳/写不重放保持，全部NET16/手机/多middle仍缺。

W library063044Z-bf939c7b 198/0/原ignored1；Debug producer071457Z-c140a37b闭、CLIbc47a17b…/test8116cf8e…，旧R全部selected含dep-info存closed-live-middle-retry-v1。W原控制v2 **073647Z-7e574645 0/4752.282312s**、29files闭：原p1s×6各600s、p3黑洞、p5十二停顿，8/8原门；helper echo实际目录读，不算PTY。事后372完成目录读均ok，无新SLO。v1历史owner513>512中断及8僵尸原字段保留；postmortem实际全消失。v2历史cap8192/同时live仍512与纯数据校准保持，旧中断不是产品红绿。

W普通Release producer **092115Z-acbcbe95 0/596.523376s**、20files闭，opt3/strip/thinLTO/codegen1、无qlog/diag；原37模拟unit通过有界ctime barrier（仅模拟fixture等待一tick，原dict/断言/校验器和产品timer不改）。原091727准备失败37项中32过/2fail/3error保留，Rust编译0，不靠删除测试放行。原生产脚本/上下文见历史/原proof，不把hash当路径。

## Mac库与NAT退出

Mac baseline **43394584580** 与disabled **43396826114** 均success，完整iroh库 **198/0/原ignored1/0filtered**，199名称/结果相同；root/vendor fmt、实际iroh/noq-proto vendor路径、noq registry1.4.0、精确tree/control/ZIP/CRC/hash均核。baseline artifact14022571285 SHAcea062709…；disabled14023303488 SHA29bf14c4…，各E/public-mac-tuple-exit-{baseline,disabled}-v3完整证据。编译8.80/7.73s，运行45.80/45.58s，库绿不判NAT可删。

原Mac v1大JSON截断、v2 guard<50GiB拒绝均留，Rust执行0。v3仅已有安全cache维护清23旧inactive测试exe约2GB，guard前后SHA不变；raw Git blob运输保持64MiB cap/Git blobSHA1+SHA256。v2/v3复用ceeb3096原source/bundle refs，非新request所有；全部闭证后只能原owner核SHA再退休。disabled有自己75ad…refs。尚未删除。

NAT强oracle scripts/netlab-nat-selection.py / warm_remote_requests_select_the_rebound_tuple_v1：r0/r1 fixture SHA **ac97726906429a5dd63dd91b5c9ed99624c273d9a253f7facb916ffcf47e8a9b**，原124读全ok、实际50001→50002/接口v4v6不变/conntrack真删除、30暖远端stream-open均新tuple、零cached-actual mismatch与journal新tuple。Rust r1单门没断言全属性。原30s caller/120s setup/45s hold/240s场景不放宽；Noq4强oracle及IPv6/手机矩阵不可互相替代。

## 仍需完成的完整范围

N1速度优先/共享测量/稳定与成本；N2总期限、取消、资源、读写/0RTT不重放及全部NET16；N3真实sync/预览/SSH/hop公平、共享/分连接、1/3/5设备CPU/线程/字节/真实唤醒与手机能耗；N4两端及middle独立/同时IPv4/NAT/IPv6隐私IID/前缀、WiFi↔蜂窝/锁屏休眠/发现发布、Relay故障域容量与布局；N5逐项custom退出、最终同源Quality/Mac/手机与设计/报告/持续回归，均未齐。旧p1h包+33.970%/字节+37.908%保持，不能称空闲最优。

I原Quality43348869616首次13/13、MacLab43355810513 attempt2（原WebKit红保留/唯一失败job重跑77case绿）、native内ZIP/source/probe/CRC闭，未本地执行/安装替换；只适用I。旧/新双向row/pin与host/child011900Z-19b49119只证明双方懂Versions的互通。物理iPhone仍旧0.2，新候选/换网/能耗接受缺。MacLab/Turing同LAN，macserver另机；不混淆网络。

移动静态审计见[MOBILE-ADDRESS-AUDIT](MOBILE-ADDRESS-AUDIT.md)：netwatch0.19.3 iOS AF_ROUTE独立通知确在，App表面去重不独立证明漏地址；连续chatter250ms debounce只列风险。registry **2026-11-13T10:15:56Z**官方API再核仍iroh1.4.1/Noq+proto1.4.0，原JSON/hash见E/upstream-registry-recheck-20261113T101556Z/verification.json；最终交付前再核。custom ledger v5（E/iroh-upgrade-patch-ledger-v5.json）60项=1上游替代已验、2Noq移除红保留、1 iroh38实际NAT移除红保留、56退出仍待；不能称56项均必要。
