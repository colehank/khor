# feat/net 当前状态与接续入口

更新于 2026-11-12 15:55 UTC。用户已恢复“继续”，完整目标“完成计划”仍为[PLAN](PLAN.md) N0–N5，**未完成**；root串行，不派发子智能体。用户“我没有验收 打错了”有效，没有新的Lab/真机接受。最新U18授权跟进最新稳定iroh并尽量减少custom；旧AGENTS仅规划状态已被后续授权覆盖。
原STATE逐字保存于[本次历史](HISTORY-20261112-iroh-upgrade-through-mcp-preparation.md)，SHA256 `3ed8de20f1d15b0088559416195ff5ac0070398554387c3993c43f7094ea17ec`；其中继续链接原stream准备/暂停/恢复历史与完整旧证据，不按摘要覆盖未提交内容。

## 当前执行与下一步

**当前在途长稳 baseline：turing-20261112T155449Z-56a8ac18，PID2128254，15:55:10 UTC启动**，task netlab-stability / revision iroh14_policy_stability_baseline_v1 / control f862ab2defb48d21236c2bdcca736c9a3d108773 / source515e99f0d886de8035801801a10e7587167cf78b。预计约50–60分钟，原90分钟守护不变。启动真实exec已终态；receipt与ledger已存，不重复提交。两arm执行前元数据 E/iroh14-policy-stability-execution-v1.json 绑定准确双方source、control、协议/source audit/三物理结果与两个readiness proof。baseline全部收件与核验终态后，只启动预声明candidate一次；不挑绿、不追加替代。长测期间不在Turing插入其他自己的编译/测试/缓存写入，独立推进本地审计与收件核验器准备。
两准确覆盖源compile-only均绿并关闭ledger：baseline **154359Z-45ffa1ec** 0/77.939305s，candidate **154632Z-eb0a7d58** 0/77.974962s，均15原files、原e532 control，format/locked netlab artifact/三锁1.4组/source diff空/identity/hash/group空通过；policy-stability-compile-readiness-v1.json只接受编译准入，runtime0。原三轮恢复物理screen已全绿并停止，不再同源追加物理试验。
当前修复源 **3b734073b410ec4130b4b57def3bf096b10c3e7a** / tree **473ee06d1a836d56d9e39bd10a7a4016fdacf530** / snapshot **4a688a33bd3da7f095cd8880ed1fd9519fb5ce09**，worktree khor-net-iroh-recovery-ack-order-v1 / branch experiment/net-iroh-recovery-ack-order-v1。只把complete_read改成先评估原recovery_probes、再mark_silent；反转三行后与红源逐字相同，原测试段/其他源码/vendor/锁/物理不变，source-audit SHA a814baae82dab7b97cc92c24b620c8ee08647104f5d6a155d7c20c75bc2d2f5d。无新state/timer/预算/新鲜性豁免，实际成本仍待测。
预期红 **150712Z-549ba3dd** 已101/200.479139s、17原files终态：Noq467全过，iroh197原通过逐名保持/原ignored1，**仅新a_first_recovery_ack_keeps_the_bounded_quality_episode真实行为失败（131ms旧质量vs85ms合同）**；node test compile过，三锁1.4组/source diff空/全部原hash/identity/group空、recovery-ack-order-library-verification-v1.json与ledger闭。不是格式/编译假红。红source70a6f7d9565e83d0f8bc42bb76940ed366d3ce10/tree5965044b/snapshot0988a71b；纯helper提取旧顺序、删新test并内联后与67逐字等同，红source-audit SHA dd08574a32bdb6886a324c6d0ec15039331031322f17e23de7aa4d0e0208ffc0。准确green full vendor **151200Z-d504e81e** 已0/180.387027s、17原files终态：Noq467/iroh198（含新增回归）全过、原ignored1，原命名人口与新测试段保持、node test compile/三锁1.4组/source diff空/hash/identity/group空/proof/ledger全闭。库绿只接受该回归与原库合同，不是物理恢复或最终接受。准确同源12app **151523Z-db3999ab** 已0/379.316808s、21原files，**276过/原ignored1**（70net、90hop及其余原命名人口），原七取消合同保持；原hash/identity/group空、recovery-ack-order-node-verification-v1.json和ledger已闭。scope仅12命令/命名人口，完整Node/N1/N3/N5/phone不接受。
原物理三fresh执行前已冻结 **E/iroh14-recovery-ack-order-physical-plan-v1.json** SHA **3133989963aed36e49181bad258aff99db29f975ae9a430202380035ab5b71be**：同source3b/control69/七原输入/4秒门/持续10秒前置、三case iroh14_recovery_ack_order_physical_r1/r2/r3，失败全留、三后停止。绑定准确vendor151200和app151523 proof均绿才准入；**ops/verify-recovery-ack-order-physical-v1.py RUN**。两个准入proof已闭并核对准确source3b；物理R1/R2/R3均已终态，三轮后停止，不晋升。
物理R1 **152244Z-802bb00c** 已0/200.580265s、16原files、124全ok，final_carried=true、原warm/decision过，X/Y Go3649477/3640302us；identity/hash/frozen attempts/group空、physical proof/ledger闭。recovery-episode-cost-v1.json只读复算两episode：X+897353/1149555/1399723us、Y+890347/1140430/1391369us各三次PING请求，6reservation/6ping请求/0merge/0unavailable，未超原三次预算；不是实际发包、全部probe/CPU/能耗或健康静默成本接受。R2/R3仍按先验固定，无其他Turing编译/测试重叠。
物理R2 **152638Z-8914d68c** 已0/214.366541s、16原files、124全ok，final_carried=true、原warm/decision过，Go3532073/3798379us，identity/hash/frozen attempts/group空/proof/ledger闭。全日志含启动准备共20episode/56reservation，10次PING请求/46merge；换网后只两Custom episode，各2PING请求+1merge，原三次上限均满足。不要把准备阶段的merged reservation当成额外发送，也不据这两个正例接受健康/静默成本。工具audit-recovery-episode-cost-v1.py只审已留日志，不做产品执行。
只读N5准备库存E/turing-build-tools-inventory-20261112-v1.json SHA b09e161487cd77c6f8f9838a65753edc374163c0f020c40e978adcff6fa681e2确认Git2.25.1/tmux3.0a、libevent.pc缺失，cc9.4/make4.2可用；零编译/安装/产品test，不改变宿主。E/mac-actions-readonly-refresh-20261112-v1.json最新Actions仍11:04原storage失败，无新恢复执行证据、零dispatch；不能把旧额度状态当成新验证，Mac准入仍以原门和实际恢复为准。混版原sync_mixed仍未执行；其scratch::own_tmux使用std temp下固定khor-test-tmux/khor-scratch，未来控制必须隔离短TMPDIR并核对本run服务器/host-child所有权，不可盲目清理共用socket。
物理R3 **153157Z-8dea5447** 已0/152.258487s、16原files、124全ok，final_carried=true、原warm/decision过，Go3766136/3757942us；原hash/identity/frozen attempts/group空/proof/ledger闭。完整结果 **E/iroh14-recovery-ack-order-physical-results-v1.json** SHA **f1a4c7ef3eb61470fb5f5c377265c224ecdebde91b460d1a80eb41296136e5d8**，fixed3/additional0/screen_passed=true/accepted=false/promotion=false。372请求全ok，六个Go3532073–3798379us；不是首连失败率或任意移动场景接受。两端实际PING请求/merge复算均不超三reservation/episode，其他探测类型/真实字节/能耗另验。

