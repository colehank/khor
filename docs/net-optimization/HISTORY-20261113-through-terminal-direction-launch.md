# feat/net 当前状态与接续入口

更新于 2026-11-13T15:14:52.517503+00:00。完整目标是[GOAL](GOAL.md)、[PLAN](PLAN.md)的N0–N5，**未完成**；逐项缺口见[REQUIREMENTS](REQUIREMENTS.md)。root串行，不派子智能体。

此前STATE逐字归档[HISTORY-20261113-through-current-noq-matrix](HISTORY-20261113-through-current-noq-matrix.md)，SHA256 `8ea6c57858cbf72d124e9ee81eb1fdc3a8c627deb23db89ea3aedb0b55667944`。它的pending/进程描述只适用于原时点，不能据历史重派；更早历史链接亦保留。本页是当前执行入口。

## 当前执行器与紧接工作

内建选择器普通Release producer **135643Z-73d37a5b 0/552.550232s**已闭21files/identity/native来源，Rust8m21s。源码仅删endpoint的一行RoadOrder注册，tree **ceca08abc132f807e74fe0b463a231b26d942645** / snapshot **b41ec41e2d76e0b753562bafe90c5737f07aebe1**；test SHA **8a6020f907a99a10faf54e1b5a8d42df9ca070000f7325211ad62284f05baec3**，CLI **5f63175c97472cf8fdce214d24d070a41ee8c236c3c9d6468b0f64b76016665f**。worktree khor-net-builtin-selector-v1 / experiment/net-builtin-selector-v1，已clean commit **5e712671e53620c14f6ac8931da64f677b557fc1**（tree不变）。内建仍含vendor补丁且默认Relay=backup，只诊断，不晋升或改回产品类型优先。

选择器固定6场已全部闭：control0d600172bb66392d8274102664934931f7be91c7，协议SHA c42d7d70fbb8f9adf66f1e09a59022322bb8c1a858c990d341be0a6255de2ed5，E/selector-comparison-progress-v1.json phase=completed、6块。RoadOrder/内建各3/3原门、各2160ok、6文件完整，替代0、运行编译0；每块13files/identity/source/native/CPU/owner/raw已核，source-refs无pending。同期RoadOrder未重现原超时，故预冻diagnostic screen **false**，不归因或晋升内建选择器。bulk目录scheduled p95 **584911/409883us**、终端 **441411/316267us**；文件中位 **23.548850/23.542792Mbit/s**；全部延迟数值门过也不抵销“未重现”限制。原Relay可用组2timeout/63blocked与更早红仍保持。完整E/selector-comparison-verification-v1.json。

紧接执行E/terminal-direction-design-v1.json的无产品改动诊断：真实PTY保持，用带私有非阻塞FIFO时刻记录的echo程序与原shell交错，区分输入到达、程序回写和返回链路。原720意图/192MiB/2秒caller/600秒owned保持；无周期产品诊断、无输入重放。只有summary完整/dropped0/无观察溢出/同CLOCK_MONOTONIC与time namespace/epoch bracket≤1ms才解释缺失输入；换echo程序本身不是性能接受。夹具已在独立khor-net-terminal-direction-v1 / experiment/net-terminal-direction-v1实现，HEADf6b9、3 staged，准确tree **776ecc6b2bf358bb9421dbbd5c988cb4f1a52a7b** / snapshot **f2131be0c896a4b9ad94ae285a0cc0b0a9a98092**。所有产品文件逐字原A，原shell控制保留；新增Python -E -S真实PTY echo、私有非阻塞FIFO、仅test进程的512KiB/5ms读者、测量后1秒内请求summary，未加产品探测/计时器。远端helper实际4/0/0已验证：分片输入、多行、满记录管道仍回显且报告drop、无业务输入的带外summary；E/terminal-direction-helper-check-v1.json/log，helperSHAf741b27cd43c71ac2a864733f8ede7f061501158920adc324519badf16d6c599，固定CondaPy SHA c46ed00d。