**长稳协议已在执行前冻结，baseline已启动，candidate尚零长测。** E/iroh14-policy-stability-plan-v1 SHAe89aaa0c657e0bb26e7dfd3f01312a417c9de0f87bd1bff41cc61407b0aa86c6；v2 SHA08c27588260363d228c415769d4e96fa01ec71217379e7292eeeae51ccc63b0c在覆盖源创建前补充测量结束后的原始peer/here journal导出和保留范围核验（journal KEEP300、最多约600行）。只验证保留范围，不能扩称上游事件从未丢失。
原5case p1/p1j/p1s/p5加新增p1h：同one_speed("p1h",65,5)，两路约130ms RTT/各5ms jitter，直接覆盖相对25%与5/10ms底真正不同的范围。原helper断言/原600秒窗口、p5四次每档短stall、p1s CPU contention及0诊断/单测试线程/单repeat保持；旧/新两arm依次各一次，不挑结果。source只有同一fixture25行新wrapper/被动测量后journal输出；所有生产/gate/计时不改，反向去掉这些插入即可还原两父源。overlay baseline **515e99f0d886de8035801801a10e7587167cf78b** / treef0005b7544d4e5d93eed5e3dfe19ac021960980b / snapshot10bb1bd227abb145b3591d94dbb413f8b7d66f49；candidate **adf39a4c1adbae1a7b35bf98e1a6e9815f7db1b7** / tree716caddfe8c009c8055d5d3b839b654474130f5b / snapshotac1b99c3fe292f00b449e233c372aa21c2ac7644。两个新worktree khor-net-iroh-stability-{baseline,candidate}-v1，分支同experiment/net-iroh-stability-{baseline,candidate}-v1。两覆盖fixture逐字相同，整个两树只差原策略三app文件和iroh recovery完成顺序文件，锁/scripts/其他源码逐字相同，source-audit SHA8e8311b241ccc790c6cbd3f9a6dd20cf8b5600f2a79d3fae29db95a08e17d3c6。
新control **f862ab2defb48d21236c2bdcca736c9a3d108773** 只从69修改netlab-stability分支的准确filter/mode及显式600s/4drops/diagnostics0默认值，runner/其他任务/UID/flock/资源/90分钟边界不变，C HEAD仍5d3691。先原e532 compile-only两准确覆盖源均绿，再启动baseline长测（预计50–60分钟），全部收件核验后candidate一次；长测中不插入Turing其他编译/测试。原p1s echo错误被helper忽略，不能扩称每次echo成功；packet/byte只有一个端点，不是总网能耗；完整N1/ordinary SLO/N5/phone不接受。新证据与协议见[IROH-RECOVERY-ORDER](IROH-RECOVERY-ORDER.md)：native R2在X+1.657s有93.923193ms实际新ACK，质量仍138.701662ms且recovery=None；首个post-judgment RTT可让mark_silent在episode建立前撤回Silence。该假设不是所有失败的唯一根因。observation SHA d488dd2868eeaa31674a290e4b20e4f606ab84d68619d3c7ac0f4c564c1fd48f，plan SHA cad768d3783b0428764ba098e73c4e23952f42e822e1b6b76af57b5ad1fbcd64。红已证实，最小换顺序候选正在同full vendor门，之后12app门；无新state/timer/配额/4秒门变更，实际probe成本仍待测。

**原生日志两fixed已全部终态，停止该批。** source bc3064af6cbaf52a4a94ee4cab4c4f2c251e0f8a/tree080dfce5/snapshot7a6c5695，control69原七文件/四秒门/预算保持。R1 **145400Z-fa959ba8** 0/162.309256s，R2 **145752Z-7b5e6698** 0/152.268069s，各16原files/124请求全ok/final_carried=true，identity/hash/frozen attempts/group空/proof/ledger闭。R1 warm/decision过Go3494224/3736614us；R2均红Go5411001/7404483us。结果E/iroh14-native-trace-results-v1.json SHA **f1cc7d4e76d25bed4064f7963703bf3afb5e751af08cc957ee4dc29faab49f51**，fixed2/additional0/accepted=false/promotion=false，捕获迟恢复但未捕获持续无carried。两端原生Noq/STREAM日志阳性，含feature真实执行；TRACE开销不作性能，缺唯一Noq drive id不自动合并stream编号。bc仅CLI opt-in filter改动，无新vendor/state/timer/probe/算法。compile-only145156Z-0bc0d1e5原0/84.043365s、15files/默认目标runtime0、proof/ledger闭；全部准备协议/source审计和原失败保留。

**旧依赖同app策略三轮诊断全部终态并停止。** old source **16e943d84cc7de86e30264a0426a32cf978961ad** / tree **f3c114d81a1c6af873e15f074efa60be5cc0171a** / snapshot **d47f30448784f6791eac71c99f265854165f6d5b**，worktree khor-net-iroh-old-stack-same-policy-v1，branch experiment/net-iroh-old-stack-same-policy-v1。旧dc738只应用028→67准确三app文件patch；全crates相对新67仅net/node两Cargo依赖清单不同，app Rust/测试/物理/scripts逐字相同，vendor/锁为旧iroh1.2/Noq1.3。source-audit-v1 SHA9286dc4460c8d9fdb4606214228af5c9e19334fea12faee0b2b1446718261877。
R1 **144049Z-562697e5** 0/160.299535s、Go3730444/3468655us；R2 **144427Z-fa855353** 0/146.225587s、Go3555379/3542873us；R3 **144738Z-d4b188ef** 0/150.262708s、Go3423765/3661948us。各124请求全ok、final_carried=true、原warm/decision均过，16原files/identity/hash/frozen attempts/group空/proof/ledger闭。结果 **E/iroh14-single-hysteresis-dependency-comparison-results-v1.json** SHA **608b6c78e26a269ace71a28b78597f9a1403dc7b480bbe072372d620af2145c7**，old_fixed3/additional0/new额外0，两个候选都不因此接受。该对照在新失败后选定、顺序未匹配/样本少，只收窄依赖变化及custom交互的调查范围，不是失败率或唯一上游回归证明。下一步原生日志定位容量/path-open与旧STREAM迁移，不再叠策略参数。
旧准备143631Z-bd0503ac原1/84.120891s、13files：Cargo结束但复用1.4 expected-version metadata拒绝1.2，零runtime，identity/hash/group空/preparation proof/ledger闭；仅suite及三个版本字面改control32dbbdf5，comparison-plan-v2 SHAed7ad997f55bd268d4976f767c0311698eb04b65b630877070a1966b61bfe861。同源143940Z-34e04129 0/25.759416s、15files，compile-only proof/ledger闭；binary SHA6cb044d575833c6986128e029993f153bce209c62f67aab015972f0133f505f5，runtime0，不是native provenance或276门接受。原comparison-plan-v1 SHA7122efcde90f4d44cbac5f3aebb8835d0c955eaf054882d5b40a827106a889f5及old physical-plan-v1 SHA349f8faa946c552bdef4e0488fc69dfdf370a4f9033e05f1434b287c265d3086保持。

**单一防抖新67固定三轮物理screen已全部终态并拒绝晋升。** 准确source **67c320135b40fb2e34d3d9a37523af2d912304e9** / tree **4077911ceb2c45a8c2ce990a5fd7ca5b35401779** / snapshot **c0d1c50fda47e827e21a7d43fa34ed4adaaa4834**，worktree khor-net-iroh-single-hysteresis-v1 / branch experiment/net-iroh-single-hysteresis-v1，control69d23d4f，原四秒门/持续10秒前置/七份输入不改。R1 **141018Z-445b94fe** 0/200.492783s，124请求全ok，warm/decision均过，X/Y Go3403177/3646386us；R2 **141413Z-cc8f4235** 0/196.530137s，R3 **142410Z-eb385819** 0/198.553189s，各124全ok但final_carried_observed=false，warm/decision均红、双向无Go。三轮16原files/identity/hash/frozen attempts/原group空/proof/ledger均闭，停止该批，不追加原样重试。
保留数据只读分析E/iroh14-single-hysteresis-physical-trace-v1.json SHA b626a1a762dfb515c8af1d1d3302658a6645e426e4130a06ecd18d017c4b03da：R2外层出现新direct后内层carried样本仍旧，M onward写flush计数增加而X没有对应read观察；这不提供缺失STREAM偏移或唯一根因。R3约5秒两外层都只剩Relay，carried约265ms比直接公共Relay131ms更慢，选择门不是唯一问题。共享测量不能证明每条外层连接恢复，需实际path/stream来源核对。
结果 **E/iroh14-single-hysteresis-physical-results-v1.json** SHA **92e5b6e59c46691d8c845f9f961e8cda7a591292479366e1f20e1ae0cdffdfda**，fixed_runs3/additional0/accepted=false/promotion=false。仅降低收益门不足以保证恢复；不开始基于该源的长稳成本接受，先定位R2/R3的carrier消失/测量质量与外层路径事实。所有失败和原先完整N0–N5范围保持。
原v2 auditor错误把final carried=true设为availability成功时的先决断言，R2因此首次审核中止；**physical-plan-v3**明确仅修数据审核逻辑。新 **ops/verify-single-hysteresis-physical-v3.py** 仍核对plan-v2的同源/同三case/七原输入，完整执行原selection/decision parser，并把final_carried_observed加入接受合取。未改产品、门槛、原解析器或期限；R2/R3已用v3记录真实红。
新67策略明确区分两个职责：已存在路径的使用/保留资格采用5/10ms底+原vendor连续两秒/决定后双边实测确认；**新建middle的估计收益仍保留原25%/5ms成本门**，原Node准备负例断言恢复，并新增一条跨层合同。只改switch_margin和hop::beats两生产体，其他生产token/vendor/锁/物理/parsers不变，E/iroh14-single-hysteresis-source-audit-v4.json SHA1e2830279e0391f40126dc5965389cc9dccc81ca741d0b25c2164971a02bfb37。新code无新state/timer/probe配额，已知carrier保留成本仍待实测。
保留全部前序：c4 **134302Z-887434b9** 101/53.853927s，原net68/ignored1逐名不变，仅两新104/131合同红；1460 **134620Z-4f615af3** 101/37.849456s，两新合同过但旧200/195边界红，69过/1红/ignored1，后续未跑；3f **135145Z-554511fc** 101/228.604469s，net70过/ignored1，hop88过/1红（a_relay_second_leg...的6ms不应新建middle负例），后续未跑。各identity/hash/group空/proof/ledger全闭。没有改掉这个准备成本负例来选绿；v3协议在继续前记录保留新建成本门、只简化已测路资格。原v1/v2协议/失败源都保留，不声称旧净行为等价。
完整12门 **140132Z-2d53d83b** 已0/363.333291s、21原files终态：**276过/原ignored1**（net70、hop90，其余原命名人口），原7取消合同保持；准确source/原hash/identity/group空与ops/verify-single-hysteresis-node-v2.py核验、proof/ledger全闭。intentional eligibility变化明确，不能称原断言语义等价；不是完整Node/普通性能/N1/phone接受。
现已按 **E/iroh14-single-hysteresis-physical-plan-v2.json** SHA **60f8ea18e646cb5a8e4bba7274088ff6ff18e33b4a1f0ff1b0a1b176ba23903e** 固定三fresh原peer物理screen（case revisions iroh14_single_hysteresis_physical_r1/r2/r3），控制69d23d4f/准确source-v3，原7输入/4秒门/预算不改，失败/缺事件均保留，三轮后停止。原physical-plan-v1绑定失败3f未执行任何物理slot，不能再用。
物理collect→identity→calibrate-peer-mobility→**ops/verify-single-hysteresis-physical-v3.py RUN**；旧v1不可用于67，v2缺少final-carried=false结果处理，不再用。screen有收益后仍需原时长健康/抖动/共同停顿、静默和实际成本旧/新对照，不接受N1/N3/最优/phone或合入canonical。普通性能源028、失败80、被动794均冻结。首连未定位、Mac/混版/真机与完整N0–N5保持。