terminal direction producer **145811Z-6f3704a7 0/544.370115s**已闭22files/identity/native来源，Rust8m14s；test SHA **499340a0655fc827e5e792af7c364ade14188560901a1a8aa332975c05462954**，CLI精确重现原 **a7e3565f15ed15d59f5215bceeb3900d153d053dd853846f623d86efbe6e4ac9**，产品无改动。E/run/terminal-direction-native-compile-verification-v1.json。

当前唯一pending为固定6场终端方向诊断 **151349Z-c16a9107 / PID461002**，control **0f50c7bcaa76a37897431f4dc0832ce271579f5a**，同tree776ecc6b/snapshotf2131be0，协议E/terminal-direction-runtime-plan-v1.json SHA **a6893aaa724d3f121c3c9f316ccdfbaf7b1808b9e99a5ea37f25f5475c7330ef**。单一远端controller自动依原/capture/capture/原/原/capture串行6场，无本地后台协调器，root是唯一source-refs writer。普通Release、不编译运行、不替代失败、6场均同可用Relay，原720/192MiB/2秒/600秒保持。

分析边界已在运行前冻结：at_ns采于I/O之后，只是完成时刻上界，晚stamp不能证明晚到或晚写；只有早于观测timeout下界的reply_written才证明程序已经写出。summary只验证截至第一summary的连续无丢记录前缀，保留其后全部原记录；缺input最多证明程序截至summary未处理完整行，不能推成包没到内核。时钟namespace/epoch bracket≤1ms/无读错与溢出仍必需；带记录程序不是最终性能或能耗接受。

E/relay-selector-static-audit-v2.json记录已排除的假设：host的Welcome后本来是raw PTY输出，fixture积累TCP字节匹配并非误解为未解码ServerOp；p.rtt本来已有传输估计回退；Main等FIN而Tunnel双工，不能只改Proto枚举来“共享连接”；内建低multipath值会被忽略；W发的是path.ping补探测，不是通用驱动wake修复。避免重走这些假设。

上一Relay可用性隔离固定6场 **132935Z-b055e94a 1/1161.211608s**已闭18files/identity/native来源/CPU/全部owned清理。control78f4657adb796765bf3317a044d2829ba0296096；协议SHA d901305ab1f394b08c403d9478475955fef5aec10f465a9408a6064b51426983。六场前置均有效、4320完整意图、替代0、运行编译0。可用Relay组原1/3、2095ok/2timeout/63blocked；不可用组3/3、2160ok，6个192MiB文件均完整。两次原2秒TTY超时分别idle_after index82与idle_before index93（后者起始调度lag10.444035秒），不能单按大文件挤占带宽解释。bulk TTY两组360个全成功，但scheduled p95 **5.874877/0.336874秒**；目录 **0.786266/0.506660秒**；文件中位23.302977/23.427441Mbit/s。失败相位的成功分位数是条件统计，blocked非网络超时。

完整[RELAY-AVAILABILITY](RELAY-AVAILABILITY.md)及E/relay-availability-comparison-verification-v1.json。实际CLI与原超时C producer040254Z-2e151f7f逐字同SHAa7e3565f，绝非新修复；只收敛到可用Relay/传输/选路交互，未定具体根因，不建议产品禁用Relay。源实验已clean commit **f6b9a75633485b8ab29f187479506f186ba88618**，tree953fd88f5f39fa99bc726296e5a84559996ce5cc、snapshot90d743c8保留，旧D前置失败原件保持。

紧接在相同可用Relay/BBR/真实业务条件下只比较RoadOrder与iroh内建选择器；后者默认Relay=backup，不符合用户速度优先要求，只作隔离控制，不晋升。必须先冻结源码/协议/实际producer后运行，不无条件重复上一批，也不把vendor内建选择器称纯上游。

producer **132330Z-5893f7e8 0/230.673910s**，Rust2m59s，22files/identity/native provenance闭；test SHA **42d8f0ba72908748cde69e3f5b91834683ec38ec90c0b084fc03564baacb8533**，CLI仍与I117/D完全同bytes **a7e3565f15ed15d59f5215bceeb3900d153d053dd853846f623d86efbe6e4ac9**。先前v1 **131622Z-0132e71c 1/214.672428s**已闭17files/identity/空owner：Cargo成功2m45s，但只准备test入口，严格验证拒绝fresh CLI的新包key；未准入、业务0。v2仅调用原prepare_build显式准备CLI，验证器不改；原未准入5文件已先归档为诊断，不作prior。