**前一固定首连阶段诊断三轮已全部终态，停止该批。** source794ceaf6/tree2a60fb17/snapshotedcb9c70，control69d23d4f，原7份物理输入/parsers及四秒门均未改。R1 **132402Z-b119bebd** 0/158.415076s，R2 **132802Z-17e9011a** 0/150.328287s，R3 **133218Z-4d9c8c05** 0/146.301250s，各16原files/124请求全ok，identity/hash/frozen attempts/group空/proof/ledger均闭。
三轮warm sampled门过但decision均红：R1 X/Y Go3369982/7610709us；R2 X无Go（原门失败、缺事件不豁免）/Y7593836us；R3 X/Y3708399/5447209us。未捕获首连失败，不能宣告首连已修；在已有原红105138/125445基础上没有新唯一根因。阶段原始记录保留重复且不自动配对并发preparation；其他候选胜出取消是已存在的正常路径，不能由缺完成日志单独归因。
结果 `E/iroh14-hop-preparation-diagnostic-results-v1.json` SHA **ed9091d8dad5cab131cb121a68b82e11b280a79949e07ffd8fbeb14ce45b0c67**，runtime3/additional0/product_fix=false/accepted=false。794仅被动观察，不晋升canonical；不用原样追加采样挑结果。
同准确源compile-only **132102Z-7c5d5c82** 已0/69.945424s终态：format、locked netlab集成目标编译/Cargo artifact JSON/versions1.4组、源码diff空、15原files/hash/identity/group空核验，proof/ledger闭；binary SHA4d0d55aad86f5fce4aa3861045beb516b55ec1e35b7b694a1a26614dde86931b，**runtime0、不是native_provenance或产品接受**。v4 source audit证明v3 token等价审计后只应用准确远端formatter patch；无新依赖/算法/timer/预算/fixture，原7份物理输入/parsers逐字相同。
所有前置失败均留证且零产品runtime：131057Z-45f84a73 原1/45.831063s（c2 rustfmt，10files）；131430Z-375bb729 原1/51.854522s（d52日志DialFailure未实现Display，12files）；131834Z-88311c28 原1/43.833485s（c218分类closure rustfmt，10files）。各identity/hash/group空/preparation proof/ledger已闭。旧c2/d52/c218及其snapshot留refs，不再提交。
通过编译后、任何产品运行前冻结的E/iroh14-hop-preparation-diagnostic-plan-v4.json SHA d8ff68714c27c97a09cb00e68dd8c8c30de2cd67aa910c939da0ed76ecd27d5a与v2 auditor保持，原v1–v3/前置原红均保存。下一步按[IROH-HYSTERESIS](IROH-HYSTERESIS.md)收敛重复防抖职责的独立设计实验：原2秒/双边决定后样本、原故障快通道、预算和四秒门保持，拟比较app25%相对收益门与绝对5/10ms底+既有确认。此提议尚未实现；旧pure choose的60→48行为须显式记录新合同变化，不能静默改断言后声称原负例全部绿。先固定输入与旧/新对照，再做短合同/物理恢复，明显无效则不投入长时健康/抖动/共同停顿对照。首连未定位独立保留，Mac/混版/真机与完整N0–N5继续。

**确认copy候选80aeeff9不接纳。** 固定三轮物理验证已全部终态：R1 **125445Z-23ca34d4** 首setup10271172us超时，原101/197.619032s、尚未换网；R2 **125919Z-c38e8c1d** 原0/150.511588s、124请求全ok，但四秒selection/decision红，X/Y Go5522531/7762941us；R3 **130239Z-456f9489** 原0/150.388833s、124请求全ok，两门过，Go3375083/3613151us。各16原files、准确源/control/原hash/frozen attempts/group空核验，proof/ledger全部终态；没有补跑替换失败。
结果 `E/iroh14-confirm-copy-physical-results-v1.json` SHA **7a80dc776d5244aa0a3422b2ef167b06f3f53ce94f1a0e5c29ad4a0e9a3c17b5**，accepted=false。候选 **80aeeff919aa641dae55ba1a18b2020fead3e05e** / tree f857f33c60e45cb7fee862c47c510cf5a8a27765 / snapshot bc35d760bc6d0cf9d47aeee390e1913c175b8a10 保留为失败实验，不晋升canonical或普通性能源。固定原7份输入/解析器及四秒门未改。
修改后全库门 **125007Z-645fe88c** 已0/233.761110s终态：Noq467过、iroh198过（含新增）/原ignored1，node lib/tests编译、三锁/原hash/source diff/group空核验；17原files，E/run/confirm-copy-library-verification-v1.json，ledger已闭。未从unit绿接受物理/手机或完整N2。
红回归源9f3a6ee6的 **124312Z-e33e746e** 已101/338.074690s终态：Noq467全过，iroh原197过/原ignored1，仅新增quality-copy行为回归真实红（实际90ms、应85ms）；node lib/tests编译/三锁/源码diff与17原hash/group空均核对通过，proof/ledger已闭。红绿回归文件逐字相同，源码差异仅probe_priority/doc，E/iroh14-confirm-copy-source-audit-v1.json。原失败行2118/2119证明两copy在同tick均PTO0；+2298.518ms moving仍pending，+2551.014ms已None/restNone，E/iroh14-confirm-copy-observation-v1.json；精确取消调用仍非原日志直接观测。
全库绿只证实窄probe/质量单位合同；三轮物理未过，不能用该修复关闭首连/9.3s/完整恢复。设计、反例与实际结果见[IROH-CONFIRM-COPY](IROH-CONFIRM-COPY.md)。

**前一固定被动诊断批次已全部终态。** 源 **08ecab50f81f5e9e357a73614dface145feef432** / tree **bb049e979141bd90bc1ab4a980b232fa78a0b99e** / snapshot **84b69692f39fe9473a0da4316ac59334e77dbdac**，control **69d23d4f912c2e3674d47010eea5b2577230e182**，仅DEBUG取消/Rest补点，算法、物理和四秒门均不改，不晋升。
首轮 **121515Z-e8d79c86**：0/261.855773s、16files/124请求全ok；warm门过51请求，X Go3285193us，Y没有Go，原decision门红保持。5条取消均是当前Custom/Ip→候选Relay，未捕获目标Relay→Custom取消。固定追加两轮计划 `E/iroh14-pending-cancel-two-diagnostics-plan-v1.json` 已执行并关闭：R1 **122705Z-1248c9ae** 0/167.663671s，两门过，X/Y Go3505368/3506880us；R2 **123405Z-8a26c7cf** 0/179.241810s，两门过，X/Y Go3283424/3538071us。各16原files/124请求，identity/hash/frozen attempts独立重算/group空核对，proof/ledger全部终态；原失败不豁免。
结果 `E/iroh14-pending-cancel-two-diagnostics-results-v1.json` SHA **60fc1b2c7003109b2976f854231d3f6d5da65dff5f82e7ee0810e14131ec6b2a**。两轮后按预先计划停止采样，**未重现9.3s/未捕获目标取消，无唯一根因/修复**。继续分析原失败日志，不能再原样重跑挑绿。

**普通性能对照已全部终态。** [结果](IROH-WORKLOAD-RESULTS.md)与[冻结协议](IROH-WORKLOAD-COMPARISON.md)：旧dc738/新028、rolling8/priority0、37份fixture/metadata逐字相同，ABBAAB六block共8640交互全ok、12file完整、102原report SHA独立核对。E/iroh14-workload-comparison-v1.json SHA **940dda764a0f7787bf2439194b9bed6802ee3dbeed3736d60fe709358328732a**。全部proof/ledger/campaign终态。
low file中位19.222→18.798s，high75.729→66.124s；low bulk终端单轮p95中位84.194→90.816ms、目录115.843→125.784ms。新block3 high终端service p95/max681.495/1102.768ms、scheduled-to-end p95 1893.762ms；新block6 low idle_before终端p95另有246.149ms。全失败/人口保持，少量fresh/共享宿主负载/终端长尾使它仍非N3/p99/全局最优/手机接受；不以文件优势宣布升级胜出。

**严格peer恢复门仍红。** 旧110236与新110840完整日志本地应用准确frozen原4秒selection/decision门（零新产品run）：旧两门过，Go3472631/3462188us；新两门红，Go **9301772/9349684us**，52个eligible warm请求中15个AB未选carried。E/iroh14-peer-strict-gate-comparison-v1.json，ledger additional_contracts保留原availability的独立scope。原物理business124全ok并不接受最佳路及时恢复；新028首次setup超时原红105138同样保留，尚未注入换网。
E/iroh14-peer-recovery-sample-gap-v1.json绑定原行：新X可见carried质量111.16ms在2.535–6.998s未刷新，age增到4.835s，Relay约135ms；原25%gain门不选该值是源码/data推断，7.372s附近新carried约98ms后才重启2s确认。查pending被取消/Rest、测量代表copy/probe copy、有界恢复预算、外层积压与调度，**无唯一根因/修复，四秒/3probe/2s/250ms和新鲜性规则不得为此放宽**。