独立Mac whole-hop **43432815139 SUCCESS**已闭：准确R9081b94d/treecf9d172b、原Linux全部92个名字，**92/0/0、105.52s**；artifact14039959512 SHAf76760c5ecb3821f546aea53b326c3f66536bb3f5b10f55a3c04f497af0b275f，14files/source/control/ZIP CRC/各raw/host/50-40GiB guard/jobs2均核。E/public-mac-live-middle-whole-hop-v1/verification.json。复用ceeb30968b274bb0b8da0f5a41ebf520原owner源码refs，新request92505a44604d4167adeab5d7e2d1bd80未退役它们。仅窄回归，不代全部NET16/最终N5，不安装/发布App。

新整理集成候选 **khor-net-product-integration-v2 / integrate/net-product-v2** clean，HEAD **3093f86662b9b9ff139f29f85692e84ec2e47358** / tree **bf4d38d73fbb8eda558d08446deff2d930b65304**：产品runtime逐字R9081b94d（I117加窄live-middle修复），另仅两份已46回归验证的native provenance工具修正；apps/.github仍逐字冻结公开main c3b098bd，保留原I。E/product-integration-v2-source-audit-v1.json。未修N3原红、未晋升/合并/发布；最终整源门仍须后续执行。root唯一ledger writer，无后台协调器，旧20826/53952已退出。

Direct/Relay配置隔离producer **124033Z-8cfa87a1 0/596.350447s**，普通Release、Rust9m06s、20files/identity/native provenance已闭；source tree **59639aac742bd404fc9bea1618f29345907ac238**，snapshot **5a41f54d37d8755cb54e31478e61a5cddff6c2f4**。test SHA5ba6fa9ea5d828be2fc782fdaac5100f5dcc3474df0d367c1f36e7a260078f98；CLI a7e3565f15ed15d59f5215bceeb3900d153d053dd853846f623d86efbe6e4ac9。实验树khor-net-direct-isolation-v1，HEADd6b97c3d、2 staged；runtime逐字I117 BBR，不含R/W/M38。

runtime **125829Z-17b3241d 1/195.119839s**，control **801cd2cb052e214505041a7d5598b9e2cc385c8b**，14files/identity/全部owned清理已核，source-refs已闭。固定6场仅执行2场：normal第1场原门通过、720/720业务ok、192MiB完整，bulk目录/TTY scheduled p95 **655612/317657us**，文件25.192292Mbit/s；第2场在计时前前置失败，无manifest/sample/bulk；其余4场未执行、替代0。完整比较**无效**，E/turing-20261113T125829Z-17b3241d/direct-isolation-partial-verification-v1.json；原raw保留，仅按预冻适配移除第1场一个非业务控制记录作原解析。

原因已由实际源码确认：node::chosen_relays明确在清单为空时关闭only_mine，恢复内置Relay，是已有兜底设计；并非产品已违反禁用设置。此前把空清单视为禁用开关是夹具假设错误。不能以此构造Direct-only性能结论，也不新增产品禁用开关。当前改为**相同配置、Relay可用性隔离**：仍只配置相同Lab Relay，两组产品runtime/BBR一致；在实验fixture中停止故障组的真实Relay并核实端口关闭，测有/无可用Relay时原真实Node工作负载。协议/前置已预冻，正在仅编译，之后才固定执行；原失败及6场计划保持不变，不回填未执行场。

原配置/发布不证明两条活Noq copy；新实验也必须限制因果解释，保留journal。Main目录/文件与Tunnel PTY本来已分连接，不能声称是新设计。仅实验诊断，不建议产品禁用Relay。编译/计时分离，计时不编译，不改原720记录/192MiB/2秒caller/600秒owned/Direct前后及功能门。

## 已闭Noq分量矩阵：不全局切CUBIC

**120549Z-13cd71dd 0/1115.477027s**，50files/identity/CPU/19个private user/net namespace及全部owned清理闭；19/19原native门，**1101/1101请求ok**、替代0。普通Release opt3/strip/thinLTO/codegen1，无qlog/diag，运行时编译0。协议0c3eaf80d795a78c3909f475ff285e015c483bbb8a12e3456c7ed8bdc76fa12a；全量E/cc-matrix-comparison-verification-v1.json，各run下cc-matrix-runtime-verification-v2.json。

300ms、20Mbit/方向、200ms队列、双向丢包固定0/2/2/0/0/2%，CUBIC/BBR各条件3轮。沿用原20秒bulk及typing→screen，新增10秒无应用请求→短流/screen；原Noq层10秒ask不是Node原2秒门的放宽。CUBIC/BBR吞吐中位：0% **16.507651/11.686758Mbit/s**；2% **0.325742/10.067335Mbit/s**。2% bulk echo p95 **763.900/455.850ms**；0% typing后screen **429.567/307.491ms**、quiet后screen **430.954/307.321ms**，这些screen各n9。CUBIC未过预冻分量screen，不能全局替换BBR。Noq模型中最大观测BBR1.283s/CUBIC1.407s，不是原Node 2秒超时的重现或根因证明；iroh/multipath/Relay、Node/IPC/同步/SSH/hop与能耗尚未由它覆盖。

原v1数据解析器只认行首marker，Rust libtest会把test-name前缀与首条stderr指标放在同一行，漏掉首CUBIC bulk/quiet行。v1原件/SHA保持；E/cc-matrix-parser-correction-v2.json预记确定性修正，v2收齐全部marker，原样本/时限/quantile/门/日志不改。原strict 2bulk/6quiet计数仍保持，全部原summary由raw重算。

该源现clean commit **d6b97c3d41d13fb1b9ef209adb89975bfdd727af**，tree **23b5d7e9d841e1411ba84a68db91ed1f95d7bcff**，worktree khor-net-cc-matrix-v1。其CUBIC endpoint仍为实验，不晋升。producer120149Z-9c128417 0/74.067894s、Rust18.97s，22files闭，testSHA7b8d34deba4507b1d96fe8c059e43bcc64c4ebae837c68c15d30a19d9e637fe7；原selected2文件保留。

构建工具修正是实际Cargo dev依赖不传播：请求test/example/bench保留自身dev边、所有normal/build边，typed/legacy IDs不一致、缺typed或未知kind拒绝；原source/actual row/dep-info/hash检查保持。原37+新增9回归在旧代码精确3fail/2error，在新代码46/0/0，真实普通Release亦获准。helperSHA7dbd2c0f4048f8cc96a3c9303565b6298c4999daccdd4fe360825d21d225ac7b；**这是明确修正后的验证器**，不能写成逐字未变。原5个准备/准入失败及未准入exe/dep-info归档完整见历史及原receipt，不作可信prior。

## 普通Node Release仍有未解决项

[COPY-WAKE-RELEASE](COPY-WAKE-RELEASE.md)固定18场/12960记录全闭：B6/6、4320ok；C6/6、4320ok，但目录尾延迟仍过原工程界限；W5/6、4200ok/2timeout/118blocked。W high在idle_before/bulk各原2秒TTY超时，low终端排队达数秒，拒绝作为修复；blocked不是网络超时。C本轮TTY成功不抹掉更早high2/3失败。qlog/native ACK各3绿亦不抹原红。

[CUBIC-ABLATION](CUBIC-ABLATION.md)普通Node固定3 high：111336Z-865645d6 1/504.032874s，2160ok、3文件完整/PTY关闭，但原门2/3（首轮direct_after=false），保留profile红。bulk目录/终端p95 351845/334029us，文件中位36.734944Mbit/s，较早C580146/350138us、24.307740改善只作非同时交错参考；不推广为根因或最终接受。当前Noq丢包矩阵进一步否决全局CUBIC方案。

## 地址变化与sole-middle的有限接受