最新已闭地址场景均准确028：roam三场景102059Z-2a7ba71e（372逻辑请求），IPv6同前缀overlap两合同111507Z-d7912aa7（248），实际QUIC NAT端口111859Z-de624e0b（124，30/30 warm实际streams选中新tuple，mismatch0），全部proof/ledger终态，详见下表；不是同连接/逐packet/手机/完整N2。

下一步把两条独立链分开定位：首连红中，原journal记录hop尝试1于约3秒结束、总dial约10秒超时，生命周期没有hop_splice_bound；可用的M两腿先前业务均有观测，但首连缺具体失败阶段，不能直接归因上游、负载或目标不可达。准备独立的被动hop阶段诊断源（由冻结028派生，仅日志），不再原样跑80挑绿。恢复时R2的X到+3.522s才开始最终两秒确认，Y +1.511s开始后又重启到+5.762s；gain判据/质量窗口/连续确认的叠加仍待审查，原budget/freshness/四秒门不放宽。冻结普通源028及旧sourcequad不动。
混版后续优先复用crates/cli/tests/sync_mixed.rs与scratch::pair_built；需准确旧/新CLI及OLD_KHOR非skip执行标记，缺OLD_KHOR直接返回不能算接受；该fixture只覆盖双向session row/pin，不扩称peer relay/全RPC。其余完整N0–N5和补丁退出继续。

Mac仍50GiB启动余量门红；只读库存43205919189又被GitHub付款/额度在prepare前拒绝（E/mac-storage-inventory-43205919189/receipt.json），storage skipped、零Mac操作。已异步请求恢复Actions或常规SSH，无回复；最新一次SSH仍banner timeout255。Turing继续，不降门/清未知缓存。
root为E/source-refs.json唯一串行原子writer；submit/launch/collect/verify待真实exec会话终态，unknown handle先恢复receipt。源/任务与所有旧证据范围见下文，不因阶段结束停止完整目标。

## 升级位置与授权边界

- Canonical `/Volumes/disk0/Khor/worktrees/current/khor-net` / `feat/net` 仍原运行时**5064fe2a**（后续均docs）；依赖仍iroh1.2.0/Noq-proto1.3.0。没有合并升级或rolling实验到canonical。
- 最新稳定升级候选 `/Volumes/disk0/Khor/worktrees/current/khor-net-iroh-upgrade-v1` / `experiment/net-iroh-upgrade-v1`，HEAD **02807c79** clean。从旧整合源dc73837e建树并合入规划文档，使用SHA匹配的发布包三方迁移到**iroh1.4.1/Noq1.4.0**，根及vendor锁已统一；七处冲突已处理，原本地专属回归和上游blocked-Relay Initial回归均保留。
- app的net/node Rust及四份wire metadata相对dc738逐字不变（E/iroh-upgrade-runtime-diff-v1.json）；依赖更新仍改变运行时，旧性能不能沿用。
- iroH补丁22的手工IPv6 trim已替换为上游helper，两上游测试函数体亦同，库门已过；IPv6 QAD测试含地址不可用提前返回，不能据普通ok扩称IPv6 WAN/手机通过。
- Noq2、5移除对照真实红，保留当前保护。E/iroh-upgrade-patch-ledger-v4.json记录60历史条目：**1项上游替代库验证、2项保留、57项待逐项退出核查**，不是57项全部必要；详见[IROH-UPGRADE](IROH-UPGRADE.md)与[UPSTREAM](UPSTREAM.md)。BLE/Tor/Nym不新增到本轮。
- 仍无main合并/发布、生产Relay采购部署、已安装App替换授权；没有新手机窗口。MacLab与Turing同LAN，构建macserver独立，不能混淆拓扑。

## 新稳定版的已核验证据

| 层 | run | 原结果与范围 |
| --- | --- | --- |
| Turing全vendor库门 | turing-20261112T084724Z-35d6dfe3 | 0/226.384657s，17原files；Noq467过、iroh197过/原flaky ignored1；node lib/tests编译过、三锁与冻结源同、post-check源码diff空 |
| Mac同树库门 | 43186999295 | 16原files/11controls；与Linux同467/197命名人口、原ignored1；node编译与三锁/源码diff通过；ZIP f8cbf61034871f8591f02bbbea4ff58953ac8c0974ade643cdec2a8278c48024 |
| Turing固定12应用门 | turing-20261112T085339Z-ab57a92e | 0/385.723121s，21原files；273过/原identity辅助ignored1，命名人口与旧dc738完全一致；无独立RPC计数扩称 |
| 36真实流合同 | turing-20261112T090144Z-223b1901 | 0/31.781419s，31原files；4份准确frozen metadata重算，独立74 Fetch RPC/10peer；273与36人口重叠不相加 |
| 实际CLI取消合同v2 | turing-20261112T095017Z-a01289cd | 0/186.335657s，21原files；两parent合同/两声明fixture helper，source2795文件独立匹配Git、CLI prior→node provenance/选中binary哈希核对，6不同任务Cancel IPC实际到达；两case4/24观测owner均退出、无强制清理/遗留owner/私有可执行文件足迹 |
| netlab集成目标编译 | turing-20261112T100217Z-d286d5bb | 0/120.570580s，15原files；锁定1.4组、目标artifact与binary SHA记录、post-compile diff空；**runtime0、非native_provenance或性能接受** |
| 新稳定版静默first-action | turing-20261112T100747Z-15afeb63 | 0/332.109970s，29原files；2 diagnostics-off guards/6参数fresh读；12准备、12完整≥10s静默与12真实STREAM阳性通过；普通时延样本0，非完整N1/能耗 |
| 实际接口换网三场景 | turing-20261112T102059Z-2a7ba71e | 0/186.422335s，13原files；dual/v4-NAT/v4→v6实际eth0重接，372逻辑请求/744起止全ok；非移动端先请求、内核地址独立核对；非真实QUIC端口rebind/同连接/手机或完整N2 |
| IPv6地址重叠替换两合同 | turing-20261112T111507Z-d7912aa7 | 0/165.659413s，13原files；2tests/248逻辑请求/496起止全ok，内核/IPv4隔离及非移动端先请求合同通过；非手机/同连接/完整N2 |
| 实际QUIC NAT端口重绑定 | turing-20261112T111859Z-de624e0b | 0/99.774935s，18原files；r0校准+r1远端先请求，124请求全ok、30/30 warm streams选中新tuple，mismatch0；非同连接/手机/完整N2 |