[TUPLE-REFRESH-EXIT](TUPLE-REFRESH-EXIT.md)：固定R M M R R M六块/12native场景，744真实读全ok，180暖远端开流，替代0。R强属性3/3、零mismatch；删patch38 helper的M38强属性0/3、累计215 cached/actual mismatch，暖远端正确数0/30、30/30、0/30。当前R/M源、iroh1.4.1/Noq1.4.0的实测NAT合同下保留patch38，不能据Mac库双198绿删除。原30秒caller/120秒setup/45秒hold/240秒scenario/300秒owned保持。完整E/tuple-exit-nat-comparison-verification-v1.json，不覆盖全部IPv6、双端/middle移动、手机或能耗。

[LIVE-MIDDLE-RETRY](LIVE-MIDDLE-RETRY.md)：R9081b94d/treecf9d172b，原sp7三真实前置失败保留，修后070832Z-6b87e77c固定3恢复3/3，1013894/784788/878995us，总5秒不变，真实Direct首腿+Relay第二腿/held hint/marker/同PID/owner皆核。Linux wholehop92绿。仅有限Ask/Forced、无target road且首腿真开放时按原HOP_WITHIN3秒复查；后台/死首腿退避、TRIES2/trust/单跳/写不重放保持。只接受此窄恢复，不代表全部NET16/手机。

## 源、环境与权限

E=/Volumes/disk0/Khor/evidence/net；全部工作树位于/Volumes/disk0/Khor/worktrees/current/。canonical feat/net runtime仍5064fe2a、iroh1.2/Noq1.3，文档提交不代表集成。I117c39e5 clean，iroh1.4.1/Noq1.4.0，冻结公开main c3b098bd的apps/.github；原交互失败及目录尾延迟未解，不最终晋升。R9081b94d clean、不含W；W treef5faf2a2拒绝；M38 tree2d702718退出失败。原B/C及所有实验/源/日志保留。khor-build-net HEAD5d3691f2未移动，仅新增控制Git对象/refs分支。

本机source-only：Git、stdlib Python源/数据/metadata/ZIP/hash与SSH/GH编排；无本机Cargo/Rust/rustfmt/Node/Bun/Xcode或产品native执行。重型仅授权Turing/macserver/可信GH；khor-build已用户明确授权public。Turing UID1110、排他flock、jobs16/nice10/Rust1.95、50GiB准入/40GiB停止/90min outer、私有user/net namespace，无sudo/宿主网络改动。Mac实际guard50/40GiB、target100GiB，SHA998f5723eb447a1634411397c18a524e5ed0d50bd6dbd81001a16626219f64eb，不降；jobs2，仅已有安全inactive owned cache维护。MacLab/Turing同LAN，macserver另机/网络。

无main合并、正式发布/标签/TestFlight、已安装App替换或生产Relay采购部署授权。保护其他任务Lab23961；准确source/PID读中央STATE，不能复用历史PID。旧I Quality43348869616首次13/13、MacLab43355810513 attempt2及内ZIP/source/hash闭证只适用I，未本地执行/安装。物理iPhone仍旧0.2，新候选/真实换网及能耗未接受；有可用候选后再约新窗口。

## 全目标剩余

N1完整路径/共享测量/稳定与成本；N2总期限、取消/资源、读写与0RTT不重放、全部NET16；N3真实同步/预览/SSH/hop公平、背压取消、共享/分连接实测与1/3/5设备CPU/线程/字节/真实唤醒和手机能耗；N4双端/middle独立及同时IPv4/NAT/IPv6隐私IID/前缀、WiFi↔蜂窝/锁屏/休眠/发现发布、Relay故障域容量与布局；N5逐项custom、最终同源Quality/Mac/手机、设计/报告与回归均未齐。旧p1h包+33.970%/字节+37.908%不抹掉。ledger v5的60项=1上游替代、2Noq移除红保留、1 iroh38移除红保留、**56待退出审查**，不是56均必要。

[MOBILE-ADDRESS-AUDIT](MOBILE-ADDRESS-AUDIT.md)：iOS netwatch AF_ROUTE独立通知确在，App表面去重不证明漏地址；250ms连续chatter仅风险。最新稳定于2026-11-13T10:15:56Z官方registry再核仍iroh1.4.1/Noq+proto1.4.0，原JSON/hash见E/upstream-registry-recheck-20261113T101556Z/verification.json；最终交付前再核。所有阶段绿都不能当N0–N5完整完成。