静默proof为`iroh14-stream-quiet-verification-v2.json`；换网为`iroh14-roam-verification-v1.json`。后者各场景首remote单次原us为710238/777650/646640，不构成p99/SLO；UDP reflector使用独立固定端口，不能替代实际QUIC端口重绑定证据。

前述每run的identity/source/bundle/原hash/原group空与behavior/compile proof分别在E/run目录，root ledger均终态。库门proof为stable-upgrade-gate-verification-v1.json；固定12为stable-upgrade-node-suite-verification-v1.json；真实流为stable-upgrade-transfer-verification-v1.json；CLI为actual-mcp-verification-v2.json；编译为netlab-compile-verification-v2.json。
实际CLI合同用控制Python sidecar，不是真实agent bundle/WAN/手机/能耗。Turing现有conda Python3.13.13（任务临时wrapper -E -S）SHA c46ed00dd978dbb20e32eb34d9e16660775b2ed4fc84395a73120804e3e66379；没有改系统环境。旧Noq447、iroh184等数量只对旧源成立，不把不同平台/重叠套件累加成独立覆盖。

## 原红与退出对照（保持）

- 新028完整node诊断 **090450Z-bfc82a31** 原101/552.029829s，旧dc738同control340b3243的 **092140Z-5216816b** 原101/636.509801s：同1387人口，同1274过/38失败/75ignored，new-only与baseline-only失败均空，E/iroh-upgrade-full-node-comparison-v1.json。**二者不算完整node接受，也不豁免原失败**。20项直接观察Git2.25.1不认initial-branch参数，tmux3.0a；两项缺同源CLI（已另跑正确前置后两合同），两项账户runtime状态失败仍待定位。Quality全前置/真实agent bundle/isolated接受与其他原红保持。
- Noq5删除源e69b137c，**091835Z-cb319e57** / control958dc554，101/33.786267s，12原files：原4fixture三过一红，合法已分配/回收ACK-only最大号加低号ACK-eliciting包，误增RTT计数5→6。fixture/锁不变、仅mod.rs移除保护；基线1.4同四命名绿。E/iroh-noq-ack-exit-comparison-v1.json。此为状态unit模型，不扩称wire测量。
- Noq2删除源79d2ca64，**093340Z-deb1f7de** / controlfc6fe1d1，101/37.609123s，12原files：双方reporting both、服务端响应前发早数据，移除保护后ObservedAddr TX1而应0；原fixture/锁不变，基线同命名绿。E/iroh-noq-zero-rtt-exit-comparison-v1.json。两ablation源不晋升。
- 首准备 **071440Z-2f31345c** 101/226.706692s：Noq stats重复FxHashMap编译红，19原files；准确返回锁/formatter patch应用后纠正得到028。首gate **072057Z-c53a26f8** 因准备写共享source cache被保护拒绝、零测试，2原files；独占锁核准旧HEAD/tree/完整patchSHA后，将整个cache及owner marker改名保存在原准备run/work/preserved-preparation-source，未删除/reset。后续准备control94b1a637改一次性clone，未重复执行。
- Mac退出 **43189070840** 原50GiB余量门拒绝、零产品测试，5原files/ZIP 4f7ca2212b4ad3968ec775e54e747c6e9f1be01ea0061f6352984a6d5891c981；只验证control/bundle transport，缺source/environment执行证据，不记identity/behavior接受。已转Turing，不盲目重发Mac。
- 首实际CLI **094221Z-91240bdf** 原1/197.852856s、20原files：CLI/node provenance已过，首parent+合法mcp_cancel_worker均Rust过，但控制未声明helper导致严格metadata拒绝，第二case零执行；原failure保持。v2只补准确helper声明并在验证前持久attempt所有权结果，原源码/函数/门不改；E/actual-mcp-metadata-parser-check-v2.json对原日志与六种错误合成副本校验，零新增产品/网络样本。

完整设计与未闭范围仍见下文，原24性能场景、尾部问题、手机失败观察、Native/Finder红与所有旧证据保持，不因最新库门/CLI合同而缩小目标。

## 已冻结输入与四真实RPC合同

E/pipeline-source-quad-v1.json：宽8A0c4e4e35/B e5527f93、宽2A ea414bfc/B2438172c；E/pipeline-w{8,2}-{batch,rolling}-source-v1.json完整四树。priority0、A原batch函数逐字不改，B函数逐字来自57ff原型；同helper/逻辑/长files_wire/所有vendor锁，宽度内只有调度循环/必要短读重启差异，跨宽只有PULL常量。共同Python wrapper只标记本轮，沿用原recorder全部人口/期限/失败门。
宽8batch36a4357d、rolling436c6664、宽2batch0a61687c、rolling19528761各四逻辑/三真实合同全过，13原files/准确metadata/独立人口/hash/identity/原group空与ledger终态，各pipeline-wire-verification-v1.json。
宽8慢尾A已发9/客户端解码8/提交1片，B已发11/解码8/提交8片；慢头两者已发9/解码8/提交1片未超开。两宽两armfuture drop均收到全部等待流STOP_SENDING且同wire独立Sessions成功。
这只证明控制responder与原future drop，**不替代生产server、Job stop、host-child或最终N3**。首格式红5f783fc8原1/零行为、原patch准确映射b020a801与原证据保持。

N1诊断静默最新a09450d0原0/339.545858s/29files，十二准备/十二≥10秒静默/十二真实请求阳性见证过；仅full日志诊断准入，不是ordinary性能、手机或完整N1。旧v1静默红、普通静默缺证与全剩余范围保留在下文及历史。

## 工作位置、执行与授权

- PRODUCT `/Volumes/disk0/Khor/worktrees/current/khor-net`，branch **feat/net**，创建基线6485cb83；当前已测代码提交5064fe2a，之后STATE/历史文档提交不冒充重新测试的源码。
- CONTROL `/Volumes/disk0/Khor/worktrees/current/khor-build-net`，HEAD仍 **5d3691f252f9dee0297fa3c494374be6119bca29**。新控制只建Git对象/ref，不移动HEAD；现有两个未跟踪__pycache__保留。
- 隔离旧rolling原型 `/Volumes/disk0/Khor/worktrees/current/khor-net-pipeline-v1`，branch experiment/net-pipeline-v1，clean **57ffe973ed11d03dc76c5fa01904719b27651b36**；未集成、priority仍-1，不能直接与当前priority0比较。
- EVIDENCE `/Volumes/disk0/Khor/evidence/net`，ops在其下ops；canonical ledger `source-refs.json`仅root串行原子写。
- 本机source-only：不跑本机Rust/Cargo/rustfmt/Node/Xcode/产品。Git、Python标准库数据/合成检查、bash -n可用。重型验证Turing与macserver+私库colehank/khor-build；具体控制和收据不得混用。
- Turing原UID1110/flock/user-netns、Rust1.95/jobs16/nice10、50GiB启动/40停止、90分钟保持；不sudo/改宿主网络/在同Turing计时中插入编译。Mac共享guard/jobs2/90分钟保持，可独立重叠。
- 用户已授权“完成计划”实施，旧AGENTS“仅保存计划”不是永久暂停。无本任务main合并/发布、中继采购/生产部署、已装App变更授权；中央GUI任务的ship授权不套用。
- MacLab与Turing同LAN；macserver为另一构建机。localhost是旧0.2.0 iPhone；此前人工反复开关恢复不算候选自动接管通过，旧手机窗口已结束。
- 图谱partial/stale，不覆盖vendor时的0nodes/0flows不是覆盖证明；未建新的CodeGraph索引。不派发子智能体。

## N1首次操作与静默：已知证据及未闭合同

协议 [IDLE-ACTION](IDLE-ACTION.md)。原p7 active10秒/idle20秒门不改。原idle选路恢复16–17秒与15秒备用保活量级接近，只是假设，不证明唯一根因或极致体验达标。

**fixture所有权旧红→修复绿已闭**：

- 旧source e471a2f1、control e9a443c4，**turing-20261112T015902Z-e8569de4**：真实caller drop后device future继续，原1失败/1负控过，清理后才断言，原101/90.034秒/13files。
- 修复 **0a8b90d9986d3426f65b61ae9d994083f1fa3321** 只在LabNode.with由调用者持有AbortHandle Drop。**turing-20261112T020453Z-b9c5278d** 原0/53.827秒/两项全绿/13files。
- 两份idle-action-ownership-verification-v1.json：guard逐字相同、control相同、准确metadata/原marker/footer/hash、旧红绑定、原group空与ledger终态。只证明fixture caller→device future；不替代server RPC/host-child/产品恢复。
- 原workload filepull是父future直接借用tokio::join!，没走LabNode.with；不据此改写prefetch结果。

**普通六次首次读取已观测，静默合同未由它们证明**：

- 首个源码725e12f7的e6569ee7仅格式红/零行为，原件保存。
- source8f87adcb的 **turing-20261112T022049Z-7b65087d**：六次读取全ok，但metadata误期望两个netem都在alpha，缺router ingress读回，原1/226.568秒/17files失败保持。
- 修正source **5a5d7e84a557b2f2000aa0dcf513d61adc4538cb** / tree d1e6496d / snapshot67db6c6f，control2cfce93d；**turing-20261112T023151Z-8d7b0ea1** 原0/224.643秒/17files。六caller/十二起止/162条命令、真实双向150ms安装及恢复清除、veth ifindex/link_index双向绑定、逐接口计数独立复算、全部原SHA与ledger终态。
- alpha phase0/14/5首次读取297.685/327.577/321.867ms；beta230.688/265.406/250.937ms。实际phase与两端事后Relay路径、全部接口差值见first-action-verification-v2.json；每格只有一个fresh，无p95/p99/显著性/唯一因果或能耗结论。
- quiet-scope-audit-v1.json绑定上述proof：没有主动前台调用不证明无后台STREAM；完整N1/quiet未接受。普通source-owned输出显式continuous_stream_silence_verified=false/full_quiet_contract_accepted=false。

**Iroh50被动见证、v1真实红与v2准入绿**：

- 只在原250ms reselect已有stats读取上加DEBUG快照：所有受跟踪连接STREAM tx/rx、actor序号、增删代数及单调/墙钟；连接在采样间新建关闭也会改代数。原合计/ActiveRefresh/策略/生产计时器不改，无新任务/定时器。
- source **09374004906a9a400004874fa957726e857bef1f** / tree b23ff21f / snapshot8ee7e55c，control **1bdb2272f23bfd59961af79cbebf04242791ce33**；**turing-20261112T030608Z-15ebb293** 原1/305.229247秒/29files/identity/原group空。
- Rust原两guard+六读全过、十二真实请求阳性对照全过；十二静默见证全红。stream-quiet-verification-v1.json准确冻结复算、独立原人口/全部原SHA与ledger终态。
- 十三次STREAM变化被见证于候选窗起始后307642–645086us，actor/registry不变。这是观察tick，不是精确发包时刻或唯一业务来源；原失败保留，不把后半段选出来改绿。

**当前v2修复的是准备阶段**：

- 新revision first_foreground_stream_witness_v2，控制73fa35f5只改同task MODE literal；runner/其他分支/原UID/flock/netns/资源/期限逐字不变。
- 仅诊断模式、候选窗之前只读双方serve.log，实际counter/actor/registry签名连续≥1秒相同、快照年龄≤1秒、总准备期限10秒。无请求、无Node状态查询、无关闭后台任务。
- 记录准备起止和各端实际稳定快照；外部逐条绑定原日志。随后原12秒候选窗、固定500ms边界内≥10秒完整见证、整窗恒定、最大1秒采样间隔/100ms时钟增量差、原phase0/14/5与30秒caller、六fresh顺序及阳性门全部不改；候选失败不重试挑窗。
- 普通模式仍10秒/off且不等待此诊断准备，诊断不作为普通计时/CPU/能耗样本。
- 11种见证+6种准备+19种first-action+9种guard合成数据，4 unittest已过；6种调度stub保持guard off→action full与首原73/79失败。全部只是工具合成准入，真实v2已按顶层run/proof通过该诊断准入；完整N1/ordinary性能仍未接受。

## Mac默认vendor与其他回归

**新Iroh50准确源码默认库已绿**：Mac **43146243502** / request **ff36367cf58e4a1ebd0e5d6f5d1c460c**，source09374004/treeb23ff21f/snapshot **901d85407023827999147998c3318192f8be302b**，control **4e34fb7dce174345410a33e892b0eb76a5cc5c23**。
artifact **13911657397** / ZIP **4d8db782bfd2767c88855d5a14f3b6249aa06e338617ed1ba7e715e5a4904ce3**；E/mac-stream-vendor-43146243502，实际文件在artifact/（不是report/）。
16files/11controls/source/bundle/两vendor锁逐字一致/各原footer和SHA核验、vendor-full-verification-v1.json与ledger终态。
Noq完整默认库447过/0忽略/0筛除；Iroh完整默认库184过/原1忽略/0筛除。唯一原ignored flaky为endpoint::tests::connect_via_relay_becomes_direct_and_sends_direct，未运行/删除/称通过；58路径/11home专项与184重叠，不相加。
相对当前5064完整vendor差异只有KHOR.md文档，运行时代码与锁逐字相同；该证明适用于vendor字节，不证明CLI新准备逻辑或最终Node/native/手机。

原e471准确源码Mac **43134579036** 固定12项net-node全部通过：net68+1原worker ignored/hop89/wires16/journal30/budget32/IPC18+六单项。该门不是全部1378 Node，也不能声称新Iroh50之后最终Node已全跑。

隔离rolling原型57ffe：4窗口逻辑、跨八片的files_wire和handoff真实scratch合同已两次过；files库32过/唯一Finder trash AppleEvent -1712失败，Mac **43124311515 / 43126157862**，整轮失败保存。一次确认已用尽，不盲目同源重跑、不豁免该门。只读主机观察43128627304为零产品测试，未证权限/响应唯一根因；后续仍需收口。

## N3已闭对照与接下来rolling

[优先级](SCHEDULING.md)：low/high各完整18fresh，分别12960交互全ok、文件全完整、原population/hash/准确metadata独立复核。低profile目录p95九对-1更快，但high文件/目录/终端服务九个对应位置六次-1更慢、三次更快；不把位置差当逐请求因果。canonical c3287169恢复中性priority0，PULL_AHEAD=8；不是全局最优已证。

[批量prefetch](PREFETCH.md)：原8,1,2,1,2,8,2,8,1九组18fresh完整终态，12功能通过/六high文件原240秒超时；12960交互全ok、153原report文件SHA与原始人口复核。low九份16MiB全完整；ahead8三份192MiB完成82.239568–98.920231秒，1/2各三high超时。超时是右截断attempt，无verified payload rate，不称文件完成时间。
prefetch-comparison-v1.json SHA **7dede5f2c580d46aff0e6138b968da2550db2e095d6de351652a56d8a7e537b0**；全部advance1–9与旧high1–6终态不能重复。

[rolling协议](PIPELINE-COMPARISON.md)及prefetch-to-pipeline-decision-v1.json在任何rolling计时前冻结：**先宽8，再独立宽2**，priority0，每宽A B B A A B、每block原w3两项再low/high各1，共12/宽、总24fresh。宽2在low的完成/交互取舍值得保留但high原超时，不能直接设全网默认。
四源码与campaign已冻结；当前十二个block/二十四场景均收件核验（含三个原high文件超时）；原campaign终态，独立尾部诊断不算新普通样本。匹配baseline/candidate包含同一未调用helper/四逻辑/大于八片的files_wire载荷；A原batch函数不动、B仅rolling循环及必需短读重启；四树审全部Git/runtime/fixture/参数，只在相同宽度比较。不拿旧prefetch当新baseline，原失败/完整人口/期限门不改。
真实RPC的已发/完成/待提交上限、补齐与caller drop已有四源码合同，用户Job/DropStop/Offer stop/cut本轮真实合同已闭，但控制responder不替代生产source。后续仍需生产server/worker、完整run_take与host-child、最终发布所有权；全前台预览、公平/背压/连接架构/规模成本仍必需。

## 完整剩余范围（不以已有绿项缩减）

- N0：最终分场景UX门、充分尾部样本与p99、相同条件完整前后对照、观测成本与成本前沿；对载荷/调度/模型作实际记录。
- N1：严重/quiet/recovery尾部、跨连接独立质量/旧样本、恢复后固定好路径匹配参考、启动/配对旧失败根因、动态模型延迟突变问题、真实静默及后台成本。旧16–17秒/单轮绿不算极致接受。
- N2：同一实际连接独立/同时IPv4与IPv6地址/前缀/NAT映射变化、换网/睡眠/黑洞/LOST，owner/cancel/late/revoke与carrier在调用开始后失效；身份、总期限、写入/0RTT不重放。
- N3：上述24rolling探索已闭；仍需完整前台文件/预览、已发现的终端尾部、持续公平/饥饿/背压、共享与分连接实测、真实server和独立host-child取消/退出、1/3/5设备CPU/字节/唤醒/能耗。文件快不代替其他行为。
- N4：Direct/公共Relay/peer relay/SSH双腿，唯一可达middle自动救通（第二腿可Relay或旧信息）、多个/移动middle、独立双栈变化、北京/香港整机/单出口/寻址故障域，自家relay约束、双家与地域/容量/运维设计。部署采购另授权。
- N5：主60项（Iroh1–41/45–50+Noq1–13；42–44仍隔离）的逐项上游等价/保留退出，其他feature/integration/physical/Noq1+physical5；最终准确源码Quality/完整Node/native/proxy、Finder依赖、多设备/手机自动换网/peer relay与真实能耗及一致报告。

历史动态f2f1775027、恢复ce3b12226、长稳de04bcc1、四peer6970ad46、模型容量c50ab6ef和cold high5d27f45e等既有证据在历史入口完整保留；warm v2路径失败、启动/配对超时及旧严重红项也保留，不能拿新窄场景覆盖。没有用户iPhone候选安装/最终接受。

进度估计：此前2–4天为低置信度估计，不因compact/session重新起算；目前没有可信完成日期。完成必须设计、状态职责、持续矩阵与同条件报告均有准确版本证据。未获准的合并/发布/采购不自动执行。

### 2026-11-12 04:03 UTC 状态回复与估计

用户再次询问；明确尚未完成，普通性能与iPhone自动换网/peer relay/能耗未接受。下一批真实RPC合同加24匹配场景暂按4–8小时预留（含排队与构建、低置信度、不是承诺），完整计划仍数天且无可信日期；此前估计偏乐观，不从本次回复重新起算。全部N0–N5范围保持。


## 本轮补丁与手机静态审查补充

E/iroh14-event-buffer-exit-audit-v1.json为补丁21容量覆盖的后续退出候选：已有上游Lagged/paths快照和app选择恢复，但mine/Openings库存尚未在Lagged对账，需真实burst消费落后回归；主动ping另评估，未删补丁/未跑新产品测试。
E/iroh14-ios-notification-static-audit-v1.json核对OS callback→resident→iroh/有界恢复链存在；callback去重键仅status+接口类型，同Wi-Fi地址-only不独立穿过该去重；iroh另有监控，不能判漏通知或现场根因。手机须记录地址/前缀/地址族变化、事件所有权及remote-first/休眠顺序，未获真机接受。详见IROH-UPGRADE末两节，运行时均未改。


## 最小peer取消诊断源（已启动，编译/诊断在途）

隔离树 `/Volumes/disk0/Khor/worktrees/current/khor-net-iroh-peer-diagnostic-v1`，branch experiment/net-iroh-peer-diagnostic-v1，源 **08ecab50f81f5e9e357a73614dface145feef432** / tree **bb049e979141bd90bc1ab4a980b232fa78a0b99e** / snapshot **84b69692f39fe9473a0da4316ac59334e77dbdac**，clean。descriptor E/iroh14-peer-diagnostic-source-v1.json，两refs refs/khor/net-experiments/iroh14-peer-diagnostic-v1/{source,snapshot}已保留。
仅在原khor_net_samples DEBUG下补confirm_move因selector无候选/保持当前而clear时的pending/requested/current/since/now/readings/silent；Moving在已有pending进入Rest时补一次原日志。两文件差异为18行日志/谓词和KHOR说明。E/iroh14-peer-diagnostic-source-audit-v1.json逐字证明去掉新增诊断/日志谓词后生产Rust复原028，Noq/app/锁/原物理fixture/两严格parser均不变；这只是结构证明，**尚未编译/格式/运行，不是修复或新的验收**。
六轮普通计时全部终态、收件/核验之后，已用原control69d23d4f、task netlab-peer-mobility-samples、revision iroh14_peer_pending_cancellation_v1、该descriptor、最后block6 receipt为predecessor启动顶层唯一诊断，保持原4秒门及所有预算，记录失败/成功，不原样无限重跑。重点区分pending取消/Rest、采样copy与probe copy、外层恢复质量；不要在原因不明时改gain/样本门/预算。当前诊断已启动，canonical/冻结性能源未改，尚无修复。
